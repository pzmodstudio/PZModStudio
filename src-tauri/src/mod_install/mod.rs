//! Installs mods downloaded by hand (Nexus, forums, GitHub, Discord...) into `Zomboid/mods`.
//!
//! Accepts a `.zip` archive or an already extracted folder, finds every mod folder inside
//! (whatever the nesting: loose files, `mods/X`, `Contents/mods/X`, B41 or B42 layout) and copies
//! each one to `Zomboid/mods/<folder>`. Archives are unpacked into a temporary folder first, with
//! entries that would escape it refused ("zip slip") and size/file-count limits. Nothing is ever
//! overwritten silently: replacing needs `replace_existing`, and the old copy is backed up first.

use crate::catalog::modinfo::parse_game_mod_info;
use crate::load_order::mod_info::{get_existing_user_zomboid_dirs, select_mod_info_for_build, version_dir_int};
pub mod from_url;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MAX_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024; // maps can be large
const MAX_FILES: usize = 200_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMod {
    pub folder: String,
    pub mod_id: Option<String>,
    pub name: Option<String>,
    pub destination: String,
    /// A previous copy was replaced (and backed up to `backup`).
    pub replaced: bool,
    pub backup: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub installed: Vec<InstalledMod>,
    /// Things the user should know (e.g. a subscribed Workshop copy with the same id wins).
    pub warnings: Vec<String>,
}

/// Returned as the error text (JSON) when folders already exist and `replace_existing` is false,
/// so the UI can ask before replacing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallConflict {
    pub existing_folders: Vec<String>,
}

pub const CONFLICT_PREFIX: &str = "INSTALL_CONFLICT:";

/// A folder is a mod folder if it holds a mod.info directly (B41 layout), in `common/`, or in a
/// version folder (`42`, `42.13`, ...).
fn is_mod_root(dir: &Path) -> bool {
    if dir.join("mod.info").is_file() || dir.join("common").join("mod.info").is_file() {
        return true;
    }
    fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                version_dir_int(&name).is_some() && e.path().join("mod.info").is_file()
            })
        })
        .unwrap_or(false)
}

/// Outermost mod folders under `root` (a mod's own version folders are not reported again).
fn find_mod_roots(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if is_mod_root(&dir) {
            found.push(dir);
            continue;
        }
        if depth >= 8 {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten() {
                let Ok(meta) = fs::symlink_metadata(e.path()) else { continue };
                if meta.is_dir() {
                    stack.push((e.path(), depth + 1));
                }
            }
        }
    }
    found.sort();
    found
}

fn safe_folder_name(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    (!cleaned.is_empty() && cleaned != "." && cleaned != "..").then_some(cleaned)
}

fn unique_temp_dir(tag: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("pzms-{}-{}-{}", tag, std::process::id(), n))
}

/// Unpacks a zip into `dest`, refusing entries outside it and enforcing size limits.
fn extract_zip(archive_path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|e| format!("Cannot open {}: {}", archive_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Not a readable zip archive: {}", e))?;
    if archive.len() > MAX_FILES {
        return Err(format!("The archive has {} entries (limit {}).", archive.len(), MAX_FILES));
    }
    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("Damaged archive entry: {}", e))?;
        let Some(relative) = entry.enclosed_name() else {
            return Err(format!("Refusing unsafe path in archive: {}", entry.name()));
        };
        let out_path = dest.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }
        total = total.saturating_add(entry.size());
        if total > MAX_TOTAL_BYTES {
            return Err("The archive unpacks to more than 4 GB; refusing.".to_string());
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = fs::File::create(&out_path).map_err(|e| format!("Cannot write {}: {}", out_path.display(), e))?;
        // Never trust the declared size alone: cap what is actually written.
        let declared = entry.size();
        let written = io::copy(&mut io::Read::take(&mut entry, declared + 1), &mut out).map_err(|e| e.to_string())?;
        if written > declared {
            return Err(format!("Archive entry {} is larger than it claims; refusing.", entry.name()));
        }
    }
    Ok(())
}

pub(crate) fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let meta = fs::symlink_metadata(entry.path())?;
        let target = dst.join(entry.file_name());
        if meta.file_type().is_symlink() {
            continue; // never follow links out of the mod
        } else if meta.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

struct Candidate {
    root: PathBuf,
    folder: String,
    mod_id: Option<String>,
    name: Option<String>,
}

fn describe_candidates(staging: &Path, fallback_name: &str) -> Result<Vec<Candidate>, String> {
    let roots = find_mod_roots(staging);
    if roots.is_empty() {
        return Err("No Project Zomboid mod found: there is no mod.info (directly, in common/ or in a 42/ folder).".to_string());
    }
    let mut candidates = Vec::new();
    for root in roots {
        let info_path = select_mod_info_for_build(&root, None).or_else(|| {
            let r = root.join("mod.info");
            r.is_file().then_some(r)
        });
        let info = info_path
            .and_then(|p| fs::read(p).ok())
            .map(|bytes| parse_game_mod_info(&String::from_utf8_lossy(&bytes)));
        let mod_id = info.as_ref().and_then(|i| i.id.clone());
        let name = info.as_ref().and_then(|i| i.name.clone()).map(|n| n.trim().to_string());
        // Loose files at the top of the archive have no folder name: use the id or the file name.
        let raw_folder = if root == staging {
            mod_id.clone().unwrap_or_else(|| fallback_name.to_string())
        } else {
            root.file_name().unwrap_or_default().to_string_lossy().to_string()
        };
        let folder = safe_folder_name(&raw_folder).ok_or_else(|| format!("Unusable mod folder name: {}", raw_folder))?;
        // The Studio's own packages are rewritten and cleaned by the Studio from their metadata;
        // a downloaded mod must not take their place.
        if folder.to_ascii_lowercase().starts_with("z_pzmodstudio_") {
            return Err(format!("{} is a name the Studio keeps for its own packages; it is not installed.", folder));
        }
        candidates.push(Candidate { root, folder, mod_id, name });
    }
    Ok(candidates)
}

/// Unpacks a 7z into `dest` with the same rules as a zip: no entry outside it, no more than
/// MAX_FILES entries, no more than MAX_TOTAL_BYTES unpacked, nothing larger than it claims.
fn extract_7z(archive_path: &Path, dest: &Path) -> Result<(), String> {
    let mut count: usize = 0;
    let mut total: u64 = 0;
    let other = |msg: String| sevenz_rust::Error::Other(msg.into());
    sevenz_rust::decompress_file_with_extract_fn(archive_path, dest, |entry, reader, _suggested| {
        count += 1;
        if count > MAX_FILES {
            return Err(other(format!("The archive has more than {} entries.", MAX_FILES)));
        }
        let name = entry.name().replace('\\', "/");
        let relative = Path::new(&name);
        let unsafe_path = relative.is_absolute()
            || relative.components().any(|c| !matches!(c, std::path::Component::Normal(_)));
        if unsafe_path {
            return Err(other(format!("Refusing unsafe path in archive: {}", entry.name())));
        }
        let out_path = dest.join(relative);
        if entry.is_directory() {
            fs::create_dir_all(&out_path).map_err(|e| other(e.to_string()))?;
            return Ok(true);
        }
        total = total.saturating_add(entry.size());
        if total > MAX_TOTAL_BYTES {
            return Err(other("The archive unpacks to more than 4 GB; refusing.".to_string()));
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| other(e.to_string()))?;
        }
        let mut out = fs::File::create(&out_path).map_err(|e| other(format!("Cannot write {}: {}", out_path.display(), e)))?;
        let declared = entry.size();
        let written = io::copy(&mut io::Read::take(reader, declared + 1), &mut out).map_err(|e| other(e.to_string()))?;
        if written > declared {
            return Err(other(format!("Archive entry {} is larger than it claims; refusing.", entry.name())));
        }
        Ok(true)
    })
    .map_err(|e| match e {
        sevenz_rust::Error::Other(msg) => msg.to_string(),
        sevenz_rust::Error::BadSignature(_) => "Not a readable 7z archive.".to_string(),
        e => format!("Not a readable 7z archive: {}", e),
    })
}

/// Installs the mod(s) found in `source` (a .zip, .7z or a folder) into `<Zomboid>/mods`.
/// `origin` says where the file came from (e.g. the Nexus page the user downloaded it from);
/// without it the install is recorded as a manual one with the file name.
pub fn install_mods(
    source: &str,
    user_zomboid_dir: &str,
    workshop_dir: &str,
    replace_existing: bool,
    origin: Option<crate::origins::ModOrigin>,
) -> Result<InstallResult, String> {
    let source_path = Path::new(source);
    let zomboid = get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;
    let mods_dir = zomboid.join("mods");

    let fallback_name = source_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Mod".into());
    let mut staging_to_clean: Option<PathBuf> = None;
    let staging = if source_path.is_dir() {
        source_path.to_path_buf()
    } else {
        let ext = source_path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let extract: fn(&Path, &Path) -> Result<(), String> = match ext.as_str() {
            "zip" => extract_zip,
            "7z" => extract_7z,
            _ => {
                return Err(format!(
                    "\".{}\" archives are not supported{}. Extract it first (for example with 7-Zip) and choose the extracted folder.",
                    ext,
                    if ext == "rar" { " (RAR is a proprietary format)" } else { "" }
                ))
            }
        };
        let dir = unique_temp_dir("install");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        staging_to_clean = Some(dir.clone());
        if let Err(e) = extract(source_path, &dir) {
            let _ = fs::remove_dir_all(&dir);
            return Err(e);
        }
        dir
    };

    let result = (|| {
        let candidates = describe_candidates(&staging, &fallback_name)?;
        let existing: Vec<String> = candidates.iter().filter(|c| mods_dir.join(&c.folder).exists()).map(|c| c.folder.clone()).collect();
        if !existing.is_empty() && !replace_existing {
            let conflict = serde_json::to_string(&InstallConflict { existing_folders: existing }).unwrap_or_default();
            return Err(format!("{}{}", CONFLICT_PREFIX, conflict));
        }

        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let mut installed = Vec::new();
        let mut warnings = Vec::new();
        for c in candidates {
            let dest = mods_dir.join(&c.folder);
            if staging_to_clean.is_none() && dest.starts_with(&c.root) {
                return Err("The source folder is inside Zomboid/mods already.".to_string());
            }
            let mut backup = None;
            let replaced = dest.exists();
            if replaced {
                let backup_path = zomboid.join("PZModStudio_Backups").join("mods").join(format!("{}-{}", c.folder, stamp));
                fs::create_dir_all(backup_path.parent().unwrap()).map_err(|e| e.to_string())?;
                fs::rename(&dest, &backup_path).map_err(|e| format!("Cannot move the old {} aside: {}", c.folder, e))?;
                backup = Some(backup_path.to_string_lossy().to_string());
            }
            if let Err(e) = copy_tree(&c.root, &dest) {
                let _ = fs::remove_dir_all(&dest);
                if let Some(b) = &backup {
                    let _ = fs::rename(b, &dest); // put the old copy back
                }
                return Err(format!("Copying {} failed: {}", c.folder, e));
            }
            installed.push(InstalledMod {
                folder: c.folder,
                mod_id: c.mod_id,
                name: c.name,
                destination: dest.to_string_lossy().to_string(),
                replaced,
                backup,
            });
        }

        let origin = origin.unwrap_or_else(|| crate::origins::ModOrigin::Manual {
            file_name: source_path.file_name().map(|n| n.to_string_lossy().to_string()),
        });
        let folders: Vec<String> = installed.iter().map(|m| m.folder.clone()).collect();
        if let Err(e) = crate::origins::record(&zomboid, &folders, &origin) {
            warnings.push(format!("Installed, but where it came from could not be saved: {}", e));
        }

        // What the game will make of it, using the same catalog as the health check.
        let catalog = crate::catalog::build_catalog(workshop_dir, &zomboid.to_string_lossy(), None);
        for m in &installed {
            let Some(id) = &m.mod_id else {
                warnings.push(format!("{}: its mod.info has no usable id= line, so the game cannot load it.", m.folder));
                continue;
            };
            if let Some(winner) = catalog.get(id) {
                if winner.root_dir != m.destination {
                    warnings.push(format!(
                        "{} is also installed at {}, which the game finds first, so the copy you just installed is ignored. Unsubscribe or remove that copy to use this one.",
                        id, winner.root_dir
                    ));
                } else if winner.loadability != crate::catalog::Loadability::Loadable {
                    warnings.push(format!(
                        "{} cannot load on your game build ({:?}). It is installed but the game will skip it.",
                        id, winner.loadability
                    ));
                }
            }
        }
        Ok(InstallResult { installed, warnings })
    })();

    if let Some(dir) = staging_to_clean {
        let _ = fs::remove_dir_all(dir);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn zomboid(tag: &str) -> Tmp {
        let t = Tmp(unique_temp_dir(tag));
        fs::create_dir_all(t.0.join("Zomboid/mods")).unwrap();
        t
    }

    fn make_zip(path: &Path, files: &[(&str, &str)]) {
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for (name, content) in files {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn installs_nested_b42_mod_from_zip_and_refuses_silent_overwrite() {
        let t = zomboid("zip");
        let z = t.0.join("Zomboid").to_string_lossy().to_string();
        let archive = t.0.join("download.zip");
        make_zip(&archive, &[
            ("SomeMod-v2/Contents/mods/SomeMod/42/mod.info", "name=Some Mod\nid=SomeMod\n"),
            ("SomeMod-v2/Contents/mods/SomeMod/42/media/lua/client/a.lua", "print(1)"),
            ("SomeMod-v2/Contents/mods/SomeMod/common/.keep", ""),
            ("SomeMod-v2/readme.txt", "hi"),
        ]);
        let r = install_mods(&archive.to_string_lossy(), &z, "", false, None).unwrap();
        assert_eq!(r.installed.len(), 1);
        assert_eq!(r.installed[0].folder, "SomeMod");
        assert_eq!(r.installed[0].mod_id.as_deref(), Some("SomeMod"));
        assert!(t.0.join("Zomboid/mods/SomeMod/42/media/lua/client/a.lua").is_file());
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);

        // Second time: conflict unless replace is confirmed; with replace the old copy is backed up.
        let err = install_mods(&archive.to_string_lossy(), &z, "", false, None).unwrap_err();
        assert!(err.starts_with(CONFLICT_PREFIX), "{}", err);
        let again = install_mods(&archive.to_string_lossy(), &z, "", true, None).unwrap();
        assert!(again.installed[0].replaced);
        assert!(Path::new(again.installed[0].backup.as_ref().unwrap()).join("42/mod.info").is_file());
    }

    #[test]
    fn loose_files_use_the_mod_id_and_b41_mods_get_a_warning() {
        let t = zomboid("loose");
        let z = t.0.join("Zomboid").to_string_lossy().to_string();
        let archive = t.0.join("old.zip");
        make_zip(&archive, &[("mod.info", "id=OldOne\nname=Old\n"), ("media/lua/client/x.lua", "")]);
        fs::write(t.0.join("Zomboid/version.txt"), "42.21.0 abc").unwrap();
        let r = install_mods(&archive.to_string_lossy(), &z, "", false, None).unwrap();
        assert_eq!(r.installed[0].folder, "OldOne");
        assert!(r.warnings.iter().any(|w| w.contains("cannot load")), "{:?}", r.warnings);
    }

    #[test]
    fn records_where_the_mod_came_from() {
        let t = zomboid("origin");
        let z = t.0.join("Zomboid").to_string_lossy().to_string();
        let archive = t.0.join("grmt.zip");
        make_zip(&archive, &[("GRMT/42/mod.info", "id=GRMT\n")]);
        let nexus = crate::origins::ModOrigin::Nexus { mod_id: 445, page_url: "https://www.nexusmods.com/projectzomboid/mods/445".into(), name: None };
        install_mods(&archive.to_string_lossy(), &z, "", false, Some(nexus.clone())).unwrap();
        let catalog = crate::catalog::build_catalog("", &z, Some(42_021));
        assert_eq!(catalog.get("GRMT").unwrap().origin, Some(nexus));
    }

    #[test]
    fn refuses_zip_slip_and_unsupported_archives() {
        let t = zomboid("slip");
        let z = t.0.join("Zomboid").to_string_lossy().to_string();
        let evil = t.0.join("evil.zip");
        make_zip(&evil, &[("../../outside.txt", "x"), ("M/mod.info", "id=M\n")]);
        let err = install_mods(&evil.to_string_lossy(), &z, "", false, None).unwrap_err();
        assert!(err.contains("unsafe path"), "{}", err);
        assert!(!t.0.join("outside.txt").exists());

        fs::write(t.0.join("mod.rar"), b"Rar!").unwrap();
        let err = install_mods(&t.0.join("mod.rar").to_string_lossy(), &z, "", false, None).unwrap_err();
        assert!(err.contains("not supported"), "{}", err);

        fs::write(t.0.join("mod.7z"), b"not a 7z at all").unwrap();
        let err = install_mods(&t.0.join("mod.7z").to_string_lossy(), &z, "", false, None).unwrap_err();
        assert!(err.contains("7z"), "{}", err);
    }

    /// A 7z gets the same treatment as a zip: the mod inside is found and installed.
    #[test]
    fn installs_from_a_7z_archive() {
        let t = zomboid("sevenz");
        let z = t.0.join("Zomboid").to_string_lossy().to_string();
        let src = t.0.join("src/SevenMod");
        fs::create_dir_all(src.join("42/media/lua/shared")).unwrap();
        fs::write(src.join("42/mod.info"), "name=Seven Mod
id=SevenMod
").unwrap();
        fs::write(src.join("42/media/lua/shared/s.lua"), "return 7").unwrap();
        let archive = t.0.join("seven.7z");
        sevenz_rust::compress_to_path(t.0.join("src"), &archive).expect("a 7z is written");

        let result = install_mods(&archive.to_string_lossy(), &z, "", false, None).expect("installed");
        assert_eq!(result.installed.len(), 1, "{:?}", result);
        assert!(t.0.join("Zomboid/mods/SevenMod/42/media/lua/shared/s.lua").is_file());
    }

    #[test]
    fn installs_from_an_extracted_folder_with_several_mods() {
        let t = zomboid("folder");
        let z = t.0.join("Zomboid").to_string_lossy().to_string();
        let src = t.0.join("extracted");
        for (m, layout) in [("A", "42"), ("B", "common")] {
            fs::create_dir_all(src.join("mods").join(m).join(layout)).unwrap();
            fs::write(src.join("mods").join(m).join(layout).join("mod.info"), format!("id={}\n", m)).unwrap();
        }
        let r = install_mods(&src.to_string_lossy(), &z, "", false, None).unwrap();
        let folders: Vec<_> = r.installed.iter().map(|m| m.folder.as_str()).collect();
        assert_eq!(folders, vec!["A", "B"]);
        assert!(src.join("mods/A/42/mod.info").is_file(), "the source folder is left as it was");
    }
}
