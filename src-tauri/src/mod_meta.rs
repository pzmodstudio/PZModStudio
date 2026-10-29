//! The player's own notes on mods and their own groups of mods, kept by the app in
//! `<Zomboid>/PZModStudio_ModMeta.json`. The Link reads them in the game from
//! `<Zomboid>/Lua/PZModStudio_LinkMeta.txt` (`note=<id>\t<text>`, `group=<name>\t<id;id;...>`),
//! written whenever they change: in the game they are shown, and a group can be turned on or
//! off; making and editing them is the app's.
//!
//! A note is about a mod and is the same in every profile. A group belongs to the profile it was
//! made in: what is read and changed here is always the active profile's groups, and the Link's
//! copy holds only those.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const META_FILE: &str = "PZModStudio_ModMeta.json";
pub const LINK_META_FILE: &str = "PZModStudio_LinkMeta.txt";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModGroup {
    pub name: String,
    pub mods: Vec<String>,
    /// The player's note on the group.
    #[serde(default)]
    pub note: String,
    /// The profile it belongs to; none for a group made before groups had one, or with no
    /// profile at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModMeta {
    /// Mod id -> the player's note.
    #[serde(default)]
    pub notes: BTreeMap<String, String>,
    #[serde(default)]
    pub groups: Vec<ModGroup>,
}

fn path(user_zomboid_dir: &Path) -> PathBuf {
    user_zomboid_dir.join(META_FILE)
}

/// Everything on disk, every profile's groups.
fn read_all(user_zomboid_dir: &Path) -> ModMeta {
    std::fs::read_to_string(path(user_zomboid_dir)).ok().and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok()).unwrap_or_default()
}

fn active_profile(user_zomboid_dir: &Path) -> Option<String> {
    crate::instance_manager::active_profile(&user_zomboid_dir.to_string_lossy()).map(|(id, _)| id)
}

/// Each profile's id and mods, on or off.
fn profiles(user_zomboid_dir: &Path) -> Vec<(String, Vec<String>)> {
    crate::instance_manager::list_instances(user_zomboid_dir.to_string_lossy().to_string())
        .unwrap_or_default()
        .into_iter()
        .map(|p| {
            let members = p.member_ids();
            (p.id, members)
        })
        .collect()
}

/// Groups made before they belonged to a profile go to every profile holding one of their mods,
/// or to the active one when none does. True when something moved (to be saved).
fn adopt_shared_groups(meta: &mut ModMeta, active: Option<&str>, profiles: &[(String, Vec<String>)]) -> bool {
    let Some(active) = active else { return false };
    if !meta.groups.iter().any(|g| g.profile.is_none()) {
        return false;
    }
    let mut out: Vec<ModGroup> = Vec::new();
    for g in std::mem::take(&mut meta.groups) {
        if g.profile.is_some() {
            out.push(g);
            continue;
        }
        let mut owners: Vec<&str> = profiles.iter().filter(|(_, members)| g.mods.iter().any(|id| members.contains(id))).map(|(id, _)| id.as_str()).collect();
        if owners.is_empty() {
            owners.push(active);
        }
        for owner in owners {
            if !out.iter().any(|x| x.profile.as_deref() == Some(owner) && x.name.eq_ignore_ascii_case(&g.name)) {
                out.push(ModGroup { profile: Some(owner.to_string()), ..g.clone() });
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    meta.groups = out;
    true
}

/// What the active profile sees: every note, its own groups.
fn scoped(meta: &ModMeta, active: Option<&str>) -> ModMeta {
    ModMeta { notes: meta.notes.clone(), groups: meta.groups.iter().filter(|g| g.profile.as_deref() == active).cloned().collect() }
}

/// Everything on disk with the old shared groups given out, and the active profile's id.
fn load(user_zomboid_dir: &Path) -> (ModMeta, Option<String>) {
    let active = active_profile(user_zomboid_dir);
    let mut all = read_all(user_zomboid_dir);
    if adopt_shared_groups(&mut all, active.as_deref(), &profiles(user_zomboid_dir)) {
        let _ = save(user_zomboid_dir, &all, active.as_deref());
    }
    (all, active)
}

/// The notes and the active profile's groups.
pub fn read(user_zomboid_dir: &Path) -> ModMeta {
    let (all, active) = load(user_zomboid_dir);
    scoped(&all, active.as_deref())
}

/// One line per note and per group; tabs and line breaks in a note become spaces (one line each).
pub fn link_text(meta: &ModMeta) -> String {
    let clean = |s: &str| s.replace(['\t', '\r', '\n'], " ").trim().to_string();
    let mut out = String::new();
    for (id, note) in &meta.notes {
        if !note.trim().is_empty() {
            out.push_str(&format!("note={}\t{}\n", id, clean(note)));
        }
    }
    for g in &meta.groups {
        out.push_str(&format!("group={}\t{}\n", clean(&g.name), g.mods.join(";")));
        if !g.note.trim().is_empty() {
            out.push_str(&format!("groupnote={}\t{}\n", clean(&g.name), clean(&g.note)));
        }
    }
    out
}

/// Saves everything, and writes the Link's copy of what the active profile sees (only when the
/// game's Lua folder is there).
fn save(user_zomboid_dir: &Path, all: &ModMeta, active: Option<&str>) -> Result<(), String> {
    let text = serde_json::to_string_pretty(all).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(path(user_zomboid_dir), text).map_err(|e| e.to_string())?;
    write_link_copy(user_zomboid_dir, &scoped(all, active));
    Ok(())
}

pub fn write_link_copy(user_zomboid_dir: &Path, meta: &ModMeta) {
    let lua = user_zomboid_dir.join("Lua");
    if !lua.is_dir() {
        return;
    }
    let file = lua.join(LINK_META_FILE);
    let text = link_text(meta);
    if std::fs::read_to_string(&file).is_ok_and(|now| now == text) {
        return;
    }
    let _ = crate::load_order::ini_parser::atomic_write(file, text);
}

/// The Link's copy for the profile active now (called when another one is activated).
pub fn refresh_link_copy(user_zomboid_dir: &Path) {
    write_link_copy(user_zomboid_dir, &read(user_zomboid_dir));
}

pub fn set_note(user_zomboid_dir: &Path, mod_id: &str, note: &str) -> Result<ModMeta, String> {
    let (mut all, active) = load(user_zomboid_dir);
    if note.trim().is_empty() {
        all.notes.remove(mod_id);
    } else {
        all.notes.insert(mod_id.to_string(), note.trim().to_string());
    }
    save(user_zomboid_dir, &all, active.as_deref())?;
    Ok(scoped(&all, active.as_deref()))
}

/// Puts a mod in a group of the active profile or takes it out; a group named for the first time
/// is made, one left empty goes away.
pub fn set_in_group(user_zomboid_dir: &Path, group: &str, mod_ids: &[String], inside: bool) -> Result<ModMeta, String> {
    let name = group.trim();
    if name.is_empty() {
        return Err("A group needs a name.".to_string());
    }
    let (mut all, active) = load(user_zomboid_dir);
    let mine = |g: &ModGroup| g.profile == active;
    let at = match all.groups.iter().position(|g| mine(g) && g.name.eq_ignore_ascii_case(name)) {
        Some(i) => i,
        None => {
            all.groups.push(ModGroup { name: name.to_string(), mods: Vec::new(), note: String::new(), profile: active.clone() });
            all.groups.len() - 1
        }
    };
    for id in mod_ids {
        let mods = &mut all.groups[at].mods;
        if inside {
            if !mods.contains(id) {
                mods.push(id.clone());
            }
        } else {
            mods.retain(|m| m != id);
        }
    }
    all.groups.retain(|g| !g.mods.is_empty());
    all.groups.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    save(user_zomboid_dir, &all, active.as_deref())?;
    Ok(scoped(&all, active.as_deref()))
}

pub fn rename_group(user_zomboid_dir: &Path, from: &str, to: &str) -> Result<ModMeta, String> {
    let to = to.trim();
    if to.is_empty() {
        return Err("A group needs a name.".to_string());
    }
    let (mut all, active) = load(user_zomboid_dir);
    if all.groups.iter().any(|g| g.profile == active && g.name.eq_ignore_ascii_case(to) && !g.name.eq_ignore_ascii_case(from)) {
        return Err(format!("There is already a group called {}.", to));
    }
    for g in all.groups.iter_mut() {
        if g.profile == active && g.name == from {
            g.name = to.to_string();
        }
    }
    save(user_zomboid_dir, &all, active.as_deref())?;
    Ok(scoped(&all, active.as_deref()))
}

pub fn set_group_note(user_zomboid_dir: &Path, name: &str, note: &str) -> Result<ModMeta, String> {
    let (mut all, active) = load(user_zomboid_dir);
    let Some(g) = all.groups.iter_mut().find(|g| g.profile == active && g.name == name) else { return Err(format!("There is no group called {}.", name)) };
    g.note = note.trim().to_string();
    save(user_zomboid_dir, &all, active.as_deref())?;
    Ok(scoped(&all, active.as_deref()))
}

pub fn delete_group(user_zomboid_dir: &Path, name: &str) -> Result<ModMeta, String> {
    let (mut all, active) = load(user_zomboid_dir);
    all.groups.retain(|g| !(g.profile == active && g.name == name));
    save(user_zomboid_dir, &all, active.as_deref())?;
    Ok(scoped(&all, active.as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("pzms-meta-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(d.join("Lua")).unwrap();
        d
    }

    #[test]
    fn notes_and_groups_are_kept_and_written_for_the_link() {
        let d = dir();
        set_note(&d, "A", "breaks with B\tsometimes\nreally").unwrap();
        set_in_group(&d, "Weapons", &["A".into(), "B".into()], true).unwrap();
        set_in_group(&d, "weapons", &["C".into()], true).unwrap();
        let meta = set_in_group(&d, "UI", &["D".into()], true).unwrap();
        assert_eq!(meta.groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), vec!["UI", "Weapons"]);
        assert_eq!(meta.groups[1].mods, vec!["A", "B", "C"], "names match without case");
        let link = std::fs::read_to_string(d.join("Lua").join(LINK_META_FILE)).unwrap();
        assert!(link.contains("note=A\tbreaks with B sometimes really\n"));
        assert!(link.contains("group=Weapons\tA;B;C\n"));
        let meta = set_in_group(&d, "UI", &["D".into()], false).unwrap();
        assert_eq!(meta.groups.len(), 1, "an empty group goes away");
        let meta = set_note(&d, "A", "  ").unwrap();
        assert!(meta.notes.is_empty(), "an empty note is taken away");
        assert!(rename_group(&d, "Weapons", "").is_err());
        assert_eq!(rename_group(&d, "Weapons", "Guns").unwrap().groups[0].name, "Guns");
        set_group_note(&d, "Guns", "load\tlast").unwrap();
        assert!(std::fs::read_to_string(d.join("Lua").join(LINK_META_FILE)).unwrap().contains("groupnote=Guns\tload last\n"));
        assert!(set_group_note(&d, "Nope", "x").is_err());
        assert!(delete_group(&d, "Guns").unwrap().groups.is_empty());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn groups_belong_to_a_profile() {
        let group = |name: &str, mods: &[&str], profile: Option<&str>| ModGroup {
            name: name.into(),
            mods: mods.iter().map(|m| m.to_string()).collect(),
            note: String::new(),
            profile: profile.map(String::from),
        };
        let mut meta = ModMeta {
            notes: BTreeMap::new(),
            groups: vec![group("Cars", &["A"], None), group("Lost", &["Z"], None), group("Mine", &["B"], Some("p2"))],
        };
        let profiles = vec![("p1".to_string(), vec!["A".to_string()]), ("p2".to_string(), vec!["A".to_string(), "B".to_string()])];
        assert!(!adopt_shared_groups(&mut meta.clone(), None, &profiles), "with no profile active nothing moves");
        assert!(adopt_shared_groups(&mut meta, Some("p1"), &profiles));
        let names = |p: &str| scoped(&meta, Some(p)).groups.into_iter().map(|g| g.name).collect::<Vec<_>>();
        assert_eq!(names("p1"), vec!["Cars", "Lost"], "an old group goes where its mods are, else to the active profile");
        assert_eq!(names("p2"), vec!["Cars", "Mine"]);
        assert!(!adopt_shared_groups(&mut meta, Some("p1"), &profiles), "given out once");
    }
}
