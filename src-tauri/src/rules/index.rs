//! The community mod index: one registry per mod in `community/index/` of the Studio's own
//! repository, built into one `community/index.json` by the repository's own check, fetched
//! once a day like the rules and kept for when the player is offline.
//!
//! What a registry says, every entry with the build it was seen on and whether the Studio
//! verified it or a person reported it: which builds the mod runs on, what it needs, what it
//! does not go with, what it must load after, where its Build 42 successor is, and the issues
//! it is known to have in the log, each with the fix the Studio can apply when there is one.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Where the built index lives: this repository, main branch.
pub const INDEX_URL: &str =
    "https://raw.githubusercontent.com/pzmodstudio/PZModStudio/main/community/index.json";
const CACHE_FILE: &str = "PZModStudio_CommunityIndex.json";

/// How sure an entry is: the Studio checked it (the log, mod.info), or a person said so.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Verified,
    #[default]
    Reported,
}

/// A relation to another mod: incompatible with it, or must load after it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Relation {
    pub id: String,
    #[serde(default)]
    pub note: Option<String>,
    /// The game build it was seen on ("42.21").
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub status: Status,
}

/// What the Studio can do about a known issue in one click, or nothing but say it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IssueFix {
    /// One of the shim's part-7 patches, by its switch name.
    ShimPatch { name: String },
    /// The mod's own file requires a module Build 42 moved: a compatible copy rewrites it, and the
    /// Monitor Center's "Fix the path" publishes the one file into the Studio package.
    RequirePath {
        /// The module as the mod requires it.
        from: String,
        /// Where Build 42 has it.
        to: String,
    },
    /// Copy the mod to Zomboid/mods and adapt it to the installed build.
    CompatibleCopy,
    /// Nothing to do: the line is harmless, or no fix is known yet.
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnownIssue {
    pub title: String,
    #[serde(default)]
    pub note: Option<String>,
    /// The file in the mod, from its root (`media/lua/shared/aFancyHandwork.lua`).
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub line: Option<u32>,
    /// What the log said, as evidence.
    #[serde(default)]
    pub log: Option<String>,
    #[serde(default = "no_fix")]
    pub fix: IssueFix,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub status: Status,
}

fn no_fix() -> IssueFix {
    IssueFix::None
}

/// The Build 42 version of a Build 41 mod, when it is a different Workshop item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Successor {
    pub id: String,
    #[serde(default)]
    pub workshop_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// One mod's registry: `community/index/<id>.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    /// The `id=` line of mod.info.
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub workshop_id: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    /// Major builds it runs on: "41", "42".
    #[serde(default)]
    pub builds: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub incompatible: Vec<Relation>,
    #[serde(default)]
    pub load_after: Vec<Relation>,
    #[serde(default)]
    pub successor: Option<Successor>,
    #[serde(default)]
    pub issues: Vec<KnownIssue>,
}

/// The built file the app downloads.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunityIndex {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub mods: BTreeMap<String, Registry>,
    /// When this copy was fetched (unix seconds); set by the Studio, not by the file.
    #[serde(default)]
    pub fetched_at: Option<i64>,
}

impl CommunityIndex {
    pub fn get(&self, mod_id: &str) -> Option<&Registry> {
        self.mods.get(mod_id)
    }

    /// The registry whose Workshop item this is, for the Browse page before anything is installed.
    pub fn by_workshop_id(&self, workshop_id: &str) -> Vec<&Registry> {
        self.mods.values().filter(|r| r.workshop_id.as_deref() == Some(workshop_id)).collect()
    }

    /// The load-after relations in the player's own shape, for the sort.
    pub fn as_user_rules(&self) -> Vec<super::UserRule> {
        self.mods
            .values()
            .flat_map(|r| {
                r.load_after
                    .iter()
                    .map(move |a| super::UserRule { mod_id: r.id.clone(), after: a.id.clone(), note: a.note.clone() })
            })
            .collect()
    }
}

/// Reads an index, refusing what is not the shape expected. A registry's key is its id.
pub fn parse(text: &str) -> Result<CommunityIndex, String> {
    let index: CommunityIndex = serde_json::from_str(text).map_err(|e| format!("The community index is not readable: {}", e))?;
    for (key, r) in &index.mods {
        if r.id.trim().is_empty() || &r.id != key {
            return Err(format!("A registry is filed under \"{}\" but says its id is \"{}\"", key, r.id));
        }
        if r.incompatible.iter().chain(r.load_after.iter()).any(|rel| rel.id.trim().is_empty() || rel.id == r.id) {
            return Err(format!("A relation of {} is malformed", r.id));
        }
        if r.issues.iter().any(|i| i.title.trim().is_empty()) {
            return Err(format!("An issue of {} has no title", r.id));
        }
    }
    Ok(index)
}

/// The last copy fetched, if any.
pub fn load_cached(user_zomboid_dir: &str) -> Option<CommunityIndex> {
    let text = std::fs::read_to_string(super::community::cache_path(user_zomboid_dir, CACHE_FILE)?).ok()?;
    parse(&text).ok()
}

/// Fetches the index now and keeps it. Says why when it cannot.
pub fn refresh(user_zomboid_dir: &str) -> Result<CommunityIndex, String> {
    let path = super::community::cache_path(user_zomboid_dir, CACHE_FILE)
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;
    let text = super::community::fetch_text(INDEX_URL).map_err(|e| format!("The community index could not be fetched: {}", e))?;
    let mut index = parse(&text)?;
    index.fetched_at = Some(super::community::now());
    let json = serde_json::to_string_pretty(&index).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(index)
}

/// The copy to use: the cached one while it is less than a day old, else a fresh fetch, else
/// the cached one however old, else none. With downloads off in Settings, the cached one only.
pub fn current(user_zomboid_dir: &str) -> Option<CommunityIndex> {
    let cached = load_cached(user_zomboid_dir);
    if !super::community::downloads_enabled(user_zomboid_dir) || cached.as_ref().and_then(|c| c.fetched_at).map(super::community::is_fresh).unwrap_or(false) {
        return cached;
    }
    refresh(user_zomboid_dir).ok().or(cached)
}

/// Whether a known issue is already dealt with on this machine: its shim patch is on, or the
/// Studio package carries the file the Monitor Center fixed.
pub fn issue_is_handled(issue: &KnownIssue, shim_on: &dyn Fn(&str) -> bool, package_root: &Path) -> bool {
    match &issue.fix {
        IssueFix::ShimPatch { name } => shim_on(name),
        IssueFix::RequirePath { .. } | IssueFix::CompatibleCopy => issue
            .file
            .as_deref()
            .map(|f| package_root.join(f).is_file() || package_root.join("42").join(f).is_file())
            .unwrap_or(false),
        IssueFix::None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_index_in_this_repository_parses_and_matches_its_registries() {
        let text = include_str!("../../../community/index.json");
        let index = parse(text).expect("the built index parses");
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../community/index");
        let mut on_disk = 0;
        for entry in std::fs::read_dir(&dir).expect("community/index exists") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            on_disk += 1;
            let registry: Registry = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
            let built = index.get(&registry.id).unwrap_or_else(|| panic!("{} is not in index.json: run node scripts/community-index.js build", registry.id));
            assert!(built.issues.len() >= registry.issues.len(), "{} differs from index.json: run node scripts/community-index.js build", registry.id);
        }
        // Reports (community/reports) add issues, and mods no registry has yet.
        assert!(index.mods.len() >= on_disk, "index.json lacks a registry: run node scripts/community-index.js build");
    }

    #[test]
    fn a_registry_filed_under_the_wrong_id_is_refused_and_relations_become_rules() {
        assert!(parse(r#"{"mods":{"A":{"id":"B"}}}"#).is_err());
        assert!(parse(r#"{"mods":{"A":{"id":"A","incompatible":[{"id":"A"}]}}}"#).is_err());
        let index = parse(r#"{"version":1,"mods":{"Overhaul":{"id":"Overhaul","load_after":[{"id":"Library","note":"recipes","build":"42.21","status":"verified"}],"issues":[{"title":"x","fix":{"kind":"shim_patch","name":"p"}}]}}}"#).unwrap();
        let rules = index.as_user_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].after, "Library");
        let issue = &index.get("Overhaul").unwrap().issues[0];
        assert!(issue_is_handled(issue, &|n| n == "p", Path::new("/nowhere")));
        assert!(!issue_is_handled(issue, &|_| false, Path::new("/nowhere")));
    }
}
