use serde::{Deserialize, Serialize};
use std::fs;
use crate::load_order::mod_info::get_all_user_zomboid_dirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstance {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub is_active: bool,
    pub active_mod_ids: Vec<String>,
    pub load_order: Vec<String>,
    /// The game build this profile was last played on from the Studio ("42.21"), if ever.
    /// Saves do not cross builds, so a profile played on one build is worth a word when the
    /// installed game is another.
    #[serde(default)]
    pub last_played_build: Option<String>,
    /// The major build this profile is meant for ("41", "42"), set by hand for a profile not yet
    /// played; None means "whatever it was last played on".
    #[serde(default)]
    pub target_build: Option<String>,
    /// The mods that belong to the profile, on or off; every other installed mod is outside it
    /// and neither the Mod List nor the game's Mods screen shows it. None for a profile from
    /// before this existed: its mods are the ones it has on.
    #[serde(default)]
    pub members: Option<Vec<String>>,
    /// The memory (MB) the game gets with this profile on (`-Xmx`); None: whatever was chosen last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_memory_mb: Option<u64>,
}

impl AppInstance {
    /// The profile's mods, on or off, in its load order (mods it has but cannot place go last).
    pub fn member_ids(&self) -> Vec<String> {
        let mut set: Vec<String> = self.members.clone().unwrap_or_else(|| self.active_mod_ids.clone());
        for id in &self.active_mod_ids {
            if !set.contains(id) {
                set.push(id.clone());
            }
        }
        let mut out: Vec<String> = self.load_order.iter().filter(|id| set.contains(id)).cloned().collect();
        for id in set {
            if !out.contains(&id) {
                out.push(id);
            }
        }
        out
    }

    /// The Studio Link mod is on in every profile and belongs to it.
    fn with_link(&mut self) {
        let link = crate::studio_link::LINK_MOD_ID.to_string();
        if !self.active_mod_ids.contains(&link) {
            self.active_mod_ids.push(link.clone());
        }
        let mut members = self.members.clone().unwrap_or_else(|| self.active_mod_ids.clone());
        if !members.contains(&link) {
            members.push(link);
        }
        self.members = Some(members);
    }

    /// The major build the profile is for: the one set by hand, else the one it was last played on.
    pub fn build_major(&self) -> Option<String> {
        self.target_build
            .clone()
            .or_else(|| self.last_played_build.as_ref().map(|b| b.split('.').next().unwrap_or(b).to_string()))
    }
}

/// What the game needs for the active profile: the Link mod installed, the file it reads, and
/// the Java agents of the profile's mods (Zombie Buddy) in or out of the game's launch file.
fn apply_link(z_dir: &std::path::Path, instance: &AppInstance) {
    if let Err(e) = crate::studio_link::install_link_mod(z_dir) {
        crate::diagnostics::error(format!("[StudioLink] {}", e));
    }
    // Groups are each profile's own: the game shows this one's.
    crate::mod_meta::refresh_link_copy(z_dir);
    // What the other profiles hold, so the game can tell a mod that is in no profile at all.
    let mut others: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(z_dir.join("PZModStudio_Instances")) {
        for path in entries.flatten().map(|e| e.path()) {
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if let Some(other) = read_instance(&path).filter(|p| p.id != instance.id) {
                for id in other.member_ids() {
                    if !others.contains(&id) {
                        others.push(id);
                    }
                }
            }
        }
    }
    if let Err(e) = crate::studio_link::write_link_file(z_dir, &instance.id, &instance.name, &instance.member_ids(), &others) {
        crate::diagnostics::error(format!("[StudioLink] {}", e));
    }
    // Tests run on throwaway folders and must never edit the real game's launch file.
    let install = if cfg!(test) { String::new() } else { crate::vfs::studio_paths().pz_install_dir };
    if !install.is_empty() {
        if let Err(e) = crate::studio_link::sync_java_agents_when_free(std::path::Path::new(&install), z_dir, &instance.active_mod_ids) {
            crate::diagnostics::error(format!("[StudioLink] {}", e));
        }
        // The profile's memory; while the game runs, its next start from the Studio sets it.
        if instance.is_active && crate::sandbox::find_running_pz_pid().is_none() {
            crate::game_memory::reapply_for_launch(std::path::Path::new(&install), z_dir);
        }
    }
}

fn read_instance(path: &std::path::Path) -> Option<AppInstance> {
    fs::read_to_string(path).ok().and_then(|c| serde_json::from_str::<AppInstance>(&c).ok())
}

fn write_instance(path: &std::path::Path, instance: &AppInstance) -> Result<(), String> {
    let json = serde_json::to_string_pretty(instance).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(path, json).map_err(|e| e.to_string())
}

/// The active profile and its file, if there is one.
fn find_active(dir: &std::path::Path) -> Option<(std::path::PathBuf, AppInstance)> {
    fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find_map(|path| {
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            return None;
        }
        read_instance(&path).filter(|i| i.is_active).map(|i| (path, i))
    })
}

/// The game wrote `default.txt` (its Mods screen was accepted): the active profile takes the
/// mods it switched on and their order, and every mod switched on becomes one of its mods.
/// Returns the profile when it changed; None when it already matched (the Studio's own write).
pub fn take_mod_list_from_game(z_dir: &std::path::Path, game_active: &[String]) -> Result<Option<AppInstance>, String> {
    let dir = z_dir.join("PZModStudio_Instances");
    let Some((path, mut inst)) = find_active(&dir) else { return Ok(None) };
    if inst.active_mod_ids == game_active {
        return Ok(None);
    }
    inst.load_order = crate::studio_link::merge_game_order(&inst.load_order, game_active);
    inst.active_mod_ids = game_active.to_vec();
    let mut members = inst.member_ids();
    for id in game_active {
        if !members.contains(id) {
            members.push(id.clone());
        }
    }
    inst.members = Some(members);
    write_instance(&path, &inst)?;

    // The Studio's saved order follows, so the Mod List opens the way the game left it.
    let master = serde_json::json!({
        "load_order": inst.load_order,
        "active_mod_ids": inst.active_mod_ids,
        "active_profile_id": inst.id,
        "active_profile_name": inst.name
    });
    let _ = crate::load_order::ini_parser::atomic_write(z_dir.join("PZModStudio_MasterLoadOrder.json"), serde_json::to_string_pretty(&master).unwrap_or_default());
    apply_link(z_dir, &inst);
    Ok(Some(inst))
}

/// Mods the game asked to add to the active profile (the Link mod's "add the new ones"): they
/// join it switched off, at the end of its order. Returns the profile when it changed.
pub fn add_members_to_active(z_dir: &std::path::Path, ids: &[String]) -> Result<Option<AppInstance>, String> {
    let dir = z_dir.join("PZModStudio_Instances");
    let Some((path, mut inst)) = find_active(&dir) else { return Ok(None) };
    let mut members = inst.member_ids();
    let mut changed = false;
    for id in ids {
        if !members.contains(id) {
            members.push(id.clone());
            changed = true;
        }
        if !inst.load_order.contains(id) {
            inst.load_order.push(id.clone());
        }
    }
    if !changed {
        return Ok(None);
    }
    inst.members = Some(members);
    write_instance(&path, &inst)?;
    apply_link(z_dir, &inst);
    Ok(Some(inst))
}

/// The profile a server was joined with becomes what the server loaded: those mods in it and on,
/// in the server's order and first in the load order (the ones the game had the player download
/// too); any other mod it holds stays in it, off, as the server does not load it. `also_off`
/// joins it switched off: the other mods of the Workshop items the server had downloaded, so they
/// are in a profile. `keep_on` stay on when they were: mods that run only on the player's side
/// (ZombieBuddy and its client-only Java mods), which the server does not list and does not need.
/// None when it already was so.
pub fn take_server_mods(user_zomboid_dir: &str, instance_id: &str, server_mods: &[String], also_off: &[String], keep_on: &[String]) -> Result<Option<AppInstance>, String> {
    let dir = get_instances_dir(user_zomboid_dir);
    let path = instance_file_path(&dir, instance_id)?;
    let Some(mut inst) = read_instance(&path) else { return Ok(None) };
    let mut active: Vec<String> = Vec::new();
    for id in server_mods.iter().map(|id| id.trim().trim_start_matches('\\').to_string()).filter(|id| !id.is_empty()) {
        if !active.contains(&id) {
            active.push(id);
        }
    }
    for id in keep_on {
        if inst.active_mod_ids.contains(id) && !active.contains(id) {
            active.push(id.clone());
        }
    }
    let mut members = inst.members.clone().unwrap_or_else(|| inst.active_mod_ids.clone());
    for id in active.iter().chain(also_off) {
        if !members.contains(id) {
            members.push(id.clone());
        }
    }
    let mut order = active.clone();
    for id in &inst.load_order {
        if !order.contains(id) {
            order.push(id.clone());
        }
    }
    let before = (inst.active_mod_ids.clone(), inst.members.clone(), inst.load_order.clone());
    inst.active_mod_ids = active;
    inst.members = Some(members);
    inst.load_order = order;
    inst.with_link();
    if (inst.active_mod_ids.clone(), inst.members.clone(), inst.load_order.clone()) == before {
        return Ok(None);
    }
    write_instance(&path, &inst)?;
    if inst.is_active {
        if let Some(z_dir) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next() {
            crate::load_order::ini_parser::write_mod_list_ini(&z_dir.join("ModListData.ini").to_string_lossy(), &inst.active_mod_ids)?;
            apply_link(&z_dir, &inst);
        }
    }
    Ok(Some(inst))
}

/// Sets which mods belong to a profile. A mod taken out of it is switched off too.
#[tauri::command(async)]
pub fn set_profile_members(user_zomboid_dir: String, instance_id: String, members: Vec<String>) -> Result<AppInstance, String> {
    let dir = get_instances_dir(&user_zomboid_dir);
    let path = instance_file_path(&dir, &instance_id)?;
    let mut inst = read_instance(&path).ok_or_else(|| "Selected profile does not exist.".to_string())?;
    let mut members = members;
    let link = crate::studio_link::LINK_MOD_ID.to_string();
    if !members.contains(&link) {
        members.push(link);
    }
    inst.active_mod_ids.retain(|id| members.contains(id));
    for id in &members {
        if !inst.load_order.contains(id) {
            inst.load_order.push(id.clone());
        }
    }
    inst.members = Some(members);
    write_instance(&path, &inst)?;
    if inst.is_active {
        if let Some(z_dir) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&user_zomboid_dir).into_iter().next() {
            crate::load_order::ini_parser::write_mod_list_ini(&z_dir.join("ModListData.ini").to_string_lossy(), &inst.active_mod_ids)?;
            apply_link(&z_dir, &inst);
        }
    }
    Ok(inst)
}

/// Path of a profile's JSON file. Profile IDs come from the UI and are joined onto a folder
/// path, so anything other than a plain identifier is refused.
fn instance_file_path(dir: &std::path::Path, instance_id: &str) -> Result<std::path::PathBuf, String> {
    let valid = !instance_id.is_empty()
        && instance_id.len() <= 64
        && instance_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !valid {
        return Err(format!("Invalid profile id: '{}'", instance_id));
    }
    Ok(dir.join(format!("{}.json", instance_id)))
}

/// Keeps `mb` as the active profile's game memory. Returns the profile's name, or None when no
/// profile is active.
pub fn set_active_memory(z_dir: &std::path::Path, mb: u64) -> Result<Option<String>, String> {
    let Some((path, mut instance)) = find_active(&z_dir.join("PZModStudio_Instances")) else { return Ok(None) };
    instance.game_memory_mb = Some(mb);
    write_instance(&path, &instance)?;
    Ok(Some(instance.name))
}

/// The active profile's game memory and name, if one is active.
pub fn active_memory(z_dir: &std::path::Path) -> Option<(Option<u64>, String)> {
    find_active(&z_dir.join("PZModStudio_Instances")).map(|(_, i)| (i.game_memory_mb, i.name))
}

/// The active profile's id and name, if one is active.
pub fn active_profile(user_zomboid_dir: &str) -> Option<(String, String)> {
    find_active(&get_instances_dir(user_zomboid_dir)).map(|(_, i)| (i.id, i.name))
}

fn get_instances_dir(user_zomboid_dir: &str) -> std::path::PathBuf {
    let user_dirs = get_all_user_zomboid_dirs(user_zomboid_dir);
    user_dirs[0].join("PZModStudio_Instances")
}

#[tauri::command(async)]
pub fn list_instances(user_zomboid_dir: String) -> Result<Vec<AppInstance>, String> {
    let dir = get_instances_dir(&user_zomboid_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut instances = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(inst) = serde_json::from_str::<AppInstance>(&content) {
                        instances.push(inst);
                    }
                }
            }
        }
    }

    instances.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(instances)
}

#[tauri::command]
pub fn create_instance(
    user_zomboid_dir: String,
    name: String,
    description: Option<String>,
    active_mod_ids: Vec<String>,
    load_order: Vec<String>,
) -> Result<AppInstance, String> {
    let dir = get_instances_dir(&user_zomboid_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let clean_name = name.trim();
    if clean_name.is_empty() {
        return Err("Instance name cannot be empty.".to_string());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let timestamp = now.as_secs();
    // Milliseconds so two profiles created in the same second do not overwrite each other.
    let id = format!("inst_{}", now.as_millis());
    let created_at = format!("{}", timestamp);

    // A new profile holds the mods it starts with switched on, and the Studio Link mod.
    let mut instance = AppInstance {
        id: id.clone(),
        name: clean_name.to_string(),
        description,
        created_at,
        is_active: false,
        members: Some(active_mod_ids.clone()),
        active_mod_ids,
        load_order,
        last_played_build: None,
        target_build: None,
        game_memory_mb: None,
    };
    instance.with_link();

    let file_path = dir.join(format!("{}.json", id));
    let json = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(&file_path, json).map_err(|e| e.to_string())?;

    Ok(instance)
}

#[tauri::command(async)]
pub fn activate_instance(user_zomboid_dir: String, instance_id: String) -> Result<(), String> {
    let dir = get_instances_dir(&user_zomboid_dir);
    let instance_file = instance_file_path(&dir, &instance_id)?;
    if !instance_file.exists() {
        return Err("Selected instance does not exist.".to_string());
    }

    let content = fs::read_to_string(&instance_file).map_err(|e| e.to_string())?;
    let mut target_instance: AppInstance = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    target_instance.is_active = true;
    target_instance.with_link();

    // 1. The game's list first: if it cannot be written, nothing has switched, and the profile
    // still marked active is the one whose mods are on.
    let z_dir = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;
    let _ = fs::create_dir_all(z_dir.join("mods"));
    crate::load_order::ini_parser::write_mod_list_ini(
        &z_dir.join("ModListData.ini").to_string_lossy(),
        &target_instance.active_mod_ids,
    )?;

    // 2. Update active state across all instance JSON files
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(c) = fs::read_to_string(&path) {
                    if let Ok(mut inst) = serde_json::from_str::<AppInstance>(&c) {
                        inst.is_active = inst.id == instance_id;
                        let _ = crate::load_order::ini_parser::atomic_write(&path, serde_json::to_string_pretty(&inst).unwrap_or_default());
                    }
                }
            }
        }
    }

    crate::load_order::ini_parser::atomic_write(&instance_file, serde_json::to_string_pretty(&target_instance).unwrap_or_default())
        .map_err(|e| e.to_string())?;

    {
        // 3. PZModStudio_MasterLoadOrder.json
        let master_order_path = z_dir.join("PZModStudio_MasterLoadOrder.json");
        let master_json = serde_json::json!({
            "load_order": target_instance.load_order,
            "active_mod_ids": target_instance.active_mod_ids,
            "active_profile_id": target_instance.id,
            "active_profile_name": target_instance.name
        });
        let _ = crate::load_order::ini_parser::atomic_write(master_order_path, serde_json::to_string_pretty(&master_json).unwrap_or_default());
        apply_link(&z_dir, &target_instance);
    }

    Ok(())
}

#[tauri::command]
pub fn save_master_load_order(
    user_zomboid_dir: String,
    load_order: Vec<String>,
    active_mod_ids: Vec<String>,
) -> Result<(), String> {
    // Runs on every toggle/reorder: only the Zomboid folder in use, and no Workshop scans.
    let z_dir = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;

    // With a profile active, the Studio Link mod stays on whatever the list says.
    let dir = get_instances_dir(&user_zomboid_dir);
    let mut active_mod_ids = active_mod_ids;
    let link = crate::studio_link::LINK_MOD_ID.to_string();
    if find_active(&dir).is_some() && !active_mod_ids.contains(&link) {
        active_mod_ids.push(link);
    }

    // 1. The active profile first, so that when default.txt changes it already matches and the
    // watcher does not take the Studio's own write for one made in the game.
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(c) = fs::read_to_string(&path) {
                    if let Ok(mut inst) = serde_json::from_str::<AppInstance>(&c) {
                        if inst.is_active {
                            // `load_order` only holds what is installed right now. Anything the
                            // profile knew about and cannot see — a mod temporarily
                            // unsubscribed, or one the machine never had — is kept, otherwise
                            // the profile quietly forgets it and can never be taken elsewhere.
                            let installed: std::collections::HashSet<&String> = load_order.iter().collect();
                            let remembered: Vec<String> = inst
                                .load_order
                                .iter()
                                .filter(|id| !installed.contains(id))
                                .cloned()
                                .collect();
                            let still_wanted: Vec<String> = inst
                                .active_mod_ids
                                .iter()
                                .filter(|id| !installed.contains(id))
                                .cloned()
                                .collect();

                            inst.load_order = load_order.clone();
                            inst.load_order.extend(remembered);
                            inst.active_mod_ids = active_mod_ids.clone();
                            inst.active_mod_ids.extend(still_wanted);
                            // A mod switched on is one of the profile's mods from then on.
                            let mut members = inst.member_ids();
                            for id in &inst.active_mod_ids {
                                if !members.contains(id) {
                                    members.push(id.clone());
                                }
                            }
                            inst.members = Some(members);
                            let _ = crate::load_order::ini_parser::atomic_write(&path, serde_json::to_string_pretty(&inst).unwrap_or_default());
                            apply_link(&z_dir, &inst);
                        }
                    }
                }
            }
        }
    }

    // 2. mods/default.txt, the list the game reads
    crate::load_order::ini_parser::write_mod_list_ini(
        &z_dir.join("ModListData.ini").to_string_lossy(),
        &active_mod_ids,
    )?;

    // 3. Write PZModStudio_MasterLoadOrder.json
    let master_order_path = z_dir.join("PZModStudio_MasterLoadOrder.json");
    let master_json = serde_json::json!({
        "load_order": load_order,
        "active_mod_ids": active_mod_ids
    });
    let _ = crate::load_order::ini_parser::atomic_write(master_order_path, serde_json::to_string_pretty(&master_json).unwrap_or_default());

    Ok(())
}

#[tauri::command]
pub fn delete_instance(user_zomboid_dir: String, instance_id: String) -> Result<(), String> {
    let dir = get_instances_dir(&user_zomboid_dir);
    let instance_file = instance_file_path(&dir, &instance_id)?;
    if !instance_file.exists() {
        return Ok(());
    }
    let was_active = read_instance(&instance_file).is_some_and(|i| i.is_active);
    // Kept in the backups, not gone: a profile is a whole mod list put together by hand.
    let backups = dir.parent().map(|z| z.join("PZModStudio_Backups").join("profiles"));
    let kept = backups.as_ref().is_some_and(|b| {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        fs::create_dir_all(b).is_ok()
            && fs::rename(&instance_file, b.join(format!("{}.{}.json", instance_id, stamp))).is_ok()
    });
    if !kept {
        fs::remove_file(&instance_file).map_err(|e| e.to_string())?;
    }
    if was_active {
        // Without a profile the game's Mods screen and saves are no longer held to its mods.
        if let Some(z_dir) = dir.parent() {
            let _ = fs::remove_file(z_dir.join("Lua").join(crate::studio_link::LINK_FILE));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn update_instance(
    user_zomboid_dir: String,
    instance: AppInstance,
) -> Result<AppInstance, String> {
    let dir = get_instances_dir(&user_zomboid_dir);
    let instance_file = instance_file_path(&dir, &instance.id)?;
    if !instance_file.exists() {
        return Err("The instance to update does not exist.".to_string());
    }
    if let Some(b) = instance.target_build.as_deref() {
        if b.len() != 2 || !b.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("A target build is a major build like 41 or 42, not \"{}\".", b));
        }
    }

    // The screen's copy may be older than the game memory set since: kept unless it says another.
    let mut instance = instance;
    if instance.game_memory_mb.is_none() {
        instance.game_memory_mb = read_instance(&instance_file).and_then(|stored| stored.game_memory_mb);
    }
    let json = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(&instance_file, json).map_err(|e| e.to_string())?;

    // If updating active instance, also sync master load order
    if instance.is_active {
        let _ = save_master_load_order(user_zomboid_dir, instance.load_order.clone(), instance.active_mod_ids.clone());
    }

    Ok(instance)
}


/// Notes the build a profile was just played on: the one in Zomboid/version.txt now.
pub fn mark_instance_played(user_zomboid_dir: String, instance_id: String) -> Result<Option<String>, String> {
    let dir = get_instances_dir(&user_zomboid_dir);
    let instance_file = instance_file_path(&dir, &instance_id)?;
    let text = fs::read_to_string(&instance_file).map_err(|e| e.to_string())?;
    let mut instance: AppInstance = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let build = crate::load_order::mod_info::read_installed_game_version(&user_zomboid_dir)
        .map(crate::catalog::modinfo::format_game_version);
    instance.last_played_build = build.clone();
    let json = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(&instance_file, json).map_err(|e| e.to_string())?;
    Ok(build)
}

#[cfg(test)]
mod build_tests {
    use super::*;

    #[test]
    fn a_profile_file_without_the_new_field_still_reads_and_the_build_is_target_or_played() {
        let text = r#"{"id":"a","name":"A","created_at":"","is_active":false,"active_mod_ids":[],"load_order":[],"last_played_build":"42.21"}"#;
        let mut p: AppInstance = serde_json::from_str(text).unwrap();
        assert_eq!(p.target_build, None);
        assert_eq!(p.build_major().as_deref(), Some("42"));
        p.target_build = Some("41".into());
        assert_eq!(p.build_major().as_deref(), Some("41"), "set by hand wins over played");
        p.last_played_build = None;
        p.target_build = None;
        assert_eq!(p.build_major(), None);
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn an_old_profile_holds_the_mods_it_has_on_in_load_order() {
        let text = r#"{"id":"a","name":"A","created_at":"","is_active":false,"active_mod_ids":["C","A"],"load_order":["A","B","C"]}"#;
        let p: AppInstance = serde_json::from_str(text).unwrap();
        assert_eq!(p.member_ids(), ids(&["A", "C"]));
    }

    #[test]
    fn mods_the_game_asks_to_add_join_the_active_profile_switched_off() {
        let z = std::env::temp_dir().join(format!("pzms-profile-add-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        let dir = z.join("PZModStudio_Instances");
        fs::create_dir_all(&dir).unwrap();
        let active: AppInstance = serde_json::from_str(
            r#"{"id":"a","name":"A","created_at":"","is_active":true,"active_mod_ids":["A"],"load_order":["A"],"members":["A"]}"#,
        ).unwrap();
        let other: AppInstance = serde_json::from_str(
            r#"{"id":"b","name":"B","created_at":"","is_active":false,"active_mod_ids":["X"],"load_order":["X"],"members":["X","Y"]}"#,
        ).unwrap();
        write_instance(&dir.join("a.json"), &active).unwrap();
        write_instance(&dir.join("b.json"), &other).unwrap();

        let after = add_members_to_active(&z, &ids(&["N", "A"])).unwrap().expect("changed");
        assert_eq!(after.active_mod_ids, ids(&["A"]), "added switched off");
        assert_eq!(after.member_ids(), ids(&["A", "N"]));
        // Asking again changes nothing.
        assert!(add_members_to_active(&z, &ids(&["N"])).unwrap().is_none());
        // The Link file names the other profile's mods, so the game can tell the new ones.
        let link = fs::read_to_string(z.join("Lua").join(crate::studio_link::LINK_FILE)).unwrap();
        assert!(link.contains("member=N\n") && link.contains("other=X\n") && link.contains("other=Y\n"), "{}", link);
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn what_the_game_switches_on_comes_back_to_the_active_profile() {
        let z = std::env::temp_dir().join(format!("pzms-profile-game-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        let dir = z.join("PZModStudio_Instances");
        fs::create_dir_all(&dir).unwrap();
        let mut p: AppInstance = serde_json::from_str(
            r#"{"id":"a","name":"A","created_at":"","is_active":true,"active_mod_ids":["A"],"load_order":["A","B","C"],"members":["A","B"]}"#,
        ).unwrap();
        write_instance(&dir.join("a.json"), &p).unwrap();

        // The same list the profile has is not a change.
        assert!(take_mod_list_from_game(&z, &ids(&["A"])).unwrap().is_none());

        // The game switched C on before A, and B off was already off.
        let after = take_mod_list_from_game(&z, &ids(&["C", "A"])).unwrap().expect("changed");
        assert_eq!(after.active_mod_ids, ids(&["C", "A"]));
        assert_eq!(after.load_order, ids(&["C", "B", "A"]));
        assert_eq!(after.member_ids(), ids(&["C", "B", "A"]));
        p = read_instance(&dir.join("a.json")).unwrap();
        assert_eq!(p.active_mod_ids, ids(&["C", "A"]));
        let link = fs::read_to_string(z.join("Lua").join(crate::studio_link::LINK_FILE)).unwrap();
        assert_eq!(link, "profile=A\nprofileid=a\nmember=C\nmember=B\nmember=A\n");
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn a_servers_profile_takes_what_the_server_loaded() {
        let z = std::env::temp_dir().join(format!("pzms-serverprofile-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        fs::create_dir_all(z.join("mods")).unwrap();
        let user = z.to_string_lossy().to_string();
        let made = create_instance(user.clone(), "Online: X".into(), None, ids(&["A", "Mine"]), ids(&["A", "Mine"])).unwrap();
        let p = take_server_mods(&user, &made.id, &ids(&["A", "B", "C"]), &ids(&["D"]), &ids(&["Mine", "Never"])).unwrap().expect("changed");
        assert_eq!(p.active_mod_ids, ids(&["A", "B", "C", "Mine", crate::studio_link::LINK_MOD_ID]), "the server's mods on, in its order; one of the player's side that was on stays on");
        assert!(!p.member_ids().contains(&"Never".to_string()), "kept on only when it was on");
        let p = take_server_mods(&user, &made.id, &ids(&["A", "B", "C"]), &ids(&["D"]), &[]).unwrap().unwrap_or(p);
        assert!(p.member_ids().contains(&"Mine".to_string()), "a mod of its own stays in it");
        assert!(!p.active_mod_ids.contains(&"Mine".to_string()), "off: the server does not load it");
        assert_eq!(&p.load_order[..3], &ids(&["A", "B", "C"])[..]);
        assert!(p.member_ids().contains(&"D".to_string()) && !p.active_mod_ids.contains(&"D".to_string()), "the rest of the server's items: in it, off");
        assert!(take_server_mods(&user, &made.id, &ids(&["A", "B", "C"]), &ids(&["D"]), &[]).unwrap().is_none(), "nothing new, nothing written");
        let _ = fs::remove_dir_all(&z);
    }
}
