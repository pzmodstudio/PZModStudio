use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::load_order::ini_parser::{read_mod_list_ini, write_mod_list_ini};
use crate::vfs::StudioPaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub workshop_id: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub pzversion: Option<String>,
    pub url: Option<String>,
    pub require: Vec<String>,
    pub load_mod_after: Vec<String>,
    pub incompatible: Vec<String>,
    pub icon_path: Option<String>,
    pub poster_url: Option<String>,
    pub is_library: bool,
    pub is_map_mod: bool,
    /// `category=` of its mod.info (map, vehicle, features, modpack), as the game's screen groups.
    #[serde(default)]
    pub category: Option<String>,
    pub enabled: bool,
    pub is_packaged: Option<bool>,
    /// Whether the installed game build can load this mod (None: build unknown).
    #[serde(default)]
    pub loadable_on_installed_build: Option<bool>,
    /// Switched on in the list the game reads, but no folder holds it (unsubscribed or deleted).
    /// The Mod List keeps it as a row so Mod Health can say so and the player can drop it.
    #[serde(default)]
    pub not_installed: bool,
    /// When it was installed (unix seconds): when its folder was made, the Workshop item's folder
    /// for a subscribed mod (Steam makes it at the first download and keeps it through updates),
    /// its own folder in Zomboid/mods otherwise. None when the disk does not say.
    #[serde(default)]
    pub installed_at: Option<u64>,
}

/// When `dir` was made (falling back to its last change on a file system without birth times).
pub fn folder_created(dir: &Path) -> Option<u64> {
    let meta = fs::metadata(dir).ok()?;
    let time = meta.created().or_else(|_| meta.modified()).ok()?;
    time.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
}

/// A row for a mod the active list names but nothing installed provides.
fn not_installed_row(id: &str) -> ModManifest {
    ModManifest {
        id: id.to_string(),
        name: id.to_string(),
        description: None,
        workshop_id: None,
        author: None,
        version: None,
        pzversion: None,
        url: None,
        require: Vec::new(),
        load_mod_after: Vec::new(),
        incompatible: Vec::new(),
        icon_path: None,
        poster_url: None,
        is_library: false,
        is_map_mod: false,
        category: None,
        enabled: true,
        is_packaged: None,
        loadable_on_installed_build: None,
        not_installed: true,
        installed_at: None,
    }
}

/// Helper function to sanitize and clean raw mod ID strings.
pub fn sanitize_mod_id(raw: &str) -> String {
    raw.trim()
        .trim_matches('\\')
        .trim_matches('/')
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

/// Clean text encoding (fixes Windows-1252 / ANSI garbage characters like Ã±, Ã¡, etc.)
fn clean_text_encoding(text: &str) -> String {
    let mut cleaned = text.to_string();
    
    // Common UTF-8 misdecoded in Windows-1252 / Latin-1 replacements
    let replacements = [
        ("Ã¡", "á"), ("Ã©", "é"), ("Ã*", "í"), ("Ã³", "ó"), ("Ãº", "ú"),
        ("Ã±", "ñ"), ("Ã‘", "Ñ"), ("Ã?", "Á"), ("Ã‰", "É"), ("Ã?", "Í"),
        ("Ã“", "Ó"), ("Ãš", "Ú"), ("â€™", "'"), ("â€œ", "\""), ("â€", "\""),
        ("â€“", "-"), ("â€”", "—"), ("Â°", "°"), ("ï»¿", ""),
    ];

    for (from, to) in replacements {
        cleaned = cleaned.replace(from, to);
    }

    // Strip unprintable control characters
    cleaned.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').collect()
}

/// Helper to robustly read mod.info content handling UTF-8 with BOM, Latin-1, and Windows-1252
fn read_text_file_lossy(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    
    // Strip UTF-8 BOM if present
    let bytes_slice = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        &bytes[..]
    };

    let decoded = match String::from_utf8(bytes_slice.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            // Fallback to ISO-8859-1 / Windows-1252 lossy decoding
            bytes_slice.iter().map(|&b| b as char).collect()
        }
    };

    Some(clean_text_encoding(&decoded))
}

/// Parses a mod.info file into a structured ModManifest struct.
pub fn parse_mod_info(path: &Path) -> Option<ModManifest> {
    if !path.exists() {
        return None;
    }

    let content = read_text_file_lossy(path)?;
    let mut id = String::new();
    let mut name = String::new();
    let mut description = None;
    let mut author = None;
    let mut category = None;
    let mut version = None;
    let mut pzversion = None;
    let mut url = None;
    let mut require = Vec::new();
    let mut load_mod_after = Vec::new();
    let mut incompatible = Vec::new();
    let mut poster_file = String::new();
    let mut icon_file = String::new();
    let mut pack = false;
    let mut tiledef = false;

    let parent_dir = path.parent();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("id=") {
            id = sanitize_mod_id(&trimmed[3..]);
        } else if trimmed.starts_with("name=") {
            name = clean_text_encoding(trimmed[5..].trim());
        } else if trimmed.starts_with("description=") {
            let desc_val = clean_text_encoding(trimmed[12..].trim());
            if !desc_val.is_empty() {
                description = Some(desc_val);
            }
        } else if trimmed.starts_with("author=") {
            let auth = clean_text_encoding(trimmed[7..].trim());
            if !auth.is_empty() {
                author = Some(auth);
            }
        } else if let Some(c) = trimmed.strip_prefix("category=") {
            let c = c.trim().to_lowercase();
            if !c.is_empty() {
                category = Some(c);
            }
        } else if trimmed.starts_with("modversion=") {
            let v = trimmed[11..].trim().to_string();
            if !v.is_empty() {
                version = Some(v);
            }
        } else if trimmed.starts_with("version=") {
            let v = trimmed[8..].trim().to_string();
            if !v.is_empty() && version.is_none() {
                version = Some(v);
            }
        } else if trimmed.starts_with("pzversion=") {
            let v = trimmed[10..].trim().to_string();
            if !v.is_empty() {
                pzversion = Some(v);
            }
        } else if trimmed.starts_with("versionMin=") {
            let v = trimmed[11..].trim().to_string();
            if !v.is_empty() && pzversion.is_none() {
                pzversion = Some(v);
            }
        } else if trimmed.starts_with("url=") {
            let u = trimmed[4..].trim().to_string();
            if !u.is_empty() {
                url = Some(u);
            }
        } else if trimmed.starts_with("require=") {
            let req_str = trimmed[8..].trim();
            for s in req_str.split(',') {
                let clean = sanitize_mod_id(s);
                let lower = clean.to_lowercase();
                let is_junk = matches!(
                    lower.as_str(),
                    "please" | "update" | "to" | "b42" | "version" | "of" | "the" | "game" | "read" | "mod" | "page" | "for" | "help" | "visit" | "steam" | "link" | "download" | "notice" | "warning"
                );
                if !clean.is_empty() && !is_junk && !require.contains(&clean) {
                    require.push(clean);
                }
            }
        } else if let Some(req_str) = trimmed.strip_prefix("loadModAfter=")
            .or_else(|| trimmed.strip_prefix("load_mod_after="))
            .or_else(|| trimmed.strip_prefix("loadAfter="))
            .or_else(|| trimmed.strip_prefix("load_after="))
            .or_else(|| trimmed.strip_prefix("loadafter="))
            .or_else(|| trimmed.strip_prefix("loadmodafter="))
            .or_else(|| trimmed.strip_prefix("LoadAfter="))
            .or_else(|| trimmed.strip_prefix("LoadModAfter="))
        {
            for s in req_str.trim().split(',') {
                let clean = sanitize_mod_id(s);
                if !clean.is_empty() && !load_mod_after.contains(&clean) {
                    load_mod_after.push(clean);
                }
            }
        } else if trimmed.starts_with("incompatible=") {
            let inc_str = trimmed[13..].trim();
            incompatible = inc_str
                .split(',')
                .map(|s| sanitize_mod_id(s))
                .filter(|s| !s.is_empty())
                .collect();
        } else if trimmed.starts_with("incompatibleWith=") {
            let inc_str = trimmed[17..].trim();
            incompatible = inc_str
                .split(',')
                .map(|s| sanitize_mod_id(s))
                .filter(|s| !s.is_empty())
                .collect();
        } else if trimmed.starts_with("poster=") {
            poster_file = trimmed[7..].trim().to_string();
        } else if trimmed.starts_with("icon=") {
            icon_file = trimmed[5..].trim().to_string();
        } else if trimmed.starts_with("pack=") {
            pack = true;
        } else if trimmed.starts_with("tiledef=") {
            tiledef = true;
        }
    }

    if id.is_empty() {
        return None;
    }

    // Format synthetic package display name uniformly as "PZ Mod Studio Patch: <name>" across Mod List, Mod Merger, and game
    if id.starts_with("Z_PZModStudio_") && !id.contains("Carrier") {
        let clean_sub = if id.starts_with("Z_PZModStudio_") {
            id["Z_PZModStudio_".len()..].to_string().replace('_', " ")
        } else {
            name.clone()
        };
        name = format!("PZ Mod Studio Patch: {}", clean_sub);
    }

    // Robust search for poster and icon images in mod directory
    let mut poster_url = None;
    let mut icon_path = None;

    if let Some(dir) = parent_dir {
        let mut dirs_to_check = vec![dir.to_path_buf()];
        if let Some(p) = dir.parent() {
            dirs_to_check.push(p.to_path_buf());
            if let Some(gp) = p.parent() {
                dirs_to_check.push(gp.to_path_buf());
            }
        }

        for search_dir in &dirs_to_check {
            let poster_candidates = vec![
                if !poster_file.is_empty() { Some(search_dir.join(&poster_file)) } else { None },
                Some(search_dir.join("poster.png")),
                Some(search_dir.join("Poster.png")),
                Some(search_dir.join("poster.jpg")),
                Some(search_dir.join("Poster.jpg")),
                Some(search_dir.join("preview.png")),
                Some(search_dir.join("preview.jpg")),
                Some(search_dir.join("icon.png")),
            ];

            for cand in poster_candidates.into_iter().flatten() {
                if cand.exists() {
                    poster_url = Some(cand.to_string_lossy().to_string());
                    break;
                }
            }
            if poster_url.is_some() {
                break;
            }
        }

        for search_dir in &dirs_to_check {
            let icon_candidates = vec![
                if !icon_file.is_empty() { Some(search_dir.join(&icon_file)) } else { None },
                Some(search_dir.join("icon.png")),
                Some(search_dir.join("Icon.png")),
            ];

            for cand in icon_candidates.into_iter().flatten() {
                if cand.exists() {
                    icon_path = Some(cand.to_string_lossy().to_string());
                    break;
                }
            }
            if icon_path.is_some() {
                break;
            }
        }
    }

    let lower_id = id.to_lowercase();
    let is_addon_or_patch = lower_id.contains("addon") || lower_id.contains("patch") || lower_id.contains("compat") || lower_id.contains("ext");
    let is_library = !is_addon_or_patch && (
        lower_id.contains("lib") ||
        lower_id.contains("manager") ||
        lower_id.contains("framework") ||
        lower_id.contains("api") ||
        lower_id.contains("core")
    );
    let is_map_mod = pack || tiledef || lower_id.contains("map");

    let mut workshop_id = extract_workshop_id_from_path(path);
    if workshop_id.is_none() {
        if let Some(ref u) = url {
            if let Some(pos) = u.find("id=") {
                let rest = &u[pos + 3..];
                let w_id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !w_id.is_empty() {
                    workshop_id = Some(w_id);
                }
            }
        }
    }

    let mut manifest = ModManifest {
        id,
        name: if name.is_empty() { "Unnamed Mod".to_string() } else { name },
        description,
        workshop_id,
        author,
        version,
        pzversion,
        url,
        require,
        load_mod_after,
        incompatible,
        icon_path,
        poster_url,
        is_library,
        is_map_mod,
        category,
        enabled: false,
        is_packaged: None,
        loadable_on_installed_build: None,
        not_installed: false,
        installed_at: None,
    };

    apply_known_dependency_heuristics(&mut manifest);
    Some(manifest)
}


/// A few popular mods need another mod but do not say so in their own `mod.info`, which leaves
/// the player with a crash and no clue. Those are listed here by Workshop id or exact mod id.
///
/// Matching on *part* of a name is deliberately not done: a rule like "any id containing tsar
/// needs Tsarslib" invents requirements for unrelated mods and fills Mod Health with errors that
/// are not real. If a mod is not listed here, its own `mod.info` is taken at its word.
pub fn apply_known_dependency_heuristics(manifest: &mut ModManifest) {
    let lower_id = manifest.id.to_lowercase();
    let w_id = manifest.workshop_id.clone().unwrap_or_default();

    fn needs(manifest: &mut ModManifest, dependency: &str) {
        if !manifest.require.iter().any(|r| r.eq_ignore_ascii_case(dependency)) {
            manifest.require.push(dependency.to_string());
        }
    }

    // Brita's Weapon Pack
    if lower_id == "brita" || w_id == "2200148440" {
        needs(manifest, "Arsenal(26)GunFighter[MAIN MOD 2.0]");
        needs(manifest, "modoptions");
    }
    // Arsenal(26) GunFighter
    if w_id == "2297098490" {
        needs(manifest, "modoptions");
    }
    // Brita's Armor Pack
    if lower_id == "brita_armor" || w_id == "2460154811" {
        needs(manifest, "modoptions");
    }

    // A mod with no requirements of its own is usually a library other mods build on.
    manifest.is_library = manifest.require.is_empty()
        && (lower_id.ends_with("lib") || lower_id.contains("library") || lower_id.contains("framework"));
}

pub fn get_all_user_zomboid_dirs(user_zomboid_dir: &str) -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    let clean = user_zomboid_dir.trim();
    if !clean.is_empty() {
        dirs.push(std::path::PathBuf::from(clean));
    }
    if let Some(home) = dirs_next::home_dir() {
        let p1 = home.join("Zomboid");
        if !dirs.contains(&p1) { dirs.push(p1); }
        let p2 = home.join("Documents").join("Zomboid");
        if !dirs.contains(&p2) { dirs.push(p2); }
    }
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let p3 = std::path::PathBuf::from(&user_profile).join("Zomboid");
        if !dirs.contains(&p3) { dirs.push(p3); }
        let p4 = std::path::PathBuf::from(&user_profile).join("Documents").join("Zomboid");
        if !dirs.contains(&p4) { dirs.push(p4); }
    }
    if dirs.is_empty() {
        dirs.push(std::path::PathBuf::from("C:\\Zomboid"));
    }
    dirs
}

/// The candidate Zomboid folders that exist on disk. Use this, not
/// `get_all_user_zomboid_dirs`, for anything that writes, so no phantom Zomboid folders appear.
pub fn get_existing_user_zomboid_dirs(user_zomboid_dir: &str) -> Vec<std::path::PathBuf> {
    get_all_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .filter(|d| d.is_dir())
        .collect()
}

pub fn get_subscribed_workshop_ids(workshop_dir: &Path) -> Option<HashSet<String>> {
    let mut candidates = Vec::new();
    if let Some(parent) = workshop_dir.parent().and_then(|p| p.parent()) {
        candidates.push(parent.join("appworkshop_108600.acf"));
        candidates.push(parent.join("appworkshop_108600.vdf"));
    }

    for candidate in candidates {
        if candidate.exists() {
            if let Ok(content) = fs::read_to_string(&candidate) {
                let mut subscribed_ids = HashSet::new();

                // 1. Try parsing WorkshopItemDetails first (most authoritative for active subscriptions)
                if content.contains("\"WorkshopItemDetails\"") {
                    let mut in_details = false;
                    let mut current_id = String::new();
                    let mut has_subscriber = false;
                    let mut brace_depth = 0;

                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.contains("\"WorkshopItemDetails\"") {
                            in_details = true;
                            continue;
                        }
                        if in_details {
                            if trimmed.starts_with('{') {
                                brace_depth += 1;
                            } else if trimmed.starts_with('}') {
                                if brace_depth == 2 {
                                    if !current_id.is_empty() && has_subscriber {
                                        subscribed_ids.insert(current_id.clone());
                                    }
                                    current_id.clear();
                                    has_subscriber = false;
                                }
                                brace_depth -= 1;
                                if brace_depth <= 0 {
                                    break;
                                }
                            } else if brace_depth == 1 {
                                let clean = trimmed.trim_matches('"').trim();
                                if clean.chars().all(|c| c.is_numeric()) && !clean.is_empty() {
                                    current_id = clean.to_string();
                                    has_subscriber = false;
                                }
                            } else if brace_depth == 2 {
                                if trimmed.contains("\"subscribedby\"") {
                                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                                    if let Some(val) = parts.last() {
                                        let sub_id = val.trim_matches('"').trim();
                                        if sub_id != "0" && !sub_id.is_empty() {
                                            has_subscriber = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !subscribed_ids.is_empty() {
                    return Some(subscribed_ids);
                }

                // 2. Fallback to WorkshopItemsInstalled
                let mut installed_ids = HashSet::new();
                let mut in_installed = false;
                let mut brace_depth = 0;

                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.contains("\"WorkshopItemsInstalled\"") {
                        in_installed = true;
                        continue;
                    }
                    if in_installed {
                        if trimmed.starts_with('{') {
                            brace_depth += 1;
                        } else if trimmed.starts_with('}') {
                            brace_depth -= 1;
                            if brace_depth <= 0 {
                                break;
                            }
                        } else if brace_depth == 1 {
                            let clean = trimmed.trim_matches('"').trim();
                            if clean.chars().all(|c| c.is_numeric()) && !clean.is_empty() {
                                installed_ids.insert(clean.to_string());
                            }
                        }
                    }
                }

                if !installed_ids.is_empty() {
                    return Some(installed_ids);
                }
            }
        }
    }
    None
}

/// Lowest version folder Build 42 reads (ChooseGameInfo.minRequiredVersion).
const MIN_VERSION_DIR: u32 = 42_000;

/// A version folder name as the game reads it (ZomboidFileSystem.getGameVersionIntFromName):
/// only major.minor count, so "42" -> 42000 and "42.13" / "42.13.1" -> 42013.
pub fn version_dir_int(name: &str) -> Option<u32> {
    let mut parts = name.trim().split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor = parts.next().map_or(0, |m| m.parse::<u32>().unwrap_or(0).min(999));
    Some(major * 1000 + minor)
}

/// The game build last run from this Zomboid folder, from `Zomboid/version.txt` ("42.21.0 <rev>").
pub fn read_installed_game_version(user_zomboid_dir: &str) -> Option<u32> {
    get_existing_user_zomboid_dirs(user_zomboid_dir).iter().find_map(|dir| {
        fs::read_to_string(dir.join("version.txt"))
            .ok()
            .and_then(|text| text.split_whitespace().next().and_then(version_dir_int))
    })
}

/// The mod.info the game reads for the mod folder `mod_root` (e.g. `.../mods/MyMod`) on build
/// `game_version`, following ZomboidFileSystem.getModVersionDirName + ChooseGameInfo:
/// B42 picks the highest version folder between 42.0 and the installed build (default "42.0")
/// and reads its mod.info, falling back to `common/mod.info`; the mod root is never read.
/// Build 41 reads only the root mod.info. An unknown build behaves like the newest one.
pub fn select_mod_info_for_build(mod_root: &Path, game_version: Option<u32>) -> Option<PathBuf> {
    let game = game_version.unwrap_or(u32::MAX);
    if game < MIN_VERSION_DIR {
        let root = mod_root.join("mod.info");
        return root.is_file().then_some(root);
    }
    let version_dir = version_dir_for_build(mod_root, game);
    [version_dir.join("mod.info"), mod_root.join("common").join("mod.info")]
        .into_iter()
        .find(|p| p.is_file())
}

/// The folders whose `media/` the game loads for this mod, lowest priority first: Build 42 reads
/// `common/` and then the version folder (which overrides it); Build 41 reads the mod root.
pub fn media_layers_for_build(mod_root: &Path, game_version: Option<u32>) -> Vec<PathBuf> {
    let game = game_version.unwrap_or(u32::MAX);
    if game < MIN_VERSION_DIR {
        return vec![mod_root.to_path_buf()];
    }
    vec![mod_root.join("common"), version_dir_for_build(mod_root, game)]
        .into_iter()
        .filter(|d| d.is_dir())
        .collect()
}

/// ZomboidFileSystem.getModVersionDirName: the highest version folder between 42.0 and the build
/// (default "42.0"), whether or not it holds a mod.info.
fn version_dir_for_build(mod_root: &Path, game: u32) -> PathBuf {
    let mut best_name = "42.0".to_string();
    let mut best = MIN_VERSION_DIR;
    if let Ok(entries) = fs::read_dir(mod_root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(v) = version_dir_int(&name) {
                if v >= best && v <= game {
                    best = v;
                    best_name = name;
                }
            }
        }
    }
    mod_root.join(best_name)
}

/// The mod folder (the direct child of a `mods` folder) that a mod.info belongs to.
fn mod_root_of(mod_info: &Path) -> Option<&Path> {
    let mut dir = mod_info.parent()?;
    loop {
        let parent = dir.parent()?;
        if parent.file_name().is_some_and(|n| n.eq_ignore_ascii_case("mods")) {
            return Some(dir);
        }
        dir = parent;
    }
}

/// How a scanned mod.info relates to what the game loads on the installed build:
/// Some(2) = this is the mod.info the game reads; Some(1) = the mod cannot load on this build
/// (B41-only, or only folders newer than the build) and this is the mod.info kept so the mod is
/// still listed: its newest one, else its root one; None = skip (another copy of the same mod
/// is the one that counts).
fn mod_info_rank(mod_info: &Path, game_version: Option<u32>) -> Option<(u8, Option<bool>)> {
    let root = mod_root_of(mod_info)?;
    match select_mod_info_for_build(root, game_version) {
        Some(selected) if selected == mod_info => Some((2, game_version.map(|_| true))),
        Some(_) => None,
        None => {
            let listed = select_mod_info_for_build(root, None).or_else(|| {
                let root_info = root.join("mod.info");
                root_info.is_file().then_some(root_info)
            });
            (listed.as_deref() == Some(mod_info)).then_some((1, game_version.map(|_| false)))
        }
    }
}

/// Scans all subscribed Workshop & local mods recursively without depth limits.
/// Preserves exact ModListData.ini load order for active mods, and sorts remaining mods deterministically.
/// The installed mod a list entry means: the exact id first (the game matches ids exactly), then
/// the same id in another case, then the same letters and digits -- each looser step only when
/// it names a single mod, so `Mod_A` never switches on `ModA` beside it.
fn match_installed_key<V>(installed: &std::collections::HashMap<String, V>, wanted: &str) -> Option<String> {
    let wanted = wanted.trim();
    if installed.contains_key(wanted) {
        return Some(wanted.to_string());
    }
    let unique = |keys: Vec<&String>| if keys.len() == 1 { Some(keys[0].clone()) } else { None };
    let lower = wanted.to_lowercase();
    let by_case: Vec<&String> = installed.keys().filter(|k| k.trim().to_lowercase() == lower).collect();
    if !by_case.is_empty() {
        return unique(by_case);
    }
    let alnum = |s: &str| s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
    let wanted_alnum = alnum(wanted);
    if wanted_alnum.is_empty() {
        return None;
    }
    unique(installed.keys().filter(|k| alnum(k) == wanted_alnum).collect())
}

pub fn scan_all_installed_mods(paths: &StudioPaths) -> Vec<ModManifest> {
    let mut all_mods_map: std::collections::HashMap<String, ModManifest> = std::collections::HashMap::new();
    let mut mod_scores: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
    let game_version = read_installed_game_version(&paths.user_zomboid_dir);

    // 1. Scan Steam Workshop mods (content/108600/) with depth 8
    let workshop_path = Path::new(&paths.workshop_dir);
    let subscribed_ids = get_subscribed_workshop_ids(workshop_path);

    if workshop_path.exists() {
        for entry in WalkDir::new(workshop_path).max_depth(8).into_iter().filter_map(|e| e.ok()) {
            if entry.file_name() == "mod.info" {
                let mod_file_path = entry.path().to_path_buf();
                let workshop_id = extract_workshop_id_from_path(&mod_file_path);

                // If appworkshop_108600.acf is present, filter out unsubscribed orphaned folders!
                if let Some(ref subs) = subscribed_ids {
                    if let Some(ref w_id) = workshop_id {
                        if !subs.contains(w_id) {
                            continue; // Skip unsubscribed orphaned workshop mod
                        }
                    }
                }

                // Steam owns everything under the Workshop folder: never create or copy files here.
                // Read the same mod.info the game reads on the installed build.
                let Some((new_score, loadable)) = mod_info_rank(&mod_file_path, game_version) else {
                    continue;
                };

                if let Some(mut manifest) = parse_mod_info(&mod_file_path) {
                    if manifest.id == "PZModStudioCarrier" {
                        continue;
                    }
                    manifest.installed_at = workshop_id.as_ref().and_then(|w| folder_created(&workshop_path.join(w)));
                    manifest.workshop_id = workshop_id;
                    manifest.loadable_on_installed_build = loadable;
                    apply_known_dependency_heuristics(&mut manifest);

                    if let Some(existing_score) = mod_scores.get(&manifest.id) {
                        if new_score <= *existing_score {
                            continue; // Same ID twice: keep the first loadable copy
                        }
                    }
                    mod_scores.insert(manifest.id.clone(), new_score);
                    all_mods_map.insert(manifest.id.clone(), manifest);
                }
            }
        }
    }

    // The Live Bridge companion mod used to be written into Zomboid/mods here, on every scan,
    // without asking. It lets another program run Lua inside the game, which is not something to
    // put in a player's mods folder uninvited: it is installed only when the user asks for it
    // (Settings, or install_bridge_companion_mod over MCP). Existing copies are left alone.

    let active_from_disk = crate::patch_generator::get_active_mod_ids_from_disk(&paths.user_zomboid_dir, &paths.mod_list_ini_path);

    // 3. Scan Zomboid/mods of the Zomboid folder the game uses. Other candidate folders (e.g.
    //    Documents/Zomboid) are not read by the game; old Studio versions left stale copies there.
    let game_user_dirs: Vec<PathBuf> = get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().take(1).collect();
    for user_dir in &game_user_dirs {
        let user_mods_path = user_dir.join("mods");
        if user_mods_path.exists() {
            for entry in WalkDir::new(&user_mods_path).max_depth(8).into_iter().filter_map(|e| e.ok()) {
                // The game refuses to list its example mod (ZomboidFileSystem.getAllModFoldersAux).
                if entry.path().components().any(|c| c.as_os_str().eq_ignore_ascii_case("examplemod")) {
                    continue;
                }
                if entry.file_name() == "mod.info" {
                    let Some((_, loadable)) = mod_info_rank(entry.path(), game_version) else {
                        continue;
                    };
                    if let Some(mut manifest) = parse_mod_info(entry.path()) {
                        if manifest.id == "PZModStudioCarrier" {
                            continue;
                        }
                        manifest.loadable_on_installed_build = loadable;
                        // Its own folder: the first one under Zomboid/mods.
                        manifest.installed_at = entry
                            .path()
                            .strip_prefix(&user_mods_path)
                            .ok()
                            .and_then(|rel| rel.components().next())
                            .and_then(|first| folder_created(&user_mods_path.join(first.as_os_str())));

                        // Check visibility for synthetic fusion packages and Live Bridge
                        if manifest.id.starts_with("Z_PZModStudio_") {
                            let mut mod_root = entry.path().parent().unwrap_or(entry.path());
                            while let Some(parent) = mod_root.parent() {
                                let folder_name = mod_root.file_name().unwrap_or_default().to_string_lossy();
                                if folder_name.starts_with("Z_PZModStudio_") {
                                    break;
                                }
                                if folder_name == "mods" || folder_name.is_empty() {
                                    break;
                                }
                                mod_root = parent;
                            }

                            let meta_path = mod_root.join("patch_metadata.json");
                            let is_active = active_from_disk.contains(&manifest.id);
                            let mut is_visible = is_active;
                            let mut is_pkg = false;

                            if meta_path.exists() {
                                if let Ok(meta_str) = fs::read_to_string(&meta_path) {
                                    if let Ok(meta_json) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                                        if meta_json["is_packaged"].as_bool() == Some(true) {
                                            is_visible = true;
                                            is_pkg = true;
                                        }
                                        if let Some(vis) = meta_json["is_visible_in_modlist"].as_bool() {
                                            is_visible = vis;
                                        }
                                    }
                                }
                            } else if manifest.id == "Z_PZModStudio_Bridge" {
                                is_visible = true;
                                is_pkg = true;
                            }

                            manifest.is_packaged = Some(is_pkg);

                            if !is_visible && !is_active {
                                continue; // Skip hidden package
                            }
                        }

                        // Preserve workshop_id, poster, and icon if previously scanned from Steam Workshop
                        if let Some(existing) = all_mods_map.get(&manifest.id) {
                            if manifest.workshop_id.is_none() {
                                manifest.workshop_id = existing.workshop_id.clone();
                            }
                            if manifest.poster_url.is_none() {
                                manifest.poster_url = existing.poster_url.clone();
                            }
                            if manifest.icon_path.is_none() {
                                manifest.icon_path = existing.icon_path.clone();
                            }
                        }

                        all_mods_map.insert(manifest.id.clone(), manifest);
                    }
                }
            }
        }
    }

    // 3. Check for Active Profile or PZModStudio_MasterLoadOrder.json to restore exact absolute load order
    let mut saved_load_order: Option<Vec<String>> = None;
    let mut saved_active_ids: Option<HashSet<String>> = None;
    let mut saved_in: Option<(std::path::PathBuf, bool)> = None;

    let user_dirs = get_all_user_zomboid_dirs(&paths.user_zomboid_dir);
    for u_dir in &user_dirs {
        // A) Check PZModStudio_Instances/ for an active profile
        let inst_dir = u_dir.join("PZModStudio_Instances");
        if inst_dir.exists() {
            if let Ok(entries) = fs::read_dir(&inst_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(c) = fs::read_to_string(&path) {
                            if let Ok(inst) = serde_json::from_str::<crate::instance_manager::AppInstance>(&c) {
                                if inst.is_active && !inst.load_order.is_empty() {
                                    saved_load_order = Some(inst.load_order);
                                    saved_active_ids = Some(inst.active_mod_ids.into_iter().collect());
                                    saved_in = Some((u_dir.clone(), true));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        if saved_load_order.is_some() {
            break;
        }

        // B) Check PZModStudio_MasterLoadOrder.json
        let master_path = u_dir.join("PZModStudio_MasterLoadOrder.json");
        if master_path.exists() {
            if let Ok(c) = fs::read_to_string(&master_path) {
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&c) {
                    if let Some(arr) = json_val["load_order"].as_array() {
                        let order: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                        if !order.is_empty() {
                            saved_load_order = Some(order);
                            let active: HashSet<String> = json_val["active_mod_ids"]
                                .as_array()
                                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                .unwrap_or_default();
                            saved_active_ids = Some(active);
                            saved_in = Some((u_dir.clone(), false));
                            break;
                        }
                    }
                }
            }
        }
    }

    // default.txt is what the game loads. When it no longer says what the Studio saved -- the
    // game's own Mods screen, an earlier list restored, a bisect trial, another tool -- it wins,
    // and the saved list takes it, so the next save does not put the old list back.
    if let (Some(order), Some(active), Some((u_dir, is_profile))) = (&saved_load_order, &saved_active_ids, &saved_in) {
        if let Ok(ini) = read_mod_list_ini(&paths.mod_list_ini_path) {
            let game_active: Vec<String> = ini.active_mods.iter().map(|s| sanitize_mod_id(s)).filter(|s| !s.is_empty()).collect();
            let installed = |id: &String| all_mods_map.contains_key(id);
            let in_game: HashSet<&String> = game_active.iter().filter(|id| installed(id)).collect();
            let in_saved: HashSet<&String> = active.iter().filter(|id| installed(id)).collect();
            if !game_active.is_empty() && in_game != in_saved {
                if *is_profile {
                    if let Ok(Some(inst)) = crate::instance_manager::take_mod_list_from_game(u_dir, &game_active) {
                        saved_load_order = Some(inst.load_order);
                        saved_active_ids = Some(inst.active_mod_ids.into_iter().collect());
                    }
                } else {
                    let merged = crate::studio_link::merge_game_order(order, &game_active);
                    let master = serde_json::json!({ "load_order": merged, "active_mod_ids": game_active });
                    let _ = crate::load_order::ini_parser::write_file_with_backup(
                        &u_dir.join("PZModStudio_MasterLoadOrder.json"),
                        &serde_json::to_string_pretty(&master).unwrap_or_default(),
                        &u_dir.join("PZModStudio_Backups"),
                    );
                    saved_load_order = Some(merged);
                    saved_active_ids = Some(game_active.into_iter().collect());
                }
            }
        }
    }

    let mut result_mods = Vec::new();
    let mut processed_ids = HashSet::new();
    let mut missing_seen: HashSet<String> = HashSet::new();

    if let Some(order) = saved_load_order {
        let active_set = saved_active_ids.unwrap_or_default();
        for raw_id in order {
            let clean_id = sanitize_mod_id(&raw_id);
            let matched_key = match_installed_key(&all_mods_map, &clean_id);

            if let Some(key) = matched_key {
                if let Some(mut manifest) = all_mods_map.remove(&key) {
                    manifest.enabled = active_set.contains(&raw_id) || active_set.contains(&manifest.id) || active_set.contains(&key);
                    result_mods.push(manifest);
                    processed_ids.insert(key);
                }
            } else if !clean_id.is_empty()
                && !clean_id.starts_with("Z_PZModStudio_")
                && (active_set.contains(&raw_id) || active_set.contains(&clean_id))
                && missing_seen.insert(clean_id.clone())
            {
                result_mods.push(not_installed_row(&clean_id));
            }
        }
    } else if let Ok(mut ini_data) = read_mod_list_ini(&paths.mod_list_ini_path) {
        let mut modified_ini = false;
        for raw_active_id in &ini_data.active_mods {
            let active_id = sanitize_mod_id(raw_active_id);

            let matched_key = match_installed_key(&all_mods_map, &active_id);

            if let Some(key) = matched_key {
                if let Some(mut manifest) = all_mods_map.remove(&key) {
                    // Open / Draft synthetic packages cannot be active in the game!
                    if manifest.id.starts_with("Z_PZModStudio_") && manifest.id != "Z_PZModStudio_Bridge" && manifest.is_packaged == Some(false) {
                        manifest.enabled = false;
                        all_mods_map.insert(key, manifest);
                        modified_ini = true;
                    } else {
                        manifest.enabled = true;
                        result_mods.push(manifest);
                        processed_ids.insert(key);
                    }
                }
            } else if active_id.starts_with("Z_PZModStudio_") {
                // This synthetic package is currently in DRAFT (is_packaged = false) or was deleted.
                // Do NOT include it in result_mods, and mark ini for pruning!
                modified_ini = true;
            } else if !active_id.is_empty() && missing_seen.insert(active_id.clone()) {
                // Unsubscribed or deleted but still switched on: the game skips it with a line in
                // console.txt. Keep it in the list so Mod Health reports it.
                result_mods.push(not_installed_row(&active_id));
            }
        }

        if modified_ini && !paths.mod_list_ini_path.is_empty() {
            ini_data.active_mods.retain(|id| !id.starts_with("Z_PZModStudio_") || processed_ids.contains(id));
            let _ = write_mod_list_ini(&paths.mod_list_ini_path, &ini_data.active_mods);
        }
    }

    // 4. Append remaining subscribed mods found on disk (disabled by default) in deterministic alphabetical order
    let mut remaining_mods: Vec<ModManifest> = all_mods_map
        .into_iter()
        .filter(|(id, _)| !processed_ids.contains(id))
        .map(|(_, mut manifest)| {
            manifest.enabled = false;
            manifest
        })
        .collect();

    // Sort remaining mods alphabetically by name and id so their position is 100% stable across refreshes!
    remaining_mods.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });

    result_mods.extend(remaining_mods);

    result_mods
}

pub fn extract_workshop_id_from_path(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy();
    let normalized = path_str.replace('/', "\\");

    // 1. Check path containing 108600\
    if let Some(idx) = normalized.find("108600\\") {
        let rest = &normalized[idx + 7..];
        let w_id = rest.split('\\').next().unwrap_or("").trim();
        if !w_id.is_empty() && w_id.chars().all(|c| c.is_ascii_digit()) {
            return Some(w_id.to_string());
        }
    }

    // 2. Check path containing content\108600\
    if let Some(idx) = normalized.to_lowercase().find("content\\108600\\") {
        let rest = &normalized[idx + 15..];
        let w_id = rest.split('\\').next().unwrap_or("").trim();
        if !w_id.is_empty() && w_id.chars().all(|c| c.is_ascii_digit()) {
            return Some(w_id.to_string());
        }
    }

    None
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exact_id_wins_and_a_loose_match_must_be_the_only_one() {
        let installed: std::collections::HashMap<String, ()> =
            ["Mod_A", "ModA", "Other"].iter().map(|k| (k.to_string(), ())).collect();
        assert_eq!(match_installed_key(&installed, "Mod_A").as_deref(), Some("Mod_A"));
        assert_eq!(match_installed_key(&installed, "ModA").as_deref(), Some("ModA"));
        assert_eq!(match_installed_key(&installed, "other").as_deref(), Some("Other"));
        // "mod-a" is both of them by letters and digits: neither is guessed.
        assert_eq!(match_installed_key(&installed, "mod-a"), None);
    }

    /// The game's Mods screen (or an earlier list restored) changed default.txt while the Studio
    /// was closed: the Mod List shows that, and the saved order takes it.
    #[test]
    fn the_list_the_game_reads_wins_over_the_saved_one() {
        let z = std::env::temp_dir().join(format!("pzms-scan-truth-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        for id in ["ModA", "ModB", "ModC"] {
            let dir = z.join("mods").join(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("mod.info"), format!("name={}\nid={}\n", id, id)).unwrap();
        }
        fs::write(
            z.join("PZModStudio_MasterLoadOrder.json"),
            r#"{"load_order":["ModA","ModB","ModC"],"active_mod_ids":["ModA","ModB"]}"#,
        )
        .unwrap();
        fs::write(z.join("mods").join("default.txt"), "VERSION = 1,\nmods\n{\n    mod = ModC,\n}\n").unwrap();

        let paths = StudioPaths {
            pz_install_dir: String::new(),
            workshop_dir: z.join("no-workshop").to_string_lossy().to_string(),
            user_zomboid_dir: z.to_string_lossy().to_string(),
            mod_list_ini_path: z.join("mods").join("default.txt").to_string_lossy().to_string(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        let mods = scan_all_installed_mods(&paths);
        let on: Vec<&str> = mods.iter().filter(|m| m.enabled).map(|m| m.id.as_str()).collect();
        assert_eq!(on, vec!["ModC"]);
        let saved = fs::read_to_string(z.join("PZModStudio_MasterLoadOrder.json")).unwrap();
        assert!(saved.contains("\"ModC\"") && !saved.contains("\"active_mod_ids\": [\n    \"ModA\""), "{}", saved);
        let _ = fs::remove_dir_all(&z);
    }

    /// A mod switched on in default.txt that nothing installed provides stays in the list, marked,
    /// so Mod Health's missing_mod reaches the Mod List. Studio packages that are gone do not.
    #[test]
    fn switched_on_but_not_installed_mods_stay_as_rows() {
        let z = std::env::temp_dir().join(format!("pzms-scan-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        let dir = z.join("mods").join("ModA");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("mod.info"), "name=ModA\nid=ModA\n").unwrap();
        fs::write(
            z.join("mods").join("default.txt"),
            "VERSION = 1,\nmods\n{\n    mod = ModA,\n    mod = Gone,\n    mod = Z_PZModStudio_Old,\n}\n",
        )
        .unwrap();
        let paths = StudioPaths {
            pz_install_dir: String::new(),
            workshop_dir: z.join("no-workshop").to_string_lossy().to_string(),
            user_zomboid_dir: z.to_string_lossy().to_string(),
            mod_list_ini_path: z.join("mods").join("default.txt").to_string_lossy().to_string(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        let mods = scan_all_installed_mods(&paths);
        let rows: Vec<(&str, bool, bool)> = mods.iter().map(|m| (m.id.as_str(), m.enabled, m.not_installed)).collect();
        assert_eq!(rows, vec![("ModA", true, false), ("Gone", true, true)]);

        // The same through a saved order.
        fs::write(
            z.join("PZModStudio_MasterLoadOrder.json"),
            r#"{"load_order":["Gone","ModA"],"active_mod_ids":["Gone","ModA"]}"#,
        )
        .unwrap();
        fs::write(z.join("mods").join("default.txt"), "VERSION = 1,\nmods\n{\n    mod = Gone,\n    mod = ModA,\n}\n").unwrap();
        let mods = scan_all_installed_mods(&paths);
        let rows: Vec<(&str, bool, bool)> = mods.iter().map(|m| (m.id.as_str(), m.enabled, m.not_installed)).collect();
        assert_eq!(rows, vec![("Gone", true, true), ("ModA", true, false)]);
        let _ = fs::remove_dir_all(&z);
    }

    /// Reads a real Workshop folder when one is given: PZMS_WORKSHOP_DIR=... cargo test
    #[test]
    fn test_subscribed_workshop_ids_filtering() {
        let Ok(dir) = std::env::var("PZMS_WORKSHOP_DIR") else { return };
        let workshop_dir = Path::new(&dir);
        if workshop_dir.exists() {
            let subs = get_subscribed_workshop_ids(workshop_dir);
            assert!(subs.is_some());
            let ids = subs.unwrap();
            assert!(!ids.is_empty(), "Must find active subscribed mods");
        }
    }

    /// Read-only report over a real Workshop folder. Run with:
    /// PZMS_WORKSHOP_DIR=<steamapps/workshop/content/108600> PZMS_GAME_VERSION=42.21 cargo test version_report -- --ignored --nocapture
    #[test]
    #[ignore]
    fn version_report_for_real_workshop() {
        let Ok(dir) = std::env::var("PZMS_WORKSHOP_DIR") else { return };
        let game = std::env::var("PZMS_GAME_VERSION").ok().and_then(|v| version_dir_int(&v));
        let (mut loadable, mut not_loadable, mut changed) = (0, 0, Vec::new());
        for entry in WalkDir::new(&dir).max_depth(8).into_iter().filter_map(|e| e.ok()) {
            if entry.file_name() != "mod.info" {
                continue;
            }
            match mod_info_rank(entry.path(), game) {
                Some((2, _)) => {
                    loadable += 1;
                    // Would the old scorer (highest 42.x regardless of build) have picked another folder?
                    let root = mod_root_of(entry.path()).unwrap();
                    if let Some(newest) = select_mod_info_for_build(root, None) {
                        if newest != entry.path() {
                            changed.push(format!("{} -> now {}", newest.display(), entry.path().display()));
                        }
                    }
                }
                Some(_) => not_loadable += 1,
                None => {}
            }
        }
        println!("loadable: {loadable}, not loadable on this build: {not_loadable}");
        println!("mods whose mod.info changes vs. 'newest folder wins': {}", changed.len());
        for c in changed.iter().take(15) {
            println!("  {c}");
        }
    }

    /// Builds `<tmp>/mods/<name>/<dir>/mod.info` for each dir ("" = the mod root).
    fn fake_mod(tag: &str, dirs: &[&str]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("pzms-vdir-{}-{}", tag, std::process::id())).join("mods").join("M");
        let _ = fs::remove_dir_all(root.parent().unwrap().parent().unwrap());
        for d in dirs {
            let dir = if d.is_empty() { root.clone() } else { root.join(d) };
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("mod.info"), format!("name=M\nid=M_{}\n", d.replace('.', "_"))).unwrap();
        }
        root
    }

    #[test]
    fn version_folder_selection_matches_the_game() {
        assert_eq!(version_dir_int("42"), Some(42_000));
        assert_eq!(version_dir_int("42.13.1"), Some(42_013));
        assert_eq!(version_dir_int("common"), None);

        // Same layout as Workshop item 2699828474: B41 root + common + several 42.x folders.
        let root = fake_mod("multi", &["", "common", "42.0", "42.13", "42.16", "42.21"]);
        let pick = |v: Option<u32>| select_mod_info_for_build(&root, v).map(|p| p.parent().unwrap().file_name().unwrap().to_string_lossy().to_string());
        assert_eq!(pick(Some(42_015)), Some("42.13".into()), "highest folder not newer than the build");
        assert_eq!(pick(Some(42_021)), Some("42.21".into()));
        assert_eq!(pick(Some(42_010)), Some("42.0".into()));
        assert_eq!(pick(None), Some("42.21".into()), "unknown build behaves like the newest");
        assert_eq!(pick(Some(41_078)), Some("M".into()), "Build 41 reads the root mod.info");

        // Version folder without its own mod.info falls back to common/, never to an older folder.
        let root2 = fake_mod("fallback", &["common", "42.13"]);
        fs::create_dir_all(root2.join("42.21")).unwrap();
        let chosen = select_mod_info_for_build(&root2, Some(42_021)).unwrap();
        assert_eq!(chosen.parent().unwrap().file_name().unwrap(), "common", "{:?}", chosen);

        // B41-only mod on B42: not loadable, but still listed through its root mod.info.
        let root3 = fake_mod("b41only", &[""]);
        assert_eq!(select_mod_info_for_build(&root3, Some(42_021)), None);
        assert_eq!(mod_info_rank(&root3.join("mod.info"), Some(42_021)), Some((1, Some(false))));
        assert_eq!(mod_info_rank(&root.join("42.13").join("mod.info"), Some(42_015)), Some((2, Some(true))));
        assert_eq!(mod_info_rank(&root.join("42.21").join("mod.info"), Some(42_015)), None);

        for r in [root, root2, root3] {
            let _ = fs::remove_dir_all(r.parent().unwrap().parent().unwrap());
        }
    }

    /// Live, read only: `cargo test --lib -- --ignored live_install_dates --nocapture`.
    #[test]
    #[ignore]
    fn live_install_dates() {
        let mut mods = scan_all_installed_mods(&crate::vfs::studio_paths());
        let dated = mods.iter().filter(|m| m.installed_at.is_some()).count();
        mods.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));
        println!("{} mods, {} with a date", mods.len(), dated);
        for m in mods.iter().take(5).chain(mods.iter().rev().take(3)) {
            println!("  {:?} {} {:?}", m.installed_at.map(|t| chrono::DateTime::from_timestamp(t as i64, 0).unwrap().format("%Y-%m-%d %H:%M").to_string()), m.name, m.workshop_id);
        }
    }
}
