pub mod mod_patch;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioPaths {
    pub pz_install_dir: String,
    pub workshop_dir: String,
    pub user_zomboid_dir: String,
    pub mod_list_ini_path: String,
    pub carrier_workshop_id: Option<String>,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetingModFileRaw {
    pub mod_id: String,
    pub mod_name: String,
    pub absolute_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsConflictRaw {
    pub id: String,
    pub relative_path: String,
    pub file_type: String, // "LUA" or "SCRIPT_TXT"
    pub start_line: usize,
    pub end_line: usize,
    pub conflict_line: usize,
    pub total_file_lines: usize,
    pub base_content: String,
    pub competing_files: Vec<CompetingModFileRaw>,
    pub merged_output: String,
    pub auto_ast_output: String,
    pub is_identical_noise: bool,
    pub status: Option<String>,
    /// What each mod changes compared to vanilla (empty when vanilla has no such file).
    #[serde(default)]
    pub mod_changes: Vec<crate::diff_engine::lua::ModChangeSummary>,
    /// Vanilla line ranges that several mods change differently.
    #[serde(default)]
    pub collisions: Vec<crate::diff_engine::lua::CollisionRegion>,
    /// How the merged output was built, in plain words.
    #[serde(default)]
    pub strategy: String,
    /// The automatic merge needs no decision (see `can_auto_merge`).
    #[serde(default)]
    pub auto_mergeable: bool,
}

/// Where a shared file stands for the active list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConflictState {
    pub relative_path: String,
    /// Mod names in load order (the last one is the copy the game uses without a merge).
    pub mods: Vec<String>,
    pub mod_ids: Vec<String>,
    /// "pending": not merged; "stale": merged, but a mod changed since; "merged": covered by the
    /// active PZ Mod Studio package.
    pub state: String,
    pub auto_mergeable: bool,
    pub strategy: String,
    pub collisions: usize,
}

/// The active, packaged PZ Mod Studio merge package (it loads last, so its files win).
pub fn active_package_dir(catalog: &crate::catalog::ModCatalog, active: &[String]) -> Option<(String, PathBuf)> {
    active.iter().rev().find_map(|entry| {
        let m = catalog.resolve(entry.trim())?;
        let is_package = m.id.starts_with("Z_PZModStudio_") && m.id != "Z_PZModStudio_Bridge" && !m.id.contains("Carrier");
        let root = PathBuf::from(&m.root_dir);
        let packaged = fs::read_to_string(root.join("patch_metadata.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["is_packaged"].as_bool())
            .unwrap_or(false);
        (is_package && packaged).then(|| (m.id.clone(), root))
    })
}

fn modified(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Every real file conflict of `active` and whether the active Studio package already covers it.
pub fn file_conflict_states(paths: &StudioPaths, catalog: &crate::catalog::ModCatalog, active: &[String]) -> Vec<FileConflictState> {
    let package = active_package_dir(catalog, active);
    scan_conflicts_with(paths, catalog, active)
        .into_iter()
        // A definition conflict gets a row of its own, which says what the two mods disagree
        // about; reported here as well it would be the same thing twice, in worse words.
        .filter(|c| !c.is_identical_noise && c.file_type != "SCRIPT_DEFINITION")
        .map(|c| {
            let in_package = package.as_ref().and_then(|(_, root)| {
                [root.join("42").join(&c.relative_path), root.join(&c.relative_path)].into_iter().find(|p| p.is_file())
            });
            let state = match in_package {
                None => "pending",
                Some(merged) => {
                    let merged_at = modified(&merged);
                    let newer = c.competing_files.iter().any(|f| matches!((modified(Path::new(&f.absolute_path)), merged_at), (Some(a), Some(b)) if a > b));
                    if newer { "stale" } else { "merged" }
                }
            };
            FileConflictState {
                mods: c.competing_files.iter().map(|f| f.mod_name.clone()).collect(),
                mod_ids: c.competing_files.iter().map(|f| f.mod_id.clone()).collect(),
                state: state.to_string(),
                auto_mergeable: c.auto_mergeable,
                strategy: c.strategy.clone(),
                collisions: c.collisions.len(),
                relative_path: c.relative_path,
            }
        })
        .collect()
}

/// For each definition conflict, whether the active package already settles it ("merged"), does
/// but was made before one of the mods changed ("stale"), or does not ("pending").
pub fn definition_merge_states(
    catalog: &crate::catalog::ModCatalog,
    active: &[String],
    conflicts: &[crate::scripts::ScriptDefConflict],
) -> Vec<String> {
    let package = active_package_dir(catalog, active);
    conflicts
        .iter()
        .map(|c| {
            let rel = crate::scripts::merged_file_path(c);
            let in_package = package.as_ref().and_then(|(_, root)| {
                [root.join("42").join(&rel), root.join(&rel)].into_iter().find(|p| p.is_file())
            });
            match in_package {
                None => "pending".to_string(),
                Some(merged) => {
                    let merged_at = modified(&merged);
                    let newer = c.absolute_files.iter().any(|f| {
                        matches!((modified(Path::new(f)), merged_at), (Some(a), Some(b)) if a > b)
                    });
                    if newer { "stale".to_string() } else { "merged".to_string() }
                }
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoMergeResult {
    pub package_folder: String,
    /// Files merged automatically in this run.
    pub merged: Vec<String>,
    /// Requested files that need a manual decision (left out).
    pub skipped: Vec<String>,
}

/// Merges the requested files automatically (only those that need no decision) into the active
/// PZ Mod Studio package (or Z_PZModStudio_MergedPatch), keeping its other resolved files, and
/// publishes it (active, loading last).
/// `package_folder` is the package the user is working in; without one the active package is
/// used, and only if there is none is a new `Z_PZModStudio_MergedPatch` made.
pub fn auto_merge_conflicts(paths: &StudioPaths, rel_paths: &[String]) -> Result<AutoMergeResult, String> {
    let active: Vec<String> = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path)
        .map(|d| d.active_mods)
        .unwrap_or_default();
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let package = active_package_dir(&catalog, &active)
        .map(|(id, _)| id)
        .unwrap_or_else(|| "Z_PZModStudio_MergedPatch".to_string());
    let drafts = crate::patch_generator::get_draft_resolutions(&paths.user_zomboid_dir, &package);

    let mut merged_files = Vec::new();
    let (mut merged, mut skipped) = (Vec::new(), Vec::new());
    for c in scan_conflicts_with(paths, &catalog, &active) {
        let requested = rel_paths.contains(&c.relative_path);
        if requested && c.auto_mergeable {
            merged_files.push(crate::patch_generator::MergedFilePayload { relative_path: c.relative_path.clone(), content: c.auto_ast_output.clone() });
            merged.push(c.relative_path);
        } else {
            if requested {
                skipped.push(c.relative_path.clone());
            }
            // Keep what the package already resolves.
            if let Some(d) = drafts.get(&c.relative_path).filter(|d| d.status == "RESOLVED") {
                merged_files.push(crate::patch_generator::MergedFilePayload { relative_path: c.relative_path.clone(), content: d.resolved_content.clone() });
            }
        }
    }
    if merged.is_empty() {
        return Ok(AutoMergeResult { package_folder: package, merged, skipped });
    }
    crate::patch_generator::generate_master_patch(crate::patch_generator::MasterPatchRequest {
        workshop_dir: Some(paths.workshop_dir.clone()),
        pz_install_dir: Some(paths.pz_install_dir.clone()),
        user_zomboid_dir: paths.user_zomboid_dir.clone(),
        mod_list_ini_path: paths.mod_list_ini_path.clone(),
        merged_files,
        package_folder_name: Some(package.clone()),
    })?;
    Ok(AutoMergeResult { package_folder: package, merged, skipped })
}

/// A string value under HKEY_CURRENT_USER or HKEY_LOCAL_MACHINE, or None when it is not there.
#[cfg(windows)]
fn registry_string(under_current_user: bool, subkey: &str, value: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ,
    };
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (key, name) = (wide(subkey), wide(value));
    let hive = if under_current_user { HKEY_CURRENT_USER } else { HKEY_LOCAL_MACHINE };
    let mut bytes: u32 = 0;
    unsafe {
        let probe = RegGetValueW(hive, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), std::ptr::null_mut(), &mut bytes);
        if probe != 0 || bytes == 0 || bytes > 64 * 1024 {
            return None;
        }
        let mut buf = vec![0u16; bytes as usize / 2 + 1];
        let read = RegGetValueW(hive, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut bytes);
        if read != 0 {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

#[cfg(not(windows))]
fn registry_string(_under_current_user: bool, _subkey: &str, _value: &str) -> Option<String> {
    None
}

/// The library paths Steam lists in `steamapps/libraryfolders.vdf` (games can live on any drive).
fn steam_libraries(steam_root: &Path) -> Vec<PathBuf> {
    let Ok(text) = fs::read_to_string(steam_root.join("steamapps").join("libraryfolders.vdf")) else {
        return Vec::new();
    };
    crate::vdf::parse(&text)
        .get("libraryfolders")
        .and_then(|b| b.block())
        .map(|entries| entries.values().filter_map(|e| e.str("path")).map(PathBuf::from).collect())
        .unwrap_or_default()
}

/// Steam writes its path in whatever shape it likes ("d:/apps/steam"): make it "D:\\Apps\\Steam"
/// so the paths the Studio stores and compares all look the same.
fn windows_style(p: PathBuf) -> PathBuf {
    let mut text = p.to_string_lossy().replace('/', "\\");
    if text.len() > 3 {
        text = text.trim_end_matches('\\').to_string();
    }
    if text.as_bytes().get(1) == Some(&b':') {
        text[..1].make_ascii_uppercase();
    }
    PathBuf::from(text)
}

/// Every Steam library on this PC: the install Steam recorded in the registry and the libraries it
/// lists, then common install folders as a fallback when the registry says nothing.
pub fn steam_library_dirs() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    fn push(roots: &mut Vec<PathBuf>, p: PathBuf) {
        let p = windows_style(p);
        let seen = roots.iter().any(|r| r.as_os_str().eq_ignore_ascii_case(p.as_os_str()));
        if !seen && p.is_dir() {
            roots.push(p);
        }
    }
    for installed in [
        registry_string(true, "Software\\Valve\\Steam", "SteamPath"),
        registry_string(false, "SOFTWARE\\WOW6432Node\\Valve\\Steam", "InstallPath"),
        registry_string(false, "SOFTWARE\\Valve\\Steam", "InstallPath"),
    ]
    .into_iter()
    .flatten()
    {
        push(&mut roots, PathBuf::from(installed));
    }
    for library in roots.clone().iter().flat_map(|r| steam_libraries(r)) {
        push(&mut roots, library);
    }
    // Steam not in the registry (portable install, another OS): look where it usually is.
    for drive in ["C", "D", "E", "F", "G", "H", "I", "J", "K", "Z"] {
        for folder in [
            "Program Files (x86)\\Steam",
            "Program Files\\Steam",
            "Steam",
            "SteamLibrary",
            "Games\\Steam",
            "Games",
            "Apps\\Steam",
            "Apps",
            "Juegos\\Steam",
            "Juegos",
        ] {
            push(&mut roots, PathBuf::from(format!("{}:\\{}", drive, folder)));
        }
    }
    roots
}

/// Auto-detects the Project Zomboid install and the user's Zomboid folder.
/// Where the Studio keeps the folders the player chose, for the parts that do not run in the
/// window: the MCP server, the default.txt watcher, the launch file edit.
fn saved_paths_file() -> Option<PathBuf> {
    dirs_next::config_dir().map(|d| d.join("PZModStudio").join("paths.json"))
}

/// Remembers the folders the window uses (called whenever it validates them).
pub fn remember_paths(paths: &StudioPaths) {
    if cfg!(test) || !paths.is_valid {
        return;
    }
    if let Some(file) = saved_paths_file() {
        if let Some(parent) = file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(paths) {
            if std::fs::read_to_string(&file).ok().as_deref() != Some(json.as_str()) {
                let tmp = file.with_extension("json.tmp");
                if std::fs::write(&tmp, json).is_ok() {
                    let _ = std::fs::rename(&tmp, &file);
                }
            }
        }
    }
}

fn saved_paths() -> Option<StudioPaths> {
    if cfg!(test) {
        return None;
    }
    saved_paths_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|c| serde_json::from_str::<StudioPaths>(&c).ok())
        .filter(|p| !p.user_zomboid_dir.is_empty() && Path::new(&p.user_zomboid_dir).is_dir())
}

/// The folders the player chose in the Studio, when they are still there; else what is detected.
pub fn studio_paths() -> StudioPaths {
    saved_paths().unwrap_or_else(auto_detect_paths)
}

/// The game folder programs are started from (the game, its server, the Steam helper): the one the
/// Studio saved or detected, never a folder the page names. A page that could pick it could make
/// the Studio run any program called ProjectZomboid64.exe, or load any steam_api64.dll.
pub fn trusted_install_dir() -> String {
    studio_paths().pz_install_dir
}

/// The Zomboid folder the player chose, or "" for the usual places (cheap: no Steam scan).
pub fn saved_user_zomboid_dir() -> String {
    saved_paths().map(|p| p.user_zomboid_dir).unwrap_or_default()
}

pub fn auto_detect_paths() -> StudioPaths {
    // Zomboid sits in the home folder, or under Documents when that is where Windows (or
    // OneDrive) puts the user's documents. Take the first one that is actually there.
    let user_zomboid = crate::load_order::mod_info::get_all_user_zomboid_dirs("")
        .into_iter()
        .find(|d| d.is_dir())
        .unwrap_or_else(|| {
            dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("C:\\")).join("Zomboid")
        });
    let mod_list_ini = user_zomboid.join("mods").join("default.txt");

    let mut pz_install_dir = String::new();
    let mut workshop_dir = String::new();

    for root in steam_library_dirs() {
        let workshop_p = root.join("steamapps").join("workshop").join("content").join("108600");
        if workshop_dir.is_empty() && workshop_p.is_dir() {
            workshop_dir = workshop_p.to_string_lossy().to_string();
        }
        let pz_p = root.join("steamapps").join("common").join("ProjectZomboid");
        if pz_install_dir.is_empty() && pz_p.is_dir() {
            pz_install_dir = pz_p.to_string_lossy().to_string();
        }
        if !workshop_dir.is_empty() && !pz_install_dir.is_empty() {
            break;
        }
    }

    let user_zomboid_dir = if user_zomboid.exists() {
        user_zomboid.to_string_lossy().to_string()
    } else {
        String::new()
    };

    let mod_list_ini_path = mod_list_ini.to_string_lossy().to_string();
    let is_valid = !pz_install_dir.is_empty() && !user_zomboid_dir.is_empty();

    StudioPaths {
        pz_install_dir,
        workshop_dir,
        user_zomboid_dir,
        mod_list_ini_path,
        carrier_workshop_id: None,
        is_valid,
    }
}

/// Validates provided paths.
pub fn validate_paths(mut paths: StudioPaths) -> StudioPaths {
    let pz_ok = !paths.pz_install_dir.is_empty() && Path::new(&paths.pz_install_dir).exists();
    let user_ok = !paths.user_zomboid_dir.is_empty() && Path::new(&paths.user_zomboid_dir).exists();

    if pz_ok && (paths.workshop_dir.is_empty() || !Path::new(&paths.workshop_dir).exists()) {
        let install_path = Path::new(&paths.pz_install_dir);
        if let Some(parent) = install_path.parent().and_then(|p| p.parent()) {
            let candidate = parent.join("workshop").join("content").join("108600");
            if candidate.exists() {
                paths.workshop_dir = candidate.to_string_lossy().to_string();
            }
        }
    }

    if user_ok && paths.mod_list_ini_path.is_empty() {
        let candidate = Path::new(&paths.user_zomboid_dir).join("mods").join("ModListData.ini");
        paths.mod_list_ini_path = candidate.to_string_lossy().to_string();
    }

    paths.is_valid = pz_ok && user_ok;
    paths
}

/// Files the game reads from every active mod and combines itself (checked in the decompiled
/// 42.20.4 code: CustomPerks, CustomSandboxOptions, TileGeometryManager, TileDepthTexture*,
/// SeamManager, SeatingManager, SpriteModelManager, ModRegistries, Translator). Several mods
/// shipping one of these is normal and never a conflict; merging them would load them twice.
pub fn is_loaded_from_every_mod(rel_path: &str) -> bool {
    let p = rel_path.replace('\\', "/").to_lowercase();
    const EACH_MOD: [&str; 8] = [
        "media/perks.txt",
        "media/sandbox-options.txt",
        "media/tilegeometry.txt",
        "media/tiledepthtextureassignments.txt",
        "media/seams.txt",
        "media/seating.txt",
        "media/spritemodels.txt",
        "media/registries.lua",
    ];
    EACH_MOD.contains(&p.as_str()) || p.starts_with("media/lua/shared/translate/")
}

/// Finds files that several active mods ship at the same path (the game loads only the copy of
/// the mod that loads last), looking only at what the game really reads: active mods (from
/// default.txt, in load order) and, per mod, the folders loaded for the installed build
/// (`common/` + version folder, or the root on Build 41). A mod's version folder overriding its own
/// `common/` file is not a conflict. Contents are read only for paths with two or more mods.
/// `scan_conflicts` showing one package's saved merges instead of the active package's.
pub fn scan_conflicts_in(paths: &StudioPaths, package_folder: Option<&str>) -> Vec<VfsConflictRaw> {
    let active: Vec<String> = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path)
        .map(|d| d.active_mods)
        .unwrap_or_default();
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    scan_conflicts_for_package(paths, &catalog, &active, package_folder)
}

pub fn scan_conflicts(paths: &StudioPaths) -> Vec<VfsConflictRaw> {
    let active: Vec<String> = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path)
        .map(|d| d.active_mods)
        .unwrap_or_default();
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    scan_conflicts_with(paths, &catalog, &active)
}

/// A Workshop mod's `.lua` / `.txt` files, by its folder, the version Steam installed (publish
/// time and size, from appworkshop_108600.acf: they change with every update) and the build.
type ModFilesKey = (String, u64, Option<u64>, Option<u32>);
static WORKSHOP_MOD_FILES: std::sync::Mutex<Option<HashMap<ModFilesKey, std::collections::BTreeMap<String, PathBuf>>>> =
    std::sync::Mutex::new(None);

/// The `.lua` and `.txt` files of a mod the game reads on this build, by `media/...` path. Listing
/// them means walking every file the mod has, and mods carry tens of thousands of textures and
/// models (one Viewpoint pack alone: 54,000 files in 18,000 folders, no Lua): a second on every
/// health check. A Workshop mod only changes when Steam updates it, so its list is kept until
/// then; a local mod (Zomboid/mods, maybe being written right now) is walked every time.
fn mod_text_files(m: &crate::catalog::CatalogMod, game: Option<u32>) -> std::collections::BTreeMap<String, PathBuf> {
    let key = m.workshop_state.as_ref().and_then(|s| s.installed_time.map(|t| (m.root_dir.clone(), t, s.size, game)));
    if let Some(key) = &key {
        let cache = WORKSHOP_MOD_FILES.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(files) = cache.as_ref().and_then(|c| c.get(key)) {
            return files.clone();
        }
    }
    let mut files: std::collections::BTreeMap<String, PathBuf> = std::collections::BTreeMap::new();
    for layer in crate::load_order::mod_info::media_layers_for_build(Path::new(&m.root_dir), game) {
        let media = layer.join("media");
        for e in WalkDir::new(&media).into_iter().filter_map(|e| e.ok()) {
            if !e.file_type().is_file() {
                continue;
            }
            let is_text = e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("lua") || x.eq_ignore_ascii_case("txt"));
            let Ok(rel) = e.path().strip_prefix(&media) else { continue };
            if is_text {
                // A later layer (the version folder) replaces the same file from common/.
                files.insert(format!("media/{}", rel.to_string_lossy().replace('\\', "/")), e.path().to_path_buf());
            }
        }
    }
    if let Some(key) = key {
        let mut cache = WORKSHOP_MOD_FILES.lock().unwrap_or_else(|p| p.into_inner());
        cache.get_or_insert_with(HashMap::new).insert(key, files.clone());
    }
    files
}

/// `scan_conflicts` for a given catalog and active list (mod ids in load order).
pub fn scan_conflicts_with(paths: &StudioPaths, catalog: &crate::catalog::ModCatalog, active: &[String]) -> Vec<VfsConflictRaw> {
    scan_conflicts_for_package(paths, catalog, active, None)
}

/// `package_folder` picks whose saved merges are shown. Without it, the active package's are
/// used: drafts belong to one package, and showing another's would publish the wrong code.
pub fn scan_conflicts_for_package(
    paths: &StudioPaths,
    catalog: &crate::catalog::ModCatalog,
    active: &[String],
    package_folder: Option<&str>,
) -> Vec<VfsConflictRaw> {
    // Found in the list as the game reads it: the Studio packages are taken out just below, and
    // looked for afterwards the active package was never there, so with no package named every
    // conflict read as pending even with all of them merged and published.
    let list_package = active_package_dir(catalog, active).map(|(id, _)| id);
    let active: Vec<String> = active
        .iter()
        .filter(|s| !s.starts_with("Z_PZModStudio_") && s.as_str() != "PZModStudioCarrier")
        .cloned()
        .collect();
    let game = catalog.game_version_int();

    // "media/..." path -> (load position, mod id, mod name, file on disk), in load order.
    let mut by_path: HashMap<String, Vec<(usize, String, String, PathBuf)>> = HashMap::new();
    let mut seen_mods = HashSet::new();
    for (position, entry) in active.iter().enumerate() {
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if m.loadability != crate::catalog::Loadability::Loadable || !seen_mods.insert(m.id.clone()) {
            continue;
        }
        for (rel, abs) in mod_text_files(m, game) {
            by_path.entry(rel).or_default().push((position, m.id.clone(), m.name.clone(), abs));
        }
    }

    // path -> the one mod that ships it, for the patched files below.
    let mut single_owners: HashMap<String, String> = HashMap::new();
    for (rel, owners) in &by_path {
        if owners.len() == 1 {
            single_owners.insert(rel.clone(), owners[0].1.clone());
        }
    }

    let mut file_map: Vec<(String, Vec<CompetingModFileRaw>)> = Vec::new();
    for (rel, mut owners) in by_path {
        if owners.len() < 2 || is_loaded_from_every_mod(&rel) {
            continue;
        }
        owners.sort_by_key(|o| o.0);
        let files: Vec<CompetingModFileRaw> = owners
            .into_iter()
            .filter_map(|(_, mod_id, mod_name, abs)| {
                let bytes = fs::read(&abs).ok()?;
                Some(CompetingModFileRaw {
                    mod_id,
                    mod_name,
                    absolute_path: abs.to_string_lossy().to_string(),
                    content: String::from_utf8_lossy(&bytes).to_string(),
                })
            })
            .collect();
        file_map.push((rel, files));
    }
    file_map.sort_by(|a, b| a.0.cmp(&b.0));

    let pz_base_dir = Path::new(&paths.pz_install_dir);

    // Load any saved draft resolutions for active packages
    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(&paths.user_zomboid_dir);
    // Saved merges belong to one package. Reading every package's at once used to show, and
    // then publish, another package's code.
    let wanted = package_folder.map(str::to_string).or(list_package);
    let mut draft_map: HashMap<String, (String, String)> = HashMap::new();
    if let Some(wanted) = wanted {
        for u_dir in &user_dirs {
            let draft_file = u_dir.join("mods").join(&wanted).join("draft_resolutions.json");
            let Ok(content) = fs::read_to_string(&draft_file) else { continue };
            let Ok(items) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&content) else { continue };
            for (k, v) in items {
                if let (Some(c), Some(st)) = (v["resolved_content"].as_str(), v["status"].as_str()) {
                    draft_map.insert(k, (c.to_string(), st.to_string()));
                }
            }
        }
    }

    let mut conflicts = Vec::new();
    let mut id_counter = 1;

    for (rel_path, unique_mods) in file_map {
        // In Build 42, media/registries.lua is an additive registration file loaded natively by the game engine from all active mods.
        // It does not clobber other mods and re-registering causes duplicate null returns in Java.
        if rel_path.ends_with("registries.lua") || rel_path.ends_with("registries.LUA") {
            continue;
        }

        {
            // Files are in load order: the last one is the copy the game uses.
            if unique_mods.len() > 1 {
                let vanilla_file_path = if rel_path.starts_with("media/") || rel_path.starts_with("media\\") {
                    pz_base_dir.join(&rel_path)
                } else {
                    pz_base_dir.join("media").join(&rel_path)
                };
                let base_content = if vanilla_file_path.exists() {
                    fs::read_to_string(&vanilla_file_path).unwrap_or_else(|_| "-- vanilla file unreadable".to_string())
                } else {
                    "-- vanilla file not present for this path".to_string()
                };

                let file_type = if rel_path.ends_with(".lua") { "LUA" } else { "SCRIPT_TXT" };
                let total_file_lines = base_content.lines().count().max(1);
                let conflict_line = find_first_differing_line(&base_content, &unique_mods);

                let is_identical_noise = if unique_mods.len() > 1 {
                    let first = unique_mods[0].content.trim();
                    unique_mods[1..].iter().all(|f| f.content.trim() == first)
                } else {
                    true
                };

                let auto_ast_output = crate::diff_engine::lua::combine_n_way_lua_or_script(&rel_path, &base_content, &unique_mods);
                let has_base = !base_content.starts_with("-- vanilla file not present") && !base_content.starts_with("-- vanilla file unreadable");
                let (mod_changes, collisions) = if has_base {
                    crate::diff_engine::lua::summarize_changes(&base_content, &unique_mods)
                } else {
                    (Vec::new(), Vec::new())
                };
                let strategy = crate::diff_engine::lua::describe_merge_strategy(&rel_path, &base_content, &unique_mods, collisions.len());
                let auto_mergeable = crate::diff_engine::lua::can_auto_merge(&rel_path, &base_content, &unique_mods, collisions.len());
                let (merged_output, status) = if let Some((saved_content, saved_status)) = draft_map.get(&rel_path) {
                    (saved_content.clone(), Some(saved_status.clone()))
                } else {
                    (auto_ast_output.clone(), None)
                };

                conflicts.push(VfsConflictRaw {
                    id: format!("conflict_{}", id_counter),
                    relative_path: rel_path,
                    file_type: file_type.to_string(),
                    start_line: 1,
                    end_line: total_file_lines,
                    conflict_line,
                    total_file_lines,
                    base_content,
                    competing_files: unique_mods,
                    merged_output,
                    auto_ast_output,
                    is_identical_noise,
                    status,
                    mod_changes,
                    collisions,
                    strategy,
                    auto_mergeable,
                });

                id_counter += 1;
            }
        }
    }

    // Two mods changing the same item in differently named files never collide on disk, so the
    // walk above cannot see them. They are the same question, so they are listed here too.
    let definitions = crate::scripts::script_definition_conflicts(catalog, &active);
    conflicts.extend(crate::scripts::as_merger_conflicts(&definitions, &draft_map));
    // Likewise a thing two mods name differently in the same language: the game reads every
    // mod's translations, so the files never collide, but the words do.
    // Only the language the game shows: forty rows of Chinese and Czech the player never sees
    // would bury the rest.
    let language = crate::translations::game_language(&paths.user_zomboid_dir);
    let translations: Vec<_> = crate::translations::translation_conflicts(catalog, &active).into_iter().filter(|c| c.lang == language).collect();
    conflicts.extend(crate::translations::as_merger_conflicts(&translations, &draft_map));

    // A file only one mod ships is no conflict, so the walk drops it -- including one the user
    // opened to patch a bug of that mod's own. Once the package holds a decision for it, it
    // belongs in the list like anything else, or it would vanish on the next rescan and the
    // package would keep a file nothing in the app could show.
    for (rel_path, (saved_content, saved_status)) in &draft_map {
        if conflicts.iter().any(|c| &c.relative_path == rel_path) {
            continue;
        }
        let Some(owner) = single_owners.get(rel_path) else { continue };
        let Ok(mut entry) = crate::vfs::mod_patch::conflict_for_mod_file(catalog, paths, owner, rel_path)
        else {
            continue;
        };
        entry.merged_output = saved_content.clone();
        entry.status = Some(saved_status.clone());
        conflicts.push(entry);
    }

    conflicts
}

/// Finds the EXACT 1-based line index where content actually differs between Vanilla and competing mods.
fn find_first_differing_line(base: &str, files: &[CompetingModFileRaw]) -> usize {
    let base_lines: Vec<&str> = base.lines().collect();
    let file_lines_list: Vec<Vec<&str>> = files.iter().map(|f| f.content.lines().collect()).collect();

    let max_lines = base_lines.len().max(
        file_lines_list.iter().map(|l| l.len()).max().unwrap_or(0)
    );

    for idx in 0..max_lines {
        let base_line = base_lines.get(idx).copied().unwrap_or("");
        
        for file_lines in &file_lines_list {
            let mod_line = file_lines.get(idx).copied().unwrap_or("");
            if base_line != mod_line {
                return idx + 1; // Return exact 1-based line number of difference
            }
        }
    }

    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_paths_are_normalised() {
        let norm = |s: &str| windows_style(PathBuf::from(s)).to_string_lossy().to_string();
        assert_eq!(norm("d:/apps/steam"), "D:\\apps\\steam");
        assert_eq!(norm("D:\\Apps\\Steam\\"), "D:\\Apps\\Steam");
        assert_eq!(norm("C:\\"), "C:\\", "a drive root keeps its slash");
    }

    /// What this PC really has. cargo test steam_detection -- --ignored --nocapture
    #[test]
    #[ignore]
    fn steam_detection_on_this_pc() {
        println!("registry SteamPath: {:?}", registry_string(true, "Software\\Valve\\Steam", "SteamPath"));
        let libs = steam_library_dirs();
        for l in &libs {
            println!("library: {}", l.display());
        }
        assert!(libs.iter().all(|l| !l.to_string_lossy().contains('/')), "paths are normalised");
        let paths = auto_detect_paths();
        println!("game:     {}", paths.pz_install_dir);
        println!("workshop: {}", paths.workshop_dir);
        println!("zomboid:  {}", paths.user_zomboid_dir);
        assert!(paths.is_valid, "the game should be found on this PC");
    }

    /// Read-only timing on the real install.
    /// PZMS_PZ_DIR=... PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=... cargo test --release scan_timing -- --ignored --nocapture
    #[test]
    #[ignore]
    fn scan_timing_on_real_install() {
        let (Ok(pz), Ok(w), Ok(z)) = (std::env::var("PZMS_PZ_DIR"), std::env::var("PZMS_WORKSHOP_DIR"), std::env::var("PZMS_ZOMBOID_DIR")) else { return };
        let paths = StudioPaths {
            pz_install_dir: pz,
            workshop_dir: w,
            mod_list_ini_path: format!("{}/mods/default.txt", z),
            user_zomboid_dir: z,
            carrier_workshop_id: None,
            is_valid: true,
        };
        let t = std::time::Instant::now();
        let conflicts = scan_conflicts(&paths);
        println!("scan: {:?}, {} conflicts", t.elapsed(), conflicts.len());
        for c in &conflicts {
            println!("  {} ({} lines base) <- {:?}", c.relative_path, c.total_file_lines, c.competing_files.iter().map(|f| (&f.mod_name, f.content.len())).collect::<Vec<_>>());
        }
    }
}
