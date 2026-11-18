//! Where the version store lives, and moving it.
//!
//! The store is `Zomboid/PZModStudio_Versions` unless the player chose another folder (another
//! drive, usually: it can take gigabytes). The choice is kept outside the store, in
//! `Zomboid/PZModStudio_VersionsFolder.json`, so the store can be found wherever it is. The
//! store's own settings (switch, cap) live inside it and travel with it.
//!
//! Moving copies every file to the new folder, reads each copy back against the original's hash,
//! and only then points the Studio at the new folder and removes the old one; on one drive the
//! folder is renamed instead. Local copies put back from a kept version are pointed at the new
//! folder too. While it moves, `Zomboid/PZModStudio_VersionsMoving.lock` stops every round.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use super::store::{self, Store, STORE_DIR};
use crate::origins::{self, ModOrigin};

const PLACE_FILE: &str = "PZModStudio_VersionsFolder.json";
const MOVING_FILE: &str = "PZModStudio_VersionsMoving.lock";
pub const MOVING: &str = "The kept versions are being moved to another folder; try again in a minute.";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Place {
    dir: String,
}

/// The folder the player chose for the store, when it is not the default.
pub fn chosen_root(zomboid: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(zomboid.join(PLACE_FILE)).ok()?;
    let place: Place = serde_json::from_str(&text).ok()?;
    let dir = place.dir.trim();
    (!dir.is_empty()).then(|| PathBuf::from(dir))
}

pub fn default_root(zomboid: &Path) -> PathBuf {
    zomboid.join(STORE_DIR)
}

fn write_place(zomboid: &Path, root: &Path) -> Result<(), String> {
    let file = zomboid.join(PLACE_FILE);
    if same_path(root, &default_root(zomboid)) {
        return match fs::remove_file(&file) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(format!("Cannot write {}: {}", file.display(), e)),
            _ => Ok(()),
        };
    }
    let text = serde_json::to_string_pretty(&Place { dir: root.to_string_lossy().to_string() }).map_err(|e| e.to_string())?;
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| format!("Cannot write {}: {}", tmp.display(), e))?;
    fs::rename(&tmp, &file).map_err(|e| format!("Cannot write {}: {}", file.display(), e))
}

/// A move is under way (a marker younger than two hours: an older one is from a move cut short).
pub fn moving(zomboid: &Path) -> bool {
    fs::metadata(zomboid.join(MOVING_FILE))
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age.as_secs() < 2 * 3600)
}

struct MovingMark(PathBuf);
impl Drop for MovingMark {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// `path` with every part that exists resolved (links, `..`, case on Windows), without the `\\?\`
/// prefix; the parts that do not exist yet are added as given.
fn normalized(path: &Path) -> PathBuf {
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    let mut probe = path.to_path_buf();
    loop {
        if let Ok(real) = probe.canonicalize() {
            let mut out = PathBuf::from(strip_verbatim(&real.to_string_lossy()));
            for part in rest.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match (probe.file_name().map(|n| n.to_os_string()), probe.parent().map(Path::to_path_buf)) {
            (Some(name), Some(parent)) if !parent.as_os_str().is_empty() => {
                rest.push(name);
                probe = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

fn strip_verbatim(p: &str) -> String {
    if let Some(unc) = p.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", unc)
    } else {
        p.strip_prefix(r"\\?\").unwrap_or(p).to_string()
    }
}

/// A comparable form: one separator, no trailing one, lower case on Windows.
fn key(path: &Path) -> String {
    let s = normalized(path).to_string_lossy().replace('/', "\\");
    let s = s.trim_end_matches('\\').to_string();
    if cfg!(windows) { s.to_lowercase() } else { s }
}

pub(crate) fn same_path(a: &Path, b: &Path) -> bool {
    key(a) == key(b)
}

/// `path` is `dir` or inside it.
fn inside(path: &Path, dir: &Path) -> bool {
    let (p, d) = (key(path), key(dir));
    !d.is_empty() && (p == d || p.starts_with(&format!("{}\\", d)))
}

fn is_drive_root(path: &Path) -> bool {
    normalized(path).parent().is_none()
}

fn is_empty_dir(path: &Path) -> bool {
    fs::read_dir(path).map(|mut d| d.next().is_none()).unwrap_or(false)
}

/// The folders the store may not go into, each with its name for the refusal.
pub fn forbidden_dirs(paths: &crate::vfs::StudioPaths, zomboid: &Path) -> Vec<(PathBuf, &'static str)> {
    let mut out = Vec::new();
    if !paths.workshop_dir.is_empty() {
        let ws = PathBuf::from(&paths.workshop_dir);
        // steamapps/workshop: Steam's own, where it downloads and stages items.
        out.push((ws.parent().and_then(Path::parent).map(Path::to_path_buf).unwrap_or_else(|| ws.clone()), "Steam's Workshop folder"));
        out.push((ws, "Steam's Workshop folder"));
    }
    if !paths.pz_install_dir.is_empty() {
        out.push((PathBuf::from(&paths.pz_install_dir), "the game's folder"));
    }
    out.push((zomboid.join("mods"), "Zomboid/mods"));
    out.push((zomboid.join("Saves"), "Zomboid/Saves"));
    out
}

/// Where a chosen folder puts the store: the folder itself when it is empty or does not exist; a
/// `PZModStudio_Versions` folder inside it when it holds other things or is a drive's root.
pub fn target_for(chosen: &Path) -> PathBuf {
    if is_drive_root(chosen) || (chosen.exists() && !is_empty_dir(chosen)) {
        chosen.join(STORE_DIR)
    } else {
        chosen.to_path_buf()
    }
}

/// What a move did.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoveOutcome {
    pub from: String,
    pub to: String,
    /// Already there: nothing was done.
    pub unchanged: bool,
    /// Files copied (0 when the folder was renamed on its drive or the store was empty).
    pub files: usize,
    pub bytes: u64,
    pub renamed: bool,
    /// Local copies in use whose record now names the new folder.
    pub copies_repointed: usize,
    /// The old folder could not be removed completely (the store works from the new one).
    pub old_left: Option<String>,
}

/// Checks a target folder for the store.
fn check_target(target: &Path, current: &Path, forbidden: &[(PathBuf, &'static str)]) -> Result<(), String> {
    if !target.is_absolute() {
        return Err(format!("{} is not a full path (it needs its drive, like D:\\Kept).", target.display()));
    }
    if is_drive_root(target) {
        return Err(format!("The root of a drive ({}) cannot hold the store: choose a folder.", target.display()));
    }
    for (dir, name) in forbidden {
        if inside(target, dir) {
            return Err(format!("{} is inside {} ({}); the kept versions go elsewhere.", target.display(), name, dir.display()));
        }
    }
    if inside(target, current) {
        return Err(format!("{} is inside the store itself.", target.display()));
    }
    if target.exists() && !(target.is_dir() && is_empty_dir(target)) {
        return Err(format!("{} already has files in it; choose an empty folder.", target.display()));
    }
    Ok(())
}

/// The files of a store to move: everything but the lock and half-written blobs.
fn store_files(root: &Path) -> Result<Vec<(String, PathBuf, u64)>, String> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|e| format!("Reading {} failed: {}", root.display(), e))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(root).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
        if rel == store::LOCK_FILE || rel.starts_with("blobs/tmp/") {
            continue;
        }
        let size = entry.metadata().map_err(|e| e.to_string())?.len();
        out.push((rel, entry.path().to_path_buf(), size));
    }
    Ok(out)
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut f = io::BufReader::new(fs::File::open(path)?);
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            return Ok(format!("{:x}", h.finalize()));
        }
        h.update(&buf[..n]);
    }
}

/// Copies `src` to `dest`, returning the hash of what was read.
fn copy_hashed(src: &Path, dest: &Path) -> io::Result<String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut input = io::BufReader::new(fs::File::open(src)?);
    let mut out = io::BufWriter::new(fs::File::create(dest)?);
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        out.write_all(&buf[..n])?;
    }
    out.flush()?;
    out.into_inner().map_err(|e| e.into_error())?.sync_all()?;
    Ok(format!("{:x}", h.finalize()))
}

/// Points the records of local copies put back from the store at its new folder.
fn repoint_copies(zomboid: &Path, from: &Path, to: &Path) -> Result<usize, String> {
    let mut n = 0;
    origins::update(zomboid, |_, record| {
        if let ModOrigin::Compat { source_dir, .. } = &mut record.origin {
            if let Ok(rest) = Path::new(source_dir.as_str()).strip_prefix(from) {
                *source_dir = to.join(rest).to_string_lossy().to_string();
                n += 1;
                return true;
            }
        }
        false
    })?;
    Ok(n)
}

/// `repoint_copies` after the move is done: a failure is said, not undone (the store works from
/// its new folder; only the copies in use no longer know which version they are).
fn repointed(zomboid: &Path, from: &Path, to: &Path, note: &mut Option<String>) -> usize {
    repoint_copies(zomboid, from, to).unwrap_or_else(|e| {
        *note = Some(format!("Moved, but the record of the local copies put back from it could not be updated: {}", e));
        0
    })
}

/// Moves the store of `zomboid` to the folder `chosen` gives (`target_for`). Refused while a
/// round holds the store; the caller refuses while the game runs. `allow_rename`: on one drive the
/// folder is renamed instead of copied.
pub fn move_store(zomboid: &Path, chosen: &Path, forbidden: &[(PathBuf, &'static str)], allow_rename: bool) -> Result<MoveOutcome, String> {
    move_store_with(zomboid, chosen, forbidden, allow_rename, crate::save_backup::disk_space)
}

/// `move_store` with the drive's free and total bytes from `space`.
fn move_store_with(
    zomboid: &Path,
    chosen: &Path,
    forbidden: &[(PathBuf, &'static str)],
    allow_rename: bool,
    space: impl Fn(&Path) -> Option<(u64, u64)>,
) -> Result<MoveOutcome, String> {
    let current = Store::at(zomboid);
    // The store's own folder chosen again.
    let target = if same_path(chosen, &current.root) { current.root.clone() } else { target_for(chosen) };
    let mut outcome = MoveOutcome {
        from: current.root.to_string_lossy().to_string(),
        to: target.to_string_lossy().to_string(),
        unchanged: false,
        files: 0,
        bytes: 0,
        renamed: false,
        copies_repointed: 0,
        old_left: None,
    };
    if same_path(&target, &current.root) {
        outcome.unchanged = true;
        return Ok(outcome);
    }
    check_target(&target, &current.root, forbidden)?;
    if moving(zomboid) {
        return Err(MOVING.to_string());
    }

    // Nothing kept yet: only the place changes.
    if !current.root.exists() {
        fs::create_dir_all(&target).map_err(|e| format!("Cannot make {}: {}", target.display(), e))?;
        write_place(zomboid, &target)?;
        return Ok(outcome);
    }

    let (store, lock) = Store::open_locked(zomboid, 0).map_err(|e| if e == MOVING { e } else { "A round of keeping is running; try again once it ends.".to_string() })?;
    let mark_path = zomboid.join(MOVING_FILE);
    fs::write(&mark_path, format!("{} {}", std::process::id(), chrono::Local::now().timestamp())).map_err(|e| e.to_string())?;
    let _mark = MovingMark(mark_path);
    let files = store_files(&store.root)?;
    outcome.bytes = files.iter().map(|f| f.2).sum();

    let same_drive = crate::disk_usage::disk_name(&normalized(&store.root)).eq_ignore_ascii_case(&crate::disk_usage::disk_name(&normalized(&target)));
    if allow_rename && same_drive {
        if target.exists() {
            let _ = fs::remove_dir(&target);
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Cannot make {}: {}", parent.display(), e))?;
        }
        if fs::rename(&store.root, &target).is_ok() {
            // The lock went with it: this move holds it no more.
            let _ = fs::remove_file(target.join(store::LOCK_FILE));
            drop(lock);
            outcome.renamed = true;
            write_place(zomboid, &target)?;
            outcome.copies_repointed = repointed(zomboid, &store.root, &target, &mut outcome.old_left);
            return Ok(outcome);
        }
        // Renaming failed (a file held open, another volume behind a link): copy instead.
    }

    // Room: the copy, and the drive's reserve after it.
    if let Some((free, total)) = space(&target) {
        if free < outcome.bytes.saturating_add(store::drive_reserve(total)) {
            return Err(format!(
                "{} has {} MB free; the kept versions take {} MB and the drive keeps {}% of its size free. Choose another drive or lower the limit first.",
                crate::disk_usage::disk_name(&normalized(&target)),
                free >> 20,
                outcome.bytes >> 20,
                store::DRIVE_RESERVE_PERCENT
            ));
        }
    }
    let target_existed = target.exists();
    let undo = |why: String| -> String {
        if target_existed {
            if let Ok(entries) = fs::read_dir(&target) {
                for e in entries.flatten() {
                    let p = e.path();
                    let _ = if p.is_dir() { fs::remove_dir_all(&p) } else { fs::remove_file(&p) };
                }
            }
        } else {
            let _ = fs::remove_dir_all(&target);
        }
        why
    };
    fs::create_dir_all(&target).map_err(|e| format!("Cannot make {}: {}", target.display(), e))?;
    let mut copied: Vec<(PathBuf, String, u64)> = Vec::with_capacity(files.len());
    for (rel, src, size) in &files {
        let dest = rel.split('/').fold(target.clone(), |p, c| p.join(c));
        match copy_hashed(src, &dest) {
            Ok(hash) => copied.push((dest, hash, *size)),
            Err(e) => return Err(undo(format!("Copying {} failed: {}. Nothing was moved.", rel, e))),
        }
    }
    // Read every copy back.
    for (dest, hash, size) in &copied {
        let ok = fs::metadata(dest).map(|m| m.len() == *size).unwrap_or(false) && hash_file(dest).map(|h| &h == hash).unwrap_or(false);
        if !ok {
            return Err(undo(format!("The copy of {} does not match the original. Nothing was moved.", dest.display())));
        }
    }
    outcome.files = copied.len();
    if let Err(e) = write_place(zomboid, &target) {
        return Err(undo(e));
    }
    outcome.copies_repointed = repointed(zomboid, &store.root, &target, &mut outcome.old_left);
    drop(lock);
    if let Err(e) = fs::remove_dir_all(&store.root) {
        outcome.old_left = Some(format!("{} could not be removed completely ({}); the Studio no longer uses it.", store.root.display(), e));
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshots::store::{KeepInfo, VersionMod};

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn tmp(tag: &str) -> Tmp {
        let t = Tmp(std::env::temp_dir().join(format!("pzms-place-{}-{}", tag, std::process::id())));
        let _ = fs::remove_dir_all(&t.0);
        fs::create_dir_all(t.0.join("Zomboid/mods")).unwrap();
        t
    }

    /// A store with one version of item 123, put back as a local copy in Zomboid/mods.
    fn kept_and_in_use(t: &Tmp) -> PathBuf {
        let z = t.0.join("Zomboid");
        let item = t.0.join("ws/123");
        fs::create_dir_all(item.join("mods/Foo/42")).unwrap();
        fs::write(item.join("mods/Foo/42/mod.info"), "id=Foo\nmodversion=1\n").unwrap();
        fs::write(item.join("mods/Foo/42/big.lua"), "x".repeat(40_000)).unwrap();
        fs::write(item.join("mods/Foo/42/pic.png"), (0..5000u32).map(|i| (i * 31 % 251) as u8).collect::<Vec<u8>>()).unwrap();
        let store = Store::at(&z);
        store
            .set_settings(&store::StoreSettings { enabled: true, cap_bytes: 3 << 30, cap_by_player: true })
            .unwrap();
        let info = KeepInfo {
            workshop_id: Some("123".into()),
            by_player: false,
            steam_ident: Some("m1".into()),
            published: Some(1),
            mods: vec![VersionMod { id: "Foo".into(), name: "Foo".into(), folder: "Foo".into(), rel: "mods/Foo".into(), version: Some("1".into()), bytes: 0 }],
        };
        let (v, _) = store.keep("123", &item, info, None).unwrap();
        let listed = crate::snapshots::list_kept_versions(&z.to_string_lossy(), "");
        crate::snapshots::restore_kept_version(&z.to_string_lossy(), &listed[0].path).unwrap();
        v.head_path
    }

    fn check_moved(t: &Tmp, to: &Path, old_head: &Path) {
        let z = t.0.join("Zomboid");
        assert_eq!(Store::at(&z).root, to);
        assert!(!default_root(&z).exists(), "the old folder is gone");
        assert!(!moving(&z));
        let listed = crate::snapshots::list_kept_versions(&z.to_string_lossy(), "");
        assert_eq!(listed.len(), 1);
        assert!(listed[0].path.starts_with(&to.to_string_lossy().to_string()), "{}", listed[0].path);
        assert!(listed[0].in_use, "the copy in Zomboid/mods is still known as this version");
        let origin = origins::load(&z).get("Foo").map(|r| r.origin.clone());
        assert!(matches!(&origin, Some(ModOrigin::Compat { source_dir, .. }) if !Path::new(source_dir).starts_with(old_head.parent().unwrap())), "{:?}", origin);
        assert_eq!(Store::at(&z).settings().cap_bytes, 3 << 30, "the settings travel with the store");
        // The moved store gives the files back.
        let store = Store::at(&z);
        let v = store.versions().into_iter().next().unwrap();
        let dest = t.0.join("again");
        store.restore_mod(&v, "Foo", &dest).unwrap();
        assert_eq!(fs::read_to_string(dest.join("42/big.lua")).unwrap(), "x".repeat(40_000));
        assert!(!to.join(store::LOCK_FILE).exists(), "no lock is left behind");
    }

    #[test]
    fn the_store_is_copied_checked_and_the_old_folder_removed() {
        let t = tmp("copy");
        let z = t.0.join("Zomboid");
        let head = kept_and_in_use(&t);
        let to = t.0.join("Other drive/Kept");
        let out = move_store_with(&z, &to, &[], false, |_| None).expect("moved");
        assert!(!out.renamed && out.files >= 5 && out.copies_repointed == 1, "{:?}", out);
        assert!(out.old_left.is_none());
        check_moved(&t, &to, &head);
        // Back to the default: the place file goes.
        let back = move_store_with(&z, &z, &[], false, |_| None).expect("moved back");
        assert_eq!(PathBuf::from(&back.to), default_root(&z), "Zomboid has files: the store goes in its own folder");
        assert!(chosen_root(&z).is_none());
        assert!(!to.exists());
        assert_eq!(crate::snapshots::list_kept_versions(&z.to_string_lossy(), "").len(), 1);
    }

    #[test]
    fn on_one_drive_the_folder_is_renamed() {
        let t = tmp("rename");
        let z = t.0.join("Zomboid");
        let head = kept_and_in_use(&t);
        let to = t.0.join("Kept");
        let out = move_store_with(&z, &to, &[], true, |_| None).expect("moved");
        assert!(out.renamed && out.copies_repointed == 1, "{:?}", out);
        check_moved(&t, &to, &head);
        assert!(move_store(&z, &to, &[], true).unwrap().unchanged);
    }

    #[test]
    fn a_folder_with_files_gets_the_store_in_a_folder_of_its_own() {
        let t = tmp("busy");
        let z = t.0.join("Zomboid");
        kept_and_in_use(&t);
        let chosen = t.0.join("Stuff");
        fs::create_dir_all(&chosen).unwrap();
        fs::write(chosen.join("notes.txt"), "mine").unwrap();
        let out = move_store_with(&z, &chosen, &[], false, |_| None).unwrap();
        assert_eq!(PathBuf::from(out.to), chosen.join(STORE_DIR));
        assert_eq!(fs::read_to_string(chosen.join("notes.txt")).unwrap(), "mine");
        // A drive's root always gets a folder of its own (nothing is written here).
        let root = PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" });
        assert_eq!(target_for(&root), root.join(STORE_DIR));
    }

    #[test]
    fn forbidden_places_and_a_running_round_are_refused() {
        let t = tmp("refused");
        let z = t.0.join("Zomboid");
        kept_and_in_use(&t);
        let paths = crate::vfs::StudioPaths {
            pz_install_dir: t.0.join("game").to_string_lossy().to_string(),
            workshop_dir: t.0.join("steamapps/workshop/content/108600").to_string_lossy().to_string(),
            user_zomboid_dir: z.to_string_lossy().to_string(),
            mod_list_ini_path: String::new(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        let forbidden = forbidden_dirs(&paths, &z);
        for bad in [
            t.0.join("steamapps/workshop/content/108600/Kept"),
            t.0.join("steamapps/workshop/Kept"),
            t.0.join("game/Kept"),
            z.join("mods/Kept"),
            z.join("PZModStudio_Versions/inner"),
            PathBuf::from("relative/Kept"),
        ] {
            assert!(move_store_with(&z, &bad, &forbidden, false, |_| None).is_err(), "{} must be refused", bad.display());
        }
        let root = PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" });
        assert!(check_target(&root, &default_root(&z), &forbidden).is_err());
        // A round holds the store.
        let (_store, lock) = Store::open_locked(&z, 0).unwrap();
        let err = move_store_with(&z, &t.0.join("Kept"), &forbidden, false, |_| None).unwrap_err();
        assert!(err.contains("round"), "{}", err);
        assert!(!t.0.join("Kept").exists() || is_empty_dir(&t.0.join("Kept")));
        drop(lock);
        // A drive without room for the copy and its 15%: a 1 GB drive with 100 MB free.
        let err = move_store_with(&z, &t.0.join("Kept"), &forbidden, false, |_| Some((100 << 20, 1 << 30))).unwrap_err();
        assert!(err.contains("15%"), "{}", err);
        assert!(!t.0.join("Kept").exists(), "nothing was copied");
        // While a move runs, no round takes the store.
        fs::write(z.join(MOVING_FILE), "1").unwrap();
        assert_eq!(Store::open_locked(&z, 0).err().as_deref(), Some(MOVING));
        fs::remove_file(z.join(MOVING_FILE)).unwrap();
        assert_eq!(Store::at(&z).root, default_root(&z), "nothing moved");
    }

    #[test]
    fn with_nothing_kept_only_the_place_changes() {
        let t = tmp("empty");
        let z = t.0.join("Zomboid");
        let to = t.0.join("Kept");
        let out = move_store(&z, &to, &[], true).unwrap();
        assert_eq!(out.files, 0);
        assert_eq!(Store::at(&z).root, to);
        assert!(to.is_dir());
    }
}
