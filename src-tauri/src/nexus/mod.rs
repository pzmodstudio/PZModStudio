//! Nexus Mods browsing for Project Zomboid through the public GraphQL v2 API (no API key).
//!
//! Only reads public listings the user asks for; downloads stay on the Nexus website (the user
//! downloads there and installs the file with `mod_install`). A public release that also downloads
//! through the API must first be registered with Nexus (see docs/ROADMAP.md, Phase 2).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

const GRAPHQL_URL: &str = "https://api.nexusmods.com/v2/graphql";
const GAME_DOMAIN: &str = "projectzomboid";
/// Nexus' numeric id for Project Zomboid (`game(domainName: "projectzomboid") { id }`).
const GAME_ID: u64 = 583;
/// Nexus asks every client to identify itself; Cloudflare also rejects anonymous user agents.
const APP_NAME: &str = "PZ Mod Studio";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusMod {
    pub mod_id: u64,
    pub name: String,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub uploader: Option<String>,
    pub version: Option<String>,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub downloads: u64,
    pub endorsements: u64,
    pub picture_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub adult_content: bool,
    /// The mod page; files are on `<page_url>?tab=files`.
    pub page_url: String,
    /// "41" / "42" when the name or summary says which build it is for (Nexus has no build tag).
    pub build_hint: Option<String>,
    /// Nexus category ("Gameplay", "Items", ...).
    pub category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NexusCategory {
    pub name: String,
    /// Project Zomboid mods in it.
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NexusRequirement {
    /// Set for requirements hosted on Nexus.
    pub mod_id: Option<u64>,
    pub name: String,
    pub url: Option<String>,
    /// Hosted somewhere else (often the Steam Workshop).
    pub external: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NexusFile {
    pub name: String,
    pub version: Option<String>,
    /// MAIN, UPDATE, OPTIONAL, MISCELLANEOUS (archived and old files are left out).
    pub category: String,
    pub size_bytes: Option<u64>,
    /// Unix seconds.
    pub uploaded: Option<u64>,
    pub description: Option<String>,
}

/// Everything the detail page shows for one Nexus mod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusModDetails {
    #[serde(rename = "mod")]
    pub info: NexusMod,
    /// The full description as BBCode (Nexus' HTML line breaks turned into newlines).
    pub description: String,
    pub requirements: Vec<NexusRequirement>,
    pub files: Vec<NexusFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusSearchResult {
    pub total: u64,
    pub offset: u64,
    pub mods: Vec<NexusMod>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NexusSort {
    Updated,
    Downloads,
    Endorsements,
    Newest,
}

impl NexusSort {
    fn field(self) -> &'static str {
        match self {
            NexusSort::Updated => "updatedAt",
            NexusSort::Downloads => "downloads",
            NexusSort::Endorsements => "endorsements",
            NexusSort::Newest => "createdAt",
        }
    }
}

pub fn mod_page_url(mod_id: u64) -> String {
    format!("https://www.nexusmods.com/{}/mods/{}", GAME_DOMAIN, mod_id)
}

/// "B42", "Build 42", "build42", "(42.13)" -> "42"; same for 41.
pub fn build_hint(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let mut found: Vec<&str> = Vec::new();
    for build in ["41", "42"] {
        let patterns = [format!("b{}", build), format!("build {}", build), format!("build{}", build), format!("{}.", build)];
        let hit = patterns.iter().any(|p| {
            lower.match_indices(p.as_str()).any(|(i, m)| {
                let before_ok = i == 0 || !lower[..i].chars().last().is_some_and(|c| c.is_ascii_alphanumeric());
                let after = lower[i + m.len()..].chars().next();
                // "42." must be followed by a digit (a version), the others by a non-digit.
                let after_ok = if m.ends_with('.') { after.is_some_and(|c| c.is_ascii_digit()) } else { !after.is_some_and(|c| c.is_ascii_digit()) };
                before_ok && after_ok
            })
        });
        if hit {
            found.push(build);
        }
    }
    match found.as_slice() {
        [one] => Some(one.to_string()),
        [_, _] => Some("41+42".to_string()),
        _ => None,
    }
}

fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().provider(ureq::tls::TlsProvider::NativeTls).build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(format!("PZModStudio/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_string)
}

fn parse_mod(node: &Value) -> Option<NexusMod> {
    let mod_id = node.get("modId")?.as_u64()?;
    let name = str_field(node, "name")?;
    let summary = str_field(node, "summary");
    let hint_source = format!("{} {}", name, summary.as_deref().unwrap_or(""));
    Some(NexusMod {
        mod_id,
        build_hint: build_hint(&hint_source),
        page_url: mod_page_url(mod_id),
        summary,
        author: str_field(node, "author"),
        uploader: node.get("uploader").and_then(|u| str_field(u, "name")),
        version: str_field(node, "version"),
        updated_at: str_field(node, "updatedAt"),
        created_at: str_field(node, "createdAt"),
        downloads: node.get("downloads").and_then(|x| x.as_u64()).unwrap_or(0),
        endorsements: node.get("endorsements").and_then(|x| x.as_u64()).unwrap_or(0),
        picture_url: str_field(node, "pictureUrl"),
        thumbnail_url: str_field(node, "thumbnailUrl"),
        adult_content: node.get("adultContent").and_then(|x| x.as_bool()).unwrap_or(false),
        category: node.pointer("/modCategory/name").and_then(|x| x.as_str()).map(str::to_string),
        name,
    })
}

const MOD_FIELDS: &str = "modId name summary author uploader { name } version updatedAt createdAt downloads endorsements pictureUrl thumbnailUrl adultContent modCategory { name }";

fn graphql(body: &Value, what: &str) -> Result<Value, String> {
    let response: Value = agent()
        .post(GRAPHQL_URL)
        .header("Application-Name", APP_NAME)
        .header("Application-Version", env!("CARGO_PKG_VERSION"))
        .send_json(body)
        .map_err(|e| format!("Nexus Mods could not be reached: {}", e))?
        .into_body()
        .read_json()
        .map_err(|e| format!("Unexpected answer from Nexus Mods: {}", e))?;
    if let Some(errors) = response.get("errors") {
        return Err(format!("Nexus Mods rejected the {}: {}", what, errors));
    }
    Ok(response)
}

/// One page of Project Zomboid mods on Nexus, optionally filtered by name (case-insensitive words)
/// and category.
pub fn search_mods(query: Option<&str>, category: Option<&str>, sort: NexusSort, offset: u64, count: u64) -> Result<NexusSearchResult, String> {
    let count = count.clamp(1, 50);
    let mut filter = json!({ "gameDomainName": [{ "value": GAME_DOMAIN }] });
    if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
        // The name filter matches words; wildcards ("*") make it match nothing.
        filter["name"] = json!([{ "value": q.replace('*', " ").trim(), "op": "WILDCARD" }]);
    }
    if let Some(c) = category.map(str::trim).filter(|c| !c.is_empty()) {
        filter["categoryName"] = json!([{ "value": c }]);
    }
    let body = json!({
        "query": format!("query($filter: ModsFilter, $sort: [ModsSort!], $offset: Int, $count: Int) {{ mods(filter: $filter, sort: $sort, offset: $offset, count: $count) {{ totalCount nodes {{ {} }} }} }}", MOD_FIELDS),
        "variables": {
            "filter": filter,
            "sort": [{ sort.field(): { "direction": "DESC" } }],
            "offset": offset,
            "count": count,
        }
    });
    let response = graphql(&body, "search")?;
    let mods = response.pointer("/data/mods").ok_or("Unexpected answer from Nexus Mods (no data)")?;
    Ok(NexusSearchResult {
        total: mods.get("totalCount").and_then(|t| t.as_u64()).unwrap_or(0),
        offset,
        mods: mods
            .get("nodes")
            .and_then(|n| n.as_array())
            .map(|nodes| nodes.iter().filter_map(parse_mod).collect())
            .unwrap_or_default(),
    })
}

/// Project Zomboid categories on Nexus with how many mods each has, largest first.
pub fn categories() -> Result<Vec<NexusCategory>, String> {
    let body = json!({
        "query": "query($filter: ModsFilter) { mods(filter: $filter, facets: { categoryName: [] }, count: 0) { facetsData } }",
        "variables": { "filter": { "gameDomainName": [{ "value": GAME_DOMAIN }] } }
    });
    let response = graphql(&body, "category list")?;
    let mut list: Vec<NexusCategory> = response
        .pointer("/data/mods/facetsData/categoryName")
        .and_then(|c| c.as_object())
        .map(|o| o.iter().map(|(name, n)| NexusCategory { name: name.clone(), count: n.as_u64().unwrap_or(0) }).collect())
        .unwrap_or_default();
    list.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    Ok(list)
}

/// Nexus descriptions are BBCode with HTML line breaks and entities mixed in.
pub fn clean_description(text: &str) -> String {
    let mut s = text.replace("\r\n", "\n");
    for br in ["\n<br />", "\n<br/>", "\n<br>", "<br />", "<br/>", "<br>"] {
        s = s.replace(br, "\n");
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        // Within the first 10 bytes, cut on a character boundary ("&Оружие" is not ASCII).
        let mut window = rest.len().min(10);
        while !rest.is_char_boundary(window) {
            window -= 1;
        }
        let end = rest[..window].find(';');
        let decoded = end.and_then(|e| {
            let entity = &rest[1..e];
            let ch = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                _ => entity
                    .strip_prefix("#x")
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            }?;
            Some((ch, e + 1))
        });
        match decoded {
            Some((ch, len)) => {
                out.push(ch);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn parse_file(v: &Value) -> Option<NexusFile> {
    let category = v.get("category")?.as_str()?.to_string();
    if matches!(category.as_str(), "ARCHIVED" | "OLD_VERSION" | "DELETED" | "REMOVED") {
        return None;
    }
    Some(NexusFile {
        name: str_field(v, "name")?,
        version: str_field(v, "version").filter(|s| !s.is_empty()),
        size_bytes: v.get("sizeInBytes").and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| s.parse().ok()))),
        uploaded: v.get("date").and_then(|x| x.as_u64()),
        description: str_field(v, "description").filter(|s| !s.is_empty()).map(|d| clean_description(&d)),
        category,
    })
}

/// Full details for one mod: description, requirements and current files.
pub fn mod_details(mod_id: u64) -> Result<NexusModDetails, String> {
    let body = json!({
        "query": format!(
            "query($ids: [CompositeDomainWithIdInput!]!, $modId: ID!, $gameId: ID!) {{ legacyModsByDomain(ids: $ids) {{ nodes {{ {} description modRequirements {{ nexusRequirements {{ nodes {{ modId modName url externalRequirement notes }} }} }} }} }} modFiles(modId: $modId, gameId: $gameId) {{ name version category sizeInBytes date description }} }}",
            MOD_FIELDS
        ),
        "variables": { "ids": [{ "gameDomain": GAME_DOMAIN, "modId": mod_id }], "modId": mod_id.to_string(), "gameId": GAME_ID.to_string() }
    });
    let response = graphql(&body, "mod details")?;
    let node = response.pointer("/data/legacyModsByDomain/nodes/0").ok_or("Nexus Mods has no mod with this id")?;
    let info = parse_mod(node).ok_or("Unexpected answer from Nexus Mods (no mod)")?;
    let requirements = node
        .pointer("/modRequirements/nexusRequirements/nodes")
        .and_then(|n| n.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|r| {
                    Some(NexusRequirement {
                        mod_id: r.get("modId").and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| s.parse().ok()))).filter(|id| *id > 0),
                        name: str_field(r, "modName").filter(|s| !s.is_empty())?,
                        url: str_field(r, "url").filter(|s| s.starts_with("http")),
                        external: r.get("externalRequirement").and_then(|x| x.as_bool()).unwrap_or(false),
                        notes: str_field(r, "notes").filter(|s| !s.is_empty()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let mut files: Vec<NexusFile> = response
        .pointer("/data/modFiles")
        .and_then(|f| f.as_array())
        .map(|a| a.iter().filter_map(parse_file).collect())
        .unwrap_or_default();
    // Main files first, newest first inside each group.
    let rank = |c: &str| match c {
        "MAIN" => 0,
        "UPDATE" => 1,
        "OPTIONAL" => 2,
        _ => 3,
    };
    files.sort_by(|a, b| rank(&a.category).cmp(&rank(&b.category)).then_with(|| b.uploaded.cmp(&a.uploaded)));
    Ok(NexusModDetails {
        description: clean_description(node.get("description").and_then(|x| x.as_str()).unwrap_or("")),
        requirements,
        files,
        info,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_hints_from_names() {
        assert_eq!(build_hint("B42 PROJECT RV Interior").as_deref(), Some("42"));
        assert_eq!(build_hint("RV Interior for Build 42.20 Stable").as_deref(), Some("42"));
        assert_eq!(build_hint("MorePlayers (B41.65)").as_deref(), Some("41"));
        assert_eq!(build_hint("Works on build41 and build 42").as_deref(), Some("41+42"));
        assert_eq!(build_hint("Ultimate Horror Sounds"), None);
        assert_eq!(build_hint("Pack of 420 items"), None, "420 is not a build");
        assert_eq!(build_hint("version 1.42"), None);
    }

    #[test]
    fn parses_api_nodes() {
        let node = json!({
            "modId": 248, "name": "B42 PROJECT RV Interior", "summary": "RV Interior",
            "author": "Mickey Knox", "uploader": { "name": "M1ckeyKnox" }, "version": "2.4",
            "updatedAt": "2026-08-03T00:11:02Z", "downloads": 6611, "endorsements": 51,
            "thumbnailUrl": "https://example/t.png", "adultContent": false
        });
        let m = parse_mod(&node).unwrap();
        assert_eq!(m.page_url, "https://www.nexusmods.com/projectzomboid/mods/248");
        assert_eq!(m.uploader.as_deref(), Some("M1ckeyKnox"));
        assert_eq!(m.build_hint.as_deref(), Some("42"));
    }

    /// Live call to Nexus (network). cargo test nexus_live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn nexus_live_search() {
        let r = search_mods(Some("horror"), None, NexusSort::Downloads, 0, 5).unwrap();
        println!("{} results: {:?}", r.total, r.mods.iter().map(|m| (&m.name, &m.category)).collect::<Vec<_>>());
        let all = search_mods(None, None, NexusSort::Updated, 0, 3).unwrap();
        assert!(all.total > 100);
        let cats = categories().unwrap();
        println!("{:?}", cats);
        let gameplay = search_mods(None, Some("Gameplay"), NexusSort::Updated, 0, 3).unwrap();
        assert!(gameplay.total > 0 && gameplay.total < all.total);
        assert!(gameplay.mods.iter().all(|m| m.category.as_deref() == Some("Gameplay")));
        let d = mod_details(248).unwrap();
        println!("{} files {:?} reqs {:?}\n{}", d.info.name, d.files.iter().map(|f| (&f.name, &f.category)).collect::<Vec<_>>(), d.requirements, &d.description[..300.min(d.description.len())]);
        assert!(!d.files.is_empty() && !d.description.contains("<br"));
    }

    #[test]
    fn cleans_nexus_descriptions() {
        assert_eq!(clean_description("[i]Hi[/i]\n<br />\n<br />Go to C:&#92;Users &amp; more&quot;"), "[i]Hi[/i]\n\nGo to C:\\Users & more\"");
        assert_eq!(clean_description("a &unknown; b & c"), "a &unknown; b & c");
        // Non-ASCII right after "&" used to cut a character in two and panic.
        assert_eq!(clean_description("Guns &Оружие mod"), "Guns &Оружие mod");
    }
}
