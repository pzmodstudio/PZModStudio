//! ZombieBuddy's list of loaded Java mods, drawn over the game's main menu.
//!
//! ZombieBuddy draws "N active Java mods: ..." across the top of the menu, at an opacity the
//! player sets in the game's Options > Mods > ZombieBuddy ("watermark opacity", 0.4 as shipped).
//! With seventy Java mods it covers the screen. The game keeps that option in
//! `Zomboid/Lua/ModOptions.ini` as `slider|ZombieBuddy|watermarkOpacity|<value>`.
//!
//! The Studio hides the list unless asked to show it, and only ever swaps between the two values
//! it knows: the shipped 0.4 and 0. A value the player set in the game is theirs and is left alone.

use std::path::Path;

const KEY: &str = "slider|ZombieBuddy|watermarkOpacity|";
const SHIPPED: f64 = 0.4;

/// What the file says now, and what was done about it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct WatermarkState {
    /// ZombieBuddy's option is in the file (the game has run with ZombieBuddy at least once).
    pub known: bool,
    /// The opacity in the file after this call.
    pub opacity: Option<f64>,
    /// This call changed the file.
    pub changed: bool,
    /// The player chose another value in the game: left as it is.
    pub players_own: bool,
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.001
}

/// Sets the list hidden (`show == false`) or as shipped (`show == true`) in one Zomboid folder.
/// With `apply == false` it only reads.
pub fn set(zomboid: &Path, show: bool, apply: bool) -> Result<WatermarkState, String> {
    let file = zomboid.join("Lua").join("ModOptions.ini");
    let Ok(bytes) = std::fs::read(&file) else {
        return Ok(WatermarkState { known: false, opacity: None, changed: false, players_own: false });
    };
    let text = String::from_utf8_lossy(&bytes).to_string();
    let Some(line) = text.lines().find(|l| l.starts_with(KEY)) else {
        return Ok(WatermarkState { known: false, opacity: None, changed: false, players_own: false });
    };
    let now: f64 = line[KEY.len()..].trim().parse().unwrap_or(SHIPPED);
    let want = if show { SHIPPED } else { 0.0 };
    // Only between the two values the Studio knows; anything else the player set in the game.
    let players_own = !same(now, SHIPPED) && !same(now, 0.0);
    if !apply || players_own || same(now, want) {
        return Ok(WatermarkState { known: true, opacity: Some(now), changed: false, players_own });
    }
    let new_line = format!("{}{}", KEY, if show { "0.4" } else { "0.0" });
    let out = text.replacen(line, &new_line, 1);
    std::fs::write(&file, out).map_err(|e| format!("Cannot write {}: {}", file.display(), e))?;
    Ok(WatermarkState { known: true, opacity: Some(want), changed: true, players_own: false })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(tag: &str, content: Option<&str>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pzms-zbwm-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Lua")).unwrap();
        if let Some(c) = content {
            std::fs::write(dir.join("Lua").join("ModOptions.ini"), c).unwrap();
        }
        dir
    }

    #[test]
    fn the_list_is_hidden_and_shown_only_between_the_two_known_values() {
        let dir = folder("swap", Some("slider|ZombieBuddy|watermarkOpacity|0.4\r\ntickbox|ZombieBuddy|autoFixModOrder|true\r\n"));
        let hidden = set(&dir, false, true).unwrap();
        assert_eq!((hidden.changed, hidden.opacity), (true, Some(0.0)));
        let text = std::fs::read_to_string(dir.join("Lua").join("ModOptions.ini")).unwrap();
        assert_eq!(text, "slider|ZombieBuddy|watermarkOpacity|0.0\r\ntickbox|ZombieBuddy|autoFixModOrder|true\r\n", "the rest of the file and its line ends are kept");
        assert!(!set(&dir, false, true).unwrap().changed, "already hidden");
        let shown = set(&dir, true, true).unwrap();
        assert_eq!((shown.changed, shown.opacity), (true, Some(0.4)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_value_the_player_set_is_left_alone_and_a_missing_option_is_not_made() {
        let own = folder("own", Some("slider|ZombieBuddy|watermarkOpacity|0.75\n"));
        let state = set(&own, false, true).unwrap();
        assert_eq!((state.changed, state.players_own, state.opacity), (false, true, Some(0.75)));
        let none = folder("none", Some("keybind|Viewpoint|keys.firstPerson|24\n"));
        assert_eq!(set(&none, false, true).unwrap(), WatermarkState { known: false, opacity: None, changed: false, players_own: false });
        assert!(!std::fs::read_to_string(none.join("Lua").join("ModOptions.ini")).unwrap().contains("ZombieBuddy"));
        let absent = folder("absent", None);
        assert!(!set(&absent, false, true).unwrap().known);
        for d in [own, none, absent] {
            let _ = std::fs::remove_dir_all(&d);
        }
    }
}
