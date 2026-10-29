//! Patching one mod's own file, for a bug no other mod is involved in.
//!
//! The Merger exists for two mods touching the same thing, and that is also the only way a file
//! could get into a package: the scan lists files more than one mod ships, and the package is
//! built from what you decide about them. A mod with a bug of its own — a line that throws on
//! every mouse click — had no way in at all, and the only advice the Studio could give was to
//! switch the mod off.
//!
//! The way in is the one the game already provides: a Lua file is read from whichever mod loads
//! last, and a Studio package loads last. A package carrying one of a mod's paths is read instead
//! of the mod's copy, for that file alone; the mod stays on, subscribed and whole. So any of its
//! text files can be opened here, edited, and saved into the package like any merge.
//!
//! What it costs is written down where the user can see it: the copy is frozen at today's version
//! of the mod, and Mod Health's "merged, but a mod changed since" is what catches the day the
//! author ships an update.

use serde::{Deserialize, Serialize};
use std::path::Path;

use walkdir::WalkDir;

/// One file of a mod, as the game would read it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFile {
    /// `media/lua/shared/aFancyHandwork.lua`: the path the package has to use to be read instead.
    pub relative_path: String,
    pub absolute_path: String,
    pub bytes: u64,
    /// Lua or a script; nothing else can be usefully edited here.
    pub kind: String,
}

/// The text files of one installed mod, from the folders the installed build reads.
pub fn files_of_mod(
    catalog: &crate::catalog::ModCatalog,
    mod_id: &str,
) -> Result<Vec<ModFile>, String> {
    let m = catalog
        .resolve(mod_id.trim())
        .ok_or_else(|| format!("{} is not installed.", mod_id))?;
    let game = catalog.game_version_int();

    let mut found: std::collections::BTreeMap<String, ModFile> = Default::default();
    for layer in crate::load_order::mod_info::media_layers_for_build(Path::new(&m.root_dir), game) {
        let media = layer.join("media");
        for entry in WalkDir::new(&media).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let kind = match entry.path().extension().and_then(|e| e.to_str()) {
                Some(e) if e.eq_ignore_ascii_case("lua") => "LUA",
                Some(e) if e.eq_ignore_ascii_case("txt") => "SCRIPT_TXT",
                _ => continue,
            };
            let Ok(rel) = entry.path().strip_prefix(&media) else { continue };
            let relative_path = format!("media/{}", rel.to_string_lossy().replace('\\', "/"));
            // A later layer's copy of the same path is the one the game reads.
            found.insert(
                relative_path.clone(),
                ModFile {
                    relative_path,
                    absolute_path: entry.path().to_string_lossy().to_string(),
                    bytes: entry.metadata().map(|md| md.len()).unwrap_or(0),
                    kind: kind.to_string(),
                },
            );
        }
    }
    Ok(found.into_values().collect())
}

/// The Lua files of a mod that ask for one module: `require("X")`, `require "X"`, `require 'X'`.
///
/// The game's `require("X") failed` line does not always say which file asked -- a
/// `KahluaThread.call` line names nobody -- and a fix that knows the new path still needs the
/// file to put it in. A mod ships a few hundred files; the ones that ask for X are the answer.
pub fn files_requiring(
    catalog: &crate::catalog::ModCatalog,
    mod_id: &str,
    module: &str,
) -> Result<Vec<ModFile>, String> {
    let wanted = module.trim();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    Ok(files_of_mod(catalog, mod_id)?
        .into_iter()
        .filter(|f| f.kind == "LUA")
        .filter(|f| {
            std::fs::read_to_string(&f.absolute_path)
                .map(|text| asks_for(&text, wanted))
                .unwrap_or(false)
        })
        .collect())
}

/// A file of some active mod that asks for a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskingFile {
    pub mod_id: String,
    pub mod_name: String,
    pub file: ModFile,
}

/// Every file of every active mod that asks for one module.
///
/// The game's `KahluaThread.call > require("X") failed` names no file and no mod; the Studio
/// guesses the mod from the lines just above, and the guess is wrong when files of two mods
/// load one after the other. The module is what the line says for certain, so that is what is
/// searched for, across the whole active list. Studio packages are skipped: their copies are
/// the player's, and a fix goes into the mod's file.
pub fn files_requiring_in_active_mods(
    catalog: &crate::catalog::ModCatalog,
    active_ids: &std::collections::HashSet<String>,
    module: &str,
) -> Vec<AskingFile> {
    let mut found = Vec::new();
    for m in catalog.mods.iter().filter(|m| m.shadowed_by.is_none()) {
        if m.id.starts_with("Z_PZModStudio_") || !active_ids.contains(&m.id) {
            continue;
        }
        for file in files_requiring(catalog, &m.id, module).unwrap_or_default() {
            found.push(AskingFile { mod_id: m.id.clone(), mod_name: m.name.clone(), file });
        }
    }
    found
}

/// Whether Lua source has a `require` of exactly this module, in any of the game's spellings.
pub fn asks_for(lua: &str, module: &str) -> bool {
    lua.match_indices("require").any(|(at, _)| {
        let rest = lua[at + "require".len()..].trim_start();
        let rest = rest.strip_prefix('(').map(str::trim_start).unwrap_or(rest);
        let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else { return false };
        rest[1..].strip_prefix(module).and_then(|after| after.chars().next()) == Some(quote)
    })
}

/// One mod's file as something the Merger can edit and save into the package.
///
/// It is the same shape as a conflict between two mods, with one of them: what the mod says on
/// the left, the proposal (its own content, to be edited) below, and a path in the package.
pub fn conflict_for_mod_file(
    catalog: &crate::catalog::ModCatalog,
    paths: &crate::vfs::StudioPaths,
    mod_id: &str,
    relative_path: &str,
) -> Result<crate::vfs::VfsConflictRaw, String> {
    let m = catalog
        .resolve(mod_id.trim())
        .ok_or_else(|| format!("{} is not installed.", mod_id))?;
    let wanted = relative_path.replace('\\', "/");
    let file = files_of_mod(catalog, mod_id)?
        .into_iter()
        .find(|f| f.relative_path == wanted)
        .ok_or_else(|| format!("{} does not ship {}.", m.name, relative_path))?;

    let content = std::fs::read_to_string(&file.absolute_path)
        .map_err(|e| format!("Cannot read {}: {}", file.absolute_path, e))?;

    // The base game's own copy, when it has one: patching a mod's override of a vanilla file is
    // worth seeing against the original.
    let vanilla = Path::new(&paths.pz_install_dir).join(&wanted);
    let base_content = std::fs::read_to_string(&vanilla)
        .unwrap_or_else(|_| "-- vanilla file not present for this path".to_string());
    let lines = content.lines().count().max(1);

    Ok(crate::vfs::VfsConflictRaw {
        id: format!("modfile_{}", wanted),
        relative_path: wanted.clone(),
        file_type: file.kind.clone(),
        start_line: 1,
        end_line: lines,
        conflict_line: 1,
        total_file_lines: lines,
        base_content,
        competing_files: vec![crate::vfs::CompetingModFileRaw {
            mod_id: m.id.clone(),
            mod_name: m.name.clone(),
            absolute_path: file.absolute_path.clone(),
            content: content.clone(),
        }],
        merged_output: content.clone(),
        auto_ast_output: content,
        is_identical_noise: false,
        status: None,
        mod_changes: Vec::new(),
        collisions: Vec::new(),
        strategy: format!(
            "Only {} ships this file, so nothing is being merged: this is its own code, to patch. \
             What you save goes into the package, which loads last, so the game reads it instead \
             of {}'s copy — for this file only. The mod stays on, and everything else in it is \
             still its own. Your copy stays as it is today: when {} updates, Mod Health says the \
             package no longer matches it.",
            m.name, m.name, m.name
        ),
        auto_mergeable: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_require_is_found_in_every_spelling_and_not_in_a_longer_name() {
        assert!(asks_for("require \"TimedActions/ISPathFindAction\"", "TimedActions/ISPathFindAction"));
        assert!(asks_for("local a = require('TimedActions/ISPathFindAction')", "TimedActions/ISPathFindAction"));
        assert!(asks_for("require( \"TimedActions/ISPathFindAction\" )", "TimedActions/ISPathFindAction"));
        assert!(!asks_for("require \"TimedActions/ISPathFindActionX\"", "TimedActions/ISPathFindAction"));
        assert!(!asks_for("-- nothing here", "TimedActions/ISPathFindAction"));
        assert!(!asks_for("require \"Other\"", "TimedActions/ISPathFindAction"));
    }

    /// Lists a real mod's files and opens one, which is what the Merger's picker does.
    /// PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=... PZMS_PZ_DIR=... PZMS_MOD_ID=FancyHandworkB42_19
    /// cargo test --release a_real_mods_files -- --ignored --nocapture
    #[test]
    #[ignore]
    fn a_real_mods_files() {
        let (Ok(workshop), Ok(zomboid), Ok(pz), Ok(mod_id)) = (
            std::env::var("PZMS_WORKSHOP_DIR"),
            std::env::var("PZMS_ZOMBOID_DIR"),
            std::env::var("PZMS_PZ_DIR"),
            std::env::var("PZMS_MOD_ID"),
        ) else {
            return;
        };
        let catalog = crate::catalog::build_catalog(&workshop, &zomboid, None);
        let paths = crate::vfs::StudioPaths {
            pz_install_dir: pz,
            workshop_dir: workshop,
            mod_list_ini_path: format!("{}/mods/default.txt", zomboid),
            user_zomboid_dir: zomboid,
            carrier_workshop_id: None,
            is_valid: true,
        };
        let files = files_of_mod(&catalog, &mod_id).expect("the mod is installed");
        println!("{} has {} editable files:", mod_id, files.len());
        for f in files.iter().take(8) {
            println!("  {:>7} bytes  {}", f.bytes, f.relative_path);
        }
        let wanted = files
            .iter()
            .find(|f| f.relative_path.ends_with("aFancyHandwork.lua"))
            .or_else(|| files.first())
            .expect("at least one file");
        let conflict = conflict_for_mod_file(&catalog, &paths, &mod_id, &wanted.relative_path)
            .expect("it opens");
        println!("\nopened {} ({} lines)", conflict.relative_path, conflict.total_file_lines);
        println!("{}", conflict.strategy);
        assert_eq!(conflict.competing_files.len(), 1);
        assert!(!conflict.merged_output.is_empty());
    }
}
