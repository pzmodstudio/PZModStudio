//! Kept versions of a mod: a copy of the folder as it is today, so that when Steam replaces it
//! with an update that breaks a save, the version that worked can be put back.
//!
//! Steam only ever gives you the newest copy of a Workshop item, and overwrites the folder in
//! place; there is no going back once it has. A kept version is a plain copy under
//! `Zomboid/PZModStudio_Snapshots/<folder>/<when>/`. Putting one back does not touch the Workshop
//! folder (Steam would just overwrite it again): it goes into `Zomboid/mods` as a local copy,
//! the way a compatible copy does, and the Studio launches the game with `-modfolders
//! mods,workshop,steam` so the copy wins over the Workshop original.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::load_order::mod_info::get_existing_user_zomboid_dirs;
use crate::origins::{self, ModOrigin};

const SNAPSHOTS_DIR: &str = "PZModStudio_Snapshots";
const MANIFEST: &str = "pzms_kept.json";

/// One kept version of a mod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeptVersion {
    pub mod_id: String,
    pub mod_name: String,
    /// The mod folder's name: what goes under Zomboid/mods when it is put back.
    pub folder: String,
    /// "2026-10-17 14:05".
    pub kept_at: String,
    /// `modversion=` from mod.info at the time, when it had one.
    pub version: Option<String>,
    pub workshop_id: Option<String>,
    /// Where the copy was taken from.
    pub source_dir: String,
    /// The kept copy itself.
    pub path: String,
    pub bytes: u64,
}

fn zomboid_dir(user_zomboid_dir: &str) -> Result<PathBuf, String> {
    get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))
}

fn dir_bytes(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Copies the mod's folder as it is now into the snapshots folder, with a manifest beside it.
pub fn keep_version(
    user_zomboid_dir: &str,
    mod_id: &str,
    mod_name: &str,
    source_root: &Path,
    version: Option<String>,
    workshop_id: Option<String>,
) -> Result<KeptVersion, String> {
    let zomboid = zomboid_dir(user_zomboid_dir)?;
    if !source_root.is_dir() {
        return Err(format!("{} is not a folder.", source_root.display()));
    }
    let folder = source_root
        .file_name()
        .ok_or("Invalid mod folder")?
        .to_string_lossy()
        .to_string();
    let now = chrono::Local::now();
    let dest = zomboid.join(SNAPSHOTS_DIR).join(&folder).join(now.format("%Y%m%d-%H%M%S").to_string());
    if dest.exists() {
        return Err("A version was kept this very second; try again.".to_string());
    }
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    crate::mod_install::copy_tree(source_root, &dest).map_err(|e| {
        let _ = fs::remove_dir_all(&dest);
        format!("Copying the mod failed: {}", e)
    })?;
    let kept = KeptVersion {
        mod_id: mod_id.to_string(),
        mod_name: mod_name.to_string(),
        folder,
        kept_at: now.format("%Y-%m-%d %H:%M").to_string(),
        version,
        workshop_id,
        source_dir: source_root.to_string_lossy().to_string(),
        path: dest.to_string_lossy().to_string(),
        bytes: dir_bytes(&dest),
    };
    let json = serde_json::to_string_pretty(&kept).map_err(|e| e.to_string())?;
    fs::write(dest.join(MANIFEST), json).map_err(|e| e.to_string())?;
    Ok(kept)
}

/// Every kept version on disk, newest first.
pub fn list_kept_versions(user_zomboid_dir: &str) -> Vec<KeptVersion> {
    let Ok(zomboid) = zomboid_dir(user_zomboid_dir) else { return Vec::new() };
    let mut found = Vec::new();
    let Ok(folders) = fs::read_dir(zomboid.join(SNAPSHOTS_DIR)) else { return Vec::new() };
    for folder in folders.flatten() {
        let Ok(versions) = fs::read_dir(folder.path()) else { continue };
        for version in versions.flatten() {
            let Ok(text) = fs::read_to_string(version.path().join(MANIFEST)) else { continue };
            if let Ok(mut kept) = serde_json::from_str::<KeptVersion>(&text) {
                // The folder may have been moved: what is on disk is the truth.
                kept.path = version.path().to_string_lossy().to_string();
                found.push(kept);
            }
        }
    }
    found.sort_by(|a, b| b.kept_at.cmp(&a.kept_at));
    found
}

/// The kept version at `path`, which has to be inside the snapshots folder.
fn kept_at(zomboid: &Path, path: &str) -> Result<(PathBuf, KeptVersion), String> {
    let dir = PathBuf::from(path);
    let root = zomboid.join(SNAPSHOTS_DIR);
    let inside = dir
        .canonicalize()
        .ok()
        .zip(root.canonicalize().ok())
        .map(|(d, r)| d.starts_with(&r))
        .unwrap_or(false);
    if !inside {
        return Err("That is not one of the Studio's kept versions.".to_string());
    }
    let text = fs::read_to_string(dir.join(MANIFEST)).map_err(|_| "This kept version has no manifest.".to_string())?;
    let kept = serde_json::from_str::<KeptVersion>(&text).map_err(|e| e.to_string())?;
    Ok((dir, kept))
}

/// Puts a kept version back, as a local copy in Zomboid/mods that wins over the Workshop
/// original. The kept copy itself stays, so this can be done again.
pub fn restore_kept_version(user_zomboid_dir: &str, path: &str) -> Result<KeptVersion, String> {
    let zomboid = zomboid_dir(user_zomboid_dir)?;
    let (dir, kept) = kept_at(&zomboid, path)?;
    let dest = zomboid.join("mods").join(&kept.folder);
    if dest.exists() {
        let is_copy = matches!(origins::load(&zomboid).get(&kept.folder).map(|r| &r.origin), Some(ModOrigin::Compat { .. }));
        if !is_copy {
            return Err(format!(
                "Zomboid/mods/{} already exists and is not a copy the Studio made; move it away first.",
                kept.folder
            ));
        }
        let backup = zomboid
            .join("PZModStudio_Backups")
            .join("mods")
            .join(format!("{}-{}", kept.folder, chrono::Local::now().format("%Y%m%d-%H%M%S")));
        fs::create_dir_all(backup.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::rename(&dest, &backup).map_err(|e| format!("Cannot move the old copy aside: {}", e))?;
    }
    crate::mod_install::copy_tree(&dir, &dest).map_err(|e| {
        let _ = fs::remove_dir_all(&dest);
        format!("Copying the kept version failed: {}", e)
    })?;
    let _ = fs::remove_file(dest.join(MANIFEST));
    origins::record(
        &zomboid,
        std::slice::from_ref(&kept.folder),
        &ModOrigin::Compat {
            workshop_id: kept.workshop_id.clone(),
            source_dir: kept.path.clone(),
            changes: vec![format!(
                "The version kept on {}{}, put back by the Studio; the Workshop copy is ignored while this is here.",
                kept.kept_at,
                kept.version.as_ref().map(|v| format!(" (modversion {})", v)).unwrap_or_default()
            )],
        },
    )?;
    Ok(kept)
}

/// Deletes one kept version from disk.
pub fn delete_kept_version(user_zomboid_dir: &str, path: &str) -> Result<(), String> {
    let zomboid = zomboid_dir(user_zomboid_dir)?;
    let (dir, _) = kept_at(&zomboid, path)?;
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    // An empty folder for the mod is noise.
    if let Some(parent) = dir.parent() {
        if fs::read_dir(parent).map(|mut d| d.next().is_none()).unwrap_or(false) {
            let _ = fs::remove_dir(parent);
        }
    }
    Ok(())
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
        let t = Tmp(std::env::temp_dir().join(format!("pzms-kept-{}-{}", tag, std::process::id())));
        let _ = fs::remove_dir_all(&t.0);
        let src = t.0.join("workshop/108600/123/mods/SomeMod");
        fs::create_dir_all(src.join("42/media/lua/shared")).unwrap();
        fs::write(src.join("42/mod.info"), "name=Some Mod\nid=SomeMod\nmodversion=1.2\n").unwrap();
        fs::write(src.join("42/media/lua/shared/a.lua"), "return 1").unwrap();
        fs::create_dir_all(t.0.join("Zomboid/mods")).unwrap();
        let z = t.0.join("Zomboid").to_string_lossy().to_string();
        (t, src, z)
    }

    #[test]
    fn a_version_is_kept_listed_put_back_and_deleted() {
        let (t, src, z) = setup("roundtrip");
        let kept = keep_version(&z, "SomeMod", "Some Mod", &src, Some("1.2".into()), Some("123".into())).expect("kept");
        assert_eq!(kept.folder, "SomeMod");
        assert!(Path::new(&kept.path).join("42/media/lua/shared/a.lua").is_file());
        assert!(kept.bytes > 0);

        let listed = list_kept_versions(&z);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].version.as_deref(), Some("1.2"));

        // Steam "updates" the original; the kept copy is what it was.
        fs::write(src.join("42/media/lua/shared/a.lua"), "return 2").unwrap();
        let back = restore_kept_version(&z, &kept.path).expect("put back");
        let local = t.0.join("Zomboid/mods/SomeMod");
        assert_eq!(fs::read_to_string(local.join("42/media/lua/shared/a.lua")).unwrap(), "return 1");
        assert!(!local.join(MANIFEST).exists(), "the manifest does not travel into the mod");
        assert_eq!(back.mod_id, "SomeMod");
        let origin = origins::load(&t.0.join("Zomboid")).get("SomeMod").map(|r| r.origin.clone());
        assert!(matches!(origin, Some(ModOrigin::Compat { .. })), "{:?}", origin);
        assert!(crate::compat::local_first(&t.0.join("Zomboid")), "the Studio launches with Zomboid/mods first");

        // Putting it back twice moves the first copy aside instead of failing.
        restore_kept_version(&z, &kept.path).expect("put back again");

        delete_kept_version(&z, &kept.path).expect("deleted");
        assert!(list_kept_versions(&z).is_empty());
        assert!(!t.0.join("Zomboid").join(SNAPSHOTS_DIR).join("SomeMod").exists());
    }

    #[test]
    fn only_the_studios_own_kept_versions_are_touched() {
        let (t, _src, z) = setup("outside");
        let elsewhere = t.0.join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        assert!(restore_kept_version(&z, &elsewhere.to_string_lossy()).is_err());
        assert!(delete_kept_version(&z, &elsewhere.to_string_lossy()).is_err());
        assert!(elsewhere.is_dir());
    }
}
