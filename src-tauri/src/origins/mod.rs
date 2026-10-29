//! Where mods in `Zomboid/mods` came from (Nexus Mods, a URL, a file the user picked).
//!
//! Steam Workshop mods carry their origin in their path; mods installed by hand do not, so the
//! Studio records it when it installs them, in `Zomboid/PZModStudio_Origins.json` (keyed by the
//! folder name under `Zomboid/mods`). Nothing is written inside the mod folders themselves.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "PZModStudio_Origins.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModOrigin {
    /// Downloaded from a Nexus Mods page.
    Nexus { mod_id: u64, page_url: String, name: Option<String> },
    /// Installed from a file or folder the user picked (forum, GitHub, Discord, ...).
    Manual { file_name: Option<String> },
    /// Downloaded from a link the user pasted (a direct zip, or a GitHub release).
    Url {
        /// The file that was downloaded.
        url: String,
        /// A page a person can open to see what it is, when the link gave one.
        page_url: Option<String>,
        /// Where it came from, in words: "the latest release of owner/repo (v1.2.0)".
        source: String,
    },
    /// A copy of another mod adapted by the Studio to run on the installed build.
    Compat {
        /// Workshop item the original comes from, when it is one.
        workshop_id: Option<String>,
        source_dir: String,
        /// What was changed, in plain words.
        changes: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OriginRecord {
    pub origin: ModOrigin,
    /// RFC 3339 time of the install.
    pub installed_at: String,
}

fn origins_path(zomboid_dir: &Path) -> PathBuf {
    zomboid_dir.join(FILE_NAME)
}

/// Folder name (under Zomboid/mods) -> where it came from. Empty when nothing was recorded.
pub fn load(zomboid_dir: &Path) -> BTreeMap<String, OriginRecord> {
    fs::read_to_string(origins_path(zomboid_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Removes the record of a folder (e.g. a compatible copy that was removed).
pub fn forget(zomboid_dir: &Path, folder: &str) -> Result<(), String> {
    let mut all = load(zomboid_dir);
    if all.remove(folder).is_none() {
        return Ok(());
    }
    let text = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    fs::write(origins_path(zomboid_dir), text).map_err(|e| e.to_string())
}

/// Records the origin of freshly installed folders (replacing any older record for them).
pub fn record(zomboid_dir: &Path, folders: &[String], origin: &ModOrigin) -> Result<(), String> {
    let mut all = load(zomboid_dir);
    let installed_at = chrono::Local::now().to_rfc3339();
    for folder in folders {
        all.insert(folder.clone(), OriginRecord { origin: origin.clone(), installed_at: installed_at.clone() });
    }
    let text = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    let path = origins_path(zomboid_dir);
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| format!("Cannot write {}: {}", tmp.display(), e))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_reloads_origins() {
        let dir = std::env::temp_dir().join(format!("pzms-origins-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(load(&dir).is_empty());
        let nexus = ModOrigin::Nexus { mod_id: 445, page_url: "https://www.nexusmods.com/projectzomboid/mods/445".into(), name: Some("GRMT".into()) };
        record(&dir, &["A".into(), "B".into()], &nexus).unwrap();
        record(&dir, &["B".into()], &ModOrigin::Manual { file_name: Some("b.zip".into()) }).unwrap();
        let all = load(&dir);
        assert_eq!(all["A"].origin, nexus);
        assert!(matches!(all["B"].origin, ModOrigin::Manual { .. }), "a newer install replaces the record");
        let _ = fs::remove_dir_all(&dir);
    }
}
