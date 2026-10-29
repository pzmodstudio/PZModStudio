//! A package's draft resolutions: the decisions saved for its files before and after a publish.

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftResolutionItem {
    pub relative_path: String,
    pub resolved_content: String,
    pub status: String,
}

pub fn save_draft_resolution(
    user_zomboid_dir: &str,
    package_folder_name: &str,
    relative_path: &str,
    resolved_content: &str,
    status: &str,
) -> Result<bool, String> {
    let clean_pkg_name = to_package_folder(package_folder_name)?;

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    if user_dirs.is_empty() {
        return Err("No valid Zomboid user directories found.".to_string());
    }

    let pkg_dir = user_dirs[0].join("mods").join(&clean_pkg_name);
    let _ = fs::create_dir_all(&pkg_dir);
    let draft_file = pkg_dir.join("draft_resolutions.json");

    let mut resolutions: std::collections::HashMap<String, DraftResolutionItem> = if draft_file.exists() {
        fs::read_to_string(&draft_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };

    if status == "CLEARED" {
        // Unmerging one file: the saved merge goes, and the file is back to the scan's own
        // proposal (what Unmerge All does for every file). Saving the vanilla text instead
        // would publish vanilla over both mods.
        resolutions.remove(relative_path);
    } else {
        resolutions.insert(
            relative_path.to_string(),
            DraftResolutionItem {
                relative_path: relative_path.to_string(),
                resolved_content: resolved_content.to_string(),
                status: status.to_string(),
            },
        );
    }

    let json_str = serde_json::to_string_pretty(&resolutions).map_err(|e| e.to_string())?;
    crate::load_order::ini_parser::guard_overwrite::<std::collections::HashMap<String, DraftResolutionItem>>(&draft_file)?;
    crate::load_order::ini_parser::atomic_write(draft_file, json_str).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn get_draft_resolutions(
    user_zomboid_dir: &str,
    package_folder_name: &str,
) -> std::collections::HashMap<String, DraftResolutionItem> {
    let Ok(clean_pkg_name) = to_package_folder(package_folder_name) else {
        return std::collections::HashMap::new();
    };

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    if user_dirs.is_empty() {
        return std::collections::HashMap::new();
    }

    let draft_file = user_dirs[0].join("mods").join(&clean_pkg_name).join("draft_resolutions.json");
    if draft_file.exists() {
        fs::read_to_string(&draft_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    }
}

pub fn clear_draft_resolutions(
    user_zomboid_dir: &str,
    package_folder_name: &str,
) -> Result<bool, String> {
    let clean_pkg_name = to_package_folder(package_folder_name)?;

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    for u_dir in user_dirs {
        let draft_file = u_dir.join("mods").join(&clean_pkg_name).join("draft_resolutions.json");
        if draft_file.exists() {
            // These are hand-written merges: keep a copy before throwing them away, the way the
            // mod list does.
            if let Ok(text) = fs::read_to_string(&draft_file) {
                let backups = u_dir.join("PZModStudio_Backups");
                let _ = fs::create_dir_all(&backups);
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let _ = fs::write(backups.join(format!("{}.merges.{}.json", clean_pkg_name, stamp)), text);
            }
            let _ = fs::remove_file(draft_file);
        }
    }
    Ok(true)
}
