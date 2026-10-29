//! The changelog a mod ships in its own folder, read from disk: what changed in each version
//! (offline, and for mods that are not on the Workshop too), and the links its author put in it
//! to support them.
//!
//! Three shapes are read:
//! - the in-game alert system's: `[ <title, usually a date> ]`, its lines, `[ ------ ]`, oldest
//!   first; a `[ ALERT_CONFIG ]` block holds `linkN = <name> = <url>,` (8 of the mods installed
//!   here use it). Only the file format is read; nothing of that mod's code is used.
//! - Markdown: `## <version or date>` headings, newest first.
//! - Anything else: the first lines, as one entry.
//!
//! What the player already read in the game (that system keeps `<mod id> = <newest title seen>`
//! in `Zomboid/Lua/chuckleberryFinn_moddingAlerts.txt`) is read, never written, so the Studio
//! does not show it as new again.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    /// The entry's heading: a date, a version, or both.
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorLink {
    pub name: String,
    /// Always https; Steam's link filter taken off, so it opens straight in the browser.
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModChangelog {
    /// The file, from the mod's folder.
    pub file: String,
    /// Newest first.
    pub entries: Vec<LogEntry>,
    pub links: Vec<AuthorLink>,
}

const MAX_ENTRIES: usize = 30;
const PLAIN_LINES: usize = 40;
/// The alert system's own file of what was read in the game.
pub const SEEN_IN_GAME_FILE: &str = "chuckleberryFinn_moddingAlerts.txt";

/// `https://steamcommunity.com/linkfilter/?u=<url>` -> `<url>`; anything not https is dropped.
pub fn clean_url(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches(',').trim();
    let url = match url.split_once("linkfilter/?u=").or_else(|| url.split_once("linkfilter/?url=")) {
        Some((_, inner)) => inner.to_string(),
        None => url.to_string(),
    };
    let url = url.replace("%3A", ":").replace("%2F", "/").replace("%3a", ":").replace("%2f", "/");
    (url.starts_with("https://") && !url.contains(char::is_whitespace) && url.len() > 10).then_some(url)
}

fn bracket_title(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[').and_then(|r| r.strip_suffix(']')).map(str::trim)
}

/// A changelog's text, in whichever shape it is written.
pub fn parse(text: &str) -> ModChangelog {
    let text = text.trim_start_matches('\u{feff}').replace('\r', "");
    let mut out = ModChangelog::default();
    if text.lines().any(|l| bracket_title(l).is_some_and(|t| t.chars().all(|c| c == '-') && t.len() >= 3)) {
        // [ title ] ... [ ------ ]
        let mut current: Option<(String, Vec<String>)> = None;
        let mut entries = Vec::new();
        for line in text.lines() {
            match bracket_title(line) {
                Some(t) if t.chars().all(|c| c == '-') && !t.is_empty() => {
                    if let Some((title, body)) = current.take() {
                        if title.eq_ignore_ascii_case("ALERT_CONFIG") {
                            for l in &body {
                                // linkN = name = url,
                                let mut parts = l.splitn(3, '=');
                                let (Some(key), Some(name), Some(url)) = (parts.next(), parts.next(), parts.next()) else { continue };
                                if !key.trim().to_ascii_lowercase().starts_with("link") {
                                    continue;
                                }
                                if let Some(url) = clean_url(url) {
                                    out.links.push(AuthorLink { name: name.trim().to_string(), url });
                                }
                            }
                        } else {
                            entries.push(LogEntry { title, text: body.join("\n").trim().to_string() });
                        }
                    }
                }
                Some(t) if current.is_none() => current = Some((t.to_string(), Vec::new())),
                _ => {
                    if let Some((_, body)) = current.as_mut() {
                        body.push(line.to_string());
                    }
                }
            }
        }
        entries.reverse();
        out.entries = entries;
    } else if text.lines().any(|l| l.starts_with("## ")) {
        let mut current: Option<(String, Vec<String>)> = None;
        for line in text.lines() {
            if let Some(h) = line.strip_prefix("## ") {
                if let Some((title, body)) = current.take() {
                    out.entries.push(LogEntry { title, text: body.join("\n").trim().to_string() });
                }
                current = Some((h.trim().to_string(), Vec::new()));
            } else if let Some((_, body)) = current.as_mut() {
                body.push(line.to_string());
            }
        }
        if let Some((title, body)) = current {
            out.entries.push(LogEntry { title, text: body.join("\n").trim().to_string() });
        }
    } else {
        let body: Vec<&str> = text.lines().take(PLAIN_LINES).collect();
        let body = body.join("\n").trim().to_string();
        if !body.is_empty() {
            out.entries.push(LogEntry { title: String::new(), text: body });
        }
    }
    out.entries.retain(|e| !e.title.is_empty() || !e.text.is_empty());
    out.entries.truncate(MAX_ENTRIES);
    out
}

fn is_changelog_name(name: &str) -> bool {
    let low = name.to_ascii_lowercase();
    ["changelog.txt", "changelog.md", "changelog.bbcode", "changelog"].contains(&low.as_str())
}

/// The changelog file of a mod folder (`.../mods/<Mod>`): beside its mod.info, in `common/`,
/// `media/`, `docs/`, a version folder, or at the Workshop item's top. A `.txt` wins over a
/// `.md`, which wins over a `.bbcode` (the alert system's own order).
pub fn find(mod_root: &Path) -> Option<PathBuf> {
    let mut dirs = vec![mod_root.to_path_buf(), mod_root.join("common"), mod_root.join("media"), mod_root.join("docs"), mod_root.join("42")];
    if let Ok(read) = std::fs::read_dir(mod_root) {
        for e in read.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if e.path().is_dir() && n.starts_with("42.") {
                dirs.push(e.path().join("media"));
                dirs.push(e.path());
            }
        }
    }
    dirs.push(mod_root.join("42").join("media"));
    // The Workshop item's own folder (`<id>/mods/<Mod>` -> `<id>`).
    if let Some(item) = mod_root.parent().filter(|p| p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("mods"))).and_then(|p| p.parent()) {
        dirs.push(item.to_path_buf());
    }
    let rank = |p: &Path| match p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("txt") => 0,
        Some("md") => 1,
        Some("bbcode") => 2,
        _ => 3,
    };
    for dir in dirs {
        let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.file_name().is_some_and(|n| is_changelog_name(&n.to_string_lossy())))
            .collect();
        found.sort_by_key(|p| rank(p));
        if let Some(p) = found.into_iter().next() {
            return Some(p);
        }
    }
    None
}

/// A mod folder's changelog, read and parsed.
pub fn for_mod(mod_root: &Path) -> Option<ModChangelog> {
    let path = find(mod_root)?;
    let bytes = std::fs::read(&path).ok()?;
    let mut log = parse(&String::from_utf8_lossy(&bytes));
    log.file = path.strip_prefix(mod_root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
    (!log.entries.is_empty() || !log.links.is_empty()).then_some(log)
}

/// What the player read in the game: mod id -> the newest entry title the alert system showed.
pub fn seen_in_game(user_zomboid_dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(user_zomboid_dir.join("Lua").join(SEEN_IN_GAME_FILE)) else { return out };
    for line in text.lines() {
        if let Some((id, title)) = line.split_once('=') {
            let (id, title) = (id.trim(), title.trim());
            if !id.is_empty() && !title.is_empty() {
                out.insert(id.to_string(), title.to_string());
            }
        }
    }
    out
}

/// The entries newer than `seen` (newest first); all of them (at most `limit`) when it is not
/// one of them.
pub fn newer_than<'a>(entries: &'a [LogEntry], seen: Option<&str>, limit: usize) -> &'a [LogEntry] {
    let seen = seen.map(str::trim);
    let cut = seen.and_then(|s| entries.iter().position(|e| e.title.trim() == s)).unwrap_or(limit.min(entries.len()));
    &entries[..cut.min(entries.len())]
}

/// Sites where people support a mod's author, and the name a link to one is shown with. A host
/// with a path wants that path to start the way written (a PayPal page is a donation page only
/// under these), and every link needs something after the host: the author's own page.
const SUPPORT_SITES: &[(&str, &str, &str)] = &[
    ("ko-fi.com", "", "Ko-fi"),
    ("patreon.com", "", "Patreon"),
    ("buymeacoffee.com", "", "Buy Me a Coffee"),
    ("paypal.me", "", "PayPal"),
    ("paypal.com", "/donate", "PayPal"),
    ("paypal.com", "/paypalme", "PayPal"),
    ("github.com", "/sponsors", "GitHub Sponsors"),
    ("liberapay.com", "", "Liberapay"),
    ("boosty.to", "", "Boosty"),
    ("opencollective.com", "", "Open Collective"),
];
const MAX_SUPPORT_LINKS: usize = 4;

/// The support links in a mod's Workshop description (Steam BBCode or plain text): Ko-fi, Patreon
/// and the like, written as `[url=...]`, as bare links or without `https://`. Only those sites,
/// only https, Steam's link filter taken off; each address once.
pub fn support_links(text: &str) -> Vec<AuthorLink> {
    let mut out: Vec<AuthorLink> = Vec::new();
    for raw in text.split(|c: char| c.is_whitespace() || "[]\"'<>(),|".contains(c)) {
        let token = raw.strip_prefix("url=").unwrap_or(raw);
        let token = match token.split_once("linkfilter/?u=").or_else(|| token.split_once("linkfilter/?url=")) {
            Some((_, inner)) => inner.replace("%3A", ":").replace("%2F", "/").replace("%3a", ":").replace("%2f", "/"),
            None => token.to_string(),
        };
        let bare = token.strip_prefix("https://").or_else(|| token.strip_prefix("http://")).unwrap_or(&token);
        let bare = bare.strip_prefix("www.").unwrap_or(bare).trim_end_matches(['.', ';', ':', '!', '?', '/']);
        let (host, path) = bare.split_once('/').map(|(h, p)| (h, format!("/{}", p))).unwrap_or((bare, String::new()));
        let host = host.to_ascii_lowercase();
        let Some((_, _, name)) = SUPPORT_SITES.iter().find(|(h, prefix, _)| host == *h && path.to_ascii_lowercase().starts_with(prefix)) else { continue };
        if path.trim_matches('/').is_empty() || path.contains(char::is_whitespace) {
            continue;
        }
        let url = format!("https://{}{}", host, path);
        if !out.iter().any(|l| l.url.eq_ignore_ascii_case(&url)) {
            out.push(AuthorLink { name: name.to_string(), url });
        }
        if out.len() == MAX_SUPPORT_LINKS {
            break;
        }
    }
    out
}

/// The changelog's links and the description's, the changelog's first, each address once.
pub fn merge_links(first: Vec<AuthorLink>, more: &[AuthorLink]) -> Vec<AuthorLink> {
    let mut out = first;
    for l in more {
        if !out.iter().any(|o| o.url.eq_ignore_ascii_case(&l.url)) {
            out.push(l.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_links_in_a_workshop_description() {
        let text = "[h2]Support me[/h2]\r\n[url=https://steamcommunity.com/linkfilter/?u=https%3A%2F%2Fko-fi.com%2Fsomeone]Ko-fi[/url] | patreon.com/someone.\nAlso https://www.buymeacoffee.com/someone and https://ko-fi.com/someone again.\n\
            https://github.com/sponsors/dev https://github.com/dev/repo https://paypal.com/donate/?hosted_button_id=X https://paypal.com/us/home https://patreon.com/ ko-fi.com";
        let links = support_links(text);
        assert_eq!(links.iter().map(|l| (l.name.as_str(), l.url.as_str())).collect::<Vec<_>>(), vec![
            ("Ko-fi", "https://ko-fi.com/someone"),
            ("Patreon", "https://patreon.com/someone"),
            ("Buy Me a Coffee", "https://buymeacoffee.com/someone"),
            ("GitHub Sponsors", "https://github.com/sponsors/dev"),
        ], "the link filter off, no scheme or www needed, each once, a repo or a page with no author is not a support link, at most four");
        let paypal = support_links("[url=https://www.paypal.com/donate/?hosted_button_id=ABC]donate[/url] http://paypal.me/someone");
        assert_eq!(paypal.iter().map(|l| l.url.as_str()).collect::<Vec<_>>(), vec!["https://paypal.com/donate/?hosted_button_id=ABC", "https://paypal.me/someone"]);
        assert!(support_links("[url=https://discord.gg/abc]Discord[/url] https://steamcommunity.com/id/x").is_empty());
        let merged = merge_links(vec![AuthorLink { name: "Chuck's Kofi".into(), url: "https://ko-fi.com/someone".into() }], &links);
        assert_eq!(merged.len(), 4, "the changelog's Ko-fi is the description's Ko-fi");
        assert_eq!(merged[0].name, "Chuck's Kofi");
    }

    #[test]
    fn the_alert_format_with_its_links() {
        let text = "[ 12/20/24 ]\n- Initial changelog.\n[ ------ ]\n\n[ ALERT_CONFIG ]\nlink1 = Chuck's Kofi = https://steamcommunity.com/linkfilter/?u=https://ko-fi.com/chuckleberryfinn,\nlink2 = Workshop = https://steamcommunity.com/id/x/myworkshopfiles/?appid=108600,\nlink3 = Bad = javascript:alert(1),\n[ ------ ]\n\n[ 1/25/25 ]\n- Fix first-startup edge-case.\n- And another.\n[ ------ ]\n";
        let log = parse(text);
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.entries[0], LogEntry { title: "1/25/25".into(), text: "- Fix first-startup edge-case.\n- And another.".into() }, "newest first");
        assert_eq!(log.links, vec![
            AuthorLink { name: "Chuck's Kofi".into(), url: "https://ko-fi.com/chuckleberryfinn".into() },
            AuthorLink { name: "Workshop".into(), url: "https://steamcommunity.com/id/x/myworkshopfiles/?appid=108600".into() },
        ]);
    }

    #[test]
    fn markdown_and_plain_changelogs() {
        let md = parse("# Changelog\n\n## 1.9.1 — 2026-10-15\n- Closed source.\n\n## 1.9.0\n- Weapon rest.\n");
        assert_eq!(md.entries.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), vec!["1.9.1 — 2026-10-15", "1.9.0"]);
        assert_eq!(md.entries[0].text, "- Closed source.");
        let plain = parse("v2: fixed things\nv1: first\n");
        assert_eq!(plain.entries.len(), 1);
        assert!(plain.entries[0].text.starts_with("v2: fixed things"));
    }

    #[test]
    fn only_what_is_newer_than_what_was_read() {
        let e = |t: &str| LogEntry { title: t.into(), text: String::new() };
        let entries = vec![e("9/25/26 v1.4.2"), e("8/23/26"), e("8/20/26 v1.3")];
        assert_eq!(newer_than(&entries, Some("8/23/26"), 5).len(), 1);
        assert_eq!(newer_than(&entries, Some("9/25/26 v1.4.2"), 5).len(), 0, "read in the game: nothing new");
        assert_eq!(newer_than(&entries, None, 2).len(), 2);
        assert_eq!(newer_than(&entries, Some("unknown"), 5).len(), 3);
    }

    #[test]
    fn the_file_is_found_where_mods_put_it() {
        let root = std::env::temp_dir().join(format!("pzms-changelog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let item = root.join("123").join("mods").join("Fence");
        std::fs::create_dir_all(item.join("common")).unwrap();
        std::fs::write(item.join("common").join("ChangeLog.md"), "## 1\n- md\n").unwrap();
        std::fs::write(item.join("common").join("ChangeLog.txt"), "[ 1/1/26 ]\n- txt\n[ ------ ]\n").unwrap();
        let log = for_mod(&item).unwrap();
        assert_eq!(log.file, "common/ChangeLog.txt", "a .txt wins");
        assert_eq!(log.entries[0].text, "- txt");
        let other = root.join("456").join("mods").join("Tactical");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(root.join("456").join("CHANGELOG.txt"), "first line\n").unwrap();
        assert!(for_mod(&other).is_some(), "at the Workshop item's top");
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod real {
    /// Read-only, over the real Workshop folder: PZMS_WORKSHOP_DIR=... cargo test real_changelogs -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_changelogs() {
        let Ok(w) = std::env::var("PZMS_WORKSHOP_DIR") else { return };
        for item in std::fs::read_dir(&w).unwrap().flatten() {
            for m in std::fs::read_dir(item.path().join("mods")).into_iter().flatten().flatten() {
                if let Some(log) = super::for_mod(&m.path()) {
                    let first = log.entries.first().map(|e| e.title.clone()).unwrap_or_default();
                    println!("{:<34} {:<28} {:>3} entries, newest {:<24} {} link(s)", m.file_name().to_string_lossy(), log.file, log.entries.len(), first, log.links.len());
                }
            }
        }
    }
}
