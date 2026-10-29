//! Which cells of the world each map mod covers, and where two of them clash.
//!
//! A map is a folder under `media/maps/` with one `X_Y.lotheader` per cell it has (a B42 cell
//! is 256 x 256 tiles). The game's own world is `media/maps/Muldraugh, KY` in the install folder.
//! A map mod's cells over the game's world replace what was there; two map mods with cells in
//! common fight over them (the one loaded later wins, the other's buildings there are gone),
//! unless one builds on the other: its `map.info` names the other in `lots=`.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub mod images;

/// The game's own world.
pub const VANILLA_MAP: &str = "Muldraugh, KY";

/// One map folder of one mod.
#[derive(Debug, Clone, Serialize)]
pub struct MapFolder {
    pub mod_id: String,
    pub mod_name: String,
    pub workshop_id: Option<String>,
    /// The folder under media/maps (what map lists and map.info's `lots=` name).
    pub folder: String,
    /// map.info's `title`.
    pub title: String,
    /// map.info's `lots=`: the maps it builds on.
    pub lots: Vec<String>,
    /// Its mod is on in the list the game reads.
    pub active: bool,
    pub cells: Vec<[i32; 2]>,
    /// How many of its cells are over the game's own world.
    pub over_vanilla: usize,
    /// The folder on disk (for its image).
    #[serde(skip)]
    pub dir: PathBuf,
}

/// A mod that is about maps without cells of its own on the grid: spawn points, a map list, or
/// the tiles and textures maps are built from.
#[derive(Debug, Clone, Serialize)]
pub struct MapRelated {
    pub mod_id: String,
    pub mod_name: String,
    pub workshop_id: Option<String>,
    pub active: bool,
    /// What it brings: "spawn" (map folders with spawn points or a map list, no cells),
    /// "tiles" (pack= / tiledef= in its mod.info).
    pub kinds: Vec<String>,
    /// The map folders it has, the tile packs and tile definitions it names.
    pub detail: Vec<String>,
}

/// Two map folders with cells in common, neither building on the other.
#[derive(Debug, Clone, Serialize)]
pub struct Clash {
    pub a: String,
    pub b: String,
    pub mod_a: String,
    pub mod_b: String,
    pub cells: Vec<[i32; 2]>,
    /// Both mods are on: they clash in the next game.
    pub both_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapCells {
    pub vanilla: Vec<[i32; 2]>,
    pub maps: Vec<MapFolder>,
    /// Mods about maps that have no cells of their own.
    pub related: Vec<MapRelated>,
    pub clashes: Vec<Clash>,
    /// Tile definition numbers more than one mod uses.
    pub tiledef_clashes: Vec<TiledefClash>,
}

/// One tile definition number (`tiledef=<name> <number>`) used by more than one mod. The number
/// has to be unique: two mods on with the same one overwrite each other's tiles.
#[derive(Debug, Clone, Serialize)]
pub struct TiledefClash {
    pub number: u32,
    /// (mod id, mod name, tiledef name, on).
    pub mods: Vec<(String, String, String, bool)>,
    /// More than one of them is on.
    pub both_active: bool,
}

/// Tile definition numbers in more than one mod (a mod naming one twice is not a clash).
pub fn tiledef_clashes(entries: &[(String, String, String, u32, bool)]) -> Vec<TiledefClash> {
    let mut by_number: BTreeMap<u32, Vec<(String, String, String, bool)>> = BTreeMap::new();
    for (id, name, def, number, active) in entries {
        let list = by_number.entry(*number).or_default();
        if !list.iter().any(|m| &m.0 == id) {
            list.push((id.clone(), name.clone(), def.clone(), *active));
        }
    }
    let mut out: Vec<TiledefClash> = by_number
        .into_iter()
        .filter(|(_, mods)| mods.len() > 1)
        .map(|(number, mods)| {
            let both_active = mods.iter().filter(|m| m.3).count() > 1;
            TiledefClash { number, mods, both_active }
        })
        .collect();
    out.sort_by(|a, b| b.both_active.cmp(&a.both_active).then(a.number.cmp(&b.number)));
    out
}

/// The cells of one map folder, from its `X_Y.lotheader` files.
pub fn folder_cells(dir: &Path) -> Vec<[i32; 2]> {
    let mut out: Vec<[i32; 2]> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let stem = name.strip_suffix(".lotheader")?;
            let (x, y) = stem.split_once('_')?;
            Some([x.parse().ok()?, y.parse().ok()?])
        })
        .collect();
    out.sort();
    out
}

fn map_info(dir: &Path) -> (String, Vec<String>) {
    let text = std::fs::read_to_string(dir.join("map.info")).unwrap_or_default();
    let field = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string()));
    let lots = field("lots=").map(|v| v.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
    (field("title=").unwrap_or_default(), lots)
}

/// Every mod folder installed: the Workshop's (`<item>/mods/<mod>`) and the local ones.
fn mod_roots(workshop_dir: &str, user_zomboid_dir: &Path) -> Vec<(PathBuf, Option<String>)> {
    let mut out = Vec::new();
    if !workshop_dir.is_empty() {
        for item in std::fs::read_dir(workshop_dir).into_iter().flatten().flatten() {
            let wid = item.file_name().to_string_lossy().to_string();
            for m in std::fs::read_dir(item.path().join("mods")).into_iter().flatten().flatten() {
                out.push((m.path(), Some(wid.clone())));
            }
        }
    }
    for m in std::fs::read_dir(user_zomboid_dir.join("mods")).into_iter().flatten().flatten() {
        if m.path().is_dir() {
            out.push((m.path(), None));
        }
    }
    out
}

/// A mod's id, name, and the tile packs and tile definitions its mod.info names.
fn mod_info_fields(root: &Path) -> Option<(String, String, Vec<String>)> {
    let info = [root.join("42").join("mod.info"), root.join("mod.info")].into_iter().find(|p| p.is_file())?;
    let text = std::fs::read_to_string(info).ok()?;
    let field = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string()));
    let id = crate::load_order::mod_info::sanitize_mod_id(&field("id=")?);
    let name = field("name=").unwrap_or_else(|| id.clone());
    let tiles: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            l.strip_prefix("pack=").map(|v| format!("pack {}", v.trim())).or_else(|| l.strip_prefix("tiledef=").map(|v| format!("tiledef {}", v.trim())))
        })
        .filter(|v| !v.ends_with(' '))
        .collect();
    Some((id, name, tiles))
}

/// Where map folders clash: cells in common, neither naming the other in `lots=`.
pub fn clashes(maps: &[MapFolder]) -> Vec<Clash> {
    let sets: Vec<HashSet<[i32; 2]>> = maps.iter().map(|m| m.cells.iter().copied().collect()).collect();
    let mut out = Vec::new();
    for i in 0..maps.len() {
        for j in (i + 1)..maps.len() {
            let (a, b) = (&maps[i], &maps[j]);
            if a.mod_id == b.mod_id || a.folder == b.folder {
                continue;
            }
            if a.lots.iter().any(|l| l == &b.folder) || b.lots.iter().any(|l| l == &a.folder) {
                continue;
            }
            let mut common: Vec<[i32; 2]> = sets[i].intersection(&sets[j]).copied().collect();
            if common.is_empty() {
                continue;
            }
            common.sort();
            out.push(Clash { a: a.folder.clone(), b: b.folder.clone(), mod_a: a.mod_id.clone(), mod_b: b.mod_id.clone(), cells: common, both_active: a.active && b.active });
        }
    }
    out.sort_by(|x, y| y.both_active.cmp(&x.both_active).then(y.cells.len().cmp(&x.cells.len())));
    out
}

/// The game's world and every installed map mod's cells, and their clashes.
pub fn map_cells(pz_install_dir: &str, workshop_dir: &str, user_zomboid_dir: &Path) -> MapCells {
    let vanilla = folder_cells(&Path::new(pz_install_dir).join("media").join("maps").join(VANILLA_MAP));
    let vanilla_set: BTreeSet<[i32; 2]> = vanilla.iter().copied().collect();
    let game = crate::load_order::mod_info::read_installed_game_version(&user_zomboid_dir.to_string_lossy());
    let active: Vec<String> = crate::load_order::ini_parser::try_parse_default_txt(&user_zomboid_dir.join("mods").join("default.txt")).unwrap_or_default();

    // One entry per (mod, folder): the version folder wins over common for the same map.
    let mut found: BTreeMap<(String, String), MapFolder> = BTreeMap::new();
    let mut related: BTreeMap<String, MapRelated> = BTreeMap::new();
    let mut tiledefs: Vec<(String, String, String, u32, bool)> = Vec::new();
    for (root, wid) in mod_roots(workshop_dir, user_zomboid_dir) {
        let Some((mod_id, mod_name, tiles)) = mod_info_fields(&root) else { continue };
        for t in &tiles {
            if let Some(rest) = t.strip_prefix("tiledef ") {
                let mut parts = rest.split_whitespace();
                if let (Some(def), Some(Ok(number))) = (parts.next(), parts.next().map(str::parse::<u32>)) {
                    tiledefs.push((mod_id.clone(), mod_name.clone(), def.to_string(), number, active.contains(&mod_id)));
                }
            }
        }
        let mut note = |kind: &str, detail: String| {
            let r = related.entry(mod_id.clone()).or_insert_with(|| MapRelated {
                mod_id: mod_id.clone(),
                mod_name: mod_name.clone(),
                workshop_id: wid.clone(),
                active: active.contains(&mod_id),
                kinds: Vec::new(),
                detail: Vec::new(),
            });
            if !r.kinds.iter().any(|k| k == kind) {
                r.kinds.push(kind.to_string());
            }
            if !r.detail.contains(&detail) {
                r.detail.push(detail);
            }
        };
        for t in &tiles {
            note("tiles", t.clone());
        }
        for layer in crate::load_order::mod_info::media_layers_for_build(&root, game) {
            for dir in std::fs::read_dir(layer.join("media").join("maps")).into_iter().flatten().flatten() {
                if !dir.path().is_dir() {
                    continue;
                }
                let folder = dir.file_name().to_string_lossy().to_string();
                let cells = folder_cells(&dir.path());
                if cells.is_empty() {
                    // Spawn points or a map list: no cells of its own, still about maps.
                    note("spawn", folder.clone());
                    continue;
                }
                let (title, lots) = map_info(&dir.path());
                let over_vanilla = cells.iter().filter(|c| vanilla_set.contains(*c)).count();
                found.insert(
                    (mod_id.clone(), folder.clone()),
                    MapFolder {
                        mod_id: mod_id.clone(),
                        mod_name: mod_name.clone(),
                        workshop_id: wid.clone(),
                        title: if title.is_empty() { folder.clone() } else { title },
                        folder,
                        lots,
                        active: active.contains(&mod_id),
                        cells,
                        over_vanilla,
                        dir: dir.path(),
                    },
                );
            }
        }
    }
    let mut maps: Vec<MapFolder> = found.into_values().collect();
    maps.sort_by(|a, b| b.active.cmp(&a.active).then(a.mod_name.to_lowercase().cmp(&b.mod_name.to_lowercase())));
    let clashes = clashes(&maps);
    // A mod with cells is on the grid already; the others are listed with what they bring.
    let with_cells: HashSet<&String> = maps.iter().map(|m| &m.mod_id).collect();
    let mut related: Vec<MapRelated> = related.into_values().filter(|r| !with_cells.contains(&r.mod_id)).collect();
    related.sort_by(|a, b| b.active.cmp(&a.active).then(a.mod_name.to_lowercase().cmp(&b.mod_name.to_lowercase())));
    let tiledef_clashes = tiledef_clashes(&tiledefs);
    MapCells { vanilla, maps, related, clashes, tiledef_clashes }
}

/// The map folders with cells, for their images.
pub fn image_dirs(cells: &MapCells) -> Vec<(PathBuf, String)> {
    cells.maps.iter().map(|m| (m.dir.clone(), m.folder.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(mod_id: &str, folder: &str, lots: &[&str], active: bool, cells: &[[i32; 2]]) -> MapFolder {
        MapFolder {
            mod_id: mod_id.into(),
            mod_name: mod_id.into(),
            workshop_id: None,
            folder: folder.into(),
            title: folder.into(),
            lots: lots.iter().map(|s| s.to_string()).collect(),
            active,
            cells: cells.to_vec(),
            over_vanilla: 0,
            dir: PathBuf::new(),
        }
    }

    #[test]
    fn cells_come_from_lotheader_names() {
        let dir = std::env::temp_dir().join(format!("pzms-cells-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["3_4.lotheader", "10_2.lotheader", "world_3_4.lotpack", "map.info", "x_y.lotheader"] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        assert_eq!(folder_cells(&dir), vec![[3, 4], [10, 2]]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn maps_sharing_cells_clash_unless_one_builds_on_the_other() {
        let maps = vec![
            map("A", "TownA", &[], true, &[[1, 1], [1, 2], [5, 5]]),
            map("B", "TownB", &[], true, &[[1, 2], [5, 5], [9, 9]]),
            map("C", "AddOnA", &["TownA"], true, &[[1, 1]]),
            map("D", "Far", &[], false, &[[9, 9]]),
        ];
        let c = clashes(&maps);
        let pairs: Vec<(String, String, usize, bool)> = c.iter().map(|x| (x.a.clone(), x.b.clone(), x.cells.len(), x.both_active)).collect();
        assert_eq!(pairs, vec![("TownA".into(), "TownB".into(), 2, true), ("TownB".into(), "Far".into(), 1, false)]);
    }

    #[test]
    fn a_tile_definition_number_in_two_mods_clashes() {
        let e = |id: &str, n: u32, on: bool| (id.to_string(), id.to_string(), format!("{}_tiles", id), n, on);
        let c = tiledef_clashes(&[e("A", 7811, true), e("B", 7811, true), e("C", 7811, false), e("D", 500, true), e("A", 7811, true), e("E", 900, true), e("F", 900, false)]);
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].number, c[0].mods.len(), c[0].both_active), (7811, 3, true));
        assert_eq!((c[1].number, c[1].both_active), (900, false));
    }

    /// Live: `cargo test -- --ignored live_map_cells --nocapture` on this PC's game and mods.
    #[test]
    #[ignore]
    fn live_map_cells() {
        let p = crate::vfs::studio_paths();
        let m = map_cells(&p.pz_install_dir, &p.workshop_dir, Path::new(&p.user_zomboid_dir));
        println!("vanilla {} cells", m.vanilla.len());
        if let Ok(out) = std::env::var("PZMS_DUMP") {
            std::fs::write(out, serde_json::to_string(&m).unwrap()).unwrap();
        }
        for f in &m.maps {
            println!("{} / {} ({}) active={} cells={} over_vanilla={} lots={:?}", f.mod_name, f.folder, f.title, f.active, f.cells.len(), f.over_vanilla, f.lots);
        }
        for r in &m.related {
            println!("related {} {:?} active={} {:?}", r.mod_name, r.kinds, r.active, r.detail);
        }
        for t in &m.tiledef_clashes {
            println!("TILEDEF {} both on={} {:?}", t.number, t.both_active, t.mods);
        }
        for c in &m.clashes {
            println!("CLASH {} x {}: {} cells, both on={}", c.a, c.b, c.cells.len(), c.both_active);
        }
    }
}
