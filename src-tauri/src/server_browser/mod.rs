//! Servers to join: the public list Steam keeps, the player's own (private) servers, what one
//! server says about itself, and the launch that joins it.
//!
//! The game itself (42.x, read in its code): a dedicated server publishes to Steam its name,
//! map, players, and the key-values `description`, `version`, `open`, `mods` and `lastWipe`;
//! the game's server browser reads them with the Source query `A2S_RULES`. It joins with
//! `ProjectZomboid64.exe +connect <ip:port> [+password <password>]` (MainScreenState reads
//! `+connect` and `+password`). On join the server sends its Workshop items and the game
//! downloads the missing ones; the player's own mod list is left as it was.
//!
//! The public list comes from the Steam Web API (`IGameServersService/GetServerList`), which
//! needs the player's Web API key; the answer is never kept on disk (the request carries it).

use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::path::Path;
use std::time::{Duration, Instant};

pub const APP_ID: u32 = 108600;
const LIST_URL: &str = "https://api.steampowered.com/IGameServersService/GetServerList/v1/";
const SAVED_FILE: &str = "PZModStudio_Servers.json";

/// One server in Steam's list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicServer {
    pub name: String,
    /// ip:port the game connects to.
    pub address: String,
    /// ip:port that answers queries (A2S).
    pub query_address: String,
    pub players: u32,
    pub max_players: u32,
    pub map: String,
    pub version: String,
    pub dedicated: bool,
    pub secure: bool,
    /// Steam's tags for it, as published.
    pub tags: String,
    /// Whether it asks for a password (the Steam client's list says; the Web API does not).
    #[serde(default)]
    pub password: Option<bool>,
    #[serde(default)]
    pub ping_ms: Option<u32>,
}

/// What Steam's list answer looks like (only what is used).
#[derive(Deserialize)]
struct ListAnswer {
    response: ListResponse,
}
#[derive(Deserialize, Default)]
struct ListResponse {
    #[serde(default)]
    servers: Vec<RawServer>,
}
#[derive(Deserialize)]
struct RawServer {
    addr: String,
    #[serde(default)]
    gameport: u16,
    #[serde(default)]
    name: String,
    #[serde(default)]
    players: u32,
    #[serde(default)]
    max_players: u32,
    #[serde(default)]
    map: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    dedicated: bool,
    #[serde(default)]
    secure: bool,
    #[serde(default)]
    gametype: String,
}

pub fn parse_list(body: &str) -> Result<Vec<PublicServer>, String> {
    let answer: ListAnswer = serde_json::from_str(body).map_err(|e| format!("Steam's server list could not be read: {}", e))?;
    Ok(answer
        .response
        .servers
        .into_iter()
        .map(|s| {
            let ip = s.addr.rsplit_once(':').map(|(ip, _)| ip.to_string()).unwrap_or_else(|| s.addr.clone());
            let port = if s.gameport > 0 { s.gameport.to_string() } else { s.addr.rsplit_once(':').map(|(_, p)| p.to_string()).unwrap_or_default() };
            PublicServer {
                name: s.name.trim().to_string(),
                address: format!("{}:{}", ip, port),
                query_address: s.addr,
                players: s.players,
                max_players: s.max_players,
                map: s.map,
                version: if build_of_tags(&s.gametype).is_empty() { s.version } else { build_of_tags(&s.gametype) },
                dedicated: s.dedicated,
                secure: s.secure,
                tags: s.gametype,
                password: None,
                ping_ms: None,
            }
        })
        .collect())
}

/// Steam's list of Project Zomboid servers, most players first. `search` narrows it by name on
/// Steam's side.
pub fn public_servers(key: &str, search: &str, limit: u32) -> Result<Vec<PublicServer>, String> {
    let mut filter = format!("\\appid\\{}", APP_ID);
    let search = search.trim();
    if !search.is_empty() {
        // Steam's own name filter: * matches anything.
        filter.push_str(&format!("\\name_match\\*{}*", search.replace('\\', "")));
    }
    let query = vec![("key".to_string(), key.to_string()), ("filter".to_string(), filter), ("limit".to_string(), limit.clamp(1, 20000).to_string())];
    let body = crate::workshop::polite::fetch(crate::workshop::polite::Request::Get { url: LIST_URL, query: &query }, Duration::ZERO, None).map_err(|r| match r {
        crate::workshop::polite::Refusal::Busy(s) => format!("Steam is limiting requests; try again in {} s.", s),
        crate::workshop::polite::Refusal::Status(403) => "Steam refused the Web API key (403). Check it in Settings.".to_string(),
        crate::workshop::polite::Refusal::Status(s) => format!("Steam answered {}.", s),
        crate::workshop::polite::Refusal::Network(e) => format!("Steam could not be reached: {}", e),
    })?;
    let mut list = parse_list(&body)?;
    list.sort_by(|a, b| b.players.cmp(&a.players).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(list)
}

/// The build a server says it runs, from its Steam tags (`...;VERSION:42.21`).
pub fn build_of_tags(tags: &str) -> String {
    tags.split(';').find_map(|t| t.trim().strip_prefix("VERSION:").map(|v| v.trim().to_string())).unwrap_or_default()
}

/// The list the running Steam client gave, in the browser's shape, narrowed by `search`.
pub fn from_client_list(list: Vec<crate::steam_client::ListedServer>, search: &str) -> Vec<PublicServer> {
    let search = search.trim().to_lowercase();
    let mut out: Vec<PublicServer> = list
        .into_iter()
        .filter(|s| search.is_empty() || s.name.to_lowercase().contains(&search))
        .map(|s| PublicServer {
            address: format!("{}:{}", s.ip, s.game_port),
            query_address: format!("{}:{}", s.ip, if s.query_port > 0 { s.query_port } else { s.game_port }),
            players: s.players.max(0) as u32,
            max_players: s.max_players.max(0) as u32,
            version: build_of_tags(&s.tags),
            dedicated: true,
            secure: s.secure,
            password: Some(s.password),
            ping_ms: (s.ping > 0).then_some(s.ping as u32),
            name: s.name,
            map: s.map,
            tags: s.tags,
        })
        .collect();
    out.sort_by(|a, b| b.players.cmp(&a.players).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

/// What one server says about itself (A2S_RULES), and how long it took to answer.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct ServerRules {
    pub description: String,
    pub version: String,
    pub open: bool,
    /// Mod ids, in the server's order.
    pub mods: Vec<String>,
    /// How many mods the server has (its `mods` value holds only the first ~127 characters).
    pub mod_count: Option<u32>,
    pub pvp: Option<bool>,
    /// Unix seconds of the last wipe, when the server says.
    pub last_wipe: Option<i64>,
    pub ping_ms: u64,
    /// Every key-value as sent, for what is not read above.
    pub raw: Vec<(String, String)>,
    /// The address that answered.
    pub query_address: String,
}

/// The mod ids of the `mods` value: separated by ';' or ',', with or without the leading '\'
/// B42 mod ids may carry.
pub fn split_mods(value: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in value.split([';', ',']) {
        let id = part.trim().trim_start_matches('\\').trim();
        if !id.is_empty() && !out.iter().any(|x| x == id) {
            out.push(id.to_string());
        }
    }
    out
}

pub fn rules_from_pairs(pairs: Vec<(String, String)>, ping_ms: u64, query_address: String) -> ServerRules {
    let get = |k: &str| pairs.iter().find(|(key, _)| key.eq_ignore_ascii_case(k)).map(|(_, v)| v.clone());
    // A long list may come split over numbered keys (mods, mods1, mods2...): joined in order.
    // `mods` (or mods:1/N, mods:2/N...) holds the first ~127 characters of the server's Mods=
    // line: the last name may be cut, and is left out when the value is full.
    let mut mods_value = get("mods").unwrap_or_default();
    let mut chunks: Vec<(u32, String)> = pairs
        .iter()
        .filter_map(|(k, v)| k.strip_prefix("mods:").and_then(|r| r.split_once('/')).and_then(|(n, _)| n.parse().ok()).map(|n: u32| (n, v.clone())))
        .collect();
    chunks.sort_by_key(|c| c.0);
    if !chunks.is_empty() {
        mods_value = chunks.into_iter().map(|c| c.1).collect();
    }
    let looks_cut = mods_value.len() >= 118 && !mods_value.ends_with(';');
    // A long description comes as description:1/3, description:2/3...: the longest complete set.
    let mut description = get("description").unwrap_or_default();
    let mut sets: std::collections::BTreeMap<u32, Vec<(u32, String)>> = std::collections::BTreeMap::new();
    for (k, v) in &pairs {
        if let Some(rest) = k.strip_prefix("description:") {
            if let Some((n, m)) = rest.split_once('/') {
                if let (Ok(n), Ok(m)) = (n.parse::<u32>(), m.parse::<u32>()) {
                    sets.entry(m).or_default().push((n, v.clone()));
                }
            }
        }
    }
    for (total, mut parts) in sets {
        parts.sort_by_key(|p| p.0);
        parts.dedup_by_key(|p| p.0);
        if parts.len() as u32 == total {
            let joined: String = parts.into_iter().map(|p| p.1).collect();
            if joined.len() > description.len() {
                description = joined;
            }
        }
    }
    let mut mods = split_mods(&mods_value);
    let mod_count: Option<u32> = get("modCount").and_then(|v| v.trim().parse().ok());
    if looks_cut && mod_count.map(|n| n as usize > mods.len()).unwrap_or(true) {
        mods.pop();
    }
    ServerRules {
        description: clean_description(&description),
        mod_count,
        pvp: get("pvp").map(|v| v == "1" || v.eq_ignore_ascii_case("true")),
        version: get("version").unwrap_or_default(),
        open: get("open").map(|v| v == "1").unwrap_or(false),
        mods,
        last_wipe: get("lastWipe").and_then(|v| v.trim().parse().ok()),
        ping_ms,
        raw: pairs,
        query_address,
    }
}

/// The description as text: the game's `<LINE>` and `\n` as new lines, its `<RGB:...>` and other
/// tags left out.
pub fn clean_description(text: &str) -> String {
    let text = text.replace("\\n", "\n").replace("<LINE>", "\n").replace("<BR>", "\n");
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        match rest[open..].find('>') {
            Some(close) if close < 40 && rest[open + 1..open + close].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || ":.,_ -".contains(c)) => rest = &rest[open + close + 1..],
            _ => {
                out.push('<');
                rest = &rest[open + 1..];
            }
        }
    }
    out.push_str(rest);
    out.lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim().to_string()
}

/// Reads a nul-terminated string at `*at`.
fn cstr(buf: &[u8], at: &mut usize) -> Option<String> {
    let start = *at;
    let end = start + buf.get(start..)?.iter().position(|b| *b == 0)?;
    *at = end + 1;
    Some(String::from_utf8_lossy(&buf[start..end]).to_string())
}

/// The key-values of an A2S_RULES answer (after the 0xFFFFFFFF header).
pub fn parse_rules_payload(p: &[u8]) -> Option<Vec<(String, String)>> {
    if p.first() != Some(&0x45) || p.len() < 3 {
        return None;
    }
    let count = u16::from_le_bytes([p[1], p[2]]) as usize;
    let mut at = 3;
    let mut out = Vec::new();
    for _ in 0..count {
        let (Some(k), Some(v)) = (cstr(p, &mut at), cstr(p, &mut at)) else { break };
        out.push((k, v));
    }
    Some(out)
}

/// One answer, put together when it came in several packets (Source split format).
fn receive(sock: &UdpSocket) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; 65536];
    let n = sock.recv(&mut buf).map_err(|e| e.to_string())?;
    let first = &buf[..n];
    if first.len() >= 4 && first[..4] == [0xFF, 0xFF, 0xFF, 0xFF] {
        return Ok(first[4..].to_vec());
    }
    if first.len() < 12 || first[..4] != [0xFE, 0xFF, 0xFF, 0xFF] {
        return Err("The server answered something that is not a Steam query answer.".to_string());
    }
    let id = i32::from_le_bytes([first[4], first[5], first[6], first[7]]);
    if id < 0 {
        return Err("The server sent a compressed answer, which is not read.".to_string());
    }
    let total = first[8] as usize;
    let mut parts: Vec<Option<Vec<u8>>> = vec![None; total.max(1)];
    let put = |parts: &mut Vec<Option<Vec<u8>>>, pkt: &[u8]| {
        if pkt.len() >= 12 && pkt[..4] == [0xFE, 0xFF, 0xFF, 0xFF] && i32::from_le_bytes([pkt[4], pkt[5], pkt[6], pkt[7]]) == id {
            let number = pkt[9] as usize;
            if number < parts.len() {
                parts[number] = Some(pkt[12..].to_vec());
            }
        }
    };
    let first = first.to_vec();
    put(&mut parts, &first);
    while parts.iter().any(Option::is_none) {
        let n = sock.recv(&mut buf).map_err(|e| e.to_string())?;
        put(&mut parts, &buf[..n]);
    }
    let joined: Vec<u8> = parts.into_iter().flatten().flatten().collect();
    if joined.len() >= 4 && joined[..4] == [0xFF, 0xFF, 0xFF, 0xFF] {
        Ok(joined[4..].to_vec())
    } else {
        Ok(joined)
    }
}

fn query_rules_at(addr: SocketAddr, timeout: Duration) -> Result<ServerRules, String> {
    let sock = UdpSocket::bind(if addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" }).map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(timeout)).map_err(|e| e.to_string())?;
    sock.connect(addr).map_err(|e| e.to_string())?;
    let mut challenge = [0xFF, 0xFF, 0xFF, 0xFF];
    for _ in 0..3 {
        let mut req = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x56];
        req.extend_from_slice(&challenge);
        sock.send(&req).map_err(|e| e.to_string())?;
        let sent = Instant::now();
        let answer = receive(&sock).map_err(|e| if e.contains("10060") || e.contains("timed out") || e.contains("WouldBlock") { "The server did not answer.".to_string() } else { e })?;
        match answer.first() {
            Some(0x41) if answer.len() >= 5 => challenge = [answer[1], answer[2], answer[3], answer[4]],
            Some(0x45) => {
                let pairs = parse_rules_payload(&answer).ok_or("The server's rules could not be read.")?;
                return Ok(rules_from_pairs(pairs, sent.elapsed().as_millis() as u64, addr.to_string()));
            }
            _ => return Err("The server's answer could not be read.".to_string()),
        }
    }
    Err("The server kept asking for a new challenge.".to_string())
}

/// The ports a server may answer queries on, for an address typed by hand: the one given, then
/// the game's defaults (16261, and the Steam ports 8766/8767 of older servers).
pub fn query_candidates(address: &str) -> Vec<String> {
    let address = address.trim();
    let (host, port) = match address.rsplit_once(':') {
        Some((h, p)) if p.parse::<u16>().is_ok() && !h.is_empty() => (h.trim_matches(['[', ']']).to_string(), Some(p.to_string())),
        _ => (address.trim_matches(['[', ']']).to_string(), None),
    };
    let host = if host.contains(':') { format!("[{}]", host) } else { host };
    let mut ports: Vec<String> = port.into_iter().collect();
    for p in ["16261", "8766", "8767", "27015"] {
        if !ports.iter().any(|x| x == p) {
            ports.push(p.to_string());
        }
    }
    ports.into_iter().map(|p| format!("{}:{}", host, p)).collect()
}

/// What the server at `address` (ip or name, with or without a port) says about itself.
pub fn server_rules(address: &str) -> Result<ServerRules, String> {
    let mut last = "The server did not answer.".to_string();
    for candidate in query_candidates(address) {
        let addrs: Vec<SocketAddr> = match candidate.to_socket_addrs() {
            Ok(a) => a.collect(),
            Err(e) => return Err(format!("{} could not be found: {}", address, e)),
        };
        for addr in addrs.into_iter().take(2) {
            match query_rules_at(addr, Duration::from_millis(1500)) {
                Ok(r) => return Ok(r),
                Err(e) => last = e,
            }
        }
    }
    Err(last)
}

/// One server the player keeps (private ones too). Its password is not here: it is kept in
/// Windows' credential store under `server_password:<address>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedServer {
    pub name: String,
    pub address: String,
    #[serde(default)]
    pub note: String,
    /// The profile active when the Studio last joined it: what the server loads goes into it.
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub has_password: bool,
    /// Kept on purpose (a star); otherwise it is only in the recent list.
    #[serde(default)]
    pub starred: bool,
    /// When the Studio last started the game joining it (Unix seconds).
    #[serde(default)]
    pub last_joined: Option<i64>,
    /// The mods the game loaded on it last time, in its order, from that session's log.
    #[serde(default)]
    pub mods: Vec<String>,
    /// The Workshop items it made the game check or download, from the same log.
    #[serde(default)]
    pub items: Vec<String>,
    /// When `mods` / `items` were read (Unix seconds).
    #[serde(default)]
    pub mods_seen: Option<i64>,
    /// The player's account on it. The game's connect window asks for it every time unless the
    /// game's own server list holds it; the Studio Link fills it in from here.
    #[serde(default)]
    pub username: String,
    /// The profile made from it with "Make a profile" (shown in its pane).
    #[serde(default)]
    pub made_profile_id: Option<String>,
}

/// The last join in a DebugLog: the Workshop items the server named
/// (`ConnectToServerState: WorkshopConfirm ... ID=<n>`) and the mods loaded after it
/// (`Mod ... > loading <id>.`), both in order. None when the log has no join.
pub fn parse_join_log(text: &str) -> Option<(Vec<String>, Vec<String>)> {
    let start = text.rfind("ConnectToServerState: Start.")?;
    let block = &text[start..];
    let mut items: Vec<String> = Vec::new();
    let mut mods: Vec<String> = Vec::new();
    for line in block.lines() {
        if let Some(at) = line.find("ConnectToServerState: WorkshopConfirm") {
            if let Some(id) = line[at..].split("ID=").nth(1) {
                let id: String = id.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !id.is_empty() && !items.contains(&id) {
                    items.push(id);
                }
            }
        } else if line.contains(" Mod ") || line.contains("Mod          ") {
            if let Some(rest) = line.split("> loading ").nth(1) {
                let id = rest.trim().trim_end_matches('.').trim().to_string();
                // "loading translation mod \"X\"" is the same mod's translations, not another mod.
                if !id.is_empty() && !id.starts_with("translation mod") && !mods.contains(&id) {
                    mods.push(id);
                }
            }
        }
    }
    (!items.is_empty() || !mods.is_empty()).then_some((items, mods))
}

/// The DebugLogs written at or after `since` (Unix seconds), oldest first.
fn debug_logs_since(user_zomboid_dir: &Path, since: i64) -> Vec<std::path::PathBuf> {
    let logs = user_zomboid_dir.join("Logs");
    let mut files: Vec<(i64, std::path::PathBuf)> = Vec::new();
    let mut look = |dir: &Path| {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_file() && p.file_name().map(|n| n.to_string_lossy().ends_with("_DebugLog.txt")).unwrap_or(false) {
                let at = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
                if at >= since {
                    files.push((at, p));
                }
            }
        }
    };
    look(&logs);
    for e in std::fs::read_dir(&logs).into_iter().flatten().flatten() {
        if e.path().is_dir() {
            look(&e.path());
        }
    }
    files.sort();
    files.into_iter().map(|(_, p)| p).collect()
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Fills in what each joined server loaded, from the logs written since the Studio joined it.
/// Returns whether anything changed (the list is then written back).
pub fn fill_from_logs(user_zomboid_dir: &Path, list: &mut [SavedServer]) -> bool {
    let mut changed = false;
    for s in list.iter_mut() {
        let Some(joined) = s.last_joined else { continue };
        if s.mods_seen.map(|seen| seen >= joined).unwrap_or(false) {
            continue;
        }
        // The first log of that session with a join in it.
        for file in debug_logs_since(user_zomboid_dir, joined - 5) {
            let Ok(bytes) = std::fs::read(&file) else { continue };
            // A join still connecting has no mods loaded yet: looked at again next time.
            if let Some((items, mods)) = parse_join_log(&String::from_utf8_lossy(&bytes)).filter(|(_, m)| !m.is_empty()) {
                s.items = items;
                s.mods = mods;
                s.mods_seen = Some(now_secs());
                changed = true;
                break;
            }
        }
    }
    changed
}

/// What the last joins loaded, from the logs; the profile each was joined with takes what the
/// server loaded, so the mods the game downloaded for it are in it. Returns the kept servers and
/// the profiles that changed.
pub fn refresh_joins(user_zomboid_dir: &Path) -> (Vec<SavedServer>, Vec<crate::instance_manager::AppInstance>) {
    let mut list = read_saved(user_zomboid_dir);
    let before: Vec<Option<i64>> = list.iter().map(|s| s.mods_seen).collect();
    let mut changed = Vec::new();
    if fill_from_logs(user_zomboid_dir, &mut list) {
        let _ = write_saved(user_zomboid_dir, &list);
        let mut catalog: Option<crate::catalog::ModCatalog> = None;
        for (s, seen) in list.iter().zip(before) {
            if s.mods_seen == seen || s.mods.is_empty() {
                continue;
            }
            if let Some(profile) = s.profile_id.as_deref() {
                // The mods of the server's Workshop items it did not switch on: they came with it.
                let user = user_zomboid_dir.to_string_lossy().to_string();
                let catalog = catalog.get_or_insert_with(|| crate::catalog::build_catalog(&crate::vfs::studio_paths().workshop_dir, &user, None));
                let came_with: Vec<String> = catalog
                    .mods
                    .iter()
                    .filter(|m| matches!(&m.source, crate::catalog::ModSource::Workshop { workshop_id } if s.items.contains(workshop_id)))
                    .map(|m| m.id.clone())
                    .collect();
                // What runs only on the player's side stays as the player had it.
                let own_side: Vec<String> = catalog.mods.iter().filter(|m| m.shadowed_by.is_none() && client_side_only(m)).map(|m| m.id.clone()).collect();
                if let Ok(Some(p)) = crate::instance_manager::take_server_mods(&user, profile, &s.mods, &came_with, &own_side) {
                    changed.push(p);
                }
            }
        }
    }
    (list, changed)
}

/// The profile just made from a server, kept with it (the server is kept too, as a recent one,
/// when it was not yet).
pub fn set_made_profile(user_zomboid_dir: &Path, address: &str, name: &str, profile_id: &str) -> Result<Vec<SavedServer>, String> {
    let mut list = read_saved(user_zomboid_dir);
    match list.iter_mut().find(|s| s.address.eq_ignore_ascii_case(address.trim())) {
        Some(s) => s.made_profile_id = Some(profile_id.to_string()),
        None => list.push(SavedServer {
            name: if name.trim().is_empty() { address.trim().to_string() } else { name.trim().to_string() },
            address: address.trim().to_string(),
            note: String::new(),
            profile_id: None,
            has_password: crate::secrets::get(&password_key(address)).ok().flatten().is_some(),
            starred: false,
            last_joined: None,
            mods: Vec::new(),
            items: Vec::new(),
            mods_seen: None,
            username: String::new(),
            made_profile_id: Some(profile_id.to_string()),
        }),
    }
    write_saved(user_zomboid_dir, &list)?;
    Ok(list)
}

/// A mod that runs only on the player's side, so a server neither lists nor needs it: ZombieBuddy
/// (it hooks into the game itself), and Java mods whose jar is client-only
/// (`javaJarFile=media/java/client/...`, which ZombieBuddy skips on a server).
fn client_side_only(m: &crate::catalog::CatalogMod) -> bool {
    if m.id == "ZombieBuddy" {
        return true;
    }
    // The mod.info the game reads, and the shared one in common/ (where Viewpoint declares it).
    let root = std::path::Path::new(&m.root_dir);
    let candidates = [m.mod_info_path.as_ref().map(std::path::PathBuf::from), Some(root.join("common").join("mod.info")), Some(root.join("mod.info"))];
    let info: String = candidates.iter().flatten().filter_map(|p| std::fs::read_to_string(p).ok()).collect::<Vec<_>>().join("
");
    info.lines().any(|l| {
        let l = l.trim();
        l.to_ascii_lowercase().starts_with("javajarfile=") && l.replace('\\', "/").contains("java/client/")
    })
}

/// Read the kept servers, with what the last joins loaded filled in from the logs.
pub fn read_saved_filled(user_zomboid_dir: &Path) -> Vec<SavedServer> {
    refresh_joins(user_zomboid_dir).0
}


/// The Studio is starting the game joining `address`: it goes to the top of the recent list.
pub fn note_join(user_zomboid_dir: &Path, address: &str, name: &str, username: &str, profile_id: Option<&str>) -> Result<(), String> {
    let mut list = read_saved(user_zomboid_dir);
    let at = list.iter().position(|s| s.address.eq_ignore_ascii_case(address.trim()));
    let mut entry = match at {
        Some(i) => list.remove(i),
        None => SavedServer {
            name: if name.trim().is_empty() { address.trim().to_string() } else { name.trim().to_string() },
            address: address.trim().to_string(),
            note: String::new(),
            profile_id: None,
            has_password: crate::secrets::get(&password_key(address)).ok().flatten().is_some(),
            starred: false,
            last_joined: None,
            mods: Vec::new(),
            items: Vec::new(),
            mods_seen: None,
            username: String::new(),
            made_profile_id: None,
        },
    };
    entry.last_joined = Some(now_secs());
    entry.profile_id = profile_id.map(str::to_string);
    if !username.trim().is_empty() {
        entry.username = username.trim().to_string();
    }
    list.insert(0, entry);
    // Recent ones that were never starred: the newest 30.
    let mut recent = 0;
    list.retain(|s| {
        if s.starred {
            return true;
        }
        recent += 1;
        recent <= 30
    });
    write_saved(user_zomboid_dir, &list)
}

pub fn saved_path(user_zomboid_dir: &Path) -> std::path::PathBuf {
    user_zomboid_dir.join(SAVED_FILE)
}

pub fn read_saved(user_zomboid_dir: &Path) -> Vec<SavedServer> {
    std::fs::read_to_string(saved_path(user_zomboid_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn write_saved(user_zomboid_dir: &Path, list: &[SavedServer]) -> Result<(), String> {
    let text = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(&saved_path(user_zomboid_dir), &text).map_err(|e| e.to_string())
}

pub fn password_key(address: &str) -> String {
    format!("server_password:{}", address.trim().to_lowercase())
}

/// Adds or replaces (same address) a kept server. `password`: Some("") forgets it, Some(x)
/// keeps x, None leaves it as it was.
pub fn save_server(user_zomboid_dir: &Path, mut server: SavedServer, password: Option<&str>) -> Result<Vec<SavedServer>, String> {
    server.address = server.address.trim().to_string();
    if server.address.is_empty() {
        return Err("A server needs an address.".to_string());
    }
    if server.name.trim().is_empty() {
        server.name = server.address.clone();
    }
    let key = password_key(&server.address);
    match password {
        Some("") => {
            let _ = crate::secrets::delete(&key);
            server.has_password = false;
        }
        Some(p) => {
            crate::secrets::set(&key, p)?;
            server.has_password = true;
        }
        None => server.has_password = crate::secrets::get(&key).ok().flatten().is_some(),
    }
    let mut list = read_saved(user_zomboid_dir);
    match list.iter_mut().find(|s| s.address.eq_ignore_ascii_case(&server.address)) {
        // What the logs said and when it was joined stay; the rest is what the player typed.
        Some(s) => {
            let (mods, items, seen, joined) = (std::mem::take(&mut s.mods), std::mem::take(&mut s.items), s.mods_seen, s.last_joined);
            *s = server;
            if s.mods.is_empty() {
                s.mods = mods;
                s.items = items;
                s.mods_seen = seen;
            }
            s.last_joined = s.last_joined.or(joined);
        }
        None => list.push(server),
    }
    write_saved(user_zomboid_dir, &list)?;
    Ok(list)
}

pub fn forget_server(user_zomboid_dir: &Path, address: &str) -> Result<Vec<SavedServer>, String> {
    let _ = crate::secrets::delete(&password_key(address));
    let mut list = read_saved(user_zomboid_dir);
    list.retain(|s| !s.address.eq_ignore_ascii_case(address.trim()));
    write_saved(user_zomboid_dir, &list)?;
    Ok(list)
}

/// The arguments that make the game join `address` (the password from the credential store when
/// none is given).
/// app -> game: the server being joined, its name and the account to fill in the game's connect
/// window (`address=<ip:port>`, `name=<name>`, `username=<name>`). The game fills it only from
/// its own server list, so without this it waits on an empty username; the Link also adds the
/// server (and the account) to the game's Favorites, so it is there after a disconnect. Never a
/// password.
pub const JOIN_FILE: &str = "PZModStudio_LinkJoin.txt";

pub fn write_join_file(user_zomboid_dir: &Path, address: &str, name: &str, username: &str) -> Result<(), String> {
    let lua = user_zomboid_dir.join("Lua");
    if !lua.is_dir() {
        return Ok(());
    }
    let clean = |s: &str| s.replace(['\r', '\n'], " ").trim().to_string();
    let text = format!("address={}\nname={}\nusername={}\n", clean(address), clean(name), clean(username));
    crate::load_order::ini_parser::atomic_write(lua.join(JOIN_FILE), text).map_err(|e| e.to_string())
}

pub fn join_args(address: &str, password: Option<&str>) -> Result<Vec<String>, String> {
    let address = address.trim();
    let bad = |v: &str| v.is_empty() || v.starts_with('-') || v.starts_with('+') || v.chars().any(|c| c.is_whitespace() || c.is_control());
    if bad(address) {
        return Err("That is not a server address (ip:port or a name).".to_string());
    }
    let mut args = vec!["+connect".to_string(), address.to_string()];
    let kept = match password {
        Some(p) => Some(p.to_string()),
        None => crate::secrets::get(&password_key(address)).ok().flatten(),
    };
    if let Some(p) = kept.filter(|p| !p.is_empty()) {
        if p.starts_with('-') || p.starts_with('+') || p.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err("The game cannot be given a password with spaces or starting with - or +; type it in the game instead.".to_string());
        }
        args.push("+password".to_string());
        args.push(p);
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steams_list_is_read_with_the_game_port() {
        let body = r#"{"response":{"servers":[{"addr":"1.2.3.4:16262","gameport":16261,"steamid":"9","name":" Big ","appid":108600,"gamedir":"zomboid","version":"1.0.0.0","product":"zomboid","region":255,"players":12,"max_players":32,"bots":0,"map":"Muldraugh, KY","secure":true,"dedicated":true,"os":"w","gametype":"hidden"},{"addr":"5.6.7.8:27015","name":"Small","players":1}]}}"#;
        let list = parse_list(body).unwrap();
        assert_eq!(list[0].name, "Big");
        assert_eq!(list[0].address, "1.2.3.4:16261");
        assert_eq!(list[0].query_address, "1.2.3.4:16262");
        assert_eq!(list[1].address, "5.6.7.8:27015", "no game port: the query port");
        assert!(parse_list(r#"{"response":{}}"#).unwrap().is_empty());
    }

    #[test]
    fn the_rules_answer_is_read_split_mods_and_all() {
        let mut p = vec![0x45, 4, 0];
        for (k, v) in [("description", "Hi"), ("version", "42.12.3"), ("open", "1"), ("mods", "\\ModA;\\ModB;ModA")] {
            p.extend_from_slice(k.as_bytes());
            p.push(0);
            p.extend_from_slice(v.as_bytes());
            p.push(0);
        }
        let r = rules_from_pairs(parse_rules_payload(&p).unwrap(), 30, "x".into());
        assert_eq!((r.description.as_str(), r.version.as_str(), r.open), ("Hi", "42.12.3", true));
        assert_eq!(r.mods, vec!["ModA", "ModB"]);
        let r = rules_from_pairs(vec![("mods:2/2".into(), ";C".into()), ("mods:1/2".into(), "A,B".into())], 0, String::new());
        assert_eq!(r.mods, vec!["A", "B", "C"]);
    }

    #[test]
    fn a_typed_address_is_tried_on_the_games_ports() {
        assert_eq!(query_candidates("1.2.3.4:16300"), vec!["1.2.3.4:16300", "1.2.3.4:16261", "1.2.3.4:8766", "1.2.3.4:8767", "1.2.3.4:27015"]);
        assert_eq!(query_candidates("play.example.com")[0], "play.example.com:16261");
        assert_eq!(join_args("1.2.3.4:16261", Some("")).unwrap(), vec!["+connect", "1.2.3.4:16261"]);
        assert_eq!(join_args("1.2.3.4:16261", Some("pw")).unwrap(), vec!["+connect", "1.2.3.4:16261", "+password", "pw"]);
        assert!(join_args("-debug", None).is_err());
        assert!(join_args("1.2.3.4 -x", None).is_err());
        assert!(join_args("1.2.3.4:16261", Some("a b")).is_err());
        assert_eq!(build_of_tags("hidden;modded;pvp;VERSION:42.21"), "42.21");
        assert_eq!(clean_description("<RGB:0.95,0.72,0.2> REVENANT <LINE> Hi\\nthere <3"), "REVENANT\n Hi\nthere <3");
        let full = "ModAAAAAAAA;".repeat(10) + "Cut";
        let r = rules_from_pairs(vec![("mods".into(), full), ("modCount".into(), "40".into())], 0, String::new());
        assert_eq!(r.mods.len(), 1, "ten of the same, the cut one left out");
        let r = rules_from_pairs(vec![("description:2/2".into(), " world".into()), ("description:1/2".into(), "hello".into()), ("description".into(), "hi".into()), ("modCount".into(), "40".into())], 0, String::new());
        assert_eq!((r.description.as_str(), r.mod_count), ("hello world", Some(40)));
    }

    #[test]
    fn the_last_join_in_a_log_gives_the_servers_items_and_mods() {
        let log = "[x] LOG  : Mod          f:0> loading MyOwnMod.\n\
[x] LOG  : General      f:0 st:0> ConnectToServerState: Start.\n\
[x] LOG  : General      f:0 st:0> ConnectToServerState: WorkshopConfirm GetItemState()=Subscribed|Installed ID=3508537032.\n\
[x] LOG  : General      f:0 st:0> ConnectToServerState: WorkshopConfirm GetItemState()=None ID=2812326159.\n\
[x] LOG  : General      f:0 st:0> ConnectToServerState: GetItemState()=Subscribed|Installed ID=3508537032.\n\
[x] LOG  : General      f:0> Loading Mods.\n\
[x] LOG  : Mod          f:0> loading NeatUI_Framework.\n\
[x] LOG  : Mod          f:0> loading \\SPNCC.\n";
        let (items, mods) = parse_join_log(log).unwrap();
        assert_eq!(items, vec!["3508537032", "2812326159"]);
        assert_eq!(mods, vec!["NeatUI_Framework", "\\SPNCC"], "only what loaded after the join");
        assert!(parse_join_log("[x] LOG  : Mod          f:0> loading A.").is_none());
    }

    /// Live: `PZMS_JOIN=ip:port cargo test -- --ignored live_join_launch --nocapture`: notes the
    /// join and starts the game with the arguments the Join button uses (no screen mode).
    #[test]
    #[ignore]
    fn live_join_launch() {
        let p = crate::vfs::studio_paths();
        let z = std::path::PathBuf::from(&p.user_zomboid_dir);
        let address = std::env::var("PZMS_JOIN").unwrap();
        note_join(&z, &address, "studio_jointest", "", None).unwrap();
        let mut args = crate::compat::launch_args(&p.user_zomboid_dir);
        args.extend(join_args(&address, None).unwrap());
        println!("args {:?}", args);
        let child = std::process::Command::new(std::path::Path::new(&p.pz_install_dir).join("ProjectZomboid64.exe")).current_dir(&p.pz_install_dir).args(&args).spawn().unwrap();
        println!("pid {}", child.id());
    }

    /// Live: what the kept servers know after a join (reads the logs).
    #[test]
    #[ignore]
    fn live_saved_filled() {
        let z = std::path::PathBuf::from(crate::vfs::studio_paths().user_zomboid_dir);
        for s in read_saved_filled(&z) {
            println!("{} {} joined={:?} seen={:?} items={:?} mods={:?}", s.name, s.address, s.last_joined, s.mods_seen, s.items, s.mods);
        }
    }

    /// Live: `PZMS_LOG=<a DebugLog> cargo test -- --ignored live_join_log --nocapture`.
    #[test]
    #[ignore]
    fn live_join_log() {
        let text = String::from_utf8_lossy(&std::fs::read(std::env::var("PZMS_LOG").unwrap()).unwrap()).to_string();
        let (items, mods) = parse_join_log(&text).expect("no join in that log");
        println!("{} items, {} mods; first mods {:?}", items.len(), mods.len(), mods.iter().take(5).collect::<Vec<_>>());
    }

    /// Live: `PZMS_PROBE=ip:port,ip:port cargo test -- --ignored live_rules --nocapture`.
    #[test]
    #[ignore]
    fn live_rules() {
        for a in std::env::var("PZMS_PROBE").unwrap_or_default().split(',').filter(|a| !a.is_empty()) {
            match server_rules(a) {
                Ok(r) => println!("{} ping={} v={} open={} pvp={:?} modCount={:?} mods={} first={:?} desc={:?} keys={:?}", a, r.ping_ms, r.version, r.open, r.pvp, r.mod_count, r.mods.len(), r.mods.iter().take(4).collect::<Vec<_>>(), r.description.chars().take(60).collect::<String>(), r.raw.iter().map(|(k, v)| format!("{}({})", k, v.len())).collect::<Vec<_>>()),
                Err(e) => println!("{} error: {}", a, e),
            }
        }
    }

    /// Live: `cargo test -- --ignored live_public_servers --nocapture` with the Web API key saved
    /// in Settings: the busiest servers and the rules of the first ones that answer.
    #[test]
    #[ignore]
    fn live_public_servers() {
        let key = crate::secrets::get(crate::secrets::STEAM_WEB_API_KEY).unwrap().expect("no Web API key saved");
        let list = public_servers(&key, "", 300).unwrap();
        println!("{} servers", list.len());
        let mut shown = 0;
        for s in list.iter().take(25) {
            match server_rules(&s.query_address) {
                Ok(r) => {
                    println!("{} | {} | {}/{} | v{} rules v{} open={} mods={} ping={} keys={:?}", s.name, s.address, s.players, s.max_players, s.version, r.version, r.open, r.mods.len(), r.ping_ms, r.raw.iter().map(|(k, v)| format!("{}({})", k, v.len())).collect::<Vec<_>>());
                    if shown == 0 {
                        println!("  first mods: {:?}", r.mods.iter().take(8).collect::<Vec<_>>());
                    }
                    shown += 1;
                }
                Err(e) => println!("{} | {} | no rules: {}", s.name, s.query_address, e),
            }
            if shown >= 5 {
                break;
            }
        }
    }
}
