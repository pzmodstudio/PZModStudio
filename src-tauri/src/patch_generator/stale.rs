//! The active package's copies of mod files, beside the mod files they stand in for.
//!
//! A package loads last, so its copy of a mod file is the one the game reads. The copy is made
//! once; when the mod later changes its own file (a Workshop update, a switch to a rewritten
//! fork) the copy goes on running the old code, and may call things the new version no longer
//! has. That is reported here, file by file.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::catalog::{CatalogMod, ModCatalog};

/// One file of the active Studio package: the mod whose file it replaces and whether that mod
/// changed the file after the copy was made.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFileState {
    /// `media/...`, forward slashes.
    pub relative_path: String,
    /// The package's mod id (`Z_PZModStudio_MergedPatch`).
    pub package_id: String,
    /// The package's folder name under `Zomboid/mods`, as `remove_file_from_package` wants it.
    pub package_folder: String,
    /// The package's short name, as the game names it ("MergedPatch").
    pub package: String,
    /// When the copy was made (the copy's file date), `YYYY-MM-DD`.
    pub copy_date: String,
    /// The active mod whose file the copy stands in for (the last loaded, when several ship it).
    pub mod_id: Option<String>,
    pub mod_name: Option<String>,
    /// When the mod's own file last changed, `YYYY-MM-DD`.
    pub mod_date: Option<String>,
    /// The mod changed its own file after the copy was made: the game reads the copy, so that
    /// change never runs.
    pub stale: bool,
    /// No mod of the active list ships the file: the copy runs on its own.
    pub orphan: bool,
    /// Two or more active mods ship it: a merge, reported with the file conflicts, not here.
    pub conflict: bool,
    /// One mod ships it and the copy is not that mod's file (line endings aside): a merge made
    /// when another mod shipped it too and is off now, or a patch of the player's.
    pub differs: bool,
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn date(t: Option<SystemTime>) -> String {
    t.map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d").to_string()).unwrap_or_default()
}

/// Files the Studio writes itself: the shim, merged script definitions, merged translations.
/// They stand in for no single mod file.
pub fn studio_made(relative_path: &str) -> bool {
    let name = relative_path.rsplit('/').next().unwrap_or(relative_path);
    // Directly under `media/` the package keeps only its own mod.info and pictures.
    let at_media_root = relative_path.matches('/').count() < 2;
    at_media_root || name.starts_with("Z_PZModStudio_") || name.starts_with("PZModStudio_") || relative_path.to_lowercase().contains("/translate/")
}

/// Where the package keeps its files: the build folder when it has one, else the root.
fn package_media_root(root: &Path) -> PathBuf {
    let build = root.join("42");
    if build.join("media").is_dir() { build } else { root.to_path_buf() }
}

/// Every `media/...` file under `media_root`, forward slashes, in path order.
fn package_files(media_root: &Path) -> Vec<String> {
    let media = media_root.join("media");
    let mut out: Vec<String> = walkdir::WalkDir::new(&media)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.path().strip_prefix(media_root).ok().map(|p| p.to_string_lossy().replace('\\', "/")))
        .collect();
    out.sort();
    out
}

/// The mod's own copy of `relative_path`: the newest among its version folders, its `common`
/// folder and its root (the game reads one of those for the installed build).
fn mod_file(m: &CatalogMod, relative_path: &str) -> Option<PathBuf> {
    let root = Path::new(&m.root_dir);
    let mut candidates: Vec<PathBuf> = m.version_folders.iter().map(|v| root.join(v).join(relative_path)).collect();
    if m.has_common {
        candidates.push(root.join("common").join(relative_path));
    }
    candidates.push(root.join(relative_path));
    candidates.into_iter().filter(|p| p.is_file()).max_by_key(|p| modified(p))
}

/// The two files say the same thing (line endings and a trailing newline aside).
fn same_text(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| std::fs::read(p).ok().map(|bytes| String::from_utf8_lossy(&bytes).replace("\r\n", "\n").trim_end().to_string());
    match (norm(a), norm(b)) {
        (Some(x), Some(y)) => x == y,
        _ => true,
    }
}

/// The active package's files that stand in for a mod's file, each beside the mod's own copy.
/// Empty without an active, packaged Studio package.
pub fn package_file_states(catalog: &ModCatalog, active: &[String]) -> Vec<PackageFileState> {
    let Some((package_id, root)) = crate::vfs::active_package_dir(catalog, active) else {
        return Vec::new();
    };
    let package_folder = root.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| package_id.clone());
    let package = package_id.strip_prefix("Z_PZModStudio_").unwrap_or(&package_id).to_string();
    let media_root = package_media_root(&root);
    let active_mods: Vec<&CatalogMod> = active
        .iter()
        .filter_map(|e| catalog.resolve(e.trim()))
        .filter(|m| !m.id.starts_with("Z_PZModStudio_"))
        .collect();
    package_files(&media_root)
        .into_iter()
        .filter(|rel| !studio_made(rel))
        .map(|rel| {
            let copy_at = modified(&media_root.join(&rel));
            let providers: Vec<(&CatalogMod, PathBuf)> = active_mods.iter().filter_map(|m| mod_file(m, &rel).map(|p| (*m, p))).collect();
            let last = providers.last();
            let mod_at = last.and_then(|(_, p)| modified(p));
            let differs = providers.len() == 1 && !same_text(&media_root.join(&rel), &providers[0].1);
            PackageFileState {
                relative_path: rel,
                package_id: package_id.clone(),
                package_folder: package_folder.clone(),
                package: package.clone(),
                copy_date: date(copy_at),
                mod_id: last.map(|(m, _)| m.id.clone()),
                mod_name: last.map(|(m, _)| m.name.clone()),
                mod_date: last.map(|_| date(mod_at)),
                stale: matches!((mod_at, copy_at), (Some(a), Some(b)) if a > b),
                orphan: providers.is_empty(),
                conflict: providers.len() > 1,
                differs,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_studios_own_files_are_not_copies_of_a_mods() {
        assert!(studio_made("media/lua/shared/Z_PZModStudio_Polyfills.lua"));
        assert!(studio_made("media/scripts/PZModStudio_item_Base.OldTire1.txt"));
        assert!(studio_made("media/lua/shared/Translate/EN/UI.json"));
        assert!(studio_made("media/mod.info"));
        assert!(!studio_made("media/lua/client/TOC/Zombies/ZombiesAmputation.lua"));
    }

    #[test]
    fn the_build_folder_is_read_when_the_package_has_one() {
        let dir = std::env::temp_dir().join(format!("pzms_stale_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("with").join("42").join("media")).unwrap();
        std::fs::create_dir_all(dir.join("with").join("media")).unwrap();
        std::fs::create_dir_all(dir.join("plain").join("media")).unwrap();
        assert_eq!(package_media_root(&dir.join("with")), dir.join("with").join("42"));
        assert_eq!(package_media_root(&dir.join("plain")), dir.join("plain"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
