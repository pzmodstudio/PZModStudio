//! The Build 42 shim every package carries, its parts as Settings lists them, and the switches of its per-mod patches.

use super::*;

/// The canonical copy lives in mods/Z_PZModStudio_Bridge; it is compiled in so the
/// installer and generated packages always ship exactly what is in the repo.
pub const MASTER_POLYFILL_LUA: &str =
    include_str!("../../../mods/Z_PZModStudio_Bridge/media/lua/shared/Z_PZModStudio_Polyfills.lua");

pub fn generate_master_polyfill_lua() -> String {
    MASTER_POLYFILL_LUA.to_string()
}

/// Puts this build's shim into every Studio package in `Zomboid/mods` whose copy differs
/// (`Z_PZModStudio_*/.../Z_PZModStudio_Polyfills.lua`, wherever a publish put it). A package
/// carries the shim of the Studio that published it, and a shim fix used to wait for a republish
/// that nobody knew to make. Returns the files rewritten. Only existing copies are touched.
pub fn refresh_installed_polyfills(user_zomboid_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut written = Vec::new();
    let Ok(entries) = std::fs::read_dir(user_zomboid_dir.join("mods")) else { return written };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("Z_PZModStudio_") || !entry.path().is_dir() {
            continue;
        }
        for file in walkdir::WalkDir::new(entry.path()).max_depth(6).into_iter().flatten() {
            if file.file_name() != "Z_PZModStudio_Polyfills.lua" || !file.file_type().is_file() {
                continue;
            }
            let current = std::fs::read_to_string(file.path()).unwrap_or_default();
            if current != MASTER_POLYFILL_LUA && std::fs::write(file.path(), MASTER_POLYFILL_LUA).is_ok() {
                written.push(file.path().to_path_buf());
            }
        }
    }
    written
}

/// One numbered part of the shim, as the shim itself heads it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyfillSection {
    /// "4.1", as written in the file.
    pub number: String,
    pub title: String,
    /// Lines of Lua under this heading, so a part that does nothing cannot look like one that does.
    pub lines: usize,
}

/// What the shim covers, read out of the shim that ships.
///
/// This list used to be written by hand in the app, which let it drift: it promised fixes the Lua
/// never made, one of them redirecting `require("ISUI/ISPanel")` to a file Build 42 does not have.
/// Reading the headings of the file that is actually written into the package cannot drift.
pub fn polyfill_sections() -> Vec<PolyfillSection> {
    let mut sections: Vec<PolyfillSection> = Vec::new();
    for line in MASTER_POLYFILL_LUA.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("-- ") else {
            if let Some(last) = sections.last_mut() {
                if !trimmed.is_empty() {
                    last.lines += 1;
                }
            }
            continue;
        };
        // A heading is "-- 4.1 Title" or "-- 4. Title"; anything else is an ordinary comment.
        let (number, title) = match rest.split_once(' ') {
            Some((number, title))
                if number.starts_with(|c: char| c.is_ascii_digit())
                    && number.trim_end_matches('.').chars().all(|c| c.is_ascii_digit() || c == '.') =>
            {
                (number.trim_end_matches('.').to_string(), title.trim().to_string())
            }
            _ => continue,
        };
        if title.is_empty() {
            continue;
        }
        // The file heads some parts twice; the first heading wins.
        if sections.iter().any(|x| x.number == number) {
            continue;
        }
        sections.push(PolyfillSection { number, title, lines: 0 });
    }
    sections
}

/// One per-mod patch of the shim's part 7, as `Poly.patchDefaults` lists it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShimPatch {
    /// `fancy_handwork`: the key the shim switches on.
    pub name: String,
    /// The comment beside it in the shim: "Fancy Handwork multiplayer bounds".
    pub label: String,
    pub enabled: bool,
}

/// The file the switches live in, beside the mod list's backups.
const PATCH_SWITCHES_FILE: &str = "PZModStudio_Patches.json";
/// The Lua every package carries so the shim reads the switches: `Z_PZModStudio_Patches.lua`
/// sorts before `Z_PZModStudio_Polyfills.lua`, and the shim reads `PZMS_PATCHES` at boot anyway.
const PATCH_SWITCHES_LUA: &str = "Z_PZModStudio_Patches.lua";

/// The per-mod patches, read out of the shim that ships: the `name = true, -- label` lines of
/// `Poly.patchDefaults`. Reading the file cannot drift from the file.
pub fn shim_patch_names() -> Vec<(String, String)> {
    let mut names = Vec::new();
    let mut inside = false;
    for line in MASTER_POLYFILL_LUA.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Poly.patchDefaults = {") {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if trimmed.starts_with('}') {
            break;
        }
        let Some((name, rest)) = trimmed.split_once('=') else { continue };
        let label = rest.split_once("--").map(|(_, l)| l.trim()).unwrap_or("").to_string();
        names.push((name.trim().to_string(), label));
    }
    names
}

/// The switches as the player left them; a patch not in the file is on.
pub fn read_patch_switches(user_zomboid_dir: &str) -> std::collections::BTreeMap<String, bool> {
    crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir)
        .first()
        .and_then(|dir| fs::read_to_string(dir.join(PATCH_SWITCHES_FILE)).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Every per-mod patch with its current state.
pub fn shim_patches(user_zomboid_dir: &str) -> Vec<ShimPatch> {
    let switches = read_patch_switches(user_zomboid_dir);
    shim_patch_names()
        .into_iter()
        .map(|(name, label)| ShimPatch { enabled: switches.get(&name).copied().unwrap_or(true), name, label })
        .collect()
}

/// The Lua that hands the switches to the shim. Only what is off is written: on is the default.
pub fn patch_switches_lua(switches: &std::collections::BTreeMap<String, bool>) -> String {
    let mut lua = String::from(
        "-- Written by PZ Mod Studio (Settings > Build 42 shim). Which of the shim's per-mod patches are off.\n",
    );
    lua.push_str("PZMS_PATCHES = {\n");
    for (name, on) in switches {
        if !on && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            lua.push_str(&format!("    {} = false,\n", name));
        }
    }
    lua.push_str("}\n");
    lua
}

/// Writes the switches file into one package, in both layouts the package keeps.
pub(crate) fn write_patch_switches_into(package_dir: &Path, lua: &str) {
    for root in [package_dir.join("media"), package_dir.join("42").join("media")] {
        let dir = root.join("lua").join("shared");
        if dir.is_dir() {
            let _ = fs::write(dir.join(PATCH_SWITCHES_LUA), lua);
        }
    }
}

/// Switches one patch and puts the switches into every Studio package on disk, so the game reads
/// them on its next launch without a publish. Returns how many packages were written.
pub fn set_patch_switch(user_zomboid_dir: &str, name: &str, enabled: bool) -> Result<usize, String> {
    if !shim_patch_names().iter().any(|(n, _)| n == name) {
        return Err(format!("The shim has no patch called {}.", name));
    }
    let dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    let Some(user_dir) = dirs.first() else {
        return Err("No valid Zomboid user directories found.".to_string());
    };
    let mut switches = read_patch_switches(user_zomboid_dir);
    switches.insert(name.to_string(), enabled);
    let json = serde_json::to_string_pretty(&switches).map_err(|e| e.to_string())?;
    fs::write(user_dir.join(PATCH_SWITCHES_FILE), json).map_err(|e| e.to_string())?;

    let lua = patch_switches_lua(&switches);
    let mut written = 0;
    if let Ok(entries) = fs::read_dir(user_dir.join("mods")) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_studio = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("Z_PZModStudio_"))
                .unwrap_or(false);
            if is_studio && path.is_dir() {
                write_patch_switches_into(&path, &lua);
                written += 1;
            }
        }
    }
    Ok(written)
}

#[cfg(test)]
mod shim_tests {
    use super::*;

    #[test]
    fn the_shims_own_headings_are_what_the_app_lists() {
        let sections = polyfill_sections();
        assert!(sections.len() >= 8, "found only {:?}", sections);
        assert_eq!(sections[0].number, "1");
        assert!(sections[0].title.contains("Translator"), "{}", sections[0].title);
        // Numbers are unique and each part has some Lua under it.
        let mut numbers: Vec<&str> = sections.iter().map(|s| s.number.as_str()).collect();
        let before = numbers.len();
        numbers.sort();
        numbers.dedup();
        assert_eq!(numbers.len(), before, "a number is listed twice");
        assert!(sections.iter().all(|s| s.lines > 0), "{:?}", sections.iter().find(|s| s.lines == 0));
    }

    #[test]
    fn a_package_no_longer_takes_the_games_own_sort_button_away() {
        // The shim is Lua the game runs; nothing in the package replaces a vanilla screen.
        assert!(!MASTER_POLYFILL_LUA.contains("ModLoadOrderUI"));
    }
}

#[cfg(test)]
mod refresh_tests {
    #[test]
    fn a_package_with_an_older_shim_gets_this_builds_copy_and_a_current_one_is_left_alone() {
        let z = std::env::temp_dir().join(format!("pz-shim-refresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&z);
        let old = z.join("mods").join("Z_PZModStudio_MergedPatch").join("42").join("media").join("lua").join("shared");
        let same = z.join("mods").join("Z_PZModStudio_Bridge").join("media").join("lua").join("shared");
        let other = z.join("mods").join("SomeMod").join("media").join("lua").join("shared");
        for d in [&old, &same, &other] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join("Z_PZModStudio_Polyfills.lua"), "-- an older shim
").unwrap();
        }
        std::fs::write(same.join("Z_PZModStudio_Polyfills.lua"), super::MASTER_POLYFILL_LUA).unwrap();
        let written = super::refresh_installed_polyfills(&z);
        assert_eq!(written, vec![old.join("Z_PZModStudio_Polyfills.lua")]);
        assert_eq!(std::fs::read_to_string(old.join("Z_PZModStudio_Polyfills.lua")).unwrap(), super::MASTER_POLYFILL_LUA);
        // Not a Studio package: not touched.
        assert_eq!(std::fs::read_to_string(other.join("Z_PZModStudio_Polyfills.lua")).unwrap(), "-- an older shim
");
        assert!(super::refresh_installed_polyfills(&z).is_empty());
        let _ = std::fs::remove_dir_all(&z);
    }
}

#[cfg(test)]
mod shim_listing {
    /// The shim that ships has to parse: a stray `end` in two thousand lines of Lua would take
    /// every package down with it, silently, at the game's boot.
    #[test]
    fn the_shim_is_valid_lua() {
        let check = crate::diff_engine::lua::validate_lua_syntax(super::MASTER_POLYFILL_LUA);
        assert!(check.is_valid, "{:?}", check.error_message);
    }

    /// Parts 5, 6 and 7 are the bridge, the vanilla guards and the per-mod patches; each has to
    /// be there, head its own code, and part 7 has to switch every patch it holds.
    #[test]
    fn the_shim_has_its_three_late_parts() {
        let sections = super::polyfill_sections();
        let find = |n: &str| sections.iter().find(|s| s.number == n).unwrap_or_else(|| panic!("part {} missing", n));
        assert!(find("5").title.contains("Build 41"), "{}", find("5").title);
        assert!(find("6").title.contains("Vanilla"), "{}", find("6").title);
        assert!(find("7").title.contains("Per-mod"), "{}", find("7").title);
        for n in ["5", "6", "7"] {
            assert!(find(n).lines > 100, "part {} has {} lines", n, find(n).lines);
        }
        let switched = super::MASTER_POLYFILL_LUA.matches("Poly.patchOn(").count();
        assert!(switched >= 13, "{} per-mod blocks are switched", switched);
    }

    /// The per-mod patches Settings lists come from the shim's own switch table.
    #[test]
    fn the_shims_per_mod_patches_are_read_from_its_switch_table() {
        let names = super::shim_patch_names();
        assert!(names.len() >= 9, "{:?}", names);
        let (name, label) = names.iter().find(|(n, _)| n == "fancy_handwork").expect("fancy_handwork");
        assert_eq!(name, "fancy_handwork");
        assert!(label.contains("Fancy Handwork"), "{}", label);
        for (n, _) in &names {
            assert!(super::MASTER_POLYFILL_LUA.contains(&format!("Poly.patchOn(\"{}\"", n)), "{} is switched in part 7", n);
        }
    }

    /// Only what is off goes into the Lua: on is the shim's default.
    #[test]
    fn the_switches_lua_names_only_what_is_off() {
        let mut switches = std::collections::BTreeMap::new();
        switches.insert("fancy_handwork".to_string(), false);
        switches.insert("sauced_carts".to_string(), true);
        switches.insert("bad name; os.exit()".to_string(), false);
        let lua = super::patch_switches_lua(&switches);
        assert!(lua.contains("    fancy_handwork = false,"));
        assert!(!lua.contains("sauced_carts"));
        assert!(!lua.contains("os.exit"));
        assert!(crate::diff_engine::lua::validate_lua_syntax(&lua).is_valid);
    }

    /// Prints the list the Settings card now shows.
    /// cargo test what_the_shim_covers -- --ignored --nocapture
    #[test]
    #[ignore]
    fn what_the_shim_covers() {
        for s in super::polyfill_sections() {
            println!("  {:>4}  {:<70} {} lines", s.number, s.title, s.lines);
        }
    }
}
