use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use crate::load_order::ini_parser::{read_mod_list_ini, write_mod_list_ini};

pub const PACKAGE_PREFIX: &str = "Z_PZModStudio_";
pub const BRIDGE_FOLDER: &str = "Z_PZModStudio_Bridge";

/// The Studio's own mods (the Live Bridge companion and the Studio Link): listed with the
/// packages so they can be seen, never renamed, filled, published over, cleaned or deleted.
pub fn is_system_folder(folder: &str) -> bool {
    folder == BRIDGE_FOLDER || folder == crate::studio_link::LINK_MOD_ID
}

/// Maps a package name ("My Patch", "PZ Mod Studio Patch: My Patch" or "Z_PZModStudio_My_Patch")
/// to its folder name, and refuses anything that is not one plain folder name inside the
/// Z_PZModStudio_ namespace. Every function that creates, writes or deletes a package folder
/// goes through this, so an empty name, ".." or a path can never reach fs::remove_dir_all.
pub fn to_package_folder(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let folder = if trimmed.starts_with(PACKAGE_PREFIX) {
        trimmed.to_string()
    } else {
        let sub = trimmed.replace("PZ Mod Studio Patch: ", "");
        format!("{}{}", PACKAGE_PREFIX, sub.trim().replace(' ', "_"))
    };
    let suffix = &folder[PACKAGE_PREFIX.len()..];
    let invalid = suffix.is_empty()
        || suffix.chars().all(|c| c == '.' || c == '_')
        || folder.ends_with('.')
        || folder.chars().any(|c| c.is_control() || "<>:\"/\\|?*".contains(c));
    if invalid {
        return Err(format!("Invalid package name: '{}'", raw));
    }
    Ok(folder)
}

/// Joins a relative path taken from a request or an imported file onto `base`, refusing
/// absolute paths, drive prefixes and `..` so the result always stays inside `base`.
pub fn safe_join(base: &Path, relative: &str) -> Result<PathBuf, String> {
    let rel = Path::new(relative.trim());
    let mut has_normal = false;
    for comp in rel.components() {
        match comp {
            Component::Normal(part) => {
                if part.to_string_lossy().contains(':') {
                    return Err(format!("Unsafe file path: '{}'", relative));
                }
                has_normal = true;
            }
            Component::CurDir => {}
            _ => return Err(format!("Unsafe file path: '{}'", relative)),
        }
    }
    if !has_normal {
        return Err(format!("Empty file path: '{}'", relative));
    }
    Ok(base.join(rel))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedFilePayload {
    pub relative_path: String, // e.g. "media/lua/client/ISUI/ISInventoryPane.lua"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterPatchRequest {
    pub workshop_dir: Option<String>,
    pub pz_install_dir: Option<String>,
    pub user_zomboid_dir: String,
    pub mod_list_ini_path: String,
    pub merged_files: Vec<MergedFilePayload>,
    pub package_folder_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedPackageInfo {
    pub folder_name: String,
    pub display_name: String,
    pub mod_id: String,
    pub description: Option<String>,
    pub is_packaged: bool,
    pub is_active_in_modlist: bool,
    /// One of the Studio's own mods (see is_system_folder): shown, locked.
    pub is_companion_bridge: bool,
    /// The Studio Link mod in particular (the game's Mods screen and the profile).
    #[serde(default)]
    pub is_studio_link: bool,
    pub created_at: Option<String>,
    pub packaged_mods: Vec<String>,
    pub merged_files: Vec<String>,
    /// Only on an import: the mods the package was made with that are missing here, and the ones
    /// whose copy here is not the copy it was made with.
    #[serde(default)]
    pub version_check: Option<crate::preset_manager::MissingModsReport>,
    /// The profile that was active when it was last published (its merges settle that list).
    #[serde(default)]
    pub built_for_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterPatchMetadata {
    pub is_packaged: bool,
    pub created_at: String,
    pub packaged_mod_ids: Vec<String>,
    pub merged_file_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterPatchStatusInfo {
    pub is_packaged: bool,
    pub created_at: Option<String>,
    pub packaged_mods: Vec<String>,
    pub merged_files: Vec<String>,
    pub missing_active_mods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterPatchResult {
    pub success: bool,
    pub patch_mod_dir: String,
    pub files_written: usize,
    /// Whether the Build 42 compatibility shim was written into the package.
    pub polyfill_shim_written: bool,
}

mod assets;
mod drafts;
mod packages;
mod publish;
mod shim;
pub mod stale;
mod transfer;

pub use assets::*;
pub use drafts::*;
pub use packages::*;
pub use publish::*;
pub use shim::*;
pub use transfer::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_validate_polyfill_lua_ast() {
        let poly_content = generate_master_polyfill_lua();
        let parse_result = crate::diff_engine::lua::validate_lua_syntax(&poly_content);
        assert!(parse_result.is_valid, "generate_master_polyfill_lua() must be valid Lua syntax: {:?}", parse_result.error_message);
    }

    #[test]
    fn test_master_polyfill_contains_core_patches() {
        // Tests must never write into a real Zomboid folder: only check the generated text.
        let poly_content = generate_master_polyfill_lua();
        assert!(poly_content.contains("IGUI_ItemCat_GunMag"));
        assert!(poly_content.contains("sanitizeAllScriptItems") || poly_content.contains("getAllItems"));
        assert!(poly_content.contains("BWORoomPrograms"));
        assert!(poly_content.contains("IsoDoor"));
    }

    #[test]
    fn package_folder_names_are_confined_to_the_namespace() {
        assert_eq!(to_package_folder("My Patch").unwrap(), "Z_PZModStudio_My_Patch");
        assert_eq!(to_package_folder("PZ Mod Studio Patch: My Patch").unwrap(), "Z_PZModStudio_My_Patch");
        assert_eq!(to_package_folder("Z_PZModStudio_pack_1").unwrap(), "Z_PZModStudio_pack_1");
        assert_eq!(to_package_folder("Parche Armas ñ").unwrap(), "Z_PZModStudio_Parche_Armas_ñ");
        for bad in ["", "   ", "..", "Z_PZModStudio_", "Z_PZModStudio_..", "../Saves", "a/b", "a\\b", "C:", "x\0y", "name."] {
            assert!(to_package_folder(bad).is_err(), "accepted {:?}", bad);
        }
    }

    #[test]
    fn bridge_cannot_be_deleted_or_renamed() {
        assert!(package_folder_candidates("Z_PZModStudio_Bridge").is_err());
        assert!(package_folder_candidates("Bridge").is_err());
        assert!(package_folder_candidates("").is_err());
        assert!(package_folder_candidates("..").is_err());
        let ok = package_folder_candidates("My Patch").unwrap();
        assert_eq!(ok, vec!["Z_PZModStudio_My_Patch", "Z_PZModStudio_My Patch"]);
    }

    #[test]
    fn safe_join_rejects_escaping_paths() {
        let base = Path::new("base");
        assert_eq!(safe_join(base, "media/lua/client/A.lua").unwrap(), base.join("media/lua/client/A.lua"));
        assert_eq!(safe_join(base, "./media/x.txt").unwrap(), base.join("./media/x.txt"));
        for bad in ["", ".", "../x.lua", "media/../../x.lua", "/etc/passwd", "C:/Windows/x", "C:x", "\\\\server\\share\\x", "media/a.lua:stream"] {
            assert!(safe_join(base, bad).is_err(), "accepted {:?}", bad);
        }
        #[cfg(windows)]
        assert!(safe_join(base, "..\\..\\Startup\\evil.bat").is_err());
    }

    #[test]
    fn force_remove_does_not_follow_directory_links() {
        let root = std::env::temp_dir().join(format!("pzms-rm-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let outside = root.join("outside");
        let pkg = root.join("Z_PZModStudio_Test");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep.txt"), "keep").unwrap();
        fs::create_dir_all(pkg.join("media")).unwrap();
        fs::write(pkg.join("media").join("a.lua"), "x").unwrap();

        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&outside, pkg.join("link")).is_ok()
            || std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(pkg.join("link"))
                .arg(&outside)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&outside, pkg.join("link")).is_ok();

        force_remove_dir_all(&pkg).unwrap();
        assert!(!pkg.exists());
        if linked {
            assert!(outside.join("keep.txt").exists(), "deleted through a link");
        }
        let _ = fs::remove_dir_all(&root);
    }
}
