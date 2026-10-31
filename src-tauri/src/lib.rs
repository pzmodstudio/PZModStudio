pub mod api_lint;
pub mod app_docs;
pub mod app_update;
pub mod bisect;
pub mod catalog;
pub mod claude_agent;
pub mod compat;
pub mod devtools;
pub mod diagnostics;
pub mod diff_engine;
pub mod discord;
pub mod discord_commands;
pub mod disk_usage;
pub mod error_report;
pub mod game_memory;
pub mod images;
pub mod instance_manager;
pub mod lists;
pub mod load_order;
pub mod map_cells;
pub mod mod_changelog;
pub mod mod_meta;
pub mod mcp;
pub mod mod_install;
pub mod mplint;
pub mod nexus;
pub mod origins;
pub mod patch_generator;
pub mod preset_manager;
pub mod rules;
pub mod sandbox;
pub mod save_backup;
pub mod security;
pub mod scripts;
pub mod self_update;
pub mod secrets;
pub mod translate;
pub mod translation_drafts;
pub mod steam_client;
pub mod steam_downloads;
pub mod studio_link;
pub mod server_browser;
pub mod server_manager;
pub mod snapshots;
pub mod translations;
pub mod vdf;
pub mod vfs;
pub mod whats_new;
pub mod window_prefs;
pub mod workshop;
pub mod workshop_publish;

use diff_engine::lua::{three_way_merge_lua, validate_lua_syntax, LuaSyntaxCheckResult, MergeChunkResult};

use instance_manager::{activate_instance, create_instance, delete_instance, list_instances, save_master_load_order, update_instance};
use load_order::ini_parser::{read_mod_list_ini, write_mod_list_ini, ModListData};
use load_order::mod_info::{scan_all_installed_mods, ModManifest};
use patch_generator::{generate_master_patch, MasterPatchRequest, MasterPatchResult};
use preset_manager::{check_missing_preset_mods, export_preset_file, import_preset_file};
use sandbox::{launch_sandbox_and_watch, SandboxLaunchConfig};
use server_manager::check_server_mods;
use server_manager::{
    create_new_server_config, delete_server_config, get_connected_players,
    get_dedicated_server_logs, get_dedicated_server_status, get_server_quick_settings,
    launch_dedicated_server, list_server_configs, save_server_log_snapshot,
    save_server_quick_settings, send_server_command, stop_dedicated_server,
    sync_client_to_server,
};
use vfs::{auto_detect_paths, validate_paths, StudioPaths, VfsConflictRaw};

#[tauri::command(async)]
fn get_auto_paths() -> StudioPaths {
    auto_detect_paths()
}

#[tauri::command(async)]
fn set_and_validate_paths(custom_paths: StudioPaths) -> StudioPaths {
    let paths = validate_paths(custom_paths);
    vfs::remember_paths(&paths);
    paths
}

#[tauri::command(async)]
fn scan_conflicts_cmd(paths: StudioPaths, package_folder: Option<String>) -> Vec<VfsConflictRaw> {
    vfs::scan_conflicts_in(&paths, package_folder.as_deref())
}

#[tauri::command(async)]
fn validate_lua_syntax_cmd(code: String) -> LuaSyntaxCheckResult {
    validate_lua_syntax(&code)
}

#[tauri::command(async)]
fn three_way_merge_lua_cmd(base: String, target_a: String, target_b: String) -> MergeChunkResult {
    three_way_merge_lua(&base, &target_a, &target_b)
}

#[tauri::command(async)]
fn read_mod_list_ini_cmd(ini_path: String) -> Result<ModListData, String> {
    read_mod_list_ini(&ini_path)
}

#[tauri::command]
fn write_mod_list_ini_cmd(ini_path: String, active_mods: Vec<String>) -> Result<(), String> {
    write_mod_list_ini(&ini_path, &active_mods)
}

#[tauri::command(async)]
fn scan_all_installed_mods_cmd(paths: StudioPaths) -> Vec<ModManifest> {
    scan_all_installed_mods(&paths)
}

/// Read-only health check of `active` (mod ids in load order) against what is installed,
/// following the game's own rules. Returns the catalog too, so the UI can show details.
#[tauri::command(async)]
fn get_mod_health_cmd(paths: StudioPaths, active: Vec<String>) -> catalog::ModHealth {
    catalog::mod_health_full(&paths, &active)
}

/// Merges the given shared files automatically (only those needing no decision) and publishes
/// the Studio package.
#[tauri::command(async)]
fn auto_merge_conflicts_cmd(paths: StudioPaths, relative_paths: Vec<String>) -> Result<vfs::AutoMergeResult, String> {
    vfs::auto_merge_conflicts(&paths, &relative_paths)
}

/// One page of Project Zomboid mods from Nexus Mods (public API, no key).
#[tauri::command(async)]
fn nexus_search_cmd(
    query: Option<String>,
    category: Option<String>,
    sort: nexus::NexusSort,
    offset: u64,
    count: u64,
) -> Result<nexus::NexusSearchResult, String> {
    nexus::search_mods(query.as_deref(), category.as_deref(), sort, offset, count)
}

/// Project Zomboid categories on Nexus with mod counts.
#[tauri::command(async)]
fn nexus_categories_cmd() -> Result<Vec<nexus::NexusCategory>, String> {
    nexus::categories()
}

/// Description, requirements and files of one Nexus mod.
#[tauri::command(async)]
fn nexus_mod_details_cmd(mod_id: u64) -> Result<nexus::NexusModDetails, String> {
    nexus::mod_details(mod_id)
}

/// Full description, screenshots and required items of one Workshop item (no key).
#[tauri::command(async)]
fn workshop_item_details_cmd(id: String) -> Result<workshop::WorkshopItemDetails, String> {
    workshop::item_details(&id)
}

/// What subscribing to these items needs too: their "Required items", all the way down.
#[tauri::command(async)]
fn workshop_requirements_cmd(ids: Vec<String>) -> Result<workshop::RequiredChain, String> {
    workshop::required_chain(&ids)
}

/// Steam's list of Project Zomboid servers, sent as `public-servers` events while servers answer:
/// through the running Steam client (no key), or the Web API when a key is saved and the client
/// cannot be asked. Returns how many were found.
#[tauri::command(async)]
fn public_servers_cmd<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) -> Result<usize, String> {
    use std::io::BufRead;
    use tauri::Emitter;
    stop_server_search_cmd();
    let exe = std::env::current_exe().map_err(|e| format!("Could not find the Studio's own program: {}", e))?;
    let mut command = std::process::Command::new(exe);
    command.arg("--steam-servers").arg(vfs::trusted_install_dir());
    command.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|e| format!("The Steam helper could not be started: {}", e))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    if let Ok(mut slot) = SERVER_SEARCH.lock() {
        *slot = Some(child);
    }
    let mut found = 0usize;
    if let Some(out) = stdout {
        for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(json) = line.trim().strip_prefix(STEAM_PART_PREFIX) {
                if let Ok(batch) = serde_json::from_str::<Vec<steam_client::ListedServer>>(json) {
                    let list = server_browser::from_client_list(batch, "");
                    found += list.len();
                    let _ = app_handle.emit("public-servers", &list);
                }
            }
        }
    }
    let status = SERVER_SEARCH.lock().ok().and_then(|mut s| s.take()).and_then(|mut c| c.wait().ok());
    if found > 0 {
        return Ok(found);
    }
    let reason = stderr
        .and_then(|mut e| {
            let mut text = String::new();
            std::io::Read::read_to_string(&mut e, &mut text).ok()?;
            steam_answer(&text)
        })
        .unwrap_or_else(|| if status.map(|s| s.success()).unwrap_or(false) { "Steam listed no servers.".to_string() } else { "The search was stopped.".to_string() });
    // The Steam client could not be asked: the Web API, when there is a key.
    match secrets::get(secrets::STEAM_WEB_API_KEY)? {
        Some(key) => {
            let list = server_browser::public_servers(&key, "", 10000)?;
            let _ = app_handle.emit("public-servers", &list);
            Ok(list.len())
        }
        None => Err(reason),
    }
}

/// Ends the server search running now, keeping what was found.
#[tauri::command]
fn stop_server_search_cmd() {
    if let Ok(mut slot) = SERVER_SEARCH.lock() {
        if let Some(mut child) = slot.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// What one server says about itself: description, version, mods, how fast it answers.
#[tauri::command(async)]
fn server_rules_cmd(address: String) -> Result<server_browser::ServerRules, String> {
    server_browser::server_rules(&address)
}

#[tauri::command(async)]
fn saved_servers_cmd<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, paths: StudioPaths) -> Result<Vec<server_browser::SavedServer>, String> {
    let (list, changed) = server_browser::refresh_joins(&zomboid_dir_of(&paths)?);
    for profile in changed {
        let _ = tauri::Emitter::emit(&app_handle, studio_link::PROFILE_CHANGED_EVENT, profile);
    }
    Ok(list)
}

/// Keeps a server in the player's list; `password` Some("") forgets it, None leaves it.
#[tauri::command(async)]
fn save_server_cmd(paths: StudioPaths, server: server_browser::SavedServer, password: Option<String>) -> Result<Vec<server_browser::SavedServer>, String> {
    server_browser::save_server(&zomboid_dir_of(&paths)?, server, password.as_deref())
}

#[tauri::command(async)]
fn forget_server_cmd(paths: StudioPaths, address: String) -> Result<Vec<server_browser::SavedServer>, String> {
    server_browser::forget_server(&zomboid_dir_of(&paths)?, &address)
}

/// Keeps the profile just made from a server with it, for its pane to say so.
#[tauri::command(async)]
fn set_server_made_profile_cmd(paths: StudioPaths, address: String, name: String, profile_id: String) -> Result<Vec<server_browser::SavedServer>, String> {
    server_browser::set_made_profile(&zomboid_dir_of(&paths)?, &address, &name, &profile_id)
}

/// The save the next launch loads ("Custom" launch): the Link loads it once the main menu is
/// up. None clears it. Only a save that is on disk, and never a server's.
#[tauri::command(async)]
fn set_launch_save_cmd(paths: StudioPaths, save: Option<String>) -> Result<(), String> {
    studio_link::write_load_file(&zomboid_dir_of(&paths)?, save.as_deref())
}

/// Starts the game joining `address` (the kept password unless one is given). The game must be
/// closed: a running one cannot be told to join.
#[tauri::command(async)]
fn join_server_cmd<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, paths: StudioPaths, address: String, name: String, password: Option<String>, username: Option<String>, test_mode: Option<String>) -> Result<u32, String> {
    if sandbox::find_running_pz_pid().is_some() {
        return Err("The game is already running. Close it first, then join from here (or use its Join screen).".to_string());
    }
    let extra_args = server_browser::join_args(&address, password.as_deref())?;
    if let Ok(z) = zomboid_dir_of(&paths) {
        let username = username.unwrap_or_default();
        // The profile played on the server: what it loads goes into this one.
        let profile = instance_manager::active_profile(&paths.user_zomboid_dir).map(|(id, _)| id);
        let _ = server_browser::note_join(&z, &address, &name, &username, profile.as_deref());
        // The account the game's connect window asks for: the one typed now, else the one kept.
        let kept = server_browser::read_saved(&z).into_iter().find(|s| s.address.eq_ignore_ascii_case(address.trim())).map(|s| s.username).unwrap_or_default();
        let _ = server_browser::write_join_file(&z, &address, &name, if username.trim().is_empty() { &kept } else { &username });
    }
    let config = SandboxLaunchConfig {
        pz_install_dir: vfs::trusted_install_dir(),
        user_zomboid_dir: paths.user_zomboid_dir.clone(),
        // The launcher's mode when started from it ("Custom"), else the plain game.
        test_mode: test_mode.filter(|m| !m.trim().is_empty()).unwrap_or_else(|| "NORMAL".to_string()),
        extra_args,
    };
    let watcher = app_handle.clone();
    let pid = launch_sandbox_and_watch(app_handle, config)?;
    // While the game is on the server (and once when it closes): what the server loaded, read
    // from the log as soon as it is there, goes into the server's profile, and the app is told.
    if let Ok(z) = zomboid_dir_of(&paths) {
        std::thread::spawn(move || {
            let mut seen_running = false;
            for round in 0..(12 * 60 * 2) {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let running = sandbox::find_running_pz_pid().is_some();
                for profile in server_browser::refresh_joins(&z).1 {
                    let _ = tauri::Emitter::emit(&watcher, studio_link::PROFILE_CHANGED_EVENT, profile);
                }
                if running {
                    seen_running = true;
                } else if seen_running || round > 10 {
                    break;
                }
            }
        });
    }
    Ok(pid)
}

/// Whether a Steam Web API key is saved (the key itself never goes back to the UI).
#[tauri::command(async)]
fn steam_key_status_cmd() -> Result<bool, String> {
    Ok(secrets::get(secrets::STEAM_WEB_API_KEY)?.is_some())
}

/// Checks the key with a one-item Workshop search, then saves it in Windows Credential Manager.
#[tauri::command(async)]
fn set_steam_key_cmd(key: String) -> Result<(), String> {
    let key = key.trim();
    if key.len() != 32 || !key.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("A Steam Web API key is 32 characters, 0-9 and A-F.".to_string());
    }
    workshop::search(key, None, workshop::WorkshopSort::Trending, None, None, 1, 1)?;
    secrets::set(secrets::STEAM_WEB_API_KEY, key)
}

#[tauri::command(async)]
fn delete_steam_key_cmd() -> Result<(), String> {
    secrets::delete(secrets::STEAM_WEB_API_KEY)
}

/// Mod texts (Steam BBCode) in the app's language; translated once, then from disk.
#[tauri::command(async)]
fn translate_texts_cmd(texts: Vec<String>, target: String) -> Result<translate::Translation, String> {
    translate::translate_texts(&vfs::studio_paths().user_zomboid_dir, &texts, &target)
}

/// Whether a DeepL key is saved (the key itself never goes back to the UI).
#[tauri::command(async)]
fn deepl_key_status_cmd() -> Result<bool, String> {
    Ok(secrets::get(translate::DEEPL_KEY)?.is_some())
}

/// Checks the key with DeepL, then saves it in Windows Credential Manager.
#[tauri::command(async)]
fn set_deepl_key_cmd(key: String) -> Result<(), String> {
    translate::save_deepl_key(&key)
}

#[tauri::command(async)]
fn delete_deepl_key_cmd() -> Result<(), String> {
    secrets::delete(translate::DEEPL_KEY)
}

#[tauri::command(async)]
fn translation_cache_info_cmd() -> translate::CacheInfo {
    translate::cache_info(&vfs::studio_paths().user_zomboid_dir)
}

#[tauri::command(async)]
fn clear_translation_cache_cmd() -> Result<usize, String> {
    translate::clear_cache(&vfs::studio_paths().user_zomboid_dir)
}

/// Searches the Project Zomboid Workshop with the saved key.
#[tauri::command(async)]
fn workshop_search_cmd(query: workshop::BrowseQuery) -> Result<workshop::WorkshopSearchResult, String> {
    // The public browse page needs no key; the Web API is the fallback when Steam changes it.
    let public = workshop::search_public(&query);
    let Err(err) = public else { return public };
    match secrets::get(secrets::STEAM_WEB_API_KEY)? {
        // The Web API cannot list collections, so the key only helps with mods.
        Some(key) if !query.collections => workshop::search(
            &key,
            query.text.as_deref(),
            query.sort.unwrap_or(workshop::WorkshopSort::Trending),
            query.build.as_deref(),
            query.category.as_deref(),
            query.page,
            30,
        ),
        _ => Err(err),
    }
}

/// Subscribes or unsubscribes through the Steam client that is already signed in. No key.
/// Steam downloads or removes the files itself afterwards.
///
/// The work happens in a short-lived copy of this program, because Steam's API may only be
/// started once in a process and the window outlives many subscriptions.
#[tauri::command(async)]
fn workshop_subscribe_cmd(
    paths: StudioPaths,
    ids: Vec<String>,
    subscribe: bool,
) -> Result<steam_client::SubscribeOutcome, String> {
    let flag = if subscribe { "--steam-subscribe" } else { "--steam-unsubscribe" };
    let _ = paths;
    let output = steam_helper(flag, &vfs::trusted_install_dir(), &ids)?;
    serde_json::from_str(&output).map_err(|e| format!("Unexpected answer from the Steam helper: {}", e))
}

/// Marks the Steam helper's own line, among whatever Steam's library prints.
pub const STEAM_ANSWER_PREFIX: &str = "PZMS-STEAM:";
/// Marks one part of a longer answer (a batch of servers), printed as it comes.
pub const STEAM_PART_PREFIX: &str = "PZMS-STEAM-PART:";

/// The server search running now, so Stop can end it.
static SERVER_SEARCH: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

/// The helper's answer out of its output, ignoring Steam's chatter.
fn steam_answer(output: &str) -> Option<String> {
    output
        .lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix(STEAM_ANSWER_PREFIX).map(|a| a.trim().to_string()))
}

/// Runs this program again with a `--steam-*` flag and returns what it printed.
fn steam_helper(flag: &str, pz_install_dir: &str, ids: &[String]) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Could not find the Studio's own program: {}", e))?;
    let mut command = std::process::Command::new(exe);
    command.arg(flag).arg(pz_install_dir).args(ids);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flashes up
    }
    let done = command.output().map_err(|e| format!("The Steam helper could not be started: {}", e))?;
    let reason = || {
        steam_answer(&String::from_utf8_lossy(&done.stderr)).unwrap_or_else(|| "Steam refused the request.".to_string())
    };
    if !done.status.success() {
        return Err(reason());
    }
    steam_answer(&String::from_utf8_lossy(&done.stdout)).ok_or_else(reason)
}

/// Whether subscribing from here is possible right now (the game's Steam library and a running
/// Steam client). Does not start Steam's API, so it costs nothing and shows no in-game status.
#[tauri::command(async)]
fn steam_client_ready_cmd(paths: StudioPaths) -> Result<bool, String> {
    let _ = paths;
    Ok(steam_client::is_ready(&vfs::trusted_install_dir()))
}

/// What Steam is downloading for the game right now (Workshop items, phase, bytes).
#[tauri::command(async)]
fn steam_downloads_cmd(paths: StudioPaths) -> steam_downloads::SteamDownloads {
    steam_downloads::status(&paths.workshop_dir)
}

/// A mod's own poster/icon as a data URL, so the webview can show a file on disk.
#[tauri::command(async)]
fn read_mod_image_cmd(paths: StudioPaths, path: String) -> Result<String, String> {
    images::read_as_data_url(&paths, &path)
}

/// Steam Workshop details for item ids (public Web API, no key).
#[tauri::command(async)]
fn workshop_details_cmd(ids: Vec<String>) -> Result<Vec<workshop::WorkshopItem>, String> {
    workshop::get_details(&ids)
}

/// Looks up pasted Workshop links / ids / a collection link.
#[tauri::command(async)]
fn workshop_lookup_cmd(input: String) -> Result<workshop::WorkshopLookup, String> {
    workshop::lookup(&input)
}

/// Installs the mod(s) in a downloaded .zip or an extracted folder into Zomboid/mods.
#[tauri::command(async)]
fn install_mods_cmd(
    source: String,
    paths: StudioPaths,
    replace_existing: bool,
    origin: Option<origins::ModOrigin>,
) -> Result<mod_install::InstallResult, String> {
    mod_install::install_mods(&source, &paths.user_zomboid_dir, &paths.workshop_dir, replace_existing, origin)
}

/// Copies a mod the installed build will not load into Zomboid/mods and adapts it.
#[tauri::command(async)]
fn make_compatible_copy_cmd(paths: StudioPaths, mod_id: String, replace_existing: bool) -> Result<compat::CompatResult, String> {
    let game = load_order::mod_info::read_installed_game_version(&paths.user_zomboid_dir)
        .ok_or("The installed game version is unknown: start Project Zomboid once so Zomboid/version.txt exists.")?;
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, Some(game));
    let original = catalog
        .mods
        .iter()
        .find(|m| m.id == mod_id && m.source != catalog::ModSource::Local)
        .ok_or_else(|| format!("{} is not installed from the Workshop.", mod_id))?;
    let workshop_id = match &original.source {
        catalog::ModSource::Workshop { workshop_id } => Some(workshop_id.clone()),
        _ => None,
    };
    // The module index of the game and the active mods, so the copy's moved require paths are
    // rewritten up front instead of failing at boot.
    let active: Vec<String> = load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path)
        .map(|d| d.active_mods)
        .unwrap_or_default();
    let index = sandbox::requires::index_modules(&paths, &catalog, &active);
    compat::make_compatible_copy_with_index(std::path::Path::new(&original.root_dir), &paths.user_zomboid_dir, game, workshop_id, replace_existing, &index)
}

#[tauri::command(async)]
fn remove_compatible_copy_cmd(paths: StudioPaths, folder: String) -> Result<(), String> {
    compat::remove_compatible_copy(&paths.user_zomboid_dir, &folder)
}

/// Keeps a copy of the mod as it is installed today, for when an update breaks a save.
#[tauri::command(async)]
fn keep_mod_version_cmd(paths: StudioPaths, mod_id: String) -> Result<snapshots::KeptVersion, String> {
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let m = catalog.resolve(&mod_id).ok_or_else(|| format!("{} is not installed.", mod_id))?;
    let workshop_id = match &m.source {
        catalog::ModSource::Workshop { workshop_id } => Some(workshop_id.clone()),
        _ => None,
    };
    snapshots::keep_version(&paths.user_zomboid_dir, &m.id, &m.name, std::path::Path::new(&m.root_dir), m.info.mod_version.clone(), workshop_id)
}

/// Mod Health's warnings for the game's Mods screen (the Link mod draws them on each mod):
/// `(severity, mod id, text)`, the text already in the player's language.
#[tauri::command(async)]
fn write_link_health_cmd(paths: StudioPaths, lines: Vec<(String, String, String)>) -> Result<(), String> {
    let Some(z_dir) = load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next() else {
        return Ok(());
    };
    studio_link::write_health_file(&z_dir, &lines)
}

/// Each mod's origin, library flag and tags for the game's Mods screen filters (see
/// `studio_link::MODS_FILE`), already in the player's language.
#[tauri::command(async)]
fn write_link_mods_cmd(paths: StudioPaths, text: String) -> Result<(), String> {
    let Some(z_dir) = load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next() else {
        return Ok(());
    };
    studio_link::write_mods_file(&z_dir, &text)
}

fn zomboid_dir_of(paths: &StudioPaths) -> Result<std::path::PathBuf, String> {
    load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| "The Zomboid folder was not found.".to_string())
}

/// What switching this list on would change in the game's launch file (Java agents such as
/// ZombieBuddy's), without changing it; and whether the game is running, so it would wait.
#[tauri::command(async)]
fn java_agents_plan_cmd(paths: StudioPaths, active_mod_ids: Vec<String>) -> Result<(studio_link::AgentChanges, bool), String> {
    let z = zomboid_dir_of(&paths)?;
    // The game folder the Studio saved or detected, never one the page names.
    let install = vfs::studio_paths().pz_install_dir;
    if install.is_empty() {
        return Ok((studio_link::AgentChanges::default(), false));
    }
    let plan = studio_link::plan_java_agents(std::path::Path::new(&install), &z, &active_mod_ids)?;
    Ok((plan, sandbox::find_running_pz_pid().is_some()))
}

/// What's new since the game was last played: Workshop mods updated since, or waiting in Steam.
/// `with_notes` reads their change notes too (cached); without, nothing leaves the PC.
#[tauri::command(async)]
fn whats_new_cmd(paths: StudioPaths, with_notes: bool, since: Option<u64>) -> Result<whats_new::WhatsNew, String> {
    let workshop = vfs::studio_paths().workshop_dir;
    Ok(whats_new::whats_new(&zomboid_dir_of(&paths)?, &workshop, with_notes, since))
}

/// The game's world and every installed map mod's cells, and where two of them clash.
#[tauri::command(async)]
fn map_cells_cmd(paths: StudioPaths) -> Result<map_cells::MapCells, String> {
    let z = zomboid_dir_of(&paths)?;
    let saved = vfs::studio_paths();
    Ok(map_cells::map_cells(&saved.pz_install_dir, &saved.workshop_dir, &z))
}

/// The real map under the grid: the game's world and each map mod with an image pyramid, put
/// together at about `px_per_cell` pixels per cell (cached).
#[tauri::command(async)]
fn map_images_cmd(paths: StudioPaths, px_per_cell: u32) -> Result<Vec<map_cells::images::MapImage>, String> {
    let z = zomboid_dir_of(&paths)?;
    let saved = vfs::studio_paths();
    let cells = map_cells::map_cells(&saved.pz_install_dir, &saved.workshop_dir, &z);
    Ok(map_cells::images::map_images(&saved.pz_install_dir, &map_cells::image_dirs(&cells), &z, px_per_cell.clamp(4, 64)))
}

/// The PNG of one map image made by map_images_cmd, as raw bytes.
#[tauri::command(async)]
fn map_image_bytes_cmd(paths: StudioPaths, id: String) -> Result<tauri::ipc::Response, String> {
    let z = zomboid_dir_of(&paths)?;
    Ok(tauri::ipc::Response::new(map_cells::images::image_bytes(&z, &id)?))
}

/// Which Mods screen the game uses when another mod manager is on: (choice, that manager).
#[tauri::command(async)]
fn link_screen_choice_cmd(paths: StudioPaths) -> Result<(String, String), String> {
    Ok(studio_link::screen_choice(&zomboid_dir_of(&paths)?))
}

#[tauri::command(async)]
fn set_link_screen_choice_cmd(paths: StudioPaths, screen: String) -> Result<(), String> {
    let z = zomboid_dir_of(&paths)?;
    let (_, other) = studio_link::screen_choice(&z);
    studio_link::set_screen_choice(&z, &screen, &other)
}

/// The player's notes on mods and groups of mods.
#[tauri::command(async)]
fn mod_meta_cmd(paths: StudioPaths) -> Result<mod_meta::ModMeta, String> {
    let z = zomboid_dir_of(&paths)?;
    let meta = mod_meta::read(&z);
    // The Link's copy, in case the game's folder came after the note.
    mod_meta::write_link_copy(&z, &meta);
    Ok(meta)
}

#[tauri::command(async)]
fn set_mod_note_cmd(paths: StudioPaths, mod_id: String, note: String) -> Result<mod_meta::ModMeta, String> {
    mod_meta::set_note(&zomboid_dir_of(&paths)?, &mod_id, &note)
}

#[tauri::command(async)]
fn set_in_mod_group_cmd(paths: StudioPaths, group: String, mod_ids: Vec<String>, inside: bool) -> Result<mod_meta::ModMeta, String> {
    mod_meta::set_in_group(&zomboid_dir_of(&paths)?, &group, &mod_ids, inside)
}

#[tauri::command(async)]
fn rename_mod_group_cmd(paths: StudioPaths, from: String, to: String) -> Result<mod_meta::ModMeta, String> {
    mod_meta::rename_group(&zomboid_dir_of(&paths)?, &from, &to)
}

#[tauri::command(async)]
fn set_mod_group_note_cmd(paths: StudioPaths, name: String, note: String) -> Result<mod_meta::ModMeta, String> {
    mod_meta::set_group_note(&zomboid_dir_of(&paths)?, &name, &note)
}

#[tauri::command(async)]
fn delete_mod_group_cmd(paths: StudioPaths, name: String) -> Result<mod_meta::ModMeta, String> {
    mod_meta::delete_group(&zomboid_dir_of(&paths)?, &name)
}

/// How the Studio's window opens and what its close button does.
#[tauri::command]
fn window_prefs_cmd() -> window_prefs::WindowPrefs {
    window_prefs::prefs()
}

#[tauri::command]
fn set_window_prefs_cmd<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, prefs: window_prefs::WindowPrefs) {
    window_prefs::set_prefs(&app_handle, prefs);
}

/// Everything a bug report needs, as text, with the user name and the PC's name masked.
#[tauri::command(async)]
fn diagnostics_report_cmd(paths: StudioPaths) -> Result<String, String> {
    let z = zomboid_dir_of(&paths)?;
    let saved = vfs::studio_paths();
    Ok(diagnostics::report(&z, &saved.pz_install_dir, &saved.workshop_dir))
}

/// A line from the window into the Studio's log (the errors it shows).
#[tauri::command]
fn log_from_window_cmd(level: String, text: String) {
    let level = if level == "ERROR" || level == "WARN" { level } else { "INFO".to_string() };
    diagnostics::log(&level, &text.chars().take(2000).collect::<String>());
}

/// Opens the folder of the Studio's log.
#[tauri::command(async)]
fn open_studio_log_folder_cmd() -> Result<(), String> {
    let dir = diagnostics::log_path().and_then(|p| p.parent().map(|d| d.to_path_buf())).ok_or("The Zomboid folder was not found.")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    open::that(&dir).map_err(|e| e.to_string())
}

/// Every save, its backups, and the room on the disk that holds them.
#[tauri::command(async)]
fn saves_overview_cmd(paths: StudioPaths) -> Result<save_backup::SavesOverview, String> {
    Ok(save_backup::overview(&zomboid_dir_of(&paths)?))
}

/// What every installed mod takes on the disk, and which of them nothing uses.
#[tauri::command(async)]
fn disk_report_cmd(paths: StudioPaths) -> Result<disk_usage::DiskReport, String> {
    let z_dir = zomboid_dir_of(&paths)?;
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    Ok(disk_usage::report(&paths.pz_install_dir, &paths.workshop_dir, &z_dir, &catalog))
}

/// An unused local mod folder or leftover Workshop folder to the Recycle Bin. The report is made
/// again here, so only a folder that is unused right now can go.
#[tauri::command(async)]
fn disk_recycle_cmd(paths: StudioPaths, folder: String) -> Result<(), String> {
    let z_dir = zomboid_dir_of(&paths)?;
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let report = disk_usage::report(&paths.pz_install_dir, &paths.workshop_dir, &z_dir, &catalog);
    disk_usage::recycle(&report, &folder, sandbox::find_running_pz_pid().is_some())
}

/// For each save whose own list names mods that are not installed: those mods.
#[tauri::command(async)]
fn save_missing_mods_cmd(paths: StudioPaths) -> Result<std::collections::BTreeMap<String, Vec<save_backup::MissingMod>>, String> {
    let z_dir = zomboid_dir_of(&paths)?;
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    Ok(save_backup::missing_mods(&z_dir, &catalog))
}

/// For each save that misses a map its own mods bring (a map or interiors mod switched on after
/// the save was made): the folders it loads and the ones it misses, with the bare chunks already
/// generated in their cells.
#[tauri::command(async)]
fn save_missing_maps_cmd(paths: StudioPaths) -> Result<std::collections::BTreeMap<String, save_backup::maps::SaveMaps>, String> {
    let z = zomboid_dir_of(&paths)?;
    let saved = vfs::studio_paths();
    let cells = map_cells::map_cells(&saved.pz_install_dir, &saved.workshop_dir, &z);
    Ok(save_backup::maps::missing_maps(&z, &saved.pz_install_dir, &cells))
}

/// Gives a save the maps its mods bring (in front of its list, as a new game would have them) and
/// moves the bare chunks of their cells out of the save, after a backup. Refused while the game runs.
/// The active package's copies of mod files beside the mod's own: which are out of date.
#[tauri::command(async)]
fn package_file_states_cmd(paths: StudioPaths, active: Vec<String>) -> Vec<patch_generator::stale::PackageFileState> {
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    patch_generator::stale::package_file_states(&catalog, &active)
}

/// For each console line given: whether its shape was seen in an earlier session, in how many,
/// and since when.
#[tauri::command(async)]
fn error_history_cmd(paths: StudioPaths, lines: Vec<String>) -> Result<sandbox::history::ErrorHistoryView, String> {
    let z = zomboid_dir_of(&paths)?;
    Ok(sandbox::history::lookup(&z, &lines))
}

/// What the active mods (or `mod_ids`) could do beyond modding: programs shipped, Java that runs
/// programs or reaches the net, Lua that does what the game's never does. Heuristics, cached per mod.
#[tauri::command(async)]
fn scan_mod_security_cmd(paths: StudioPaths, mod_ids: Option<Vec<String>>) -> Result<Vec<security::ModScan>, String> {
    let z = zomboid_dir_of(&paths)?;
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let ids: Vec<String> = match mod_ids {
        Some(ids) if !ids.is_empty() => ids,
        _ => load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default(),
    };
    Ok(security::scan_mods(&z, &catalog, &ids))
}

/// The characters and vehicles of a save, with positions and the item types they carry.
#[tauri::command(async)]
fn save_inside_cmd(paths: StudioPaths, save_id: String) -> Result<save_backup::inside::SaveInside, String> {
    let z = zomboid_dir_of(&paths)?;
    save_backup::inside::read(&z, &save_id)
}

#[tauri::command(async)]
fn save_add_maps_cmd(paths: StudioPaths, save_id: String) -> Result<save_backup::maps::SaveMapsFix, String> {
    let z = zomboid_dir_of(&paths)?;
    let saved = vfs::studio_paths();
    let cells = map_cells::map_cells(&saved.pz_install_dir, &saved.workshop_dir, &z);
    let result = save_backup::maps::add_maps(&z, &saved.pz_install_dir, &cells, &save_id, sandbox::find_running_pz_pid().is_some());
    studio_link::write_saves_file(&z);
    result
}

/// For each save with a recorded session: mods added, removed and updated since.
#[tauri::command(async)]
fn save_session_diffs_cmd(paths: StudioPaths) -> Result<std::collections::BTreeMap<String, studio_link::SessionDiff>, String> {
    Ok(studio_link::session_diffs(&zomboid_dir_of(&paths)?))
}

/// A backup the player asked for: kept until they remove it.
#[tauri::command(async)]
fn backup_save_cmd(paths: StudioPaths, save_id: String) -> Result<save_backup::BackupOutcome, String> {
    let z = zomboid_dir_of(&paths)?;
    let result = save_backup::backup_save(&z, &save_id, save_backup::REASON_MANUAL, true);
    studio_link::write_saves_file(&z);
    result
}

/// Puts a backup back as the save (backing the save up first). Refused while the game runs.
#[tauri::command(async)]
fn restore_save_backup_cmd(paths: StudioPaths, save_id: String, file: String) -> Result<Option<save_backup::BackupOutcome>, String> {
    let z = zomboid_dir_of(&paths)?;
    let result = save_backup::restore_backup(&z, &save_id, &file, sandbox::find_running_pz_pid().is_some());
    studio_link::write_saves_file(&z);
    result
}

#[tauri::command(async)]
fn delete_save_backup_cmd(paths: StudioPaths, save_id: String, file: String) -> Result<(), String> {
    let z = zomboid_dir_of(&paths)?;
    let result = save_backup::delete_backup(&z, &save_id, &file);
    studio_link::write_saves_file(&z);
    result
}

/// Opens the folder the save backups go to (made if it is not there yet).
#[tauri::command(async)]
fn open_backups_folder_cmd(paths: StudioPaths) -> Result<(), String> {
    let dir = zomboid_dir_of(&paths)?.join(save_backup::BACKUP_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    open::that(&dir).map_err(|e| e.to_string())
}

/// Kept: never taken out to make room for newer automatic backups.
#[tauri::command(async)]
fn set_save_backup_kept_cmd(paths: StudioPaths, save_id: String, file: String, kept: bool) -> Result<save_backup::BackupEntry, String> {
    save_backup::set_kept(&zomboid_dir_of(&paths)?, &save_id, &file, kept)
}

/// Where the player is in the game (as the Link mod last wrote it) and what each save last ran with.
#[tauri::command(async)]
fn link_status_cmd(paths: StudioPaths) -> studio_link::LinkStatus {
    match load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next() {
        Some(z_dir) => studio_link::link_status(&z_dir),
        None => studio_link::LinkStatus::default(),
    }
}

/// What each mod did to the last session's log: exceptions and warnings the game laid at its door.
#[tauri::command(async)]
fn mod_error_stats_cmd(paths: StudioPaths) -> Vec<sandbox::ModErrorStat> {
    sandbox::error_stats_from_console(&paths.user_zomboid_dir)
}

/// Whether a (canonical) path is inside the game, Workshop or Zomboid folder the Studio saved.
/// The page's own idea of those folders is not asked: a page that could name them could open,
/// or have the editor open, anything on the disk.
fn inside_studio_folders(target: &std::path::Path) -> bool {
    let paths = vfs::studio_paths();
    [&paths.workshop_dir, &paths.user_zomboid_dir, &paths.pz_install_dir]
        .iter()
        .filter(|d| !d.is_empty())
        .filter_map(|d| std::fs::canonicalize(d).ok())
        .any(|root| target.starts_with(root))
}

/// The names a mod's Lua calls that the installed build does not have (globals, methods,
/// events), by file and line, with the other active mods' definitions counted as known.
#[tauri::command(async)]
fn api_lint_mod_cmd(paths: StudioPaths, mod_id: String, root_dir: String) -> Result<api_lint::ModApiReport, String> {
    let root = std::fs::canonicalize(&root_dir).map_err(|_| format!("{} is not there any more.", root_dir))?;
    if !inside_studio_folders(&root) {
        return Err("That folder is outside your game and Zomboid folders.".to_string());
    }
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let active = load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default();
    api_lint::check_mod(&paths, &catalog, &mod_id, &root, &active)
}

/// The player's own Workshop items (`Zomboid/Workshop/<item>`), as their workshop.txt says.
#[tauri::command(async)]
fn workshop_uploads_cmd(paths: StudioPaths) -> Result<Vec<workshop_publish::WorkshopItemMeta>, String> {
    Ok(workshop_publish::list_uploads(&zomboid_dir_of(&paths)?, &paths.workshop_dir))
}

/// Writes an item's title, description, tags and visibility to its workshop.txt.
#[tauri::command(async)]
fn workshop_upload_save_meta_cmd(paths: StudioPaths, folder: String, title: String, description: String, tags: Vec<String>, visibility: String) -> Result<workshop_publish::WorkshopItemMeta, String> {
    workshop_publish::save_meta(&zomboid_dir_of(&paths)?, &folder, &title, &description, &tags, &visibility)
}

/// Makes a Workshop item folder for a mod that has none: the mod copied under Contents/mods, a
/// workshop.txt, its poster as the preview.
#[tauri::command(async)]
fn workshop_upload_prepare_cmd(paths: StudioPaths, root_dir: String) -> Result<workshop_publish::WorkshopItemMeta, String> {
    let root = std::fs::canonicalize(&root_dir).map_err(|_| format!("{} is not there any more.", root_dir))?;
    if !inside_studio_folders(&root) {
        return Err("That folder is outside your game and Zomboid folders.".to_string());
    }
    workshop_publish::prepare_upload(&zomboid_dir_of(&paths)?, &paths.workshop_dir, &root)
}

/// Takes an item folder out of Zomboid/Workshop (to the Recycle Bin). Nothing on Steam changes.
#[tauri::command(async)]
fn workshop_upload_remove_cmd(paths: StudioPaths, folder: String) -> Result<(), String> {
    workshop_publish::remove_item(&zomboid_dir_of(&paths)?, &folder)
}

/// The files an upload would send, with what changed since the last upload from here.
#[tauri::command(async)]
fn workshop_upload_preview_cmd(paths: StudioPaths, folder: String) -> Result<workshop_publish::UploadPreview, String> {
    workshop_publish::preview(&zomboid_dir_of(&paths)?, &paths.workshop_dir, &folder)
}

/// Publishes the item through the running Steam client; progress as `workshop-upload-progress` events.
#[tauri::command(async)]
fn workshop_publish_cmd(app: tauri::AppHandle, paths: StudioPaths, folder: String, change_note: String) -> Result<workshop_publish::PublishOutcome, String> {
    let z = zomboid_dir_of(&paths)?;
    workshop_publish::publish(&paths.pz_install_dir, &z, &paths.workshop_dir, &folder, &change_note, |p| {
        let _ = tauri::Emitter::emit(&app, "workshop-upload-progress", p);
    })
}

/// A mod's translation languages: the source language and how much of it each language has.
#[tauri::command(async)]
fn translation_drafts_status_cmd(root_dir: String) -> Result<translation_drafts::DraftStatus, String> {
    let root = std::fs::canonicalize(&root_dir).map_err(|_| format!("{} is not there any more.", root_dir))?;
    if !inside_studio_folders(&root) {
        return Err("That folder is outside your game and Zomboid folders.".to_string());
    }
    translation_drafts::status(&root)
}

/// Writes the missing keys of those languages, translated from the mod's source language.
#[tauri::command(async)]
fn write_translation_drafts_cmd(root_dir: String, langs: Vec<String>) -> Result<translation_drafts::DraftReport, String> {
    let root = std::fs::canonicalize(&root_dir).map_err(|_| format!("{} is not there any more.", root_dir))?;
    if !inside_studio_folders(&root) {
        return Err("That folder is outside your game and Zomboid folders.".to_string());
    }
    translation_drafts::write_drafts(&vfs::studio_paths().user_zomboid_dir, &root, &langs)
}

/// Every Lua and script file of a mod, checked: one row per problem with file, line and column.
#[tauri::command(async)]
fn validate_mod_cmd(paths: StudioPaths, root_dir: String) -> Result<devtools::ModValidation, String> {
    let root = std::fs::canonicalize(&root_dir).map_err(|_| format!("{} is not there any more.", root_dir))?;
    let _ = &paths;
    if !inside_studio_folders(&root) {
        return Err("That folder is outside your game and Zomboid folders.".to_string());
    }
    if !devtools::looks_like_a_mod(&root) {
        return Err("That folder has no mod.info.".to_string());
    }
    devtools::validate_mod(&root)
}

/// Opens a file of a mod at a line: VS Code when it is on the PATH, else the file's own program.
#[tauri::command(async)]
fn open_in_editor_cmd(paths: StudioPaths, file: String, line: Option<u32>, col: Option<u32>) -> Result<String, String> {
    let target = std::fs::canonicalize(&file).map_err(|_| format!("{} is not there.", file))?;
    let _ = &paths;
    if !inside_studio_folders(&target) {
        return Err("That file is outside your game and Zomboid folders.".to_string());
    }
    devtools::open_in_editor(&target, line, col)
}

/// The multiplayer check of one mod folder.
#[tauri::command(async)]
fn mp_check_mod_cmd(paths: StudioPaths, root_dir: String) -> Result<mplint::MpReport, String> {
    let root = std::fs::canonicalize(&root_dir).map_err(|_| format!("{} is not there any more.", root_dir))?;
    let _ = &paths;
    if !inside_studio_folders(&root) {
        return Err("That folder is outside your game and Zomboid folders.".to_string());
    }
    let game = (!paths.pz_install_dir.is_empty()).then(|| std::path::PathBuf::from(&paths.pz_install_dir));
    // The description the game reads for this build's folder, then common/, then the root.
    let description = ["42/mod.info", "common/mod.info", "mod.info"]
        .iter()
        .find_map(|rel| std::fs::read_to_string(root.join(rel)).ok())
        .map(|text| crate::catalog::modinfo::parse_game_mod_info(&text).description);
    mplint::assess(&root, game.as_deref(), description.as_deref())
}

/// One report per mod, by id, for the mods named (the active list, usually).
#[derive(serde::Serialize)]
struct MpModReport {
    mod_id: String,
    name: String,
    report: mplint::MpReport,
}

#[tauri::command(async)]
fn mp_check_mods_cmd(paths: StudioPaths, mod_ids: Vec<String>) -> Vec<MpModReport> {
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let game = (!paths.pz_install_dir.is_empty()).then(|| std::path::PathBuf::from(&paths.pz_install_dir));
    mod_ids
        .iter()
        .filter(|id| !id.starts_with("Z_PZModStudio_"))
        .filter_map(|id| catalog.resolve(id.trim()))
        .filter_map(|m| {
            mplint::assess(std::path::Path::new(&m.root_dir), game.as_deref(), Some(m.info.description.as_str()))
                .ok()
                .map(|report| MpModReport { mod_id: m.id.clone(), name: m.name.clone(), report })
        })
        .collect()
}

/// Where TileZed and WorldEd are, if anywhere: the named folder first, then the usual places.
#[tauri::command(async)]
fn map_tools_cmd(configured: Vec<String>) -> devtools::MapTools {
    devtools::find_map_tools(&configured)
}

/// Starts TileZed or WorldEd (only those) from its own folder.
#[tauri::command(async)]
fn launch_map_tool_cmd(exe: String) -> Result<(), String> {
    devtools::launch_map_tool(&exe)
}

/// A new mod under Zomboid/mods, laid out the way Build 42 reads it.
#[tauri::command(async)]
fn create_mod_cmd(user_zomboid_dir: String, new_mod: devtools::NewMod) -> Result<devtools::CreatedMod, String> {
    devtools::create_mod(&user_zomboid_dir, &new_mod)
}

/// The mod's Lua files changed after a moment (unix ms); all of them for 0.
#[tauri::command(async)]
fn changed_lua_files_cmd(root_dir: String, since_ms: u64) -> Vec<devtools::ChangedFile> {
    devtools::changed_lua_files(std::path::Path::new(&root_dir), since_ms)
}

/// Starts watching a mod's folder: every Lua file saved under it is checked and reloaded in the
/// running game, each stage sent as a `dev-reload` event.
#[tauri::command(async)]
fn dev_watch_start_cmd(app: tauri::AppHandle, paths: StudioPaths, root_dir: String) -> Result<(), String> {
    devtools::start_dev_watch(app, std::path::PathBuf::from(root_dir), paths.user_zomboid_dir)
}

#[tauri::command(async)]
fn dev_watch_stop_cmd(paths: StudioPaths) {
    devtools::stop_dev_watch(&paths.user_zomboid_dir);
}

/// Reloads every Lua file of the mod now (the same stages as the watcher, as events).
#[tauri::command(async)]
fn dev_reload_all_cmd(app: tauri::AppHandle, paths: StudioPaths, root_dir: String) {
    let files = devtools::changed_lua_files(std::path::Path::new(&root_dir), 0);
    std::thread::spawn(move || {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        devtools::send_reload(&app, &paths.user_zomboid_dir, files, now);
    });
}

/// A mod's own changelog (its entries, newest first, and its author's support links), if it ships one.
#[tauri::command(async)]
fn mod_changelog_cmd(paths: StudioPaths, mod_id: String) -> Option<mod_changelog::ModChangelog> {
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let m = catalog.mods.iter().find(|m| m.id == mod_id && m.shadowed_by.is_none())?;
    mod_changelog::for_mod(std::path::Path::new(&m.root_dir))
}

/// What's new: a local mod's changelog entries are read.
#[tauri::command(async)]
fn changelog_mark_read_cmd(paths: StudioPaths, mod_id: String) -> Result<(), String> {
    whats_new::mark_changelog_read(&zomboid_dir_of(&paths)?, &mod_id)
}

/// A server's Discord settings, and whether a webhook is stored for it.
#[tauri::command(async)]
fn discord_view_cmd(paths: StudioPaths, server: String) -> Result<discord::DiscordView, String> {
    Ok(discord::view(&zomboid_dir_of(&paths)?, &server))
}

/// Saves them; `webhook` "" forgets the stored address, a new one replaces it, none keeps it.
#[tauri::command(async)]
fn discord_save_cmd(paths: StudioPaths, server: String, settings: discord::DiscordSettings, webhook: Option<String>) -> Result<discord::DiscordView, String> {
    discord::save(&zomboid_dir_of(&paths)?, &server, settings, webhook)
}

/// A server `.ini` of this Zomboid folder's `Server/` (nothing else is read or written as one).
fn server_ini_of(paths: &StudioPaths, file_path: &str) -> Result<std::path::PathBuf, String> {
    let dir = zomboid_dir_of(paths)?.join("Server");
    let path = std::path::PathBuf::from(file_path);
    let inside = path.parent().and_then(|p| p.canonicalize().ok()) == dir.canonicalize().ok();
    if !inside || path.extension().and_then(|e| e.to_str()) != Some("ini") || !path.is_file() {
        return Err(format!("{} is not a server .ini of Zomboid/Server.", file_path));
    }
    Ok(path)
}

/// What a server's `.ini` says about the game's own Discord bot (never the token).
#[tauri::command(async)]
fn game_bot_cmd(paths: StudioPaths, file_path: String) -> Result<discord::GameBot, String> {
    discord::game_bot(&server_ini_of(&paths, &file_path)?)
}

/// Writes the game's own Discord bot settings into the server's `.ini`.
#[tauri::command(async)]
fn save_game_bot_cmd(paths: StudioPaths, file_path: String, enabled: bool, token: Option<String>, chat_channel: String, log_channel: String, clear_command_channel: bool) -> Result<discord::GameBot, String> {
    let ini = server_ini_of(&paths, &file_path)?;
    discord::save_game_bot(&zomboid_dir_of(&paths)?, &ini, enabled, token, &chat_channel, &log_channel, clear_command_channel)
}

/// Posts a test message to the server's channel.
/// How the Discord commands are doing (None while they are off).
#[tauri::command]
fn discord_commands_status_cmd() -> Option<discord_commands::CommandsStatus> {
    discord_commands::status()
}

/// Kick, ban or message the players of a server this Studio runs, over RCON. `action` is "kick",
/// "ban" or "message"; the server's answer comes back.
#[tauri::command(async)]
fn server_player_action_cmd(paths: StudioPaths, server: String, action: String, player: Option<String>, text: Option<String>) -> Result<String, String> {
    use server_manager::PlayerAction;
    let player = player.unwrap_or_default();
    let text = text.unwrap_or_default();
    let action = match action.as_str() {
        "kick" => PlayerAction::Kick { user: &player, reason: &text },
        "ban" => PlayerAction::Ban { user: &player, reason: &text },
        "message" => PlayerAction::Message { text: &text },
        other => return Err(format!("Unknown action: {}", other)),
    };
    server_manager::player_action(&zomboid_dir_of(&paths)?, &server, &action)
}

#[tauri::command(async)]
fn discord_test_cmd(paths: StudioPaths, server: String) -> Result<(), String> {
    discord::send_test(&zomboid_dir_of(&paths)?, &server)
}

/// Whether ProjectZomboid64.exe is running, and its pid.
#[tauri::command(async)]
fn game_status_cmd() -> Option<u32> {
    sandbox::find_running_pz_pid()
}

/// The player's lists of mods (Favorites first), from Zomboid/PZModStudio_Lists.json.
#[tauri::command(async)]
fn mod_lists_cmd(user_zomboid_dir: String) -> Vec<lists::ModList> {
    lists::load_lists(&user_zomboid_dir)
}

#[tauri::command(async)]
fn create_mod_list_cmd(user_zomboid_dir: String, name: String) -> Result<lists::ModList, String> {
    lists::create_list(&user_zomboid_dir, &name)
}

#[tauri::command(async)]
fn rename_mod_list_cmd(user_zomboid_dir: String, list_id: String, name: String) -> Result<lists::ModList, String> {
    lists::rename_list(&user_zomboid_dir, &list_id, &name)
}

#[tauri::command(async)]
fn delete_mod_list_cmd(user_zomboid_dir: String, list_id: String) -> Result<Vec<lists::ModList>, String> {
    lists::delete_list(&user_zomboid_dir, &list_id)
}

#[tauri::command(async)]
fn add_to_mod_list_cmd(user_zomboid_dir: String, list_id: String, item: lists::ListItem) -> Result<lists::ModList, String> {
    lists::add_to_list(&user_zomboid_dir, &list_id, item)
}

#[tauri::command(async)]
fn remove_from_mod_list_cmd(user_zomboid_dir: String, list_id: String, key: String) -> Result<lists::ModList, String> {
    lists::remove_from_list(&user_zomboid_dir, &list_id, &key)
}

/// Favorites and the game's own stars made one list (see `lists::sync_game_favourites`).
#[tauri::command(async)]
fn sync_game_favourites_cmd(user_zomboid_dir: String, installed: Vec<lists::InstalledMod>) -> Result<Vec<lists::ModList>, String> {
    lists::sync_game_favourites(&user_zomboid_dir, &installed)
}

/// How long each mod's files took to load in the last session, from the game's DebugLog.
#[tauri::command(async)]
fn mod_load_times_cmd(paths: StudioPaths) -> sandbox::load_times::LoadTimes {
    sandbox::load_times::from_logs(&paths.user_zomboid_dir)
}

/// The memory the game's Java may use, the PC's, and what the Studio suggests.
#[tauri::command(async)]
fn game_memory_cmd(paths: StudioPaths) -> game_memory::GameMemory {
    game_memory::status(&paths.pz_install_dir, &paths.user_zomboid_dir)
}

/// Gives the game this much memory (MB) from its next start.
#[tauri::command(async)]
fn set_game_memory_cmd(paths: StudioPaths, mb: u64) -> Result<game_memory::GameMemory, String> {
    game_memory::set(&paths.pz_install_dir, &paths.user_zomboid_dir, mb)
}

/// The game's last starts, stage by stage, newest first (from its DebugLogs).
#[tauri::command(async)]
fn boot_history_cmd(paths: StudioPaths) -> Vec<sandbox::load_times::BootTimes> {
    sandbox::load_times::boot_history(&paths.user_zomboid_dir, 12)
}

/// Whether the in-game profiler is on, and what it measured last.
#[tauri::command(async)]
fn profiler_status_cmd(paths: StudioPaths) -> sandbox::profiler::ProfilerStatus {
    sandbox::profiler::status(&paths.user_zomboid_dir, &paths.workshop_dir)
}

/// What Viewpoint's Smooth Frames mod wrote to the console this session: frame times by the
/// minute and the Lua handlers, per mod, that cost the most. None when it wrote nothing.
#[tauri::command(async)]
fn smooth_frames_cmd(paths: StudioPaths) -> Option<sandbox::smooth_frames::SmoothFramesReport> {
    sandbox::smooth_frames::read(&paths.user_zomboid_dir)
}

/// Puts the profiler first in the mod list (and in the last save's) or takes it out.
#[tauri::command(async)]
fn profiler_set_cmd(paths: StudioPaths, on: bool) -> Result<sandbox::profiler::ProfilerStatus, String> {
    sandbox::profiler::set_on(&paths.user_zomboid_dir, &paths.workshop_dir, on)
}

/// The active profile was just played: remember on which build.
#[tauri::command(async)]
fn mark_profile_played_cmd(user_zomboid_dir: String, instance_id: String) -> Result<Option<String>, String> {
    instance_manager::mark_instance_played(user_zomboid_dir, instance_id)
}

#[tauri::command(async)]
fn list_kept_versions_cmd(paths: StudioPaths) -> Vec<snapshots::KeptVersion> {
    snapshots::list_kept_versions(&paths.user_zomboid_dir)
}

/// Puts a kept version back as a local copy that wins over the Workshop original.
#[tauri::command(async)]
fn restore_kept_version_cmd(paths: StudioPaths, path: String) -> Result<snapshots::KeptVersion, String> {
    snapshots::restore_kept_version(&paths.user_zomboid_dir, &path)
}

#[tauri::command(async)]
fn delete_kept_version_cmd(paths: StudioPaths, path: String) -> Result<(), String> {
    snapshots::delete_kept_version(&paths.user_zomboid_dir, &path)
}

/// Game-exact load-order sort of the whole list (enabled and disabled), keeping the user's order
/// wherever no require/loadModAfter/loadModBefore rule forces a change.
#[tauri::command(async)]
fn sort_load_order_cmd(paths: StudioPaths, order: Vec<String>) -> catalog::SortResult {
    let mut catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    // The player's own rules, as if each mod had written loadModAfter itself.
    rules::apply_to_catalog(&mut catalog, &rules::load_rules(&paths.user_zomboid_dir));
    rules::apply_to_catalog(&mut catalog, &rules::community::current().as_user_rules());
    rules::apply_to_catalog(&mut catalog, &rules::index::current(&paths.user_zomboid_dir).as_user_rules());
    catalog::sort_load_order(&catalog, &order)
}

/// The Studio's mod index as the app carries it, less the issues this player dismissed: it is small.
#[tauri::command(async)]
fn community_index_cmd(paths: StudioPaths) -> rules::index::CommunityIndex {
    rules::index::current(&paths.user_zomboid_dir)
}

/// Shows again every known issue this player dismissed; how many.
#[tauri::command(async)]
fn restore_index_issues_cmd(paths: StudioPaths) -> Result<usize, String> {
    rules::index::clear_dismissed(&paths.user_zomboid_dir)
}

/// Marks a known issue of the index as no longer happening on this machine, or brings it back.
#[tauri::command(async)]
fn dismiss_index_issue_cmd(paths: StudioPaths, mod_id: String, title: String, dismissed: bool) -> Result<(), String> {
    rules::index::set_dismissed(&paths.user_zomboid_dir, &mod_id, &title, dismissed)
}

/// Whether a newer PZ Mod Studio is out (asked of GitHub once a day while the check is on);
/// `force` is Settings' "Check now".
#[tauri::command(async)]
fn app_update_status_cmd(user_zomboid_dir: String, force: bool) -> app_update::UpdateStatus {
    app_update::check(&user_zomboid_dir, force)
}

/// "Update now": the release `version` is downloaded, checked (signed checksums, then the zip's
/// SHA-256) and put in place of the running app, which then starts the new one and closes.
#[tauri::command(async)]
fn self_update_cmd<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>, version: String) -> Result<(), String> {
    let exe = self_update::install(&version)?;
    std::process::Command::new(&exe).spawn().map_err(|e| format!("The new version is in place but did not start: {}. Open it again by hand.", e))?;
    app_handle.exit(0);
    Ok(())
}

#[tauri::command(async)]
fn set_app_update_check_cmd(user_zomboid_dir: String, enabled: bool) -> Result<app_update::UpdateStatus, String> {
    app_update::set_enabled(&user_zomboid_dir, enabled)
}

/// "Not now" on the notice: quiet until the next version.
#[tauri::command(async)]
fn dismiss_app_update_cmd(user_zomboid_dir: String, version: String) -> Result<(), String> {
    app_update::dismiss(&user_zomboid_dir, &version)
}

#[tauri::command(async)]
fn list_load_rules_cmd(paths: StudioPaths) -> Vec<rules::UserRule> {
    rules::load_rules(&paths.user_zomboid_dir)
}

#[tauri::command(async)]
fn add_load_rule_cmd(paths: StudioPaths, mod_id: String, after: String, note: Option<String>) -> Result<Vec<rules::UserRule>, String> {
    rules::add_rule(&paths.user_zomboid_dir, &mod_id, &after, note)
}

#[tauri::command(async)]
fn remove_load_rule_cmd(paths: StudioPaths, mod_id: String, after: String) -> Result<Vec<rules::UserRule>, String> {
    rules::remove_rule(&paths.user_zomboid_dir, &mod_id, &after)
}

#[tauri::command(async)]
fn launch_sandbox_cmd<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    mut config: SandboxLaunchConfig,
) -> Result<u32, String> {
    config.pz_install_dir = vfs::trusted_install_dir();
    launch_sandbox_and_watch(app_handle, config)
}

/// The hunt in progress, if the player left one running.
/// Things two or more active mods declare (the same item, recipe or vehicle in different files).
/// Read-only, and asked for on its own because it reads every script of every active mod.
#[tauri::command(async)]
fn script_conflicts_cmd(paths: StudioPaths, active: Vec<String>) -> Vec<catalog::HealthIssue> {
    catalog::script_definition_issues(&paths, &active)
}

/// Every earlier version of the mod list the Studio kept, newest first.
/// What the Build 42 shim covers, read from the shim that ships inside merge packages.
/// What a link offers, worked out without downloading it.
/// The player's own sandbox presets, so one can be put into a pack.
/// The text files of one installed mod, to pick one to patch.
/// For each `require("...") failed` asked about, where that module really is.
///
/// Indexing every Lua file of the game and the active mods takes a moment, so the whole set is
/// answered in one call rather than one card at a time.
#[tauri::command(async)]
fn resolve_failed_requires_cmd(
    paths: StudioPaths,
    wanted: Vec<String>,
) -> Vec<sandbox::requires::RequireAnswer> {
    let active: Vec<String> = load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path)
        .map(|d| d.active_mods)
        .unwrap_or_default();
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let index = sandbox::requires::index_modules(&paths, &catalog, &active);
    wanted.iter().map(|w| sandbox::requires::answer_for(w, &index)).collect()
}

#[tauri::command(async)]
fn mod_files_cmd(paths: StudioPaths, mod_id: String) -> Result<Vec<vfs::mod_patch::ModFile>, String> {
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    vfs::mod_patch::files_of_mod(&catalog, &mod_id)
}

/// The files of a mod that `require` this module: where a known path fix has to go.
#[tauri::command(async)]
fn mod_files_requiring_cmd(paths: StudioPaths, mod_id: String, module: String) -> Result<Vec<vfs::mod_patch::ModFile>, String> {
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    vfs::mod_patch::files_requiring(&catalog, &mod_id, &module)
}

/// Every file of every active mod that `require`s this module: the log does not always say which.
#[tauri::command(async)]
fn active_mods_files_requiring_cmd(paths: StudioPaths, module: String) -> Vec<vfs::mod_patch::AskingFile> {
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let active = patch_generator::get_active_mod_ids_from_disk(&paths.user_zomboid_dir, &paths.mod_list_ini_path);
    vfs::mod_patch::files_requiring_in_active_mods(&catalog, &active, &module)
}

/// One file out of a package, published in place: the game reads the mod's own copy again.
#[tauri::command(async)]
fn remove_file_from_package_cmd(
    user_zomboid_dir: String,
    mod_list_ini_path: String,
    package_folder_name: String,
    file: String,
) -> Result<MasterPatchResult, String> {
    patch_generator::remove_file_from_package(&user_zomboid_dir, &mod_list_ini_path, &package_folder_name, &file)
}

/// One mod's file, in the shape the Merger edits and saves into the package.
#[tauri::command(async)]
fn mod_file_to_patch_cmd(
    paths: StudioPaths,
    mod_id: String,
    relative_path: String,
) -> Result<vfs::VfsConflictRaw, String> {
    let catalog = catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    vfs::mod_patch::conflict_for_mod_file(&catalog, &paths, &mod_id, &relative_path)
}

#[tauri::command(async)]
fn sandbox_presets_cmd(
    user_zomboid_dir: String,
) -> Vec<preset_manager::sandbox_presets::SandboxPresetInfo> {
    preset_manager::sandbox_presets::list(&user_zomboid_dir)
}

/// Writes a pack's sandbox settings into `Zomboid/Sandbox Presets`, never over one of the
/// player's own. Returns the name the game will show.
#[tauri::command(async)]
fn write_sandbox_preset_cmd(
    user_zomboid_dir: String,
    sandbox: preset_manager::sandbox_presets::PresetSandbox,
) -> Result<String, String> {
    preset_manager::sandbox_presets::write_from_pack(&user_zomboid_dir, &sandbox)
}

#[tauri::command(async)]
fn resolve_url_install_cmd(url: String) -> Result<mod_install::from_url::UrlInstallPlan, String> {
    mod_install::from_url::resolve(&url)
}

/// Downloads a link and installs the mod(s) in it, recording where they came from.
#[tauri::command(async)]
fn install_from_url_cmd(
    paths: StudioPaths,
    url: String,
    replace_existing: bool,
) -> Result<mod_install::InstallResult, String> {
    mod_install::from_url::install_from_url(
        &url,
        &paths.user_zomboid_dir,
        &paths.workshop_dir,
        replace_existing,
    )
}

#[tauri::command(async)]
fn polyfill_sections_cmd() -> Vec<patch_generator::PolyfillSection> {
    patch_generator::polyfill_sections()
}

/// The shim's per-mod patches with their switches.
#[tauri::command(async)]
fn shim_patches_cmd(user_zomboid_dir: String) -> Vec<patch_generator::ShimPatch> {
    patch_generator::shim_patches(&user_zomboid_dir)
}

/// Switches one per-mod patch and writes the switches into every Studio package on disk.
#[tauri::command(async)]
fn set_shim_patch_cmd(user_zomboid_dir: String, name: String, enabled: bool) -> Result<usize, String> {
    patch_generator::set_patch_switch(&user_zomboid_dir, &name, enabled)
}

#[tauri::command(async)]
fn mod_list_history_cmd(paths: StudioPaths) -> Vec<load_order::ini_parser::ModListVersion> {
    load_order::ini_parser::list_mod_list_history(&paths.mod_list_ini_path, &paths.user_zomboid_dir)
}

/// Puts one of them back, and says which mods it switches on.
#[tauri::command(async)]
fn restore_mod_list_version_cmd(paths: StudioPaths, backup_file: String) -> Result<Vec<String>, String> {
    load_order::ini_parser::restore_mod_list_version(
        &paths.mod_list_ini_path,
        &paths.user_zomboid_dir,
        &backup_file,
    )
}

#[tauri::command(async)]
fn bisect_state_cmd(paths: StudioPaths) -> Option<bisect::BisectSession> {
    bisect::load(&paths.user_zomboid_dir)
}

/// Starts halving the list. The mods switched on now are the suspects; the list is replaced by
/// the first trial and kept in the session to be put back at the end.
#[tauri::command(async)]
fn bisect_start_cmd(
    paths: StudioPaths,
    mods: Vec<bisect::BisectMod>,
) -> Result<bisect::BisectSession, String> {
    bisect::start(&paths.mod_list_ini_path, &paths.user_zomboid_dir, &mods)
}

/// Did it still happen? Narrows the suspects and switches the next trial on.
#[tauri::command(async)]
fn bisect_answer_cmd(
    paths: StudioPaths,
    still_happens: bool,
) -> Result<bisect::BisectSession, String> {
    bisect::answer(&paths.mod_list_ini_path, &paths.user_zomboid_dir, still_happens)
}

/// Gives up and puts the player's own list back, optionally without one mod.
#[tauri::command(async)]
fn bisect_stop_cmd(paths: StudioPaths, switch_off: Option<String>) -> Result<Vec<String>, String> {
    bisect::stop(
        &paths.mod_list_ini_path,
        &paths.user_zomboid_dir,
        switch_off.as_deref(),
    )
}

#[tauri::command(async)]
fn generate_master_patch_cmd(req: MasterPatchRequest) -> Result<MasterPatchResult, String> {
    generate_master_patch(req)
}

#[tauri::command]
fn pick_folder_cmd(default_path: Option<String>) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(ref path) = default_path {
        if !path.is_empty() {
            dialog = dialog.set_directory(path);
        }
    }
    dialog.pick_folder().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_save_file_cmd(
    default_name: Option<String>,
    filter_name: Option<String>,
    filter_ext: Option<String>,
) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    if let (Some(ref fname), Some(ref fext)) = (filter_name, filter_ext) {
        dialog = dialog.add_filter(fname, &[fext.as_str()]);
    } else {
        dialog = dialog.add_filter("PZ Mod Studio File", &["pzmerge", "pzpack", "json"]);
    }
    if let Some(name) = default_name {
        dialog = dialog.set_file_name(&name);
    }
    dialog.save_file().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_open_file_cmd(
    filter_name: Option<String>,
    filter_ext: Option<String>,
) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    if let (Some(ref fname), Some(ref fext)) = (filter_name, filter_ext) {
        // "zip,7z": one filter, several extensions.
        let exts: Vec<&str> = fext.split(',').map(str::trim).filter(|e| !e.is_empty()).collect();
        dialog = dialog.add_filter(fname, &exts);
    } else {
        dialog = dialog.add_filter("PZ Mod Studio File", &["pzmerge", "pzpack", "json"]);
    }
    dialog.pick_file().map(|p| p.to_string_lossy().to_string())
}

/// Builds one Markdown file describing an error: what broke, on which build, in which mod, with
/// the code. Meant to be handed to a person or an AI agent that was not at the machine.
#[tauri::command(async)]
fn error_report_cmd(paths: StudioPaths, request: error_report::ErrorReportRequest) -> Result<String, String> {
    error_report::build(&paths, &request)
}

/// Writes that report where the user chose.
#[tauri::command(async)]
fn save_text_file_cmd(path: String, contents: String) -> Result<(), String> {
    // Only a text report: a page that could call this with any name could otherwise drop a
    // .bat or .lnk anywhere the user can write (the Startup folder, say).
    if !has_extension(&path, &["md", "txt"]) {
        return Err("Only .md and .txt files can be saved here.".to_string());
    }
    std::fs::write(&path, contents).map_err(|e| format!("The file could not be saved: {}", e))
}

fn has_extension(path: &str, allowed: &[&str]) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| allowed.iter().any(|a| e.eq_ignore_ascii_case(a)))
}

/// Where the Studio's own MCP server sits, so the config the user copies is one that works.
#[tauri::command(async)]
fn mcp_server_path_cmd() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let beside = exe.with_file_name("pz-mcp-server.exe");
    // In a portable copy the console server sits next to the app; falling back to the app itself
    // is still right, because it serves MCP with --mcp.
    Ok(if beside.is_file() { beside } else { exe }.to_string_lossy().to_string())
}

/// Shows a mod's folder in Explorer. The path comes from the UI, so it is checked against the
/// folders the user configured first: this opens mods, not anywhere on the disk.
#[tauri::command(async)]
fn open_mod_folder_cmd(paths: StudioPaths, folder: String) -> Result<(), String> {
    let target = std::fs::canonicalize(&folder).map_err(|_| format!("{} is not there any more.", folder))?;
    let _ = &paths;
    if !inside_studio_folders(&target) {
        return Err("That folder is outside your game and Zomboid folders.".to_string());
    }
    open::that(&target).map_err(|e| format!("Windows could not open that folder: {}", e))?;
    Ok(())
}

#[tauri::command(async)]
fn open_package_folder_cmd(user_zomboid_dir: String, package_folder_name: String) -> Result<bool, String> {
    // One plain folder name in the package namespace, under the Zomboid folder the Studio saved:
    // the page cannot point this at another folder, or at a program to open.
    let _ = user_zomboid_dir;
    let clean_pkg_name = patch_generator::to_package_folder(&package_folder_name)?;

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(&vfs::studio_paths().user_zomboid_dir);
    if user_dirs.is_empty() {
        return Err("User Zomboid folder not found.".to_string());
    }

    let pkg_dir = user_dirs[0].join("mods").join(&clean_pkg_name);
    let _ = std::fs::create_dir_all(&pkg_dir);

    open::that(&pkg_dir).map_err(|e| format!("Error opening physical folder: {}", e))?;
    Ok(true)
}

#[tauri::command]
fn open_logs_folder_cmd(user_zomboid_dir: String) -> Result<bool, String> {
    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(&user_zomboid_dir);
    let target = if !user_dirs.is_empty() {
        let logs_dir = user_dirs[0].join("Logs");
        if logs_dir.exists() {
            logs_dir
        } else {
            user_dirs[0].clone()
        }
    } else {
        if let Some(home) = dirs_next::home_dir() {
            let p = home.join("Zomboid").join("Logs");
            if p.exists() { p } else { home.join("Zomboid") }
        } else {
            return Err("Zomboid folder not found.".to_string());
        }
    };

    open::that(&target).map_err(|e| format!("Error opening logs folder: {}", e))?;
    Ok(true)
}

#[tauri::command(async)]
fn list_available_log_files_cmd(user_zomboid_dir: String) -> Vec<sandbox::LogFileInfo> {
    sandbox::list_available_log_files(&user_zomboid_dir)
}

#[tauri::command(async)]
fn read_log_file_cmd(file_path: String, max_lines: Option<usize>) -> Result<Vec<String>, String> {
    if !has_extension(&file_path, &["txt", "log"]) {
        return Err("Only .txt and .log files are read as logs.".to_string());
    }
    sandbox::read_log_file(&file_path, max_lines)
}

#[tauri::command]
fn open_external_url_cmd(url: String) -> Result<(), String> {
    let url = validate_external_url(&url)?;
    open::that(url).map_err(|e| format!("Failed to open URL in browser: {}", e))
}

/// Only plain http(s) links are opened. URLs come from mod.info `url=` lines, which mod
/// authors control, so anything else (file:, ms-settings:, shell metacharacters...) is refused.
fn validate_external_url(url: &str) -> Result<&str, String> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    let has_web_scheme = lower.starts_with("https://") || lower.starts_with("http://");
    let has_control_chars = url.chars().any(|c| c.is_control() || c == '"');
    if !has_web_scheme || has_control_chars || url.len() > 2048 {
        return Err(format!("Refusing to open non-web URL: {}", url));
    }
    Ok(url)
}

#[tauri::command(async)]
fn clean_master_patch_cmd(req: MasterPatchRequest) -> Result<bool, String> {
    patch_generator::clean_master_patch(req)
}

/// One more file into a package, published in place: what it held stays, packaged stays packaged.
#[tauri::command(async)]
fn publish_file_into_package_cmd(
    user_zomboid_dir: String,
    mod_list_ini_path: String,
    package_folder_name: String,
    relative_path: String,
    content: String,
) -> Result<MasterPatchResult, String> {
    patch_generator::publish_file_into_package(
        &user_zomboid_dir,
        &mod_list_ini_path,
        &package_folder_name,
        &relative_path,
        &content,
    )
}

#[tauri::command(async)]
fn get_master_patch_status_cmd(user_zomboid_dir: String, mod_list_ini_path: String, package_folder_name: Option<String>) -> patch_generator::MasterPatchStatusInfo {
    patch_generator::get_master_patch_status(&user_zomboid_dir, &mod_list_ini_path, package_folder_name)
}

#[tauri::command(async)]
fn list_merged_packages_cmd(user_zomboid_dir: String, mod_list_ini_path: String) -> Vec<patch_generator::MergedPackageInfo> {
    patch_generator::list_merged_packages(&user_zomboid_dir, &mod_list_ini_path)
}

#[tauri::command]
fn create_merged_package_cmd(user_zomboid_dir: String, mod_list_ini_path: String, name: String, description: Option<String>) -> Result<patch_generator::MergedPackageInfo, String> {
    patch_generator::create_merged_package(&user_zomboid_dir, &mod_list_ini_path, &name, description.as_deref())
}

#[tauri::command]
fn rename_merged_package_cmd(user_zomboid_dir: String, mod_list_ini_path: String, old_folder: String, new_name: String, description: Option<String>) -> Result<patch_generator::MergedPackageInfo, String> {
    patch_generator::rename_merged_package(&user_zomboid_dir, &mod_list_ini_path, &old_folder, &new_name, description.as_deref())
}

#[tauri::command(async)]
fn delete_merged_package_cmd(user_zomboid_dir: String, mod_list_ini_path: String, folder_name: String) -> Result<bool, String> {
    patch_generator::delete_merged_package(&user_zomboid_dir, &mod_list_ini_path, &folder_name)
}

#[tauri::command]
fn save_draft_resolution_cmd(
    user_zomboid_dir: String,
    package_folder_name: String,
    relative_path: String,
    resolved_content: String,
    status: String,
) -> Result<bool, String> {
    patch_generator::save_draft_resolution(
        &user_zomboid_dir,
        &package_folder_name,
        &relative_path,
        &resolved_content,
        &status,
    )
}

#[tauri::command]
fn clear_draft_resolutions_cmd(
    user_zomboid_dir: String,
    package_folder_name: String,
) -> Result<bool, String> {
    patch_generator::clear_draft_resolutions(&user_zomboid_dir, &package_folder_name)
}

#[tauri::command(async)]
fn export_merged_package_cmd(
    user_zomboid_dir: String,
    package_folder_name: String,
    target_file_path: String,
) -> Result<bool, String> {
    patch_generator::export_merged_package(&user_zomboid_dir, &package_folder_name, &target_file_path)
}

#[tauri::command(async)]
fn import_merged_package_cmd(
    user_zomboid_dir: String,
    mod_list_ini_path: String,
    source_file_path: String,
) -> Result<patch_generator::MergedPackageInfo, String> {
    patch_generator::import_merged_package(&user_zomboid_dir, &mod_list_ini_path, &source_file_path)
}

#[tauri::command]
fn toggle_package_in_modlist_cmd(
    user_zomboid_dir: String,
    mod_list_ini_path: String,
    folder_name: String,
    enabled: bool,
) -> Result<bool, String> {
    patch_generator::toggle_package_in_modlist(&user_zomboid_dir, &mod_list_ini_path, &folder_name, enabled)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // What a previous "Update now" renamed aside.
            self_update::remove_old_copies();
            // A panic anywhere is logged with where it happened, and the window is told.
            let handle = app.handle().clone();
            diagnostics::install_panic_hook(move |text| {
                let _ = tauri::Emitter::emit(&handle, diagnostics::PANIC_EVENT, text);
            });
            diagnostics::log("INFO", &format!("PZ Mod Studio {} started", env!("CARGO_PKG_VERSION")));
            // Follow console.txt from the start: the game is often launched from Steam.
            sandbox::watch_console_log(app.handle().clone());
            // What the game's Mods screen changes comes back to the active profile.
            studio_link::watch_mod_list(app.handle().clone());
            // The hosted server's news to its Discord channel, when the player set one.
            discord::watch();
            discord_commands::watch();
            // The window opens where and how it was left (it starts hidden so it does not jump).
            if let Some(window) = app.get_webview_window("main") {
                window_prefs::restore(&window);
                let _ = window.show();
                window_prefs::restore_state(&window);
            }
            if window_prefs::prefs().close_to_tray {
                window_prefs::ensure_tray(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let Some(webview) = window.app_handle().get_webview_window(window.label()) else { return };
            match event {
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => window_prefs::remember(&webview),
                tauri::WindowEvent::CloseRequested { api, .. } => window_prefs::on_close_requested(&webview, api),
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_auto_paths,
            set_and_validate_paths,
            scan_conflicts_cmd,
            validate_lua_syntax_cmd,
            three_way_merge_lua_cmd,
            read_mod_list_ini_cmd,
            write_mod_list_ini_cmd,
            scan_all_installed_mods_cmd,
            get_mod_health_cmd,
            auto_merge_conflicts_cmd,
            sort_load_order_cmd,
            list_load_rules_cmd,
            community_index_cmd,
            dismiss_index_issue_cmd,
            restore_index_issues_cmd,
            app_update_status_cmd,
            self_update_cmd,
            server_player_action_cmd,
            set_app_update_check_cmd,
            dismiss_app_update_cmd,
            add_load_rule_cmd,
            remove_load_rule_cmd,
            make_compatible_copy_cmd,
            remove_compatible_copy_cmd,
            keep_mod_version_cmd,
            list_kept_versions_cmd,
            mark_profile_played_cmd,
            mod_error_stats_cmd,
            write_link_health_cmd,
            write_link_mods_cmd,
            link_status_cmd,
            saves_overview_cmd,
            map_cells_cmd,
            mod_meta_cmd,
            set_mod_note_cmd,
            set_in_mod_group_cmd,
            rename_mod_group_cmd,
            delete_mod_group_cmd,
            set_mod_group_note_cmd,
            link_screen_choice_cmd,
            set_link_screen_choice_cmd,
            window_prefs_cmd,
            set_window_prefs_cmd,
            public_servers_cmd,
            stop_server_search_cmd,
            server_rules_cmd,
            saved_servers_cmd,
            save_server_cmd,
            forget_server_cmd,
            set_server_made_profile_cmd,
            join_server_cmd,
            map_images_cmd,
            map_image_bytes_cmd,
            diagnostics_report_cmd,
            log_from_window_cmd,
            open_studio_log_folder_cmd,
            whats_new_cmd,
            save_session_diffs_cmd,
            save_missing_mods_cmd,
            save_missing_maps_cmd,
            save_add_maps_cmd,
            save_inside_cmd,
            scan_mod_security_cmd,
            error_history_cmd,
            package_file_states_cmd,
            set_launch_save_cmd,
            disk_report_cmd,
            disk_recycle_cmd,
            java_agents_plan_cmd,
            backup_save_cmd,
            restore_save_backup_cmd,
            delete_save_backup_cmd,
            set_save_backup_kept_cmd,
            open_backups_folder_cmd,
            mod_load_times_cmd,
            profiler_status_cmd,
            smooth_frames_cmd,
            boot_history_cmd,
            game_memory_cmd,
            set_game_memory_cmd,
            profiler_set_cmd,
            mod_lists_cmd,
            validate_mod_cmd,
            api_lint_mod_cmd,
            workshop_uploads_cmd,
            workshop_upload_save_meta_cmd,
            workshop_upload_prepare_cmd,
            workshop_upload_preview_cmd,
            workshop_upload_remove_cmd,
            workshop_publish_cmd,
            translation_drafts_status_cmd,
            write_translation_drafts_cmd,
            open_in_editor_cmd,
            game_status_cmd,
            create_mod_cmd,
            map_tools_cmd,
            mp_check_mod_cmd,
            mp_check_mods_cmd,
            launch_map_tool_cmd,
            changed_lua_files_cmd,
            dev_watch_start_cmd,
            discord_view_cmd,
            game_bot_cmd,
            save_game_bot_cmd,
            mod_changelog_cmd,
            changelog_mark_read_cmd,
            discord_save_cmd,
            discord_test_cmd,
            discord_commands_status_cmd,
            dev_watch_stop_cmd,
            dev_reload_all_cmd,
            create_mod_list_cmd,
            rename_mod_list_cmd,
            delete_mod_list_cmd,
            add_to_mod_list_cmd,
            remove_from_mod_list_cmd,
            sync_game_favourites_cmd,
            restore_kept_version_cmd,
            delete_kept_version_cmd,
            nexus_search_cmd,
            nexus_categories_cmd,
            nexus_mod_details_cmd,
            workshop_item_details_cmd,
            workshop_requirements_cmd,
            install_mods_cmd,
            workshop_details_cmd,
            workshop_lookup_cmd,
            steam_key_status_cmd,
            set_steam_key_cmd,
            delete_steam_key_cmd,
            translate_texts_cmd,
            deepl_key_status_cmd,
            set_deepl_key_cmd,
            delete_deepl_key_cmd,
            translation_cache_info_cmd,
            clear_translation_cache_cmd,
            workshop_search_cmd,
            read_mod_image_cmd,
            workshop_subscribe_cmd,
            steam_client_ready_cmd,
            steam_downloads_cmd,
            launch_sandbox_cmd,
            script_conflicts_cmd,
            resolve_failed_requires_cmd,
            mod_files_cmd,
            mod_files_requiring_cmd,
            active_mods_files_requiring_cmd,
            remove_file_from_package_cmd,
            mod_file_to_patch_cmd,
            sandbox_presets_cmd,
            write_sandbox_preset_cmd,
            resolve_url_install_cmd,
            install_from_url_cmd,
            polyfill_sections_cmd,
            shim_patches_cmd,
            set_shim_patch_cmd,
            mod_list_history_cmd,
            restore_mod_list_version_cmd,
            bisect_state_cmd,
            bisect_start_cmd,
            bisect_answer_cmd,
            bisect_stop_cmd,
            generate_master_patch_cmd,
            clean_master_patch_cmd,
            publish_file_into_package_cmd,
            get_master_patch_status_cmd,
            list_merged_packages_cmd,
            create_merged_package_cmd,
            rename_merged_package_cmd,
            delete_merged_package_cmd,
            toggle_package_in_modlist_cmd,
            save_draft_resolution_cmd,
            clear_draft_resolutions_cmd,
            export_merged_package_cmd,
            import_merged_package_cmd,
            open_package_folder_cmd,
            open_mod_folder_cmd,
            mcp_server_path_cmd,
            error_report_cmd,
            save_text_file_cmd,
            open_logs_folder_cmd,
            list_available_log_files_cmd,
            read_log_file_cmd,
            pick_folder_cmd,
            pick_save_file_cmd,
            pick_open_file_cmd,
            open_external_url_cmd,
            export_preset_file,
            import_preset_file,
            check_missing_preset_mods,
            list_server_configs,
            sync_client_to_server,
            check_server_mods,
            create_new_server_config,
            delete_server_config,
            launch_dedicated_server,
            stop_dedicated_server,
            get_dedicated_server_status,
            get_dedicated_server_logs,
            save_server_log_snapshot,
            get_connected_players,
            send_server_command,
            get_server_quick_settings,
            save_server_quick_settings,
            list_instances,
            create_instance,
            activate_instance,
            delete_instance,
            update_instance,
            save_master_load_order,
            instance_manager::set_profile_members
        ])
        // A dedicated server outlives this window on purpose: people close the Studio and keep
        // playing. Stopping it is the Servers tab's button, where the user means it.

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_helpers_answer_is_found_among_steams_chatter() {
        let output = "Setting breakpad minidump AppID = 108600\nSteamInternal_SetMinidumpSteamID: ...\nPZMS-STEAM:{\"done\":[\"7\"]}\n";
        assert_eq!(super::steam_answer(output).as_deref(), Some("{\"done\":[\"7\"]}"));
        assert_eq!(super::steam_answer("only steam noise\n"), None);
    }

    use super::validate_external_url;

    #[test]
    fn only_web_urls_are_opened() {
        assert!(validate_external_url("https://steamcommunity.com/sharedfiles/filedetails/?id=1&b=2").is_ok());
        assert!(validate_external_url(" HTTP://example.com ").is_ok());
        for bad in ["steam://url/CommunityFilePage/2699828474", "steam://run/108600", "ftp://x&calc", "file:///C:/Windows/System32/calc.exe", "calc.exe", "ms-settings:", "https://x\"&calc", "https://x\r\ncalc"] {
            assert!(validate_external_url(bad).is_err(), "accepted {:?}", bad);
        }
    }
}

#[cfg(test)]
mod fixture_dump {
    /// Writes what the app's main commands answer on this machine to PZMS_DUMP (JSON), for
    /// screenshots with the real data. Read-only. PZMS_ZOMBOID_DIR, PZMS_DUMP.
    #[test]
    #[ignore]
    fn dump_the_main_commands() {
        let (Ok(_), Ok(out)) = (std::env::var("PZMS_ZOMBOID_DIR"), std::env::var("PZMS_DUMP")) else { return };
        let paths = crate::vfs::auto_detect_paths();
        let active = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path).map(|d| d.active_mods).unwrap_or_default();
        let dump = serde_json::json!({
            "paths": paths,
            "scan_all_installed_mods_cmd": super::scan_all_installed_mods_cmd(paths.clone()),
            "get_mod_health_cmd": super::get_mod_health_cmd(paths.clone(), active.clone()),
            "scan_conflicts_cmd": super::scan_conflicts_cmd(paths.clone(), None),
            "scan_conflicts_for_package": super::scan_conflicts_cmd(paths.clone(), Some("Z_PZModStudio_MergedPatch".to_string())),
            "list_merged_packages_cmd": super::list_merged_packages_cmd(paths.user_zomboid_dir.clone(), paths.mod_list_ini_path.clone()),
            "mod_error_stats_cmd": super::mod_error_stats_cmd(paths.clone()),
            "mod_load_times_cmd": super::mod_load_times_cmd(paths.clone()),
            "polyfill_sections_cmd": super::polyfill_sections_cmd(),
            "shim_patches_cmd": super::shim_patches_cmd(paths.user_zomboid_dir.clone()),
            "community_index_cmd": super::community_index_cmd(paths.clone()),
            "list_instances": crate::instance_manager::list_instances(paths.user_zomboid_dir.clone()).unwrap_or_default(),
            "mod_lists_cmd": super::mod_lists_cmd(paths.user_zomboid_dir.clone()),
        });
        std::fs::write(out, serde_json::to_string(&dump).unwrap()).unwrap();
    }

    /// One page of the public Workshop browse, mods and collections, for the Browse screenshots.
    #[test]
    #[ignore]
    fn dump_a_workshop_page() {
        let Ok(out) = std::env::var("PZMS_DUMP_WS") else { return };
        let mods: crate::workshop::BrowseQuery = serde_json::from_value(serde_json::json!({"perPage": 12, "sort": "trending", "build": "42", "days": 7, "collections": false, "page": 1})).unwrap();
        let colls: crate::workshop::BrowseQuery = serde_json::from_value(serde_json::json!({"perPage": 8, "sort": "trending", "days": 7, "collections": true, "page": 1})).unwrap();
        let dump = serde_json::json!({
            "mods": super::workshop_search_cmd(mods).ok(),
            "collections": super::workshop_search_cmd(colls).ok(),
        });
        std::fs::write(out, serde_json::to_string(&dump).unwrap()).unwrap();
    }}
