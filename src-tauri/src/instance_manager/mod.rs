pub mod forget_missing;

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
    /// An isolated profile ("Isolate this profile"; the name is from when it was saves only): its
    /// single-player saves, game options and mod settings are its own, put in place when it is
    /// switched on and put away when another is (profile_saves). The mods themselves are shared.
    /// Written only when true: a profile file from before this field reads and writes back the same.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own_saves: bool,
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

/// Mods a save uses that the active profile lacks, added to it switched on (the Link's "Add to
/// profile" on a save with mods outside the profile): members, load order and the active list,
/// and the game's list written as on a profile switch, so the profile now matches the save.
pub fn add_members_to_active_on(z_dir: &std::path::Path, ids: &[String]) -> Result<Option<AppInstance>, String> {
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
            changed = true;
        }
        if !inst.active_mod_ids.contains(id) {
            inst.active_mod_ids.push(id.clone());
            changed = true;
        }
    }
    if !changed {
        return Ok(None);
    }
    inst.members = Some(members);
    // The game's list first, as a profile switch does: if it cannot be written, the profile stays as it was.
    let _ = std::fs::create_dir_all(z_dir.join("mods"));
    crate::load_order::ini_parser::write_mod_list_ini(&z_dir.join("ModListData.ini").to_string_lossy(), &inst.active_mod_ids)?;
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

/// The active profile's id, name and the mods it has on, in its load order.
pub fn active_profile_on(z_dir: &std::path::Path) -> Option<(String, String, Vec<String>)> {
    find_active(&z_dir.join("PZModStudio_Instances")).map(|(_, i)| (i.id, i.name, i.active_mod_ids))
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
        own_saves: false,
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

    let z_dir = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;
    // 0. Its saves in place (own, or the shared ones): refused while the game runs, before anything.
    crate::profile_saves::switch_to(&z_dir, &crate::profile_saves::set_of(target_instance.own_saves, &target_instance.id))?;

    // 1. The game's list first: if it cannot be written, nothing has switched, and the profile
    // still marked active is the one whose mods are on.
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
                        // Only the files whose mark changes: the others stay byte for byte.
                        if inst.is_active != (inst.id == instance_id) {
                            inst.is_active = inst.id == instance_id;
                            let _ = crate::load_order::ini_parser::atomic_write(&path, serde_json::to_string_pretty(&inst).unwrap_or_default());
                        }
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
    forget: Option<Vec<String>>,
) -> Result<(), String> {
    // Not-installed mods the player took out of the list on purpose: the profile lets them go.
    let forget: std::collections::HashSet<String> = forget.unwrap_or_default().into_iter().collect();
    // Runs on every toggle/reorder: only the Zomboid folder in use, and no Workshop scans.
    let z_dir = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;

    // With a profile active, the Studio Link mod stays on whatever the list says.
    let dir = get_instances_dir(&user_zomboid_dir);
    // A Studio package (Z_PZModStudio_*) with no folder any more was deleted or renamed: the
    // profile does not keep it (no screen could show it, so no one could take it out), and an id
    // is in the list once.
    let gone_package = |id: &str| id.starts_with("Z_PZModStudio_") && !z_dir.join("mods").join(id).is_dir();
    let mut seen_ids = std::collections::HashSet::new();
    let mut active_mod_ids: Vec<String> = active_mod_ids.into_iter().filter(|id| !gone_package(id) && seen_ids.insert(id.clone())).collect();
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
                                .filter(|id| !installed.contains(id) && !forget.contains(*id) && !gone_package(id))
                                .cloned()
                                .collect();
                            let still_wanted: Vec<String> = inst
                                .active_mod_ids
                                .iter()
                                .filter(|id| !installed.contains(id) && !forget.contains(*id) && !gone_package(id) && !active_mod_ids.contains(id))
                                .cloned()
                                .collect();

                            inst.load_order = load_order.clone();
                            inst.load_order.extend(remembered);
                            inst.active_mod_ids = active_mod_ids.clone();
                            inst.active_mod_ids.extend(still_wanted);
                            let mut once = std::collections::HashSet::new();
                            inst.active_mod_ids.retain(|id| once.insert(id.clone()));
                            let mut once = std::collections::HashSet::new();
                            inst.load_order.retain(|id| once.insert(id.clone()));
                            // A mod switched on is one of the profile's mods from then on.
                            let mut members = inst.member_ids();
                            members.retain(|id| !forget.contains(id) && !gone_package(id));
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
    let stored = read_instance(&instance_file);
    let was_active = stored.as_ref().is_some_and(|i| i.is_active);
    // Isolated: its saves become shared ones first and its settings go to the backups: deleting a
    // profile never loses a world or a setting.
    if stored.as_ref().is_some_and(|i| i.own_saves) {
        if let Some(z) = dir.parent() {
            crate::profile_saves::give_back(z, &instance_id)?;
        }
    }
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
    let stored = read_instance(&instance_file);
    if instance.game_memory_mb.is_none() {
        instance.game_memory_mb = stored.as_ref().and_then(|s| s.game_memory_mb);
    }
    // Isolation changes only with set_own_saves (worlds and settings are moved then).
    instance.own_saves = stored.as_ref().is_some_and(|s| s.own_saves);
    let json = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::atomic_write(&instance_file, json).map_err(|e| e.to_string())?;

    // If updating active instance, also sync master load order
    if instance.is_active {
        let _ = save_master_load_order(user_zomboid_dir, instance.load_order.clone(), instance.active_mod_ids.clone(), None);
    }

    Ok(instance)
}


/// A profile is isolated (`bring`: shared worlds that go with it; it starts with a copy of the
/// options and mod settings in place) or stops being so (its worlds become shared, its settings
/// are kept in the backups).
#[tauri::command]
pub fn set_own_saves(user_zomboid_dir: String, instance_id: String, own: bool, bring: Vec<String>) -> Result<AppInstance, String> {
    let dir = get_instances_dir(&user_zomboid_dir);
    let instance_file = instance_file_path(&dir, &instance_id)?;
    let mut instance = read_instance(&instance_file).ok_or_else(|| "The profile does not exist.".to_string())?;
    let z = dir.parent().ok_or_else(|| "Zomboid folder not found".to_string())?.to_path_buf();
    if own == instance.own_saves {
        return Ok(instance);
    }
    if own {
        crate::profile_saves::take_own(&z, &instance.id, instance.is_active, &bring)?;
    } else {
        crate::profile_saves::give_back(&z, &instance.id)?;
    }
    instance.own_saves = own;
    crate::load_order::ini_parser::atomic_write(&instance_file, serde_json::to_string_pretty(&instance).unwrap_or_default()).map_err(|e| e.to_string())?;
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
    fn a_saves_mods_added_to_the_profile_come_on_so_it_matches_the_save() {
        let z = std::env::temp_dir().join(format!("pzms-profile-addon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        let dir = z.join("PZModStudio_Instances");
        fs::create_dir_all(&dir).unwrap();
        let active: AppInstance = serde_json::from_str(
            r#"{"id":"a","name":"A","created_at":"","is_active":true,"active_mod_ids":["A"],"load_order":["A","B"],"members":["A","B"]}"#,
        ).unwrap();
        write_instance(&dir.join("a.json"), &active).unwrap();

        // N is new; B was in the profile but off: both come on.
        let after = add_members_to_active_on(&z, &ids(&["N", "B"])).unwrap().expect("changed");
        assert_eq!(after.active_mod_ids, ids(&["A", "N", "B"]));
        assert_eq!(after.member_ids(), ids(&["A", "B", "N"]));
        // The game's list holds them too.
        let list = fs::read_to_string(z.join("mods").join("default.txt")).unwrap_or_default() + &fs::read_to_string(z.join("ModListData.ini")).unwrap_or_default();
        assert!(list.contains('N') && list.contains('B'), "{}", list);
        assert!(add_members_to_active_on(&z, &ids(&["N"])).unwrap().is_none(), "asking again changes nothing");
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn a_studio_package_with_no_folder_leaves_the_profile_and_ids_are_kept_once() {
        let z = std::env::temp_dir().join(format!("pzms-profile-ghost-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        let dir = z.join("PZModStudio_Instances");
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(z.join("mods").join("Z_PZModStudio_Real")).unwrap();
        // As seen on the KnoxLives test profile: the generic package id, gone from disk, twice.
        let p: AppInstance = serde_json::from_str(
            r#"{"id":"a","name":"A","created_at":"","is_active":true,"active_mod_ids":["A","Z_PZModStudio_MergedPatch","Gone","Z_PZModStudio_MergedPatch"],"load_order":["A","Z_PZModStudio_MergedPatch","Gone"],"members":["A","Z_PZModStudio_MergedPatch","Gone"]}"#,
        ).unwrap();
        write_instance(&dir.join("a.json"), &p).unwrap();
        let user = z.to_string_lossy().to_string();
        save_master_load_order(user, ids(&["A", "Z_PZModStudio_Real"]), ids(&["A", "Z_PZModStudio_MergedPatch", "Z_PZModStudio_Real", "A"]), None).unwrap();
        let after = read_instance(&dir.join("a.json")).unwrap();
        // The gone package is out; a mod only not installed ("Gone") is still remembered; once each.
        assert_eq!(after.active_mod_ids, ids(&["A", "Z_PZModStudio_Real", crate::studio_link::LINK_MOD_ID, "Gone"]));
        assert!(!after.load_order.iter().any(|id| id == "Z_PZModStudio_MergedPatch"), "{:?}", after.load_order);
        assert!(!after.member_ids().iter().any(|id| id == "Z_PZModStudio_MergedPatch"));
        let list = fs::read_to_string(z.join("mods").join("default.txt")).unwrap_or_default();
        assert!(!list.contains("Z_PZModStudio_MergedPatch"), "{}", list);
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
    /// The whole flow of a profile with saves of its own, through the same calls the app makes:
    /// give it its own saves (taking one shared world), switch profiles both ways, take them back,
    /// delete it. No world is ever lost or duplicated.
    #[test]
    fn a_profile_with_saves_of_its_own_from_start_to_deletion() {
        let z = std::env::temp_dir().join(format!("pzms-own-saves-flow-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        fs::create_dir_all(z.join("mods")).unwrap();
        let user = z.to_string_lossy().to_string();
        let world = |name: &str| {
            let d = z.join("Saves").join("Sandbox").join(name);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("map_ver.bin"), name).unwrap();
        };
        world("Shared One");
        world("Goes With Mine");
        fs::create_dir_all(z.join("Saves/Multiplayer/servertest")).unwrap();
        // The game's options and a mod's settings: each isolated profile has its own.
        fs::create_dir_all(z.join("Lua")).unwrap();
        fs::write(z.join("options.ini"), "shared").unwrap();
        fs::write(z.join("Lua/ModOptions.ini"), "shared").unwrap();
        let setting = |rel: &str| fs::read_to_string(z.join(rel)).unwrap_or_default();
        let in_place = || -> Vec<String> {
            let mut v: Vec<String> = fs::read_dir(z.join("Saves/Sandbox")).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
            v.sort();
            v
        };
        let shared = create_instance(user.clone(), "Shared".into(), None, vec![], vec![]).unwrap();
        let mine = create_instance(user.clone(), "Mine".into(), None, vec![], vec![]).unwrap();
        activate_instance(user.clone(), shared.id.clone()).unwrap();

        // Mine (not active) gets its own saves, taking one shared world.
        let mine = set_own_saves(user.clone(), mine.id.clone(), true, vec!["Sandbox/Goes With Mine".into()]).unwrap();
        assert!(mine.own_saves);
        assert_eq!(in_place(), vec!["Shared One".to_string()]);

        // Switching to it: only its world; the server's never moves.
        activate_instance(user.clone(), mine.id.clone()).unwrap();
        assert_eq!(in_place(), vec!["Goes With Mine".to_string()]);
        assert!(z.join("Saves/Multiplayer/servertest").is_dir());
        assert_eq!(setting("options.ini"), "shared", "it starts with a copy");
        // A new world made while it is on stays its own, and so do its options.
        world("Made In Mine");
        fs::write(z.join("options.ini"), "mine").unwrap();
        fs::write(z.join("Lua/ModOptions.ini"), "mine").unwrap();

        // Back to the shared profile, and to Mine again.
        activate_instance(user.clone(), shared.id.clone()).unwrap();
        assert_eq!(in_place(), vec!["Shared One".to_string()]);
        assert_eq!((setting("options.ini"), setting("Lua/ModOptions.ini")), ("shared".to_string(), "shared".to_string()));
        activate_instance(user.clone(), mine.id.clone()).unwrap();
        assert_eq!(in_place(), vec!["Goes With Mine".to_string(), "Made In Mine".to_string()]);
        assert_eq!((setting("options.ini"), setting("Lua/ModOptions.ini")), ("mine".to_string(), "mine".to_string()));
        assert!(z.join("Lua").join(crate::studio_link::LINK_FILE).is_file(), "the Link's files never move");

        // An edit from the screen cannot flip it without moving the worlds.
        let mut edited = mine.clone();
        edited.own_saves = false;
        edited.is_active = true;
        assert!(update_instance(user.clone(), edited).unwrap().own_saves, "kept: only set_own_saves changes it");

        // Deleting it while it is on: its worlds join the shared ones, nothing lost.
        delete_instance(user.clone(), mine.id.clone()).unwrap();
        assert_eq!(in_place(), vec!["Goes With Mine".to_string(), "Made In Mine".to_string(), "Shared One".to_string()]);
        assert_eq!(crate::profile_saves::state(&z).current, crate::profile_saves::SHARED);
        for w in ["Goes With Mine", "Made In Mine", "Shared One"] {
            assert_eq!(fs::read_to_string(z.join("Saves/Sandbox").join(w).join("map_ver.bin")).unwrap(), w, "{} intact", w);
        }
        // The shared options are back in place; its own are kept in the backups, not laid over them.
        assert_eq!(setting("options.ini"), "shared");
        let kept = fs::read_dir(z.join("PZModStudio_Backups/ProfileSettings").join(&mine.id)).unwrap().flatten().next().unwrap().path();
        assert_eq!(fs::read_to_string(kept.join("options.ini")).unwrap(), "mine");
        let _ = fs::remove_dir_all(&z);
    }

}

/// Isolated profiles driven through the app's own calls on a real Zomboid folder: a copy of this
/// PC's (`PZMS_ISO_COPY`, the long run) or the folder itself (`PZMS_ISO_REAL`, a short run). Never
/// run by `cargo test`: `cargo test --lib isolation_on_real_data -- --ignored --test-threads=1`.
/// The profile named in `PZMS_ISO_KEEP` must be active, not isolated, and is never isolated or
/// edited: it is checked byte for byte at the end, and is the active profile again.
#[cfg(test)]
mod isolation_on_real_data {
    use super::*;
    use crate::profile_saves as ps;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    fn fingerprint(p: &Path) -> (u64, u64) {
        use std::hash::{Hash, Hasher};
        let bytes = fs::read(p).unwrap_or_default();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut h);
        (bytes.len() as u64, h.finish())
    }

    fn is_link(p: &Path) -> bool {
        fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false)
    }

    /// Every file under each of `rels` (the profile's settings in place), with its fingerprint.
    fn snapshot(z: &Path, rels: &[String]) -> BTreeMap<String, (u64, u64)> {
        fn walk(base: &Path, p: &Path, out: &mut BTreeMap<String, (u64, u64)>) {
            if is_link(p) {
                return;
            }
            if p.is_dir() {
                for e in fs::read_dir(p).into_iter().flatten().flatten() {
                    walk(base, &e.path(), out);
                }
            } else if p.is_file() {
                out.insert(p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"), fingerprint(p));
            }
        }
        let mut out = BTreeMap::new();
        for rel in rels {
            walk(z, &z.join(rel), &mut out);
        }
        out
    }

    /// The single-player worlds in place (mode/world).
    fn worlds(z: &Path) -> Vec<String> {
        let mut out = Vec::new();
        for m in fs::read_dir(z.join("Saves")).into_iter().flatten().flatten() {
            let mode = m.file_name().to_string_lossy().to_string();
            for w in fs::read_dir(m.path()).into_iter().flatten().flatten().filter(|w| w.path().is_dir()) {
                out.push(format!("{}/{}", mode, w.file_name().to_string_lossy()));
            }
        }
        out.sort();
        out
    }

    fn files_under(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for e in fs::read_dir(dir).into_iter().flatten().flatten() {
            if e.path().is_dir() && !is_link(&e.path()) {
                out.extend(files_under(&e.path()));
            } else {
                out.push(e.path());
            }
        }
        out
    }

    fn by_name(user: &str, name: &str) -> AppInstance {
        list_instances(user.to_string()).unwrap().into_iter().find(|i| i.name == name).unwrap_or_else(|| panic!("no profile named {}", name))
    }

    fn run(z: &Path, long: bool) {
        assert!(crate::sandbox::find_running_pz_pid().is_none(), "the game runs");
        assert!(crate::server_manager::mp_test::running_servers().is_empty(), "a server runs");
        let user = z.to_string_lossy().to_string();
        let keep_name = std::env::var("PZMS_ISO_KEEP").expect("PZMS_ISO_KEEP: the profile that must stay as it is");
        let keep = by_name(&user, &keep_name);
        let keep_file = get_instances_dir(&user).join(format!("{}.json", keep.id));
        let keep_bytes = fs::read(&keep_file).unwrap();
        assert!(keep.is_active && !keep.own_saves, "{} must be active and not isolated", keep_name);
        // Activating it again writes it back: only when that writes the same bytes is it touched.
        let mut again = keep.clone();
        again.with_link();
        assert_eq!(serde_json::to_string_pretty(&again).unwrap().as_bytes(), &keep_bytes[..], "{}: re-activating it would not write the same file; stopping before anything moves", keep_name);
        let s = ps::state(z);
        assert_eq!((s.current.as_str(), s.moving.is_none()), (ps::SHARED, true));

        // What must be the same at the end: the settings in place and the worlds.
        let rels = ps::settings_in_place(z);
        let settings_before = snapshot(z, &rels);
        let mut worlds_before = worlds(z);
        eprintln!("{} settings entries in place ({} files), worlds: {:?}", rels.len(), settings_before.len(), worlds_before);
        let stamp = chrono::Local::now().format("%H%M%S").to_string();
        let dummy = [format!("Sandbox/PZMS_IsoTest_A_{}", stamp), format!("Sandbox/PZMS_IsoTest_B_{}", stamp)];
        if long {
            for d in &dummy {
                fs::create_dir_all(z.join("Saves").join(d)).unwrap();
                fs::write(z.join("Saves").join(d).join("map_ver.bin"), d.as_bytes()).unwrap();
                fs::write(z.join("Saves").join(d).join("map_t.bin"), vec![7u8; 4096]).unwrap();
            }
        }
        let bring: Vec<String> = if long { dummy.to_vec() } else { vec![] };

        let test = create_instance(user.clone(), format!("PZMS Isolation Test {}", stamp), None, keep.active_mod_ids.clone(), keep.load_order.clone()).unwrap();
        let test = set_own_saves(user.clone(), test.id.clone(), true, bring.clone()).unwrap();
        assert!(test.own_saves);
        let left: Vec<String> = ps::left_shared(z).into_iter().map(|l| l.rel).collect();
        eprintln!("left shared (too big to copy): {:?}", left);
        assert_eq!(worlds(z), worlds_before, "the brought worlds went with it");
        assert_eq!(snapshot(z, &rels), settings_before, "isolating copies; the settings in place stay");

        let rounds = if long { 3 } else { 1 };
        for round in 0..rounds {
            activate_instance(user.clone(), test.id.clone()).unwrap();
            let in_place = worlds(z);
            let mut want: Vec<String> = in_place.iter().filter(|w| w.starts_with("Multiplayer/")).cloned().collect();
            want.extend(bring.iter().cloned());
            want.sort();
            assert_eq!(in_place, want, "round {}: only its worlds (and the multiplayer ones)", round);
            let now = snapshot(z, &rels);
            if round == 0 {
                assert_eq!(now, settings_before, "it starts with a copy");
            }
            if long && round == 0 {
                // A setting changed inside the isolated profile, and a mod's new file.
                let mut opts = fs::read_to_string(z.join("options.ini")).unwrap();
                opts.push_str("pzmsIsoTest=1\r\n");
                fs::write(z.join("options.ini"), opts).unwrap();
                fs::write(z.join("Lua/PZMS_IsoTest_option.ini"), "isolated=1\r\n").unwrap();
            }
            if long && round > 0 {
                assert!(fs::read_to_string(z.join("options.ini")).unwrap().contains("pzmsIsoTest=1"), "round {}: its own options came back", round);
                assert!(z.join("Lua/PZMS_IsoTest_option.ini").is_file());
            }
            activate_instance(user.clone(), keep.id.clone()).unwrap();
            assert_eq!(worlds(z), worlds_before, "round {}: the shared worlds back, its own away", round);
            assert_eq!(snapshot(z, &rels), settings_before, "round {}: the shared settings back exactly", round);
            assert!(!z.join("Lua/PZMS_IsoTest_option.ini").exists());
            assert_eq!(fs::read(&keep_file).unwrap(), keep_bytes, "round {}: {} unchanged", round, keep_name);
        }

        if long {
            // Isolation turned off while it is on: its worlds become shared, its settings kept aside.
            activate_instance(user.clone(), test.id.clone()).unwrap();
            let off = set_own_saves(user.clone(), test.id.clone(), false, vec![]).unwrap();
            assert!(!off.own_saves);
            assert_eq!(snapshot(z, &rels), settings_before, "the shared settings in place, not the test's");
            worlds_before.extend(dummy.iter().cloned());
            worlds_before.sort();
            assert_eq!(worlds(z), worlds_before, "its worlds joined the shared ones");
            let kept = fs::read_dir(z.join("PZModStudio_Backups/ProfileSettings").join(&test.id)).unwrap().flatten().next().unwrap().path();
            assert!(fs::read_to_string(kept.join("options.ini")).unwrap().contains("pzmsIsoTest=1"));
            assert!(kept.join("Lua/PZMS_IsoTest_option.ini").is_file());
        }
        delete_instance(user.clone(), test.id.clone()).unwrap();
        activate_instance(user.clone(), keep.id.clone()).unwrap();
        if long {
            for d in &dummy {
                fs::remove_dir_all(z.join("Saves").join(d)).unwrap();
            }
            worlds_before.retain(|w| !dummy.contains(w));
        }

        // At the end: as at the start.
        assert_eq!(ps::state(z), ps::State { current: ps::SHARED.into(), moving: None });
        assert_eq!(worlds(z), worlds_before);
        assert_eq!(snapshot(z, &rels), settings_before);
        assert_eq!(ps::settings_in_place(z).into_iter().filter(|r| !rels.contains(r)).collect::<Vec<_>>(), Vec::<String>::new(), "nothing new in place");
        assert_eq!(fs::read(&keep_file).unwrap(), keep_bytes, "{} unchanged", keep_name);
        assert!(by_name(&user, &keep_name).is_active);
        let left_in_store = files_under(&z.join("PZModStudio_ProfileSaves"));
        assert!(left_in_store.is_empty(), "nothing left put away: {:?}", left_in_store);
        if !long {
            let kept = z.join("PZModStudio_Backups/ProfileSettings").join(&test.id);
            eprintln!("the test profile's settings kept in {} ({} files)", kept.display(), files_under(&kept).len());
        }
    }

    #[test]
    #[ignore]
    fn on_a_copy_of_this_pcs_zomboid() {
        let z = PathBuf::from(std::env::var("PZMS_ISO_COPY").expect("PZMS_ISO_COPY: a copy of the Zomboid folder"));
        run(&z, true);
    }

    #[test]
    #[ignore]
    fn on_this_pcs_zomboid() {
        let z = PathBuf::from(std::env::var("PZMS_ISO_REAL").expect("PZMS_ISO_REAL: the Zomboid folder"));
        run(&z, false);
    }

    /// A process named java.exe whose command line runs the game server (`PZMS_FAKE_JAVA`: a
    /// harmless program copied as java.exe, e.g. cmd.exe) is seen as a server.
    #[test]
    #[ignore]
    fn a_running_server_is_seen() {
        let exe = std::env::var("PZMS_FAKE_JAVA").expect("PZMS_FAKE_JAVA");
        let mut child = std::process::Command::new(&exe).args(["/c", "ping -n 15 127.0.0.1 >nul & rem zombie.network.GameServer -servername fakeiso"]).spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(800));
        let seen = crate::server_manager::mp_test::running_servers();
        let _ = child.kill();
        // (Its -servername is inside cmd's quoted command: the name is read in the unit tests.)
        assert!(seen.iter().any(|(pid, _)| *pid == child.id()), "{:?}", seen);
    }
}
