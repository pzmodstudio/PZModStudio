//! Java mods ZombieBuddy left out of a session, and what that did to the mods built on them.
//!
//! ZombieBuddy checks each Java mod's signature (`.zbs`) against its author's Steam profile. When it
//! does not match, it writes `[ZB] Excluded: ...\mod.info (invalid ZBS: ..., modId=X)` and the mod's
//! Java never loads. Addons compiled against that mod's classes then fail with
//! `NoClassDefFoundError: <its package>/...`: some only switch a feature off, but one exposed to
//! Lua takes the game or the server down at boot (`Exception in thread "main"`), and the player
//! sees only "the server did not start". Seen on 2026-11-16 with Viewpoint 0.1.7, whose jar was
//! signed with a key its author's profile does not show.
//!
//! It also leaves a mod out by policy (`blocked by policy=prompt`): the player's approvals file
//! (`~/.zombie_buddy/mod_approvals.json`) holds a saved "no" for that very jar, so its dialog never
//! asks again. Seen on 2026-11-16: a "no" saved during Viewpoint's signature hours kept the fixed jar
//! out, and its addons took the game down. ZombieBuddy shows the dialog again when Shift is held while
//! the game loads; that file is the player's decision and the Studio never writes it.
//!
//! Read from a log's text; nothing here decides for the player.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExcludedMod {
    /// The mod id ZombieBuddy names (`modId=`).
    pub mod_id: String,
    /// "signature": the `.zbs` did not verify; "denied": blocked by `policy=prompt` (a "no" saved for
    /// this jar in ZombieBuddy's approvals, or given in its dialog); "unapproved": blocked by another
    /// policy (`deny-new`, the Studio's servers: a jar never approved on this PC); "other": `reason`.
    pub kind: String,
    pub reason: String,
    /// Mods (by the tag they log under, or the class exposed to Lua) that failed for want of its classes.
    pub addons: Vec<String>,
    /// A missing class of this mod stopped the session at boot (the game or the server exited).
    pub fatal: bool,
}

fn after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    line.find(marker).map(|i| &line[i + marker.len()..])
}

/// The class a `NoClassDefFoundError` or `ClassNotFoundException` names, as `a/b/C`.
fn missing_class(line: &str) -> Option<String> {
    let rest = after(line, "NoClassDefFoundError: ").or_else(|| after(line, "ClassNotFoundException: "))?;
    let class: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '/' || *c == '.' || *c == '_' || *c == '$').collect();
    (!class.is_empty()).then(|| class.replace('.', "/"))
}

/// The `[Tag]` a mod logs under, at the start of the message part of a line.
fn tag_of(line: &str) -> Option<String> {
    let msg = line.rsplit_once("> ").map(|(_, m)| m).unwrap_or(line).trim_start();
    let inner = msg.strip_prefix('[')?;
    let end = inner.find(']')?;
    let tag = &inner[..end];
    (tag != "ZB" && !tag.is_empty() && tag.len() <= 40).then(|| tag.to_string())
}

/// Whether a missing class belongs to the excluded mod: its first package segment is the mod's id
/// (Viewpoint -> `viewpoint/...`), or the Java package its mod.info declares.
fn belongs(class: &str, mod_id: &str, packages: &[String]) -> bool {
    let first = class.split('/').next().unwrap_or_default();
    first.eq_ignore_ascii_case(mod_id) || packages.iter().any(|p| class.starts_with(&p.replace('.', "/")))
}

/// The mods ZombieBuddy excluded in this log, each with the addons that failed without it.
/// `packages`: for a mod id, the Java packages its mod.info names (may be empty).
pub fn analyze(log: &str, packages: impl Fn(&str) -> Vec<String>) -> Vec<ExcludedMod> {
    let mut out: Vec<ExcludedMod> = Vec::new();
    for line in log.lines() {
        let Some(rest) = after(line, "[ZB] Excluded: ") else { continue };
        let Some(open) = rest.find(" (") else { continue };
        // "(blocked by policy=prompt, modId=X)." ends with a full stop; "(invalid ZBS: ..., modId=X)" does not.
        let why = rest[open + 2..].trim_end().trim_end_matches('.').trim_end_matches(')');
        if why.starts_with("loaded as Java agent") {
            continue; // ZombieBuddy itself
        }
        let Some(id) = after(why, "modId=").map(|s| s.split([')', ',']).next().unwrap_or_default().trim().to_string()) else { continue };
        if id.is_empty() {
            continue;
        }
        if out.iter().any(|e| e.mod_id == id) {
            continue;
        }
        let kind = if why.contains("invalid ZBS") {
            "signature"
        } else if why.contains("blocked by policy=prompt") {
            "denied"
        } else if why.contains("blocked by policy") {
            "unapproved"
        } else {
            "other"
        };
        let reason = why.split(", modId=").next().unwrap_or(why).to_string();
        out.push(ExcludedMod { mod_id: id, kind: kind.to_string(), reason, addons: Vec::new(), fatal: false });
    }
    if out.is_empty() {
        return out;
    }
    let pkgs: Vec<Vec<String>> = out.iter().map(|e| packages(&e.mod_id)).collect();
    let lines: Vec<&str> = log.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(class) = missing_class(line) else { continue };
        let Some(at) = out.iter().position(|e| belongs(&class, &e.mod_id, &pkgs[out.iter().position(|x| x.mod_id == e.mod_id).unwrap()])) else { continue };
        let main = line.contains("Exception in thread \"main\"") || (i > 0 && lines[i - 1].contains("Exception in thread \"main\""));
        let who = if main {
            out[at].fatal = true;
            // The class being exposed to Lua when it failed: the last one ZombieBuddy named before.
            lines[..i].iter().rev().take(40).find_map(|l| after(l, "Exposing class to Lua: ").map(|s| s.rsplit(" as ").next().unwrap_or(s).trim().to_string()))
        } else {
            tag_of(line)
        };
        if let Some(who) = who.filter(|w| !w.eq_ignore_ascii_case(&out[at].mod_id)) {
            if !out[at].addons.contains(&who) {
                out[at].addons.push(who);
            }
        }
    }
    out
}

/// Whether ZombieBuddy put a jar of the mod in folder `folder` (`...\mods\<folder>\...`) on the
/// classpath in this log: its Java loaded in that session.
pub fn java_loaded(log: &str, folder: &str) -> bool {
    if folder.is_empty() {
        return false;
    }
    let needle = format!("\\mods\\{}\\", folder.to_ascii_lowercase());
    log.lines().filter_map(|l| after(l, "[ZB] added to classpath: ")).any(|p| p.to_ascii_lowercase().replace('/', "\\").contains(&needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = r#"LOG  : General      f:0 st:5,712,225> [ZB] ZBS verification invalid for C:\Users\x\Zomboid\mods\Viewpoint\42\media\java\client\Viewpoint.jar: uploaderID=N/A, shortMessage=Could not verify signature.
LOG  : General      f:0 st:5,712,982> [ZB] Excluded: C:\Users\x\Zomboid\mods\ZombieBuddy\42\mod.info (loaded as Java agent, skipping normal mod loading) (version 2.3.4)
LOG  : General      f:0 st:5,712,982> [ZB] Excluded: C:\Users\x\Zomboid\mods\Viewpoint\common\mod.info (invalid ZBS: Could not find JavaModZBS:<64 hex> on Steam profile — add it to your profile summary., modId=Viewpoint)
LOG  : General      f:0 st:5,714,042> [ProjectFixit] WARN Surface Fix did not start: java.lang.NoClassDefFoundError: viewpoint/world/WorldMesher
LOG  : General      f:0 st:5,718,573> [VVC Viewpoint] addon visuals stopped for this session: java.lang.NoClassDefFoundError: viewpoint/models/Models
LOG  : General      f:0 st:5,718,600> [Other] java.lang.NoClassDefFoundError: somethingelse/Thing
LOG  : General      f:0 st:5,729,658> [ZB] Exposing class to Lua: BlackHawkViewpoint
LOG  : General      f:0 st:5,729,658> [ZB] Exposing class to Lua: Tracers as VPTTracers
LOG  : General      f:0 st:5,729,658> Exception in thread "main" LOG  : General      f:0 st:5,729,659> java.lang.NoClassDefFoundError: viewpoint/core/Frame
LOG  : General      f:0 st:5,729,660> Caused by: java.lang.ClassNotFoundException: viewpoint.core.Frame
"#;

    #[test]
    fn names_the_excluded_mod_its_addons_and_that_it_stopped_the_boot() {
        let found = analyze(LOG, |_| Vec::new());
        assert_eq!(found.len(), 1, "ZombieBuddy's own line is not an exclusion: {:?}", found);
        let vp = &found[0];
        assert_eq!((vp.mod_id.as_str(), vp.kind.as_str(), vp.fatal), ("Viewpoint", "signature", true));
        assert_eq!(vp.addons, vec!["ProjectFixit", "VVC Viewpoint", "VPTTracers"], "the class of another package is not counted");
        assert!(vp.reason.starts_with("invalid ZBS"));
    }

    #[test]
    fn a_no_saved_in_zombiebuddy_is_told_apart() {
        let log = "[16-11-26 10:30:28.728] LOG  : General      f:0> [ZB] Excluded: C:\\Users\\x\\Zomboid\\mods\\Viewpoint\\common\\mod.info (blocked by policy=prompt, modId=Viewpoint).\n\
[16-11-26 10:30:29.000] LOG  : General      f:0> [ZB] Excluded: C:\\x\\Other\\mod.info (blocked by policy=deny-new, modId=Other).\n";
        let found = analyze(log, |_| Vec::new());
        let kinds: Vec<(&str, &str)> = found.iter().map(|e| (e.mod_id.as_str(), e.kind.as_str())).collect();
        assert_eq!(kinds, vec![("Viewpoint", "denied"), ("Other", "unapproved")], "the full stop after the reason is not part of the id");
        assert_eq!(found[0].reason, "blocked by policy=prompt");
    }

    #[test]
    fn a_jar_on_the_classpath_is_a_java_that_loaded() {
        let log = "LOG  : General      f:0> [ZB] added to classpath: C:\\Users\\x\\Zomboid\\mods\\Viewpoint\\42\\media\\java\\client\\Viewpoint.jar.\n";
        assert!(java_loaded(log, "Viewpoint"));
        assert!(!java_loaded(log, "ViewpointRTX"), "another folder that starts the same is not it");
        assert!(!java_loaded(LOG, "Viewpoint"), "an excluded mod never reaches the classpath");
    }

    #[test]
    fn nothing_without_an_exclusion() {
        assert!(analyze("LOG > [X] java.lang.NoClassDefFoundError: viewpoint/core/Frame\n", |_| Vec::new()).is_empty());
    }
}

#[cfg(test)]
mod live {
    /// `PZMS_LOG=<a console.txt or server-console.txt> cargo test --release --lib zb_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn zb_live() {
        let Ok(path) = std::env::var("PZMS_LOG") else { return };
        let text = std::fs::read_to_string(path).unwrap();
        println!("{:#?}", super::analyze(&text, |_| Vec::new()));
    }
}
