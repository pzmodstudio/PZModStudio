//! Putting a mod list into a server's `.ini`, and saying what is wrong with the one that is there.
//!
//! A dedicated server does not read `default.txt`. It reads three lines of its own, and all three
//! have to agree or the server starts and quietly plays vanilla:
//!
//! - `WorkshopItems=` are the Workshop ids Steam downloads. Missing from here, a mod never arrives.
//! - `Mods=` are the mod ids the game then loads. Missing from here, a downloaded mod sits unused.
//! - `Map=` are map folder names, with the base map last. A map mod missing from here is ignored,
//!   and every one of those is a mistake nothing in the game reports.
//!
//! One mod id can come from several Workshop items and one Workshop item can hold several mods, so
//! the two lines are not a copy of each other; both are built from the catalog of what is really
//! installed. The file is written through the same backed-up, atomic writer as the mod list.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

use crate::catalog::{HealthIssue, ModCatalog, ModSource, Severity};

/// The Workshop item a mod came from, when it came from one.
fn workshop_id_of(m: &crate::catalog::CatalogMod) -> Option<&str> {
    match &m.source {
        ModSource::Workshop { workshop_id } => Some(workshop_id.as_str()),
        _ => None,
    }
}

/// What a server's `.ini` says about mods, and what does not add up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerModReport {
    pub mods: Vec<String>,
    pub workshop_items: Vec<String>,
    pub maps: Vec<String>,
    pub issues: Vec<HealthIssue>,
}

/// The value of a `Key=a;b;c` line, split the way the server reads it.
pub fn ini_list(content: &str, key: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix(&format!("{}=", key)))
        .map(|v| {
            v.split(';')
                // B42 writes mod ids as `\ModId`; the id is what follows.
                .map(|s| if key == "Mods" { s.trim().trim_start_matches('\\').to_string() } else { s.trim().to_string() })
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The map folders a mod ships, as `Map=` names them.
pub fn maps_in_mod(root: &Path, game_version: Option<u32>) -> Vec<String> {
    let mut found = BTreeSet::new();
    for layer in crate::load_order::mod_info::media_layers_for_build(root, game_version) {
        let maps = layer.join("media").join("maps");
        if let Ok(entries) = std::fs::read_dir(&maps) {
            for e in entries.flatten() {
                if e.path().is_dir() {
                    found.insert(e.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    found.into_iter().collect()
}

/// The base game's own map folders, which belong at the end of `Map=` and are not mod maps.
pub fn vanilla_maps(pz_install_dir: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(Path::new(pz_install_dir).join("media").join("maps")) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                found.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }
    found.sort();
    found
}

/// The three lines a server needs for this mod list, in load order.
///
/// `Map=` keeps whatever the file already had that is not a mod map — the base map, usually — at
/// the end, where the game needs it, and puts this list's map mods in front of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerLines {
    pub mods: Vec<String>,
    pub workshop_items: Vec<String>,
    pub maps: Vec<String>,
}

pub fn lines_for(
    catalog: &ModCatalog,
    active: &[String],
    existing_map_line: &[String],
    pz_install_dir: &str,
) -> ServerLines {
    let game = catalog.game_version_int();
    let mut mods: Vec<String> = Vec::new();
    let mut workshop_items: Vec<String> = Vec::new();
    let mut mod_maps: Vec<String> = Vec::new();

    for entry in active {
        let Some(m) = catalog.resolve(entry.trim()) else {
            // Not installed here: it still belongs in Mods=, because the server may have it.
            if !mods.iter().any(|x| x == entry) {
                mods.push(entry.to_string());
            }
            continue;
        };
        if !mods.iter().any(|x| x == &m.id) {
            mods.push(m.id.clone());
        }
        if let Some(ws) = workshop_id_of(m) {
            if !workshop_items.iter().any(|x| x == ws) {
                workshop_items.push(ws.to_string());
            }
        }
        for map in maps_in_mod(Path::new(&m.root_dir), game) {
            if !mod_maps.iter().any(|x| x == &map) {
                mod_maps.push(map);
            }
        }
    }

    // Whatever was already at the end and is not a mod's map stays there (the base map, a map
    // the server has from elsewhere). A map of a mod switched off goes: the server no longer
    // loads that mod, so the folder is not there for it.
    let vanilla = vanilla_maps(pz_install_dir);
    let maps_of_any_mod: BTreeSet<String> = catalog
        .mods
        .iter()
        .flat_map(|m| maps_in_mod(Path::new(&m.root_dir), game))
        .filter(|m| !vanilla.contains(m))
        .collect();
    let mut tail: Vec<String> = existing_map_line
        .iter()
        .filter(|m| !mod_maps.contains(m) && !maps_of_any_mod.contains(*m))
        .cloned()
        .collect();
    if tail.is_empty() {
        // A server with no Map= line at all plays the default map.
        let default = "Muldraugh, KY".to_string();
        if vanilla.is_empty() || vanilla.contains(&default) {
            tail.push(default);
        } else {
            tail.push(vanilla[0].clone());
        }
    }
    let maps = mod_maps.into_iter().chain(tail).collect();

    ServerLines { mods, workshop_items, maps }
}

/// Everything in the `.ini` that will not do what the admin meant, read against what is installed.
pub fn check(catalog: &ModCatalog, content: &str, pz_install_dir: &str) -> ServerModReport {
    let mods = ini_list(content, "Mods");
    let workshop_items = ini_list(content, "WorkshopItems");
    let maps = ini_list(content, "Map");
    let game = catalog.game_version_int();
    let vanilla = vanilla_maps(pz_install_dir);
    let mut issues: Vec<HealthIssue> = Vec::new();
    let mut add = |severity: Severity, code: &str, message: String, params: &[(&str, &str)]| {
        issues.push(HealthIssue {
            params: crate::catalog::issue_params(params),
            details: vec![],
            severity,
            code: code.to_string(),
            mod_id: None,
            message,
            fixes: Vec::new(),
            related: Vec::new(),
        });
    };

    // Mods= against what is installed, and against WorkshopItems=.
    let mut covered_items: BTreeSet<String> = BTreeSet::new();
    for entry in &mods {
        let Some(m) = catalog.resolve(entry.trim()) else {
            add(
                Severity::Warning,
                "server_mod_not_installed",
                format!(
                    "Mods= lists {}, which is not installed on this machine. If the server does not have it either, it will not load.",
                    entry
                ),
                &[("entry", entry)],
            );
            continue;
        };
        match workshop_id_of(m) {
            Some(ws) => {
                covered_items.insert(ws.to_string());
                if !workshop_items.iter().any(|x| x == ws) {
                    add(
                        Severity::Error,
                        "server_item_missing",
                        format!(
                            "{} is in Mods= but its Workshop id {} is not in WorkshopItems=, so the server never downloads it and the mod is missing in game.",
                            m.name, ws
                        ),
                        &[("mod", &m.name), ("item", ws)],
                    );
                }
            }
            None => add(
                Severity::Warning,
                "server_mod_not_from_workshop",
                format!(
                    "{} did not come from the Workshop, so the server cannot download it: copy its folder into the server's Zomboid/mods itself.",
                    m.name
                ),
                &[("mod", &m.name)],
            ),
        }
        if m.loadability != crate::catalog::Loadability::Loadable {
            add(
                Severity::Warning,
                "server_mod_not_for_build",
                format!("{} cannot be loaded by the build installed here ({}).", m.name, entry),
                &[("mod", &m.name), ("entry", entry)],
            );
        }
        // Map mods have to be named in Map= as well, or the game ignores their map.
        let mod_maps = maps_in_mod(Path::new(&m.root_dir), game);
        let forgotten: Vec<String> = mod_maps.iter().filter(|x| !maps.contains(x)).cloned().collect();
        if !forgotten.is_empty() {
            add(
                Severity::Error,
                "server_map_missing",
                format!(
                    "{} adds the map(s) {}, which Map= does not list. The mod loads, the map does not.",
                    m.name,
                    forgotten.join(", ")
                ),
                &[("mod", &m.name), ("maps", &forgotten.join(", "))],
            );
        }
    }

    // WorkshopItems= that nothing in Mods= uses.
    for item in &workshop_items {
        if covered_items.contains(item) {
            continue;
        }
        let from_item: Vec<&str> = catalog
            .mods
            .iter()
            .filter(|m| workshop_id_of(m) == Some(item.as_str()))
            .map(|m| m.id.as_str())
            .collect();
        let message = if from_item.is_empty() {
            format!(
                "WorkshopItems= has {}, which is not installed here, and nothing in Mods= uses it. The server downloads it and loads nothing from it.",
                item
            )
        } else {
            format!(
                "WorkshopItems= has {}, which holds {}, but Mods= does not list {}. It is downloaded and never loaded.",
                item,
                from_item.join(", "),
                if from_item.len() == 1 { "it" } else { "them" }
            )
        };
        let holds = from_item.join(", ");
        let params: &[(&str, &str)] = if from_item.is_empty() {
            &[("variant", "not_installed"), ("item", item)]
        } else {
            &[("variant", if from_item.len() == 1 { "holds_one" } else { "holds_many" }), ("item", item), ("mods", &holds)]
        };
        add(Severity::Warning, "server_item_unused", message, params);
    }

    // The base map goes last: the game reads Map= in order and the last entry is the base.
    if !maps.is_empty() && !vanilla.is_empty() {
        let last = maps.last().unwrap();
        if !vanilla.contains(last) {
            add(
                Severity::Warning,
                "server_base_map_last",
                format!(
                    "Map= ends with {}, which is not one of the game's own maps. The base map (for example Muldraugh, KY) belongs last.",
                    last
                ),
                &[("map", last)],
            );
        }
    }

    issues.sort_by(|a, b| a.severity.cmp(&b.severity));
    ServerModReport { mods, workshop_items, maps, issues }
}

/// Replaces `Mods=`, `WorkshopItems=` and `Map=` in the file's text, keeping everything else —
/// every other setting, its order, its comments — exactly as it was.
pub fn with_mod_lines(content: &str, lines: &ServerLines) -> String {
    let eol = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let wanted = [
        ("Mods", lines.mods.join(";")),
        ("WorkshopItems", lines.workshop_items.join(";")),
        ("Map", lines.maps.join(";")),
    ];
    let mut seen = [false; 3];
    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        let mut replaced = false;
        for (i, (key, value)) in wanted.iter().enumerate() {
            if trimmed.starts_with(&format!("{}=", key)) {
                out.push(format!("{}={}", key, value));
                seen[i] = true;
                replaced = true;
                break;
            }
        }
        if !replaced {
            out.push(line.to_string());
        }
    }
    for (i, (key, value)) in wanted.iter().enumerate() {
        if !seen[i] {
            out.push(format!("{}={}", key, value));
        }
    }
    let mut text = out.join(eol);
    if content.ends_with('\n') || content.ends_with("\r\n") {
        text.push_str(eol);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lines_value_is_read_the_way_the_server_reads_it() {
        let ini = "PVP=true\r\nMods=A;B ; C\r\nWorkshopItems=1;2\r\n";
        assert_eq!(ini_list(ini, "Mods"), vec!["A", "B", "C"]);
        assert_eq!(ini_list(ini, "WorkshopItems"), vec!["1", "2"]);
        assert!(ini_list(ini, "Map").is_empty());
        // B42's backslash-prefixed ids are the same ids.
        assert_eq!(ini_list("Mods=\\ModA;\\ModB\r\n", "Mods"), vec!["ModA", "ModB"]);
    }

    #[test]
    fn only_the_three_lines_change_and_missing_ones_are_added() {
        let ini = "PVP=true\r\nMods=Old\r\nMaxPlayers=16\r\n";
        let lines = ServerLines {
            mods: vec!["A".into(), "B".into()],
            workshop_items: vec!["111".into()],
            maps: vec!["MyMap".into(), "Muldraugh, KY".into()],
        };
        let out = with_mod_lines(ini, &lines);
        assert_eq!(
            out,
            "PVP=true\r\nMods=A;B\r\nMaxPlayers=16\r\nWorkshopItems=111\r\nMap=MyMap;Muldraugh, KY\r\n"
        );
    }

    #[test]
    fn the_base_map_stays_at_the_end_and_mod_maps_go_in_front() {
        // The map line already there names a base map; the list adds two mod maps.
        let existing = vec!["Riverside, KY".to_string()];
        let mod_maps = vec!["BigTown".to_string(), "SmallTown".to_string()];
        let tail: Vec<String> = existing.iter().filter(|m| !mod_maps.contains(m)).cloned().collect();
        let joined: Vec<String> = mod_maps.into_iter().chain(tail).collect();
        assert_eq!(joined, vec!["BigTown", "SmallTown", "Riverside, KY"]);
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// Reads a real server .ini against a real install and prints what does not add up.
    /// PZMS_PZ_DIR=... PZMS_WORKSHOP_DIR=... PZMS_ZOMBOID_DIR=... PZMS_SERVER_INI=...
    /// cargo test --release a_real_server_ini -- --ignored --nocapture
    #[test]
    #[ignore]
    fn a_real_server_ini() {
        let (Ok(pz), Ok(workshop), Ok(zomboid), Ok(ini)) = (
            std::env::var("PZMS_PZ_DIR"),
            std::env::var("PZMS_WORKSHOP_DIR"),
            std::env::var("PZMS_ZOMBOID_DIR"),
            std::env::var("PZMS_SERVER_INI"),
        ) else {
            return;
        };
        let content = std::fs::read_to_string(&ini).expect("the .ini reads");
        let catalog = crate::catalog::build_catalog(&workshop, &zomboid, None);
        let report = check(&catalog, &content, &pz);
        println!(
            "Mods={} WorkshopItems={} Map={:?}",
            report.mods.len(),
            report.workshop_items.len(),
            report.maps
        );
        println!("{} issue(s):", report.issues.len());
        for i in &report.issues {
            println!("  [{:?}] {}", i.severity, i.message);
        }

        // And what a sync from the player's own active list would write.
        let active = crate::load_order::ini_parser::read_mod_list_ini(&format!("{}/mods/default.txt", zomboid))
            .map(|d| d.active_mods)
            .unwrap_or_default();
        let lines = lines_for(&catalog, &active, &report.maps, &pz);
        println!(
            "a sync would write {} mods, {} items, Map={:?}",
            lines.mods.len(),
            lines.workshop_items.len(),
            lines.maps
        );
        let updated = with_mod_lines(&content, &lines);
        let changed = content.lines().zip(updated.lines()).filter(|(a, b)| a != b).count();
        println!("{} line(s) of the file would change", changed);
        assert!(updated.lines().count() >= content.lines().count());
    }
}
