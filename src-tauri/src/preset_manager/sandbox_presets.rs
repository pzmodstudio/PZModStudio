//! The sandbox settings a shared mod list was played with.
//!
//! A `.pzpack` has always carried the mods and their order. It never carried the rules of the
//! game: how many zombies, how fast loot respawns, how long the power lasts — and, because the
//! game writes every mod's own options into the same file, the settings those mods added. Two
//! players running the same list under different sandbox settings are not playing the same game,
//! and the settings are the part people spend an evening tuning.
//!
//! The game keeps them as self-contained text in `Zomboid/Sandbox Presets/<name>.cfg`, which is
//! what goes into the pack and what is written back out. Nothing is overwritten on the way in: an
//! imported preset that clashes with one of yours is saved under a free name.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::load_order::mod_info::get_existing_user_zomboid_dirs;

/// One of the player's own sandbox presets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPresetInfo {
    /// What the game calls it, which is the file name without `.cfg`.
    pub name: String,
    pub path: String,
    pub bytes: u64,
    /// How many settings it sets.
    pub settings: usize,
    /// How many of those are a mod's own (`VTB.ServerValidation`, `Bandits.Something`), which only
    /// mean anything to someone who has that mod.
    pub mod_settings: usize,
}

/// A preset travelling inside a pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSandbox {
    pub name: String,
    /// The `.cfg` as the game wrote it.
    pub contents: String,
    pub settings: usize,
    pub mod_settings: usize,
}

fn presets_dir(zomboid: &Path) -> PathBuf {
    zomboid.join("Sandbox Presets")
}

/// `Key=value` lines, and how many of the keys are namespaced by a mod.
fn count_settings(text: &str) -> (usize, usize) {
    let mut settings = 0;
    let mut from_mods = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with("--") {
            continue;
        }
        let Some((key, _)) = trimmed.split_once('=') else { continue };
        let key = key.trim();
        if key.is_empty() || key == "Version" {
            continue;
        }
        settings += 1;
        if key.contains('.') {
            from_mods += 1;
        }
    }
    (settings, from_mods)
}

/// The player's sandbox presets, newest name order, across every Zomboid folder in use.
pub fn list(user_zomboid_dir: &str) -> Vec<SandboxPresetInfo> {
    let mut found: Vec<SandboxPresetInfo> = Vec::new();
    for zomboid in get_existing_user_zomboid_dirs(user_zomboid_dir) {
        let Ok(entries) = std::fs::read_dir(presets_dir(&zomboid)) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("cfg")) {
                continue;
            }
            let Some(name) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else { continue };
            if found.iter().any(|p| p.name == name) {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let (settings, mod_settings) = count_settings(&text);
            found.push(SandboxPresetInfo {
                name,
                bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
                path: path.to_string_lossy().to_string(),
                settings,
                mod_settings,
            });
        }
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found
}

/// Reads one of the player's presets for a pack to carry.
pub fn read_for_pack(user_zomboid_dir: &str, name: &str) -> Result<PresetSandbox, String> {
    let wanted = list(user_zomboid_dir)
        .into_iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("There is no sandbox preset called \"{}\".", name))?;
    let contents = std::fs::read_to_string(&wanted.path)
        .map_err(|e| format!("Cannot read {}: {}", wanted.path, e))?;
    let (settings, mod_settings) = count_settings(&contents);
    Ok(PresetSandbox { name: wanted.name, contents, settings, mod_settings })
}

/// A file name the game will accept, with nothing in it that could point elsewhere.
fn safe_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        "Imported".to_string()
    } else {
        cleaned.chars().take(60).collect()
    }
}

/// Writes a pack's preset into `Zomboid/Sandbox Presets`, never over one of the player's own.
/// Returns the name the game will show.
pub fn write_from_pack(user_zomboid_dir: &str, sandbox: &PresetSandbox) -> Result<String, String> {
    let zomboid = get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;
    let dir = presets_dir(&zomboid);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {}", dir.display(), e))?;

    let base = safe_name(&sandbox.name);
    let mut name = base.clone();
    let mut n = 2;
    while dir.join(format!("{}.cfg", name)).exists() {
        name = format!("{} ({})", base, n);
        n += 1;
        if n > 50 {
            return Err("Too many presets by that name already.".to_string());
        }
    }
    let path = dir.join(format!("{}.cfg", name));
    std::fs::write(&path, &sandbox.contents)
        .map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: &str = "Version=6\n\
                       # a comment\n\
                       Zombies=4\n\
                           -- 1 = Insane\n\
                       Distribution=1\n\
                       VTB.FurnitureCollision=true\n\
                       Bandits.Spawn=false\n";

    #[test]
    fn a_presets_settings_are_counted_and_the_mods_own_are_told_apart() {
        let (settings, from_mods) = count_settings(CFG);
        // Version is the format, not a setting; comments and the "-- 1 = Insane" notes are not either.
        assert_eq!(settings, 4);
        assert_eq!(from_mods, 2);
    }

    #[test]
    fn importing_never_writes_over_a_preset_of_your_own() {
        let dir = std::env::temp_dir().join(format!("pzms-sandbox-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Sandbox Presets")).unwrap();
        let zomboid = dir.to_string_lossy().to_string();
        std::fs::write(dir.join("Sandbox Presets").join("Mine.cfg"), "Version=6\nZombies=1\n").unwrap();

        // The listing covers every Zomboid folder in use, so this machine's own presets may be
        // in it too; what matters is the one just written.
        let mine = |z: &str| list(z).into_iter().find(|p| p.name.starts_with("Mine"));
        let listed = mine(&zomboid).expect("the preset is listed");
        assert_eq!(listed.name, "Mine");
        assert_eq!(listed.settings, 1);

        let packed = read_for_pack(&zomboid, "Mine").unwrap();
        assert!(packed.contents.contains("Zombies=1"));

        // The same name arriving from a pack lands beside it, not on it.
        let written = write_from_pack(&zomboid, &packed).unwrap();
        assert_eq!(written, "Mine (2)");
        assert_eq!(
            std::fs::read_to_string(dir.join("Sandbox Presets").join("Mine.cfg")).unwrap(),
            "Version=6\nZombies=1\n"
        );
        assert!(list(&zomboid).iter().any(|p| p.name == "Mine (2)"));

        // A name with a path in it cannot escape the folder.
        let nasty = PresetSandbox {
            name: "../../evil".to_string(),
            contents: "Version=6\n".to_string(),
            settings: 0,
            mod_settings: 0,
        };
        let written = write_from_pack(&zomboid, &nasty).unwrap();
        assert_eq!(written, "______evil");
        assert!(dir.join("Sandbox Presets").join("______evil.cfg").is_file());

        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// Reads the real presets on this machine and packs one.
    /// PZMS_ZOMBOID_DIR=... cargo test the_real_sandbox_presets -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_real_sandbox_presets() {
        let Ok(zomboid) = std::env::var("PZMS_ZOMBOID_DIR") else { return };
        let found = list(&zomboid);
        println!("{} sandbox preset(s):", found.len());
        for p in &found {
            println!(
                "  {:<30} {:>8} bytes  {} settings ({} from mods)",
                p.name, p.bytes, p.settings, p.mod_settings
            );
        }
        if let Some(first) = found.first() {
            let packed = read_for_pack(&zomboid, &first.name).expect("it reads");
            println!(
                "packed \"{}\": {} characters, {} settings",
                packed.name,
                packed.contents.len(),
                packed.settings
            );
            assert_eq!(packed.settings, first.settings);
        }
    }
}
