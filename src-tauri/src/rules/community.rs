//! The Studio's own load-order knowledge: `community/rules.json` in the Studio's repository,
//! built into the app. A rule here is honoured like one of the player's, and named as the
//! Studio's with its source, so it can be checked and argued. Nothing is downloaded: each
//! version of the app carries the rules known when it was built.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// The rules as this build of the app carries them.
const BUNDLED: &str = include_str!("../../../community/rules.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommunityRule {
    pub mod_id: String,
    pub after: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommunityPair {
    pub a: String,
    pub b: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunityRules {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub load_after: Vec<CommunityRule>,
    #[serde(default)]
    pub incompatible: Vec<CommunityPair>,
}

/// What the Health screen says about the knowledge the app carries: the rules, and the mod index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommunitySummary {
    pub rules: usize,
    pub pairs: usize,
    pub updated: Option<String>,
    /// Registries in the mod index.
    #[serde(default)]
    pub mods: usize,
    #[serde(default)]
    pub index_updated: Option<String>,
    /// Known issues the player marked as no longer happening.
    #[serde(default)]
    pub dismissed: usize,
}

/// Everything the app knows, counted.
pub fn summary(user_zomboid_dir: &str) -> CommunitySummary {
    let rules = current();
    let index = super::index::current(user_zomboid_dir);
    CommunitySummary {
        rules: rules.load_after.len(),
        pairs: rules.incompatible.len(),
        updated: rules.updated.clone(),
        mods: index.mods.len(),
        index_updated: index.updated.clone(),
        dismissed: super::index::dismissed(user_zomboid_dir).len(),
    }
}

impl CommunityRules {
    /// The load-after rules in the player's own shape, for the sort.
    pub fn as_user_rules(&self) -> Vec<super::UserRule> {
        self.load_after
            .iter()
            .map(|r| super::UserRule { mod_id: r.mod_id.clone(), after: r.after.clone(), note: r.note.clone() })
            .collect()
    }
}

/// Reads the rules text, refusing anything that is not the shape expected.
pub fn parse(text: &str) -> Result<CommunityRules, String> {
    let rules: CommunityRules = serde_json::from_str(text).map_err(|e| format!("The Studio's rules file is not readable: {}", e))?;
    for r in &rules.load_after {
        if r.mod_id.trim().is_empty() || r.after.trim().is_empty() || r.mod_id == r.after {
            return Err(format!("A Studio rule is malformed: {:?}", r));
        }
    }
    for p in &rules.incompatible {
        if p.a.trim().is_empty() || p.b.trim().is_empty() || p.a == p.b {
            return Err(format!("A Studio pair is malformed: {:?}", p));
        }
    }
    Ok(rules)
}

/// The rules this build carries (the repository's test keeps the file well formed).
pub fn current() -> &'static CommunityRules {
    static RULES: OnceLock<CommunityRules> = OnceLock::new();
    RULES.get_or_init(|| parse(BUNDLED).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules_the_app_carries_parse() {
        let rules = parse(BUNDLED).expect("community/rules.json parses");
        assert_eq!(rules.version, 1);
    }

    #[test]
    fn a_malformed_rule_is_refused_and_the_shape_round_trips() {
        assert!(parse(r#"{"load_after":[{"mod_id":"A","after":"A"}]}"#).is_err());
        assert!(parse(r#"{"incompatible":[{"a":"","b":"B"}]}"#).is_err());
        let rules = parse(r#"{"version":1,"updated":"2026-10-18","load_after":[{"mod_id":"Overhaul","after":"Library","note":"recipes","source":"tested on 42.21"}],"incompatible":[{"a":"X","b":"Y"}]}"#).unwrap();
        assert_eq!(rules.as_user_rules()[0].after, "Library");
        assert_eq!(rules.incompatible.len(), 1);
    }
}
