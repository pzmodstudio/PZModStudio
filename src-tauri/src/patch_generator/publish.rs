//! Writing a package to disk as a mod the game reads, one file at a time or all at once, and taking it back out.

use super::*;

/// `media/...` as the package stores it, so the same file written two ways compares equal.
pub(crate) fn normalise_media_path(path: &str) -> String {
    let p = path.replace('\\', "/");
    let p = p.trim_start_matches("./").to_string();
    if p.starts_with("media/") { p } else { format!("media/{}", p.trim_start_matches('/')) }
}

/// The files the package's last publish wrote, from the metadata it left behind. Without that
/// metadata nothing is assumed, and nothing is deleted.
pub(crate) fn previously_written(patch_mod_dir: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(patch_mod_dir.join("patch_metadata.json")) else {
        return Vec::new();
    };
    serde_json::from_str::<MasterPatchMetadata>(&text)
        .map(|meta| meta.merged_file_paths)
        .unwrap_or_default()
}

/// The package's metadata after a publish: what this publish knows replaces its keys, the rest of
/// the file stays (the description and visibility the package was given), and it records the
/// profile it was built under.
pub(crate) fn metadata_after_publish(previous: Option<&str>, meta: &MasterPatchMetadata, profile: Option<(&str, &str)>) -> String {
    let mut value = previous
        .and_then(|t| serde_json::from_str::<serde_json::Value>(t.trim_start_matches('\u{feff}')).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    if let (Some(obj), Ok(serde_json::Value::Object(new))) = (value.as_object_mut(), serde_json::to_value(meta)) {
        obj.extend(new);
        match profile {
            Some((id, name)) => {
                obj.insert("built_for_profile_id".into(), id.into());
                obj.insert("built_for_profile_name".into(), name.into());
            }
            None => {
                obj.remove("built_for_profile_id");
                obj.remove("built_for_profile_name");
            }
        }
    }
    serde_json::to_string_pretty(&value).unwrap_or_default()
}

/// Generates a synthetic master patch mod under Zomboid/mods/<package_folder_name> and updates ModListData.ini.
pub fn generate_master_patch(req: MasterPatchRequest) -> Result<MasterPatchResult, String> {
    let pkg_name = req.package_folder_name
        .clone()
        .unwrap_or_else(|| "Z_PZModStudio_MergedPatch".to_string());

    let clean_pkg_name = to_package_folder(&pkg_name)?;

    let display_name = if clean_pkg_name.starts_with("Z_PZModStudio_") {
        clean_pkg_name["Z_PZModStudio_".len()..].to_string().replace('_', " ")
    } else {
        clean_pkg_name.clone()
    };

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(&req.user_zomboid_dir);
    if user_dirs.is_empty() {
        return Err("No valid Zomboid directories provided to generate master patch.".to_string());
    }

    let primary_dir = user_dirs[0].join("mods").join(&clean_pkg_name);

    // Refuse the whole request up front if any path would escape the package folder.
    for file in &req.merged_files {
        safe_join(&primary_dir, &file.relative_path)?;
    }

    let polyfill_lua_content = generate_master_polyfill_lua();

    let png_256 = get_preview_png_bytes();

    // What this publish will contain, so what the last one left behind can be told apart from
    // what somebody else put in the package.
    let now_writing: std::collections::HashSet<String> =
        req.merged_files.iter().map(|f| normalise_media_path(&f.relative_path)).collect();

    // The package is built next to the game's mods folder and swapped in at the end, so the game
    // (or a crash halfway) never sees half a publish. The copy starts from what the package holds
    // now, so files placed in it by hand, its drafts and its pictures come along.
    let staging_root = user_dirs[0].join("PZModStudio_Staging");
    let stage = staging_root.join(format!("{}.new", clean_pkg_name));
    let _ = force_remove_dir_all(&stage); // left by a publish that was interrupted
    fs::create_dir_all(&staging_root).map_err(|e| e.to_string())?;
    if primary_dir.is_dir() {
        copy_dir_all(&primary_dir, &stage).map_err(|e| {
            let _ = force_remove_dir_all(&stage);
            format!("Could not copy the package to stage the publish: {}", e)
        })?;
    } else {
        fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    }

    // The package keeps the name and description it was given (create, rename); only a package
    // without them gets the default ones.
    let (kept_name, kept_description) = read_name_and_description(&primary_dir.join("mod.info"));
    let mut mod_info_content = String::new();
    mod_info_content.push_str(&format!("name={}\r\n", kept_name.unwrap_or_else(|| format!("PZ Mod Studio Patch: {}", display_name))));
    mod_info_content.push_str(&format!("id={}\r\n", clean_pkg_name));
    mod_info_content.push_str(&format!(
        "description={}\r\n",
        kept_description.unwrap_or_else(|| "Auto-generated compatibility patch package generated by Project Zomboid Mod Studio.".to_string())
    ));
    mod_info_content.push_str("poster=poster.png\r\n");
    mod_info_content.push_str("icon=icon.png\r\n");
    mod_info_content.push_str("modversion=1.0.0\r\n");
    mod_info_content.push_str("pzversion=41,42\r\n");
    mod_info_content.push_str("versionMin=41.00\r\n");
    mod_info_content.push_str("author=PZ Mod Studio\r\n");

    let built = (|| -> Result<usize, String> {
        let patch_mod_dir = &stage;
        let mut files_written = 0;

        // Only what a previous publish wrote and this one does not is removed.
        //
        // This used to delete `media/` and `42/` whole, which also deleted files the package held
        // for other reasons -- a mod's own file patched by hand, for instance -- without a word.
        // A merge of two unrelated files, run from Mod Health, silently took such a patch with it.
        for old in previously_written(patch_mod_dir) {
            if now_writing.contains(&normalise_media_path(&old)) {
                continue;
            }
            remove_published_file(patch_mod_dir, &old);
        }
        // The shim is rewritten below; an older copy must not survive under another name.
        for root in [patch_mod_dir.join("media"), patch_mod_dir.join("42").join("media")] {
            let _ = fs::remove_file(root.join("lua").join("shared").join("Z_PZModStudio_Polyfills.lua"));
        }

        // A picture placed in the package by hand stays.
        let write_png = |path: PathBuf| {
            if !path.exists() {
                let _ = fs::write(path, &png_256);
            }
        };

        // 1. Top level mod.info and media/
        fs::write(patch_mod_dir.join("mod.info"), &mod_info_content).map_err(|e| e.to_string())?;
        write_png(patch_mod_dir.join("poster.png"));
        write_png(patch_mod_dir.join("icon.png"));

        let media_dir = patch_mod_dir.join("media");
        let _ = fs::create_dir_all(&media_dir);
        let _ = fs::write(media_dir.join("mod.info"), &mod_info_content);
        write_png(media_dir.join("poster.png"));
        write_png(media_dir.join("icon.png"));

        // 2. Build 42 native subfolder structure (42/mod.info and 42/media/)
        let dir_42 = patch_mod_dir.join("42");
        let _ = fs::create_dir_all(&dir_42);
        fs::write(dir_42.join("mod.info"), &mod_info_content).map_err(|e| e.to_string())?;
        write_png(dir_42.join("poster.png"));
        write_png(dir_42.join("icon.png"));

        let media_42_dir = dir_42.join("media");
        let _ = fs::create_dir_all(&media_42_dir);
        let _ = fs::write(media_42_dir.join("mod.info"), &mod_info_content);

        // Write merged files to both top level and 42/ subfolder
        let mut failed: Vec<String> = Vec::new();
        for file in &req.merged_files {
            if file.relative_path.contains("sandbox-options.txt") || file.content.starts_with("-- vanilla file not present") {
                continue;
            }
            for root in [patch_mod_dir, &dir_42] {
                let dest_path = safe_join(root, &file.relative_path)?;
                if let Some(parent) = dest_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Err(e) = fs::write(&dest_path, &file.content) {
                    failed.push(format!("{} ({})", file.relative_path, e));
                }
            }
            files_written += 1;
        }
        // A package half written is not published: the game would load part of the merges.
        if !failed.is_empty() {
            failed.dedup();
            return Err(format!("These files could not be written into the package: {}", failed.join(", ")));
        }

        // Polyfills in both media/lua/shared and 42/media/lua/shared
        for shared in [media_dir.join("lua").join("shared"), media_42_dir.join("lua").join("shared")] {
            let _ = fs::create_dir_all(&shared);
            fs::write(shared.join("Z_PZModStudio_Polyfills.lua"), &polyfill_lua_content).map_err(|e| e.to_string())?;
        }

        // The shim's per-mod switches, as Settings left them, so a fresh package honours them.
        write_patch_switches_into(patch_mod_dir, &patch_switches_lua(&read_patch_switches(&req.user_zomboid_dir)));

        // Nothing else is written: a package carries merged files and the shim, and leaves the
        // game's own screens alone. It used to replace the in-game mod-order screen's "Auto"
        // button with a message saying sorting was locked by the Studio, which is not the
        // Studio's to take away — the next save of the mod list puts its order back anyway.
        Ok(files_written)
    })();
    let files_written = match built {
        Ok(n) => n,
        Err(e) => {
            let _ = force_remove_dir_all(&stage);
            return Err(e);
        }
    };

    // 2. The package goes last in the list the game reads, where a patch has to be to win. Its
    //    metadata and drafts are written into the staged copy, so they arrive with the files.
    let mod_list = if req.mod_list_ini_path.is_empty() { None } else { read_mod_list_ini(&req.mod_list_ini_path).ok() };
    let mod_list = mod_list.map(|mut mod_list| {
        // Other packages are the user's and stay. Only this one moves.
        mod_list.active_mods.retain(|id| id != &clean_pkg_name);
        mod_list.active_mods.push(clean_pkg_name.clone());
        mod_list
    });
    if let Some(mod_list) = &mod_list {
        let merged_file_paths: Vec<String> = req.merged_files.iter().map(|f| f.relative_path.clone()).collect();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        let meta = MasterPatchMetadata {
            is_packaged: true,
            created_at: format!("Epoch-{}", timestamp),
            packaged_mod_ids: mod_list.active_mods.clone(),
            merged_file_paths,
        };
        let profile = crate::instance_manager::active_profile(&req.user_zomboid_dir);
        let previous = fs::read_to_string(stage.join("patch_metadata.json")).ok();
        let meta_json = metadata_after_publish(previous.as_deref(), &meta, profile.as_ref().map(|(id, name)| (id.as_str(), name.as_str())));
        let _ = fs::write(stage.join("patch_metadata.json"), &meta_json);

        // Every merged file is kept as a resolved draft, so it stays resolved when the package is
        // unpacked or reopened.
        let mut draft_resolutions = get_draft_resolutions(&req.user_zomboid_dir, &clean_pkg_name);
        for file in &req.merged_files {
            draft_resolutions.insert(
                file.relative_path.clone(),
                DraftResolutionItem {
                    relative_path: file.relative_path.clone(),
                    resolved_content: file.content.clone(),
                    status: "RESOLVED".to_string(),
                },
            );
        }
        if let Ok(draft_json) = serde_json::to_string_pretty(&draft_resolutions) {
            let _ = fs::write(stage.join("draft_resolutions.json"), &draft_json);
        }
    }

    swap_into_place(&stage, &primary_dir, &staging_root.join(format!("{}.old", clean_pkg_name)))?;

    if let Some(mod_list) = mod_list {
        // The Mod List reads the saved order, not default.txt, so put the package there too:
        // without this it comes back as disabled and the next toggle drops it again.
        remember_package_in_saved_order(&req.user_zomboid_dir, &clean_pkg_name, &mod_list.active_mods);
        let _ = write_mod_list_ini(&req.mod_list_ini_path, &mod_list.active_mods);
    }

    Ok(MasterPatchResult {
        success: true,
        patch_mod_dir: primary_dir.to_string_lossy().to_string(),
        files_written,
        polyfill_shim_written: true,
    })
}

/// The `name=` and `description=` a package's `mod.info` carries, if any.
fn read_name_and_description(mod_info: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = fs::read_to_string(mod_info) else {
        return (None, None);
    };
    let field = |key: &str| {
        text.lines()
            .find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim().to_string()))
            .filter(|v| !v.is_empty())
    };
    (field("name="), field("description="))
}

/// Removes one file a publish wrote, from the package's top level and its `42/` folder.
fn remove_published_file(package_dir: &Path, relative: &str) {
    // The list is read from the package's own metadata, which an installed archive can carry
    // too: only paths that stay inside the package are touched.
    let inner = relative.trim_start_matches("media/").trim_start_matches("media\\");
    for root in [package_dir.to_path_buf(), package_dir.join("42")] {
        if let Ok(p) = safe_join(&root, relative) {
            let _ = fs::remove_file(p);
        }
        if let Ok(p) = safe_join(&root, inner) {
            let _ = fs::remove_file(p);
        }
    }
}

/// Copies a folder tree. Links are not followed: a package holds plain files.
fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Puts the staged package where the game reads it. The current one is moved aside first and
/// comes back if the staged one cannot take its place, so a failed swap changes nothing.
fn swap_into_place(stage: &Path, live: &Path, aside: &Path) -> Result<(), String> {
    let _ = force_remove_dir_all(aside);
    let had_live = live.exists();
    if had_live {
        if let Err(e) = fs::rename(live, aside) {
            let _ = force_remove_dir_all(stage);
            return Err(format!(
                "The package is in use and could not be replaced ({}). Close the game or any window showing its folder, then publish again.",
                e
            ));
        }
    }
    if let Err(e) = fs::rename(stage, live) {
        if had_live {
            let _ = fs::rename(aside, live);
        }
        let _ = force_remove_dir_all(stage);
        return Err(format!("The published package could not be put in place: {}", e));
    }
    let _ = force_remove_dir_all(aside);
    Ok(())
}

/// Puts one more file into a package the user already has, by the same publish the Merger runs:
/// every resolved draft the package holds is published again, plus this one. Nothing the package
/// held is dropped, and a packaged package stays packaged.
///
/// This is how one file gets into a published package without unpacking it: unpacking deletes
/// every published file from the game until the next publish, which is a lot to pay for one
/// corrected `require`.
pub fn publish_file_into_package(
    user_zomboid_dir: &str,
    mod_list_ini_path: &str,
    package_folder_name: &str,
    relative_path: &str,
    content: &str,
) -> Result<MasterPatchResult, String> {
    let package = to_package_folder(package_folder_name)?;
    let mut merged_files: Vec<MergedFilePayload> = get_draft_resolutions(user_zomboid_dir, &package)
        .into_values()
        .filter(|d| d.status == "RESOLVED" && d.relative_path != relative_path)
        .map(|d| MergedFilePayload { relative_path: d.relative_path, content: d.resolved_content })
        .collect();
    merged_files.push(MergedFilePayload { relative_path: relative_path.to_string(), content: content.to_string() });
    generate_master_patch(MasterPatchRequest {
        workshop_dir: None,
        pz_install_dir: None,
        user_zomboid_dir: user_zomboid_dir.to_string(),
        mod_list_ini_path: mod_list_ini_path.to_string(),
        merged_files,
        package_folder_name: Some(package),
    })
}

/// Takes one file out of a package, published in place: its draft goes, everything else the
/// package holds is published again without it, and the game reads the mod's own copy of that
/// file on its next launch. `file` is the path in the package, or just the file's name.
pub fn remove_file_from_package(
    user_zomboid_dir: &str,
    mod_list_ini_path: &str,
    package_folder_name: &str,
    file: &str,
) -> Result<MasterPatchResult, String> {
    let package = to_package_folder(package_folder_name)?;
    let wanted = file.replace('\\', "/");
    let mut drafts = get_draft_resolutions(user_zomboid_dir, &package);
    let Some(path) = drafts
        .keys()
        .find(|k| **k == wanted || k.ends_with(&format!("/{}", wanted)))
        .cloned()
    else {
        return Err(format!("{} holds no copy of {}.", package, file));
    };
    drafts.remove(&path);

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    let Some(user_dir) = user_dirs.first() else {
        return Err("No valid Zomboid user directories found.".to_string());
    };
    let draft_file = user_dir.join("mods").join(&package).join("draft_resolutions.json");
    let json = serde_json::to_string_pretty(&drafts).map_err(|e| e.to_string())?;
    fs::write(&draft_file, json).map_err(|e| e.to_string())?;

    let merged_files = drafts
        .into_values()
        .filter(|d| d.status == "RESOLVED")
        .map(|d| MergedFilePayload { relative_path: d.relative_path, content: d.resolved_content })
        .collect();
    generate_master_patch(MasterPatchRequest {
        workshop_dir: None,
        pz_install_dir: None,
        user_zomboid_dir: user_zomboid_dir.to_string(),
        mod_list_ini_path: mod_list_ini_path.to_string(),
        merged_files,
        package_folder_name: Some(package),
    })
}

/// Takes out of a package what its publish wrote -- the merged files, the shim, the switches and
/// the `media/` and `42/` copies of `mod.info` -- and leaves anything else that is in there, such
/// as a file placed by hand. Folders left empty go too. The top `mod.info`, the pictures, the
/// metadata and the drafts stay: the package is still there to reopen and publish again.
fn unpublish_in_place(dir: &Path) {
    for old in previously_written(dir) {
        remove_published_file(dir, &old);
    }
    for media in [dir.join("media"), dir.join("42").join("media")] {
        let shared = media.join("lua").join("shared");
        let _ = fs::remove_file(shared.join("Z_PZModStudio_Polyfills.lua"));
        let _ = fs::remove_file(shared.join("Z_PZModStudio_Patches.lua"));
        let _ = fs::remove_file(media.join("mod.info"));
    }
    for generated in ["mod.info", "poster.png", "icon.png"] {
        let _ = fs::remove_file(dir.join("42").join(generated));
    }
    for name in ["poster.png", "icon.png"] {
        let _ = fs::remove_file(dir.join("media").join(name));
    }
    for root in [dir.join("media"), dir.join("42")] {
        remove_empty_dirs(&root);
    }
}

/// Removes `dir` and the folders under it that hold no file.
fn remove_empty_dirs(dir: &Path) {
    let Ok(meta) = fs::symlink_metadata(dir) else { return };
    if !meta.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                remove_empty_dirs(&entry.path());
            }
        }
    }
    let _ = fs::remove_dir(dir); // fails, as it should, while something is left inside
}

/// Safely removes synthetic patch files for a specific package from disk and removes it from active load order.
pub fn clean_master_patch(req: MasterPatchRequest) -> Result<bool, String> {
    let target_folder_name = to_package_folder(
        req.package_folder_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Z_PZModStudio_MergedPatch"),
    )?;
    if super::is_system_folder(&target_folder_name) {
        return Err(format!("The companion mod '{}' cannot be cleaned.", BRIDGE_FOLDER));
    }

    let mut target_dirs = Vec::new();

    for user_dir in crate::load_order::mod_info::get_all_user_zomboid_dirs(&req.user_zomboid_dir) {
        target_dirs.push(user_dir.join("mods").join(&target_folder_name));
        target_dirs.push(user_dir.join("Lua").join("mods").join(&target_folder_name));
    }
    if let Some(ref install_dir) = req.pz_install_dir {
        if !install_dir.is_empty() {
            target_dirs.push(Path::new(install_dir).join("mods").join(&target_folder_name));
        }
    }

    for dir in target_dirs {
        if dir.exists() {
            unpublish_in_place(&dir);

            // Mark package_metadata.json as is_packaged = false (DRAFT mode)
            let meta_path = dir.join("patch_metadata.json");
            if meta_path.exists() {
                if let Ok(content) = fs::read_to_string(&meta_path) {
                    if let Ok(mut meta) = serde_json::from_str::<MasterPatchMetadata>(&content) {
                        meta.is_packaged = false;
                        if let Ok(new_json) = serde_json::to_string_pretty(&meta) {
                            let _ = fs::write(&meta_path, &new_json);
                        }
                    }
                }
            }
        }
    }

    if !req.mod_list_ini_path.is_empty() {
        if let Ok(mut mod_list) = read_mod_list_ini(&req.mod_list_ini_path) {
            mod_list.active_mods.retain(|id| id != &target_folder_name);
            let _ = write_mod_list_ini(&req.mod_list_ini_path, &mod_list.active_mods);
        }
    }

    Ok(true)
}

pub fn get_master_patch_status(user_zomboid_dir: &str, mod_list_ini_path: &str, package_folder_name: Option<String>) -> MasterPatchStatusInfo {
    let target_folder = package_folder_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Z_PZModStudio_MergedPatch".to_string());

    let clean_sub = target_folder
        .replace("PZ Mod Studio Patch: ", "")
        .replace("Z_PZModStudio_", "");
    let clean_sub = clean_sub.trim().to_string();

    let candidates = vec![
        target_folder.clone(),
        format!("Z_PZModStudio_{}", clean_sub),
        format!("Z_PZModStudio_{}", clean_sub.replace(' ', "_")),
    ];

    let all_user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    if all_user_dirs.is_empty() {
        return MasterPatchStatusInfo {
            is_packaged: false,
            created_at: None,
            packaged_mods: Vec::new(),
            missing_active_mods: Vec::new(),
            merged_files: Vec::new(),
        };
    }

    let mut meta_path = None;

    'outer: for user_dir in &all_user_dirs {
        for cand in &candidates {
            let p = user_dir.join("mods").join(cand).join("patch_metadata.json");
            if p.exists() {
                meta_path = Some(p);
                break 'outer;
            }
        }
    }

    let active_mods: Vec<String> = if !mod_list_ini_path.is_empty() {
        read_mod_list_ini(mod_list_ini_path)
            .map(|data| data.active_mods.into_iter().filter(|s| !candidates.contains(s)).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if let Some(ref m_path) = meta_path {
        if let Ok(content) = fs::read_to_string(m_path) {
            if let Ok(meta) = serde_json::from_str::<MasterPatchMetadata>(&content) {
                let missing: Vec<String> = active_mods
                    .iter()
                    .cloned()
                    .filter(|id| !meta.packaged_mod_ids.contains(id))
                    .collect();

                return MasterPatchStatusInfo {
                    is_packaged: meta.is_packaged,
                    created_at: Some(meta.created_at),
                    packaged_mods: meta.packaged_mod_ids,
                    merged_files: meta.merged_file_paths,
                    missing_active_mods: missing,
                };
            }
        }
    }

    MasterPatchStatusInfo {
        is_packaged: false,
        created_at: None,
        packaged_mods: Vec::new(),
        merged_files: Vec::new(),
        missing_active_mods: active_mods,
    }
}

#[cfg(test)]
mod live_mod_fix {
    /// Writes one corrected mod file into a package of its own, the way the Merger would.
    /// PZMS_ZOMBOID_DIR=... PZMS_FIX_FILE=<the corrected file> PZMS_FIX_PATH=media/...
    /// cargo test --release puts_a_fixed_mod_file_in_a_package -- --ignored --nocapture
    #[test]
    #[ignore]
    fn puts_a_fixed_mod_file_in_a_package() {
        let (Ok(zomboid), Ok(file), Ok(rel)) = (
            std::env::var("PZMS_ZOMBOID_DIR"),
            std::env::var("PZMS_FIX_FILE"),
            std::env::var("PZMS_FIX_PATH"),
        ) else {
            return;
        };
        let content = std::fs::read_to_string(&file).expect("the corrected file");
        let result = super::generate_master_patch(super::MasterPatchRequest {
            workshop_dir: std::env::var("PZMS_WORKSHOP_DIR").ok(),
            pz_install_dir: std::env::var("PZMS_PZ_DIR").ok(),
            user_zomboid_dir: zomboid.clone(),
            mod_list_ini_path: format!("{}/mods/default.txt", zomboid),
            merged_files: vec![super::MergedFilePayload { relative_path: rel.clone(), content }],
            package_folder_name: Some("Z_PZModStudio_ModFixes".to_string()),
        })
        .expect("the package is written");
        println!("package: {}", result.patch_mod_dir);
        println!("files written: {}", result.files_written);
        println!("shim: {}", result.polyfill_shim_written);
    }

    /// Puts one corrected mod file into a package the user already has, beside their merges:
    /// every resolved draft the package holds is published again, plus the fix, exactly what
    /// the Merger's publish sends. Nothing the package held is dropped.
    /// PZMS_ZOMBOID_DIR=... PZMS_PACKAGE=Z_PZModStudio_MergedPatch PZMS_FIX_FILE=<the corrected file> PZMS_FIX_PATH=media/...
    /// cargo test --release adds_a_fixed_mod_file_to_an_existing_package -- --ignored --nocapture
    #[test]
    #[ignore]
    fn adds_a_fixed_mod_file_to_an_existing_package() {
        let (Ok(zomboid), Ok(package), Ok(file), Ok(rel)) = (
            std::env::var("PZMS_ZOMBOID_DIR"),
            std::env::var("PZMS_PACKAGE"),
            std::env::var("PZMS_FIX_FILE"),
            std::env::var("PZMS_FIX_PATH"),
        ) else {
            return;
        };
        let content = std::fs::read_to_string(&file).expect("the corrected file");
        let result = super::publish_file_into_package(
            &zomboid,
            &format!("{}/mods/default.txt", zomboid),
            &package,
            &rel,
            &content,
        )
        .expect("the package is written");
        println!("package: {}", result.patch_mod_dir);
        println!("files written: {}", result.files_written);
    }
}

#[cfg(test)]
mod publish_keeps_files {
    use super::*;

    fn package_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pzms-publish-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("mods")).unwrap();
        dir
    }

    fn publish(zomboid: &Path, files: Vec<(&str, &str)>) -> MasterPatchResult {
        generate_master_patch(MasterPatchRequest {
            workshop_dir: None,
            pz_install_dir: None,
            user_zomboid_dir: zomboid.to_string_lossy().to_string(),
            mod_list_ini_path: zomboid.join("mods").join("default.txt").to_string_lossy().to_string(),
            merged_files: files
                .into_iter()
                .map(|(path, content)| MergedFilePayload {
                    relative_path: path.to_string(),
                    content: content.to_string(),
                })
                .collect(),
            package_folder_name: Some("Z_PZModStudio_Test".to_string()),
        })
        .expect("published")
    }

    /// One file into a package that already holds two: three come out, none lost.
    #[test]
    fn one_more_file_joins_what_the_package_already_holds() {
        let zomboid = package_dir("joins");
        let ini = zomboid.join("mods").join("default.txt");
        fs::write(&ini, "VERSION = 1,\n\nmods\n{\n    mod = A,\n}\n").unwrap();
        publish(&zomboid, vec![("media/lua/shared/a.lua", "a"), ("media/lua/shared/b.lua", "b")]);

        let result = publish_file_into_package(
            &zomboid.to_string_lossy(),
            &ini.to_string_lossy(),
            "Z_PZModStudio_Test",
            "media/lua/shared/c.lua",
            "c",
        )
        .expect("published");
        assert_eq!(result.files_written, 3);

        let pkg = zomboid.join("mods").join("Z_PZModStudio_Test");
        for name in ["a", "b", "c"] {
            let path = pkg.join("42").join("media").join("lua").join("shared").join(format!("{}.lua", name));
            assert_eq!(fs::read_to_string(&path).unwrap(), name, "{} is there", name);
        }
        let drafts = get_draft_resolutions(&zomboid.to_string_lossy(), "Z_PZModStudio_Test");
        assert_eq!(drafts.len(), 3);
        let _ = fs::remove_dir_all(&zomboid);
    }

    /// Taking one file out by its bare name leaves the others published and the draft gone.
    #[test]
    fn one_file_leaves_and_the_others_stay() {
        let zomboid = package_dir("leaves");
        let ini = zomboid.join("mods").join("default.txt");
        fs::write(&ini, "VERSION = 1,\n\nmods\n{\n    mod = A,\n}\n").unwrap();
        publish(&zomboid, vec![("media/lua/shared/a.lua", "a"), ("media/lua/client/x/b.lua", "b")]);

        let result = remove_file_from_package(
            &zomboid.to_string_lossy(),
            &ini.to_string_lossy(),
            "Z_PZModStudio_Test",
            "b.lua",
        )
        .expect("published without it");
        assert_eq!(result.files_written, 1);

        let pkg = zomboid.join("mods").join("Z_PZModStudio_Test");
        assert!(pkg.join("42").join("media").join("lua").join("shared").join("a.lua").is_file());
        assert!(!pkg.join("42").join("media").join("lua").join("client").join("x").join("b.lua").exists());
        assert!(!pkg.join("media").join("lua").join("client").join("x").join("b.lua").exists());
        let drafts = get_draft_resolutions(&zomboid.to_string_lossy(), "Z_PZModStudio_Test");
        assert_eq!(drafts.len(), 1);
        assert!(drafts.contains_key("media/lua/shared/a.lua"));

        assert!(remove_file_from_package(&zomboid.to_string_lossy(), &ini.to_string_lossy(), "Z_PZModStudio_Test", "nope.lua").is_err());
        let _ = fs::remove_dir_all(&zomboid);
    }

    #[test]
    fn a_second_publish_keeps_what_it_did_not_write_and_drops_what_it_did() {
        let zomboid = package_dir("keeps");
        publish(&zomboid, vec![("media/lua/shared/a.lua", "-- first")]);
        let pkg = zomboid.join("mods").join("Z_PZModStudio_Test");

        // Something else put a file in the package: a mod's own file, patched by hand.
        let byhand = pkg.join("media").join("lua").join("shared").join("byhand.lua");
        fs::create_dir_all(byhand.parent().unwrap()).unwrap();
        fs::write(&byhand, "-- patched by hand").unwrap();

        // A later publish of a different file.
        publish(&zomboid, vec![("media/lua/shared/b.lua", "-- second")]);

        // What it wrote now is there, what it wrote before is gone, and the hand-made file stays.
        assert!(pkg.join("media/lua/shared/b.lua").is_file());
        assert!(!pkg.join("media/lua/shared/a.lua").exists(), "the old merge should go");
        assert!(byhand.is_file(), "a file this publish did not write must survive");
        assert_eq!(fs::read_to_string(&byhand).unwrap(), "-- patched by hand");

        let _ = fs::remove_dir_all(&zomboid);
    }

    #[test]
    fn re_publishing_the_same_file_replaces_it() {
        let zomboid = package_dir("replaces");
        publish(&zomboid, vec![("media/lua/shared/a.lua", "-- first")]);
        publish(&zomboid, vec![("media/lua/shared/a.lua", "-- second")]);
        let pkg = zomboid.join("mods").join("Z_PZModStudio_Test");
        assert_eq!(fs::read_to_string(pkg.join("media/lua/shared/a.lua")).unwrap(), "-- second");
        let _ = fs::remove_dir_all(&zomboid);
    }
    /// The package keeps the name and description it was given, and the staging folder is gone
    /// once the publish is in place.
    #[test]
    fn publishing_keeps_the_description_and_leaves_no_staging_behind() {
        let zomboid = package_dir("describe");
        let pkg = zomboid.join("mods").join("Z_PZModStudio_Test");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("mod.info"), "name=My fixes\r\nid=Z_PZModStudio_Test\r\ndescription=Fixes for my server\r\n").unwrap();
        fs::write(pkg.join("poster.png"), "mine").unwrap();
        publish(&zomboid, vec![("media/lua/shared/a.lua", "a")]);

        let info = fs::read_to_string(pkg.join("mod.info")).unwrap();
        assert!(info.contains("name=My fixes") && info.contains("description=Fixes for my server"), "{}", info);
        assert_eq!(fs::read_to_string(pkg.join("42").join("mod.info")).unwrap(), info);
        assert_eq!(fs::read_to_string(pkg.join("poster.png")).unwrap(), "mine");
        let staging = zomboid.join("PZModStudio_Staging");
        assert!(fs::read_dir(&staging).map(|d| d.count() == 0).unwrap_or(true), "staging left behind");
        let _ = fs::remove_dir_all(&zomboid);
    }

    /// Unpack takes out what the publish wrote and nothing else.
    #[test]
    fn unpacking_keeps_files_placed_by_hand() {
        let zomboid = package_dir("unpack");
        let ini = zomboid.join("mods").join("default.txt");
        fs::write(&ini, "VERSION = 1,\n\nmods\n{\n    mod = A,\n}\n").unwrap();
        publish(&zomboid, vec![("media/lua/shared/a.lua", "a")]);
        let pkg = zomboid.join("mods").join("Z_PZModStudio_Test");
        let byhand = pkg.join("42").join("media").join("lua").join("client").join("byhand.lua");
        fs::create_dir_all(byhand.parent().unwrap()).unwrap();
        fs::write(&byhand, "-- by hand").unwrap();

        clean_master_patch(MasterPatchRequest {
            workshop_dir: None,
            pz_install_dir: None,
            user_zomboid_dir: zomboid.to_string_lossy().to_string(),
            mod_list_ini_path: ini.to_string_lossy().to_string(),
            merged_files: Vec::new(),
            package_folder_name: Some("Z_PZModStudio_Test".to_string()),
        })
        .unwrap();

        assert!(byhand.is_file(), "a hand-placed file must survive Unpack");
        assert!(!pkg.join("42").join("media").join("lua").join("shared").join("a.lua").exists());
        assert!(!pkg.join("media").join("lua").join("shared").exists(), "empty folders go");
        assert!(!pkg.join("42").join("mod.info").exists());
        assert!(pkg.join("mod.info").is_file() && pkg.join("draft_resolutions.json").is_file());
        assert!(!fs::read_to_string(&ini).unwrap().contains("Z_PZModStudio_Test"));
        let _ = fs::remove_dir_all(&zomboid);
    }
}


#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn a_publish_keeps_what_the_package_was_given_and_records_the_profile() {
        let before = r#"{"is_packaged": false, "is_visible_in_modlist": false, "description": "mine", "created_at": "1", "packaged_mod_ids": [], "merged_file_paths": []}"#;
        let meta = MasterPatchMetadata { is_packaged: true, created_at: "Epoch-2".into(), packaged_mod_ids: vec!["A".into()], merged_file_paths: vec!["media/x.lua".into()] };
        let after: serde_json::Value = serde_json::from_str(&metadata_after_publish(Some(before), &meta, Some(("coop", "Coop")))).unwrap();
        assert_eq!(after["is_visible_in_modlist"], false, "visibility kept");
        assert_eq!(after["description"], "mine", "description kept");
        assert_eq!(after["is_packaged"], true);
        assert_eq!(after["packaged_mod_ids"][0], "A");
        assert_eq!(after["built_for_profile_name"], "Coop");
        let again: serde_json::Value = serde_json::from_str(&metadata_after_publish(Some(&after.to_string()), &meta, None)).unwrap();
        assert!(again.get("built_for_profile_id").is_none(), "no profile active: none recorded");
        assert!(serde_json::from_value::<MasterPatchMetadata>(again).is_ok(), "the typed reader still reads it");
    }
}
