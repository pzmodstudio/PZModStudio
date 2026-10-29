//! The player's own load-order rules: "A loads after B", kept in `Zomboid/PZModStudio_Rules.json`.
//!
//! The game's rules come from mod.info (`require`, `loadModAfter`, `loadModBefore`) and the
//! Studio's sort and health check already follow them. What they cannot know is what the player
//! found out by playing: that this overhaul has to come after that library or the recipes vanish.
//! A rule here is honoured by the sort as if the mod had written `loadModAfter` itself, and the
//! health check says when the list breaks one, in the player's own words.

pub mod community;
pub mod github_device;
pub mod index;
pub mod propose;
pub mod shared_lists;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::load_order::mod_info::get_existing_user_zomboid_dirs;

const RULES_FILE: &str = "PZModStudio_Rules.json";

/// One rule: `mod_id` must load after `after`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserRule {
    pub mod_id: String,
    pub after: String,
    /// Why, in the player's words, when they said.
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RulesFile {
    #[serde(default)]
    load_after: Vec<UserRule>,
}

fn rules_path(user_zomboid_dir: &str) -> Result<PathBuf, String> {
    get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .map(|d| d.join(RULES_FILE))
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))
}

/// Every rule on disk; none when there is no file yet.
pub fn load_rules(user_zomboid_dir: &str) -> Vec<UserRule> {
    rules_path(user_zomboid_dir)
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<RulesFile>(&t).ok())
        .map(|f| f.load_after)
        .unwrap_or_default()
}

fn save_rules(user_zomboid_dir: &str, rules: &[UserRule]) -> Result<(), String> {
    let path = rules_path(user_zomboid_dir)?;
    let file = RulesFile { load_after: rules.to_vec() };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::guard_overwrite::<RulesFile>(&path)?;
    crate::load_order::ini_parser::atomic_write(path, json).map_err(|e| e.to_string())
}

/// Adds "mod_id after after". The same pair twice is one rule; a mod after itself is refused.
pub fn add_rule(user_zomboid_dir: &str, mod_id: &str, after: &str, note: Option<String>) -> Result<Vec<UserRule>, String> {
    let (mod_id, after) = (mod_id.trim(), after.trim());
    if mod_id.is_empty() || after.is_empty() {
        return Err("A rule needs two mods.".to_string());
    }
    if mod_id == after {
        return Err("A mod cannot load after itself.".to_string());
    }
    let mut rules = load_rules(user_zomboid_dir);
    if rules.iter().any(|r| r.mod_id == after && r.after == mod_id) {
        return Err(format!("There is already a rule the other way round: {} loads after {}.", after, mod_id));
    }
    if let Some(existing) = rules.iter_mut().find(|r| r.mod_id == mod_id && r.after == after) {
        existing.note = note;
    } else {
        rules.push(UserRule { mod_id: mod_id.to_string(), after: after.to_string(), note });
    }
    save_rules(user_zomboid_dir, &rules)?;
    Ok(rules)
}

pub fn remove_rule(user_zomboid_dir: &str, mod_id: &str, after: &str) -> Result<Vec<UserRule>, String> {
    let mut rules = load_rules(user_zomboid_dir);
    let before = rules.len();
    rules.retain(|r| !(r.mod_id == mod_id && r.after == after));
    if rules.len() == before {
        return Err(format!("There is no rule that {} loads after {}.", mod_id, after));
    }
    save_rules(user_zomboid_dir, &rules)?;
    Ok(rules)
}

/// Makes the catalog carry the rules as if each mod had written `loadModAfter` itself, so the
/// game-exact sort places them without knowing the difference.
pub fn apply_to_catalog(catalog: &mut crate::catalog::ModCatalog, rules: &[UserRule]) {
    for rule in rules {
        for m in catalog.mods.iter_mut().filter(|m| m.id == rule.mod_id) {
            if !m.info.load_after.iter().any(|a| a == &rule.after) {
                m.info.load_after.push(rule.after.clone());
            }
        }
    }
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

    fn zomboid(tag: &str) -> (Tmp, String) {
        let t = Tmp(std::env::temp_dir().join(format!("pzms-rules-{}-{}", tag, std::process::id())));
        let _ = fs::remove_dir_all(&t.0);
        fs::create_dir_all(t.0.join("mods")).unwrap();
        let z = t.0.to_string_lossy().to_string();
        (t, z)
    }

    #[test]
    fn rules_are_kept_once_each_and_can_go() {
        let (_t, z) = zomboid("keep");
        assert!(load_rules(&z).is_empty());
        add_rule(&z, "Overhaul", "Library", Some("recipes vanish otherwise".into())).unwrap();
        add_rule(&z, "Overhaul", "Library", None).unwrap();
        add_rule(&z, "Other", "Library", None).unwrap();
        let rules = load_rules(&z);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].note, None, "the later add replaced the note");
        assert!(add_rule(&z, "Library", "Overhaul", None).is_err(), "the other way round is refused");
        assert!(add_rule(&z, "A", "A", None).is_err());
        remove_rule(&z, "Overhaul", "Library").unwrap();
        assert_eq!(load_rules(&z).len(), 1);
        assert!(remove_rule(&z, "Overhaul", "Library").is_err());
    }
}
