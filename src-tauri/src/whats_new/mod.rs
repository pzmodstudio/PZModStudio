//! What's new since you last played: the Workshop mods whose version on disk was published after
//! the last session, and the ones with an update waiting in Steam, each with its change notes
//! and a mark when the update may break a save in progress.
//!
//! - "Last played" is the latest of the sessions the Studio Link recorded
//!   (`PZModStudio_SaveRefs.json`) and Steam's `TimeLastAppRan` for the game.
//! - Versions come from Steam's own record (`appworkshop_108600.acf`): offline, no key.
//! - Change notes come from the item's public change notes page, one `<p id="<unix time>">` per
//!   update. They are cached in `<Zomboid>/PZModStudio_Changelogs.json` and read again only
//!   when Steam knows a newer version than the cached one; at most `PAGES_PER_READ` pages a
//!   read, through the shared limiter, stopping at Steam's first "slow down".
//! - "May break saves" is a guess, said as one: a map mod (its cells can change under a save),
//!   or notes that talk about new saves, wipes, removed or renamed things, reworks.
//! - The changelog a mod ships in its own folder (`mod_changelog`) is read too: offline, and for
//!   mods that are not on the Workshop (Zomboid/mods: local, Nexus). What the player already read
//!   in the game's alert system is not new; a local mod's entries are new until "Mark as read"
//!   (the newest one seen is kept in `PZModStudio_ChangelogSeen.json`, the first time without
//!   showing anything, or every mod would be new).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One update of one item, as its change notes page has it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeNote {
    /// When it was published (unix seconds).
    pub at: u64,
    pub text: String,
}

/// Why an update may break a save.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Risk {
    /// "map" or "notes".
    pub code: String,
    /// "breaks": a save in progress does not take it (a map mod, or notes saying so outright);
    /// "read": words found in the notes (something redone, taken out, an id changed, "new game"
    /// without saying it is required), worth reading before loading a save.
    pub level: String,
    /// The words of the notes that raised it.
    pub detail: String,
}

/// A profile that has the mod on.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileOn {
    pub name: String,
    /// The profile that is on now (the list the game reads).
    pub current: bool,
}

/// One Workshop item that changed, or will.
#[derive(Debug, Clone, Serialize)]
pub struct UpdatedItem {
    pub workshop_id: String,
    pub title: String,
    pub mod_ids: Vec<String>,
    /// One of its mods is on in the list the game reads.
    pub active: bool,
    /// The profiles that have one of its mods on.
    pub profiles: Vec<ProfileOn>,
    /// Publish time of the version on disk, and of the newest Steam knows.
    pub installed_time: Option<u64>,
    pub latest_time: Option<u64>,
    /// Steam knows a newer version it has not downloaded yet.
    pub pending: bool,
    /// The version on disk was published after the last session.
    pub since_last_play: bool,
    pub map_mod: bool,
    /// The notes of the updates since the last session (and of the waiting one), newest first.
    pub notes: Vec<ChangeNote>,
    /// Why the notes are missing: "later" (not read yet, to spare Steam), "busy" (Steam is
    /// limiting requests), or what went wrong.
    pub notes_error: Option<String>,
    pub risks: Vec<Risk>,
    /// The entries of the mod's own changelog not read yet, newest first.
    pub changelog: Vec<crate::mod_changelog::LogEntry>,
    /// That file, from the mod's folder.
    pub changelog_file: Option<String>,
    /// The links its author put in it to support them.
    pub support: Vec<crate::mod_changelog::AuthorLink>,
    /// The newest entry was already shown in the game (its alert system).
    pub read_in_game: bool,
    /// A mod in Zomboid/mods, not from the Workshop: its changelog is what is new.
    pub local: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhatsNew {
    /// When the game was last played (unix seconds), if known.
    pub last_played: Option<u64>,
    /// From when updates are listed: the last session, or the time the player picked.
    pub since: Option<u64>,
    pub items: Vec<UpdatedItem>,
}

const NOTES_KEPT: usize = 10;
/// Pages read in one go: Steam starts refusing after a few dozen quick ones.
const PAGES_PER_READ: usize = 12;
const CACHE_FILE: &str = "PZModStudio_Changelogs.json";
/// Local mods: the newest changelog entry the player has seen (mod id -> its key).
const LOCAL_SEEN_FILE: &str = "PZModStudio_ChangelogSeen.json";
/// Entries shown when nothing says which were read.
const LOG_ENTRIES_SHOWN: usize = 3;

/// An entry's identity: its title, or the start of its text when it has none.
fn entry_key(e: &crate::mod_changelog::LogEntry) -> String {
    if e.title.trim().is_empty() { e.text.chars().take(60).collect() } else { e.title.trim().to_string() }
}

fn read_local_seen(z: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(z.join(LOCAL_SEEN_FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn write_local_seen(z: &Path, seen: &BTreeMap<String, String>) {
    if let Ok(text) = serde_json::to_string_pretty(seen) {
        let _ = crate::load_order::ini_parser::atomic_write(z.join(LOCAL_SEEN_FILE), text);
    }
}

/// "Mark as read" on a local mod: its newest changelog entry is the last one seen.
pub fn mark_changelog_read(user_zomboid_dir: &Path, mod_id: &str) -> Result<(), String> {
    let root = local_mod_roots(user_zomboid_dir).into_iter().find(|(id, _, _)| id == mod_id).map(|(_, _, r)| r).ok_or("That mod is not in Zomboid/mods.")?;
    let log = crate::mod_changelog::for_mod(&root).ok_or("That mod has no changelog.")?;
    let newest = log.entries.first().map(entry_key).ok_or("That changelog has no entries.")?;
    let mut seen = read_local_seen(user_zomboid_dir);
    seen.insert(mod_id.to_string(), newest);
    write_local_seen(user_zomboid_dir, &seen);
    Ok(())
}

/// A mod folder's id, from its mod.info (the Build 42 one first).
fn mod_id_of(root: &Path) -> Option<String> {
    let info = [root.join("42").join("mod.info"), root.join("mod.info")].into_iter().find(|p| p.is_file())?;
    let text = std::fs::read_to_string(info).ok()?;
    text.lines().find_map(|l| l.trim().strip_prefix("id=").map(|v| crate::load_order::mod_info::sanitize_mod_id(v.trim())))
}

/// The mods in `Zomboid/mods` (not the Studio's own): id, name, folder.
fn local_mod_roots(z: &Path) -> Vec<(String, String, std::path::PathBuf)> {
    let mut out = Vec::new();
    for m in std::fs::read_dir(z.join("mods")).into_iter().flatten().flatten() {
        let root = m.path();
        let folder = m.file_name().to_string_lossy().to_string();
        if !root.is_dir() || folder.starts_with("Z_PZModStudio_") || folder == crate::studio_link::LINK_MOD_ID {
            continue;
        }
        let info = [root.join("42").join("mod.info"), root.join("mod.info")].into_iter().find(|p| p.is_file());
        let Some(text) = info.and_then(|i| std::fs::read_to_string(i).ok()) else { continue };
        let field = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string()));
        let Some(id) = field("id=") else { continue };
        let name = field("name=").unwrap_or_else(|| id.clone());
        out.push((crate::load_order::mod_info::sanitize_mod_id(&id), name, root));
    }
    out
}

/// An item's own changelog: what is new in it, given what was read in the game.
fn attach_changelog(item: &mut UpdatedItem, roots: &[(String, std::path::PathBuf)], seen_in_game: &BTreeMap<String, String>) {
    for (id, root) in roots {
        let Some(log) = crate::mod_changelog::for_mod(root) else { continue };
        let game_seen = seen_in_game.get(id).map(String::as_str);
        item.read_in_game = log.entries.first().is_some_and(|e| game_seen == Some(e.title.trim()));
        item.changelog = crate::mod_changelog::newer_than(&log.entries, game_seen, LOG_ENTRIES_SHOWN).to_vec();
        item.changelog_file = Some(log.file);
        item.support = log.links;
        return;
    }
}

/// Words in change notes that say outright that a save in progress does not take the update.
const BREAKS: &[&str] = &[
    "new save", "fresh save", "fresh start", "start a new", "wipe", "not save compatible", "not save-compatible", "save incompatib",
    "break save", "breaks save", "broken save", "corrupt", "new game required", "new game needed", "new game is required",
    "requires a new game", "require a new game", "need a new game", "needs a new game",
];
/// Words that only say something was redone, taken out or changed its id: only words found in
/// the notes, worth reading before loading a save. Not "removed", "renamed" or "reset" alone:
/// a bug removed or a cooldown reset is most of what notes say.
const READ_FIRST: &[&str] = &[
    "new game", "incompatible", "changed id", "id change", "ids changed", "migrat", "removed item", "removed vehicle", "removed recipe",
    "removed trait", "removed profession", "renamed item", "rework", "reworked", "overhaul", "rewrite", "rewritten",
];
/// "No new game needed", "not incompatible", "without a new save": a negation a few words before
/// the match, in the same sentence, turns it around.
const NEGATIONS: &[&str] = &["no ", "not ", "n't ", "without ", "never ", "no need", "needn't", "doesn't", "don't", "isn't", "won't", "shouldn't"];

/// Is the match at `at` in `low` denied by a negation a few words earlier in the same sentence?
fn negated_before(low: &str, at: usize) -> bool {
    let start = low[..at].rfind(['.', '!', '?', '\n', ';', ':']).map(|i| i + 1).unwrap_or(0);
    let window = &low[start.max(at.saturating_sub(40))..at];
    NEGATIONS.iter().any(|n| window.contains(n))
}

fn decode(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ").replace("&amp;", "&")
}

/// The notes on a change notes page, newest first.
pub fn parse_changelog(html: &str) -> Vec<ChangeNote> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("<p id=\"") {
        rest = &rest[i + 7..];
        let Some(q) = rest.find('"') else { break };
        let at = rest[..q].parse::<u64>().ok();
        let Some(open_end) = rest.find('>') else { break };
        let body_start = open_end + 1;
        let Some(close) = rest[body_start..].find("</p>") else { break };
        let raw = &rest[body_start..body_start + close];
        rest = &rest[body_start + close..];
        let Some(at) = at else { continue };
        // <br> to new lines, every other tag out.
        let mut text = String::new();
        let mut in_tag = false;
        let mut tag = String::new();
        for c in raw.chars() {
            if in_tag {
                if c == '>' {
                    in_tag = false;
                    let t = tag.to_ascii_lowercase();
                    if t.starts_with("br") || t.starts_with("/li") || t.starts_with("li") {
                        text.push('\n');
                    }
                    tag.clear();
                } else {
                    tag.push(c);
                }
            } else if c == '<' {
                in_tag = true;
            } else {
                text.push(c);
            }
        }
        let text = decode(&text).lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim().to_string();
        out.push(ChangeNote { at, text });
    }
    out.sort_by(|a, b| b.at.cmp(&a.at));
    out
}

/// The words of `list` in these notes, each once.
fn words_in(notes: &[ChangeNote], list: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for note in notes {
        let low = note.text.to_lowercase();
        for w in list {
            if found.iter().any(|f: &String| f == w) {
                continue;
            }
            // Every place the words are, until one that no negation denies.
            let mut from = 0;
            while let Some(i) = low[from..].find(w) {
                let at = from + i;
                if !negated_before(&low, at) {
                    found.push(w.to_string());
                    break;
                }
                from = at + w.len();
            }
        }
    }
    found
}

/// The words in these notes that say a save may not take the update.
pub fn risky_words(notes: &[ChangeNote]) -> Vec<String> {
    words_in(notes, BREAKS)
}

/// The words that only say something was taken out or redone (none of them when a stronger
/// one is there already).
pub fn read_first_words(notes: &[ChangeNote]) -> Vec<String> {
    let strong = risky_words(notes);
    words_in(notes, READ_FIRST).into_iter().filter(|w| !strong.iter().any(|s| s.contains(w.as_str()))).collect()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CachedNotes {
    /// The newest version Steam knew when the page was read.
    fetched_for: u64,
    notes: Vec<ChangeNote>,
}

fn read_cache(z: &Path) -> BTreeMap<String, CachedNotes> {
    std::fs::read_to_string(z.join(CACHE_FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn write_cache(z: &Path, cache: &BTreeMap<String, CachedNotes>) {
    let path = z.join(CACHE_FILE);
    if crate::load_order::ini_parser::guard_overwrite::<BTreeMap<String, CachedNotes>>(&path).is_err() {
        return;
    }
    if let Ok(text) = serde_json::to_string(cache) {
        let _ = crate::load_order::ini_parser::atomic_write(&path, text);
    }
}

/// The mods of a Workshop item folder: id, name, and whether it adds map cells.
fn item_mods(item_dir: &Path) -> Vec<(String, String, bool)> {
    let mut out = Vec::new();
    for m in std::fs::read_dir(item_dir.join("mods")).into_iter().flatten().flatten() {
        let root = m.path();
        let info = [root.join("42").join("mod.info"), root.join("mod.info")].into_iter().find(|p| p.is_file());
        let Some(info) = info else { continue };
        let Ok(text) = std::fs::read_to_string(&info) else { continue };
        let field = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string()));
        let Some(id) = field("id=") else { continue };
        let name = field("name=").unwrap_or_else(|| id.clone());
        let has_maps = |d: &Path| std::fs::read_dir(d.join("media").join("maps")).map(|mut r| r.next().is_some()).unwrap_or(false);
        let map = has_maps(&root.join("42")) || has_maps(&root.join("common")) || has_maps(&root);
        out.push((crate::load_order::mod_info::sanitize_mod_id(&id), name, map));
    }
    out
}

/// When the game was last played: the latest recorded session, or Steam's last run of the game.
pub fn last_played(user_zomboid_dir: &Path, steam_last_ran: Option<u64>) -> Option<u64> {
    let session = crate::studio_link::save_references(user_zomboid_dir).values().map(|r| r.at).max();
    match (session, steam_last_ran) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

/// Everything updated since the last session, or waiting in Steam. With `with_notes`, each
/// item's change notes are read too (cached; Steam is asked only for items it has a newer
/// version of than the cache). Without, nothing leaves the PC.
pub fn whats_new(user_zomboid_dir: &Path, workshop_dir: &str, with_notes: bool, from: Option<u64>) -> WhatsNew {
    let state = crate::workshop::local_state(workshop_dir);
    let last = last_played(user_zomboid_dir, state.as_ref().and_then(|s| s.time_last_app_ran));
    let since = from.or(last);
    let Some(state) = state else { return WhatsNew { last_played: last, since, items: Vec::new() } };
    let active: Vec<String> = crate::load_order::ini_parser::try_parse_default_txt(&user_zomboid_dir.join("mods").join("default.txt")).unwrap_or_default();
    let plain = |id: &str| crate::load_order::mod_info::sanitize_mod_id(id).rsplit('/').next().unwrap_or_default().to_string();
    // Each profile: name, is it the one on now, the plain ids it has on.
    let profiles: Vec<(String, bool, Vec<String>)> = crate::instance_manager::list_instances(user_zomboid_dir.to_string_lossy().to_string())
        .unwrap_or_default()
        .into_iter()
        .map(|p| (p.name, p.is_active, p.active_mod_ids.iter().map(|m| plain(m)).collect()))
        .collect();
    let profiles_with = |ids: &[String]| -> Vec<ProfileOn> {
        let ids: Vec<String> = ids.iter().map(|i| plain(i)).collect();
        profiles.iter().filter(|(_, _, on)| ids.iter().any(|i| on.contains(i))).map(|(n, current, _)| ProfileOn { name: n.clone(), current: *current }).collect()
    };

    let mut items: Vec<UpdatedItem> = Vec::new();
    for (wid, s) in &state.items {
        let since_last_play = matches!((s.installed_time, since), (Some(i), Some(p)) if i > p);
        if !since_last_play && !s.update_pending {
            continue;
        }
        let mods = item_mods(&Path::new(workshop_dir).join(wid));
        if mods.is_empty() {
            continue;
        }
        items.push(UpdatedItem {
            workshop_id: wid.clone(),
            title: mods[0].1.clone(),
            mod_ids: mods.iter().map(|m| m.0.clone()).collect(),
            active: mods.iter().any(|m| active.contains(&m.0)),
            profiles: profiles_with(&mods.iter().map(|m| m.0.clone()).collect::<Vec<_>>()),
            installed_time: s.installed_time,
            latest_time: s.latest_time,
            pending: s.update_pending,
            since_last_play,
            map_mod: mods.iter().any(|m| m.2),
            notes: Vec::new(),
            notes_error: None,
            risks: Vec::new(),
            changelog: Vec::new(),
            changelog_file: None,
            support: Vec::new(),
            read_in_game: false,
            local: false,
        });
    }

    // Each item's own changelog, and what was read of it in the game.
    let seen_in_game = crate::mod_changelog::seen_in_game(user_zomboid_dir);
    for item in items.iter_mut() {
        let item_dir = Path::new(workshop_dir).join(&item.workshop_id).join("mods");
        let roots: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(&item_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|m| mod_id_of(&m.path()).map(|id| (id, m.path())))
            .collect();
        attach_changelog(item, &roots, &seen_in_game);
    }

    // Mods in Zomboid/mods that are on: their changelog's new entries.
    let mut local_seen = read_local_seen(user_zomboid_dir);
    let mut learned = false;
    for (id, name, root) in local_mod_roots(user_zomboid_dir) {
        if !active.contains(&id) {
            continue;
        }
        let Some(log) = crate::mod_changelog::for_mod(&root) else { continue };
        let Some(newest) = log.entries.first().map(entry_key) else { continue };
        let game_seen = seen_in_game.get(&id).cloned();
        let Some(recorded) = local_seen.get(&id).cloned() else {
            // The first time: what is there now is what the player has.
            local_seen.insert(id.clone(), newest);
            learned = true;
            continue;
        };
        if recorded == newest || game_seen.as_deref() == Some(newest.as_str()) {
            continue;
        }
        let cut = log.entries.iter().position(|e| entry_key(e) == recorded).unwrap_or(LOG_ENTRIES_SHOWN.min(log.entries.len()));
        items.push(UpdatedItem {
            workshop_id: format!("local:{}", id),
            title: name,
            profiles: profiles_with(std::slice::from_ref(&id)),
            mod_ids: vec![id],
            active: true,
            installed_time: None,
            latest_time: None,
            pending: false,
            since_last_play: true,
            map_mod: false,
            notes: Vec::new(),
            notes_error: None,
            risks: Vec::new(),
            changelog: log.entries[..cut].to_vec(),
            changelog_file: Some(log.file),
            support: log.links,
            read_in_game: false,
            local: true,
        });
    }
    if learned {
        write_local_seen(user_zomboid_dir, &local_seen);
    }

    // Mods that are on first, then the newest.
    items.sort_by(|a, b| b.active.cmp(&a.active).then(b.latest_time.cmp(&a.latest_time)));

    if with_notes {
        let mut cache = read_cache(user_zomboid_dir);
        let mut asked = 0;
        let mut refused = false;
        for item in items.iter_mut() {
            let newest = item.latest_time.or(item.installed_time).unwrap_or(0);
            let cached = cache.get(&item.workshop_id).filter(|c| c.fetched_for >= newest).cloned();
            let notes = match cached {
                Some(c) => Ok(c.notes),
                // Once Steam says "slow down", the rest wait for another read.
                None if refused => Err("busy".to_string()),
                None if asked >= PAGES_PER_READ => Err("later".to_string()),
                None => {
                    // Spaced out by the shared limiter (polite).
                    asked += 1;
                    let url = format!("https://steamcommunity.com/sharedfiles/filedetails/changelog/{}", item.workshop_id);
                    let read = crate::workshop::fetch_page(&url).map(|html| parse_changelog(&html)).inspect(|notes| {
                        cache.insert(item.workshop_id.clone(), CachedNotes { fetched_for: newest, notes: notes.clone() });
                    });
                    if read.as_ref().is_err_and(|e| e.contains("429")) {
                        refused = true;
                        Err("busy".to_string())
                    } else {
                        read
                    }
                }
            };
            match notes {
                Ok(all) => {
                    // The updates after the last session; for a waiting one, after the version on disk.
                    let from = if item.pending { item.installed_time.unwrap_or(0).min(since.unwrap_or(u64::MAX)) } else { since.unwrap_or(0) };
                    item.notes = all.into_iter().filter(|n| n.at > from).take(NOTES_KEPT).collect();
                }
                Err(e) => item.notes_error = Some(e),
            }
        }
        write_cache(user_zomboid_dir, &cache);
    }

    for item in items.iter_mut() {
        if item.map_mod {
            item.risks.push(Risk { code: "map".into(), level: "breaks".into(), detail: String::new() });
        }
        // The mod's own changelog says it too.
        let own: Vec<ChangeNote> = item.changelog.iter().map(|e| ChangeNote { at: 0, text: format!("{}\n{}", e.title, e.text) }).collect();
        let all_notes: Vec<ChangeNote> = item.notes.iter().cloned().chain(own).collect();
        let strong = risky_words(&all_notes);
        if !strong.is_empty() {
            item.risks.push(Risk { code: "notes".into(), level: "breaks".into(), detail: strong.join(", ") });
        }
        let weak = read_first_words(&all_notes);
        if !weak.is_empty() {
            item.risks.push(Risk { code: "notes".into(), level: "read".into(), detail: weak.join(", ") });
        }
    }
    WhatsNew { last_played: last, since, items }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_notes_are_read_with_their_dates_and_lines() {
        let html = r#"<div class="detailBox workshopAnnouncement noFooter changeLogCtn">
            <div class="changelog headline"> Update: 21 Sep @ 5:15am </div>
            <p id="1789992909">Version 1.0.9 &mdash; Cleanup<br><br>&middot; Removed leftover debug output.<br>&quot;Grid&quot; &amp; logs</p>
            </div><p id="1789654852">Version 1.0.9 (beta2)<ul><li>Added icons</li></ul></p><p id="x">not a note</p>"#;
        let notes = parse_changelog(html);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].at, 1789992909);
        assert!(notes[0].text.starts_with("Version 1.0.9"));
        assert!(notes[0].text.contains("\n\n"));
        assert!(notes[0].text.ends_with("\"Grid\" & logs"));
        assert!(notes[1].text.contains("Added icons"));
        assert!(risky_words(&notes).is_empty(), "debug output taken out breaks nothing");
        assert!(read_first_words(&notes).is_empty(), "removed alone says nothing about a save");
    }

    #[test]
    fn risky_words_are_found_once_and_harmless_notes_have_none() {
        let n = |t: &str| ChangeNote { at: 1, text: t.into() };
        assert!(risky_words(&[n("Fixed a typo in the tooltip.")]).is_empty());
        assert_eq!(
            risky_words(&[n("Big REWORK: start a new save."), n("Rework again, NOT save compatible")]),
            ["new save", "start a new", "not save compatible"].map(String::from).to_vec()
        );
        assert_eq!(read_first_words(&[n("Removed items: Axe. Rework of crafting.")]), ["removed item", "rework"].map(String::from).to_vec());
        assert!(risky_words(&[n("Removed items: Axe.")]).is_empty(), "an item taken out is worth reading, not a broken save");
        // A negation in the same sentence turns the words around.
        assert!(read_first_words(&[n("No new game needed."), n("This is not incompatible with 42.")]).is_empty());
        assert!(risky_words(&[n("Works on existing saves, no new save required.")]).is_empty());
        assert!(risky_words(&[n("Fully save compatible!")]).is_empty());
        assert_eq!(risky_words(&[n("Not save compatible.")]), vec!["not save compatible".to_string()]);
        assert_eq!(read_first_words(&[n("Removed. New game recommended for the new town.")]), vec!["new game".to_string()], "the sentence before does not deny it");
        assert_eq!(risky_words(&[n("Requires a new game.")]), vec!["requires a new game".to_string()]);
    }

    /// Live, on a copy of the list and sessions: `PZMS_REAL_ZOMBOID=<Zomboid> PZMS_REAL_WORKSHOP=<content/108600>
    /// cargo test -- --ignored live_whats_new --nocapture`. Reads Steam for the notes.
    #[test]
    #[ignore]
    fn live_whats_new() {
        let (Ok(real), Ok(workshop)) = (std::env::var("PZMS_REAL_ZOMBOID"), std::env::var("PZMS_REAL_WORKSHOP")) else { return };
        let z = std::env::temp_dir().join(format!("pzms-whatsnew-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&z);
        std::fs::create_dir_all(z.join("mods")).unwrap();
        let real = std::path::PathBuf::from(real);
        let _ = std::fs::copy(real.join("mods").join("default.txt"), z.join("mods").join("default.txt"));
        let _ = std::fs::copy(real.join("PZModStudio_SaveRefs.json"), z.join("PZModStudio_SaveRefs.json"));
        let started = std::time::Instant::now();
        let week = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() - 7 * 86400;
        let w = whats_new(&z, &workshop, true, Some(week));
        println!("since {:?}; {} items in {:?}", w.since, w.items.len(), started.elapsed());
        for i in w.items.iter().take(15) {
            println!("{} {:?} active={} pending={} new={} map={} notes={} err={:?} risks={:?}", i.workshop_id, i.title, i.active, i.pending, i.since_last_play, i.map_mod, i.notes.len(), i.notes_error, i.risks);
        }
        if let Some(n) = w.items.iter().find_map(|i| i.notes.first()) {
            println!("first note: {}", n.text.chars().take(200).collect::<String>());
        }
        let cache_path = z.join(CACHE_FILE);
        println!("cache file: {:?} parse: {:?}", std::fs::metadata(&cache_path).map(|m| m.len()), std::fs::read_to_string(&cache_path).map(|t| serde_json::from_str::<BTreeMap<String, CachedNotes>>(&t).map(|m| m.len()).map_err(|e| e.to_string())));
        let again = std::time::Instant::now();
        let _ = whats_new(&z, &workshop, true, Some(week));
        println!("second read (cached) in {:?}", again.elapsed());
        let _ = std::fs::remove_dir_all(&z);
    }

    #[test]
    fn last_played_is_the_later_of_the_sessions_and_steam() {
        let z = std::env::temp_dir().join(format!("pzms-whatsnew-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&z);
        std::fs::create_dir_all(&z).unwrap();
        assert_eq!(last_played(&z, Some(50)), Some(50));
        std::fs::write(z.join("PZModStudio_SaveRefs.json"), r#"{"Sandbox/A":{"mods":[],"at":100},"Sandbox/B":{"mods":[],"at":70}}"#).unwrap();
        assert_eq!(last_played(&z, Some(50)), Some(100));
        assert_eq!(last_played(&z, Some(150)), Some(150));
        assert_eq!(last_played(&z, None), Some(100));
        let _ = std::fs::remove_dir_all(&z);
    }
}

#[cfg(test)]
mod real {
    /// Read-only, no network: PZMS_ZOMBOID_DIR=... PZMS_WORKSHOP_DIR=... cargo test real_whats_new_logs -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_whats_new_logs() {
        let (Ok(z), Ok(w)) = (std::env::var("PZMS_ZOMBOID_DIR"), std::env::var("PZMS_WORKSHOP_DIR")) else { return };
        let month_ago = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() - 30 * 86400;
        let news = super::whats_new(std::path::Path::new(&z), &w, false, Some(month_ago));
        for i in news.items.iter().filter(|i| i.changelog_file.is_some()) {
            println!("{:<34} new entries {:>2} {:<28} read in game {:<5} links {} risks {:?}", i.title, i.changelog.len(), i.changelog.first().map(|e| e.title.clone()).unwrap_or_default(), i.read_in_game, i.support.len(), i.risks.iter().map(|r| format!("{}:{}", r.level, r.detail)).collect::<Vec<_>>());
        }
        println!("{} items, {} with a changelog", news.items.len(), news.items.iter().filter(|i| i.changelog_file.is_some()).count());
    }
}
