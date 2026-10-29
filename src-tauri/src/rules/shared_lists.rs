//! Other players' lists: one file per list in this repository's `community/lists/<id>.json`, built
//! into `community/lists.json` by `scripts/community-lists.js`, fetched once a day the way the mod
//! index is. A list is shared by pull request, so a person reads it before anyone receives it.
//! Copying one makes it the player's own list (Browse > My lists), where Subscribe all and Make a
//! profile already work.

use serde::{Deserialize, Serialize};

const LISTS_URL: &str =
    "https://raw.githubusercontent.com/pzmodstudio/PZModStudio/main/community/lists.json";
const CACHE_FILE: &str = "PZModStudio_CommunityLists.json";
pub const MAX_ITEMS: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workshop_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mod_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nexus_id: Option<u64>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedList {
    /// The file's name without `.json`: lower-case letters, digits and `-`.
    pub id: String,
    pub name: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    /// "41" or "42".
    #[serde(default)]
    pub build: String,
    #[serde(default)]
    pub updated: String,
    pub items: Vec<SharedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunityLists {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub lists: Vec<SharedList>,
    /// When this copy was fetched (unix seconds); None for the copy that ships.
    #[serde(default)]
    pub fetched_at: Option<i64>,
}

/// `Survival Plus (B42)` -> `survival-plus-b42`.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').chars().take(60).collect::<String>().trim_end_matches('-').to_string()
}

/// What is wrong with a list, the same rules as the repository's check (`scripts/community-lists.js`).
pub fn problems(list: &SharedList) -> Vec<String> {
    let mut p = Vec::new();
    let id_ok = (3..=60).contains(&list.id.len()) && list.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && !list.id.starts_with('-') && !list.id.ends_with('-');
    if !id_ok {
        p.push("id: 3 to 60 lower-case letters, digits and -".to_string());
    }
    if list.name.trim().is_empty() || list.name.chars().count() > 80 {
        p.push("name: 1 to 80 characters".to_string());
    }
    if list.author.trim().is_empty() || list.author.chars().count() > 60 {
        p.push("author: 1 to 60 characters".to_string());
    }
    if list.description.chars().count() > 500 {
        p.push("description: at most 500 characters".to_string());
    }
    if !list.build.is_empty() && list.build != "41" && list.build != "42" {
        p.push("build: \"41\" or \"42\"".to_string());
    }
    if list.items.is_empty() || list.items.len() > MAX_ITEMS {
        p.push(format!("items: 1 to {}", MAX_ITEMS));
    }
    for (i, item) in list.items.iter().enumerate() {
        let workshop_ok = item.workshop_id.as_deref().map(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()));
        if workshop_ok == Some(false) {
            p.push(format!("items[{}]: workshop_id is the number in the Workshop link", i));
        }
        if item.workshop_id.is_none() && item.nexus_id.is_none() && item.mod_id.as_deref().map(str::trim).unwrap_or("").is_empty() {
            p.push(format!("items[{}]: a Workshop id, a Nexus id or a mod id", i));
        }
        if item.name.trim().is_empty() {
            p.push(format!("items[{}]: name is missing", i));
        }
    }
    p
}

/// A player's list as it goes to the repository.
pub fn from_list(list: &crate::lists::ModList, author: &str, description: &str, build: &str, today: &str) -> SharedList {
    SharedList {
        id: slug(&list.name),
        name: list.name.trim().to_string(),
        author: author.trim().to_string(),
        description: description.trim().to_string(),
        build: build.to_string(),
        updated: today.to_string(),
        items: list
            .items
            .iter()
            .map(|i| SharedItem { workshop_id: i.workshop_id.clone(), mod_id: if i.workshop_id.is_some() { None } else { i.mod_id.clone() }, nexus_id: i.nexus_id, name: i.name.clone() })
            .collect(),
    }
}

fn parse(text: &str) -> Result<CommunityLists, String> {
    let mut all: CommunityLists = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("The community lists are not readable: {}", e))?;
    // A list that would not pass the repository's check is left out rather than shown half.
    all.lists.retain(|l| problems(l).is_empty());
    Ok(all)
}

pub fn load_cached(user_zomboid_dir: &str) -> Option<CommunityLists> {
    let text = std::fs::read_to_string(super::community::cache_path(user_zomboid_dir, CACHE_FILE)?).ok()?;
    parse(&text).ok()
}

pub fn refresh(user_zomboid_dir: &str) -> Result<CommunityLists, String> {
    let path = super::community::cache_path(user_zomboid_dir, CACHE_FILE).ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;
    let text = super::community::fetch_text(LISTS_URL).map_err(|e| format!("The community lists could not be fetched: {}", e))?;
    let mut all = parse(&text)?;
    all.fetched_at = Some(super::community::now());
    let json = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(all)
}

/// The cached copy while it is less than a day old, else a fresh fetch, else the cached one,
/// else the copy that ships. With downloads off in Settings, never fetched.
pub fn current(user_zomboid_dir: &str, force: bool) -> CommunityLists {
    let cached = load_cached(user_zomboid_dir);
    let fresh = cached.as_ref().and_then(|c| c.fetched_at).map(super::community::is_fresh).unwrap_or(false);
    let got = if !super::community::downloads_enabled(user_zomboid_dir) || (fresh && !force) { cached } else { refresh(user_zomboid_dir).ok().or(cached) };
    got.unwrap_or_else(|| parse(include_str!("../../../community/lists.json")).unwrap_or_default())
}

/// Makes a community list the player's own (named "<name> (<author>)", a number added when taken).
pub fn copy_to_my_lists(user_zomboid_dir: &str, shared: &SharedList) -> Result<crate::lists::ModList, String> {
    let existing = crate::lists::load_lists(user_zomboid_dir);
    let base = format!("{} ({})", shared.name, shared.author);
    let mut name = base.clone();
    let mut n = 2;
    while existing.iter().any(|l| l.name.eq_ignore_ascii_case(&name)) {
        name = format!("{} {}", base, n);
        n += 1;
    }
    let mut list = crate::lists::create_list(user_zomboid_dir, &name)?;
    for item in &shared.items {
        let entry = crate::lists::ListItem { workshop_id: item.workshop_id.clone(), mod_id: item.mod_id.clone(), nexus_id: item.nexus_id, name: item.name.clone(), added_at: String::new() };
        list = crate::lists::add_to_list(user_zomboid_dir, &list.id, entry)?;
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_becomes_a_file_name() {
        assert_eq!(slug("Survival Plus (B42)"), "survival-plus-b42");
        assert_eq!(slug("  Mis mods de vehículos!! "), "mis-mods-de-vehiculos");
        assert_eq!(slug("Año ñandú"), "ano-nandu");
    }

    #[test]
    fn a_list_is_checked_like_the_repository_does() {
        let mut list = SharedList {
            id: "cars".into(),
            name: "Cars".into(),
            author: "pichu66".into(),
            description: String::new(),
            build: "42".into(),
            updated: "2026-10-21".into(),
            items: vec![SharedItem { workshop_id: Some("2392709985".into()), mod_id: None, nexus_id: None, name: "Autotsar Trailers".into() }],
        };
        assert!(problems(&list).is_empty());
        list.items.push(SharedItem { workshop_id: Some("abc".into()), mod_id: None, nexus_id: None, name: "x".into() });
        list.items.push(SharedItem { workshop_id: None, mod_id: None, nexus_id: None, name: "y".into() });
        list.build = "40".into();
        assert_eq!(problems(&list).len(), 3);
    }

    #[test]
    fn the_lists_in_this_repository_parse() {
        let all = parse(include_str!("../../../community/lists.json")).expect("community/lists.json parses");
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../community/lists");
        let files = std::fs::read_dir(&dir).map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().extension().is_some_and(|x| x == "json")).count()).unwrap_or(0);
        assert_eq!(all.lists.len(), files, "lists.json is rebuilt from community/lists (node scripts/community-lists.js build)");
    }
}
