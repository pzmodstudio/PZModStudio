//! Translations two mods give differently: the same key, the same language, other words.
//!
//! The game's Translator reads the translation files of every active mod (checked in the
//! decompiled 42.20.4 code), so two mods shipping `Translate/EN/ItemName.json` never collide on
//! disk, and the file scan leaves them alone. What they can do is name the same thing twice —
//! `Base.Axe` as "Axe" in one and "Hatchet" in the other — and then the mod that loads last has
//! the last word, silently. That is the same question the script definitions raise, and it goes
//! to the same places: a row in Mod Health and a conflict in the Merger, whose saved file lands
//! in the Studio package, which loads last of all.
//!
//! Keys: the game reads `ItemName`, `Recipes` and `EvolvedRecipeName` keys bare (`Base.Axe`) and
//! the other categories with the category in front (`ContextMenu_Foo`); mods write both forms
//! and both are read, so a key is compared with the category prefix stripped and written back
//! the way the winning mod wrote it.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// One translation file of one mod: where, which language and category, and its entries.
#[derive(Debug, Clone)]
pub struct TranslationFile {
    pub mod_id: String,
    pub mod_name: String,
    pub lang: String,
    pub category: String,
    pub absolute_path: String,
    /// Key as written -> text.
    pub entries: Vec<(String, String)>,
}

/// One key two or more mods translate differently, in load order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationConflict {
    pub lang: String,
    pub category: String,
    /// The key with the category prefix stripped, as compared.
    pub key: String,
    pub mod_ids: Vec<String>,
    pub mods: Vec<String>,
    pub absolute_files: Vec<String>,
    /// Each mod's key as it wrote it, and its text, in the same order.
    pub written_keys: Vec<String>,
    pub values: Vec<String>,
}

/// `ItemName_EN.json` and `ItemName.json` are both the ItemName category.
pub fn category_of(file_stem: &str, lang: &str) -> String {
    let suffix = format!("_{}", lang);
    file_stem.strip_suffix(&suffix).unwrap_or(file_stem).to_string()
}

/// The key as compared: `ContextMenu_Foo` and `Foo` are one key in ContextMenu.
pub fn bare_key(key: &str, category: &str) -> String {
    let prefix = format!("{}_", category);
    key.strip_prefix(&prefix).unwrap_or(key).to_string()
}

/// Reads one translation JSON: a flat object of strings; anything else is skipped.
pub fn parse_entries(text: &str) -> Vec<(String, String)> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    map.into_iter().filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string()))).collect()
}

/// The translation files of one mod, from the media layers the installed build reads.
pub fn files_of_mod(mod_id: &str, mod_name: &str, root: &Path, game_version: Option<u32>) -> Vec<TranslationFile> {
    let mut out = Vec::new();
    for layer in crate::load_order::mod_info::media_layers_for_build(root, game_version) {
        let shared = layer.join("media").join("lua").join("shared");
        let Ok(dirs) = std::fs::read_dir(&shared) else { continue };
        let Some(translate) = dirs.flatten().map(|e| e.path()).find(|p| p.is_dir() && p.file_name().and_then(|n| n.to_str()).map(|n| n.eq_ignore_ascii_case("translate")).unwrap_or(false)) else { continue };
        for entry in WalkDir::new(&translate).min_depth(2).max_depth(2).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || !path.extension().map(|e| e.eq_ignore_ascii_case("json")).unwrap_or(false) {
                continue;
            }
            let lang = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("").to_uppercase();
            let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
            // Mod.json names and describes the mod it sits in; every mod has one and they never
            // compete.
            if category_of(stem, &lang).eq_ignore_ascii_case("Mod") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else { continue };
            let entries = parse_entries(&text);
            if entries.is_empty() {
                continue;
            }
            out.push(TranslationFile {
                mod_id: mod_id.to_string(),
                mod_name: mod_name.to_string(),
                lang: lang.clone(),
                category: category_of(stem, &lang),
                absolute_path: path.to_string_lossy().to_string(),
                entries,
            });
        }
    }
    out
}

/// Keys two or more mods translate differently, from their files in load order. The same text
/// twice is no conflict; a mod's own version folder replacing its `common/` copy is already
/// settled by the layers. Sorted by language, category, key.
pub fn conflicts_in(files: &[TranslationFile]) -> Vec<TranslationConflict> {
    // (lang, category, bare key) -> (mod_id, mod_name, file, written key, text), first write per mod wins.
    let mut by_key: BTreeMap<(String, String, String), Vec<(String, String, String, String, String)>> = BTreeMap::new();
    for f in files {
        for (written, text) in &f.entries {
            let k = (f.lang.clone(), f.category.clone(), bare_key(written, &f.category));
            let list = by_key.entry(k).or_default();
            if list.iter().any(|(id, ..)| id == &f.mod_id) {
                continue;
            }
            list.push((f.mod_id.clone(), f.mod_name.clone(), f.absolute_path.clone(), written.clone(), text.clone()));
        }
    }
    by_key
        .into_iter()
        // Different words, not different whitespace.
        .filter(|(_, list)| list.len() > 1 && list.iter().any(|(.., v)| v.trim() != list[0].4.trim()))
        .map(|((lang, category, key), list)| TranslationConflict {
            lang,
            category,
            key,
            mod_ids: list.iter().map(|x| x.0.clone()).collect(),
            mods: list.iter().map(|x| x.1.clone()).collect(),
            absolute_files: list.iter().map(|x| x.2.clone()).collect(),
            written_keys: list.iter().map(|x| x.3.clone()).collect(),
            values: list.iter().map(|x| x.4.clone()).collect(),
        })
        .collect()
}

/// Every conflict among the active mods, in load order.
pub fn translation_conflicts(catalog: &crate::catalog::ModCatalog, active: &[String]) -> Vec<TranslationConflict> {
    let game = catalog.game_version_int();
    let mut files = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in active {
        if entry.starts_with("Z_PZModStudio_") || entry == "PZModStudioCarrier" {
            continue;
        }
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if m.loadability != crate::catalog::Loadability::Loadable || !seen.insert(m.id.clone()) {
            continue;
        }
        files.extend(files_of_mod(&m.id, &m.name, Path::new(&m.root_dir), game));
    }
    conflicts_in(&files)
}

/// The language the game shows, from `Zomboid/options.ini` (`language=EN`); EN when unsaid.
/// Mod Health speaks only of that one: a wrong word in a language the player never sees is
/// nothing to act on, though the Merger lists every language.
pub fn game_language(user_zomboid_dir: &str) -> String {
    crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir)
        .first()
        .and_then(|d| std::fs::read_to_string(d.join("options.ini")).ok())
        .and_then(|t| t.lines().find_map(|l| l.trim().strip_prefix("language=").map(|v| v.trim().to_uppercase())))
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "EN".to_string())
}

/// Where the package's copy goes: the same path the mods use, which the game reads from the
/// package too, last of all.
pub fn merged_file_path(lang: &str, category: &str) -> String {
    format!("media/lua/shared/Translate/{}/{}.json", lang, category)
}

/// `"Axe" (A) against "Hatchet" (B, C)`: each different text once, with who says it.
pub fn distinct_values(c: &TranslationConflict) -> String {
    distinct_value_list(c).join(" against ")
}

/// Each value once, with the mods that give it: `"Axe" (A, B)`.
pub fn distinct_value_list(c: &TranslationConflict) -> Vec<String> {
    let mut seen: Vec<(String, Vec<String>)> = Vec::new();
    for (i, v) in c.values.iter().enumerate() {
        let v = v.trim().to_string();
        match seen.iter_mut().find(|(x, _)| x == &v) {
            Some((_, who)) => who.push(c.mods[i].clone()),
            None => seen.push((v, vec![c.mods[i].clone()])),
        }
    }
    seen.iter().map(|(v, who)| format!("\"{}\" ({})", v, who.join(", "))).collect()
}

fn json_object(pairs: &[(String, String)]) -> String {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    serde_json::to_string_pretty(&serde_json::Value::Object(map)).unwrap_or_else(|_| "{}".to_string()) + "\n"
}

/// The conflicts of one language and category as one Merger conflict: each mod's clashing keys
/// as it wrote them, and the proposal — what the game shows today, the last mod's words — as a
/// JSON the package can carry. Grouped so one file holds every decision for that category.
pub fn as_merger_conflicts(conflicts: &[TranslationConflict], drafts: &std::collections::HashMap<String, (String, String)>) -> Vec<crate::vfs::VfsConflictRaw> {
    let mut groups: BTreeMap<(String, String), Vec<&TranslationConflict>> = BTreeMap::new();
    for c in conflicts {
        groups.entry((c.lang.clone(), c.category.clone())).or_default().push(c);
    }
    groups
        .into_iter()
        .map(|((lang, category), group)| {
            let rel_path = merged_file_path(&lang, &category);
            // Mods in the order they load, each with only the keys it clashes on.
            let mut mod_order: Vec<(String, String, String)> = Vec::new();
            for c in &group {
                for (i, id) in c.mod_ids.iter().enumerate() {
                    if !mod_order.iter().any(|(x, ..)| x == id) {
                        mod_order.push((id.clone(), c.mods[i].clone(), c.absolute_files[i].clone()));
                    }
                }
            }
            let competing: Vec<crate::vfs::CompetingModFileRaw> = mod_order
                .iter()
                .map(|(id, name, file)| {
                    let pairs: Vec<(String, String)> = group
                        .iter()
                        .filter_map(|c| c.mod_ids.iter().position(|x| x == id).map(|i| (c.written_keys[i].clone(), c.values[i].clone())))
                        .collect();
                    crate::vfs::CompetingModFileRaw { mod_id: id.clone(), mod_name: name.clone(), absolute_path: file.clone(), content: json_object(&pairs) }
                })
                .collect();
            let proposal_pairs: Vec<(String, String)> = group
                .iter()
                .map(|c| (c.written_keys.last().cloned().unwrap_or_default(), c.values.last().cloned().unwrap_or_default()))
                .collect();
            let proposal = json_object(&proposal_pairs);
            let names: Vec<&str> = mod_order.iter().map(|(_, n, _)| n.as_str()).collect();
            let example = group.first().map(|c| format!(" {} is {}.", c.key, distinct_values(c))).unwrap_or_default();
            let strategy = format!(
                "{} name {} thing{} differently in {} ({}).{} The game reads every mod's translations and keeps the last loaded, so {} has the last word today; the proposal keeps those words. Pick one mod's, or write your own: the file goes in your package, which loads last.",
                names.join(" and "),
                group.len(),
                if group.len() == 1 { "" } else { "s" },
                lang,
                category,
                example,
                names.last().copied().unwrap_or("")
            );
            let (merged_output, status) = match drafts.get(&rel_path) {
                Some((saved, saved_status)) => (saved.clone(), Some(saved_status.clone())),
                None => (proposal.clone(), None),
            };
            let lines = proposal.lines().count().max(1);
            crate::vfs::VfsConflictRaw {
                id: format!("translation_{}", rel_path),
                relative_path: rel_path,
                file_type: "TRANSLATION".to_string(),
                start_line: 1,
                end_line: lines,
                conflict_line: 1,
                total_file_lines: lines,
                base_content: "-- vanilla file not present for this path".to_string(),
                competing_files: competing,
                merged_output,
                auto_ast_output: proposal,
                is_identical_noise: false,
                status,
                mod_changes: Vec::new(),
                collisions: Vec::new(),
                strategy,
                auto_mergeable: false,
            }
        })
        .collect()
}

/// Whether the package already holds a file for the group, and whether a mod changed since:
/// pending, merged or stale, the words the other conflicts use.
pub fn merge_state(package_root: Option<&PathBuf>, lang: &str, category: &str, absolute_files: &[String]) -> String {
    let rel = merged_file_path(lang, category);
    let Some(root) = package_root else { return "pending".to_string() };
    let Some(merged) = [root.join("42").join(&rel), root.join(&rel)].into_iter().find(|p| p.is_file()) else { return "pending".to_string() };
    let merged_at = merged.metadata().ok().and_then(|m| m.modified().ok());
    let newer = absolute_files.iter().any(|f| matches!((Path::new(f).metadata().ok().and_then(|m| m.modified().ok()), merged_at), (Some(a), Some(b)) if a > b));
    if newer { "stale".to_string() } else { "merged".to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(mod_id: &str, lang: &str, category: &str, entries: &[(&str, &str)]) -> TranslationFile {
        TranslationFile {
            mod_id: mod_id.into(),
            mod_name: mod_id.into(),
            lang: lang.into(),
            category: category.into(),
            absolute_path: format!("/{}/{}/{}.json", mod_id, lang, category),
            entries: entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    #[test]
    fn categories_and_keys_are_read_in_both_forms() {
        assert_eq!(category_of("ItemName_EN", "EN"), "ItemName");
        assert_eq!(category_of("ItemName", "EN"), "ItemName");
        assert_eq!(bare_key("ContextMenu_Foo", "ContextMenu"), "Foo");
        assert_eq!(bare_key("Base.Axe", "ItemName"), "Base.Axe");
        assert_eq!(parse_entries(r#"{"a": "b", "n": 1, "o": {}}"#), vec![("a".to_string(), "b".to_string())]);
        assert!(parse_entries("not json").is_empty());
    }

    #[test]
    fn only_different_words_for_the_same_key_are_a_conflict_and_the_last_mod_wins_in_the_proposal() {
        let files = vec![
            file("A", "EN", "ItemName", &[("Base.Axe", "Axe"), ("Base.Saw", "Saw")]),
            file("B", "EN", "ItemName", &[("ItemName_Base.Axe", "Hatchet"), ("Base.Saw", "Saw"), ("Base.Pan", "Pan")]),
            file("C", "FR", "ItemName", &[("Base.Axe", "Hache")]),
        ];
        let conflicts = conflicts_in(&files);
        assert_eq!(conflicts.len(), 1, "{:#?}", conflicts);
        let c = &conflicts[0];
        assert_eq!((c.lang.as_str(), c.category.as_str(), c.key.as_str()), ("EN", "ItemName", "Base.Axe"));
        assert_eq!(c.values, vec!["Axe", "Hatchet"]);
        assert_eq!(c.written_keys, vec!["Base.Axe", "ItemName_Base.Axe"], "each mod's key as it wrote it");
        let merger = as_merger_conflicts(&conflicts, &Default::default());
        assert_eq!(merger.len(), 1);
        assert_eq!(merger[0].relative_path, "media/lua/shared/Translate/EN/ItemName.json");
        assert_eq!(merger[0].file_type, "TRANSLATION");
        assert_eq!(merger[0].competing_files.len(), 2);
        assert!(merger[0].competing_files[0].content.contains("\"Base.Axe\": \"Axe\""));
        assert!(merger[0].auto_ast_output.contains("\"ItemName_Base.Axe\": \"Hatchet\""), "{}", merger[0].auto_ast_output);
        assert!(merger[0].strategy.contains("B has the last word"), "{}", merger[0].strategy);
        assert_eq!(merge_state(None, "EN", "ItemName", &[]), "pending");
    }
}
