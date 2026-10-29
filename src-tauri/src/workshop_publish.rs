//! Publishing a mod to the Steam Workshop from the Studio, with the game's own files.
//!
//! The game's uploader works from `Zomboid/Workshop/<item>/`: `workshop.txt` (title, the
//! description one `description=` line per line, `tags=A;B`, `visibility=`, and `id=` once
//! published), `preview.png`, and `Contents/mods/<mod>/` with the mod itself. The Studio reads
//! and writes the same files, so an item made here is one the game's screen shows, and the
//! other way round. What the game's uploader does not do and this does: a `.workshopignore`
//! (one pattern per line, gitignore-like: `*.psd`, `.git/`, `/docs`) kept out of the upload
//! through a filtered copy, a look at the files before sending with what changed since the
//! last upload from here (a manifest of hashes in `PZModStudio_Cache/workshop_uploads/`), and
//! the "Workshop ID / Mod ID" lines added to the description once, not once per upload.
//!
//! The upload itself goes through the running Steam client, with the game's `steam_api64.dll`
//! (`steam_client::publish_item`): `CreateItem` for a new item, `StartItemUpdate` +
//! `SubmitItemUpdate` for every upload, progress read while it runs. Steam asks a first-time
//! publisher to accept the Workshop legal agreement on its site; the result says so.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One item folder in `Zomboid/Workshop`, as its `workshop.txt` describes it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkshopItemMeta {
    /// The folder's name in `Zomboid/Workshop`.
    pub folder: String,
    pub path: String,
    /// Steam's id once published.
    pub id: Option<u64>,
    pub title: String,
    /// The description lines joined with line breaks.
    pub description: String,
    pub tags: Vec<String>,
    /// `public`, `friends`, `private` or `unlisted`.
    pub visibility: String,
    pub has_preview: bool,
    /// The mods inside `Contents/mods`.
    pub mod_ids: Vec<String>,
    /// A subscribed Workshop item of someone else ships one of these mods (its id): the mod is
    /// theirs, and the Studio will not upload it. None when this item is that item (same id).
    #[serde(default)]
    pub published_elsewhere: Option<String>,
    /// Lines of workshop.txt this reader does not know, kept as they are.
    #[serde(default)]
    pub other_lines: Vec<String>,
}

pub fn parse_workshop_txt(folder: &str, path: &str, text: &str) -> WorkshopItemMeta {
    let mut m = WorkshopItemMeta {
        folder: folder.to_string(),
        path: path.to_string(),
        id: None,
        title: String::new(),
        description: String::new(),
        tags: Vec::new(),
        visibility: "public".to_string(),
        has_preview: false,
        mod_ids: Vec::new(),
        published_elsewhere: None,
        other_lines: Vec::new(),
    };
    let mut description: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        let Some((key, value)) = line.split_once('=') else {
            if !line.trim().is_empty() {
                m.other_lines.push(line.to_string());
            }
            continue;
        };
        match key.trim() {
            "version" => {}
            "id" => m.id = value.trim().parse().ok(),
            "title" => m.title = value.trim().to_string(),
            "description" => description.push(value.to_string()),
            "tags" => m.tags = value.split(';').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
            "visibility" => m.visibility = value.trim().to_lowercase(),
            _ => m.other_lines.push(line.to_string()),
        }
    }
    m.description = description.join("\n");
    m
}

/// `workshop.txt` as the game writes it.
pub fn workshop_txt(m: &WorkshopItemMeta) -> String {
    let mut out = String::from("version=1\n");
    if let Some(id) = m.id {
        out.push_str(&format!("id={}\n", id));
    }
    out.push_str(&format!("title={}\n", m.title.trim()));
    for line in m.description.lines() {
        out.push_str(&format!("description={}\n", line.trim_end_matches('\r')));
    }
    out.push_str(&format!("tags={}\n", m.tags.iter().map(|t| t.trim()).filter(|t| !t.is_empty()).collect::<Vec<_>>().join(";")));
    out.push_str(&format!("visibility={}\n", if m.visibility.is_empty() { "public" } else { m.visibility.as_str() }));
    for l in &m.other_lines {
        out.push_str(l);
        out.push('\n');
    }
    out
}

fn workshop_dir(user_zomboid_dir: &Path) -> PathBuf {
    user_zomboid_dir.join("Workshop")
}

fn mods_inside(item: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    let mods = item.join("Contents").join("mods");
    for e in std::fs::read_dir(mods).into_iter().flatten().flatten() {
        if e.path().is_dir() {
            let folder = e.path();
            let info = crate::load_order::mod_info::select_mod_info_for_build(&folder, None).or_else(|| folder.join("mod.info").is_file().then(|| folder.join("mod.info")));
            let id = info
                .and_then(|p| std::fs::read(p).ok())
                .map(|b| crate::catalog::modinfo::parse_game_mod_info(&String::from_utf8_lossy(&b)))
                .and_then(|i| i.id)
                .unwrap_or_else(|| e.file_name().to_string_lossy().to_string());
            ids.push(id);
        }
    }
    ids
}

/// Mod id -> the subscribed Workshop item that ships it. A mod that arrives through a
/// subscription is someone's published work; the only item allowed to upload it again is the
/// one with that same id (the author's own upload folder).
pub fn subscribed_mods(user_zomboid_dir: &Path, workshop_dir: &str) -> BTreeMap<String, String> {
    let catalog = crate::catalog::build_catalog(workshop_dir, &user_zomboid_dir.to_string_lossy(), None);
    let mut out = BTreeMap::new();
    for m in &catalog.mods {
        if let crate::catalog::ModSource::Workshop { workshop_id } = &m.source {
            out.insert(m.id.clone(), workshop_id.clone());
        }
    }
    out
}

/// The subscribed item of someone else that ships one of this item's mods, if any.
fn elsewhere(item: &WorkshopItemMeta, subscribed: &BTreeMap<String, String>) -> Option<String> {
    item.mod_ids
        .iter()
        .filter_map(|id| subscribed.get(id))
        .find(|wid| item.id.map(|own| own.to_string()) .as_deref() != Some(wid.as_str()))
        .cloned()
}

fn read_item(item: &Path) -> Option<WorkshopItemMeta> {
    let folder = item.file_name()?.to_string_lossy().to_string();
    let text = std::fs::read_to_string(item.join("workshop.txt")).unwrap_or_default();
    let mut m = parse_workshop_txt(&folder, &item.to_string_lossy(), &text);
    if m.title.is_empty() {
        m.title = folder.clone();
    }
    m.has_preview = item.join("preview.png").is_file();
    m.mod_ids = mods_inside(item);
    Some(m)
}

/// Every item folder of `Zomboid/Workshop` (one with a `Contents` folder or a `workshop.txt`),
/// each marked when a subscribed item of someone else ships its mod.
pub fn list_uploads(user_zomboid_dir: &Path, workshop: &str) -> Vec<WorkshopItemMeta> {
    let subscribed = subscribed_mods(user_zomboid_dir, workshop);
    let mut out = Vec::new();
    for e in std::fs::read_dir(workshop_dir(user_zomboid_dir)).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() && (p.join("Contents").is_dir() || p.join("workshop.txt").is_file()) {
            if let Some(mut m) = read_item(&p) {
                m.published_elsewhere = elsewhere(&m, &subscribed);
                out.push(m);
            }
        }
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out
}

fn item_dir(user_zomboid_dir: &Path, folder: &str) -> Result<PathBuf, String> {
    if folder.is_empty() || folder.contains(['/', '\\']) || folder == "." || folder == ".." {
        return Err("Not a Workshop item folder name.".to_string());
    }
    let dir = workshop_dir(user_zomboid_dir).join(folder);
    if !dir.is_dir() {
        return Err(format!("{} is not in Zomboid/Workshop.", folder));
    }
    Ok(dir)
}

/// Writes the title, description, tags and visibility into the item's `workshop.txt`.
pub fn save_meta(user_zomboid_dir: &Path, folder: &str, title: &str, description: &str, tags: &[String], visibility: &str) -> Result<WorkshopItemMeta, String> {
    let dir = item_dir(user_zomboid_dir, folder)?;
    let mut m = read_item(&dir).ok_or("Could not read the item.")?;
    if title.trim().is_empty() {
        return Err("The title cannot be empty.".to_string());
    }
    m.title = title.trim().to_string();
    m.description = description.replace("\r\n", "\n");
    m.tags = tags.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    m.visibility = match visibility {
        "friends" | "private" | "unlisted" => visibility.to_string(),
        _ => "public".to_string(),
    };
    std::fs::write(dir.join("workshop.txt"), workshop_txt(&m)).map_err(|e| format!("Could not write workshop.txt: {}", e))?;
    read_item(&dir).ok_or_else(|| "Could not read the item back.".to_string())
}

/// Makes `Zomboid/Workshop/<folder>/` for a mod that has no item yet: the mod copied under
/// `Contents/mods/<its folder>`, a `workshop.txt` with its name and description, and its
/// poster as `preview.png` when it has one.
pub fn prepare_upload(user_zomboid_dir: &Path, workshop: &str, mod_root: &Path) -> Result<WorkshopItemMeta, String> {
    let info_path = crate::load_order::mod_info::select_mod_info_for_build(mod_root, None)
        .or_else(|| mod_root.join("mod.info").is_file().then(|| mod_root.join("mod.info")))
        .ok_or("The folder has no mod.info.")?;
    let info = crate::catalog::modinfo::parse_game_mod_info(&String::from_utf8_lossy(&std::fs::read(&info_path).map_err(|e| e.to_string())?));
    let id = info.id.clone().ok_or("The mod.info has no id= line.")?;
    if let Some(wid) = subscribed_mods(user_zomboid_dir, workshop).get(&id) {
        return Err(format!("{} is already on the Workshop as item {}, from a subscription: it is its author's to publish, not the Studio's to upload again.", id, wid));
    }
    let folder_name = mod_root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(id.clone());
    let item = workshop_dir(user_zomboid_dir).join(&folder_name);
    if item.exists() {
        return Err(format!("Zomboid/Workshop/{} is already there: pick it from the list.", folder_name));
    }
    let dest = item.join("Contents").join("mods").join(&folder_name);
    crate::mod_install::copy_tree(mod_root, &dest).map_err(|e| format!("Could not copy the mod: {}", e))?;
    let meta = WorkshopItemMeta {
        folder: folder_name.clone(),
        path: item.to_string_lossy().to_string(),
        id: None,
        title: info.name.clone().unwrap_or(id.clone()),
        description: info.description.clone(),
        tags: Vec::new(),
        visibility: "public".to_string(),
        has_preview: false,
        mod_ids: vec![id],
        published_elsewhere: None,
        other_lines: Vec::new(),
    };
    std::fs::write(item.join("workshop.txt"), workshop_txt(&meta)).map_err(|e| e.to_string())?;
    for name in ["poster.png", "preview.png", "icon.png"] {
        let poster = info_path.parent().map(|p| p.join(name)).filter(|p| p.is_file()).or_else(|| mod_root.join(name).is_file().then(|| mod_root.join(name)));
        if let Some(p) = poster {
            let _ = std::fs::copy(p, item.join("preview.png"));
            break;
        }
    }
    read_item(&item).ok_or_else(|| "Could not read the item back.".to_string())
}

/// Takes an item folder out of `Zomboid/Workshop`: to the Recycle Bin, so it can come back.
/// Nothing on Steam changes.
pub fn remove_item(user_zomboid_dir: &Path, folder: &str) -> Result<(), String> {
    let dir = item_dir(user_zomboid_dir, folder)?;
    crate::disk_usage::to_recycle_bin(&dir)
}

/// gitignore-like patterns of a `.workshopignore` in the item folder or its `Contents`.
#[derive(Debug, Clone, Default)]
pub struct IgnoreRules {
    patterns: Vec<(String, bool)>, // (pattern without a trailing slash, directories only)
}

/// What no upload wants: version control, editor and OS leftovers, the rules file itself.
const ALWAYS_IGNORED: &[&str] = &[".git/", ".svn/", ".hg/", ".idea/", ".vscode/", "node_modules/", ".DS_Store", "Thumbs.db", "desktop.ini", ".workshopignore", "*.psd", "*.xcf", "*.blend", "*.blend1", "*.bak", "*.tmp", "*~"];

fn glob_match(pattern: &str, text: &str) -> bool {
    // `*` any run, `?` one char, the rest literal; case-insensitive (Windows paths).
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    fn go(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') => (0..=t.len()).any(|i| go(&p[1..], &t[i..])),
            Some('?') => !t.is_empty() && go(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && go(&p[1..], &t[1..]),
        }
    }
    go(&p, &t)
}

impl IgnoreRules {
    pub fn parse(text: &str) -> IgnoreRules {
        let mut rules = IgnoreRules::default();
        for raw in ALWAYS_IGNORED.iter().map(|s| s.to_string()).chain(text.lines().map(|l| l.to_string())) {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let dir_only = line.ends_with('/');
            let pat = line.trim_end_matches('/').replace('\\', "/");
            rules.patterns.push((pat, dir_only));
        }
        rules
    }

    pub fn load(item: &Path) -> IgnoreRules {
        let text = std::fs::read_to_string(item.join(".workshopignore"))
            .or_else(|_| std::fs::read_to_string(item.join("Contents").join(".workshopignore")))
            .unwrap_or_default();
        IgnoreRules::parse(&text)
    }

    /// `rel` is the path under `Contents`, with `/`.
    pub fn ignores(&self, rel: &str, is_dir: bool) -> bool {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        for (pat, dir_only) in &self.patterns {
            if *dir_only && !is_dir {
                // a directory rule also hides what is inside it
                let dir_pat = pat.trim_start_matches('/');
                if rel.split('/').any(|seg| glob_match(dir_pat, seg)) && !pat.contains('/') {
                    return true;
                }
                if pat.starts_with('/') && (rel.starts_with(&format!("{}/", dir_pat)) || glob_match(&format!("{}/*", dir_pat), rel)) {
                    return true;
                }
                continue;
            }
            if let Some(anchored) = pat.strip_prefix('/') {
                if glob_match(anchored, rel) || rel.starts_with(&format!("{}/", anchored)) {
                    return true;
                }
            } else if pat.contains('/') {
                if glob_match(pat, rel) || rel.starts_with(&format!("{}/", pat)) {
                    return true;
                }
            } else if glob_match(pat, name) {
                return true;
            }
        }
        false
    }
}

/// One file of the upload, and how it compares with the last upload from here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFile {
    pub path: String,
    pub bytes: u64,
    /// `new`, `changed`, `same`, or `removed` (in the last upload, not here now).
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadPreview {
    pub item: WorkshopItemMeta,
    pub files: Vec<UploadFile>,
    pub ignored: Vec<String>,
    pub total_bytes: u64,
    pub new_count: usize,
    pub changed_count: usize,
    pub same_count: usize,
    pub removed_count: usize,
    /// The last upload from the Studio: unix time, or None.
    pub last_upload_at: Option<u64>,
    pub has_ignore_file: bool,
    /// What Steam will not take: no Contents, no mod inside, no title.
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Manifest {
    id: u64,
    at: u64,
    /// Relative path -> hash.
    files: BTreeMap<String, String>,
}

fn manifest_path(user_zomboid_dir: &Path, folder: &str) -> PathBuf {
    user_zomboid_dir.join("PZModStudio_Cache").join("workshop_uploads").join(format!("{}.json", crate::security::safe_name(folder)))
}

fn hash_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Some(format!("{:x}", h.finalize())[..24].to_string())
}

/// The files under `Contents` the upload takes, relative with `/`, and the ones the rules keep out.
fn walk_contents(item: &Path, rules: &IgnoreRules) -> (Vec<(String, PathBuf, u64)>, Vec<String>) {
    let contents = item.join("Contents");
    let mut files = Vec::new();
    let mut ignored = Vec::new();
    let mut it = walkdir::WalkDir::new(&contents).follow_links(false).min_depth(1).into_iter();
    while let Some(Ok(e)) = it.next() {
        let rel = e.path().strip_prefix(&contents).unwrap_or(e.path()).to_string_lossy().replace('\\', "/");
        let is_dir = e.file_type().is_dir();
        if rules.ignores(&rel, is_dir) {
            ignored.push(if is_dir { format!("{}/", rel) } else { rel });
            if is_dir {
                it.skip_current_dir();
            }
            continue;
        }
        if !is_dir {
            let bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
            files.push((rel, e.path().to_path_buf(), bytes));
        }
    }
    files.sort();
    ignored.sort();
    (files, ignored)
}

/// What an upload of this item would send, against the last upload from here.
pub fn preview(user_zomboid_dir: &Path, workshop: &str, folder: &str) -> Result<UploadPreview, String> {
    let dir = item_dir(user_zomboid_dir, folder)?;
    let mut item = read_item(&dir).ok_or("Could not read the item.")?;
    item.published_elsewhere = elsewhere(&item, &subscribed_mods(user_zomboid_dir, workshop));
    let rules = IgnoreRules::load(&dir);
    let (walked, ignored) = walk_contents(&dir, &rules);
    let manifest: Option<Manifest> = std::fs::read_to_string(manifest_path(user_zomboid_dir, folder)).ok().and_then(|t| serde_json::from_str(&t).ok());
    let mut files = Vec::new();
    let mut total = 0u64;
    let (mut new_count, mut changed_count, mut same_count) = (0, 0, 0);
    for (rel, path, bytes) in &walked {
        total += bytes;
        let state = match manifest.as_ref().and_then(|m| m.files.get(rel)) {
            None => "new",
            Some(old) => {
                if hash_file(path).as_deref() == Some(old.as_str()) {
                    "same"
                } else {
                    "changed"
                }
            }
        };
        match state {
            "new" => new_count += 1,
            "changed" => changed_count += 1,
            _ => same_count += 1,
        }
        files.push(UploadFile { path: rel.clone(), bytes: *bytes, state: state.to_string() });
    }
    let mut removed_count = 0;
    if let Some(m) = &manifest {
        for old in m.files.keys() {
            if !walked.iter().any(|(rel, _, _)| rel == old) {
                removed_count += 1;
                files.push(UploadFile { path: old.clone(), bytes: 0, state: "removed".to_string() });
            }
        }
    }
    let mut problems = Vec::new();
    if !dir.join("Contents").is_dir() {
        problems.push("No Contents folder: the game's uploader puts the mod in Contents/mods/<mod>.".to_string());
    } else if item.mod_ids.is_empty() {
        problems.push("No mod in Contents/mods: nothing to publish.".to_string());
    }
    if item.title.trim().is_empty() {
        problems.push("No title.".to_string());
    }
    if !item.has_preview {
        problems.push("No preview.png: the item will show without a picture.".to_string());
    }
    if let Some(wid) = &item.published_elsewhere {
        problems.push(format!("Someone else's mod: item {} on the Workshop ships it (you are subscribed to it). The Studio does not upload other people's mods. If that item is yours, put id={} in workshop.txt and Steam will check the account.", wid, wid));
    }
    Ok(UploadPreview {
        item,
        files,
        ignored,
        total_bytes: total,
        new_count,
        changed_count,
        same_count,
        removed_count,
        last_upload_at: manifest.as_ref().map(|m| m.at),
        has_ignore_file: dir.join(".workshopignore").is_file() || dir.join("Contents").join(".workshopignore").is_file(),
        problems,
    })
}

/// The description Steam gets: the item's, with the game's "Workshop ID / Mod ID" lines once.
pub fn description_for_steam(m: &WorkshopItemMeta, id: Option<u64>) -> String {
    let mut text = m.description.trim_end().to_string();
    let has_ids = text.lines().any(|l| l.trim_start().starts_with("Mod ID:"));
    if !has_ids && !m.mod_ids.is_empty() {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        if let Some(id) = id {
            text.push_str(&format!("Workshop ID: {}\n", id));
        }
        for mid in &m.mod_ids {
            text.push_str(&format!("Mod ID: {}\n", mid));
        }
    }
    text
}

fn visibility_code(v: &str) -> i32 {
    match v {
        "friends" => 1,
        "private" => 2,
        "unlisted" => 3,
        _ => 0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishOutcome {
    pub id: u64,
    pub created: bool,
    pub files: usize,
    pub bytes: u64,
    /// Steam wants the Workshop legal agreement accepted on its site first; the item is there
    /// but hidden until then.
    pub needs_legal_agreement: bool,
    pub url: String,
}

/// Stage and bytes of an upload under way, for the screen.
#[derive(Debug, Clone, Serialize)]
pub struct PublishProgress {
    pub folder: String,
    /// `copying`, `creating`, `preparing`, `uploading`, `preview`, `committing`, `done`.
    pub stage: String,
    pub processed: u64,
    pub total: u64,
}

/// Publishes the item: a filtered copy of `Contents` is what Steam reads, the item is created
/// when it has no id yet (and the id written to workshop.txt at once, before the upload), the
/// manifest of what went up is kept for the next look.
pub fn publish(pz_install_dir: &str, user_zomboid_dir: &Path, workshop: &str, folder: &str, change_note: &str, mut progress: impl FnMut(PublishProgress)) -> Result<PublishOutcome, String> {
    let dir = item_dir(user_zomboid_dir, folder)?;
    let mut item = read_item(&dir).ok_or("Could not read the item.")?;
    if let Some(wid) = elsewhere(&item, &subscribed_mods(user_zomboid_dir, workshop)) {
        return Err(format!("Not uploaded: item {} on the Workshop already ships this mod, and it is not this item. The Studio does not upload other people's mods.", wid));
    }
    if !dir.join("Contents").is_dir() || item.mod_ids.is_empty() {
        return Err("Nothing to publish: Contents/mods/<mod> is missing.".to_string());
    }
    if item.title.trim().is_empty() {
        return Err("The item has no title.".to_string());
    }
    // 1. The filtered copy.
    progress(PublishProgress { folder: folder.to_string(), stage: "copying".into(), processed: 0, total: 0 });
    let rules = IgnoreRules::load(&dir);
    let (files, _) = walk_contents(&dir, &rules);
    let staging = user_zomboid_dir.join("PZModStudio_Cache").join("workshop_upload").join(crate::security::safe_name(folder));
    let _ = std::fs::remove_dir_all(&staging);
    let content = staging.join("Contents");
    std::fs::create_dir_all(&content).map_err(|e| e.to_string())?;
    let mut bytes = 0u64;
    let mut manifest = Manifest::default();
    for (rel, path, size) in &files {
        let to = content.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::copy(path, &to).map_err(|e| format!("Could not copy {}: {}", rel, e))?;
        bytes += size;
        if let Some(h) = hash_file(path) {
            manifest.files.insert(rel.clone(), h);
        }
    }
    let preview_png = dir.join("preview.png").is_file().then(|| dir.join("preview.png"));
    // 2. Steam.
    let request = crate::steam_client::PublishRequest {
        item_id: item.id,
        title: item.title.clone(),
        description: description_for_steam(&item, item.id),
        tags: item.tags.clone(),
        visibility: visibility_code(&item.visibility),
        content_dir: content.clone(),
        preview: preview_png,
        change_note: change_note.to_string(),
    };
    let folder_name = folder.to_string();
    let mut on_created = |id: u64| {
        // The id into workshop.txt at once: an upload that fails after this still has its item.
        item.id = Some(id);
        let _ = std::fs::write(dir.join("workshop.txt"), workshop_txt(&item));
    };
    let result = crate::steam_client::publish_item(pz_install_dir, request, &mut on_created, |stage, processed, total| {
        progress(PublishProgress { folder: folder_name.clone(), stage: stage.to_string(), processed, total });
    });
    let _ = std::fs::remove_dir_all(&staging);
    let (id, legal, created) = result?;
    // 3. Remembered.
    manifest.id = id;
    manifest.at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Some(parent) = manifest_path(user_zomboid_dir, folder).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(manifest_path(user_zomboid_dir, folder), serde_json::to_string(&manifest).unwrap_or_default());
    progress(PublishProgress { folder: folder.to_string(), stage: "done".into(), processed: bytes, total: bytes });
    Ok(PublishOutcome { id, created, files: files.len(), bytes, needs_legal_agreement: legal, url: format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", id) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workshop_txt_round_trips_the_games_shape() {
        let text = "version=1\nid=3824256253\ntitle=PZ Mod Manager Studio Link\ndescription=[b]PZ Mod Studio[/b] line one\ndescription=\ndescription=line three\ntags=Build 42;Framework;QoL\nvisibility=public\n";
        let m = parse_workshop_txt("Z_StudioLink", "x", text);
        assert_eq!(m.id, Some(3824256253));
        assert_eq!(m.title, "PZ Mod Manager Studio Link");
        assert_eq!(m.description, "[b]PZ Mod Studio[/b] line one\n\nline three");
        assert_eq!(m.tags, vec!["Build 42", "Framework", "QoL"]);
        assert_eq!(workshop_txt(&m), text);
        let fresh = parse_workshop_txt("New", "x", "");
        assert_eq!(fresh.visibility, "public");
        assert!(workshop_txt(&fresh).starts_with("version=1\ntitle=\n"));
    }

    #[test]
    fn ignore_rules_read_like_gitignore_and_always_drop_the_leftovers() {
        let r = IgnoreRules::parse("# comment\n*.psd\n/docs\nbuild/\nsrc/*.txt\n");
        assert!(r.ignores(".git", true) && r.ignores(".git/config", false));
        assert!(r.ignores("mods/X/Thumbs.db", false));
        assert!(r.ignores("mods/X/art/hero.PSD", false));
        assert!(r.ignores("docs", true) && r.ignores("docs/readme.md", false) && !r.ignores("mods/docs/readme.md", false));
        assert!(r.ignores("mods/X/build", true) && r.ignores("mods/X/build/out.jar", false));
        assert!(r.ignores("src/notes.txt", false) && !r.ignores("src/notes.lua", false));
        assert!(!r.ignores("mods/X/mod.info", false));
    }

    #[test]
    fn the_description_gets_the_id_lines_once() {
        let mut m = parse_workshop_txt("X", "x", "title=T\ndescription=Hello\n");
        m.mod_ids = vec!["MyMod".into()];
        assert_eq!(description_for_steam(&m, Some(42)), "Hello\n\nWorkshop ID: 42\nMod ID: MyMod\n");
        m.description = "Hello\n\nWorkshop ID: 42\nMod ID: MyMod".into();
        assert_eq!(description_for_steam(&m, Some(42)), "Hello\n\nWorkshop ID: 42\nMod ID: MyMod");
    }

    #[test]
    fn a_preview_lists_the_files_and_what_changed_since_the_last_upload() {
        let z = std::env::temp_dir().join(format!("pzms-publish-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&z);
        let item = z.join("Workshop").join("MyItem");
        let m = item.join("Contents").join("mods").join("MyMod");
        std::fs::create_dir_all(m.join("media").join("lua")).unwrap();
        std::fs::create_dir_all(m.join(".git")).unwrap();
        std::fs::write(m.join("mod.info"), "name=My Mod\nid=MyMod\n").unwrap();
        std::fs::write(m.join("media").join("lua").join("a.lua"), "print(1)").unwrap();
        std::fs::write(m.join("art.psd"), "x").unwrap();
        std::fs::write(m.join(".git").join("HEAD"), "ref").unwrap();
        std::fs::write(item.join("workshop.txt"), "version=1\ntitle=My Item\ntags=QoL\nvisibility=public\n").unwrap();
        let p = preview(&z, "", "MyItem").unwrap();
        assert_eq!(p.item.mod_ids, vec!["MyMod"]);
        assert_eq!(p.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), vec!["mods/MyMod/media/lua/a.lua", "mods/MyMod/mod.info"]);
        assert!(p.ignored.contains(&"mods/MyMod/.git/".to_string()) && p.ignored.contains(&"mods/MyMod/art.psd".to_string()));
        assert_eq!((p.new_count, p.changed_count, p.same_count, p.removed_count), (2, 0, 0, 0));
        assert!(p.problems.iter().any(|x| x.contains("preview.png")));
        // a manifest from before: one same, one changed, one removed
        let mut files = BTreeMap::new();
        files.insert("mods/MyMod/mod.info".to_string(), hash_file(&m.join("mod.info")).unwrap());
        files.insert("mods/MyMod/media/lua/a.lua".to_string(), "old".to_string());
        files.insert("mods/MyMod/gone.lua".to_string(), "old".to_string());
        let mp = manifest_path(&z, "MyItem");
        std::fs::create_dir_all(mp.parent().unwrap()).unwrap();
        std::fs::write(&mp, serde_json::to_string(&Manifest { id: 1, at: 5, files }).unwrap()).unwrap();
        let p = preview(&z, "", "MyItem").unwrap();
        assert_eq!((p.new_count, p.changed_count, p.same_count, p.removed_count), (0, 1, 1, 1));
        assert_eq!(p.last_upload_at, Some(5));
        // a mod prepared from a folder
        let local = z.join("mods").join("Other");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::write(local.join("mod.info"), "name=Other Mod\nid=Other\ndescription=Does things\nposter=poster.png\n").unwrap();
        std::fs::write(local.join("poster.png"), "png").unwrap();
        let made = prepare_upload(&z, "", &local).unwrap();
        assert_eq!((made.title.as_str(), made.description.as_str(), made.has_preview, made.id), ("Other Mod", "Does things", true, None));
        assert!(z.join("Workshop").join("Other").join("Contents").join("mods").join("Other").join("mod.info").is_file());
        assert!(prepare_upload(&z, "", &local).is_err(), "already there");
        // a mod that comes through a subscription is someone else's
        let ws = z.join("workshop");
        let theirs = ws.join("555").join("mods").join("Theirs");
        std::fs::create_dir_all(&theirs).unwrap();
        std::fs::write(theirs.join("mod.info"), "name=Theirs\nid=Theirs\n").unwrap();
        let copy = z.join("mods").join("TheirsCopy");
        std::fs::create_dir_all(&copy).unwrap();
        std::fs::write(copy.join("mod.info"), "name=Theirs\nid=Theirs\n").unwrap();
        let ws_s = ws.to_string_lossy().to_string();
        assert!(subscribed_mods(&z, &ws_s).get("Theirs").is_some(), "the catalog sees the subscribed item");
        assert!(prepare_upload(&z, &ws_s, &copy).unwrap_err().contains("555"));
        let item = z.join("Workshop").join("TheirsItem").join("Contents").join("mods").join("Theirs");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("mod.info"), "name=Theirs\nid=Theirs\n").unwrap();
        std::fs::write(z.join("Workshop").join("TheirsItem").join("workshop.txt"), "version=1\ntitle=Theirs\n").unwrap();
        let listed = list_uploads(&z, &ws_s);
        assert_eq!(listed.iter().find(|u| u.folder == "TheirsItem").unwrap().published_elsewhere.as_deref(), Some("555"));
        assert!(publish("", &z, &ws_s, "TheirsItem", "", |_| {}).unwrap_err().contains("other people"));
        // the author's own upload folder of that item carries its id: allowed
        std::fs::write(z.join("Workshop").join("TheirsItem").join("workshop.txt"), "version=1\nid=555\ntitle=Theirs\n").unwrap();
        assert!(list_uploads(&z, &ws_s).iter().find(|u| u.folder == "TheirsItem").unwrap().published_elsewhere.is_none());
        let saved = save_meta(&z, "Other", "Other Mod!", "a\nb", &["Build 42".into(), "".into()], "unlisted").unwrap();
        assert_eq!((saved.title.as_str(), saved.description.as_str(), saved.visibility.as_str()), ("Other Mod!", "a\nb", "unlisted"));
        assert_eq!(saved.tags, vec!["Build 42"]);
        let _ = std::fs::remove_dir_all(&z);
    }
}
