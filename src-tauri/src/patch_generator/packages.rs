//! The packages themselves: listing, creating, renaming, switching in the mod list, deleting.

use super::*;

pub fn get_active_mod_ids_from_disk(user_zomboid_dir: &str, mod_list_ini_path: &str) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();

    // 1. Check PZModStudio_MasterLoadOrder.json in user_dirs
    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    for u_dir in &user_dirs {
        let master_json_path = u_dir.join("PZModStudio_MasterLoadOrder.json");
        if master_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&master_json_path) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(arr) = parsed.get("active_mod_ids").and_then(|v| v.as_array()) {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                set.insert(s.to_string());
                            }
                        }
                        if !set.is_empty() {
                            return set;
                        }
                    }
                }
            }
        }

        // 2. Check active profile in PZModStudio_Profiles
        let profiles_dir = u_dir.join("PZModStudio_Profiles");
        if profiles_dir.exists() {
            if let Ok(entries) = fs::read_dir(&profiles_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(content) = fs::read_to_string(&p) {
                            if let Ok(inst) = serde_json::from_str::<crate::instance_manager::AppInstance>(&content) {
                                if inst.is_active {
                                    for id in inst.active_mod_ids {
                                        set.insert(id);
                                    }
                                    if !set.is_empty() {
                                        return set;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Check mod_list_ini_path / ModListData.ini
    if !mod_list_ini_path.is_empty() {
        if let Ok(data) = read_mod_list_ini(mod_list_ini_path) {
            for id in data.active_mods {
                set.insert(id);
            }
            if !set.is_empty() {
                return set;
            }
        }
    }

    for u_dir in &user_dirs {
        for candidate_ini in &[u_dir.join("ModListData.ini"), u_dir.join("mods").join("ModListData.ini")] {
            if candidate_ini.exists() {
                if let Ok(data) = read_mod_list_ini(&candidate_ini.to_string_lossy()) {
                    for id in data.active_mods {
                        set.insert(id);
                    }
                    if !set.is_empty() {
                        return set;
                    }
                }
            }
        }

        // 4. Check mods.txt
        let mods_txt = u_dir.join("mods.txt");
        if mods_txt.exists() {
            if let Ok(content) = fs::read_to_string(&mods_txt) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        set.insert(trimmed.to_string());
                    }
                }
                if !set.is_empty() {
                    return set;
                }
            }
        }
    }

    set
}

pub fn list_merged_packages(user_zomboid_dir: &str, _mod_list_ini_path: &str) -> Vec<MergedPackageInfo> {
    let mut packages = Vec::new();
    let mut seen_folders = std::collections::HashSet::new();

    let mut candidate_dirs = Vec::new();
    let clean_user = user_zomboid_dir.trim();
    if !clean_user.is_empty() {
        let p = PathBuf::from(clean_user);
        if p.exists() {
            candidate_dirs.push(p.clone());
            candidate_dirs.push(p.join("mods"));
        }
    }
    for user_dir in crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir) {
        if user_dir.exists() {
            candidate_dirs.push(user_dir.clone());
        }
        let m_dir = user_dir.join("mods");
        if m_dir.exists() {
            candidate_dirs.push(m_dir);
        }
    }

    if let Some(home) = dirs_next::home_dir() {
        let h1 = home.join("Zomboid").join("mods");
        if h1.exists() { candidate_dirs.push(h1); }
        let h2 = home.join("Documents").join("Zomboid").join("mods");
        if h2.exists() { candidate_dirs.push(h2); }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let p1 = PathBuf::from(profile).join("Zomboid").join("mods");
        if p1.exists() { candidate_dirs.push(p1); }
    }

    let detected_paths = crate::vfs::auto_detect_paths();
    if !detected_paths.workshop_dir.is_empty() {
        let ws_mods = Path::new(&detected_paths.workshop_dir).join("mods");
        if ws_mods.exists() {
            candidate_dirs.push(ws_mods);
        }
    }

    for mods_dir in &candidate_dirs {
        if let Ok(entries) = fs::read_dir(mods_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if folder_name.starts_with("Z_PZModStudio_") && !folder_name.contains("Carrier") {
                        if !seen_folders.insert(folder_name.clone()) {
                            continue;
                        }

                        let is_companion_bridge = folder_name == "Z_PZModStudio_Bridge";

                        let mut parsed_desc = None;
                        let display_name = if is_companion_bridge {
                            parsed_desc = Some("Official companion mod for live game control, command execution, and AI agent communication.".to_string());
                            "PZ Mod Studio Live Bridge (Companion Mod)".to_string()
                        } else if let Ok(info_str) = fs::read_to_string(path.join("mod.info")) {
                            let mut parsed_name = None;
                            for line in info_str.lines() {
                                let trimmed = line.trim();
                                if trimmed.starts_with("name=") {
                                    let val = trimmed[5..].trim().to_string();
                                    if !val.is_empty() {
                                        let clean = val
                                            .replace("PZ Mod Studio Patch: ", "")
                                            .replace("PZ Mod Studio Patch:", "")
                                            .replace("Z_PZModStudio_", "");
                                        parsed_name = Some(clean.trim().replace('_', " "));
                                    }
                                } else if trimmed.starts_with("description=") {
                                    let val = trimmed[12..].trim().to_string();
                                    if !val.is_empty() {
                                        parsed_desc = Some(val);
                                    }
                                }
                            }
                            parsed_name.unwrap_or_else(|| {
                                let clean = folder_name["Z_PZModStudio_".len()..].to_string().replace('_', " ");
                                clean
                            })
                        } else {
                            let clean = folder_name["Z_PZModStudio_".len()..].to_string().replace('_', " ");
                            clean
                        };

                        let meta_path = path.join("patch_metadata.json");
                        
                        let built_for_profile = fs::read_to_string(&meta_path)
                            .ok()
                            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                            .and_then(|v| v.get("built_for_profile_name").and_then(|n| n.as_str()).map(str::to_string));
                        let (is_packaged, created_at, packaged_mods, merged_files, meta_visible, meta_desc) = if meta_path.exists() {
                            if let Ok(content) = fs::read_to_string(&meta_path) {
                                if let Ok(meta_json) = serde_json::from_str::<serde_json::Value>(&content) {
                                    let is_pkg = meta_json["is_packaged"].as_bool().unwrap_or(false);
                                    let created = meta_json["created_at"].as_str().map(|s| s.to_string());
                                    let mods = meta_json["packaged_mod_ids"]
                                        .as_array()
                                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                        .unwrap_or_default();
                                    let files = meta_json["merged_file_paths"]
                                        .as_array()
                                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                        .unwrap_or_default();
                                    let vis = meta_json.get("is_visible_in_modlist").and_then(|v| v.as_bool());
                                    let desc = meta_json.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                                    (is_pkg, created, mods, files, vis, desc)
                                } else {
                                    (false, None, Vec::new(), Vec::new(), None, None)
                                }
                            } else {
                                (false, None, Vec::new(), Vec::new(), None, None)
                            }
                        } else {
                            (false, None, Vec::new(), Vec::new(), None, None)
                        };

                        let description = parsed_desc.or(meta_desc);
                        let is_active_in_modlist = meta_visible.unwrap_or(true);

                        packages.push(MergedPackageInfo {
                            folder_name: folder_name.clone(),
                            display_name,
                            mod_id: folder_name,
                            description,
                            is_packaged,
                            is_active_in_modlist,
                            is_companion_bridge,
                            is_studio_link: false,
                            created_at,
                            packaged_mods,
                            merged_files,
                            version_check: None,
                            built_for_profile,
                        });
                    }
                }
            }
        }
    }

    // The Studio Link mod, shown like the companion: the Studio installs and updates it.
    if let Some(dir) = candidate_dirs.iter().map(|d| d.join(crate::studio_link::LINK_MOD_ID)).find(|d| d.is_dir()) {
        if seen_folders.insert(crate::studio_link::LINK_MOD_ID.to_string()) {
            let info = fs::read_to_string(dir.join("42").join("mod.info")).or_else(|_| fs::read_to_string(dir.join("mod.info"))).unwrap_or_default();
            let version = info.lines().find_map(|l| l.trim().strip_prefix("modversion=").map(|v| v.trim().to_string())).unwrap_or_default();
            packages.push(MergedPackageInfo {
                folder_name: crate::studio_link::LINK_MOD_ID.to_string(),
                display_name: if version.is_empty() { "PZ Mod Studio Link".to_string() } else { format!("PZ Mod Studio Link {}", version) },
                mod_id: crate::studio_link::LINK_MOD_ID.to_string(),
                description: Some("Official companion mod that shows the active profile in the game's Mods screen, sends what you change there back to the Studio, and guards saves from loading with the wrong mods.".to_string()),
                is_packaged: true,
                is_active_in_modlist: true,
                is_companion_bridge: true,
                is_studio_link: true,
                created_at: None,
                packaged_mods: Vec::new(),
                merged_files: Vec::new(),
                version_check: None,
                built_for_profile: None,
            });
        }
    }

    packages.sort_by(|a, b| {
        if a.is_companion_bridge != b.is_companion_bridge {
            a.is_companion_bridge.cmp(&b.is_companion_bridge)
        } else {
            a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase())
        }
    });

    packages
}

pub fn create_merged_package(user_zomboid_dir: &str, mod_list_ini_path: &str, name: &str, description: Option<&str>) -> Result<MergedPackageInfo, String> {
    if name.trim().is_empty() {
        return Err("Package name cannot be empty.".to_string());
    }
    let folder_name = to_package_folder(name)?;
    if super::is_system_folder(&folder_name) {
        return Err(format!("'{}' is reserved for the companion mod.", BRIDGE_FOLDER));
    }
    let clean_sub = folder_name[PACKAGE_PREFIX.len()..].to_string();
    let display_name = clean_sub.replace('_', " ");
    let in_game_title = format!("PZ Mod Studio Patch: {}", display_name);
    let final_desc = description.unwrap_or("Compatibility merge package synthesized by PZ Mod Studio.").trim();

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    if user_dirs.is_empty() {
        return Err("Invalid Zomboid directory.".to_string());
    }
    let pkg_dir = user_dirs[0].join("mods").join(&folder_name);
    // Making it again would reset its record and leave its published files untracked.
    if pkg_dir.exists() {
        return Err(format!("A package named '{}' already exists.", folder_name));
    }
    fs::create_dir_all(&pkg_dir).map_err(|e| e.to_string())?;

    let mod_info_content = format!(
        "name={}\r\nid={}\r\ndescription={}\r\nposter=poster.png\r\nicon=icon.png\r\nmodversion=1.0.0\r\npzversion=41,42\r\nversionMin=41.00\r\nauthor=PZ Mod Studio\r\n",
        in_game_title, folder_name, final_desc
    );
    let _ = fs::write(pkg_dir.join("mod.info"), &mod_info_content);
    let png_256 = get_preview_png_bytes();
    let _ = fs::write(pkg_dir.join("poster.png"), &png_256);
    let _ = fs::write(pkg_dir.join("icon.png"), &png_256);

    let meta = serde_json::json!({
        "is_packaged": false,
        "is_visible_in_modlist": true,
        "description": final_desc,
        "created_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs().to_string()).unwrap_or_default(),
        "packaged_mod_ids": [],
        "merged_file_paths": []
    });
    if let Ok(meta_json) = serde_json::to_string_pretty(&meta) {
        let _ = fs::write(pkg_dir.join("patch_metadata.json"), &meta_json);
    }

    if !mod_list_ini_path.is_empty() {
        if let Ok(mut mod_list) = read_mod_list_ini(mod_list_ini_path) {
            mod_list.active_mods.retain(|id| id != &folder_name);
            let _ = write_mod_list_ini(mod_list_ini_path, &mod_list.active_mods);
        }
    }

    Ok(MergedPackageInfo {
        folder_name: folder_name.clone(),
        display_name,
        mod_id: folder_name,
        description: Some(final_desc.to_string()),
        is_packaged: false,
        is_active_in_modlist: true,
        is_companion_bridge: false,
        is_studio_link: false,
        created_at: None,
        packaged_mods: Vec::new(),
        merged_files: Vec::new(),
        version_check: None,
        built_for_profile: None,
    })
}

/// Two paths naming the same folder (Windows file names ignore case, so a rename that only
/// changes case is not a clash).
fn same_folder(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

pub fn rename_merged_package(
    user_zomboid_dir: &str,
    mod_list_ini_path: &str,
    old_folder: &str,
    new_name: &str,
    new_description: Option<&str>,
) -> Result<MergedPackageInfo, String> {
    let old_candidates = package_folder_candidates(old_folder)?;

    if new_name.trim().is_empty() {
        return Err("The new name cannot be empty.".to_string());
    }
    let new_folder = to_package_folder(new_name)?;
    if super::is_system_folder(&new_folder) {
        return Err(format!("'{}' is reserved for the companion mod.", BRIDGE_FOLDER));
    }
    let clean_new_sub = new_folder[PACKAGE_PREFIX.len()..].to_string();
    let display_name = clean_new_sub.replace('_', " ");
    let in_game_title = format!("PZ Mod Studio Patch: {}", display_name);
    let final_desc = new_description.unwrap_or("Compatibility merge package synthesized by PZ Mod Studio.").trim();

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    if user_dirs.is_empty() {
        return Err("Invalid Zomboid directory.".to_string());
    }

    let mod_info_content = format!(
        "name={}\r\nid={}\r\ndescription={}\r\nposter=poster.png\r\nicon=icon.png\r\nmodversion=1.0.0\r\npzversion=41,42\r\nversionMin=41.00\r\nauthor=PZ Mod Studio\r\n",
        in_game_title, new_folder, final_desc
    );

    for dir in &user_dirs {
        for old_target in &old_candidates {
            let old_p = dir.join("mods").join(old_target);
            let new_p = dir.join("mods").join(&new_folder);
            if old_p.exists() {
                if new_p.exists() && !same_folder(&old_p, &new_p) {
                    return Err(format!("A package named '{}' already exists.", new_folder));
                }
                fs::rename(&old_p, &new_p).map_err(|e| format!("Cannot rename the package: {}", e))?;
                let _ = fs::write(new_p.join("mod.info"), &mod_info_content);
                // B42 reads 42/mod.info: every copy has to carry the new id, or the game no
                // longer finds the package under the name now in default.txt.
                for copy in ["42/mod.info", "media/mod.info", "42/media/mod.info", "common/mod.info"] {
                    let path = new_p.join(copy);
                    if let Ok(text) = fs::read_to_string(&path) {
                        let renamed: Vec<String> = text
                            .lines()
                            .map(|l| if l.trim_start().starts_with("id=") { format!("id={}", new_folder) } else { l.to_string() })
                            .collect();
                        let _ = fs::write(&path, renamed.join("\r\n") + "\r\n");
                    }
                }
                let meta_path = new_p.join("patch_metadata.json");
                if meta_path.exists() {
                    if let Ok(meta_str) = fs::read_to_string(&meta_path) {
                        if let Ok(mut meta_json) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                            meta_json["description"] = serde_json::json!(final_desc);
                            let _ = fs::write(&meta_path, serde_json::to_string_pretty(&meta_json).unwrap_or_default());
                        }
                    }
                }
            }
        }
    }

    let mut is_active = false;
    if !mod_list_ini_path.is_empty() {
        if let Ok(mut mod_list) = read_mod_list_ini(mod_list_ini_path) {
            for id in &mut mod_list.active_mods {
                if old_candidates.contains(id) {
                    *id = new_folder.clone();
                    is_active = true;
                }
            }
            let _ = write_mod_list_ini(mod_list_ini_path, &mod_list.active_mods);
        }
    }

    Ok(MergedPackageInfo {
        folder_name: new_folder.clone(),
        display_name,
        mod_id: new_folder,
        description: Some(final_desc.to_string()),
        is_packaged: false,
        is_active_in_modlist: is_active,
        is_companion_bridge: false,
        is_studio_link: false,
        created_at: None,
        packaged_mods: Vec::new(),
        merged_files: Vec::new(),
        version_check: None,
        built_for_profile: None,
    })
}

pub fn toggle_package_in_modlist(
    user_zomboid_dir: &str,
    mod_list_ini_path: &str,
    folder_name: &str,
    enabled: bool,
) -> Result<bool, String> {
    let clean_sub = folder_name
        .replace("PZ Mod Studio Patch: ", "")
        .replace("Z_PZModStudio_", "");
    let clean_sub = clean_sub.trim();

    let target_id = if folder_name.starts_with("Z_PZModStudio_") {
        folder_name.to_string()
    } else {
        format!("Z_PZModStudio_{}", clean_sub.replace(' ', "_"))
    };

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);

    // 1. Write is_visible_in_modlist to patch_metadata.json
    for u_dir in &user_dirs {
        for cand in &[target_id.clone(), folder_name.to_string()] {
            let pkg_dir = u_dir.join("mods").join(cand);
            if pkg_dir.exists() {
                let meta_path = pkg_dir.join("patch_metadata.json");
                let mut meta_json = if meta_path.exists() {
                    fs::read_to_string(&meta_path)
                        .ok()
                        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                        .unwrap_or_else(|| serde_json::json!({}))
                } else {
                    serde_json::json!({})
                };
                meta_json["is_visible_in_modlist"] = serde_json::json!(enabled);
                let _ = fs::write(meta_path, serde_json::to_string_pretty(&meta_json).unwrap_or_default());
            }
        }
    }

    // 2. If disabled (hidden from ModList), remove from active_mods so it's not active while hidden
    if !enabled {
        // Read the list in order: a HashSet here used to shuffle the player's whole load order.
        let mut active_mods: Vec<String> = crate::load_order::ini_parser::read_mod_list_ini(mod_list_ini_path)
            .map(|d| d.active_mods)
            .unwrap_or_else(|_| {
                get_active_mod_ids_from_disk(user_zomboid_dir, mod_list_ini_path).into_iter().collect()
            });
        let initial_len = active_mods.len();
        active_mods.retain(|id| id != &target_id && id != folder_name);

        if active_mods.len() != initial_len {
            let mut load_order: Vec<String> = Vec::new();
            for u_dir in &user_dirs {
                let master_json_path = u_dir.join("PZModStudio_MasterLoadOrder.json");
                if master_json_path.exists() {
                    if let Ok(content) = fs::read_to_string(&master_json_path) {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(arr) = parsed.get("load_order").and_then(|v| v.as_array()) {
                                for item in arr {
                                    if let Some(s) = item.as_str() {
                                        load_order.push(s.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let _ = crate::instance_manager::save_master_load_order(user_zomboid_dir.to_string(), load_order, active_mods);
        }
    }

    Ok(true)
}

/// Deletes a folder tree, clearing read-only flags on the way. Symlinks and junctions are
/// removed as links: their targets (which may be a real mod or a whole drive) are never walked.
pub(crate) fn force_remove_dir_all(path: &Path) -> std::io::Result<()> {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if meta.file_type().is_symlink() || is_reparse_point(&meta) {
        // A directory link (junction or dir symlink) is removed with remove_dir on Windows.
        return fs::remove_dir(path).or_else(|_| fs::remove_file(path));
    }
    if meta.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let _ = force_remove_dir_all(&entry.path());
            }
        }
        let _ = fs::remove_dir(path);
    } else {
        let mut permissions = meta.permissions();
        if permissions.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = fs::set_permissions(path, permissions);
        }
        let _ = fs::remove_file(path);
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn is_reparse_point(meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
pub(crate) fn is_reparse_point(_meta: &fs::Metadata) -> bool {
    false
}

/// The folder names a package may be stored under (older versions kept spaces in the name).
/// Only names that pass `to_package_folder` are returned, so callers can delete them safely.
pub(crate) fn package_folder_candidates(folder_name: &str) -> Result<Vec<String>, String> {
    let primary = to_package_folder(folder_name)?;
    let mut candidates = vec![primary.clone()];
    let sub = primary[PACKAGE_PREFIX.len()..].to_string();
    for alt in [format!("{}{}", PACKAGE_PREFIX, sub.replace('_', " "))] {
        if !candidates.contains(&alt) && to_package_folder(&alt).as_deref() == Ok(alt.as_str()) {
            candidates.push(alt);
        }
    }
    if candidates.iter().any(|c| super::is_system_folder(c)) {
        return Err(format!("The companion mod '{}' is an internal system component.", BRIDGE_FOLDER));
    }
    Ok(candidates)
}

pub fn delete_merged_package(user_zomboid_dir: &str, mod_list_ini_path: &str, folder_name: &str) -> Result<bool, String> {
    let candidates = package_folder_candidates(folder_name)?;

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    for dir in &user_dirs {
        for target in &candidates {
            let _ = force_remove_dir_all(&dir.join("mods").join(target));
            let _ = force_remove_dir_all(&dir.join("Lua").join("mods").join(target));
            let _ = force_remove_dir_all(&dir.join("Workshop").join("PZModStudioCarrier").join("Contents").join("mods").join(target));
        }
    }

    // Also remove from dynamically detected installation and workshop folders
    let detected_paths = crate::vfs::auto_detect_paths();
    let mut extra_dirs = Vec::new();
    if !detected_paths.pz_install_dir.is_empty() {
        extra_dirs.push(Path::new(&detected_paths.pz_install_dir).join("mods"));
    }
    if !detected_paths.workshop_dir.is_empty() {
        extra_dirs.push(Path::new(&detected_paths.workshop_dir).join("mods"));
    }
    for extra_dir in extra_dirs {
        for target in &candidates {
            let p = extra_dir.join(target);
            if p.exists() {
                let _ = force_remove_dir_all(&p);
            }
        }
    }

    if !mod_list_ini_path.is_empty() {
        if let Ok(mut mod_list) = read_mod_list_ini(mod_list_ini_path) {
            mod_list.active_mods.retain(|id| !candidates.contains(id));
            let _ = write_mod_list_ini(mod_list_ini_path, &mod_list.active_mods);
        }
    }

    Ok(true)
}

/// Adds a freshly published package to the saved load order and to the active profile, last,
/// so the Mod List agrees with `default.txt` instead of showing it as disabled.
pub(crate) fn remember_package_in_saved_order(user_zomboid_dir: &str, package: &str, active_mods: &[String]) {
    let Some(z_dir) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
    else {
        return;
    };
    let master = z_dir.join("PZModStudio_MasterLoadOrder.json");
    let mut load_order: Vec<String> = fs::read_to_string(&master)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| {
            v.get("load_order").and_then(|a| a.as_array()).map(|a| {
                a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()
            })
        })
        .unwrap_or_default();
    if load_order.is_empty() {
        // Nothing saved yet: the list the game reads is the best order we have.
        load_order = active_mods.to_vec();
    } else {
        load_order.retain(|id| id != package);
        load_order.push(package.to_string());
    }
    let _ = crate::instance_manager::save_master_load_order(
        user_zomboid_dir.to_string(),
        load_order,
        active_mods.to_vec(),
    );
}
