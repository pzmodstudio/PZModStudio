//! Steam Workshop details for Project Zomboid items through Steam's public Web API (no key):
//! `ISteamRemoteStorage/GetPublishedFileDetails` and `GetCollectionDetails`.
//! Searching the Workshop (`IPublishedFileService/QueryFiles`) needs a Steam Web API key and is
//! not done here; users subscribe on the item's Workshop page in their browser.

pub mod polite;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const DETAILS_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/";
const COLLECTION_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetCollectionDetails/v1/";
const QUERY_URL: &str = "https://api.steampowered.com/IPublishedFileService/QueryFiles/v1/";
const PZ_APP_ID: u64 = 108600;
const BATCH: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopItem {
    pub workshop_id: String,
    /// False when Steam has no public item with this id (deleted, private, wrong id).
    pub found: bool,
    pub title: String,
    pub summary: String,
    pub preview_url: Option<String>,
    /// Unix seconds.
    pub time_updated: Option<u64>,
    /// When it was first posted, unix seconds.
    #[serde(default)]
    pub time_created: Option<u64>,
    pub subscriptions: u64,
    pub favorited: u64,
    pub file_size: u64,
    pub tags: Vec<String>,
    /// "41", "42"... from the "Build NN" tags.
    pub builds: Vec<String>,
    /// False for items of another game.
    pub is_project_zomboid: bool,
    pub banned: bool,
    /// The item's Workshop page (subscribe there when logged in on steamcommunity.com).
    pub page_url: String,
    /// A collection's items, in the order the author put them (empty for a normal mod).
    pub children: Vec<String>,
    /// True when this is a collection rather than a single mod.
    pub is_collection: bool,
    /// Where to support its author (Ko-fi, Patreon...), from the full description; empty when
    /// Steam sent only a short one (a search) or the author put none.
    #[serde(default)]
    pub support: Vec<crate::mod_changelog::AuthorLink>,
}

/// What Steam has installed for one subscribed item (from appworkshop_108600.acf, offline).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstalledItemState {
    /// Publish time (unix seconds) of the version on disk.
    pub installed_time: Option<u64>,
    /// Publish time of the newest version Steam knows about.
    pub latest_time: Option<u64>,
    pub size: Option<u64>,
    /// Steam knows a newer version that is not downloaded yet (the usual cause of
    /// "Workshop item version is different than the server's").
    pub update_pending: bool,
    /// The version on disk was published after the game last ran.
    pub updated_since_last_play: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkshopLocalState {
    /// When Project Zomboid last ran (unix seconds).
    pub time_last_app_ran: Option<u64>,
    pub items: std::collections::BTreeMap<String, InstalledItemState>,
}

/// Reads Steam's record of installed Project Zomboid Workshop items. `workshop_content_dir` is
/// `.../steamapps/workshop/content/108600`; the record is `.../steamapps/workshop/appworkshop_108600.acf`.
pub fn local_state(workshop_content_dir: &str) -> Option<WorkshopLocalState> {
    let acf = std::path::Path::new(workshop_content_dir).parent()?.parent()?.join("appworkshop_108600.acf");
    let text = std::fs::read_to_string(acf).ok()?;
    let root = crate::vdf::parse(&text);
    let app = root.get("AppWorkshop")?;
    let last_ran = app.u64("TimeLastAppRan").filter(|t| *t > 0);
    let details = app.get("WorkshopItemDetails");
    let mut items = std::collections::BTreeMap::new();
    for (id, installed) in app.get("WorkshopItemsInstalled").and_then(|b| b.block()).into_iter().flatten() {
        let detail = details.and_then(|d| d.get(id));
        let installed_time = installed.u64("timeupdated").or_else(|| detail.and_then(|d| d.u64("timeupdated")));
        let latest_time = detail.and_then(|d| d.u64("latest_timeupdated")).or(installed_time);
        let manifest = installed.str("manifest").or_else(|| detail.and_then(|d| d.str("manifest")));
        let latest_manifest = detail.and_then(|d| d.str("latest_manifest"));
        let update_pending = matches!((latest_time, installed_time), (Some(l), Some(i)) if l > i)
            || matches!((latest_manifest, manifest), (Some(l), Some(m)) if l != m && l != "0");
        items.insert(
            id.clone(),
            InstalledItemState {
                installed_time,
                latest_time,
                size: installed.u64("size"),
                update_pending,
                updated_since_last_play: matches!((installed_time, last_ran), (Some(i), Some(r)) if i > r),
            },
        );
    }
    Some(WorkshopLocalState { time_last_app_ran: last_ran, items })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopLookup {
    /// Set when the input was a collection.
    pub collection: Option<WorkshopItem>,
    pub items: Vec<WorkshopItem>,
}

pub fn page_url(id: &str) -> String {
    format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", id)
}

/// One client for every Workshop request: it keeps its connections to Steam open, so browsing
/// does not pay a fresh TLS handshake per page, collection or detail (an Agent is a cheap handle
/// to a shared pool, cloned out of here).
/// How long Steam's answers are kept on disk, by kind.
const DETAILS_TTL: Duration = Duration::from_secs(10 * 60);
const PAGE_TTL: Duration = Duration::from_secs(30 * 60);
const SEARCH_TTL: Duration = Duration::from_secs(5 * 60);




/// Workshop ids in free text: links (`...filedetails/?id=123`, `steam://url/CommunityFilePage/123`)
/// or bare numbers, in order, without duplicates.
pub fn parse_ids(input: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for token in input.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let token = token.trim();
        let candidate = if let Some(pos) = token.find("id=") {
            token[pos + 3..].chars().take_while(|c| c.is_ascii_digit()).collect::<String>()
        } else if let Some(pos) = token.find("CommunityFilePage/") {
            token[pos + 18..].chars().take_while(|c| c.is_ascii_digit()).collect()
        } else if token.chars().all(|c| c.is_ascii_digit()) {
            token.to_string()
        } else {
            String::new()
        };
        if candidate.len() >= 4 && !ids.contains(&candidate) {
            ids.push(candidate);
        }
    }
    ids
}

/// Plain-text start of a BBCode description.
fn summarize(description: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in description.chars() {
        match c {
            '[' => in_tag = true,
            ']' if in_tag => in_tag = false,
            _ if !in_tag => out.push(if c == '\r' || c == '\n' { ' ' } else { c }),
            _ => {}
        }
    }
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > 280 {
        format!("{}...", collapsed.chars().take(280).collect::<String>())
    } else {
        collapsed
    }
}

fn parse_item(v: &Value) -> Option<WorkshopItem> {
    let id = v.get("publishedfileid")?.as_str()?.to_string();
    let num = |key: &str| v.get(key).and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| s.parse().ok())));
    // GetPublishedFileDetails says whether it found the item; the browse page only ever lists
    // items that exist, and sends no result code at all.
    let found = match v.get("result").and_then(|r| r.as_u64()) {
        Some(code) => code == 1,
        None => v.get("title").is_some(),
    };
    let tags: Vec<String> = v
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|a| a.iter().filter_map(|t| t.get("tag").and_then(|x| x.as_str()).map(str::to_string)).collect())
        .unwrap_or_default();
    let builds = tags
        .iter()
        .filter_map(|t| t.strip_prefix("Build ").map(|b| b.trim().to_string()))
        .filter(|b| !b.is_empty() && b.chars().all(|c| c.is_ascii_digit()))
        .collect();
    Some(WorkshopItem {
        found,
        title: v.get("title").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        // GetPublishedFileDetails sends "description"; QueryFiles "short_description"/"file_description".
        summary: ["short_description", "description", "file_description"]
            .iter()
            .find_map(|k| v.get(*k).and_then(|x| x.as_str()).filter(|s| !s.is_empty()))
            .map(summarize)
            .unwrap_or_default(),
        preview_url: v.get("preview_url").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
        time_updated: num("time_updated"),
        time_created: num("time_created"),
        subscriptions: num("subscriptions").unwrap_or(0),
        favorited: num("favorited").unwrap_or(0),
        file_size: num("file_size").unwrap_or(0),
        is_project_zomboid: num("consumer_app_id").or_else(|| num("consumer_appid")) == Some(PZ_APP_ID),
        banned: num("banned").unwrap_or(0) != 0,
        page_url: page_url(&id),
        // A collection browse result carries its items; GetPublishedFileDetails does not.
        children: v
            .get("children")
            .and_then(|c| c.as_array())
            .map(|a| a.iter().filter_map(|c| c.get("publishedfileid").and_then(|x| x.as_str()).map(str::to_string)).collect())
            .unwrap_or_default(),
        is_collection: num("file_type") == Some(2) || v.get("num_children").and_then(|c| c.as_u64()).unwrap_or(0) > 0,
        // Only a full description: a search's short one is cut and may cut a link.
        support: ["description", "file_description"]
            .iter()
            .find_map(|k| v.get(*k).and_then(|x| x.as_str()).filter(|s| !s.is_empty()))
            .map(crate::mod_changelog::support_links)
            .unwrap_or_default(),
        tags,
        builds,
        workshop_id: id,
    })
}

fn form(prefix: &str, count_key: &str, ids: &[String]) -> Vec<(String, String)> {
    let mut pairs = vec![(count_key.to_string(), ids.len().to_string())];
    for (i, id) in ids.iter().enumerate() {
        pairs.push((format!("{}[{}]", prefix, i), id.clone()));
    }
    pairs
}

/// A Steam Web API form post. A dropped connection or a passing server error is tried twice more
/// (a pooled connection Steam closed while idle fails once, then works), so one hiccup does not
/// read as "no internet".
fn post_form(url: &str, pairs: Vec<(String, String)>, ttl: Duration) -> Result<Value, String> {
    let body = polite::fetch(polite::Request::PostForm { url, form: &pairs }, ttl, None).map_err(|r| r.say())?;
    serde_json::from_str(&body).map_err(|e| format!("Unexpected answer from Steam: {}", e))
}

/// Details for Workshop ids, in the same order (100 per request).
pub fn get_details(ids: &[String]) -> Result<Vec<WorkshopItem>, String> {
    let mut items = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(BATCH) {
        let v = post_form(DETAILS_URL, form("publishedfileids", "itemcount", chunk), DETAILS_TTL)?;
        let list = v.pointer("/response/publishedfiledetails").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        items.extend(list.iter().filter_map(parse_item));
    }
    Ok(items)
}

/// Child item ids when `id` is a collection, None when it is not one.
fn collection_children(id: &str) -> Result<Option<Vec<String>>, String> {
    let v = post_form(COLLECTION_URL, form("publishedfileids", "collectioncount", &[id.to_string()]), PAGE_TTL)?;
    let details = v.pointer("/response/collectiondetails/0");
    if details.and_then(|d| d.get("result")).and_then(|r| r.as_u64()) != Some(1) {
        return Ok(None);
    }
    let children = details
        .and_then(|d| d.get("children"))
        .and_then(|c| c.as_array())
        .map(|a| a.iter().filter_map(|c| c.get("publishedfileid").and_then(|x| x.as_str()).map(str::to_string)).collect())
        .unwrap_or_default();
    Ok(Some(children))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkshopSort {
    /// Best text match (used automatically when there is search text).
    Relevance,
    Trending,
    MostSubscribed,
    RecentlyUpdated,
    Newest,
    TopRated,
}

impl WorkshopSort {
    /// EPublishedFileQueryType values.
    fn query_type(self) -> u32 {
        match self {
            WorkshopSort::TopRated => 0,
            WorkshopSort::Newest => 1,
            WorkshopSort::Trending => 3,
            WorkshopSort::MostSubscribed => 9,
            WorkshopSort::Relevance => 12,
            WorkshopSort::RecentlyUpdated => 21,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopSearchResult {
    pub total: u64,
    pub page: u32,
    /// Pages available for this search (Steam stops at 1000).
    pub pages: u32,
    pub items: Vec<WorkshopItem>,
}

impl WorkshopSort {
    /// `browsesort` value on the public Workshop browse page.
    fn browse_sort(self) -> &'static str {
        match self {
            WorkshopSort::TopRated => "toprated",
            WorkshopSort::Newest => "mostrecent",
            // The page ranks search text by relevance on its own.
            WorkshopSort::Trending | WorkshopSort::Relevance => "trend",
            WorkshopSort::MostSubscribed => "totaluniquesubscribers",
            WorkshopSort::RecentlyUpdated => "lastupdated",
        }
    }
}

/// The `<script type="application/json">` state the Workshop browse page is built from.
fn page_state(html: &str) -> Option<Value> {
    let mut rest = html;
    while let Some(i) = rest.find("type=\"application/json\"") {
        rest = &rest[i..];
        let start = rest.find('>')? + 1;
        let end = rest[start..].find("</script>")? + start;
        let parsed = serde_json::from_str::<Value>(&rest[start..end]).ok();
        if let Some(v) = parsed {
            return Some(v);
        }
        rest = &rest[end..];
    }
    None
}

/// The QueryFiles answer embedded in the page: the object holding `total_count` and `results`.
/// Some of it is nested as JSON inside strings, so those are parsed too.
fn find_results(value: &Value, depth: u32) -> Option<Value> {
    if depth > 24 {
        return None;
    }
    match value {
        Value::Object(map) => {
            if map.contains_key("total_count") && map.get("results").is_some_and(Value::is_array) {
                return Some(value.clone());
            }
            map.values().find_map(|v| find_results(v, depth + 1))
        }
        Value::Array(items) => items.iter().find_map(|v| find_results(v, depth + 1)),
        Value::String(s) if s.len() > 200 && s.contains("\"total_count\"") => {
            serde_json::from_str::<Value>(s).ok().and_then(|v| find_results(&v, depth + 1))
        }
        _ => None,
    }
}

/// What to list on the Workshop browse page.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BrowseQuery {
    pub text: Option<String>,
    pub sort: Option<WorkshopSort>,
    /// "41" / "42": keeps items tagged "Build NN".
    pub build: Option<String>,
    /// One of [`CATEGORIES`].
    pub category: Option<String>,
    /// How far back "most popular" looks: 1, 7, 90 or 365 days (Steam's own choices).
    pub days: Option<u32>,
    /// List collections instead of single mods.
    pub collections: bool,
    pub page: u32,
    pub per_page: u32,
}

/// Searches the Workshop through its public browse page, which embeds the same item data the
/// Web API returns. No Steam Web API key needed.
pub fn search_public(q: &BrowseQuery) -> Result<WorkshopSearchResult, String> {
    let mut sort = q.sort.unwrap_or(WorkshopSort::Trending);
    // Steam answers "most subscribed" with nothing at all when asked for collections (checked
    // against the live site: every other order returns them). Top rated is the nearest thing it
    // will answer, and an order the caller did not expect beats an empty page.
    if q.collections && sort == WorkshopSort::MostSubscribed {
        sort = WorkshopSort::TopRated;
    }
    let page = q.page.max(1);
    let per_page = if q.per_page == 0 { 30 } else { q.per_page.clamp(1, 30) };
    let mut url_query: Vec<(String, String)> = vec![
        ("appid".into(), PZ_APP_ID.to_string()),
        ("section".into(), if q.collections { "collections" } else { "readytouseitems" }.into()),
        ("browsesort".into(), sort.browse_sort().to_string()),
        ("numperpage".into(), per_page.to_string()),
        ("p".into(), page.to_string()),
    ];
    if sort == WorkshopSort::Trending {
        // Steam offers today, a week, three months and a year; anything else it ignores.
        url_query.push(("days".into(), q.days.filter(|d| [1, 7, 90, 365].contains(d)).unwrap_or(7).to_string()));
    }
    if let Some(t) = q.text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        url_query.push(("searchtext".into(), t.to_string()));
    }
    for tag in tag_filters(q.build.as_deref(), q.category.as_deref()) {
        url_query.push(("requiredtags[]".into(), tag));
    }
    // Through the shared limiter (retries, Steam's pace, the disk cache).
    let html = polite::fetch(polite::Request::Get { url: "https://steamcommunity.com/workshop/browse/", query: &url_query }, SEARCH_TTL, None).map_err(|r| match r {
        polite::Refusal::Network(e) => format!("The Steam Workshop could not be reached: {}", e),
        other => other.say(),
    })?;
    let results = page_state(&html)
        .as_ref()
        .and_then(|v| find_results(v, 0))
        .ok_or("The Steam Workshop page could not be read (Steam may have changed it).")?;
    let num = |key: &str| results.get(key).and_then(|x| x.as_u64());
    // Steam's search sometimes refuses the query itself: `eresult` 10 with "Rejected CRE query,
    // because there are already 501 jobs and the max we allow are 500" and no results (seen
    // 2026-10-20 for every mod listing; its site shows "There was a problem performing that
    // action. (10)"). Said as such, not as "0 mods".
    if num("eresult").is_some_and(|r| r != 1) || results.get("error").and_then(|e| e.as_str()).is_some_and(|e| !e.is_empty()) {
        return Err("Steam Workshop busy: Steam's own Workshop search is refusing queries right now (its site shows the same error). Try again in a while.".to_string());
    }
    Ok(WorkshopSearchResult {
        total: num("total_count").unwrap_or(0),
        page: num("current_page").unwrap_or(page as u64) as u32,
        pages: num("total_pages").unwrap_or(1) as u32,
        items: results
            .get("results")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(parse_item).collect())
            .unwrap_or_default(),
    })
}

/// "Build NN" plus the chosen category, as the Workshop spells its tags.
fn tag_filters(build: Option<&str>, category: Option<&str>) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(b) = build.filter(|b| !b.is_empty() && b.chars().all(|c| c.is_ascii_digit())) {
        tags.push(format!("Build {}", b));
    }
    if let Some(c) = category.and_then(|c| CATEGORIES.iter().find(|k| k.eq_ignore_ascii_case(c))) {
        tags.push(c.to_string());
    }
    tags
}

/// Searches Project Zomboid Workshop items with the user's own Steam Web API key.
/// `build` ("41" / "42") keeps only items tagged "Build NN"; `category` one of [`CATEGORIES`].
pub fn search(
    key: &str,
    text: Option<&str>,
    sort: WorkshopSort,
    build: Option<&str>,
    category: Option<&str>,
    page: u32,
    per_page: u32,
) -> Result<WorkshopSearchResult, String> {
    let text = text.map(str::trim).filter(|t| !t.is_empty());
    // Text search only works with the relevance ranking; without text, relevance means trending.
    let sort = match (text.is_some(), sort) {
        (true, _) => WorkshopSort::Relevance,
        (false, WorkshopSort::Relevance) => WorkshopSort::Trending,
        (false, s) => s,
    };
    let page = page.max(1);
    let mut q: Vec<(String, String)> = Vec::new();
    let mut add = |k: String, v: String| q.push((k, v));
    add("key".into(), key.to_string());
    add("appid".into(), PZ_APP_ID.to_string());
    add("query_type".into(), sort.query_type().to_string());
    add("page".into(), page.to_string());
    add("numperpage".into(), per_page.clamp(1, 100).to_string());
    for (k, v) in [("filetype", "0"), ("return_tags", "true"), ("return_previews", "true"), ("return_short_description", "true"), ("return_vote_data", "false")] {
        add(k.into(), v.into());
    }
    if sort == WorkshopSort::Trending {
        add("days".into(), "7".into());
    }
    if let Some(t) = text {
        add("search_text".into(), t.to_string());
    }
    let tags = tag_filters(build, category);
    for (i, t) in tags.iter().enumerate() {
        add(format!("requiredtags[{}]", i), t.clone());
    }
    if !tags.is_empty() {
        add("match_all_tags".into(), "true".into());
    }
    // Never kept on disk: the Web API key is part of the request.
    let body = polite::fetch(polite::Request::Get { url: QUERY_URL, query: &q }, Duration::ZERO, None).map_err(|r| match r {
        polite::Refusal::Status(401) | polite::Refusal::Status(403) => {
            "Steam rejected the Web API key. Check it in Browse Mods > Steam Workshop (or create a new one at steamcommunity.com/dev/apikey).".to_string()
        }
        other => other.say(),
    })?;
    let v: Value = serde_json::from_str(&body).map_err(|e| format!("Unexpected answer from Steam: {}", e))?;
    let r = v.get("response").ok_or("Unexpected answer from Steam (no response)")?;
    let total = r.get("total").and_then(|t| t.as_u64()).unwrap_or(0);
    Ok(WorkshopSearchResult {
        total,
        page,
        pages: total.div_ceil(per_page.clamp(1, 100) as u64).min(1000) as u32,
        items: r
            .get("publishedfiledetails")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(parse_item).collect())
            .unwrap_or_default(),
    })
}

/// The Workshop's own categories for Project Zomboid (tags shown on the Workshop browse page).
pub const CATEGORIES: &[&str] = &[
    "Animals", "Audio", "Balance", "Building", "Clothing/Armor", "Farming", "Food", "Framework", "Hardmode",
    "Interface", "Items", "Language/Translation", "Literature", "Map", "Military", "Misc", "Models",
    "Multiplayer", "Pop Culture", "QoL", "Realistic", "Silly/Fun", "Skills", "Textures", "Traits", "Vehicles",
    "Weapons", "WIP",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequiredItem {
    pub workshop_id: String,
    pub title: String,
}

/// Everything the detail page shows for one Workshop item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopItemDetails {
    pub item: WorkshopItem,
    /// The full description as the author wrote it (Steam BBCode).
    pub description: String,
    pub time_created: Option<u64>,
    /// Author's Steam name (from the public page; None when it could not be read).
    pub author: Option<String>,
    /// Full-size screenshot URLs (public page).
    pub screenshots: Vec<String>,
    /// YouTube video ids (public page).
    pub videos: Vec<String>,
    /// "Required items" the author listed (public page).
    pub required_items: Vec<RequiredItem>,
}

/// What the item's public page adds to the Web API data (no key needed).
#[derive(Debug, Clone, Default, PartialEq)]
struct PageExtras {
    author: Option<String>,
    screenshots: Vec<String>,
    videos: Vec<String>,
    required_items: Vec<RequiredItem>,
}

fn decode_entities(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// Reads author, screenshots, videos and required items from the item's public Workshop page.
fn parse_item_page(html: &str) -> PageExtras {
    let mut out = PageExtras::default();
    if let Some(i) = html.find("class=\"friendBlockContent\">") {
        let rest = &html[i + 27..];
        let name = decode_entities(rest[..rest.find('<').unwrap_or(0)].trim());
        if !name.is_empty() {
            out.author = Some(name);
        }
    }
    for (i, _) in html.match_indices("highlight_strip_screenshot") {
        let rest = &html[i..];
        let Some(src) = rest.find("<img src=\"") else { continue };
        let url = &rest[src + 10..];
        let url = &url[..url.find('"').unwrap_or(0)];
        // Strip the thumbnail sizing to get the full image.
        let full = url.split('?').next().unwrap_or("").to_string();
        if full.starts_with("https://") && !out.screenshots.contains(&full) {
            out.screenshots.push(full);
        }
    }
    for (i, m) in html.match_indices("img.youtube.com/vi/") {
        let id: String = html[i + m.len()..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
        if id.len() >= 6 && !out.videos.contains(&id) {
            out.videos.push(id);
        }
    }
    if let Some(start) = html.find("id=\"RequiredItems\">") {
        let mut rest = &html[start + 19..];
        const LINK: &str = "<a href=\"https://steamcommunity.com/workshop/filedetails/?id=";
        loop {
            let trimmed = rest.trim_start();
            let Some(after) = trimmed.strip_prefix(LINK) else { break };
            let id: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            let Some(t) = after.find("class=\"requiredItem\">") else { break };
            let text = &after[t + 21..];
            let title = decode_entities(text[..text.find("</div>").unwrap_or(0)].trim());
            if !id.is_empty() {
                out.required_items.push(RequiredItem { workshop_id: id, title });
            }
            let Some(end) = after.find("</a>") else { break };
            rest = &after[end + 4..];
        }
    }
    out
}

/// Full details for one item: the Web API (description, stats, tags) plus its public page
/// (screenshots, videos, required items). The page part is best effort.
pub fn item_details(id: &str) -> Result<WorkshopItemDetails, String> {
    let v = post_form(DETAILS_URL, form("publishedfileids", "itemcount", &[id.to_string()]), DETAILS_TTL)?;
    let raw = v.pointer("/response/publishedfiledetails/0").cloned().ok_or("Unexpected answer from Steam (no item)")?;
    let item = parse_item(&raw).ok_or("Unexpected answer from Steam (no item id)")?;
    if !item.found {
        return Err("Steam has no public item with this id (removed, private or wrong id).".to_string());
    }
    let extras = fetch_page(&page_url(id)).ok().map(|html| parse_item_page(&html)).unwrap_or_default();
    Ok(WorkshopItemDetails {
        description: raw.get("description").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        time_created: raw.get("time_created").and_then(|x| x.as_u64()),
        author: extras.author,
        screenshots: extras.screenshots,
        videos: extras.videos,
        required_items: extras.required_items,
        item,
    })
}

// ------------------------------------------------------------------ what an item needs

/// One item a subscription needs, found by following "Required items" from the items asked for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredNode {
    pub workshop_id: String,
    pub title: String,
    pub preview_url: Option<String>,
    pub file_size: u64,
    /// False when Steam has no public item with this id (removed or private).
    pub found: bool,
    /// Titles of the items that list it.
    pub required_by: Vec<String>,
    /// 1 for what the items asked for list, 2 for what those list, ...
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredChain {
    pub items: Vec<RequiredNode>,
    /// The chain was longer than what is followed (`CHAIN_MAX_ITEMS` / `CHAIN_MAX_DEPTH`).
    pub cut_short: bool,
    /// Items whose page could not be read (Steam busy, no connection): what they need is not in
    /// the list, which may be incomplete.
    pub unread: Vec<String>,
    /// The items asked for, with their titles (when Steam gave them).
    pub asked: Vec<RequiredItem>,
}

const CHAIN_MAX_ITEMS: usize = 60;
const CHAIN_MAX_DEPTH: u32 = 6;
const PAGE_CACHE_SECS: u64 = 600;

// Pages read in this session: the same chain is asked for again when a dialog opens twice.
static PAGE_REQUIRED: std::sync::Mutex<Option<std::collections::HashMap<String, (std::time::Instant, Vec<RequiredItem>)>>> = std::sync::Mutex::new(None);

/// The "Required items" on an item's public page (cached for ten minutes).
fn page_required_items(id: &str) -> Result<Vec<RequiredItem>, String> {
    if let Ok(guard) = PAGE_REQUIRED.lock() {
        if let Some((at, items)) = guard.as_ref().and_then(|m| m.get(id)) {
            if at.elapsed().as_secs() < PAGE_CACHE_SECS {
                return Ok(items.clone());
            }
        }
    }
    let html = fetch_page(&page_url(id)).map_err(|e| format!("Cannot read the Workshop page of {}: {}", id, e))?;
    let items = parse_item_page(&html).required_items;
    if let Ok(mut guard) = PAGE_REQUIRED.lock() {
        guard.get_or_insert_with(Default::default).insert(id.to_string(), (std::time::Instant::now(), items.clone()));
    }
    Ok(items)
}

/// An item's public Workshop page. Steam's edge sometimes refuses this client ("429") while
/// answering others from the same machine (seen 2026-10-20 during its Workshop trouble): then
/// the page is asked for once through the curl that ships with Windows.
pub(crate) fn fetch_page(url: &str) -> Result<String, String> {
    let curl = || page_through_system_curl(url);
    polite::fetch(polite::Request::Get { url, query: &[] }, PAGE_TTL, Some(&curl)).map_err(|r| r.say())
}

/// A Workshop page through `%SystemRoot%\\System32\\curl.exe` (Windows 10 and later), no window.
#[cfg(windows)]
fn page_through_system_curl(url: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    let curl = std::path::Path::new(&root).join("System32").join("curl.exe");
    if !url.starts_with("https://steamcommunity.com/") || !curl.is_file() {
        return None;
    }
    let out = std::process::Command::new(curl)
        .args(["-s", "-f", "-L", "--max-time", "20", "--max-filesize", "8388608", url])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(not(windows))]
fn page_through_system_curl(_url: &str) -> Option<String> {
    None
}

/// Everything the given items need, following "Required items" down the chain, each item once
/// and never the ones asked for. `pages` reads one item's list (the public page, or a test's).
pub fn required_chain_with(ids: &[String], mut pages: impl FnMut(&str) -> Result<Vec<RequiredItem>, String>) -> (Vec<RequiredNode>, bool, Vec<String>) {
    let asked: std::collections::HashSet<&String> = ids.iter().collect();
    let mut nodes: Vec<RequiredNode> = Vec::new();
    let mut queue: std::collections::VecDeque<(String, String, u32)> = std::collections::VecDeque::new();
    for id in ids {
        queue.push_back((id.clone(), String::new(), 0));
    }
    let mut read: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut cut_short = false;
    let mut unread: Vec<String> = Vec::new();
    while let Some((id, title, depth)) = queue.pop_front() {
        if !read.insert(id.clone()) {
            continue;
        }
        if depth >= CHAIN_MAX_DEPTH {
            cut_short = true;
            continue;
        }
        let parent = if title.is_empty() { id.clone() } else { title };
        // One page that cannot be read leaves out what that item needs, not the whole chain.
        let Ok(required) = pages(&id) else {
            unread.push(id.clone());
            continue;
        };
        for req in required {
            if asked.contains(&req.workshop_id) {
                continue;
            }
            if let Some(node) = nodes.iter_mut().find(|n| n.workshop_id == req.workshop_id) {
                if !node.required_by.contains(&parent) {
                    node.required_by.push(parent.clone());
                }
                continue;
            }
            if nodes.len() >= CHAIN_MAX_ITEMS {
                cut_short = true;
                break;
            }
            nodes.push(RequiredNode {
                workshop_id: req.workshop_id.clone(),
                title: req.title.clone(),
                preview_url: None,
                file_size: 0,
                found: true,
                required_by: vec![parent.clone()],
                depth: depth + 1,
            });
            queue.push_back((req.workshop_id.clone(), req.title.clone(), depth + 1));
        }
    }
    (nodes, cut_short, unread)
}

/// What subscribing to `ids` needs too, with each item's picture, size and whether Steam still
/// has it. Pages are read gently (a short pause between them).
pub fn required_chain(ids: &[String]) -> Result<RequiredChain, String> {
    // Steam's pace, its slow-downs and the retries are the shared limiter's (polite).
    let (mut items, cut_short, unread) = required_chain_with(ids, page_required_items);
    let mut asked: Vec<RequiredItem> = ids.iter().map(|id| RequiredItem { workshop_id: id.clone(), title: id.clone() }).collect();
    if !items.is_empty() {
        let wanted: Vec<String> = items.iter().map(|n| n.workshop_id.clone()).chain(ids.iter().cloned()).collect();
        if let Ok(details) = get_details(&wanted) {
            // The items asked for are named by their title, not their id.
            let title_of = |id: &String| details.iter().find(|d| &d.workshop_id == id && !d.title.is_empty()).map(|d| d.title.clone());
            for a in asked.iter_mut() {
                if let Some(t) = title_of(&a.workshop_id) {
                    a.title = t;
                }
            }
            for node in items.iter_mut() {
                for by in node.required_by.iter_mut() {
                    if ids.contains(by) {
                        if let Some(t) = title_of(by) {
                            *by = t;
                        }
                    }
                }
                if let Some(d) = details.iter().find(|d| d.workshop_id == node.workshop_id) {
                    node.found = d.found && !d.banned;
                    if !d.title.is_empty() {
                        node.title = d.title.clone();
                    }
                    node.preview_url = d.preview_url.clone();
                    node.file_size = d.file_size;
                }
            }
        }
    }
    Ok(RequiredChain { items, cut_short, unread, asked })
}

/// Looks up whatever the user pasted: item links/ids, or one collection link (expanded).
pub fn lookup(input: &str) -> Result<WorkshopLookup, String> {
    let ids = parse_ids(input);
    if ids.is_empty() {
        return Err("No Workshop link or id found. Paste a link like https://steamcommunity.com/sharedfiles/filedetails/?id=2699828474".to_string());
    }
    if ids.len() == 1 {
        if let Some(children) = collection_children(&ids[0])? {
            let collection = get_details(&ids).ok().and_then(|mut v| v.pop());
            return Ok(WorkshopLookup { collection, items: get_details(&children)? });
        }
    }
    Ok(WorkshopLookup { collection: None, items: get_details(&ids)? })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_chain_follows_required_items_down_each_once() {
        // 1 needs 2 and 3; 2 needs 3 and 4; 4 needs 1 (a loop back to what was asked for).
        let page = |id: &str| -> Result<Vec<RequiredItem>, String> {
            let r = |w: &str| RequiredItem { workshop_id: w.to_string(), title: format!("Item {}", w) };
            Ok(match id {
                "1" => vec![r("2"), r("3")],
                "2" => vec![r("3"), r("4")],
                "4" => vec![r("1")],
                _ => vec![],
            })
        };
        let (nodes, cut, unread) = required_chain_with(&["1".to_string()], page);
        assert!(!cut && unread.is_empty());
        let got: Vec<(String, u32, Vec<String>)> = nodes.iter().map(|n| (n.workshop_id.clone(), n.depth, n.required_by.clone())).collect();
        assert_eq!(
            got,
            vec![
                ("2".to_string(), 1, vec!["1".to_string()]),
                ("3".to_string(), 1, vec!["1".to_string(), "Item 2".to_string()]),
                ("4".to_string(), 2, vec!["Item 2".to_string()]),
            ]
        );
        // Nothing listed: nothing to add.
        assert!(required_chain_with(&["9".to_string()], page).0.is_empty());
        // A page that cannot be read is named, and the rest is still followed.
        let flaky = |id: &str| -> Result<Vec<RequiredItem>, String> {
            if id == "2" { Err("429".into()) } else { page(id) }
        };
        let (nodes, _, unread) = required_chain_with(&["1".to_string()], flaky);
        assert_eq!(unread, vec!["2".to_string()]);
        assert_eq!(nodes.iter().map(|n| n.workshop_id.as_str()).collect::<Vec<_>>(), vec!["2", "3"]);
    }

    /// Live: `cargo test -- --ignored live_required_chain --nocapture` (reads Steam).
    #[test]
    #[ignore]
    fn live_required_chain() {
        let chain = required_chain(&["3490188370".to_string(), "3502080466".to_string()]).unwrap();
        for n in &chain.items {
            println!("{} {:?} depth {} by {:?} size {} found {}", n.workshop_id, n.title, n.depth, n.required_by, n.file_size, n.found);
        }
        println!("unread {:?}", chain.unread);
        assert!(!chain.items.is_empty() || !chain.unread.is_empty());
    }

    #[test]
    fn a_chain_that_never_ends_is_cut_short() {
        let page = |id: &str| -> Result<Vec<RequiredItem>, String> {
            let next = (id.parse::<u32>().unwrap() + 1).to_string();
            Ok(vec![RequiredItem { title: next.clone(), workshop_id: next }])
        };
        let (nodes, cut, _) = required_chain_with(&["1".to_string()], page);
        assert!(cut);
        assert_eq!(nodes.len() as u32, CHAIN_MAX_DEPTH);
    }

    #[test]
    fn finds_ids_in_pasted_text() {
        let text = "https://steamcommunity.com/sharedfiles/filedetails/?id=2699828474&searchtext=x \
                    steam://url/CommunityFilePage/3437629766, 2699828474 ; nope 12";
        assert_eq!(parse_ids(text), vec!["2699828474", "3437629766"]);
    }

    #[test]
    fn parses_details_and_build_tags() {
        let v = json!({
            "publishedfileid": "3437629766", "result": 1, "consumer_app_id": 108600, "title": "CleanUI [B42]",
            "description": "[h2]Project Zomboid Build 42.12.3[/h2]\r\nClean UI", "time_updated": 1791810633,
            "subscriptions": 859173, "favorited": 24247, "file_size": "13119426",
            "tags": [{ "tag": "Build 42" }, { "tag": "Interface" }]
        });
        let item = parse_item(&v).unwrap();
        assert!(item.found && item.is_project_zomboid);
        assert_eq!(item.builds, vec!["42"]);
        assert_eq!(item.file_size, 13_119_426);
        assert_eq!(item.summary, "Project Zomboid Build 42.12.3 Clean UI");
        assert_eq!(item.page_url, "https://steamcommunity.com/sharedfiles/filedetails/?id=3437629766");
        assert!(!parse_item(&json!({ "publishedfileid": "1", "result": 9 })).unwrap().found);
        // A browse result has no "result" field, and is there because it exists.
        assert!(parse_item(&json!({ "publishedfileid": "3", "title": "From the browse page" })).unwrap().found);
        assert!(!parse_item(&json!({ "publishedfileid": "4" })).unwrap().found, "nothing at all is not an item");
        // QueryFiles field names.
        let q = parse_item(&json!({ "publishedfileid": "2", "result": 1, "consumer_appid": 108600,
            "short_description": "Short [b]text[/b]", "tags": [{ "tag": "Build 41", "display_name": "Build 41" }] })).unwrap();
        assert!(q.is_project_zomboid);
        assert_eq!(q.summary, "Short text");
        assert_eq!(q.builds, vec!["41"]);
    }

    #[test]
    fn reads_install_state_from_the_acf() {
        let root = std::env::temp_dir().join(format!("pzms-acf-{}", std::process::id()));
        let content = root.join("steamapps/workshop/content/108600");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(root.join("steamapps/workshop/appworkshop_108600.acf"), r#""AppWorkshop"
{
	"TimeLastAppRan"		"2000"
	"WorkshopItemsInstalled"
	{
		"1" { "size" "10" "timeupdated" "1500" "manifest" "a" }
		"2" { "size" "20" "timeupdated" "2500" "manifest" "b" }
		"3" { "size" "30" "timeupdated" "1000" "manifest" "c" }
	}
	"WorkshopItemDetails"
	{
		"1" { "manifest" "a" "timeupdated" "1500" "latest_timeupdated" "1500" "latest_manifest" "a" }
		"2" { "manifest" "b" "timeupdated" "2500" "latest_timeupdated" "2500" "latest_manifest" "b" }
		"3" { "manifest" "c" "timeupdated" "1000" "latest_timeupdated" "3000" "latest_manifest" "d" }
	}
}"#).unwrap();
        let state = local_state(&content.to_string_lossy()).unwrap();
        assert_eq!(state.time_last_app_ran, Some(2000));
        assert!(!state.items["1"].update_pending && !state.items["1"].updated_since_last_play);
        assert!(state.items["2"].updated_since_last_play);
        assert!(state.items["3"].update_pending);
        assert_eq!(state.items["3"].latest_time, Some(3000));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Live Steam call with a deliberately invalid key: must come back as a clear key error.
    #[test]
    #[ignore]
    fn workshop_live_search_rejects_bad_key() {
        let err = search("00000000000000000000000000000000", Some("horror"), WorkshopSort::Relevance, Some("42"), Some("Map"), 1, 5).unwrap_err();
        println!("{}", err);
        assert!(err.contains("rejected the Web API key"), "{}", err);
    }

    #[test]
    fn reads_the_public_item_page() {
        let html = r#"<div class="friendBlockContent">Some &amp; Author<br><span>Offline</span></div>
            <div class="highlight_strip_item highlight_strip_movie"><img src="https://img.youtube.com/vi/aSo4f_GBvV8/default.jpg"></div>
            <div role="button" class="highlight_strip_item highlight_strip_screenshot" id="thumb_screenshot_1" >
                <img src="https://images.steamusercontent.com/ugc/1/AB/?imw=116&imh=65&ima=fit"></div>
            <div class="requiredItemsContainer" id="RequiredItems">
                <a href="https://steamcommunity.com/workshop/filedetails/?id=2447729538" target="_blank" data-subscribed="0">
                    <div class="requiredItem">
                        Fluffy Hair [B41/B42]                </div>
                </a>
                <a href="https://steamcommunity.com/workshop/filedetails/?id=3041733782" target="_blank">
                    <div class="requiredItem">Spongie&#39;s Unlocker</div>
                </a>
            </div>
            <a href="https://steamcommunity.com/workshop/filedetails/?id=999"><div class="requiredItem">Not required</div></a>"#;
        let p = parse_item_page(html);
        assert_eq!(p.author.as_deref(), Some("Some & Author"));
        assert_eq!(p.screenshots, vec!["https://images.steamusercontent.com/ugc/1/AB/"]);
        assert_eq!(p.videos, vec!["aSo4f_GBvV8"]);
        assert_eq!(
            p.required_items,
            vec![
                RequiredItem { workshop_id: "2447729538".into(), title: "Fluffy Hair [B41/B42]".into() },
                RequiredItem { workshop_id: "3041733782".into(), title: "Spongie's Unlocker".into() },
            ]
        );
    }

    #[test]
    fn reads_the_browse_page_state() {
        let html = r#"<html><script type="application/json">not json</script>
            <script type="application/json">{"renderContext":{"queryData":"{\"queries\":[{\"state\":{\"data\":{\"total_count\":903,\"total_pages\":31,\"current_page\":2,\"results\":[{\"publishedfileid\":\"7\",\"result\":1,\"consumer_appid\":108600,\"title\":\"A map\",\"short_description\":\"Hi\",\"tags\":[{\"tag\":\"Build 42\"},{\"tag\":\"Map\"}],\"subscriptions\":18}]}}}]}"}}</script></html>"#;
        let results = find_results(&page_state(html).unwrap(), 0).unwrap();
        assert_eq!(results.get("total_count").and_then(|v| v.as_u64()), Some(903));
        let items: Vec<WorkshopItem> = results.get("results").unwrap().as_array().unwrap().iter().filter_map(parse_item).collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "A map");
        assert_eq!(items[0].builds, vec!["42"]);
        assert!(items[0].found && items[0].is_project_zomboid);
        assert!(page_state("<html>no state</html>").is_none());
    }

    #[test]
    fn browse_tags_follow_the_workshop_spelling() {
        assert_eq!(tag_filters(Some("42"), Some("qol")), vec!["Build 42".to_string(), "QoL".to_string()]);
        assert_eq!(tag_filters(Some(""), Some("not a category")), Vec::<String>::new());
    }

    /// Live Steam call, no key. cargo test workshop_live_public -- --ignored --nocapture
    #[test]
    #[ignore]
    fn workshop_live_public_search() {
        let b42 = |page: u32| BrowseQuery { build: Some("42".into()), page, per_page: 30, ..Default::default() };
        let trending = search_public(&b42(1)).unwrap();
        println!("trending: {} items of {} ({} pages)", trending.items.len(), trending.total, trending.pages);
        assert!(trending.total > 1000 && trending.items.len() == 30);
        assert!(trending.items.iter().all(|i| i.builds.contains(&"42".to_string())));
        assert!(trending.items.iter().all(|i| i.found), "browse results must count as found, or the cards do not open");
        let page3 = search_public(&b42(3)).unwrap();
        assert_eq!(page3.page, 3);
        assert!(page3.items[0].workshop_id != trending.items[0].workshop_id);
        let today = search_public(&BrowseQuery { days: Some(1), ..b42(1) }).unwrap();
        assert!(today.items[0].workshop_id != trending.items[0].workshop_id, "the time window changes the ranking");
        let cars = search_public(&BrowseQuery { text: Some("car".into()), category: Some("Vehicles".into()), ..b42(1) }).unwrap();
        println!("car/Vehicles: {} of {}: {:?}", cars.items.len(), cars.total, cars.items.iter().take(3).map(|i| &i.title).collect::<Vec<_>>());
        assert!(cars.total > 0 && cars.total < trending.total);
        let none = search_public(&BrowseQuery { text: Some("zzzzqqqxxnotamod".into()), ..b42(1) }).unwrap();
        assert!(none.items.is_empty());
    }

    /// Live Steam call, no key. cargo test workshop_live_collections -- --ignored --nocapture
    #[test]
    #[ignore]
    fn workshop_live_collections() {
        let page = search_public(&BrowseQuery { collections: true, page: 1, per_page: 30, ..Default::default() }).unwrap();
        let first = &page.items[0];
        println!("{} collections; first: {:?} with {} items", page.total, first.title, first.children.len());
        assert!(page.items.iter().all(|i| i.is_collection));
        assert!(!first.children.is_empty(), "a collection browse result carries its items");
    }

    /// Live Steam call. cargo test workshop_live_details -- --ignored --nocapture
    #[test]
    #[ignore]
    fn workshop_live_details() {
        let d = item_details("2463184726").unwrap();
        println!("{} by {:?}: {} screenshots, {} videos, required {:?}", d.item.title, d.author, d.screenshots.len(), d.videos.len(), d.required_items);
        assert!(!d.description.is_empty());
        assert!(!d.required_items.is_empty());
    }

    /// Live Steam call. cargo test workshop_live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn workshop_live_lookup() {
        let r = lookup("https://steamcommunity.com/sharedfiles/filedetails/?id=2699828474").unwrap();
        assert!(r.collection.is_none());
        println!("{:?}", r.items.iter().map(|i| (&i.title, &i.builds)).collect::<Vec<_>>());
        assert_eq!(r.items[0].title, "Rebalanced Prop Moving");
    }
}

#[cfg(test)]
mod live_collection_tests {
    use super::*;

    /// Browses real collections and reads one, which is what "Make a profile from this" needs.
    /// PZMS_WORKSHOP_DIR=... cargo test a_real_collection_lists_its_mods -- --ignored --nocapture
    #[test]
    #[ignore]
    fn a_real_collection_lists_its_mods() {
        let q = BrowseQuery {
            text: None,
            sort: Some(WorkshopSort::MostSubscribed),
            build: None,
            category: None,
            days: None,
            collections: true,
            page: 1,
            per_page: 30,
        };
        let found = match search_public(&q) {
            Ok(r) => r,
            Err(e) => {
                println!("browsing collections failed: {}", e);
                return;
            }
        };
        println!("{} collections, page 1 has {}", found.total, found.items.len());
        let Some(first) = found.items.iter().find(|i| !i.children.is_empty()) else {
            println!("no collection on this page carries its children");
            return;
        };
        println!("\"{}\" ({}) holds {} items", first.title, first.workshop_id, first.children.len());

        let read = lookup(&page_url(&first.workshop_id)).expect("the collection reads");
        println!("read back: {} items", read.items.len());
        assert_eq!(read.items.len(), first.children.len());

        if let Ok(dir) = std::env::var("PZMS_WORKSHOP_DIR") {
            let installed: std::collections::HashSet<String> = std::fs::read_dir(&dir)
                .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
                .unwrap_or_default();
            let have = read.items.iter().filter(|i| installed.contains(&i.workshop_id)).count();
            println!("installed here: {}, left out of a profile: {}", have, read.items.len() - have);

            // What the profile dialog would say for each collection on this page.
            let mut best = (0usize, String::new(), 0usize);
            for c in &found.items {
                let mine = c.children.iter().filter(|id| installed.contains(*id)).count();
                if mine > best.0 {
                    best = (mine, c.title.clone(), c.children.len());
                }
            }
            if best.0 > 0 {
                println!(
                    "best overlap on this page: \"{}\" would make a profile of {} mod(s), leaving out {}",
                    best.1,
                    best.0,
                    best.2 - best.0
                );
            } else {
                println!("none of this page's collections overlaps what is installed here");
            }
        }
    }
}

#[cfg(test)]
mod live_support {
    /// The support links of every installed Workshop item, from Steam. cargo test --lib installed_support_links -- --ignored --nocapture
    #[test]
    #[ignore]
    fn installed_support_links() {
        let dir = r"D:\Apps\Steam\steamapps\workshop\content\108600";
        let ids: Vec<String> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect();
        let items = super::get_details(&ids).unwrap();
        let with: Vec<_> = items.iter().filter(|i| !i.support.is_empty()).collect();
        for i in &with {
            println!("{:<45} {}", i.title.chars().take(45).collect::<String>(), i.support.iter().map(|l| format!("{} {}", l.name, l.url)).collect::<Vec<_>>().join(" | "));
        }
        println!("{} of {} installed items have support links", with.len(), items.len());
    }
}