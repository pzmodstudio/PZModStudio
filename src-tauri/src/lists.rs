//! The player's own lists of mods — Favorites and any they make — kept in
//! `Zomboid/PZModStudio_Lists.json`. What a Workshop collection is, but the player's, offline,
//! and able to hold a mod from anywhere: a Workshop item by its id, a local or Nexus mod by its
//! mod id. A list is for keeping, subscribing all at once, or making a profile from.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use crate::load_order::mod_info::get_existing_user_zomboid_dirs;

const LISTS_FILE: &str = "PZModStudio_Lists.json";
/// The list every player has, made on first use.
pub const FAVORITES: &str = "favorites";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListItem {
    #[serde(default)]
    pub workshop_id: Option<String>,
    #[serde(default)]
    pub mod_id: Option<String>,
    /// A Nexus mod not installed yet, by its id on Nexus.
    #[serde(default)]
    pub nexus_id: Option<u64>,
    /// What to call it when nothing else is at hand (the Workshop title, the mod's name).
    pub name: String,
    #[serde(default)]
    pub added_at: String,
}

impl ListItem {
    /// What makes two items the same: the Workshop item, else the Nexus mod, else the mod id.
    pub fn key(&self) -> String {
        self.workshop_id
            .clone()
            .map(|w| format!("w:{}", w))
            .or_else(|| self.nexus_id.map(|n| format!("n:{}", n)))
            .or_else(|| self.mod_id.clone().map(|m| format!("m:{}", m)))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModList {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ListsFile {
    #[serde(default)]
    lists: Vec<ModList>,
}

fn lists_path(user_zomboid_dir: &str) -> Result<PathBuf, String> {
    get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .map(|d| d.join(LISTS_FILE))
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))
}

fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Every list, Favorites first; Favorites exists even before anything is in it.
pub fn load_lists(user_zomboid_dir: &str) -> Vec<ModList> {
    let mut lists = lists_path(user_zomboid_dir)
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<ListsFile>(&t).ok())
        .map(|f| f.lists)
        .unwrap_or_default();
    if let Some(i) = lists.iter().position(|l| l.id == FAVORITES) {
        if i != 0 {
            let fav = lists.remove(i);
            lists.insert(0, fav);
        }
    } else {
        lists.insert(0, ModList { id: FAVORITES.into(), name: "Favorites".into(), created_at: now(), items: vec![] });
    }
    lists
}

fn save(user_zomboid_dir: &str, lists: &[ModList]) -> Result<(), String> {
    let path = lists_path(user_zomboid_dir)?;
    let json = serde_json::to_string_pretty(&ListsFile { lists: lists.to_vec() }).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::guard_overwrite::<ListsFile>(&path)?;
    crate::load_order::ini_parser::atomic_write(path, json).map_err(|e| e.to_string())
}

/// A readable id from the name, unique among the lists.
fn id_for(name: &str, lists: &[ModList]) -> String {
    let base: String = name.trim().to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let base = base.trim_matches('-').to_string();
    let base = if base.is_empty() { "list".to_string() } else { base };
    let mut id = base.clone();
    let mut n = 2;
    while lists.iter().any(|l| l.id == id) {
        id = format!("{}-{}", base, n);
        n += 1;
    }
    id
}

pub fn create_list(user_zomboid_dir: &str, name: &str) -> Result<ModList, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A list needs a name.".to_string());
    }
    let mut lists = load_lists(user_zomboid_dir);
    if lists.iter().any(|l| l.name.eq_ignore_ascii_case(name)) {
        return Err(format!("There is already a list called {}.", name));
    }
    let list = ModList { id: id_for(name, &lists), name: name.to_string(), created_at: now(), items: vec![] };
    lists.push(list.clone());
    save(user_zomboid_dir, &lists)?;
    Ok(list)
}

pub fn rename_list(user_zomboid_dir: &str, list_id: &str, name: &str) -> Result<ModList, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A list needs a name.".to_string());
    }
    let mut lists = load_lists(user_zomboid_dir);
    let list = lists.iter_mut().find(|l| l.id == list_id).ok_or_else(|| format!("There is no list {}.", list_id))?;
    list.name = name.to_string();
    let out = list.clone();
    save(user_zomboid_dir, &lists)?;
    Ok(out)
}

/// Favorites stays; any other list can go.
pub fn delete_list(user_zomboid_dir: &str, list_id: &str) -> Result<Vec<ModList>, String> {
    if list_id == FAVORITES {
        return Err("Favorites cannot be deleted; take the mods out of it instead.".to_string());
    }
    let mut lists = load_lists(user_zomboid_dir);
    let before = lists.len();
    lists.retain(|l| l.id != list_id);
    if lists.len() == before {
        return Err(format!("There is no list {}.", list_id));
    }
    save(user_zomboid_dir, &lists)?;
    Ok(lists)
}

/// Puts a mod in a list, once: the same Workshop item or mod id twice is one entry.
pub fn add_to_list(user_zomboid_dir: &str, list_id: &str, mut item: ListItem) -> Result<ModList, String> {
    if item.key().is_empty() {
        return Err("A list entry needs a Workshop id or a mod id.".to_string());
    }
    let mut lists = load_lists(user_zomboid_dir);
    let list = lists.iter_mut().find(|l| l.id == list_id).ok_or_else(|| format!("There is no list {}.", list_id))?;
    if !list.items.iter().any(|i| i.key() == item.key()) {
        if item.added_at.is_empty() {
            item.added_at = now();
        }
        list.items.push(item);
    }
    let out = list.clone();
    save(user_zomboid_dir, &lists)?;
    Ok(out)
}

/// Takes a mod out of a list, by its key (`w:<workshop id>` or `m:<mod id>`), a Workshop id, or a mod id.
pub fn remove_from_list(user_zomboid_dir: &str, list_id: &str, key: &str) -> Result<ModList, String> {
    let mut lists = load_lists(user_zomboid_dir);
    let list = lists.iter_mut().find(|l| l.id == list_id).ok_or_else(|| format!("There is no list {}.", list_id))?;
    let before = list.items.len();
    list.items.retain(|i| !(i.key() == key || i.workshop_id.as_deref() == Some(key) || i.mod_id.as_deref() == Some(key)));
    if list.items.len() == before {
        return Err(format!("{} is not in {}.", key, list.name));
    }
    let out = list.clone();
    save(user_zomboid_dir, &lists)?;
    Ok(out)
}

// ------------------------------------------------------------------ the game's favourites

/// The game's own Mods screen (and the Studio Link's) keeps its stars in `Zomboid/Lua/` in this
/// file, first line `!fav!:id;id;`.
const GAME_FAVOURITES_FILE: &str = "pz_modlist_settings.cfg";
/// What the game's stars were the last time they were synced, so a star taken off on either side
/// is told apart from one never put there.
const GAME_FAVOURITES_SEEN: &str = "PZModStudio_GameFavourites.json";

/// An installed mod as the sync needs it.
#[derive(Debug, Clone, Deserialize)]
pub struct InstalledMod {
    pub mod_id: String,
    #[serde(default)]
    pub workshop_id: Option<String>,
    pub name: String,
}

fn plain(id: &str) -> String {
    id.trim().trim_start_matches('\\').to_string()
}

/// A real Workshop id (local mods carry none, or the placeholder 9999999999).
fn workshop_of(m: &InstalledMod) -> Option<&str> {
    m.workshop_id.as_deref().filter(|w| !w.is_empty() && *w != "0" && *w != "9999999999")
}

/// The game's stars and the rest of its file, kept as they are when the stars are written back.
fn read_game_favourites(lua: &std::path::Path) -> (BTreeSet<String>, Vec<String>) {
    let mut stars = BTreeSet::new();
    let mut others = Vec::new();
    let Ok(text) = fs::read_to_string(lua.join(GAME_FAVOURITES_FILE)) else { return (stars, others) };
    for (i, line) in text.lines().enumerate() {
        match line.strip_prefix("!fav!:") {
            Some(ids) if i == 0 => stars.extend(ids.split(';').map(plain).filter(|s| !s.is_empty())),
            _ if !line.is_empty() => others.push(line.to_string()),
            _ => {}
        }
    }
    (stars, others)
}

/// Favorites and the game's stars made one: a star put or taken off in the game since the last
/// sync is put or taken off here, and the game then gets every favourite that is installed (a
/// Workshop item stands for all its mods), plus what it had that is not installed here.
/// Nothing is written while the game's Lua folder is not there yet.
pub fn sync_game_favourites(user_zomboid_dir: &str, installed: &[InstalledMod]) -> Result<Vec<ModList>, String> {
    let mut lists = load_lists(user_zomboid_dir);
    let Some(z) = get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next() else { return Ok(lists) };
    let lua = z.join("Lua");
    if !lua.is_dir() {
        return Ok(lists);
    }
    let (game, others) = read_game_favourites(&lua);
    let seen_path = z.join(GAME_FAVOURITES_SEEN);
    let seen: BTreeSet<String> = fs::read_to_string(&seen_path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let added: Vec<&String> = game.difference(&seen).collect();
    let removed: BTreeSet<&String> = seen.difference(&game).collect();
    let by_id: std::collections::HashMap<&str, &InstalledMod> = installed.iter().map(|m| (m.mod_id.as_str(), m)).collect();
    let ids_of = |item: &ListItem| -> Vec<String> {
        match (&item.workshop_id, &item.mod_id) {
            (Some(w), _) => installed.iter().filter(|m| workshop_of(m) == Some(w.as_str())).map(|m| m.mod_id.clone()).collect(),
            (None, Some(id)) if by_id.contains_key(id.as_str()) => vec![id.clone()],
            _ => Vec::new(),
        }
    };

    let fav = &mut lists[0];
    let before = fav.items.clone();
    fav.items.retain(|item| !ids_of(item).iter().any(|id| removed.contains(id)));
    for id in added {
        if fav.items.iter().any(|item| ids_of(item).contains(id)) {
            continue;
        }
        if let Some(m) = by_id.get(id.as_str()) {
            let workshop = workshop_of(m).map(str::to_string);
            fav.items.push(ListItem { mod_id: if workshop.is_some() { None } else { Some(id.clone()) }, workshop_id: workshop, nexus_id: None, name: m.name.clone(), added_at: now() });
        }
    }
    let mut stars: BTreeSet<String> = fav.items.iter().flat_map(&ids_of).collect();
    stars.extend(game.iter().filter(|id| !by_id.contains_key(id.as_str()) && !removed.contains(id)).cloned());
    let changed = fav.items != before;

    if stars != game {
        let mut text = format!("!fav!:{}\n", stars.iter().map(|id| format!("{};", id)).collect::<String>());
        for line in &others {
            text.push_str(line);
            text.push('\n');
        }
        crate::load_order::ini_parser::atomic_write(lua.join(GAME_FAVOURITES_FILE), text).map_err(|e| e.to_string())?;
    }
    if stars != seen {
        let json = serde_json::to_string(&stars).map_err(|e| e.to_string())?;
        crate::load_order::ini_parser::atomic_write(&seen_path, json).map_err(|e| e.to_string())?;
    }
    if changed {
        save(user_zomboid_dir, &lists)?;
    }
    Ok(lists)
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
        let t = Tmp(std::env::temp_dir().join(format!("pzms-lists-{}-{}", tag, std::process::id())));
        let _ = fs::remove_dir_all(&t.0);
        fs::create_dir_all(t.0.join("mods")).unwrap();
        let z = t.0.to_string_lossy().to_string();
        (t, z)
    }

    fn item(w: Option<&str>, m: Option<&str>) -> ListItem {
        ListItem { workshop_id: w.map(str::to_string), mod_id: m.map(str::to_string), nexus_id: None, name: "X".into(), added_at: String::new() }
    }

    #[test]
    fn favorites_is_always_there_and_a_mod_goes_in_once() {
        let (_t, z) = zomboid("fav");
        let lists = load_lists(&z);
        assert_eq!(lists[0].id, FAVORITES);
        add_to_list(&z, FAVORITES, item(Some("123"), Some("Mod"))).unwrap();
        let fav = add_to_list(&z, FAVORITES, item(Some("123"), None)).unwrap();
        assert_eq!(fav.items.len(), 1, "the same Workshop item twice is one entry");
        assert!(!fav.items[0].added_at.is_empty());
        assert!(add_to_list(&z, FAVORITES, item(None, None)).is_err());
        assert!(delete_list(&z, FAVORITES).is_err());
        let fav = remove_from_list(&z, FAVORITES, "123").unwrap();
        assert!(fav.items.is_empty());
    }

    #[test]
    fn a_list_gets_a_readable_unique_id_and_can_be_renamed_and_deleted() {
        let (_t, z) = zomboid("make");
        let a = create_list(&z, "Zombie overhauls!").unwrap();
        assert_eq!(a.id, "zombie-overhauls");
        assert!(create_list(&z, "zombie overhauls!").is_err(), "the same name twice is refused");
        let b = create_list(&z, "zombie-overhauls").unwrap();
        assert_eq!(b.id, "zombie-overhauls-2");
        add_to_list(&z, &a.id, item(None, Some("LocalMod"))).unwrap();
        let renamed = rename_list(&z, &a.id, "Overhauls").unwrap();
        assert_eq!(renamed.name, "Overhauls");
        assert_eq!(renamed.items[0].key(), "m:LocalMod");
        let left = delete_list(&z, &b.id).unwrap();
        assert_eq!(left.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(), vec![FAVORITES, "zombie-overhauls"]);
        assert!(remove_from_list(&z, &a.id, "nope").is_err());
    }

    #[test]
    fn favourites_and_the_games_stars_are_one_list() {
        let (_t, z) = zomboid("favs");
        let lua = PathBuf::from(&z).join("Lua");
        fs::create_dir_all(&lua).unwrap();
        fs::write(lua.join(GAME_FAVOURITES_FILE), "!fav!:\\B;Gone;\nother=1\n").unwrap();
        let mods = |list: &[(&str, &str)]| list.iter().map(|(id, w)| InstalledMod { mod_id: id.to_string(), workshop_id: Some(w.to_string()), name: id.to_string() }).collect::<Vec<_>>();
        let installed = mods(&[("A", "111"), ("A2", "111"), ("B", ""), ("C", "333")]);
        add_to_list(&z, FAVORITES, item(Some("111"), None)).unwrap();
        // First sync: both sides put together.
        let lists = sync_game_favourites(&z, &installed).unwrap();
        assert_eq!(lists[0].items.iter().map(|i| i.key()).collect::<Vec<_>>(), vec!["w:111", "m:B"]);
        let file = fs::read_to_string(lua.join(GAME_FAVOURITES_FILE)).unwrap();
        assert_eq!(file, "!fav!:A;A2;B;Gone;\nother=1\n", "a Workshop item is all its mods; what is not installed here stays");
        // Taken off in the game, starred in the game: the same here.
        fs::write(lua.join(GAME_FAVOURITES_FILE), "!fav!:B;C;Gone;\nother=1\n").unwrap();
        let lists = sync_game_favourites(&z, &installed).unwrap();
        assert_eq!(lists[0].items.iter().map(|i| i.key()).collect::<Vec<_>>(), vec!["m:B", "w:333"]);
        // Taken off here: off in the game.
        remove_from_list(&z, FAVORITES, "m:B").unwrap();
        sync_game_favourites(&z, &installed).unwrap();
        assert!(fs::read_to_string(lua.join(GAME_FAVOURITES_FILE)).unwrap().starts_with("!fav!:C;Gone;\n"));
    }
}
