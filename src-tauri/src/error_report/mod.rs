//! Everything someone needs to fix one error, in a single file you can hand to an AI agent.
//!
//! A player who hits a Lua error can read the mod's name in the Monitor, but fixing it means
//! knowing which game build they are on, what that mod ships, which other mod replaces the same
//! file, and what the failing file contains. All of that is already in the Studio; this gathers
//! it into one Markdown file, written for whoever reads it next — a person or an agent.

use crate::vfs::StudioPaths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// What the Monitor knows about the error, handed back for the report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ErrorReportRequest {
    /// The log line as the game wrote it.
    pub raw_error: String,
    /// The Studio's one-line reading of it.
    pub title: String,
    pub explanation: String,
    /// The mod the game blamed, when it named one.
    pub mod_name: Option<String>,
    /// The file named on the line, e.g. `NC_Main.lua`.
    pub source_file: Option<String>,
    /// Surrounding log lines the Monitor already had, oldest first.
    pub log_context: Vec<String>,
}

/// Biggest file worth pasting whole; beyond that the report says where it is instead.
const MAX_INLINE_BYTES: u64 = 120 * 1024;

fn game_build(paths: &StudioPaths) -> Option<String> {
    for dir in crate::load_order::mod_info::get_all_user_zomboid_dirs(&paths.user_zomboid_dir) {
        if let Ok(text) = std::fs::read_to_string(dir.join("version.txt")) {
            let first = text.lines().next().unwrap_or("").trim().to_string();
            if !first.is_empty() {
                return Some(first);
            }
        }
    }
    None
}

/// The file the error names, somewhere inside the mod's folder.
fn find_source_file(root: &Path, file_name: &str) -> Option<PathBuf> {
    let wanted = file_name.rsplit(['/', '\\']).next()?.to_lowercase();
    walkdir::WalkDir::new(root)
        .max_depth(12)
        .into_iter()
        .filter_map(Result::ok)
        .find(|e| {
            e.file_type().is_file()
                && e.file_name().to_string_lossy().to_lowercase() == wanted
        })
        .map(|e| e.into_path())
}

/// `media/...` path of a file inside a mod, which is how the game refers to it.
fn media_path(file: &Path) -> Option<String> {
    let text = file.to_string_lossy().replace('\\', "/");
    let at = text.to_lowercase().find("/media/")?;
    Some(text[at + 1..].to_string())
}

/// The text key in `... ERROR: Formatting "Sandbox_NeatLockpicking_X"`.
fn formatting_key(line: &str) -> Option<String> {
    let rest = line.split("Formatting \"").nth(1)?;
    let key = rest.split('"').next()?.trim();
    (!key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')).then(|| key.to_string())
}

/// Where a translation key is defined: which mod, which file, the line.
#[derive(Debug, Clone, PartialEq)]
struct KeyHit {
    key: String,
    mod_id: String,
    mod_name: String,
    file: String,
    line: String,
}

/// The game's language (`language=` in Zomboid/options.ini), "EN" when it says nothing.
fn game_language(user_zomboid_dir: &str) -> String {
    crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .and_then(|d| std::fs::read_to_string(d.join("options.ini")).ok())
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("language=").map(|v| v.trim().to_uppercase())))
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "EN".to_string())
}

/// Looks for each key in the translation files of the active mods (`media/lua/shared/Translate`
/// of the folders the build loads, .txt and .json), last-loading mod first as the game reads them,
/// and within a mod the player's language first, then English, then the rest.
fn find_translation_keys(catalog: &crate::catalog::ModCatalog, active: &[String], keys: &[String], language: &str) -> Vec<KeyHit> {
    let mut hits: Vec<KeyHit> = Vec::new();
    let game = catalog.game_version_int();
    let rank = |lower: &str| {
        if lower.contains(&format!("/translate/{}/", language.to_lowercase())) {
            0
        } else if lower.contains("/translate/en/") {
            1
        } else {
            2
        }
    };
    for entry in active.iter().rev() {
        if hits.len() == keys.len() {
            break;
        }
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        let mut files: Vec<(usize, PathBuf)> = Vec::new();
        for layer in crate::load_order::mod_info::media_layers_for_build(Path::new(&m.root_dir), game) {
            let dir = layer.join("media").join("lua").join("shared").join("Translate");
            for e in walkdir::WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
                let lower = e.path().to_string_lossy().replace('\\', "/").to_lowercase();
                if e.file_type().is_file() && (lower.ends_with(".txt") || lower.ends_with(".json")) {
                    files.push((rank(&lower), e.path().to_path_buf()));
                }
            }
        }
        files.sort_by_key(|(r, _)| *r);
        for (_, path) in &files {
            let path = path.as_path();
            let Ok(text) = std::fs::read_to_string(path) else { continue };
            for key in keys {
                if hits.iter().any(|h| &h.key == key) {
                    continue;
                }
                let quoted = format!("\"{}\"", key);
                if let Some(line) = text.lines().find(|l| {
                    let t = l.trim_start();
                    (t.starts_with(key.as_str()) && t[key.len()..].trim_start().starts_with('=')) || t.starts_with(&quoted)
                }) {
                    hits.push(KeyHit {
                        key: key.clone(),
                        mod_id: m.id.clone(),
                        mod_name: m.name.clone(),
                        file: media_path(path).unwrap_or_else(|| path.to_string_lossy().to_string()),
                        line: line.to_string(),
                    });
                }
            }
        }
    }
    hits
}

/// One Markdown file: what broke, on what, in which mod, with the code.
pub fn build(paths: &StudioPaths, req: &ErrorReportRequest) -> Result<String, String> {
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let active: Vec<String> = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path)
        .map(|d| d.active_mods)
        .unwrap_or_default();

    let blamed = req.mod_name.as_ref().and_then(|name| {
        catalog
            .mods
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name) || m.id.eq_ignore_ascii_case(name))
    });

    let mut out = String::new();
    out.push_str("# Project Zomboid mod error\n\n");
    out.push_str(
        "This file was written by PZ Mod Studio so the error below can be fixed without the \
         machine it happened on. Everything here was read from the player's own install.\n\n",
    );

    // ---------------------------------------------------------------- what to do with it
    out.push_str("## What is being asked\n\n");
    out.push_str(
        "Work out why this error happens and propose a fix. A fix is either a corrected copy of \
         the file shown below, or a short patch the player can apply; say which lines change and \
         why. If the cause is that the mod was written for Build 41 and this is Build 42, say \
         which call changed and what replaces it. If it cannot be fixed from the outside — the \
         mod needs its author — say that plainly instead of guessing.\n\n",
    );

    // ---------------------------------------------------------------- the error
    out.push_str("## The error\n\n");
    out.push_str(&format!("{}\n\n", req.title));
    if !req.explanation.is_empty() && req.explanation != req.title {
        out.push_str(&format!("{}\n\n", req.explanation));
    }
    out.push_str("As the game wrote it:\n\n```\n");
    out.push_str(req.raw_error.trim());
    out.push_str("\n```\n\n");

    if !req.log_context.is_empty() {
        out.push_str("Lines around it in `console.txt`:\n\n```\n");
        for line in &req.log_context {
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out.push_str("```\n\n");
    }

    // ---------------------------------------------------------------- the setup
    out.push_str("## The setup\n\n");
    out.push_str(&format!(
        "- Game build: {}\n",
        game_build(paths).unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!("- Mods switched on: {}\n", active.len()));
    out.push_str(&format!("- Mods installed: {}\n\n", catalog.mods.len()));

    // ---------------------------------------------------------------- the mod
    match blamed {
        None => {
            if let Some(name) = &req.mod_name {
                out.push_str(&format!(
                    "## The mod\n\nThe game blamed **{}**, which is not in the Studio's catalogue \
                     any more (it may have been removed since).\n\n",
                    name
                ));
            } else {
                out.push_str("## The mod\n\nThe game did not name a mod on this line.\n\n");
            }
        }
        Some(m) => {
            out.push_str(&format!("## The mod: {}\n\n", m.name));
            out.push_str(&format!("- Mod id: `{}`\n", m.id));
            if let Some(v) = &m.info.mod_version {
                out.push_str(&format!("- Mod version: {}\n", v));
            }
            if let Some(a) = &m.info.author {
                out.push_str(&format!("- Author: {}\n", a));
            }
            match &m.source {
                crate::catalog::ModSource::Workshop { workshop_id } => {
                    out.push_str(&format!(
                        "- From the Steam Workshop, item {} (https://steamcommunity.com/sharedfiles/filedetails/?id={})\n",
                        workshop_id, workshop_id
                    ));
                }
                _ => out.push_str("- Installed locally, not from the Workshop\n"),
            }
            out.push_str(&format!("- Folder: `{}`\n", m.root_dir));
            if !m.version_folders.is_empty() {
                out.push_str(&format!(
                    "- Build folders it ships: {}{}\n",
                    m.version_folders.join(", "),
                    if m.has_common { " (plus common/)" } else { "" }
                ));
            }
            if let (Some(min), Some(max)) = (m.info.version_min, m.info.version_max) {
                out.push_str(&format!("- Declares it runs on builds {} to {}\n", min, max));
            }
            if !m.info.require.is_empty() {
                out.push_str(&format!("- Requires: {}\n", m.info.require.join(", ")));
            }
            out.push('\n');
        }
    }

    // ---------------------------------------------------------------- the file
    let found = blamed.zip(req.source_file.as_ref()).and_then(|(m, file)| {
        find_source_file(Path::new(&m.root_dir), file).map(|p| (m, p))
    });

    if let Some((_, file)) = &found {
        let relative = media_path(file).unwrap_or_else(|| file.to_string_lossy().to_string());
        out.push_str(&format!("## The file: `{}`\n\n", relative));

        // Another mod replacing the same file is the usual cause of "it worked on its own".
        let same_path: Vec<&str> = catalog
            .mods
            .iter()
            .filter(|other| {
                other.id != blamed.map(|b| b.id.clone()).unwrap_or_default()
                    && active.iter().any(|a| a.eq_ignore_ascii_case(&other.id))
                    && find_source_file(Path::new(&other.root_dir), &relative).is_some()
            })
            .map(|m| m.name.as_str())
            .collect();
        if !same_path.is_empty() {
            out.push_str(&format!(
                "Other mods that are switched on ship a file of the same name: {}. Only one copy \
                 of a given path is read by the game — the one loaded last.\n\n",
                same_path.join(", ")
            ));
        }

        let size = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
        if size > MAX_INLINE_BYTES {
            out.push_str(&format!(
                "The file is {} KB, too big to include here. It is at `{}`.\n\n",
                size / 1024,
                file.display()
            ));
        } else {
            match std::fs::read_to_string(file) {
                Ok(text) => {
                    out.push_str("```lua\n");
                    out.push_str(text.trim_end());
                    out.push_str("\n```\n\n");
                }
                Err(e) => out.push_str(&format!("The file could not be read: {}\n\n", e)),
            }
        }
    } else if let Some(file) = req.source_file.as_ref().filter(|f| !f.ends_with(".java")) {
        // A .java name is the game's own class the stack went through, never a mod's file.
        out.push_str(&format!(
            "## The file\n\nThe error names `{}`, which was not found inside the mod's folder.\n\n",
            file
        ));
    }

    // ---------------------------------------------------------------- translation texts
    // `Formatting "Sandbox_X_Y"` names the text, not the mod: the mod is whoever ships that key.
    let mut keys: Vec<String> = Vec::new();
    for line in std::iter::once(req.raw_error.as_str()).chain(req.log_context.iter().map(String::as_str)) {
        if let Some(key) = formatting_key(line) {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    if !keys.is_empty() {
        let found = find_translation_keys(&catalog, &active, &keys, &game_language(&paths.user_zomboid_dir));
        out.push_str("## The texts\n\n");
        out.push_str(
            "The game formats every translated text with Java's String.format, where `%` starts a \
             placeholder. A text with a bare `%` (\"50%\", \"(%)\") makes that throw; the game catches \
             it, logs this warning and shows the text unformatted, so nothing else breaks. The fix \
             is in the mod's translation file: write `%%` for a literal percent sign.\n\n",
        );
        for key in &keys {
            match found.iter().find(|f| &f.key == key) {
                Some(f) => out.push_str(&format!(
                    "- `{}`: **{}** (`{}`), `{}`:\n  ```\n  {}\n  ```\n",
                    key, f.mod_name, f.mod_id, f.file, f.line.trim()
                )),
                None => out.push_str(&format!("- `{}`: not found in the translation files of the mods switched on\n", key)),
            }
        }
        out.push('\n');
    }

    out.push_str("---\n\nWritten by PZ Mod Studio.\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(root: &Path) -> StudioPaths {
        StudioPaths {
            pz_install_dir: String::new(),
            workshop_dir: root.join("workshop").to_string_lossy().to_string(),
            user_zomboid_dir: root.to_string_lossy().to_string(),
            mod_list_ini_path: root.join("mods").join("default.txt").to_string_lossy().to_string(),
            carrier_workshop_id: None,
            is_valid: true,
        }
    }


    /// Writes a report from a real install and a real console.txt, and prints it.
    /// PZMS_PZ_DIR=... PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=... PZMS_CONSOLE=...
    /// cargo test report_from_a_real_error -- --ignored --nocapture
    #[test]
    #[ignore]
    fn report_from_a_real_error() {
        let (Ok(pz), Ok(workshop), Ok(zomboid), Ok(console)) = (
            std::env::var("PZMS_PZ_DIR"),
            std::env::var("PZMS_WORKSHOP_DIR"),
            std::env::var("PZMS_ZOMBOID_DIR"),
            std::env::var("PZMS_CONSOLE"),
        ) else {
            return;
        };
        let paths = StudioPaths {
            pz_install_dir: pz,
            workshop_dir: workshop,
            mod_list_ini_path: format!("{}/mods/default.txt", zomboid),
            user_zomboid_dir: zomboid,
            carrier_workshop_id: None,
            is_valid: true,
        };
        let text = std::fs::read_to_string(&console).expect("console.txt");
        let lines: Vec<&str> = text.lines().collect();
        let mut blame = crate::sandbox::ModBlame::default();
        let mut found = None;
        for (i, line) in lines.iter().enumerate() {
            blame.observe(line);
            if found.is_none() && (line.starts_with("WARN") || line.starts_with("ERROR")) {
                if let Some(name) = blame.of(line) {
                    found = Some((i, line.to_string(), name));
                }
            }
        }
        let Some((at, line, mod_name)) = found else {
            println!("no attributed error in that log");
            return;
        };
        let from = at.saturating_sub(10);
        let to = (at + 6).min(lines.len());
        let req = ErrorReportRequest {
            raw_error: line.clone(),
            title: format!("{} reported a problem", mod_name),
            explanation: line.split_once('>').map(|(_, r)| r.trim().to_string()).unwrap_or_default(),
            mod_name: Some(mod_name),
            source_file: line
                .split(")).")
                .nth(1)
                .and_then(|r| r.split('>').next())
                .map(|f| f.trim().to_string()),
            log_context: lines[from..to].iter().map(|l| l.to_string()).collect(),
        };
        let report = build(&paths, &req).unwrap();
        println!("----- {} characters -----\n{}", report.len(), report);
        assert!(report.contains("## What is being asked"));
    }

    #[test]
    fn a_report_carries_the_error_the_mod_and_the_code() {
        let root = std::env::temp_dir().join(format!("pzms-report-{}", std::process::id()));
        let mod_dir = root.join("mods").join("NaturesCall");
        let lua_dir = mod_dir.join("42").join("media").join("lua").join("client");
        std::fs::create_dir_all(&lua_dir).unwrap();
        std::fs::write(
            mod_dir.join("mod.info"),
            "name=Nature's Call\r\nid=NaturesCall\r\nmodversion=2.1\r\nauthor=Someone\r\n",
        )
        .unwrap();
        std::fs::write(lua_dir.join("NC_Main.lua"), "require(\"TimedActions/ISMoveablesAction\")\n").unwrap();
        std::fs::write(root.join("mods").join("default.txt"), "mods=NaturesCall\n").unwrap();
        std::fs::write(root.join("version.txt"), "42.20.4\n").unwrap();

        let req = ErrorReportRequest {
            raw_error: "WARN : Lua f:0 at Lua((MOD:Nature's Call)).NC_Main.lua> require(\"TimedActions/ISMoveablesAction\") failed".into(),
            title: "Nature's Call reported a problem".into(),
            explanation: "require(\"TimedActions/ISMoveablesAction\") failed".into(),
            mod_name: Some("Nature's Call".into()),
            source_file: Some("NC_Main.lua".into()),
            log_context: vec!["LOG : General f:0> loading".into()],
        };
        let report = build(&paths(&root), &req).unwrap();

        assert!(report.contains("## What is being asked"), "an agent is told what to do");
        assert!(report.contains("42.20.4"), "the game build is there");
        assert!(report.contains("Nature's Call"), "the mod is named");
        assert!(report.contains("`NaturesCall`"), "the mod id is there");
        assert!(report.contains("Mod version: 2.1"));
        assert!(report.contains("require(\"TimedActions/ISMoveablesAction\")"), "the code is included");
        assert!(report.contains("media/lua/client/NC_Main.lua"), "the path the game uses");
        assert!(report.contains("LOG : General f:0> loading"), "log context is kept");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn it_still_writes_a_report_when_the_mod_is_gone() {
        let root = std::env::temp_dir().join(format!("pzms-report-none-{}", std::process::id()));
        std::fs::create_dir_all(root.join("mods")).unwrap();
        let req = ErrorReportRequest {
            raw_error: "ERROR: General f:0> something broke".into(),
            title: "Something broke".into(),
            mod_name: Some("A Mod That Left".into()),
            ..Default::default()
        };
        let report = build(&paths(&root), &req).unwrap();
        assert!(report.contains("not in the Studio's catalogue"));
        assert!(report.contains("something broke"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The Translator `%` report on this machine: which mods ship the keys. PZMS_ZOMBOID_DIR.
    #[test]
    #[ignore]
    fn the_real_translator_report_names_the_mods() {
        if std::env::var("PZMS_ZOMBOID_DIR").is_err() { return; }
        let paths = crate::vfs::auto_detect_paths();
        let req = ErrorReportRequest {
            title: "Translator Format Exception (% character)".into(),
            explanation: String::new(),
            raw_error: "WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.UnknownFormatConversionException: Conversion = ')' at Formatter.parse(null:-1). Message: ERROR: Formatting \"Sandbox_NeatLockpicking_ReinforcedYellowHitValue\"".into(),
            log_context: vec![
                "WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.UnknownFormatConversionException: Conversion = '.' at Formatter.parse(null:-1). Message: ERROR: Formatting \"Sandbox_InteractiveTailoring_BonusXP_tooltip\"".into(),
                "WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.UnknownFormatConversionException: Conversion = ')' at Formatter.parse(null:-1). Message: ERROR: Formatting \"Sandbox_ReactiveSE_EventChance\"".into(),
            ],
            mod_name: None,
            source_file: Some("zombie/core/Translator.java".into()),
        };
        let text = build(&paths, &req).unwrap();
        let at = text.find("## The texts").expect("a texts section");
        println!("{}", &text[at..]);
        assert!(!text.contains("Translator.java"));
    }}
