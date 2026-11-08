//! What a dedicated server needs for Build 42 Java mods (ZombieBuddy), and for mods that only
//! live in the Steam Workshop folder when the server runs without Steam.
//!
//! - ZombieBuddy is not loaded through the mod list alone: the JVM must start with its native
//!   agent, `-agentlib:zbNative` (`zbNative.dll` sits in the game folder, which is already on
//!   `java.library.path`). The game's own launcher reads it from `ProjectZomboid64.json`; a server
//!   started straight with `java.exe` has to be given it.
//! - With the agent, ZombieBuddy asks in a window before loading a jar it has not seen. A server
//!   has no one to answer that window and would wait for good, so the server gets
//!   `policy=deny-new`: it loads the jars this PC already approved (in
//!   `~/.zombie_buddy/mod_approvals.json`, by mod id and the jar's SHA-256) and refuses the rest.
//!   A jar that changed since it was approved (a mod update, a rebuild) is a new jar.
//! - A server started with `-nosteam` reads mods only from `<cachedir>/mods` (the game's
//!   `getCacheDirSub("mods")`; `-modfolders` is not a server option). A mod that exists only in
//!   `steamapps/workshop/content/108600/<item>/mods/<folder>` is "not found" there, and every mod
//!   that `require=`s it is dropped with it. A copy in `Zomboid/mods` fixes that. Not a link: with
//!   some Workshop mods linked there (a directory junction), a NoSteam server crashes while it
//!   creates the world (`NetChecksum$Checksummer.addFile` gets a null path from ScriptManager, then
//!   the map's WorldGenOverride.lua fails and the server says "Server Terminated" but stays up).
//!   A copy carries a marker with how new the Workshop folder was, so an update shows as stale.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::catalog::{CatalogMod, HealthIssue, ModCatalog, ModSource, Severity};

/// What the server's JVM gets when its `Mods=` has ZombieBuddy.
pub const ZOMBIEBUDDY_AGENT_ARG: &str = "-agentlib:zbNative=policy=deny-new";

const ZOMBIEBUDDY_ID: &str = "ZombieBuddy";

/// Whether the `.ini`'s `Mods=` names ZombieBuddy (B42 writes it `\ZombieBuddy`).
pub fn mods_line_has_zombiebuddy(ini_content: &str) -> bool {
    super::mod_sync::ini_list(ini_content, "Mods").iter().any(|m| m.eq_ignore_ascii_case(ZOMBIEBUDDY_ID))
}

/// The agent argument for this server's JVM, when it needs one.
pub fn agent_arg_for(ini_content: &str) -> Option<&'static str> {
    mods_line_has_zombiebuddy(ini_content).then_some(ZOMBIEBUDDY_AGENT_ARG)
}

/// One decision ZombieBuddy remembers: this mod's jar with this hash, loaded or refused.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Approval {
    pub id: String,
    pub jar_hash: String,
    pub decision: bool,
}

#[derive(Deserialize)]
struct ApprovalsFile {
    #[serde(default)]
    mods: Vec<ApprovalEntry>,
}

#[derive(Deserialize)]
struct ApprovalEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    jar_hash: String,
    #[serde(default)]
    decision: bool,
}

/// `%USERPROFILE%\.zombie_buddy\mod_approvals.json`.
pub fn approvals_path() -> Option<PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".zombie_buddy").join("mod_approvals.json"))
}

/// The approvals in that file; empty when there is none (nothing approved yet).
pub fn read_approvals(path: &Path) -> Vec<Approval> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<ApprovalsFile>(t.trim_start_matches('\u{feff}')).ok())
        .map(|f| {
            f.mods
                .into_iter()
                .map(|e| Approval { id: e.id, jar_hash: e.jar_hash.to_ascii_lowercase(), decision: e.decision })
                .collect()
        })
        .unwrap_or_default()
}

fn sha256_file(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).ok()?;
    Some(Sha256::digest(&bytes).iter().map(|b| format!("{:02x}", b)).collect())
}

/// The jars a mod's `mod.info` names (`javaJarFile=`), resolved the way ZombieBuddy does:
/// relative to the folder of that `mod.info`.
pub fn java_jars(m: &CatalogMod) -> Vec<PathBuf> {
    let Some(info_path) = m.mod_info_path.as_ref().map(PathBuf::from) else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(&info_path) else { return Vec::new() };
    let base = info_path.parent().map(Path::to_path_buf).unwrap_or_default();
    text.lines()
        .filter_map(|l| {
            let l = l.trim().trim_start_matches('\u{feff}');
            let (key, value) = l.split_once('=')?;
            key.trim().eq_ignore_ascii_case("javaJarFile").then(|| value.trim().to_string())
        })
        .flat_map(|v| v.split([',', ';']).map(|s| s.trim().to_string()).collect::<Vec<_>>())
        .filter(|v| !v.is_empty())
        .map(|v| base.join(v.replace('\\', "/")))
        .collect()
}

/// What a server with this `Mods=` will do with the Java mods in it, as issues: a jar
/// ZombieBuddy has not approved on this PC (refused under `deny-new`), one it was told to refuse,
/// and Java mods with no ZombieBuddy in the list at all.
pub fn java_issues(catalog: &ModCatalog, mods: &[String], approvals: &[Approval]) -> Vec<HealthIssue> {
    let mut issues = Vec::new();
    let has_zb = mods.iter().any(|m| m.eq_ignore_ascii_case(ZOMBIEBUDDY_ID));
    let mut without_zb: Vec<String> = Vec::new();
    for entry in mods {
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if m.id.eq_ignore_ascii_case(ZOMBIEBUDDY_ID) {
            continue; // its own jar is the agent's, not one it approves
        }
        let jars: Vec<PathBuf> = java_jars(m).into_iter().filter(|j| j.is_file()).collect();
        if jars.is_empty() {
            continue;
        }
        if !has_zb {
            without_zb.push(m.name.clone());
            continue;
        }
        for jar in jars {
            let Some(hash) = sha256_file(&jar) else { continue };
            let jar_name = jar.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let decided = approvals.iter().rev().find(|a| a.id == m.id && a.jar_hash == hash);
            match decided {
                Some(a) if a.decision => {}
                Some(_) => issues.push(issue(
                    Severity::Error,
                    "server_java_denied",
                    format!(
                        "{}: ZombieBuddy was told to refuse {} on this PC, so the server does not load it either. Allow it from ZombieBuddy's window in the game (or remove its line from ~/.zombie_buddy/mod_approvals.json).",
                        m.name, jar_name
                    ),
                    &[("mod", &m.name), ("jar", &jar_name)],
                    &m.id,
                )),
                None => issues.push(issue(
                    Severity::Error,
                    "server_java_not_approved",
                    format!(
                        "{}: its jar {} is not approved in ZombieBuddy on this PC yet (new or changed since it was approved). The server only loads approved jars (it has no window to ask), so start the game once with this mod on and allow it in ZombieBuddy's window, then start the server.",
                        m.name, jar_name
                    ),
                    &[("mod", &m.name), ("jar", &jar_name)],
                    &m.id,
                )),
            }
        }
    }
    if !without_zb.is_empty() {
        let names = without_zb.join(", ");
        issues.push(issue(
            Severity::Warning,
            "server_java_without_zombiebuddy",
            format!(
                "{} carry Java code (javaJarFile=), but Mods= has no ZombieBuddy: their jars never load on the server, only their Lua.",
                names
            ),
            &[("mods", &names)],
            "",
        ));
    }
    issues
}

/// Why a mod of `Mods=` needs a copy in `Zomboid/mods`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CopyNeed {
    /// Only in the Workshop folder: a server without Steam reports it "not found".
    Missing,
    /// `Zomboid/mods` has a link (junction) to the Workshop folder instead of a copy. A NoSteam
    /// server crashes at world creation with some of those (ScriptManager gets a null path from
    /// `NetChecksum$Checksummer.addFile`, then the map's WorldGenOverride.lua fails).
    Linked,
    /// A copy the Studio made, and the Workshop item has been updated since.
    Stale,
}

/// A mod of `Mods=` that needs a (fresh) copy in `Zomboid/mods`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkshopCopy {
    pub id: String,
    pub name: String,
    /// The mod's folder in the Workshop item (`.../108600/<item>/mods/<folder>`).
    pub source: String,
    /// Its folder name, the name the copy gets in `Zomboid/mods`.
    pub folder: String,
    pub workshop_id: String,
    pub need: CopyNeed,
    /// The mods of `Mods=` that require it, dropped by the server with it.
    pub required_by: Vec<String>,
}

/// The file a copy made by the Studio carries, saying where it came from and how new that was.
pub const COPY_MARKER: &str = ".pzms_workshop_copy.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CopyMarker {
    id: String,
    workshop_id: String,
    source: String,
    /// The newest modification time (unix seconds) in the Workshop folder when it was copied.
    source_newest: u64,
    copied_at: u64,
}

fn unix_secs(t: std::time::SystemTime) -> u64 {
    t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The newest modification time of anything in a folder (Steam rewrites the files it updates).
fn newest_mtime(root: &Path) -> u64 {
    walkdir::WalkDir::new(root)
        .into_iter()
        .flatten()
        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        .map(unix_secs)
        .max()
        .unwrap_or(0)
}

fn read_marker(copy_root: &Path) -> Option<CopyMarker> {
    serde_json::from_str(&std::fs::read_to_string(copy_root.join(COPY_MARKER)).ok()?).ok()
}

/// Whether a folder is a link (junction or symbolic link) rather than a real folder.
fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) || is_reparse_point(path)
}

#[cfg(target_os = "windows")]
fn is_reparse_point(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

#[cfg(not(target_os = "windows"))]
fn is_reparse_point(_path: &Path) -> bool {
    false
}

fn same_folder(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// The mods of `Mods=` that need a copy of their Workshop folder in `Zomboid/mods`: those only
/// in the Workshop folder (when `nosteam`: a server with Steam downloads them), those linked
/// there instead of copied, and the Studio's own copies the Workshop item has outgrown. A copy
/// someone made by hand (no marker) is theirs and is left alone.
pub fn copies_needed(catalog: &ModCatalog, mods: &[String], nosteam: bool) -> Vec<WorkshopCopy> {
    let mut out: Vec<WorkshopCopy> = Vec::new();
    for entry in mods {
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if out.iter().any(|w| w.id == m.id) {
            continue;
        }
        let Some((ws, workshop_id)) = catalog.mods.iter().find_map(|c| match &c.source {
            ModSource::Workshop { workshop_id } if c.id == m.id => Some((c, workshop_id.clone())),
            _ => None,
        }) else {
            continue;
        };
        let local = catalog.mods.iter().find(|c| c.id == m.id && matches!(c.source, ModSource::Local));
        let need = match local {
            None if nosteam => CopyNeed::Missing,
            None => continue,
            Some(l) => {
                let root = Path::new(&l.root_dir);
                if is_link(root) {
                    if !same_folder(root, Path::new(&ws.root_dir)) {
                        continue; // a link to something else (a mod in development) is not ours to judge
                    }
                    CopyNeed::Linked
                } else {
                    match read_marker(root) {
                        Some(marker) if newest_mtime(Path::new(&ws.root_dir)) > marker.source_newest => CopyNeed::Stale,
                        _ => continue,
                    }
                }
            }
        };
        let folder = Path::new(&ws.root_dir).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if folder.is_empty() {
            continue;
        }
        let required_by: Vec<String> = mods
            .iter()
            .filter_map(|e| catalog.resolve(e.trim()))
            .filter(|d| d.info.require.iter().any(|r| r.trim().trim_start_matches('\\') == m.id))
            .map(|d| d.name.clone())
            .collect();
        out.push(WorkshopCopy {
            id: m.id.clone(),
            name: m.name.clone(),
            source: ws.root_dir.clone(),
            folder,
            workshop_id,
            need,
            required_by,
        });
    }
    out
}

pub fn copy_issues(found: &[WorkshopCopy]) -> Vec<HealthIssue> {
    found
        .iter()
        .map(|w| {
            let dropped = w.required_by.join(", ");
            let also = if dropped.is_empty() { String::new() } else { format!(" and also drops {}, which require it", dropped) };
            let (severity, code, message) = match w.need {
                CopyNeed::Missing => (
                    Severity::Error,
                    "server_mod_workshop_only",
                    format!(
                        "{} is only in the Steam Workshop folder. A server started without Steam (NoSteam) only reads Zomboid/mods, so it reports it \"not found\"{}. Copy it into Zomboid/mods.",
                        w.name, also
                    ),
                ),
                CopyNeed::Linked => (
                    Severity::Error,
                    "server_mod_workshop_linked",
                    format!(
                        "{} is in Zomboid/mods as a link to its Workshop folder, not a copy. With some linked mods a server started without Steam crashes while it creates the world (\"absPath:null\", then \"Server Terminated\"). Replace the link with a copy.",
                        w.name
                    ),
                ),
                CopyNeed::Stale => (
                    Severity::Warning,
                    "server_mod_copy_stale",
                    format!(
                        "{}: its copy in Zomboid/mods is older than its Workshop item ({}), which was updated since. The server, and the game on this PC, still load the old copy. Copy it again.",
                        w.name, w.workshop_id
                    ),
                ),
            };
            let mut params: Vec<(&str, &str)> = vec![("mod", w.name.as_str()), ("item", w.workshop_id.as_str())];
            if !dropped.is_empty() && w.need == CopyNeed::Missing {
                params.push(("variant", "required"));
                params.push(("dropped", dropped.as_str()));
            }
            issue(severity, code, message, &params, &w.id)
        })
        .collect()
}

/// Copies each mod's Workshop folder into `<zomboid>/mods/<folder>`, with a marker saying where
/// from and how new it was (so a later Workshop update shows as stale). Replaces only what the
/// check found there: a link to that same Workshop folder, or the Studio's own stale copy; any
/// other folder of that name is never touched. Returns the copies made.
pub fn copy_into_mods(user_zomboid_dir: &Path, found: &[WorkshopCopy]) -> Result<Vec<String>, String> {
    let mods_dir = user_zomboid_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| format!("Cannot create {}: {}", mods_dir.display(), e))?;
    let mut made = Vec::new();
    for w in found {
        if w.folder.is_empty() || w.folder.contains(['/', '\\']) || w.folder.contains("..") {
            return Err(format!("Invalid folder name: {}", w.folder));
        }
        let source = Path::new(&w.source);
        let dest = mods_dir.join(&w.folder);
        let exists = std::fs::symlink_metadata(&dest).is_ok();
        // What is there now must be what the check saw, or it is not the Studio's to replace.
        let replace_ok = match w.need {
            CopyNeed::Missing => !exists,
            CopyNeed::Linked => is_link(&dest) && same_folder(&dest, source),
            CopyNeed::Stale => !is_link(&dest) && read_marker(&dest).is_some_and(|m| m.id == w.id),
        };
        if !replace_ok {
            return Err(format!(
                "Zomboid/mods already has a folder called {} that the Studio did not put there for {}: rename or move it first.",
                w.folder, w.id
            ));
        }

        let newest = newest_mtime(source);
        let tmp = mods_dir.join(format!(".{}.pzms-copying", w.folder));
        let _ = std::fs::remove_dir_all(&tmp);
        if let Err(e) = copy_tree(source, &tmp) {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(format!("Could not copy {}: {}", w.name, e));
        }
        let marker = CopyMarker {
            id: w.id.clone(),
            workshop_id: w.workshop_id.clone(),
            source: w.source.clone(),
            source_newest: newest,
            copied_at: unix_secs(std::time::SystemTime::now()),
        };
        std::fs::write(tmp.join(COPY_MARKER), serde_json::to_string_pretty(&marker).unwrap_or_default())
            .map_err(|e| format!("Could not write {}: {}", COPY_MARKER, e))?;

        if exists {
            let removed = if w.need == CopyNeed::Linked {
                // Removes the link itself, never what it points to.
                std::fs::remove_dir(&dest)
            } else {
                std::fs::remove_dir_all(&dest)
            };
            if let Err(e) = removed {
                let _ = std::fs::remove_dir_all(&tmp);
                return Err(format!("Could not replace {} (is the game or a server using it?): {}", dest.display(), e));
            }
        }
        std::fs::rename(&tmp, &dest).map_err(|e| format!("Could not put the copy of {} in place: {}", w.name, e))?;
        made.push(dest.to_string_lossy().to_string());
    }
    Ok(made)
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    for entry in walkdir::WalkDir::new(from) {
        let entry = entry.map_err(std::io::Error::other)?;
        let rel = entry.path().strip_prefix(from).map_err(std::io::Error::other)?;
        let target = to.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn issue(severity: Severity, code: &str, message: String, params: &[(&str, &str)], mod_id: &str) -> HealthIssue {
    HealthIssue {
        params: crate::catalog::issue_params(params),
        details: vec![],
        severity,
        code: code.to_string(),
        mod_id: (!mod_id.is_empty()).then(|| mod_id.to_string()),
        message,
        fixes: Vec::new(),
        related: Vec::new(),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pzms-javamods-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_mod(root: &Path, info: &str) {
        fs::create_dir_all(root.join("42")).unwrap();
        fs::write(root.join("42").join("mod.info"), info).unwrap();
    }

    #[test]
    fn the_agent_goes_in_only_when_mods_has_zombiebuddy() {
        assert_eq!(agent_arg_for("Mods=\\ZombieBuddy;\\KnoxLives\r\n"), Some(ZOMBIEBUDDY_AGENT_ARG));
        assert_eq!(agent_arg_for("Mods=KnoxLives;ZombieBuddy\n"), Some(ZOMBIEBUDDY_AGENT_ARG));
        assert_eq!(agent_arg_for("Mods=KnoxLives\nWorkshopItems=ZombieBuddy\n"), None);
        assert_eq!(agent_arg_for("Mods=ZombieBuddyAddon\n"), None);
        assert!(ZOMBIEBUDDY_AGENT_ARG.ends_with("policy=deny-new"), "a server must never wait on the approval window");
    }

    #[test]
    fn java_jars_are_flagged_until_their_hash_is_approved() {
        let base = temp("approve");
        let workshop = base.join("108600");
        let zomboid = base.join("Zomboid");
        fs::create_dir_all(&workshop).unwrap();
        let zb = zomboid.join("mods").join("ZombieBuddy");
        write_mod(&zb, "id=ZombieBuddy\nname=ZombieBuddy\njavaJarFile=../libs/ZombieBuddy.jar\n");
        fs::create_dir_all(zb.join("libs")).unwrap();
        fs::write(zb.join("libs").join("ZombieBuddy.jar"), b"zb").unwrap();
        let kl = zomboid.join("mods").join("KnoxLives");
        write_mod(&kl, "id=KnoxLives\nname=Knox Lives\njavaJarFile=media/java/KnoxLives.jar\n");
        fs::create_dir_all(kl.join("42").join("media").join("java")).unwrap();
        let jar = kl.join("42").join("media").join("java").join("KnoxLives.jar");
        fs::write(&jar, b"version 1").unwrap();

        let catalog = crate::catalog::build_catalog(&workshop.to_string_lossy(), &zomboid.to_string_lossy(), None);
        let mods = vec!["ZombieBuddy".to_string(), "KnoxLives".to_string()];
        let m = catalog.resolve("KnoxLives").unwrap();
        assert_eq!(java_jars(m), vec![kl.join("42").join("media/java/KnoxLives.jar")]);

        let codes = |a: &[Approval]| java_issues(&catalog, &mods, a).into_iter().map(|i| i.code).collect::<Vec<_>>();
        assert_eq!(codes(&[]), vec!["server_java_not_approved"], "ZombieBuddy's own jar is never asked about");

        let hash = sha256_file(&jar).unwrap();
        assert_eq!(hash.len(), 64);
        let approved = Approval { id: "KnoxLives".into(), jar_hash: hash.clone(), decision: true };
        assert!(codes(&[approved.clone()]).is_empty());
        let other_mod = Approval { id: "Other".into(), jar_hash: hash.clone(), decision: true };
        assert_eq!(codes(&[other_mod]), vec!["server_java_not_approved"], "approvals are per mod id");
        let denied = Approval { id: "KnoxLives".into(), jar_hash: hash, decision: false };
        assert_eq!(codes(&[denied]), vec!["server_java_denied"]);

        // A rebuilt jar is a new jar.
        fs::write(&jar, b"version 2").unwrap();
        assert_eq!(codes(&[approved]), vec!["server_java_not_approved"]);

        // Without ZombieBuddy in Mods= the jar never loads at all.
        let no_zb = java_issues(&catalog, &["KnoxLives".to_string()], &[]);
        assert_eq!(no_zb.iter().map(|i| i.code.as_str()).collect::<Vec<_>>(), vec!["server_java_without_zombiebuddy"]);

        // The approvals file as ZombieBuddy writes it.
        let file = base.join("mod_approvals.json");
        fs::write(&file, r#"{"formatVersion":2,"mods":[{"id":"KnoxLives","jar_hash":"ABC","decision":true,"author_id":1},{"id":"X","workshop_id":5,"jar_hash":"def","decision":false}]}"#).unwrap();
        assert_eq!(
            read_approvals(&file),
            vec![
                Approval { id: "KnoxLives".into(), jar_hash: "abc".into(), decision: true },
                Approval { id: "X".into(), jar_hash: "def".into(), decision: false }
            ]
        );
        assert!(read_approvals(&base.join("missing.json")).is_empty());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn workshop_mods_are_copied_never_linked_and_copies_go_stale() {
        let base = temp("nosteam");
        let workshop = base.join("108600");
        let zomboid = base.join("Zomboid");
        let ws_root = workshop.join("3396446795").join("mods").join("MoodleFramework");
        write_mod(&ws_root, "id=MoodleFramework\nname=Moodle Framework\n");
        fs::create_dir_all(ws_root.join("42").join("media").join("scripts")).unwrap();
        fs::write(ws_root.join("42").join("media").join("scripts").join("a.txt"), "module Base {}").unwrap();
        write_mod(&zomboid.join("mods").join("NaturesCall"), "id=NC_NaturesCall\nname=Nature's Call\nrequire=\\MoodleFramework\n");
        let mods = vec!["MoodleFramework".to_string(), "NC_NaturesCall".to_string()];
        let catalog = || crate::catalog::build_catalog(&workshop.to_string_lossy(), &zomboid.to_string_lossy(), None);

        // With Steam the server downloads it: nothing to copy. Without, it is missing.
        assert!(copies_needed(&catalog(), &mods, false).is_empty());
        let found = copies_needed(&catalog(), &mods, true);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].need, found[0].folder.as_str(), found[0].workshop_id.as_str()), (CopyNeed::Missing, "MoodleFramework", "3396446795"));
        assert_eq!(found[0].required_by, vec!["Nature's Call"]);
        let issues = copy_issues(&found);
        assert_eq!(issues[0].code, "server_mod_workshop_only");
        assert_eq!(issues[0].params.get("variant").map(String::as_str), Some("required"));

        // Copied: a real folder with its marker, and nothing more to do.
        let made = copy_into_mods(&zomboid, &found).unwrap();
        let dest = zomboid.join("mods").join("MoodleFramework");
        assert_eq!(made, vec![dest.to_string_lossy().to_string()]);
        assert!(!is_link(&dest) && dest.join("42").join("media").join("scripts").join("a.txt").is_file());
        assert!(dest.join(COPY_MARKER).is_file());
        assert!(copies_needed(&catalog(), &mods, true).is_empty());
        assert!(copy_into_mods(&zomboid, &found).is_err(), "a folder already there is never replaced as missing");

        // The Workshop item updates: the copy is stale (with Steam too: the game loads the copy).
        let mut marker = read_marker(&dest).unwrap();
        marker.source_newest -= 10;
        fs::write(dest.join(COPY_MARKER), serde_json::to_string(&marker).unwrap()).unwrap();
        let stale = copies_needed(&catalog(), &mods, false);
        assert_eq!(stale.iter().map(|w| w.need).collect::<Vec<_>>(), vec![CopyNeed::Stale]);
        assert_eq!(copy_issues(&stale)[0].code, "server_mod_copy_stale");
        copy_into_mods(&zomboid, &stale).unwrap();
        assert!(copies_needed(&catalog(), &mods, false).is_empty(), "copied again, fresh");

        // A copy made by hand (no marker) is the player's: never flagged.
        fs::remove_file(dest.join(COPY_MARKER)).unwrap();
        assert!(copies_needed(&catalog(), &mods, true).is_empty());
        fs::remove_dir_all(&dest).unwrap();

        // A link to the Workshop folder (what an older Studio made) is replaced by a copy.
        #[cfg(target_os = "windows")]
        {
            let out = std::process::Command::new("cmd.exe").arg("/c").arg("mklink").arg("/J").arg(&dest).arg(&ws_root).output().unwrap();
            assert!(out.status.success());
            let linked = copies_needed(&catalog(), &mods, false);
            assert_eq!(linked.iter().map(|w| w.need).collect::<Vec<_>>(), vec![CopyNeed::Linked]);
            assert_eq!(copy_issues(&linked)[0].code, "server_mod_workshop_linked");
            copy_into_mods(&zomboid, &linked).unwrap();
            assert!(!is_link(&dest) && dest.join(COPY_MARKER).is_file());
            assert!(ws_root.join("42").join("mod.info").is_file(), "the Workshop folder the link pointed to is untouched");
            assert!(copies_needed(&catalog(), &mods, true).is_empty());
        }
        let _ = fs::remove_dir_all(&base);
    }
}
