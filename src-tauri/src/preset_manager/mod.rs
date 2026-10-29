//! Sharing a mod list as a `.pzpack`, and telling the other player how their copy differs.
//!
//! A list of mod ids is enough to switch the same mods on, and not enough to get the same game.
//! Mods change: a recipe moves, an item is renamed, a map is redrawn. "It works on mine" usually
//! means the two machines have different versions of the same list. So a pack also records, for
//! every mod, the version its `mod.info` claimed and the publish time of the Workshop copy the
//! exporter actually had, plus the game build it was made on — and on import those are compared
//! with what is here, which is the whole point of writing them down.
//!
//! Nothing is downgraded or forced: Steam gives everyone the newest copy and the Studio does not
//! fight it. The pack says what the list was tested with, and the import says where you differ.

pub mod sandbox_presets;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetModEntry {
    pub mod_id: String,
    pub name: String,
    pub workshop_id: Option<String>,
    pub enabled: bool,
    /// What the exporter's `mod.info` claimed (`modversion=`), when it said anything.
    #[serde(default)]
    pub version: Option<String>,
    /// Publish time (unix seconds) of the Workshop copy the exporter had on disk.
    #[serde(default)]
    pub updated_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModPreset {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub created_at: String,
    pub mods: Vec<PresetModEntry>,
    pub load_order: Vec<String>,
    /// The game build the pack was made on, as `Zomboid/version.txt` spells it.
    #[serde(default)]
    pub game_version: Option<String>,
    /// The sandbox settings the sender played this list with, when they chose to include them.
    #[serde(default)]
    pub sandbox: Option<sandbox_presets::PresetSandbox>,
}

/// One mod whose copy here is not the copy the pack was made with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetModDrift {
    pub mod_id: String,
    pub name: String,
    pub pack_version: Option<String>,
    pub your_version: Option<String>,
    pub pack_updated_at: Option<u64>,
    pub your_updated_at: Option<u64>,
    /// "older": your copy was published before the pack's. "newer": after it. "different": only
    /// the version in `mod.info` differs, which says nothing about which came first.
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingModsReport {
    pub missing_mods: Vec<PresetModEntry>,
    pub installed_count: usize,
    pub total_count: usize,
    /// Installed, but not the same copy the pack was made with.
    #[serde(default)]
    pub drift: Vec<PresetModDrift>,
    /// The build the pack was made on, when it recorded one, and the build here.
    #[serde(default)]
    pub pack_game_version: Option<String>,
    #[serde(default)]
    pub your_game_version: Option<String>,
}

/// Writes the pack, filling in what the sender's own install knows: each mod's version, the
/// publish time of the copy on disk, and the game build. The caller gives the list; the facts
/// about it are read here, so a pack cannot claim a version nobody had.
#[tauri::command(async)]
pub fn export_preset_file(
    paths: crate::vfs::StudioPaths,
    preset: ModPreset,
    file_path: String,
    sandbox_preset: Option<String>,
) -> Result<ModPreset, String> {
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let mut stamped = preset;
    stamped.game_version = catalog.game_version.clone();
    // The settings are read here too, so a pack cannot carry a preset the sender does not have.
    stamped.sandbox = match sandbox_preset.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        Some(name) => Some(sandbox_presets::read_for_pack(&paths.user_zomboid_dir, name)?),
        None => None,
    };
    for entry in &mut stamped.mods {
        if let Some(m) = catalog.resolve(&entry.mod_id) {
            entry.version = m.info.mod_version.clone();
            entry.updated_at = m.workshop_state.as_ref().and_then(|s| s.installed_time);
        }
    }
    let json = serde_json::to_string_pretty(&stamped).map_err(|e| e.to_string())?;
    fs::write(Path::new(&file_path), json)
        .map_err(|e| format!("Cannot write {}: {}", file_path, e))?;
    Ok(stamped)
}

#[tauri::command(async)]
pub fn import_preset_file(file_path: String) -> Result<ModPreset, String> {
    let content = fs::read_to_string(Path::new(&file_path))
        .map_err(|e| format!("Cannot read {}: {}", file_path, e))?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid .pzpack format: {}", e))
}

/// What is missing for this pack, and where what you have is not what it was made with.
#[tauri::command(async)]
pub fn check_missing_preset_mods(
    preset: ModPreset,
    user_zomboid_dir: String,
    workshop_dir: String,
) -> Result<MissingModsReport, String> {
    let catalog = crate::catalog::build_catalog(&workshop_dir, &user_zomboid_dir, None);
    Ok(compare_with_installed(&preset, &catalog))
}

/// The comparison itself, so it can be tested without an install.
pub fn compare_with_installed(
    preset: &ModPreset,
    catalog: &crate::catalog::ModCatalog,
) -> MissingModsReport {
    let mut missing = Vec::new();
    let mut drift = Vec::new();

    for entry in &preset.mods {
        let Some(m) = catalog.resolve(&entry.mod_id) else {
            missing.push(entry.clone());
            continue;
        };
        let yours_updated = m.workshop_state.as_ref().and_then(|s| s.installed_time);
        let yours_version = m.info.mod_version.clone();

        // Publish times are the only thing that can be ordered: a version string is whatever the
        // author felt like writing.
        let direction = match (entry.updated_at, yours_updated) {
            (Some(pack), Some(yours)) if yours < pack => Some("older"),
            (Some(pack), Some(yours)) if yours > pack => Some("newer"),
            (Some(_), Some(_)) => None,
            _ => match (&entry.version, &yours_version) {
                (Some(pack), Some(yours)) if pack != yours => Some("different"),
                _ => None,
            },
        };
        if let Some(direction) = direction {
            drift.push(PresetModDrift {
                mod_id: m.id.clone(),
                name: m.name.clone(),
                pack_version: entry.version.clone(),
                your_version: yours_version,
                pack_updated_at: entry.updated_at,
                your_updated_at: yours_updated,
                direction: direction.to_string(),
            });
        }
    }

    let total = preset.mods.len();
    MissingModsReport {
        installed_count: total - missing.len(),
        missing_mods: missing,
        total_count: total,
        drift,
        pack_game_version: preset.game_version.clone(),
        your_game_version: catalog.game_version.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{CatalogMod, Loadability, ModCatalog, ModSource};
    use crate::catalog::modinfo::GameModInfo;

    fn installed(id: &str, version: Option<&str>, published: Option<u64>) -> CatalogMod {
        CatalogMod {
            id: id.to_string(),
            name: id.to_string(),
            source: ModSource::Workshop { workshop_id: "1".to_string() },
            root_dir: String::new(),
            mod_info_path: None,
            version_folders: Vec::new(),
            has_common: false,
            has_root_mod_info: true,
            loadability: Loadability::Loadable,
            shadowed_by: None,
            origin: None,
            workshop_state: published.map(|t| crate::workshop::InstalledItemState {
                installed_time: Some(t),
                latest_time: Some(t),
                size: None,
                update_pending: false,
                updated_since_last_play: false,
            }),
            info: GameModInfo {
                id: Some(id.to_string()),
                mod_version: version.map(str::to_string),
                ..Default::default()
            },
        }
    }

    fn entry(id: &str, version: Option<&str>, published: Option<u64>) -> PresetModEntry {
        PresetModEntry {
            mod_id: id.to_string(),
            name: id.to_string(),
            workshop_id: Some("1".to_string()),
            enabled: true,
            version: version.map(str::to_string),
            updated_at: published,
        }
    }

    fn pack(mods: Vec<PresetModEntry>, game: Option<&str>) -> ModPreset {
        ModPreset {
            id: "p".into(),
            name: "p".into(),
            description: None,
            author: None,
            created_at: "now".into(),
            load_order: mods.iter().map(|m| m.mod_id.clone()).collect(),
            mods,
            game_version: game.map(str::to_string),
            sandbox: None,
        }
    }

    #[test]
    fn an_older_copy_a_newer_one_and_a_matching_one_are_told_apart() {
        let catalog = ModCatalog {
            game_version: Some("42.21".into()),
            mods: vec![
                installed("Same", Some("1.0"), Some(1_000)),
                installed("Older", Some("1.0"), Some(900)),
                installed("Newer", Some("2.0"), Some(1_100)),
            ],
        };
        let report = compare_with_installed(
            &pack(
                vec![
                    entry("Same", Some("1.0"), Some(1_000)),
                    entry("Older", Some("1.0"), Some(1_000)),
                    entry("Newer", Some("1.0"), Some(1_000)),
                    entry("Gone", Some("1.0"), Some(1_000)),
                ],
                Some("42.20"),
            ),
            &catalog,
        );
        assert_eq!(report.total_count, 4);
        assert_eq!(report.installed_count, 3);
        assert_eq!(report.missing_mods[0].mod_id, "Gone");
        let by_id: Vec<(&str, &str)> =
            report.drift.iter().map(|d| (d.mod_id.as_str(), d.direction.as_str())).collect();
        assert_eq!(by_id, vec![("Older", "older"), ("Newer", "newer")]);
        assert_eq!(report.pack_game_version.as_deref(), Some("42.20"));
        assert_eq!(report.your_game_version.as_deref(), Some("42.21"));
    }

    #[test]
    fn without_publish_times_only_the_version_string_is_left_to_compare() {
        let catalog = ModCatalog {
            game_version: None,
            mods: vec![installed("Local", Some("2.0"), None)],
        };
        let report = compare_with_installed(&pack(vec![entry("Local", Some("1.0"), None)], None), &catalog);
        assert_eq!(report.drift.len(), 1);
        assert_eq!(report.drift[0].direction, "different");
    }

    #[test]
    fn a_pack_from_before_this_said_nothing_about_versions_and_is_still_read() {
        // No `version`, `updated_at` or `game_version`: the old format.
        let old = r#"{"id":"p","name":"p","description":null,"author":null,
            "created_at":"now","mods":[{"mod_id":"A","name":"A","workshop_id":null,"enabled":true}],
            "load_order":["A"]}"#;
        let preset: ModPreset = serde_json::from_str(old).expect("the old format still reads");
        assert!(preset.game_version.is_none());
        assert!(preset.mods[0].version.is_none());

        let catalog = ModCatalog { game_version: Some("42.21".into()), mods: vec![installed("A", Some("1.0"), Some(5))] };
        let report = compare_with_installed(&preset, &catalog);
        // Nothing to compare against, so nothing is claimed.
        assert!(report.drift.is_empty());
        assert_eq!(report.installed_count, 1);
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// Makes a pack from the real active list, then reads it back as another player would.
    /// PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=...
    /// cargo test --release a_real_pack_records_what_it_was_made_with -- --ignored --nocapture
    #[test]
    #[ignore]
    fn a_real_pack_records_what_it_was_made_with() {
        let (Ok(workshop), Ok(zomboid)) = (
            std::env::var("PZMS_WORKSHOP_DIR"),
            std::env::var("PZMS_ZOMBOID_DIR"),
        ) else {
            return;
        };
        let catalog = crate::catalog::build_catalog(&workshop, &zomboid, None);
        let active = crate::load_order::ini_parser::read_mod_list_ini(&format!("{}/mods/default.txt", zomboid))
            .map(|d| d.active_mods)
            .unwrap_or_default();

        // The pack as the export command builds it.
        let mut mods: Vec<PresetModEntry> = Vec::new();
        for entry in &active {
            if let Some(m) = catalog.resolve(entry) {
                mods.push(PresetModEntry {
                    mod_id: m.id.clone(),
                    name: m.name.clone(),
                    workshop_id: None,
                    enabled: true,
                    version: m.info.mod_version.clone(),
                    updated_at: m.workshop_state.as_ref().and_then(|s| s.installed_time),
                });
            }
        }
        let pack = ModPreset {
            id: "live".into(),
            name: "live".into(),
            description: None,
            author: None,
            created_at: "now".into(),
            load_order: mods.iter().map(|m| m.mod_id.clone()).collect(),
            mods,
            game_version: catalog.game_version.clone(),
            sandbox: None,
        };
        let with_version = pack.mods.iter().filter(|m| m.version.is_some()).count();
        let with_time = pack.mods.iter().filter(|m| m.updated_at.is_some()).count();
        println!(
            "pack: {} mods on build {:?}; {} record a version, {} a publish time",
            pack.mods.len(),
            pack.game_version,
            with_version,
            with_time
        );

        // Read back on this very machine: nothing should differ.
        let same = compare_with_installed(&pack, &catalog);
        println!("same machine: {} missing, {} differing", same.missing_mods.len(), same.drift.len());
        assert!(same.missing_mods.is_empty());
        assert!(same.drift.is_empty(), "{:?}", same.drift);

        // Now as another player whose copies are a day older, and who is missing one mod.
        let mut elsewhere = pack.clone();
        for m in &mut elsewhere.mods {
            m.updated_at = m.updated_at.map(|t| t + 86_400);
        }
        elsewhere.mods.push(PresetModEntry {
            mod_id: "AMotherNobodyHas".into(),
            name: "A mod nobody has".into(),
            workshop_id: Some("1".into()),
            enabled: true,
            version: None,
            updated_at: None,
        });
        let report = compare_with_installed(&elsewhere, &catalog);
        println!(
            "as another player: {} missing, {} differing ({} say yours is older)",
            report.missing_mods.len(),
            report.drift.len(),
            report.drift.iter().filter(|d| d.direction == "older").count()
        );
        assert_eq!(report.missing_mods.len(), 1);
        assert_eq!(report.drift.len(), with_time);
        assert!(report.drift.iter().all(|d| d.direction == "older"));
    }
}
