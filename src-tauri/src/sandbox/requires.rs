//! Where a module a mod asked for actually lives.
//!
//! `require("Vehicle/Vehicles") failed` is the commonest complaint in a Build 42 log, and on its
//! own it tells you nothing you can act on: the mod wanted a Lua module and the game did not find
//! it there. Build 42 moved a great deal of Lua about, so most of the time the module is still
//! there under another path — `Vehicles/Vehicles`, in that case — and the mod can be patched to
//! ask for the right one. Sometimes it is in another mod. Sometimes it is nowhere, and then the
//! mod needs its author or a dependency nobody has installed.
//!
//! Which of the three it is can be looked up, and that is the difference between a warning and
//! something a person can do. So the game's Lua and every active mod's Lua are indexed by module
//! name, and the answer is whichever of the three it turns out to be.

use serde::{Deserialize, Serialize};
use std::path::Path;

use walkdir::WalkDir;

/// A module that exists, and where.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCandidate {
    /// The path a `require` would name: `Vehicles/ISUI/ISVehiclePartMenu`.
    pub module_path: String,
    /// `client`, `server` or `shared`: the game keeps the three apart, and a file in one cannot
    /// require a module in another.
    pub phase: String,
    /// "the game" or the mod's name.
    pub owner: String,
    /// True when it sits at exactly the path that failed, which means the file is there and
    /// something else went wrong — usually the mod that owns it loading later.
    pub same_path: bool,
}

/// What became of one failed `require`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequireAnswer {
    /// What the mod asked for.
    pub wanted: String,
    /// Where the module turned up, nearest spelling first. Empty when it is nowhere.
    pub candidates: Vec<ModuleCandidate>,
    /// Said plainly, for the card.
    pub summary: String,
}

/// `.../media/lua/<client|server|shared>/Some/Path.lua` -> `Some/Path`, which is how `require`
/// names it. Anything outside those three folders is not requirable and is left out.
fn module_path_of(file: &Path) -> Option<(String, String)> {
    let text = file.to_string_lossy().replace('\\', "/").to_lowercase();
    let at = text.find("/media/lua/")?;
    let rest = &text[at + "/media/lua/".len()..];
    let (phase, tail) = rest.split_once('/')?;
    if !matches!(phase, "client" | "server" | "shared") {
        return None;
    }
    let tail = tail.strip_suffix(".lua")?;
    // Keep the real spelling, not the lowercased one used for finding it.
    let original = file.to_string_lossy().replace('\\', "/");
    let start = original.len() - tail.len() - 4;
    Some((original[start..original.len() - 4].to_string(), phase.to_string()))
}

pub(crate) fn index_lua(root: &Path, owner: &str, into: &mut Vec<(String, ModuleCandidate)>) {
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if !entry.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("lua")) {
            continue;
        }
        let Some((module_path, phase)) = module_path_of(entry.path()) else { continue };
        let key = module_path
            .rsplit('/')
            .next()
            .unwrap_or(&module_path)
            .to_lowercase();
        into.push((
            key,
            ModuleCandidate { module_path, phase, owner: owner.to_string(), same_path: false },
        ));
    }
}

/// Everything requirable in the game and in the active mods, by module name.
pub fn index_modules(
    paths: &crate::vfs::StudioPaths,
    catalog: &crate::catalog::ModCatalog,
    active: &[String],
) -> Vec<(String, ModuleCandidate)> {
    let mut index = Vec::new();
    index_lua(&Path::new(&paths.pz_install_dir).join("media").join("lua"), "the game", &mut index);

    let game = catalog.game_version_int();
    let mut seen = std::collections::HashSet::new();
    for entry in active {
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if !seen.insert(m.id.clone()) {
            continue;
        }
        for layer in crate::load_order::mod_info::media_layers_for_build(Path::new(&m.root_dir), game) {
            index_lua(&layer.join("media").join("lua"), &m.name, &mut index);
        }
    }
    index
}

/// Where the module behind one failed `require` really is.
pub fn answer_for(wanted: &str, index: &[(String, ModuleCandidate)]) -> RequireAnswer {
    let wanted_clean = wanted.trim().trim_matches('"').replace('\\', "/");
    let key = wanted_clean.rsplit('/').next().unwrap_or(&wanted_clean).to_lowercase();

    let mut candidates: Vec<ModuleCandidate> = Vec::new();
    for (name, candidate) in index {
        if *name != key {
            continue;
        }
        let same_path = candidate.module_path.eq_ignore_ascii_case(&wanted_clean);
        if candidates
            .iter()
            .any(|c| c.module_path == candidate.module_path && c.owner == candidate.owner)
        {
            continue;
        }
        candidates.push(ModuleCandidate { same_path, ..candidate.clone() });
    }
    // The one at the same path first: it says the module is there and the problem is elsewhere.
    candidates.sort_by_key(|c| !c.same_path);

    let summary = match candidates.first() {
        None => format!(
            "No module called {} exists in the game or in any mod you have switched on. The mod \
             is asking for something that is not there: either it needs another mod you do not \
             have, or it is a bug for its author.",
            wanted_clean
        ),
        Some(found) if found.same_path => format!(
            "{} does have {}, at exactly that path, under {}/. The file is there, so the failure \
             is about where or when it was asked for: the game keeps client, server and shared \
             apart, and a file in one cannot require a module in another; and a module is not \
             there yet if the mod that owns it loads later. Changing the path fixes neither.",
            found.owner, found.module_path, found.phase
        ),
        Some(found) => format!(
            "It is {} that has this module, at {} (under {}/) — not at {}. Build 42 moved a lot \
             of Lua about. Patching that line of the mod to ask for the path on the right is \
             what fixes it.",
            found.owner, found.module_path, found.phase, wanted_clean
        ),
    };

    RequireAnswer { wanted: wanted_clean, candidates, summary }
}

/// The module a `require("...") failed` line is about.
pub fn wanted_module(line: &str) -> Option<String> {
    let after = line.split("require(").nth(1)?;
    let inside = after.split(')').next()?;
    let cleaned = inside.trim().trim_matches('"').trim_matches('\'').trim();
    (!cleaned.is_empty()).then(|| cleaned.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(path: &str, owner: &str) -> (String, ModuleCandidate) {
        (
            path.rsplit('/').next().unwrap().to_lowercase(),
            ModuleCandidate {
                module_path: path.to_string(),
                phase: "client".to_string(),
                owner: owner.to_string(),
                same_path: false,
            },
        )
    }

    #[test]
    fn the_module_the_line_is_about() {
        let line = "WARN : Lua f:0 at Lua((MOD:X)).file.lua> require(\"Vehicle/Vehicles\") failed";
        assert_eq!(wanted_module(line).as_deref(), Some("Vehicle/Vehicles"));
        assert_eq!(wanted_module("nothing here"), None);
    }

    #[test]
    fn a_module_that_moved_is_pointed_at_its_new_path() {
        let index = vec![candidate("Vehicles/Vehicles", "the game")];
        let answer = answer_for("Vehicle/Vehicles", &index);
        assert_eq!(answer.candidates.len(), 1);
        assert!(!answer.candidates[0].same_path);
        assert!(answer.summary.contains("Vehicles/Vehicles"), "{}", answer.summary);
        assert!(answer.summary.contains("Patching"), "{}", answer.summary);
    }

    #[test]
    fn a_module_that_is_right_where_it_was_asked_for_is_about_load_order() {
        let index = vec![candidate("TOC/Controllers/ItemsController", "The Only Cure Core")];
        let answer = answer_for("TOC/Controllers/ItemsController", &index);
        assert!(answer.candidates[0].same_path);
        assert!(answer.summary.contains("client, server and shared"), "{}", answer.summary);
    }

    #[test]
    fn a_module_nobody_has_says_so() {
        let answer = answer_for("LSMoodleManager", &[]);
        assert!(answer.candidates.is_empty());
        assert!(answer.summary.contains("not there"), "{}", answer.summary);
    }
}

#[cfg(test)]
mod live_tests {
    /// Answers every failed require in a real log.
    /// PZMS_PZ_DIR=... PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=... PZMS_CONSOLE=...
    /// cargo test --release the_real_failed_requires -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_real_failed_requires() {
        let (Ok(pz), Ok(workshop), Ok(zomboid), Ok(console)) = (
            std::env::var("PZMS_PZ_DIR"),
            std::env::var("PZMS_WORKSHOP_DIR"),
            std::env::var("PZMS_ZOMBOID_DIR"),
            std::env::var("PZMS_CONSOLE"),
        ) else {
            return;
        };
        let paths = crate::vfs::StudioPaths {
            pz_install_dir: pz,
            workshop_dir: workshop.clone(),
            mod_list_ini_path: format!("{}/mods/default.txt", zomboid),
            user_zomboid_dir: zomboid.clone(),
            carrier_workshop_id: None,
            is_valid: true,
        };
        let active = crate::load_order::ini_parser::read_mod_list_ini(&paths.mod_list_ini_path)
            .map(|d| d.active_mods)
            .unwrap_or_default();
        let catalog = crate::catalog::build_catalog(&workshop, &zomboid, None);
        let started = std::time::Instant::now();
        let index = super::index_modules(&paths, &catalog, &active);
        println!("{} modules indexed in {:?}", index.len(), started.elapsed());

        let text = std::fs::read_to_string(&console).expect("console.txt");
        let mut asked: Vec<String> = Vec::new();
        for line in text.lines() {
            if !line.contains("failed") {
                continue;
            }
            if let Some(w) = super::wanted_module(line) {
                if !asked.contains(&w) {
                    asked.push(w);
                }
            }
        }
        println!("{} distinct failed requires:\n", asked.len());
        for w in &asked {
            let answer = super::answer_for(w, &index);
            println!("  {}\n    {}\n", w, answer.summary);
        }
    }
}
