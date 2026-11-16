//! What a check reads from one Workshop mod, kept on disk until Steam updates the mod, so the
//! first check after the Studio starts does not read every file again. Read on a PC short of free
//! memory (the game and a server open), Windows had dropped the files from its cache and a first
//! check took one to five minutes on a 390-mod list; kept here, it is a file per mod.
//!
//! `Zomboid/PZModStudio_Cache/mods/<kind>/<hash>.json`, the key holding the mod's folder, Steam's
//! install time and size, the game build and whatever else the reading depends on. Only Workshop
//! mods (a local mod can change any time without a new install time). A cache that cannot be read
//! or written is ignored: the mod is read as before.

use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;

/// Bump when what is kept changes shape or meaning: every older file is then not read.
const VERSION: u32 = 1;

fn dir(kind: &str) -> Option<PathBuf> {
    if cfg!(test) {
        return None; // never the PC's own Zomboid folder from a test
    }
    let paths = crate::vfs::studio_paths();
    let z = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next()?;
    Some(z.join("PZModStudio_Cache").join("mods").join(kind))
}

fn file(kind: &str, key: &str) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (VERSION, key).hash(&mut h);
    Some(dir(kind)?.join(format!("{:016x}.json", h.finish())))
}

#[derive(serde::Deserialize)]
struct Kept<T> {
    key: String,
    value: T,
}

#[derive(Serialize)]
struct KeptRef<'a, T> {
    key: &'a str,
    value: &'a T,
}

pub fn load<T: DeserializeOwned>(kind: &str, key: &str) -> Option<T> {
    let text = std::fs::read_to_string(file(kind, key)?).ok()?;
    let kept: Kept<T> = serde_json::from_str(&text).ok()?;
    // The hash names the file; the key inside says it is really this mod's.
    (kept.key == key).then_some(kept.value)
}

pub fn save<T: Serialize>(kind: &str, key: &str, value: &T) {
    let Some(path) = file(kind, key) else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(&KeptRef { key, value }) {
        let _ = crate::load_order::ini_parser::atomic_write(&path, text);
    }
}

/// The key of a Workshop mod as installed: folder, Steam's install time and size, and the rest.
pub fn workshop_key(m: &crate::catalog::CatalogMod, extra: &str) -> Option<String> {
    let s = m.workshop_state.as_ref()?;
    Some(format!("{}|{}|{:?}|{}", m.root_dir, s.installed_time?, s.size, extra))
}
