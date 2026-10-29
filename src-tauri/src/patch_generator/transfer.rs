//! A package as one .pzmerge file: export and import.

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedMergedPackage {
    pub format_version: u32,
    pub folder_name: String,
    pub display_name: String,
    pub created_at: String,
    pub is_packaged: bool,
    pub packaged_mod_ids: Vec<String>,
    pub draft_resolutions: std::collections::HashMap<String, DraftResolutionItem>,
    pub merged_files: Vec<MergedFilePayload>,
    /// The mods the package was made with, each with the version its `mod.info` claimed and the
    /// publish time of the Workshop copy the exporter had: the merged files are those copies'
    /// code, so on another machine they only fit the same copies. Missing in files from before.
    #[serde(default)]
    pub mods: Vec<crate::preset_manager::PresetModEntry>,
    /// The game build it was made on.
    #[serde(default)]
    pub game_version: Option<String>,
}

/// Every installed mod, Workshop ones included (an empty Workshop folder would leave them out).
fn installed_catalog(user_zomboid_dir: &str) -> crate::catalog::ModCatalog {
    let workshop_dir = crate::vfs::auto_detect_paths().workshop_dir;
    crate::catalog::build_catalog(&workshop_dir, user_zomboid_dir, None)
}

/// Of the mods that were on when the package was made, the ones its files come from: a mod that
/// ships one of the package's paths (in any of its version folders), or, for a script block the
/// Merger settled, a mod its header names ("item X is declared by A and B"). A package records
/// the whole active list, and checking a hundred unrelated mods on import would only be noise.
fn mods_behind_files(
    catalog: &crate::catalog::ModCatalog,
    mod_ids: &[String],
    files: &[MergedFilePayload],
) -> Vec<String> {
    mod_ids
        .iter()
        .filter(|id| !id.starts_with("Z_PZModStudio_"))
        .filter(|id| {
            let Some(m) = catalog.resolve(id) else { return false };
            let root = std::path::Path::new(&m.root_dir);
            let mut bases = vec![root.to_path_buf(), root.join("common")];
            bases.extend(m.version_folders.iter().map(|v| root.join(v)));
            files.iter().any(|f| {
                bases.iter().any(|b| b.join(&f.relative_path).is_file())
                    || (f.relative_path.contains("/PZModStudio_")
                        && f.content.chars().take(600).collect::<String>().contains(m.name.as_str()))
            })
        })
        .cloned()
        .collect()
}

/// The package's mods as they are on this machine, for the file.
fn stamp_mods(catalog: &crate::catalog::ModCatalog, mod_ids: &[String]) -> Vec<crate::preset_manager::PresetModEntry> {
    mod_ids
        .iter()
        .filter(|id| !id.starts_with("Z_PZModStudio_"))
        .map(|id| {
            let m = catalog.resolve(id);
            crate::preset_manager::PresetModEntry {
                mod_id: id.clone(),
                name: m.map(|m| m.name.clone()).unwrap_or_else(|| id.clone()),
                workshop_id: m.and_then(|m| match &m.source {
                    crate::catalog::ModSource::Workshop { workshop_id } => Some(workshop_id.clone()),
                    _ => None,
                }),
                enabled: true,
                version: m.and_then(|m| m.info.mod_version.clone()),
                updated_at: m.and_then(|m| m.workshop_state.as_ref().and_then(|s| s.installed_time)),
            }
        })
        .collect()
}

pub fn export_merged_package(
    user_zomboid_dir: &str,
    package_folder_name: &str,
    target_file_path: &str,
) -> Result<bool, String> {
    let clean_pkg_name = to_package_folder(package_folder_name)?;

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    if user_dirs.is_empty() {
        return Err("No valid Zomboid user directories found.".to_string());
    }

    let pkg_dir = user_dirs[0].join("mods").join(&clean_pkg_name);
    if !pkg_dir.exists() {
        return Err(format!("The package '{}' does not exist on disk.", clean_pkg_name));
    }

    let meta_path = pkg_dir.join("patch_metadata.json");
    let (is_packaged, created_at, packaged_mods) = if meta_path.exists() {
        if let Ok(content) = fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<MasterPatchMetadata>(&content) {
                (meta.is_packaged, meta.created_at, meta.packaged_mod_ids)
            } else {
                (false, String::new(), Vec::new())
            }
        } else {
            (false, String::new(), Vec::new())
        }
    } else {
        (false, String::new(), Vec::new())
    };

    let draft_resolutions = get_draft_resolutions(user_zomboid_dir, &clean_pkg_name);

    let display_name = clean_pkg_name["Z_PZModStudio_".len()..].replace('_', " ");

    let mut merged_files = Vec::new();
    let media_dir = pkg_dir.join("media");
    if media_dir.exists() {
        for entry in walkdir::WalkDir::new(&media_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(rel) = entry.path().strip_prefix(&pkg_dir) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        merged_files.push(MergedFilePayload {
                            relative_path: rel_str,
                            content,
                        });
                    }
                }
            }
        }
    }

    let catalog = installed_catalog(user_zomboid_dir);
    let behind = mods_behind_files(&catalog, &packaged_mods, &merged_files);
    let mods = stamp_mods(&catalog, &behind);
    let export_payload = ExportedMergedPackage {
        format_version: 1,
        mods,
        game_version: catalog.game_version.clone(),
        folder_name: clean_pkg_name,
        display_name,
        created_at,
        is_packaged,
        packaged_mod_ids: packaged_mods,
        draft_resolutions,
        merged_files,
    };

    let json_str = serde_json::to_string_pretty(&export_payload).map_err(|e| e.to_string())?;
    fs::write(target_file_path, json_str).map_err(|e| e.to_string())?;

    Ok(true)
}

/// `wanted`, or `wanted_2`, `wanted_3`... whichever is not a folder in `mods_dir` yet.
fn free_package_folder(mods_dir: &Path, wanted: &str) -> String {
    if !mods_dir.join(wanted).exists() {
        return wanted.to_string();
    }
    (2..)
        .map(|n| format!("{}_{}", wanted, n))
        .find(|name| !mods_dir.join(name).exists())
        .unwrap_or_else(|| wanted.to_string())
}

pub fn import_merged_package(
    user_zomboid_dir: &str,
    _mod_list_ini_path: &str,
    source_file_path: &str,
) -> Result<MergedPackageInfo, String> {
    if source_file_path.ends_with(".pzpack") {
        return Err("This file is a Collection Preset (.pzpack), not a Merge Package (.pzmerge). To import mod list presets, go to 'Mod List' > 'Presets' > 'Import .pzpack'.".to_string());
    }

    let content = fs::read_to_string(source_file_path).map_err(|e| format!("Error reading file: {}", e))?;

    if content.contains("\"load_order\"") && !content.contains("\"merged_files\"") {
        return Err("This file corresponds to a Mod List Preset (.pzpack). Please import it from the 'Mod List' tab.".to_string());
    }

    let imported: ExportedMergedPackage = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid .pzmerge package format: {}", e))?;

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    if user_dirs.is_empty() {
        return Err("No valid Zomboid user directories found.".to_string());
    }

    let clean_folder = to_package_folder(&imported.folder_name)?;
    if super::is_system_folder(&clean_folder) {
        return Err("A .pzmerge package cannot replace the companion mod 'Z_PZModStudio_Bridge'.".to_string());
    }
    if imported.merged_files.is_empty() {
        return Err(
            "This package has no merged files in it — whoever exported it had not published it yet, \
             so there is nothing to install. Ask them to press Package & Publish and export again."
                .to_string(),
        );
    }

    // A package already here is the user's own work (drafts, merges, hand patches): the import
    // goes beside it under a free name instead of over it.
    let mods_dir = user_dirs[0].join("mods");
    let clean_folder = free_package_folder(&mods_dir, &clean_folder);
    let pkg_dir = mods_dir.join(&clean_folder);

    // Check every path in the (untrusted) file before anything is written.
    let mut file_targets = Vec::with_capacity(imported.merged_files.len());
    for file in &imported.merged_files {
        let lower = file.relative_path.trim().replace('\\', "/").trim_start_matches("./").to_ascii_lowercase();
        if ["mod.info", "patch_metadata.json", "draft_resolutions.json"].contains(&lower.as_str()) {
            return Err(format!("A .pzmerge cannot carry its own {}.", file.relative_path.trim()));
        }
        let dest = safe_join(&pkg_dir, &file.relative_path)?;
        let dest_42 = safe_join(&pkg_dir.join("42"), &file.relative_path)?;
        file_targets.push((dest, dest_42, &file.content));
    }

    fs::create_dir_all(&pkg_dir).map_err(|e| e.to_string())?;

    // The name ends up in mod.info, one key per line: keep it on one line.
    let display_name: String = imported.display_name.chars().filter(|c| !c.is_control()).collect();
    let in_game_title = format!("PZ Mod Studio Patch: {}", display_name);

    let mod_info_content = format!(
        "name={}\r\nid={}\r\ndescription=Compatibility merge package synthesized by PZ Mod Studio.\r\nposter=poster.png\r\nicon=icon.png\r\nmodversion=1.0.0\r\npzversion=41,42\r\nversionMin=41.00\r\nauthor=PZ Mod Studio\r\n",
        in_game_title, clean_folder
    );
    let _ = fs::write(pkg_dir.join("mod.info"), &mod_info_content);
    let png_256 = get_preview_png_bytes();
    let _ = fs::write(pkg_dir.join("poster.png"), &png_256);
    let _ = fs::write(pkg_dir.join("icon.png"), &png_256);

    // A merge package is vanilla files rewritten with other mods' code spliced in. Without
    // those mods the game runs code that refers to things that are not there, so the import
    // reports what is missing rather than letting it fail in game.
    // And the merged files are the exporter's copies of those mods: with another copy here they
    // can put old code over new, so the import also says where the copies differ.
    let catalog = installed_catalog(user_zomboid_dir);
    let entries = if imported.mods.is_empty() {
        // A file from before versions were recorded: only what is missing can be told.
        imported
            .packaged_mod_ids
            .iter()
            .filter(|id| !id.starts_with("Z_PZModStudio_"))
            .map(|id| crate::preset_manager::PresetModEntry {
                mod_id: id.clone(),
                name: id.clone(),
                workshop_id: None,
                enabled: true,
                version: None,
                updated_at: None,
            })
            .collect()
    } else {
        imported.mods.clone()
    };
    let as_pack = crate::preset_manager::ModPreset {
        id: String::new(),
        name: imported.display_name.clone(),
        description: None,
        author: None,
        created_at: imported.created_at.clone(),
        load_order: Vec::new(),
        mods: entries,
        game_version: imported.game_version.clone(),
        sandbox: None,
    };
    let version_check = crate::preset_manager::compare_with_installed(&as_pack, &catalog);
    let missing: Vec<String> = version_check.missing_mods.iter().map(|m| m.mod_id.clone()).collect();

    let meta = MasterPatchMetadata {
        // Whoever built it says it is published; on this machine it has not been checked
        // against anything, so it starts as a draft until the user publishes it here.
        is_packaged: false,
        created_at: if imported.created_at.is_empty() {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs().to_string()).unwrap_or_default()
        } else {
            imported.created_at.clone()
        },
        packaged_mod_ids: imported.packaged_mod_ids.clone(),
        merged_file_paths: imported.merged_files.iter().map(|f| f.relative_path.clone()).collect(),
    };
    if let Ok(meta_json) = serde_json::to_string_pretty(&meta) {
        let _ = fs::write(pkg_dir.join("patch_metadata.json"), &meta_json);
    }

    if let Ok(draft_json) = serde_json::to_string_pretty(&imported.draft_resolutions) {
        let _ = fs::write(pkg_dir.join("draft_resolutions.json"), &draft_json);
    }

    // Write merged files to disk (top level and the 42/ subfolder for native B42 compatibility)
    for (dest, dest_42, content) in file_targets {
        for target in [dest, dest_42] {
            if let Some(parent) = target.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(target, content);
        }
    }

    let description = if missing.is_empty() {
        "Imported merge package. Check it in the Mod Merger, then publish it here.".to_string()
    } else {
        format!(
            "Imported merge package. It patches {} mod(s) you do not have ({}), so parts of it will not work until you install them. Check it in the Mod Merger before playing.",
            missing.len(),
            missing.join(", ")
        )
    };

    Ok(MergedPackageInfo {
        folder_name: clean_folder.clone(),
        display_name,
        mod_id: clean_folder,
        // It has not been published on this machine yet, whatever the file claims.
        is_packaged: false,
        description: Some(description),
        is_active_in_modlist: false,
        is_companion_bridge: false,
        is_studio_link: false,
        created_at: Some(meta.created_at),
        packaged_mods: imported.packaged_mod_ids,
        merged_files: imported.merged_files.iter().map(|f| f.relative_path.clone()).collect(),
        version_check: Some(version_check),
        built_for_profile: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_import_never_lands_on_a_package_that_is_there() {
        let mods = std::env::temp_dir().join(format!("pzms-free-pkg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&mods);
        fs::create_dir_all(mods.join("Z_PZModStudio_MergedPatch")).unwrap();
        fs::create_dir_all(mods.join("Z_PZModStudio_MergedPatch_2")).unwrap();
        assert_eq!(free_package_folder(&mods, "Z_PZModStudio_MergedPatch"), "Z_PZModStudio_MergedPatch_3");
        assert_eq!(free_package_folder(&mods, "Z_PZModStudio_Other"), "Z_PZModStudio_Other");
        let _ = fs::remove_dir_all(&mods);
    }

    #[test]
    fn a_pzmerge_from_before_versions_were_recorded_still_reads() {
        let old = r#"{"format_version":1,"folder_name":"Z_PZModStudio_X","display_name":"X","created_at":"","is_packaged":true,"packaged_mod_ids":["A"],"draft_resolutions":{},"merged_files":[]}"#;
        let p: ExportedMergedPackage = serde_json::from_str(old).unwrap();
        assert!(p.mods.is_empty() && p.game_version.is_none());
    }

    /// Exports a real package to a temp file and compares its recorded copies with this install.
    /// Writes nothing in the Zomboid folder. PZMS_ZOMBOID_DIR, optional PZMS_PACKAGE.
    #[test]
    #[ignore]
    fn a_real_package_records_its_mods_copies() {
        let Ok(zomboid) = std::env::var("PZMS_ZOMBOID_DIR") else { return };
        let package = std::env::var("PZMS_PACKAGE").unwrap_or_else(|_| "Z_PZModStudio_MergedPatch".into());
        let out = std::env::temp_dir().join("pzms-real-export.pzmerge");
        export_merged_package(&zomboid, &package, &out.to_string_lossy()).unwrap();
        let p: ExportedMergedPackage = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
        println!("{} mods recorded, build {:?}", p.mods.len(), p.game_version);
        for m in &p.mods {
            println!("  {} v{:?} ws {:?} updated {:?}", m.mod_id, m.version, m.workshop_id, m.updated_at);
        }
        let pack = crate::preset_manager::ModPreset {
            id: String::new(), name: p.display_name.clone(), description: None, author: None,
            created_at: String::new(), load_order: Vec::new(), mods: p.mods.clone(),
            game_version: p.game_version.clone(), sandbox: None,
        };
        let r = crate::preset_manager::compare_with_installed(&pack, &installed_catalog(&zomboid));
        println!("same machine: {} missing, {} differing", r.missing_mods.len(), r.drift.len());
        assert!(r.missing_mods.is_empty() && r.drift.is_empty(), "the exporter's own machine matches itself");
        let _ = fs::remove_file(out);
    }
}
