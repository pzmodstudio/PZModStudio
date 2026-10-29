//! Mod maps a save does not load.
//!
//! A save keeps the list of map folders it was made with, in `map_ver.bin` (Build 42: a version
//! number, then the folder names joined with `;`, as the game's `MapGroups` ordered them at new
//! game: the active mods' maps first, the game's own world last). The game reads lots only from
//! those folders and never adds to the list: a map mod, or a mod with interiors on its own cells
//! (the RV interior mods put each RV far outside the world), switched on after the save was made
//! is loaded as a mod but its map is not, so where its cells should be the game generates bare
//! land. A save born with an empty mod list (the Link's old bug) has only the game's own maps.
//!
//! The check here: for each save, the map folders of the mods in its own list that `map_ver.bin`
//! does not name. The fix: those folders put in front of the list, the way a new game would have
//! them, and the chunks the save already generated in their cells moved out of the save when no
//! listed map covers those cells (bare land the game made because it had nothing to put there),
//! so the game builds them from the map next time. A cell that a listed map covers too is left as
//! it is: the player may have been there.

use super::{backup_save, backups_root, list_saves, save_dir, save_mods, saves_dir, REASON_BEFORE_FIX};
use crate::map_cells::{folder_cells, MapCells};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// A Build 42 chunk is 8 x 8 tiles (`map/<x>/<y>.bin`), a cell 256 x 256 tiles: 32 chunks.
pub const CHUNKS_PER_CELL: i32 = 32;

/// A map folder a save's mods bring that the save does not load.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MissingMap {
    /// The folder under media/maps, as the list names it.
    pub folder: String,
    pub mod_id: String,
    pub mod_name: String,
    pub cells: usize,
    /// Chunks the save already generated in its cells that no listed map covers: bare land where
    /// the map should be. The fix moves them out of the save.
    pub empty_chunks: usize,
    /// Its cells that a listed map covers too: the chunks there stay as they are.
    pub over_listed: usize,
}

/// What a save loads and what it is missing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SaveMaps {
    /// The folders `map_ver.bin` names, in order.
    pub listed: Vec<String>,
    /// In the order the mods come in the save's list.
    pub missing: Vec<MissingMap>,
    pub empty_chunks: usize,
}

/// What the fix did.
#[derive(Debug, Clone, Serialize)]
pub struct SaveMapsFix {
    pub listed: Vec<String>,
    pub added: Vec<String>,
    pub moved_chunks: usize,
    /// Where the moved chunks are (empty when none moved).
    pub chunks_dir: String,
    /// The backup zip made first.
    pub backup: String,
}

/// `map_ver.bin`: a big-endian version, a big-endian char count, UTF-16BE chars.
pub fn read_map_ver(save: &Path) -> Option<(i32, Vec<String>)> {
    let b = fs::read(save.join("map_ver.bin")).ok()?;
    if b.len() < 8 {
        return None;
    }
    let version = i32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    let count = i32::from_be_bytes([b[4], b[5], b[6], b[7]]);
    if count < 0 || 8 + 2 * count as usize != b.len() {
        return None;
    }
    let units: Vec<u16> = b[8..].chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    let text = String::from_utf16(&units).ok()?;
    Some((version, text.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()))
}

pub fn write_map_ver(save: &Path, version: i32, maps: &[String]) -> Result<(), String> {
    let text = maps.join(";");
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut b = Vec::with_capacity(8 + 2 * units.len());
    b.extend_from_slice(&version.to_be_bytes());
    b.extend_from_slice(&(units.len() as i32).to_be_bytes());
    for u in units {
        b.extend_from_slice(&u.to_be_bytes());
    }
    fs::write(save.join("map_ver.bin"), b).map_err(|e| format!("Cannot write map_ver.bin: {}", e))
}

/// The chunk files (`map/<x>/<y>.bin`) the save has inside these cells.
fn chunk_files_in(save: &Path, cells: &BTreeSet<[i32; 2]>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cells.is_empty() {
        return out;
    }
    for dir in fs::read_dir(save.join("map")).into_iter().flatten().flatten() {
        let Ok(cx) = dir.file_name().to_string_lossy().parse::<i32>() else { continue };
        if !cells.iter().any(|c| c[0] == cx.div_euclid(CHUNKS_PER_CELL)) {
            continue;
        }
        for file in fs::read_dir(dir.path()).into_iter().flatten().flatten() {
            let name = file.file_name().to_string_lossy().to_string();
            let Some(cy) = name.strip_suffix(".bin").and_then(|s| s.parse::<i32>().ok()) else { continue };
            if cells.contains(&[cx.div_euclid(CHUNKS_PER_CELL), cy.div_euclid(CHUNKS_PER_CELL)]) {
                out.push(file.path());
            }
        }
    }
    out
}

/// The cells every listed folder covers: the game's own maps from the install, mod maps by folder name.
fn covered_cells(pz_install_dir: &str, cells: &MapCells, listed: &[String]) -> BTreeSet<[i32; 2]> {
    let mut out = BTreeSet::new();
    for name in listed {
        let vanilla = Path::new(pz_install_dir).join("media").join("maps").join(name);
        if vanilla.is_dir() {
            out.extend(folder_cells(&vanilla));
        }
        for m in cells.maps.iter().filter(|m| &m.folder == name) {
            out.extend(m.cells.iter().copied());
        }
    }
    out
}

/// The check for one save, or None when it keeps no list of its own (a server's save), has no
/// `map_ver.bin`, or is missing nothing.
pub fn check_save(save: &Path, pz_install_dir: &str, cells: &MapCells) -> Option<SaveMaps> {
    let (_, listed) = read_map_ver(save)?;
    let mods: Vec<String> = save_mods(save).iter().map(|m| crate::load_order::mod_info::sanitize_mod_id(m)).map(|m| m.rsplit(['/', '\\']).next().unwrap_or_default().to_string()).collect();
    if mods.is_empty() {
        return None;
    }
    let mut missing: Vec<MissingMap> = Vec::new();
    for mod_id in &mods {
        for m in cells.maps.iter().filter(|m| &m.mod_id == mod_id && !listed.contains(&m.folder)) {
            if missing.iter().any(|x| x.folder == m.folder) {
                continue;
            }
            missing.push(MissingMap { folder: m.folder.clone(), mod_id: m.mod_id.clone(), mod_name: m.mod_name.clone(), cells: m.cells.len(), empty_chunks: 0, over_listed: 0 });
        }
    }
    if missing.is_empty() {
        return None;
    }
    let covered = covered_cells(pz_install_dir, cells, &listed);
    for miss in &mut missing {
        let Some(m) = cells.maps.iter().find(|m| m.folder == miss.folder) else { continue };
        let own: BTreeSet<[i32; 2]> = m.cells.iter().copied().filter(|c| !covered.contains(c)).collect();
        miss.over_listed = m.cells.len() - own.len();
        miss.empty_chunks = chunk_files_in(save, &own).len();
    }
    let empty_chunks = missing.iter().map(|m| m.empty_chunks).sum();
    Some(SaveMaps { listed, missing, empty_chunks })
}

/// For each save that misses a map its own mods bring: what it loads and what it misses.
pub fn missing_maps(user_zomboid_dir: &Path, pz_install_dir: &str, cells: &MapCells) -> BTreeMap<String, SaveMaps> {
    let mut out = BTreeMap::new();
    for id in list_saves(user_zomboid_dir) {
        if let Some(report) = check_save(&saves_dir(user_zomboid_dir).join(&id), pz_install_dir, cells) {
            out.insert(id, report);
        }
    }
    out
}

/// Puts the missing maps in front of the save's list and moves the bare chunks in their cells
/// out of the save (to `PZModStudio_SaveBackups/<mode>/<world>/chunks-<stamp>/`), after a backup
/// of the save. Refused while the game runs.
pub fn add_maps(user_zomboid_dir: &Path, pz_install_dir: &str, cells: &MapCells, save_id: &str, game_running: bool) -> Result<SaveMapsFix, String> {
    if game_running {
        return Err("Project Zomboid is running: close it first, the game writes the save while it runs.".to_string());
    }
    let save = save_dir(user_zomboid_dir, save_id)?;
    let report = check_save(&save, pz_install_dir, cells).ok_or_else(|| format!("{} is missing no map of its mods.", save_id))?;
    let (version, listed) = read_map_ver(&save).ok_or_else(|| "Cannot read the save's map_ver.bin.".to_string())?;
    let backup = backup_save(user_zomboid_dir, save_id, REASON_BEFORE_FIX, false)?;

    let covered = covered_cells(pz_install_dir, cells, &listed);
    let mut own: BTreeSet<[i32; 2]> = BTreeSet::new();
    for miss in &report.missing {
        if let Some(m) = cells.maps.iter().find(|m| m.folder == miss.folder) {
            own.extend(m.cells.iter().copied().filter(|c| !covered.contains(c)));
        }
    }
    let chunks = chunk_files_in(&save, &own);
    let mut chunks_dir = String::new();
    if !chunks.is_empty() {
        let (mode, world) = super::split_id(save_id)?;
        let stamp = chrono_stamp();
        let dir = backups_root(user_zomboid_dir).join(mode).join(world).join(format!("chunks-{}", stamp));
        for file in &chunks {
            let cx = file.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let target = dir.join(&cx);
            fs::create_dir_all(&target).map_err(|e| format!("Cannot make {}: {}", target.display(), e))?;
            let dst = target.join(file.file_name().unwrap_or_default());
            fs::rename(file, &dst).or_else(|_| fs::copy(file, &dst).map(|_| ()).and_then(|_| fs::remove_file(file))).map_err(|e| format!("Cannot move {}: {}", file.display(), e))?;
        }
        for dir in fs::read_dir(save.join("map")).into_iter().flatten().flatten() {
            if dir.path().is_dir() && fs::read_dir(dir.path()).map(|mut d| d.next().is_none()).unwrap_or(false) {
                let _ = fs::remove_dir(dir.path());
            }
        }
        chunks_dir = dir.to_string_lossy().to_string();
    }

    let added: Vec<String> = report.missing.iter().map(|m| m.folder.clone()).collect();
    let mut new_list = added.clone();
    new_list.extend(listed.iter().cloned());
    write_map_ver(&save, version, &new_list)?;
    Ok(SaveMapsFix { listed: new_list, added, moved_chunks: chunks.len(), chunks_dir, backup: backup.entry.file })
}

/// Local time, in the form the backup zips use.
fn chrono_stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_cells::MapFolder;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pz-save-maps-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn folder(mod_id: &str, name: &str, cells: Vec<[i32; 2]>) -> MapFolder {
        MapFolder {
            mod_id: mod_id.into(),
            mod_name: mod_id.to_uppercase(),
            workshop_id: None,
            folder: name.into(),
            title: name.into(),
            lots: vec![],
            active: true,
            cells,
            over_vanilla: 0,
            dir: PathBuf::new(),
        }
    }

    #[test]
    fn map_ver_round_trips() {
        let d = temp("roundtrip");
        let maps = vec!["map_distanciado".to_string(), "Muldraugh, KY".to_string()];
        write_map_ver(&d, 249, &maps).unwrap();
        assert_eq!(read_map_ver(&d), Some((249, maps)));
        // The game's own form: version 249, 161 chars, UTF-16BE.
        let b = fs::read(d.join("map_ver.bin")).unwrap();
        assert_eq!(&b[..8], &[0, 0, 0, 249, 0, 0, 0, 29]);
        assert_eq!(&b[8..12], &[0, b'm', 0, b'a']);
    }

    #[test]
    fn a_mod_map_the_save_does_not_list_is_missing_with_its_bare_chunks_counted() {
        let z = temp("missing");
        let save = z.join("Saves").join("Sandbox").join("w");
        fs::create_dir_all(save.join("map").join("2873")).unwrap();
        fs::create_dir_all(save.join("map").join("1600")).unwrap();
        // Chunk 2873,1546 is in cell 89,48 (the mod's); chunk 1600,900 is in cell 50,28 (not).
        fs::write(save.join("map").join("2873").join("1546.bin"), b"x").unwrap();
        fs::write(save.join("map").join("1600").join("900.bin"), b"x").unwrap();
        fs::write(save.join("mods.txt"), "VERSION = 1,\n\nmods\n{\n    mod = 12345\\RVMod,\n    mod = Other,\n}\n").unwrap();
        write_map_ver(&save, 249, &["Muldraugh, KY".to_string()]).unwrap();
        let cells = MapCells {
            vanilla: vec![],
            maps: vec![folder("RVMod", "rv_rooms", vec![[89, 48], [50, 28]]), folder("NotOn", "elsewhere", vec![[1, 1]])],
            related: vec![],
            clashes: vec![],
            tiledef_clashes: vec![],
        };
        // The install has only "Muldraugh, KY" with cell 50,28: that cell is covered, 89,48 is not.
        let install = z.join("game");
        let van = install.join("media").join("maps").join("Muldraugh, KY");
        fs::create_dir_all(&van).unwrap();
        fs::write(van.join("50_28.lotheader"), b"").unwrap();
        let install_s = install.to_string_lossy().to_string();

        let all = missing_maps(&z, &install_s, &cells);
        let r = &all["Sandbox/w"];
        assert_eq!(r.listed, vec!["Muldraugh, KY".to_string()]);
        assert_eq!(r.missing.len(), 1);
        assert_eq!(r.missing[0].folder, "rv_rooms");
        assert_eq!(r.missing[0].mod_id, "RVMod");
        assert_eq!(r.missing[0].over_listed, 1);
        assert_eq!(r.missing[0].empty_chunks, 1);

        assert!(add_maps(&z, &install_s, &cells, "Sandbox/w", true).is_err());
        let fix = add_maps(&z, &install_s, &cells, "Sandbox/w", false).unwrap();
        assert_eq!(fix.added, vec!["rv_rooms".to_string()]);
        assert_eq!(fix.listed, vec!["rv_rooms".to_string(), "Muldraugh, KY".to_string()]);
        assert_eq!(fix.moved_chunks, 1);
        assert!(!save.join("map").join("2873").join("1546.bin").exists());
        assert!(save.join("map").join("1600").join("900.bin").exists());
        assert!(Path::new(&fix.chunks_dir).join("2873").join("1546.bin").exists());
        assert_eq!(read_map_ver(&save).unwrap().1, fix.listed);
        assert!(z.join("PZModStudio_SaveBackups").join("Sandbox").join("w").join(&fix.backup).exists());
        // Nothing missing now.
        assert!(missing_maps(&z, &install_s, &cells).is_empty());
        let _ = fs::remove_dir_all(&z);
    }
}
