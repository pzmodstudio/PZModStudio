//! Photos of a profile (its mods, their versions and what Mod Health found), kept per profile in
//! `Zomboid/PZModStudio_ProfileSnapshots/<profile id>/<when>.json`. The app makes and compares
//! them (utils/profileSnapshot.ts); here they are only kept, listed, read and deleted. Names are
//! checked so nothing is ever written or read outside that folder.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const DIR: &str = "PZModStudio_ProfileSnapshots";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SnapshotEntry {
    pub file: String,
    pub taken_at: u64,
    pub label: String,
    pub mods_on: usize,
    pub findings: usize,
    pub game_version: Option<String>,
}

/// A profile id or a file name: letters, digits, `_`, `-` and `.`, never `..` or a separator.
fn plain(name: &str) -> bool {
    !name.is_empty() && !name.contains("..") && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn folder(zomboid: &Path, profile_id: &str) -> Result<PathBuf, String> {
    if !plain(profile_id) {
        return Err(format!("Not a profile id: {}", profile_id));
    }
    Ok(zomboid.join(DIR).join(profile_id))
}

/// Keeps a photo; returns its file name (`<taken_at>.json`, a number added when two share a second).
pub fn save(zomboid: &Path, profile_id: &str, json: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("Not a photo: {}", e))?;
    let taken_at = value.get("taken_at").and_then(|v| v.as_u64()).ok_or("The photo has no taken_at")?;
    let dir = folder(zomboid, profile_id)?;
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {}", dir.display(), e))?;
    let mut name = format!("{}.json", taken_at);
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{}-{}.json", taken_at, n);
        n += 1;
    }
    crate::load_order::ini_parser::atomic_write(dir.join(&name), json).map_err(|e| format!("Cannot keep the photo: {}", e))?;
    Ok(name)
}

/// The profile's photos, newest first.
pub fn list(zomboid: &Path, profile_id: &str) -> Result<Vec<SnapshotEntry>, String> {
    let dir = folder(zomboid, profile_id)?;
    let Ok(entries) = fs::read_dir(&dir) else { return Ok(Vec::new()) };
    let mut out: Vec<SnapshotEntry> = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .filter_map(|e| {
            let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(e.path()).ok()?).ok()?;
            Some(SnapshotEntry {
                file: e.file_name().to_string_lossy().into_owned(),
                taken_at: v.get("taken_at")?.as_u64()?,
                label: v.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string(),
                mods_on: v.get("mods").and_then(|m| m.as_array()).map(|m| m.iter().filter(|x| x.get("on").and_then(|o| o.as_bool()) == Some(true)).count()).unwrap_or(0),
                findings: v.get("findings").and_then(|f| f.as_array()).map(|f| f.len()).unwrap_or(0),
                game_version: v.get("game_version").and_then(|g| g.as_str()).map(str::to_string),
            })
        })
        .collect();
    // Two photos of one second: `<t>-2.json` was kept after `<t>.json`.
    let nth = |f: &str| f.trim_end_matches(".json").rsplit_once('-').and_then(|(_, n)| n.parse::<u32>().ok()).unwrap_or(1);
    out.sort_by(|a, b| b.taken_at.cmp(&a.taken_at).then(nth(&b.file).cmp(&nth(&a.file))));
    Ok(out)
}

pub fn read(zomboid: &Path, profile_id: &str, file: &str) -> Result<String, String> {
    if !plain(file) {
        return Err(format!("Not a photo file: {}", file));
    }
    fs::read_to_string(folder(zomboid, profile_id)?.join(file)).map_err(|e| format!("Cannot read the photo: {}", e))
}

pub fn delete(zomboid: &Path, profile_id: &str, file: &str) -> Result<(), String> {
    if !plain(file) {
        return Err(format!("Not a photo file: {}", file));
    }
    fs::remove_file(folder(zomboid, profile_id)?.join(file)).map_err(|e| format!("Cannot delete the photo: {}", e))
}

/// Every profile's photos: how many, and the bytes they take.
pub fn info(zomboid: &Path) -> (usize, u64) {
    walkdir::WalkDir::new(zomboid.join(DIR))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".json"))
        .fold((0, 0), |(n, b), e| (n + 1, b + e.metadata().map(|m| m.len()).unwrap_or(0)))
}

/// Deletes every profile's photos (Settings' clear-cache button); returns how many went.
pub fn clear_all(zomboid: &Path) -> Result<usize, String> {
    let (n, _) = info(zomboid);
    let dir = zomboid.join(DIR);
    if dir.is_dir() {
        fs::remove_dir_all(&dir).map_err(|e| format!("Cannot delete the photos: {}", e))?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photos_are_kept_listed_read_and_deleted_and_never_outside_their_folder() {
        let z = std::env::temp_dir().join(format!("pzms-photos-{}", std::process::id()));
        let _ = fs::remove_dir_all(&z);
        let photo = |t: u64, label: &str| format!(r#"{{"version":1,"taken_at":{},"label":"{}","game_version":"42.22","mods":[{{"id":"A","on":true}},{{"id":"B","on":false}}],"findings":[{{"key":"k"}}]}}"#, t, label);
        let first = save(&z, "inst_1", &photo(100, "before")).unwrap();
        let second = save(&z, "inst_1", &photo(100, "same second")).unwrap();
        assert_eq!((first.as_str(), second.as_str()), ("100.json", "100-2.json"));
        save(&z, "inst_1", &photo(200, "after")).unwrap();
        let listed = list(&z, "inst_1").unwrap();
        assert_eq!(listed.iter().map(|e| e.label.as_str()).collect::<Vec<_>>(), ["after", "same second", "before"]);
        assert_eq!((listed[0].mods_on, listed[0].findings, listed[0].game_version.as_deref()), (1, 1, Some("42.22")));
        assert!(read(&z, "inst_1", "100.json").unwrap().contains("before"));
        delete(&z, "inst_1", "100.json").unwrap();
        assert_eq!(list(&z, "inst_1").unwrap().len(), 2);
        assert!(list(&z, "other").unwrap().is_empty());
        assert!(save(&z, "../evil", &photo(1, "x")).is_err());
        assert!(read(&z, "inst_1", "..\\..\\secret.json").is_err());
        assert!(save(&z, "inst_1", "not json").is_err());
        assert_eq!(info(&z).0, 2);
        assert_eq!(clear_all(&z).unwrap(), 2);
        assert_eq!(info(&z), (0, 0));
        let _ = fs::remove_dir_all(&z);
    }
}
