//! `mod.info` reading that reproduces the game's own rules (ChooseGameInfo.readModInfoAux,
//! 42.20.4), so the Studio sees exactly the id, dependencies and version range the game sees.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameModInfo {
    /// None when there is no `id=` line the game accepts (the game cannot use the mod).
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: String,
    pub poster: Option<String>,
    pub icon: Option<String>,
    pub author: Option<String>,
    pub mod_version: Option<String>,
    pub category: Option<String>,
    pub url: Option<String>,
    pub require: Vec<String>,
    pub incompatible: Vec<String>,
    pub load_after: Vec<String>,
    pub load_before: Vec<String>,
    /// major * 1000 + minor, like GameVersion.getInt().
    pub version_min: Option<u32>,
    pub version_max: Option<u32>,
    /// Set when the game rejects the whole file (e.g. `versionMin=42` without a minor part).
    pub invalid_reason: Option<String>,
}

/// GameVersion.parse: `<major>.<minor><anything>`; anything else is an error.
pub fn parse_game_version(text: &str) -> Option<u32> {
    let (major, rest) = text.split_once('.')?;
    let major: u32 = major.parse().ok()?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let minor: u32 = digits.parse().ok()?;
    Some(major * 1000 + minor)
}

pub fn format_game_version(v: u32) -> String {
    format!("{}.{}", v / 1000, v % 1000)
}

/// A list value: the game removes every backslash and splits on commas. Empty entries and
/// surrounding spaces are dropped here (the game trims when it looks entries up).
fn list_value(line: &str, key: &str) -> Vec<String> {
    line.replace(key, "")
        .replace('\\', "")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Lines the way Java's BufferedReader.readLine splits them (\r\n, \n or a lone \r).
fn java_lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n').flat_map(|l| l.split('\r'))
}

pub fn parse_game_mod_info(text: &str) -> GameModInfo {
    let mut info = GameModInfo::default();
    let non_empty = |s: String| (!s.trim().is_empty()).then_some(s);

    // Same order of checks as the game: the first key a line *contains* wins, so a description
    // line that happens to contain "name=" really does replace the name in game.
    for line in java_lines(text) {
        if line.contains("name=") {
            if let Some(v) = non_empty(line.replace("name=", "")) {
                info.name = Some(v);
            }
        } else if line.contains("poster=") {
            info.poster = non_empty(line.replace("poster=", "").trim().to_string());
        } else if line.contains("description=") {
            info.description.push_str(&line.replace("description=", ""));
        } else if line.contains("require=") {
            info.require = list_value(line, "require=");
        } else if line.contains("incompatible=") {
            info.incompatible = list_value(line, "incompatible=");
        } else if line.contains("loadModAfter=") {
            info.load_after = list_value(line, "loadModAfter=");
        } else if line.contains("loadModBefore=") {
            info.load_before = list_value(line, "loadModBefore=");
        } else if line.starts_with("id=") {
            if let Some(v) = non_empty(line.replace("id=", "").trim().to_string()) {
                info.id = Some(v);
            }
        } else if line.contains("author=") {
            info.author = Some(line.replace("author=", ""));
        } else if line.contains("modversion=") {
            info.mod_version = Some(line.replace("modversion=", ""));
        } else if line.contains("icon=") {
            info.icon = non_empty(line.replace("icon=", "").trim().to_string());
        } else if line.contains("category=") {
            info.category = Some(line.replace("category=", ""));
        } else if line.contains("url=") {
            info.url = Some(line.replace("url=", ""));
        } else if line.starts_with("versionMax=") || line.starts_with("versionMin=") {
            let (key, is_max) = if line.starts_with("versionMax=") { ("versionMax=", true) } else { ("versionMin=", false) };
            let value = line.replace(key, "").trim().to_string();
            if value.is_empty() {
                continue;
            }
            match parse_game_version(&value) {
                Some(v) if is_max => info.version_max = Some(v),
                Some(v) => info.version_min = Some(v),
                None => {
                    info.invalid_reason = Some(format!(
                        "{}{} is not a valid game version (the game needs major.minor, e.g. 42.0); the game rejects this mod.info",
                        key, value
                    ));
                }
            }
        }
    }
    info
}

impl GameModInfo {
    /// ChooseGameInfo.Mod.isAvailableSelf: inside versionMin..versionMax for this build.
    pub fn fits_version(&self, game_version: u32) -> bool {
        self.version_min.is_none_or(|min| min <= game_version) && self.version_max.is_none_or(|max| max >= game_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_like_the_game() {
        let info = parse_game_mod_info(
            "name=My Mod\r\nid= MyMod \r\nrequire=\\LibA, \\LibB,\r\nincompatible=\\Neat_Building\r\nloadModAfter=Viewpoint\r\nversionMin=42.13.0\rversionMax=42.20\n",
        );
        assert_eq!(info.id.as_deref(), Some("MyMod"));
        assert_eq!(info.name.as_deref(), Some("My Mod"));
        assert_eq!(info.require, vec!["LibA", "LibB"]);
        assert_eq!(info.incompatible, vec!["Neat_Building"]);
        assert_eq!(info.load_after, vec!["Viewpoint"]);
        assert_eq!(info.version_min, Some(42_013));
        assert_eq!(info.version_max, Some(42_020));
        assert!(info.invalid_reason.is_none());
        assert!(info.fits_version(42_015) && !info.fits_version(42_021));
    }

    #[test]
    fn reproduces_game_quirks() {
        // Keys are case-sensitive: the game ignores "ID=" and "versionmin=".
        let info = parse_game_mod_info("ID=Upper\nversionmin=42.0\n");
        assert_eq!(info.id, None);
        assert_eq!(info.version_min, None);
        // "versionMin=42" has no minor part: GameVersion.parse throws and the mod is rejected.
        assert!(parse_game_mod_info("id=X\nversionMin=42\n").invalid_reason.is_some());
        // A UTF-8 BOM before "id=" hides the id from startsWith("id=").
        assert_eq!(parse_game_mod_info("\u{feff}id=X\n").id, None);
        // First matching key wins: this description line becomes the name.
        let quirky = parse_game_mod_info("id=X\ndescription=Use rename= to rename\n");
        assert_eq!(quirky.name.as_deref(), Some("description=Use re to rename"));
    }
}
