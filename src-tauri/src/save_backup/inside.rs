//! What a save holds, read from its own databases: the characters (`players.db`) and the
//! vehicles (`vehicles.db`), with their positions, and the item types they carry.
//!
//! The game keeps each character and each vehicle as a blob written by Java's `writeUTF`
//! (a two-byte length, then the bytes) for every string in it. The blob's layout changes with
//! the build and is not parsed here; the strings in it are enough for the item and vehicle
//! types: every `Module.Type` string is one item (or the vehicle's own script), so the list is
//! read off the names and counted. Approximate, and said so in the UI.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemCount {
    pub item_type: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePlayer {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dead: bool,
    /// Item types found in the character's data, most frequent first.
    pub items: Vec<ItemCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveVehicle {
    pub id: i64,
    /// The vehicle's script ("Base.CarNormal"), or "" when the blob did not say.
    pub script: String,
    pub x: f64,
    pub y: f64,
    /// Squares from the first character, when there is one.
    pub distance: Option<f64>,
    pub items: Vec<ItemCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveInside {
    pub save_id: String,
    pub players: Vec<SavePlayer>,
    pub vehicles: Vec<SaveVehicle>,
}

/// Every string Java's `writeUTF` left in the blob: a big-endian u16 length, then that many
/// printable ASCII bytes. Shorter than two or longer than 120 is not one.
pub fn java_strings(blob: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < blob.len() {
        let len = ((blob[i] as usize) << 8) | blob[i + 1] as usize;
        if (2..=120).contains(&len) && i + 2 + len <= blob.len() {
            let s = &blob[i + 2..i + 2 + len];
            if s.iter().all(|&c| (32..127).contains(&c)) {
                out.push(String::from_utf8_lossy(s).to_string());
                i += 2 + len;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// `Module.Type`: an item's full type (or a vehicle script).
fn is_full_type(s: &str) -> bool {
    let Some((module, name)) = s.split_once('.') else { return false };
    !module.is_empty()
        && !name.is_empty()
        && module.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The item types among the blob's strings, counted, most frequent first; `skip` left out.
pub fn item_types(strings: &[String], skip: Option<&str>) -> Vec<ItemCount> {
    let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
    for s in strings {
        if is_full_type(s) && Some(s.as_str()) != skip {
            *counts.entry(s.as_str()).or_insert(0) += 1;
        }
    }
    let mut out: Vec<ItemCount> = counts.into_iter().map(|(t, c)| ItemCount { item_type: t.to_string(), count: c }).collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.item_type.cmp(&b.item_type)));
    out
}

/// The vehicle's script: the first `Module.Type` string in its blob (the game writes it first).
pub fn vehicle_script(strings: &[String]) -> String {
    strings.iter().find(|s| is_full_type(s)).cloned().unwrap_or_default()
}

fn open(path: &Path) -> Result<rusqlite::Connection, String> {
    rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(|e| format!("Could not open {}: {}", path.display(), e))
}

/// The characters and vehicles of `<Zomboid>/Saves/<save_id>`.
pub fn read(user_zomboid_dir: &Path, save_id: &str) -> Result<SaveInside, String> {
    let (mode, world) = save_id.split_once(['/', '\\']).ok_or("A save id is <mode>/<world>.")?;
    let save = user_zomboid_dir.join("Saves").join(mode).join(world);
    if !save.is_dir() {
        return Err(format!("No save at {}.", save.display()));
    }
    let mut players = Vec::new();
    let players_db = save.join("players.db");
    if players_db.is_file() {
        let conn = open(&players_db)?;
        let mut stmt = conn.prepare("SELECT name, x, y, z, isDead, data FROM localPlayers ORDER BY id").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    r.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                    r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                    r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                    r.get::<_, Option<bool>>(4)?.unwrap_or(false),
                    r.get::<_, Option<Vec<u8>>>(5)?.unwrap_or_default(),
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows.flatten() {
            let (name, x, y, z, dead, blob) = row;
            let strings = java_strings(&blob);
            players.push(SavePlayer { name, x, y, z, dead, items: item_types(&strings, None) });
        }
    }
    let mut vehicles = Vec::new();
    let vehicles_db = save.join("vehicles.db");
    if vehicles_db.is_file() {
        let conn = open(&vehicles_db)?;
        let mut stmt = conn.prepare("SELECT id, x, y, data FROM vehicles ORDER BY id").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                    r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                    r.get::<_, Option<Vec<u8>>>(3)?.unwrap_or_default(),
                ))
            })
            .map_err(|e| e.to_string())?;
        let at = players.first().map(|p| (p.x, p.y));
        for row in rows.flatten() {
            let (id, x, y, blob) = row;
            let strings = java_strings(&blob);
            let script = vehicle_script(&strings);
            let items = item_types(&strings, Some(script.as_str()).filter(|s| !s.is_empty()));
            let distance = at.map(|(px, py)| ((px - x).powi(2) + (py - y).powi(2)).sqrt());
            vehicles.push(SaveVehicle { id, script, x, y, distance, items });
        }
        vehicles.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
    }
    Ok(SaveInside { save_id: save_id.to_string(), players, vehicles })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf(s: &str) -> Vec<u8> {
        let mut v = vec![(s.len() >> 8) as u8, s.len() as u8];
        v.extend_from_slice(s.as_bytes());
        v
    }

    #[test]
    fn strings_are_read_off_a_blob_and_item_types_counted() {
        let mut blob = vec![0u8, 1, 7, 200, 255];
        blob.extend(utf("Base.CarNormal"));
        blob.extend([0, 0, 3]);
        blob.extend(utf("TrunkDoor"));
        blob.extend(utf("Base.Axe"));
        blob.extend([9, 9]);
        blob.extend(utf("Base.Axe"));
        blob.extend(utf("x")); // too short to be one
        let strings = java_strings(&blob);
        assert_eq!(strings, vec!["Base.CarNormal", "TrunkDoor", "Base.Axe", "Base.Axe"]);
        assert_eq!(vehicle_script(&strings), "Base.CarNormal");
        assert_eq!(item_types(&strings, Some("Base.CarNormal")), vec![ItemCount { item_type: "Base.Axe".into(), count: 2 }]);
        assert!(!is_full_type("opening:artillery1") && !is_full_type("Base.") && is_full_type("FR.fr_ch_stepvan_80"));
    }

    #[test]
    fn a_save_id_names_mode_and_world() {
        let dir = std::env::temp_dir().join(format!("pzms_inside_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(read(&dir, "Sandbox").is_err());
        assert!(read(&dir, "Sandbox/none").unwrap_err().starts_with("No save at"));
        std::fs::create_dir_all(dir.join("Saves").join("Sandbox").join("w")).unwrap();
        let empty = read(&dir, "Sandbox/w").expect("a save with no databases");
        assert!(empty.players.is_empty() && empty.vehicles.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
