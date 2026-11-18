//! Mods the active profile has on that a save does not load.
//!
//! In Build 42 a save loads with its own mod list (`mods.txt`), whatever the profile says, so a
//! mod switched on in the profile after the save was made is simply not there when it loads. The
//! player is offered to add those mods to the save: it is backed up first, then they go into its
//! list where the profile's load order puts them (the save's own mods keep their places), and a
//! mod that brings map folders gets them in the save's map list too (`maps::add_maps_of`): a save
//! keeps the map folders it was made with, and without that the mod would load without its map.

use super::{backup_save, maps, save_dir, save_mods, REASON_BEFORE_FIX};
use crate::map_cells::MapCells;
use serde::Serialize;
use std::fs;
use std::path::Path;

/// What the active profile has on that one save does not load.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProfileModsForSave {
    pub profile_id: String,
    pub profile_name: String,
    /// In the profile's load order.
    pub mods: Vec<String>,
    /// Those of them that bring map folders (only when the map data was read).
    pub map_mods: Vec<String>,
}

/// What adding them did.
#[derive(Debug, Clone, Serialize)]
pub struct AddedToSave {
    pub added: Vec<String>,
    /// The save's list now, in order.
    pub list: Vec<String>,
    /// The backup zip made first.
    pub backup: String,
    /// The map folders the added mods brought, put in the save's map list.
    pub maps: Option<maps::SaveMapsFix>,
}

/// `\Mod`, `12345\Mod`, `12345/Mod` -> `Mod`.
pub fn plain(id: &str) -> String {
    crate::load_order::mod_info::sanitize_mod_id(id).rsplit(['/', '\\']).next().unwrap_or_default().to_string()
}

/// The mods on in the profile (in its order) that the save's list lacks. The Studio Link is left
/// out: it is the Studio's own, not something the player switched on.
pub fn not_in_save(profile_on: &[String], save_list: &[String]) -> Vec<String> {
    let have: Vec<String> = save_list.iter().map(|m| plain(m)).collect();
    let mut out: Vec<String> = Vec::new();
    for id in profile_on {
        let p = plain(id);
        if p.is_empty() || p == crate::studio_link::LINK_MOD_ID || have.contains(&p) || out.contains(&p) {
            continue;
        }
        out.push(p);
    }
    out
}

/// The save's list with `add` put in where the profile's order places them: each one right after
/// the nearest mod before it in the profile that the list already has, else right before the
/// nearest one after it, else at the end. The save's own mods keep their order and their form.
pub fn merge_in_profile_order(save_list: &[String], profile_on: &[String], add: &[String]) -> Vec<String> {
    let profile: Vec<String> = profile_on.iter().map(|m| plain(m)).collect();
    let mut out: Vec<String> = save_list.to_vec();
    let mut adding: Vec<String> = add.iter().map(|m| plain(m)).filter(|m| !m.is_empty()).collect();
    adding.sort_by_key(|m| profile.iter().position(|p| p == m).unwrap_or(usize::MAX));
    for id in adding {
        if out.iter().any(|m| plain(m) == id) {
            continue;
        }
        let at = profile.iter().position(|p| p == &id);
        let place = at.and_then(|at| {
            let before = profile[..at].iter().rev().find_map(|p| out.iter().position(|m| &plain(m) == p)).map(|i| i + 1);
            before.or_else(|| profile[at + 1..].iter().find_map(|p| out.iter().position(|m| &plain(m) == p)))
        });
        match place {
            Some(i) => out.insert(i, id),
            None => out.push(id),
        }
    }
    out
}

/// `mods.txt` with its `mods` block holding `list`, everything else (the maps block, line ends)
/// as it was. None when it has no mods block.
pub fn with_mods_block(text: &str, list: &[String]) -> Option<String> {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let head = lines.iter().position(|l| {
        let t = l.trim();
        t == "mods" || t.starts_with("mods ") || t.starts_with("mods{")
    })?;
    let open = (head..lines.len()).find(|&i| lines[i].contains('{'))?;
    let close = (open + 1..lines.len()).find(|&i| lines[i].trim_start().starts_with('}'))?;
    let mut out = String::new();
    for l in &lines[..=open] {
        out.push_str(l);
    }
    if !out.ends_with('\n') {
        out.push_str(eol);
    }
    for id in list {
        out.push_str(&format!("    mod = {},{}", id, eol));
    }
    for l in &lines[close..] {
        out.push_str(l);
    }
    Some(out)
}

/// What the profile has on that this save lacks, or None when there is nothing to offer: the save
/// keeps no list of its own (a server's) or an empty one (the Link asks about that apart), or
/// lacks nothing.
pub fn for_save(save: &Path, profile_id: &str, profile_name: &str, profile_on: &[String], cells: Option<&MapCells>) -> Option<ProfileModsForSave> {
    if !save.join("mods.txt").is_file() {
        return None;
    }
    let list = save_mods(save);
    if list.is_empty() {
        return None;
    }
    let mods = not_in_save(profile_on, &list);
    if mods.is_empty() {
        return None;
    }
    let map_mods = cells
        .map(|c| mods.iter().filter(|m| c.maps.iter().any(|f| &f.mod_id == *m)).cloned().collect())
        .unwrap_or_default();
    Some(ProfileModsForSave { profile_id: profile_id.to_string(), profile_name: profile_name.to_string(), mods, map_mods })
}

/// Backs the save up, then adds the profile's mods it lacks to its list (only those of `only`,
/// when given), in the profile's order, and the map folders those mods bring to its map list.
/// The caller makes sure the game is not playing it (at the main menu is fine: the game reads the
/// list when the save loads).
pub fn add_profile_mods(user_zomboid_dir: &Path, pz_install_dir: &str, cells: Option<&MapCells>, save_id: &str, profile_on: &[String], only: Option<&[String]>) -> Result<AddedToSave, String> {
    let save = save_dir(user_zomboid_dir, save_id)?;
    let path = save.join("mods.txt");
    let text = fs::read_to_string(&path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let list = save_mods(&save);
    let mut add = not_in_save(profile_on, &list);
    if let Some(only) = only {
        let wanted: Vec<String> = only.iter().map(|m| plain(m)).collect();
        add.retain(|m| wanted.contains(m));
    }
    if add.is_empty() {
        return Err(format!("{} already has every mod the profile has on.", save_id));
    }
    let new_list = merge_in_profile_order(&list, profile_on, &add);
    let new_text = with_mods_block(&text, &new_list).ok_or_else(|| format!("{} has no mod list to add to.", path.display()))?;
    let backup = backup_save(user_zomboid_dir, save_id, REASON_BEFORE_FIX, false)?.entry.file;
    crate::load_order::ini_parser::atomic_write(&path, new_text).map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
    let maps = match cells {
        Some(cells) => maps::add_maps_of(user_zomboid_dir, pz_install_dir, cells, save_id, &add, &backup)?,
        None => None,
    };
    Ok(AddedToSave { added: add, list: new_list, backup, maps })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn only_the_profiles_mods_the_save_lacks_are_offered_in_profile_order() {
        let profile = ids(&["ZombieBuddy", "Z_StudioLink", "B", "NewOne", "\\A", "Later"]);
        let save = ids(&["A", "12345\\B", "OwnOnly"]);
        assert_eq!(not_in_save(&profile, &save), ids(&["ZombieBuddy", "NewOne", "Later"]));
        assert!(not_in_save(&ids(&["A"]), &ids(&["\\A"])).is_empty());
    }

    #[test]
    fn added_mods_go_where_the_profile_order_puts_them() {
        let profile = ids(&["First", "A", "Mid", "B", "C", "Last"]);
        // The save has A, C, B (its own order) and a mod of its own.
        let save = ids(&["\\A", "C", "Own", "B"]);
        let merged = merge_in_profile_order(&save, &profile, &ids(&["Last", "Mid", "First"]));
        // First: nothing before it in the profile is in the list, so before A. Mid: after A.
        // Last: after C (the nearest before it in the profile that the list has).
        assert_eq!(merged, ids(&["First", "\\A", "Mid", "C", "Last", "Own", "B"]));
        // A mod the profile does not order goes last; one already there is not doubled.
        assert_eq!(merge_in_profile_order(&ids(&["A"]), &ids(&["A"]), &ids(&["X", "A"])), ids(&["A", "X"]));
    }

    #[test]
    fn the_mods_block_is_rewritten_and_the_rest_kept() {
        let text = "VERSION = 1,\r\n\r\nmods\r\n{\r\n    mod = A,\r\n    mod = \\B,\r\n}\r\n\r\nmaps\r\n{\r\n    map = Muldraugh, KY,\r\n}\r\n";
        let out = with_mods_block(text, &ids(&["A", "New", "\\B"])).unwrap();
        assert_eq!(out, "VERSION = 1,\r\n\r\nmods\r\n{\r\n    mod = A,\r\n    mod = New,\r\n    mod = \\B,\r\n}\r\n\r\nmaps\r\n{\r\n    map = Muldraugh, KY,\r\n}\r\n");
        let empty = "VERSION = 1,\n\nmods\n{\n}\n\nmaps\n{\n}\n";
        assert_eq!(with_mods_block(empty, &ids(&["X"])).unwrap(), "VERSION = 1,\n\nmods\n{\n    mod = X,\n}\n\nmaps\n{\n}\n");
        assert!(with_mods_block("VERSION = 1,\n", &ids(&["X"])).is_none());
    }

    #[test]
    fn adding_backs_the_save_up_first_and_brings_the_added_mods_maps() {
        let z = std::env::temp_dir().join(format!("pz-profile-mods-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        let save = z.join("Saves").join("Sandbox").join("w");
        fs::create_dir_all(save.join("map").join("2873")).unwrap();
        fs::write(save.join("map").join("2873").join("1546.bin"), b"bare").unwrap();
        fs::write(save.join("mods.txt"), "VERSION = 1,\n\nmods\n{\n    mod = A,\n    mod = C,\n}\n\nmaps\n{\n}\n").unwrap();
        maps::write_map_ver(&save, 249, &["Muldraugh, KY".to_string()]).unwrap();
        let cells = MapCells {
            vanilla: vec![],
            maps: vec![crate::map_cells::MapFolder {
                mod_id: "MapMod".into(),
                mod_name: "Map Mod".into(),
                workshop_id: None,
                folder: "map_mod".into(),
                title: "map_mod".into(),
                lots: vec![],
                active: true,
                cells: vec![[89, 48]],
                over_vanilla: 0,
                dir: std::path::PathBuf::new(),
            }],
            related: vec![],
            clashes: vec![],
            tiledef_clashes: vec![],
        };
        let profile = ids(&["A", "B", "C", "MapMod", "Z_StudioLink"]);
        let offer = for_save(&save, "p1", "Test", &profile, Some(&cells)).unwrap();
        assert_eq!(offer.mods, ids(&["B", "MapMod"]));
        assert_eq!(offer.map_mods, ids(&["MapMod"]));

        // Only what the player was shown is added.
        let out = add_profile_mods(&z, "", Some(&cells), "Sandbox/w", &profile, Some(&ids(&["B", "MapMod", "Other"]))).unwrap();
        assert_eq!(out.added, ids(&["B", "MapMod"]));
        assert_eq!(save_mods(&save), ids(&["A", "B", "C", "MapMod"]));
        assert!(fs::read_to_string(save.join("mods.txt")).unwrap().ends_with("}\n\nmaps\n{\n}\n"));
        let fix = out.maps.unwrap();
        assert_eq!(fix.added, ids(&["map_mod"]));
        assert_eq!(maps::read_map_ver(&save).unwrap().1, ids(&["map_mod", "Muldraugh, KY"]));
        assert_eq!(fix.moved_chunks, 1);
        assert_eq!(fix.backup, out.backup);
        // The backup holds the save as it was before.
        assert!(z.join("PZModStudio_SaveBackups").join("Sandbox").join("w").join(&out.backup).exists());
        // Nothing left to add.
        assert!(for_save(&save, "p1", "Test", &profile, Some(&cells)).is_none());
        assert!(add_profile_mods(&z, "", None, "Sandbox/w", &profile, None).is_err());
        let _ = fs::remove_dir_all(&z);
    }
}
