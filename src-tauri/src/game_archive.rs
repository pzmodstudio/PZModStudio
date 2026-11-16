//! The game's own Lua and scripts of every build the Studio sees, kept in
//! `Zomboid/PZModStudio_GameArchive/<build>/game.zip` (about a fifth of the 140 MB they take).
//!
//! After an update the game's files are the new build's, and what they were is gone. Kept here,
//! the build before stays readable, so a later check can tell, for a mod's copy of a game file or
//! function, whether the game now does what the mod did (the mod is no longer needed) or whether
//! the mod now writes an old copy over newer code (worse than useless). Archiving is the first
//! step: nothing reads the archive yet but `read_file` and the listing.
//!
//! One archive per build, made once: a build is told by its version (`42.22.2`, from
//! `Zomboid/version.txt` when it agrees with the jar) and by the jar's size and date, so a hotfix
//! that keeps the version still gets its own (`42.22.2-<revision>`).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// The folders of the game kept: its Lua (every function a mod can replace) and its scripts
/// (items, recipes, vehicles a mod can redefine).
const KEPT: [&str; 2] = ["media/lua", "media/scripts"];
const ARCHIVE_DIR: &str = "PZModStudio_GameArchive";
const ZIP_NAME: &str = "game.zip";
const MANIFEST: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchivedBuild {
    /// The folder's name: the version, with the revision added when one version has two jars.
    pub build: String,
    /// The version the game says (`42.22.2`).
    pub version: String,
    pub revision: Option<String>,
    /// projectzomboid.jar's size and modification time (unix seconds): what tells one build from another.
    pub jar_len: u64,
    pub jar_modified: u64,
    /// When it was archived (unix seconds).
    pub archived_at: u64,
    pub files: u64,
    /// The zip's size on disk.
    pub bytes: u64,
}

/// The game as installed now, told apart from any other build.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledBuild {
    pub version: String,
    pub revision: Option<String>,
    pub jar_len: u64,
    pub jar_modified: u64,
}

fn unix(t: std::time::SystemTime) -> u64 {
    t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// `Zomboid/version.txt` as the game writes it at each start: `42.22.2 3d39148846 agents=1`.
/// Its version, and the revision (the second word, or `revision=` on a later line).
pub fn parse_version_txt(text: &str) -> Option<(String, Option<String>)> {
    let mut words = text.lines().next()?.split_whitespace();
    let version = words.next()?.to_string();
    if !version.chars().next()?.is_ascii_digit() {
        return None;
    }
    let revision = words
        .next()
        .filter(|w| !w.contains('='))
        .map(str::to_string)
        .or_else(|| text.lines().find_map(|l| l.trim().strip_prefix("revision=").map(|r| r.trim().to_string())));
    Some((version, revision.filter(|r| !r.is_empty())))
}

/// Which build is installed: the jar says major.minor (`42.22`); version.txt adds the patch and
/// the revision, but only the game rewrites it, at its next start, so it is trusted only while it
/// agrees with the jar.
pub fn installed_build(install_dir: &Path, zomboid_dir: &Path) -> Option<InstalledBuild> {
    let jar = install_dir.join("projectzomboid.jar");
    let meta = fs::metadata(&jar).ok()?;
    let jar_version = crate::load_order::mod_info::jar_game_version(&jar).map(crate::catalog::modinfo::format_game_version);
    let from_txt = fs::read_to_string(zomboid_dir.join("version.txt")).ok().and_then(|t| parse_version_txt(&t));
    let (version, revision) = match (jar_version, from_txt) {
        (Some(jar), Some((txt, rev))) if txt == jar || txt.starts_with(&format!("{}.", jar)) => (txt, rev),
        (Some(jar), _) => (jar, None),
        (None, Some((txt, rev))) => (txt, rev),
        (None, None) => return None,
    };
    Some(InstalledBuild { version, revision, jar_len: meta.len(), jar_modified: meta.modified().map(unix).unwrap_or(0) })
}

pub fn archive_root(zomboid_dir: &Path) -> PathBuf {
    zomboid_dir.join(ARCHIVE_DIR)
}

/// Every build kept, oldest first.
pub fn list(zomboid_dir: &Path) -> Vec<ArchivedBuild> {
    let Ok(entries) = fs::read_dir(archive_root(zomboid_dir)) else { return Vec::new() };
    let mut out: Vec<ArchivedBuild> = entries
        .flatten()
        .filter(|e| e.path().join(ZIP_NAME).is_file())
        .filter_map(|e| fs::read_to_string(e.path().join(MANIFEST)).ok())
        .filter_map(|t| serde_json::from_str(&t).ok())
        .collect();
    out.sort_by_key(|b: &ArchivedBuild| (b.jar_modified, b.archived_at));
    out
}

/// The build kept before `build` (the one an update replaced), if any.
pub fn previous(zomboid_dir: &Path, build: &str) -> Option<ArchivedBuild> {
    let all = list(zomboid_dir);
    let at = all.iter().position(|b| b.build == build)?;
    at.checked_sub(1).map(|i| all[i].clone())
}

/// One game file of a kept build (`media/lua/client/ISUI/ISButton.lua`), as text.
pub fn read_file(zomboid_dir: &Path, build: &str, rel_path: &str) -> Option<String> {
    let zip = archive_root(zomboid_dir).join(build).join(ZIP_NAME);
    let mut archive = zip::ZipArchive::new(io::BufReader::new(fs::File::open(zip).ok()?)).ok()?;
    let mut entry = archive.by_name(&rel_path.replace('\\', "/")).ok()?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Keeps the installed build's Lua and scripts, unless they already are. Returns the new archive,
/// or None when this build was kept before (or the game folder has nothing to keep).
pub fn archive_installed(install_dir: &Path, zomboid_dir: &Path) -> Result<Option<ArchivedBuild>, String> {
    let Some(now) = installed_build(install_dir, zomboid_dir) else { return Ok(None) };
    let kept = list(zomboid_dir);
    if kept.iter().any(|b| b.jar_len == now.jar_len && b.jar_modified == now.jar_modified) {
        return Ok(None);
    }
    // From version.txt, which anything can write: a folder name of plain characters only.
    let plain = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '-').collect::<String>();
    let now = InstalledBuild { version: plain(&now.version), revision: now.revision.as_deref().map(plain), ..now };
    if now.version.is_empty() || now.version.contains("..") {
        return Ok(None);
    }
    // The same version with another jar (a hotfix): its own folder, named by revision or date.
    let build = if kept.iter().any(|b| b.build == now.version) {
        format!("{}-{}", now.version, now.revision.clone().unwrap_or_else(|| now.jar_modified.to_string()))
    } else {
        now.version.clone()
    };
    let files = files_to_keep(install_dir);
    if files.is_empty() {
        return Ok(None);
    }
    let dir = archive_root(zomboid_dir).join(&build);
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {}", dir.display(), e))?;
    // Written aside, then renamed: a half-written zip is never taken for an archive.
    let partial = dir.join(format!("{}.partial", ZIP_NAME));
    write_zip(install_dir, &files, &partial)?;
    let zip = dir.join(ZIP_NAME);
    fs::rename(&partial, &zip).map_err(|e| format!("Cannot finish {}: {}", zip.display(), e))?;
    let made = ArchivedBuild {
        build,
        version: now.version,
        revision: now.revision,
        jar_len: now.jar_len,
        jar_modified: now.jar_modified,
        archived_at: unix(std::time::SystemTime::now()),
        files: files.len() as u64,
        bytes: fs::metadata(&zip).map(|m| m.len()).unwrap_or(0),
    };
    let json = serde_json::to_string_pretty(&made).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(dir.join(MANIFEST), json).map_err(|e| format!("Cannot write the archive's manifest: {}", e))?;
    Ok(Some(made))
}

/// The game's files under KEPT, as paths relative to the game folder with forward slashes.
fn files_to_keep(install_dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for top in KEPT {
        let root = install_dir.join(top);
        for entry in walkdir::WalkDir::new(&root).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            if let Ok(rel) = entry.path().strip_prefix(install_dir) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out.sort();
    out
}

fn write_zip(install_dir: &Path, files: &[String], dest: &Path) -> Result<(), String> {
    let file = fs::File::create(dest).map_err(|e| format!("Cannot create {}: {}", dest.display(), e))?;
    let mut zip = zip::ZipWriter::new(io::BufWriter::new(file));
    // Text compresses well; this runs in the background, so a better level than a save backup's.
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6));
    for rel in files {
        zip.start_file(rel.as_str(), options).map_err(|e| format!("Cannot write the archive: {}", e))?;
        let mut input = fs::File::open(install_dir.join(rel)).map_err(|e| format!("Cannot read {}: {}", rel, e))?;
        io::copy(&mut input, &mut zip).map_err(|e| format!("Cannot archive {}: {}", rel, e))?;
    }
    let writer = zip.finish().map_err(|e| format!("Cannot finish the archive: {}", e))?;
    let file = writer.into_inner().map_err(|e| format!("Cannot finish the archive: {}", e.error()))?;
    file.sync_all().map_err(|e| format!("Cannot finish the archive: {}", e))
}

/// What is kept, for the app and agents: the installed build, every archive, and the one made now.
#[derive(Debug, Clone, Serialize)]
pub struct ArchiveStatus {
    pub installed: Option<String>,
    pub folder: String,
    pub kept: Vec<ArchivedBuild>,
    /// Made by this call (archive_now, or a build not kept yet).
    pub made: Option<ArchivedBuild>,
}

/// The archive's state; with `archive_now` the installed build is kept first when it is new.
pub fn status(paths: &crate::vfs::StudioPaths, archive_now: bool) -> Result<ArchiveStatus, String> {
    let zomboid = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or("Zomboid folder not found")?;
    let install = Path::new(&paths.pz_install_dir);
    let made = if archive_now { archive_installed(install, &zomboid)? } else { None };
    Ok(ArchiveStatus {
        installed: installed_build(install, &zomboid).map(|b| b.version),
        folder: archive_root(&zomboid).to_string_lossy().into_owned(),
        kept: list(&zomboid),
        made,
    })
}

/// At the Studio's start, in the background: the installed build is kept if it is new.
pub fn archive_in_background() {
    std::thread::spawn(|| {
        let paths = crate::vfs::studio_paths();
        if paths.pz_install_dir.is_empty() || paths.user_zomboid_dir.is_empty() {
            return;
        }
        let Some(zomboid) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next() else { return };
        let started = std::time::Instant::now();
        match archive_installed(Path::new(&paths.pz_install_dir), &zomboid) {
            Ok(Some(b)) => crate::diagnostics::log(
                "INFO",
                &format!("Kept the game's code of build {} ({} files, {} KB) in {:.1}s", b.build, b.files, b.bytes / 1024, started.elapsed().as_secs_f32()),
            ),
            Ok(None) => {}
            Err(e) => crate::diagnostics::log("WARN", &format!("Could not keep the game's code of this build: {}", e)),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(dir: &Path, jar: &[u8], lua: &str) {
        fs::create_dir_all(dir.join("media/lua/client")).unwrap();
        fs::create_dir_all(dir.join("media/scripts")).unwrap();
        fs::create_dir_all(dir.join("media/textures")).unwrap();
        fs::write(dir.join("projectzomboid.jar"), jar).unwrap();
        fs::write(dir.join("media/lua/client/ISButton.lua"), lua).unwrap();
        fs::write(dir.join("media/scripts/items.txt"), "module Base { }").unwrap();
        fs::write(dir.join("media/textures/big.png"), [0u8; 64]).unwrap();
    }

    #[test]
    fn version_txt_is_read_as_the_game_writes_it() {
        assert_eq!(parse_version_txt("42.22.2 3d39148846 agents=1\nrevision=3d39148846 pzbullet=1.0.0.28\n"), Some(("42.22.2".into(), Some("3d39148846".into()))));
        assert_eq!(parse_version_txt("42.21 agents=1\nrevision=abc\n"), Some(("42.21".into(), Some("abc".into()))));
        assert_eq!(parse_version_txt("garbage"), None);
    }

    #[test]
    fn each_build_is_kept_once_and_the_one_before_stays_readable() {
        let tmp = std::env::temp_dir().join(format!("pzms-game-archive-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let (install, zomboid) = (tmp.join("game"), tmp.join("Zomboid"));
        fs::create_dir_all(&zomboid).unwrap();
        // No jar version in a test jar: version.txt names the build.
        fs::write(zomboid.join("version.txt"), "42.21.1 aaa agents=1\n").unwrap();
        game(&install, b"jar one", "function ISButton:new() return 1 end");

        let first = archive_installed(&install, &zomboid).unwrap().expect("first build kept");
        assert_eq!(first.build, "42.21.1");
        assert_eq!(first.files, 2, "Lua and scripts only, not textures");
        assert!(archive_installed(&install, &zomboid).unwrap().is_none(), "the same build is not kept twice");

        // The update: another jar, other code.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(zomboid.join("version.txt"), "42.22.2 bbb agents=1\n").unwrap();
        game(&install, b"jar two, longer", "function ISButton:new() return 2 end");
        let second = archive_installed(&install, &zomboid).unwrap().expect("new build kept");
        assert_eq!(second.build, "42.22.2");

        assert_eq!(list(&zomboid).iter().map(|b| b.build.as_str()).collect::<Vec<_>>(), ["42.21.1", "42.22.2"]);
        assert_eq!(previous(&zomboid, "42.22.2").map(|b| b.build), Some("42.21.1".into()));
        assert_eq!(read_file(&zomboid, "42.21.1", "media/lua/client/ISButton.lua").as_deref(), Some("function ISButton:new() return 1 end"));
        assert_eq!(read_file(&zomboid, "42.22.2", "media\\lua\\client\\ISButton.lua").as_deref(), Some("function ISButton:new() return 2 end"));

        // A hotfix that keeps the version: its own folder, by revision.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(zomboid.join("version.txt"), "42.22.2 ccc agents=1\n").unwrap();
        game(&install, b"jar three, hotfix!", "function ISButton:new() return 3 end");
        assert_eq!(archive_installed(&install, &zomboid).unwrap().map(|b| b.build), Some("42.22.2-ccc".into()));
        assert!(!archive_root(&zomboid).join("42.22.2").join("game.zip.partial").exists());
        let _ = fs::remove_dir_all(&tmp);
    }
}
