//! "Make a compatible copy": adapts a mod that the installed build will not load (B41-only, a
//! too-low versionMax, only folders for newer builds) by copying it to `Zomboid/mods` and fixing
//! what can be fixed safely. The original (usually a Workshop item) is never modified.
//!
//! The copy keeps the original mod id, so mods that require it keep working. Because the game
//! normally prefers Workshop copies over `Zomboid/mods` (first copy of an id wins, search order
//! "workshop,steam,mods"), the Studio starts the game with `-modfolders mods,workshop,steam`
//! while compatible copies exist, and the catalog follows the same order.

use crate::catalog::modinfo::{format_game_version, parse_game_mod_info};
use crate::load_order::mod_info::{get_existing_user_zomboid_dirs, select_mod_info_for_build, version_dir_int};
use crate::origins::{self, ModOrigin};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub mod recipes;

/// Launch argument that makes the game read Zomboid/mods before Workshop folders.
pub const LOCAL_FIRST_ARG: [&str; 2] = ["-modfolders", "mods,workshop,steam"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatResult {
    pub mod_id: String,
    pub folder: String,
    pub destination: String,
    /// What was changed to make it load.
    pub changes: Vec<String>,
    /// What could not be fixed automatically and may still break in game.
    pub warnings: Vec<String>,
}

/// True when compatible copies exist in this Zomboid folder, so the game must read
/// Zomboid/mods first (see `LOCAL_FIRST_ARG`).
pub fn local_first(zomboid_dir: &Path) -> bool {
    origins::load(zomboid_dir)
        .iter()
        .any(|(folder, r)| matches!(r.origin, ModOrigin::Compat { .. }) && zomboid_dir.join("mods").join(folder).is_dir())
}

/// Launch arguments to add for this Zomboid folder.
pub fn launch_args(user_zomboid_dir: &str) -> Vec<String> {
    match get_existing_user_zomboid_dirs(user_zomboid_dir).first() {
        Some(dir) if local_first(dir) || crate::studio_link::local_link_must_win(dir) => LOCAL_FIRST_ARG.iter().map(|s| s.to_string()).collect(),
        _ => Vec::new(),
    }
}

/// Rewrites (or adds) `key=value` in a mod.info, keeping every other line as it is.
fn set_mod_info_value(path: &Path, key: &str, value: &str) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let prefix = format!("{}=", key);
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            if line.starts_with(&prefix) {
                found = true;
                format!("{}{}", prefix, value)
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(format!("{}{}", prefix, value));
    }
    fs::write(path, lines.join(eol) + eol).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

fn files_under(dir: &Path, ext: &str) -> Vec<PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case(ext)))
        .map(|e| e.into_path())
        .collect()
}

/// Every item type Build 41 could declare, and what Build 42 calls it. Checked against the
/// installed game by `the_item_types_are_the_ones_the_game_uses`: Build 42 writes `ItemType` on
/// all 5105 of its own items and `Type` on none of them, with these namespaced values.
const ITEM_TYPES_B41_TO_B42: [(&str, &str); 14] = [
    ("Normal", "base:normal"),
    ("Food", "base:food"),
    ("Clothing", "base:clothing"),
    ("Weapon", "base:weapon"),
    ("WeaponPart", "base:weaponpart"),
    ("Container", "base:container"),
    ("Drainable", "base:drainable"),
    ("Literature", "base:literature"),
    ("Key", "base:key"),
    ("AlarmClock", "base:alarmclock"),
    ("AlarmClockClothing", "base:alarmclockclothing"),
    ("Radio", "base:radio"),
    ("Moveable", "base:moveable"),
    ("Map", "base:map"),
];

/// Rewrites Build 41 `Type =` lines inside `item` blocks as Build 42 `ItemType =`, in place.
///
/// The file is edited line by line rather than reparsed and written out, so a mod author's own
/// formatting, comments and ordering survive; only the lines that have to change, change. Brace
/// depth says which block a line is in, because `Type =` means something else outside an `item`
/// (a vehicle part, a fixing) and must be left alone there. A value Build 41 never had is left
/// alone too, and reported: guessing a type would be worse than the mod not loading it.
fn convert_item_types(roots: &[PathBuf]) -> (Vec<String>, Vec<String>) {
    let mut changed_items = 0usize;
    let mut changed_files = 0usize;
    let mut unknown: Vec<String> = Vec::new();

    for root in roots {
        for script in files_under(&root.join("media").join("scripts"), "txt") {
            let Ok(text) = fs::read_to_string(&script) else { continue };
            let mut out = String::with_capacity(text.len());
            let mut here = 0usize;
            // The kind of block each open brace belongs to, innermost last.
            let mut stack: Vec<String> = Vec::new();
            // What has been read since the last `{`, `}` or `,`: a header, or a field. Kept as
            // written, so whatever is passed through is byte for byte what the author wrote.
            let mut statement = String::new();
            // The same text with the comments left out, which is what a block header and a field
            // name are read from: a comment sitting above a header is not part of it.
            let mut code = String::new();
            // A comment can hold anything, braces included, so it counts as text and never as
            // code. A field with a comment inside it is passed through rather than rewritten.
            let mut commented = false;
            let mut in_block_comment = false;
            let mut in_line_comment = false;

            // Walked character by character, so a whole item written on one line is seen too.
            let chars: Vec<char> = text.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let c = chars[i];
                let next = chars.get(i + 1).copied();

                if in_block_comment {
                    statement.push(c);
                    if c == '*' && next == Some('/') {
                        statement.push('/');
                        i += 2;
                        in_block_comment = false;
                        continue;
                    }
                    i += 1;
                    continue;
                }
                if in_line_comment {
                    statement.push(c);
                    if c == '\n' {
                        in_line_comment = false;
                    }
                    i += 1;
                    continue;
                }
                if c == '/' && (next == Some('*') || next == Some('/')) {
                    in_block_comment = next == Some('*');
                    in_line_comment = next == Some('/');
                    commented = true;
                    statement.push(c);
                    statement.push(next.unwrap());
                    i += 2;
                    continue;
                }

                match c {
                    '{' => {
                        let head = code.split_whitespace().next().unwrap_or_default().to_string();
                        stack.push(head);
                        out.push_str(&statement);
                        out.push('{');
                        statement.clear();
                        code.clear();
                        commented = false;
                    }
                    '}' | ',' | ';' => {
                        let in_item = stack.last().is_some_and(|k| k == "item");
                        match convert_type_statement(&statement).filter(|_| in_item && !commented) {
                            Some((rewritten, Some(_))) => {
                                here += 1;
                                out.push_str(&rewritten);
                            }
                            Some((kept, None)) => {
                                unknown.push(statement.trim().to_string());
                                out.push_str(&kept);
                            }
                            None => out.push_str(&statement),
                        }
                        statement.clear();
                        code.clear();
                        commented = false;
                        if c == '}' {
                            stack.pop();
                        }
                        out.push(c);
                    }
                    other => {
                        statement.push(other);
                        code.push(other);
                    }
                }
                i += 1;
            }
            out.push_str(&statement);

            if here > 0 {
                // Everything but the converted fields is passed through untouched, line endings
                // and all, because the file was walked as it is rather than reassembled.
                if fs::write(&script, out).is_ok() {
                    changed_items += here;
                    changed_files += 1;
                }
            }
        }
    }

    let mut changes = Vec::new();
    if changed_items > 0 {
        changes.push(format!(
            "Rewrote {} item(s) in {} script file(s) from the Build 41 \"Type =\" field to Build 42's \
             \"ItemType =\" with its namespaced value (Clothing -> base:clothing).",
            changed_items, changed_files
        ));
    }
    let mut warnings = Vec::new();
    if !unknown.is_empty() {
        unknown.sort();
        unknown.dedup();
        warnings.push(format!(
            "{} item type(s) were left as they are because Build 41 had no such type, so there is \
             nothing to map them to: {}.",
            unknown.len(),
            unknown.join(", ")
        ));
    }
    (changes, warnings)
}

/// One line: `Some((rewritten, Some(b42 value)))` when it is a `Type =` line that maps,
/// `Some((line, None))` when it is one that does not, `None` when it is not a `Type =` line.
fn convert_type_statement(statement: &str) -> Option<(String, Option<&'static str>)> {
    let trimmed = statement.trim_start();
    let lead = &statement[..statement.len() - trimmed.len()];
    // "TypeSomething = x" is a different field: the `=` has to follow "Type" itself.
    let value = trimmed.strip_prefix("Type")?.trim_start().strip_prefix('=')?.trim();
    if value.is_empty() {
        return None;
    }
    match ITEM_TYPES_B41_TO_B42.iter().find(|(b41, _)| b41.eq_ignore_ascii_case(value)) {
        Some((_, b42)) => Some((format!("{}ItemType = {}", lead, b42), Some(*b42))),
        None => Some((statement.to_string(), None)),
    }
}

/// Things in the folders the game will read that typically break when a B41 mod runs on B42.
fn scan_for_problems(roots: &[PathBuf], target_is_b42: bool) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut old_recipes = 0usize;
    let mut old_item_fields = 0usize;
    let mut txt_translations = 0usize;
    let mut lua_errors: Vec<String> = Vec::new();
    for root in roots {
        let media = root.join("media");
        for script in files_under(&media.join("scripts"), "txt") {
            let text = String::from_utf8_lossy(&fs::read(&script).unwrap_or_default()).to_string();
            for line in text.lines() {
                let t = line.trim_start();
                if t.starts_with("recipe ") || t.starts_with("recipe\t") {
                    old_recipes += 1;
                }
                if t.starts_with("Type") && t[4..].trim_start().starts_with('=') {
                    old_item_fields += 1;
                }
            }
        }
        let translate = media.join("lua").join("shared").join("Translate");
        txt_translations += files_under(&translate, "txt")
            .iter()
            .filter(|t| json_sibling(t).map(|j| !j.is_file()).unwrap_or(true))
            .count();
        for lua in files_under(&media.join("lua"), "lua") {
            let text = String::from_utf8_lossy(&fs::read(&lua).unwrap_or_default()).to_string();
            if !crate::diff_engine::lua::validate_lua_syntax(&text).is_valid {
                lua_errors.push(lua.file_name().unwrap_or_default().to_string_lossy().to_string());
            }
        }
    }
    if target_is_b42 {
        if old_recipes > 0 {
            warnings.push(format!(
                "{} old-style recipe(s): Build 42 replaced crafting (craftRecipe), so these will not work until ported by hand.",
                old_recipes
            ));
        }
        if old_item_fields > 0 {
            warnings.push(format!(
                "{} item(s) use the Build 41 \"Type =\" field; Build 42 uses \"ItemType =\". Those items may not appear or may error.",
                old_item_fields
            ));
        }
        if txt_translations > 0 {
            warnings.push(format!(
                "{} translation file(s) are .txt; Build 42.15+ reads JSON translations, so texts may show as raw keys.",
                txt_translations
            ));
        }
    }
    if !lua_errors.is_empty() {
        warnings.push(format!("Lua syntax errors in: {}.", lua_errors.join(", ")));
    }
    warnings
}

/// Creates (or, with `replace_existing`, recreates) a compatible copy of the mod folder
/// `source_root` in `<Zomboid>/mods`, adapted to `game_version` (major*1000+minor).
pub fn make_compatible_copy(
    source_root: &Path,
    user_zomboid_dir: &str,
    game_version: u32,
    workshop_id: Option<String>,
    replace_existing: bool,
) -> Result<CompatResult, String> {
    make_compatible_copy_with_index(source_root, user_zomboid_dir, game_version, workshop_id, replace_existing, &[])
}

/// As `make_compatible_copy`, with the module index of the game and the active mods, so the
/// copy's `require` paths that Build 42 moved are rewritten up front instead of failing at boot.
pub fn make_compatible_copy_with_index(
    source_root: &Path,
    user_zomboid_dir: &str,
    game_version: u32,
    workshop_id: Option<String>,
    replace_existing: bool,
    index: &[(String, crate::sandbox::requires::ModuleCandidate)],
) -> Result<CompatResult, String> {
    let zomboid = get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;
    let folder = source_root.file_name().ok_or("Invalid mod folder")?.to_string_lossy().to_string();
    let dest = zomboid.join("mods").join(&folder);
    if source_root.starts_with(zomboid.join("mods")) {
        return Err("This mod is already in Zomboid/mods; edit it there instead of copying it.".to_string());
    }

    if dest.exists() {
        let is_compat = matches!(origins::load(&zomboid).get(&folder).map(|r| &r.origin), Some(ModOrigin::Compat { .. }));
        if !is_compat || !replace_existing {
            return Err(format!(
                "Zomboid/mods/{} already exists{}.",
                folder,
                if is_compat { " (an earlier compatible copy): replace it to make a fresh one" } else { " and is not a compatible copy made by the Studio" }
            ));
        }
        let backup = zomboid.join("PZModStudio_Backups").join("mods").join(format!("{}-{}", folder, chrono::Local::now().format("%Y%m%d-%H%M%S")));
        fs::create_dir_all(backup.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::rename(&dest, &backup).map_err(|e| format!("Cannot move the old copy aside: {}", e))?;
    }

    crate::mod_install::copy_tree(source_root, &dest).map_err(|e| {
        let _ = fs::remove_dir_all(&dest);
        format!("Copying the mod failed: {}", e)
    })?;

    let result = adapt(&dest, game_version, index);
    let (changes, warnings, mod_id) = match result {
        Ok(v) => v,
        Err(e) => {
            let _ = fs::remove_dir_all(&dest);
            return Err(e);
        }
    };

    origins::record(
        &zomboid,
        std::slice::from_ref(&folder),
        &ModOrigin::Compat { workshop_id, source_dir: source_root.to_string_lossy().to_string(), changes: changes.clone() },
    )?;

    Ok(CompatResult { mod_id, folder, destination: dest.to_string_lossy().to_string(), changes, warnings })
}

/// `.../Translate/EN/ItemName_EN.txt` -> `.../Translate/EN/ItemName.json`: the Build 42 file for
/// the same category, beside it. None for a file not named `<Category>_<LANG>.txt` in a `<LANG>` folder.
fn json_sibling(txt: &Path) -> Option<PathBuf> {
    let lang = txt.parent()?.file_name()?.to_str()?;
    let stem = txt.file_stem()?.to_str()?;
    let category = stem.strip_suffix(&format!("_{}", lang))?;
    if category.is_empty() {
        return None;
    }
    Some(txt.with_file_name(format!("{}.json", category)))
}

/// The Build 41 translation table: `Category_LANG = { key = "value", ... }`, with `--` comments,
/// optional trailing commas and `"a" .. "b"` joins. Returns the pairs in file order.
pub fn parse_b41_translation(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("--") || line.ends_with('{') || line == "}" {
            continue;
        }
        let Some((key, rest)) = line.split_once('=') else { continue };
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == ' ') {
            continue;
        }
        // Every quoted string on the line, joined: `"a" .. "b"` is one value.
        let mut value = String::new();
        let mut chars = rest.chars().peekable();
        let mut found = false;
        while let Some(c) = chars.next() {
            if c != '"' {
                continue;
            }
            found = true;
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        if let Some(n) = chars.next() {
                            value.push(match n {
                                'n' => '\n',
                                't' => '\t',
                                other => other,
                            });
                        }
                    }
                    '"' => break,
                    other => value.push(other),
                }
            }
        }
        if found {
            pairs.push((key.trim().to_string(), value));
        }
    }
    pairs
}

/// Categories whose Build 42 file drops the `Category_` prefix from the key: in the game's own
/// ItemName.json the key is `Base.Axe`, in Recipes.json `AddACigarette`. Every other category
/// keeps the whole key (`ContextMenu_Destroy`). Read from the vanilla files of 42.21.
const PREFIX_DROPPED: [&str; 3] = ["ItemName", "Recipe", "EvolvedRecipeName"];

/// The JSON keys one Build 41 entry becomes. For the categories that drop the prefix, both the
/// bare key the game's own files use and the prefixed one are written: a key the game never
/// looks up costs nothing, and some mods ship the prefixed form and are read.
fn b42_keys(category: &str, key: &str) -> Vec<String> {
    let prefix = PREFIX_DROPPED.iter().find(|p| category.starts_with(*p)).map(|p| format!("{}_", p));
    match prefix {
        Some(prefix) => match key.strip_prefix(&prefix) {
            Some(bare) => vec![bare.to_string(), key.to_string()],
            None => vec![key.to_string(), format!("{}{}", prefix, key)],
        },
        None => vec![key.to_string()],
    }
}

/// The Build 42 file for one Build 41 table.
pub fn b41_table_to_json(category: &str, text: &str) -> String {
    let mut out = String::from("{\n");
    let mut first = true;
    for (key, value) in parse_b41_translation(text) {
        for k in b42_keys(category, &key) {
            if !first {
                out.push_str(",\n");
            }
            first = false;
            out.push_str("    ");
            out.push_str(&serde_json::to_string(&k).unwrap_or_default());
            out.push_str(": ");
            out.push_str(&serde_json::to_string(&value).unwrap_or_default());
        }
    }
    out.push_str("\n}\n");
    out
}

/// Writes a Build 42 JSON beside every Build 41 `.txt` translation that has none yet. The `.txt`
/// stays: Build 42 ignores it, and the copy is meant to be removable.
fn convert_translations(roots: &[PathBuf]) -> (Vec<String>, Vec<String>) {
    let mut written = 0usize;
    let mut categories: std::collections::BTreeSet<String> = Default::default();
    let mut warnings = Vec::new();
    for root in roots {
        let translate = root.join("media").join("lua").join("shared").join("Translate");
        for txt in files_under(&translate, "txt") {
            let Some(json) = json_sibling(&txt) else { continue };
            if json.is_file() {
                continue;
            }
            let Ok(bytes) = fs::read(&txt) else { continue };
            let text = match String::from_utf8(bytes) {
                Ok(t) => t,
                Err(_) => {
                    warnings.push(format!(
                        "{} is not UTF-8 (Build 41 used a legacy encoding for some languages); left as it is.",
                        txt.strip_prefix(root).unwrap_or(&txt).display()
                    ));
                    continue;
                }
            };
            let category = json.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let content = b41_table_to_json(&category, &text);
            if content.trim() == "{\n}" || parse_b41_translation(&text).is_empty() {
                continue;
            }
            if fs::write(&json, content).is_ok() {
                written += 1;
                categories.insert(category);
            }
        }
    }
    let mut changes = Vec::new();
    if written > 0 {
        changes.push(format!(
            "Wrote {} Build 42 JSON translation file(s) ({}) from the Build 41 .txt ones; ItemName and Recipe keys are written both bare and prefixed, as the game's own files and the mods disagree.",
            written,
            categories.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    (changes, warnings)
}

/// Every `require` in a Lua file, as (module, the literal span to replace).
fn requires_in(lua: &str) -> Vec<(String, std::ops::Range<usize>)> {
    let mut found = Vec::new();
    for (at, _) in lua.match_indices("require") {
        let after = &lua[at + "require".len()..];
        let trimmed = after.trim_start();
        let mut skip = after.len() - trimmed.len();
        let trimmed = match trimmed.strip_prefix('(') {
            Some(rest) => {
                skip += 1 + (rest.len() - rest.trim_start().len());
                rest.trim_start()
            }
            None => trimmed,
        };
        let Some(quote) = trimmed.chars().next().filter(|c| *c == '"' || *c == '\'') else { continue };
        let body = &trimmed[1..];
        let Some(end) = body.find(quote) else { continue };
        let module = &body[..end];
        if module.is_empty() || module.contains('\n') {
            continue;
        }
        let start = at + "require".len() + skip + 1;
        found.push((module.to_string(), start..start + end));
    }
    found
}

/// Rewrites `require` paths that Build 42 moved, in every Lua file of the copy, when the module
/// index knows exactly one other place for the module. A module at the asked path is left (the
/// failure, if any, is about load order, not the path); one found nowhere is named in a warning;
/// one found at several paths is left and named, since a guess would be a lie.
fn fix_require_paths(roots: &[PathBuf], index: &[(String, crate::sandbox::requires::ModuleCandidate)]) -> (Vec<String>, Vec<String>) {
    use crate::sandbox::requires::answer_for;
    let mut rewritten: Vec<String> = Vec::new();
    let mut nowhere: std::collections::BTreeSet<String> = Default::default();
    let mut several: std::collections::BTreeSet<String> = Default::default();
    let mut files = 0usize;
    for root in roots {
        for lua in files_under(&root.join("media").join("lua"), "lua") {
            let Ok(text) = fs::read_to_string(&lua) else { continue };
            let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
            for (module, span) in requires_in(&text) {
                let answer = answer_for(&module, index);
                let Some(first) = answer.candidates.first() else {
                    nowhere.insert(module.clone());
                    continue;
                };
                if first.same_path {
                    continue;
                }
                let mut paths: Vec<&str> = answer.candidates.iter().map(|c| c.module_path.as_str()).collect();
                paths.sort();
                paths.dedup();
                if paths.len() != 1 {
                    several.insert(module.clone());
                    continue;
                }
                rewritten.push(format!("{} -> {}", module, paths[0]));
                edits.push((span, paths[0].to_string()));
            }
            if edits.is_empty() {
                continue;
            }
            let mut out = text.clone();
            for (span, to) in edits.into_iter().rev() {
                out.replace_range(span, &to);
            }
            if fs::write(&lua, out).is_ok() {
                files += 1;
            }
        }
    }
    let mut changes = Vec::new();
    if !rewritten.is_empty() {
        rewritten.sort();
        rewritten.dedup();
        changes.push(format!(
            "Rewrote {} require path(s) Build 42 moved, in {} file(s): {}.",
            rewritten.len(), files, rewritten.join(", ")
        ));
    }
    let mut warnings = Vec::new();
    if !nowhere.is_empty() {
        warnings.push(format!(
            "require() of a module that is nowhere (game or active mods), left as is: {}. The mod may need another mod, or this is a bug for its author.",
            nowhere.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if !several.is_empty() {
        warnings.push(format!(
            "require() of a module found at several paths, left as is: {}.",
            several.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    (changes, warnings)
}

/// Adapts the copied folder in place. Returns (changes, warnings, mod id).
fn adapt(dest: &Path, game_version: u32, index: &[(String, crate::sandbox::requires::ModuleCandidate)]) -> Result<(Vec<String>, Vec<String>, String), String> {
    let mut changes = Vec::new();
    let target = format_game_version(game_version);
    let is_b42 = game_version >= 42_000;

    // 1. Make sure the build has a folder (and mod.info) it reads.
    if select_mod_info_for_build(dest, Some(game_version)).is_none() {
        let root_info = dest.join("mod.info");
        if is_b42 && root_info.is_file() {
            // Build 41 layout: B42 needs common/ + a version folder.
            let v42 = dest.join("42");
            fs::create_dir_all(&v42).map_err(|e| e.to_string())?;
            fs::copy(&root_info, v42.join("mod.info")).map_err(|e| e.to_string())?;
            if dest.join("media").is_dir() {
                crate::mod_install::copy_tree(&dest.join("media"), &v42.join("media")).map_err(|e| e.to_string())?;
            }
            for extra in ["poster.png", "icon.png", "preview.png"] {
                if dest.join(extra).is_file() {
                    let _ = fs::copy(dest.join(extra), v42.join(extra));
                }
            }
            fs::create_dir_all(dest.join("common")).map_err(|e| e.to_string())?;
            changes.push("Created the Build 42 layout (42/ and common/) from the Build 41 files.".to_string());
        } else {
            // Only folders for newer builds: offer the oldest of them to this build.
            let oldest = fs::read_dir(dest)
                .map_err(|e| e.to_string())?
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let v = version_dir_int(&name)?;
                    (v > game_version && e.path().join("mod.info").is_file()).then_some((v, name))
                })
                .min();
            let Some((_, name)) = oldest else {
                return Err("No mod.info found that could be adapted.".to_string());
            };
            let new_dir = dest.join(&target);
            crate::mod_install::copy_tree(&dest.join(&name), &new_dir).map_err(|e| e.to_string())?;
            changes.push(format!("This mod only had files for build {}: made them available as {}/ for your build.", name, target));
        }
    }

    let info_path = select_mod_info_for_build(dest, Some(game_version)).ok_or("The adapted copy still has no usable mod.info.")?;
    let info = parse_game_mod_info(&String::from_utf8_lossy(&fs::read(&info_path).map_err(|e| e.to_string())?));
    let mod_id = info.id.clone().ok_or("The mod.info has no id= line, so the game cannot use this mod.")?;

    // 2. Version range the game checks (major.minor only).
    let text = String::from_utf8_lossy(&fs::read(&info_path).unwrap_or_default()).to_string();
    for (key, current) in [("versionMax", info.version_max), ("versionMin", info.version_min)] {
        let raw = text.lines().find_map(|l| l.strip_prefix(&format!("{}=", key))).map(|v| v.trim().to_string());
        let Some(raw) = raw.filter(|r| !r.is_empty()) else { continue };
        let needs_fix = match (key, current) {
            (_, None) => true, // invalid string like "42": the game rejects the whole mod.info
            ("versionMax", Some(v)) => v < game_version,
            (_, Some(v)) => v > game_version,
        };
        if needs_fix {
            set_mod_info_value(&info_path, key, &target)?;
            changes.push(format!(
                "{}={} -> {} (the game only compares major.minor, so {} blocked build {}).",
                key, raw, target, raw, target
            ));
        }
    }
    if info.invalid_reason.is_some() && changes.iter().all(|c| !c.starts_with("version")) {
        changes.push("Fixed an invalid version value in mod.info.".to_string());
    }
    if changes.is_empty() {
        changes.push("Nothing needed changing: the copy is identical to the original.".to_string());
    }

    // 3. Build 41 fields the Build 42 parser does not know.
    let mut roots = vec![info_path.parent().unwrap().to_path_buf()];
    if dest.join("common").is_dir() {
        roots.push(dest.join("common"));
    }
    let mut warnings = Vec::new();
    if is_b42 {
        let (converted, left_alone) = convert_item_types(&roots);
        changes.extend(converted);
        warnings.extend(left_alone);
        let (made, trouble) = convert_translations(&roots);
        changes.extend(made);
        warnings.extend(trouble);
        // After the translations: the recipe names are looked up in the EN JSON just written.
        let (made, trouble) = recipes::convert_recipes(&roots);
        changes.extend(made);
        warnings.extend(trouble);
        if !index.is_empty() {
            // The copy's own modules count too: a mod requires its own files as often as the game's.
            let mut own = index.to_vec();
            for root in &roots {
                crate::sandbox::requires::index_lua(&root.join("media").join("lua"), &mod_id, &mut own);
            }
            let (moved, nowhere) = fix_require_paths(&roots, &own);
            changes.extend(moved);
            warnings.extend(nowhere);
        }
    }

    // 4. What may still break.
    warnings.extend(scan_for_problems(&roots, is_b42));
    if changes.len() > 1 {
        // "Nothing needed changing" is only true when nothing else is listed.
        changes.retain(|c| !c.starts_with("Nothing needed changing"));
    }
    Ok((changes, warnings, mod_id))
}

/// Removes a compatible copy (moved to Zomboid/PZModStudio_Backups/mods, not deleted).
pub fn remove_compatible_copy(user_zomboid_dir: &str, folder: &str) -> Result<(), String> {
    let zomboid = get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next().ok_or("Zomboid folder not found")?;
    if !matches!(origins::load(&zomboid).get(folder).map(|r| &r.origin), Some(ModOrigin::Compat { .. })) {
        return Err(format!("{} is not a compatible copy made by the Studio.", folder));
    }
    let dir = zomboid.join("mods").join(folder);
    if dir.is_dir() {
        let backup = zomboid.join("PZModStudio_Backups").join("mods").join(format!("{}-removed-{}", folder, chrono::Local::now().format("%Y%m%d-%H%M%S")));
        fs::create_dir_all(backup.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::rename(&dir, &backup).map_err(|e| format!("Cannot move the copy away: {}", e))?;
    }
    origins::forget(&zomboid, folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn setup(tag: &str) -> (Tmp, PathBuf, String) {
        let t = Tmp(std::env::temp_dir().join(format!("pzms-compat-{}-{}", tag, std::process::id())));
        let _ = fs::remove_dir_all(&t.0);
        let ws = t.0.join("workshop/108600/123/mods");
        fs::create_dir_all(&ws).unwrap();
        fs::create_dir_all(t.0.join("Zomboid/mods")).unwrap();
        let z = t.0.join("Zomboid").to_string_lossy().to_string();
        (t, ws, z)
    }

    #[test]
    fn moved_require_paths_are_rewritten_and_the_rest_left_alone() {
        let (_t, ws, z) = setup("requires");
        let src = ws.join("Asker");
        fs::create_dir_all(src.join("media/lua/client")).unwrap();
        fs::write(src.join("mod.info"), "name=Asker\nid=Asker\n").unwrap();
        fs::write(
            src.join("media/lua/client/Asker.lua"),
            "require \"Vehicle/ISVehiclePartMenu\"\nlocal u = require('Vehicles/VehicleUtils')\nrequire(\"ISUI/ISPanel\")\nrequire \"AskerHelper\"\n",
        )
        .unwrap();
        fs::write(src.join("media/lua/client/AskerHelper.lua"), "return {}").unwrap();
        let cand = |path: &str| crate::sandbox::requires::ModuleCandidate {
            module_path: path.to_string(), phase: "client".to_string(), owner: "the game".to_string(), same_path: false,
        };
        // Build 42 keeps ISPanel where it was and moved ISVehiclePartMenu; VehicleUtils is gone.
        let index = vec![
            ("isvehiclepartmenu".to_string(), cand("Vehicles/ISUI/ISVehiclePartMenu")),
            ("ispanel".to_string(), cand("ISUI/ISPanel")),
        ];
        let result = make_compatible_copy_with_index(&src, &z, 42_021, None, false, &index).expect("copied");
        let lua = fs::read_to_string(Path::new(&result.destination).join("42/media/lua/client/Asker.lua")).unwrap();
        assert!(lua.contains("require \"Vehicles/ISUI/ISVehiclePartMenu\""), "{}", lua);
        assert!(lua.contains("require(\"ISUI/ISPanel\")"), "a module at its path is left: {}", lua);
        assert!(lua.contains("require \"AskerHelper\""), "the mod's own module is found in the copy: {}", lua);
        assert!(lua.contains("require('Vehicles/VehicleUtils')"), "a module nowhere is left: {}", lua);
        let joined = result.changes.join(" ");
        assert!(joined.contains("Vehicle/ISVehiclePartMenu -> Vehicles/ISUI/ISVehiclePartMenu"), "{}", joined);
        assert!(result.warnings.iter().any(|w| w.contains("Vehicles/VehicleUtils")), "{:?}", result.warnings);
    }

    #[test]
    fn a_build_41_table_becomes_the_build_42_json_with_the_vanilla_key_rule() {
        let txt = "ItemName_EN = {\n\t-- a comment\n\tItemName_Base.Thing = \"A Thing\",\n\tItemName_Base.Other = \"Says \\\"hi\\\"\" .. \" twice\"\n}\n";
        let pairs = parse_b41_translation(txt);
        assert_eq!(pairs, vec![("ItemName_Base.Thing".to_string(), "A Thing".to_string()), ("ItemName_Base.Other".to_string(), "Says \"hi\" twice".to_string())]);

        let json = b41_table_to_json("ItemName", txt);
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["Base.Thing"], "A Thing", "the bare key, as vanilla writes it");
        assert_eq!(parsed["ItemName_Base.Thing"], "A Thing", "and the prefixed one some mods use");
        assert_eq!(parsed["Base.Other"], "Says \"hi\" twice");

        let menu = b41_table_to_json("ContextMenu", "ContextMenu_EN = {\n ContextMenu_Light = \"Light\"\n}");
        let parsed: serde_json::Value = serde_json::from_str(&menu).unwrap();
        assert_eq!(parsed["ContextMenu_Light"], "Light");
        assert!(parsed.get("Light").is_none(), "other categories keep the whole key");

        assert_eq!(json_sibling(Path::new("x/Translate/EN/IG_UI_EN.txt")).unwrap().file_name().unwrap(), "IG_UI.json");
        assert!(json_sibling(Path::new("x/Translate/EN/notes.txt")).is_none());
    }

    #[test]
    fn a_compatible_copy_gets_json_translations_beside_the_txt_ones() {
        let (_t, ws, z) = setup("translate");
        let src = ws.join("Talky");
        fs::create_dir_all(src.join("media/lua/shared/Translate/EN")).unwrap();
        fs::write(src.join("mod.info"), "name=Talky\nid=Talky\n").unwrap();
        fs::write(src.join("media/lua/shared/Translate/EN/ItemName_EN.txt"), "ItemName_EN = {\n\tItemName_Base.Cup = \"Cup\",\n}\n").unwrap();
        fs::write(src.join("media/lua/shared/Translate/EN/UI_EN.txt"), "UI_EN = {\n\tUI_talky_hello = \"Hello\"\n}\n").unwrap();
        let result = make_compatible_copy(&src, &z, 42_021, None, false).expect("copied");
        let dest = Path::new(&result.destination);
        let item = fs::read_to_string(dest.join("42/media/lua/shared/Translate/EN/ItemName.json")).expect("ItemName.json written");
        assert!(item.contains("\"Base.Cup\": \"Cup\""), "{}", item);
        let ui = fs::read_to_string(dest.join("42/media/lua/shared/Translate/EN/UI.json")).unwrap();
        assert!(ui.contains("\"UI_talky_hello\": \"Hello\""), "{}", ui);
        assert!(dest.join("42/media/lua/shared/Translate/EN/UI_EN.txt").is_file(), "the txt stays");
        let joined = result.changes.join(" ");
        assert!(joined.contains("JSON translation"), "{}", joined);
        assert!(!result.warnings.iter().any(|w| w.contains(".txt; Build 42.15+")), "no txt warning once converted: {:?}", result.warnings);
    }

    #[test]
    fn fixes_a_too_low_version_max_like_one_building_one_key() {
        let (_t, ws, z) = setup("range");
        let src = ws.join("OneBuildingOneKey");
        fs::create_dir_all(src.join("42.0/media/lua/client")).unwrap();
        fs::create_dir_all(src.join("common")).unwrap();
        fs::write(src.join("42.0/mod.info"), "name=One Building, One Key\r\nid=OneBuildingOneKey\r\nversionMin=42.20\r\nversionMax=42.20.99\r\n").unwrap();
        fs::write(src.join("42.0/media/lua/client/a.lua"), "local x = 1").unwrap();

        let r = make_compatible_copy(&src, &z, 42_021, Some("123".into()), false).unwrap();
        assert_eq!(r.mod_id, "OneBuildingOneKey");
        let info = fs::read_to_string(Path::new(&r.destination).join("42.0/mod.info")).unwrap();
        assert!(info.contains("versionMax=42.21\r\n"), "{}", info);
        assert!(info.contains("versionMin=42.20\r\n"), "untouched when fine");
        assert!(r.changes.iter().any(|c| c.contains("42.20.99")));
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(fs::read_to_string(src.join("42.0/mod.info")).unwrap().contains("42.20.99"), true, "original untouched");

        let zp = Path::new(&z);
        assert!(local_first(zp));
        assert_eq!(launch_args(&z), vec!["-modfolders", "mods,workshop,steam"]);
        // The catalog now prefers the copy, which loads.
        let cat = crate::catalog::build_catalog(&ws.parent().unwrap().parent().unwrap().to_string_lossy(), &z, Some(42_021));
        let used = cat.get("OneBuildingOneKey").unwrap();
        assert_eq!(used.source, crate::catalog::ModSource::Local);
        assert_eq!(used.loadability, crate::catalog::Loadability::Loadable);

        // Again: refused unless replacing; removing moves it to backups.
        assert!(make_compatible_copy(&src, &z, 42_021, None, false).is_err());
        make_compatible_copy(&src, &z, 42_021, None, true).unwrap();
        remove_compatible_copy(&z, "OneBuildingOneKey").unwrap();
        assert!(!zp.join("mods/OneBuildingOneKey").exists());
        assert!(!local_first(zp));
    }

    /// Dry run on a real mod folder into a temporary Zomboid folder (the real one is not touched).
    /// PZMS_COMPAT_SOURCE=<.../mods/SomeMod> PZMS_GAME_VERSION=42.21 cargo test compat_dry_run -- --ignored --nocapture
    #[test]
    #[ignore]
    fn compat_dry_run_on_real_mod() {
        let Ok(src) = std::env::var("PZMS_COMPAT_SOURCE") else { return };
        let game = std::env::var("PZMS_GAME_VERSION").ok().and_then(|v| version_dir_int(&v)).unwrap_or(42_021);
        let (_t, _ws, z) = setup("dryrun");
        let r = make_compatible_copy(Path::new(&src), &z, game, None, false).unwrap();
        println!("id {} -> {}", r.mod_id, r.destination);
        for c in &r.changes { println!("  change: {}", c); }
        for w in &r.warnings { println!("  warning: {}", w); }
    }

    #[test]
    fn builds_the_b42_layout_for_a_b41_mod_and_reports_what_may_break() {
        let (_t, ws, z) = setup("b41");
        let src = ws.join("OldMod");
        fs::create_dir_all(src.join("media/scripts")).unwrap();
        fs::create_dir_all(src.join("media/lua/shared/Translate/EN")).unwrap();
        fs::write(src.join("mod.info"), "name=Old\nid=OldMod\n").unwrap();
        fs::write(src.join("media/scripts/items.txt"), "module Base {\n  item Thing {\n    Type = Normal,\n  }\n  recipe Make Thing {\n  }\n}\n").unwrap();
        fs::write(src.join("media/lua/shared/Translate/EN/IG_UI_EN.txt"), "IGUI_EN = {}").unwrap();
        fs::write(src.join("media/lua/shared/broken.lua"), "local = 1").unwrap();

        let r = make_compatible_copy(&src, &z, 42_021, None, false).unwrap();
        let dest = Path::new(&r.destination);
        assert!(dest.join("42/mod.info").is_file() && dest.join("common").is_dir());
        assert!(dest.join("42/media/scripts/items.txt").is_file());
        assert!(r.changes.iter().any(|c| c.contains("Build 42 layout")));
        let joined = r.warnings.join(" | ");
        assert!(joined.contains("recipe"), "{}", joined);
        assert!(joined.contains("translation"), "{}", joined);
        assert!(joined.contains("broken.lua"), "{}", joined);
        // The Build 41 item field is not a warning any more: it was converted.
        assert!(!joined.contains("Type ="), "{}", joined);
        assert!(r.changes.iter().any(|c| c.contains("ItemType")), "{:?}", r.changes);
        let items = fs::read_to_string(dest.join("42/media/scripts/items.txt")).unwrap();
        assert!(items.contains("ItemType = base:normal,"), "{}", items);
    }

    #[test]
    fn only_an_items_own_type_field_is_converted() {
        let text = r#"module Base
{
    item Thing
    {
        DisplayName = Thing,
        Type = Clothing,
    }
    fixing Fix
    {
        Type = Other,
    }
    item Odd { Type = SomethingNew, }
}
"#;
        let dir = std::env::temp_dir().join(format!("pzms-itemtype-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("media/scripts")).unwrap();
        fs::write(dir.join("media/scripts/x.txt"), text).unwrap();

        let (changes, warnings) = convert_item_types(&[dir.clone()]);
        let after = fs::read_to_string(dir.join("media/scripts/x.txt")).unwrap();

        // The item was converted, with its indentation kept.
        assert!(after.contains("        ItemType = base:clothing,"), "{}", after);
        // A `Type =` that is not an item's own field is left exactly as it was.
        assert!(after.contains("        Type = Other,"), "{}", after);
        // A value Build 41 never had is left alone and reported instead of guessed.
        assert!(after.contains("item Odd { Type = SomethingNew, }"), "{}", after);
        assert!(changes[0].contains("Rewrote 1 item"), "{:?}", changes);
        assert!(warnings[0].contains("SomethingNew"), "{:?}", warnings);
        // Nothing else moved: same number of lines, same ending.
        assert_eq!(after.lines().count(), text.lines().count());
        assert!(after.ends_with('\n'));

        let _ = fs::remove_dir_all(&dir);
    }

    /// Converts a copy of a real mod folder and prints every line that changed, to be sure only
    /// the fields meant to change did.
    /// PZMS_CONVERT_SOURCE=<a mod folder> cargo test converts_only_the_fields -- --ignored --nocapture
    #[test]
    #[ignore]
    fn converts_only_the_fields() {
        let Ok(src) = std::env::var("PZMS_CONVERT_SOURCE") else { return };
        let work = std::env::temp_dir().join(format!("pzms-convert-{}", std::process::id()));
        let _ = fs::remove_dir_all(&work);
        crate::mod_install::copy_tree(Path::new(&src), &work).unwrap();

        let before: Vec<(PathBuf, String)> = files_under(&work.join("media").join("scripts"), "txt")
            .into_iter()
            .filter_map(|f| fs::read_to_string(&f).ok().map(|t| (f, t)))
            .collect();
        let (changes, warnings) = convert_item_types(&[work.clone()]);
        println!("changes: {:?}
warnings: {:?}", changes, warnings);

        let mut differing_lines = 0;
        for (path, old) in &before {
            let new = fs::read_to_string(path).unwrap();
            if &new == old {
                continue;
            }
            assert_eq!(new.lines().count(), old.lines().count(), "line count changed in {}", path.display());
            for (a, b) in old.lines().zip(new.lines()) {
                if a != b {
                    differing_lines += 1;
                    println!("  - {}
  + {}", a, b);
                    assert!(b.contains("ItemType ="), "a line changed that was not a Type field: {}", b);
                }
            }
        }
        println!("{} line(s) differ", differing_lines);
        let _ = fs::remove_dir_all(&work);
    }

    /// The table above is only right if it is what the installed game writes.
    /// PZMS_PZ_DIR=... cargo test the_item_types_are_the_ones_the_game_uses -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_item_types_are_the_ones_the_game_uses() {
        let Ok(pz) = std::env::var("PZMS_PZ_DIR") else { return };
        let scripts = Path::new(&pz).join("media").join("scripts");
        let mut used: std::collections::BTreeSet<String> = Default::default();
        let mut old_field = 0usize;
        for f in files_under(&scripts, "txt") {
            let Ok(text) = fs::read_to_string(&f) else { continue };
            for line in text.lines() {
                let t = line.trim();
                if let Some(v) = t.strip_prefix("ItemType") {
                    if let Some(v) = v.trim_start().strip_prefix('=') {
                        used.insert(v.trim().trim_end_matches(',').trim().to_string());
                    }
                } else if t.starts_with("Type") && t[4..].trim_start().starts_with('=') {
                    old_field += 1;
                }
            }
        }
        println!("{} ItemType values in the game: {:?}", used.len(), used);
        assert_eq!(old_field, 0, "the game still writes the Build 41 field somewhere");
        for (b41, b42) in ITEM_TYPES_B41_TO_B42 {
            assert!(used.contains(b42), "the game does not use {} (mapped from {})", b42, b41);
        }
    }
}
