//! What people say about a Workshop item: its public comment thread.
//!
//! Steam renders an item's comments as JSON at
//! `steamcommunity.com/comment/PublishedFile_Public/render/{creator SteamID64}/{item id}/?start=&count=`
//! (`success`, `total_count`, `comments_html`, newest first; read on 2026-11-04). The thread is
//! filed under the author, so the author's id comes from GetPublishedFileDetails first (the same
//! request the detail page makes, answered from the cache). An author whose profile is private
//! has no public thread: Steam answers `success: false` with "This profile is private.".

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const RENDER_URL: &str = "https://steamcommunity.com/comment/PublishedFile_Public/render";
/// New comments arrive; a few minutes is fresh enough to read and keeps Steam asked rarely.
const COMMENTS_TTL: Duration = Duration::from_secs(5 * 60);
/// The most comments asked for at once.
pub const COMMENTS_MAX: u32 = 50;
/// SteamID64 = this + the 32-bit account id Steam puts in `data-miniprofile`.
const STEAM_ID64_BASE: u64 = 76561197960265728;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkshopComment {
    /// Steam's id of the comment.
    pub id: String,
    pub author: String,
    /// The commenter's Steam profile page.
    pub author_profile: Option<String>,
    /// When it was posted, unix seconds.
    pub when: Option<u64>,
    /// When it was posted, as Steam words it ("21 October, 2026 @ 1:06:59 pm PST").
    pub when_text: String,
    /// Plain text, line breaks kept.
    pub text: String,
    /// Written by the item's author.
    pub by_author: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkshopComments {
    pub workshop_id: String,
    /// Comments the item has in all.
    pub total: u64,
    /// Where this page starts (0 = the newest comment).
    pub start: u32,
    /// Newest first.
    pub comments: Vec<WorkshopComment>,
    /// Why Steam shows no comments (its own words, e.g. "This profile is private."); None when
    /// the thread was read.
    pub unavailable: Option<String>,
}

/// `count` comments of a Workshop item from `start` (0 = newest), up to [`COMMENTS_MAX`].
pub fn comments(id: &str, start: u32, count: u32) -> Result<WorkshopComments, String> {
    let item = super::get_details(&[id.to_string()])?.into_iter().next().ok_or("Unexpected answer from Steam (no item)")?;
    if !item.found {
        return Err("Steam has no public item with this id (removed, private or wrong id).".to_string());
    }
    let Some(creator) = item.creator else {
        return Err("Steam did not say who made this item, so its comments cannot be found.".to_string());
    };
    let count = count.clamp(1, COMMENTS_MAX);
    let url = format!("{}/{}/{}/?start={}&count={}", RENDER_URL, creator, id, start, count);
    let curl = || super::page_through_system_curl(&url);
    let body = super::polite::fetch(super::polite::Request::Get { url: &url, query: &[] }, COMMENTS_TTL, Some(&curl)).map_err(|r| r.say())?;
    parse_render(id, &creator, start, &body)
}

/// Reads Steam's render answer.
pub fn parse_render(id: &str, creator: &str, start: u32, body: &str) -> Result<WorkshopComments, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("Unexpected answer from Steam: {}", e))?;
    let mut out = WorkshopComments { workshop_id: id.to_string(), total: 0, start, comments: Vec::new(), unavailable: None };
    if v.get("success").and_then(|s| s.as_bool()) != Some(true) {
        out.unavailable = Some(v.get("error").and_then(|e| e.as_str()).unwrap_or("Steam did not send the comments.").to_string());
        return Ok(out);
    }
    let num = |key: &str| v.get(key).and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| s.parse().ok())));
    out.total = num("total_count").unwrap_or(0);
    out.start = num("start").map(|s| s as u32).unwrap_or(start);
    let author_account = creator.parse::<u64>().ok().and_then(|c| c.checked_sub(STEAM_ID64_BASE));
    out.comments = parse_comments_html(v.get("comments_html").and_then(|h| h.as_str()).unwrap_or(""), author_account);
    Ok(out)
}

/// The comments in `comments_html`. `author_account` is the item author's 32-bit account id.
fn parse_comments_html(html: &str, author_account: Option<u64>) -> Vec<WorkshopComment> {
    const OPEN: &str = "class=\"commentthread_comment ";
    let starts: Vec<usize> = html.match_indices(OPEN).map(|(i, _)| i).collect();
    let mut out = Vec::new();
    for (n, &at) in starts.iter().enumerate() {
        let block = &html[at..starts.get(n + 1).copied().unwrap_or(html.len())];
        let id = attr_after(block, "id=\"comment_").map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>()).unwrap_or_default();
        let link_at = block.find("commentthread_author_link");
        let link = link_at.map(|i| &block[i..]);
        let author_profile = link.and_then(|l| attr_after(l, "href=\"")).map(|s| decode(s)).filter(|s| s.starts_with("https://"));
        let account = link.and_then(|l| attr_after(l, "data-miniprofile=\"")).and_then(|s| s.parse::<u64>().ok());
        let author = link
            .and_then(|l| {
                let name = between(l, "<bdi>", "</bdi>").or_else(|| l.find('>').map(|i| &l[i + 1..l.find("</a>").unwrap_or(l.len())]))?;
                Some(html_to_text(name))
            })
            .unwrap_or_default();
        let stamp = block.find("data-timestamp=\"");
        let when = stamp.and_then(|i| attr_after(&block[i..], "data-timestamp=\"")).and_then(|s| s.parse().ok());
        // The title sits on the same tag, just before the timestamp.
        let when_text = stamp
            .and_then(|i| {
                let tag = &block[block[..i].rfind('<').unwrap_or(0)..i];
                attr_after(tag, "title=\"").map(decode)
            })
            .unwrap_or_default();
        let text = block
            .find("class=\"commentthread_comment_text\"")
            .and_then(|i| block[i..].find('>').map(|j| i + j + 1))
            .map(|body| html_to_text(inner_div(&block[body..])))
            .unwrap_or_default();
        if id.is_empty() && text.is_empty() {
            continue;
        }
        out.push(WorkshopComment {
            id,
            author,
            author_profile,
            when,
            when_text,
            text,
            by_author: account.is_some() && account == author_account,
        });
    }
    out
}

/// The value of the attribute whose opening (`name="`) is given, up to its closing quote.
fn attr_after<'a>(s: &'a str, open: &str) -> Option<&'a str> {
    let i = s.find(open)? + open.len();
    let rest = &s[i..];
    Some(&rest[..rest.find('"')?])
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let i = s.find(open)? + open.len();
    let rest = &s[i..];
    Some(&rest[..rest.find(close)?])
}

/// What a `<div>` holds, from just after its opening tag to its own `</div>` (quotes nest divs).
fn inner_div(s: &str) -> &str {
    let mut depth = 1usize;
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest.starts_with("</div") {
            depth -= 1;
            if depth == 0 {
                return &s[..i];
            }
            i += 5;
        } else if rest.starts_with("<div") {
            depth += 1;
            i += 4;
        } else {
            i += rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
    }
    s
}

/// Steam's comment HTML as plain text: tags out (an emoticon's `alt` kept), `<br>` and block ends
/// as line breaks, entities decoded, Steam's own indentation and blank runs trimmed.
pub(crate) fn html_to_text(html: &str) -> String {
    let mut raw = String::new();
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        raw.push_str(&rest[..lt].replace(['\r', '\n', '\t'], " "));
        let after = &rest[lt..];
        let Some(gt) = after.find('>') else {
            raw.push_str(&after.replace(['\r', '\n', '\t'], " "));
            rest = "";
            break;
        };
        let tag = &after[1..gt];
        let name: String = tag.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
        match name.as_str() {
            "br" => raw.push('\n'),
            "div" | "p" | "blockquote" | "li" | "ul" | "ol" | "h1" | "h2" | "h3" if tag.starts_with('/') || name != "li" => raw.push('\n'),
            "li" => raw.push_str("\n- "),
            "img" => {
                if let Some(alt) = attr_after(tag, "alt=\"") {
                    raw.push_str(alt);
                }
            }
            _ => {}
        }
        rest = &after[gt + 1..];
    }
    raw.push_str(&rest.replace(['\r', '\n', '\t'], " "));
    let decoded = decode(&raw);
    let mut lines: Vec<String> = decoded.split('\n').map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect();
    // No more than one blank line in a row, none at the ends.
    lines.dedup_by(|a, b| a.is_empty() && b.is_empty());
    let joined = lines.join("\n");
    joined.trim_matches('\n').to_string()
}

/// HTML entities: the named ones Steam uses and every numeric one.
fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp..];
        let end = after[1..].find(|c: char| c == ';' || c == '&' || c.is_whitespace()).map(|i| i + 1);
        let decoded = end.filter(|&e| after.as_bytes()[e] == b';').and_then(|e| {
            let name = &after[1..e];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                _ => name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            }?;
            Some((c, e + 1))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &after[len..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Steam's answer for Mod Options (2169435993), 15 comments, saved 2026-11-04.
    const MOD_OPTIONS: &str = include_str!("testdata/comments_2169435993.json");
    const MOD_OPTIONS_CREATOR: &str = "76561198023676128";
    /// Steam's answer for Nexus Tow Truck (3794824552), 2 comments, one by its author, saved 2026-11-04.
    const TOW_TRUCK: &str = include_str!("testdata/comments_3794824552.json");
    const TOW_TRUCK_CREATOR: &str = "76561198036950864";

    #[test]
    fn a_page_of_comments_reads_with_author_time_and_text() {
        let c = parse_render("2169435993", MOD_OPTIONS_CREATOR, 0, MOD_OPTIONS).unwrap();
        assert_eq!(c.total, 897);
        assert_eq!(c.start, 0);
        assert_eq!(c.comments.len(), 15);
        assert!(c.unavailable.is_none());
        for comment in &c.comments {
            assert!(!comment.id.is_empty() && comment.id.chars().all(|ch| ch.is_ascii_digit()), "{:?}", comment);
            assert!(!comment.author.is_empty(), "{:?}", comment);
            assert!(comment.author_profile.as_deref().unwrap_or("").starts_with("https://steamcommunity.com/"), "{:?}", comment);
            assert!(comment.when.unwrap_or(0) > 1_500_000_000, "{:?}", comment);
            assert!(comment.when_text.contains('@'), "{:?}", comment);
            assert!(!comment.text.contains('<') && !comment.text.contains("&quot;"), "{:?}", comment);
        }
        // Newest first, as Steam sends them.
        assert!(c.comments.windows(2).all(|w| w[0].when >= w[1].when));
        assert_eq!(c.comments[0].text, "b42 already has mod option, this mod isn\u{2019}t needed in b42");
        assert_eq!(c.comments[5].text, "полезно");
    }

    #[test]
    fn line_breaks_links_and_entities_come_out_as_plain_text() {
        let c = parse_render("2169435993", MOD_OPTIONS_CREATOR, 0, MOD_OPTIONS).unwrap();
        let config = c.comments.iter().find(|x| x.text.starts_with("@anyone interested.")).unwrap();
        assert!(config.text.contains("click the \"config\" button"), "{}", config.text);
        assert!(config.text.ends_with("bring them up.\n\nhttps://steamcommunity.com/sharedfiles/filedetails/?id=3778911985"), "{}", config.text);
        let folder = c.comments.iter().find(|x| x.text.starts_with("confirmed working on 42.20.2")).unwrap();
        assert_eq!(folder.text, "confirmed working on 42.20.2 with @GiaoSuNgannhY method. thanks.\njust made \"42\" folder in it, didnt have to make \"common\" folder btw.");
        let path = c.comments.iter().find(|x| x.text.starts_with("Go to YourPath")).unwrap();
        assert!(path.text.contains(r"YourPath\steamapps\workshop\content\108600\2169435993\mods\ModOptions"), "{}", path.text);
    }

    #[test]
    fn the_authors_own_reply_is_marked() {
        let c = parse_render("3794824552", TOW_TRUCK_CREATOR, 0, TOW_TRUCK).unwrap();
        assert_eq!(c.total, 2);
        let names: Vec<(&str, bool)> = c.comments.iter().map(|x| (x.author.as_str(), x.by_author)).collect();
        assert_eq!(names, vec![("Caedo", false), ("Nexus382", true)]);
        assert_eq!(c.comments[0].author_profile.as_deref(), Some("https://steamcommunity.com/profiles/76561197983329816"));
        assert_eq!(c.comments[0].when, Some(1792613219));
        assert_eq!(c.comments[0].when_text, "21 October, 2026 @ 1:06:59 pm PST");
        assert!(c.comments[0].text.starts_with("This mod has me wondering"));
        assert!(c.comments[0].text.ends_with("Is that something possible to do?"));
    }

    #[test]
    fn a_private_thread_says_why_instead_of_failing() {
        let c = parse_render("2200148440", "76561198010755218", 0, r#"{"success":false,"error":"This profile is private."}"#).unwrap();
        assert_eq!(c.unavailable.as_deref(), Some("This profile is private."));
        assert!(c.comments.is_empty());
        assert!(parse_render("1", "1", 0, "<html>").is_err());
    }

    #[test]
    fn html_becomes_text() {
        assert_eq!(html_to_text("a<br>b<br><br><br>c"), "a\nb\n\nc");
        assert_eq!(html_to_text("\n\t\t\t  hi &amp; bye &#39;x&#x27; &lt;3&nbsp;  "), "hi & bye 'x' <3");
        assert_eq!(html_to_text("<blockquote class=\"bb_blockquote\"><div class=\"quoteauthor\">Originally posted by <b>X</b>:</div>old</blockquote>new"), "Originally posted by X:\nold\nnew");
        assert_eq!(html_to_text("nice <img src=\"e.png\" alt=\":steamhappy:\" class=\"emoticon\">"), "nice :steamhappy:");
        assert_eq!(html_to_text("R&D & fish &unknown; done"), "R&D & fish &unknown; done");
        assert_eq!(inner_div("a<div>b</div>c</div>tail"), "a<div>b</div>c");
    }
}
