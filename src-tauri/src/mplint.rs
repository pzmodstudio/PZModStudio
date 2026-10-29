//! The multiplayer check: what a mod's client code does directly that the game, in
//! multiplayer, only does on the server.
//!
//! The facts come from the game's own Lua (42.21): where client code would change the world, a
//! vehicle, an animal or the map, it does `if isClient() then sendClientCommand(...) else <do
//! it> end`, and `media/lua/server/ClientCommands.lua` and `Vehicles/VehicleCommands.lua` hold
//! the server's side — one handler per command, each calling the Java methods that do the work.
//! Those methods, called straight from a mod's client or shared Lua with no `isClient()` branch
//! around them, change only that client's copy of the world in multiplayer, or are refused.
//! Each rule names the handler, so every finding can be checked against the game's code.
//!
//! What the check does not claim: a player's own inventory (the engine syncs it), timed actions
//! (`forceComplete` and the like run under `isServer()` by design), and anything whose method
//! name is too plain to mean one thing (`rotate`, `remove`, `set`, `setCustomName` — items have
//! one too), nor the `transmit*` primitives, which multiplayer-aware mods call on purpose, nor
//! fluids and `setExplored`, which the engine syncs for items and the UI sets freely. Tried on
//! 113 real mods (2026-10-18): those were the noise. A file that mentions
//! `isClient()` or `sendClientCommand` anywhere is marked "aware": its author knows about
//! multiplayer, and the finding is a note, not a warning.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// One method the server does for the client in multiplayer.
pub struct Rule {
    /// `:method(` as it appears in Lua.
    pub method: &'static str,
    /// What it is about, for the reader.
    pub what: &'static str,
    /// The server handler that does it: `Commands.object.removeBush`.
    pub handler: &'static str,
    /// The file the handler lives in, under media/lua.
    pub file: &'static str,
}

const CC: &str = "server/ClientCommands.lua";
const VC: &str = "server/Vehicles/VehicleCommands.lua";

/// Read from the two handler files of 42.21; only methods whose name means one thing.
pub const RULES: &[Rule] = &[
    // The world
    Rule { method: "removeErosionObject", what: "removing a bush or erosion object", handler: "Commands.object.removeBush", file: CC },
    Rule { method: "toggleCurtain", what: "opening or closing a curtain", handler: "Commands.object.openCloseCurtain", file: CC },
    Rule { method: "setUsesExternalWaterSource", what: "plumbing an object", handler: "Commands.object.plumbObject", file: CC },
    Rule { method: "setExplosionTimer", what: "setting a bomb's timer", handler: "Commands.object.setBombTimer", file: CC },
    Rule { method: "setFuelAmount", what: "fuelling a fireplace or barbecue", handler: "Commands.fireplace.setFuel / Commands.bbq.setFuel", file: CC },
    Rule { method: "setMaxTemperature", what: "setting a stove", handler: "Commands.stove.setOvenParamsAndToggle", file: CC },
    Rule { method: "becomeCorpseItem", what: "turning a body into an item", handler: "Commands.deadBody.addBody", file: CC },
    Rule { method: "triggerThunderEvent", what: "triggering thunder", handler: "Commands.event.thunder", file: CC },
    Rule { method: "disableErosion", what: "disabling erosion on a square", handler: "Commands.erosion.disableForSquare", file: CC },
    // The player
    Rule { method: "setKnownInSquares", what: "revealing map", handler: "Commands.map.setKnownInSquares", file: CC },
    // Animals
    Rule { method: "killAnimal", what: "killing an animal", handler: "Commands.animal.kill", file: CC },
    Rule { method: "setWoolQuantity", what: "an animal's wool", handler: "Commands.animal.setWool", file: CC },
    Rule { method: "setMilkQuantity", what: "an animal's milk", handler: "Commands.animal.setMilk", file: CC },
    Rule { method: "setPregnant", what: "an animal's pregnancy", handler: "Commands.animal.pregnant", file: CC },
    Rule { method: "setFertilized", what: "an animal's fertilisation", handler: "Commands.animal.fertilized", file: CC },
    Rule { method: "addAnimalInside", what: "putting an animal in a hutch", handler: "Commands.animal.hutch", file: CC },
    Rule { method: "setHutchDirt", what: "a hutch's dirt", handler: "Commands.hutch.dirt", file: CC },
    // Vehicles
    Rule { method: "tryStartEngine", what: "starting a vehicle's engine", handler: "Commands.startEngine", file: VC },
    Rule { method: "setHeadlightsOn", what: "a vehicle's headlights", handler: "Commands.setHeadlightsOn", file: VC },
    Rule { method: "setStoplightsOn", what: "a vehicle's stop lights", handler: "Commands.setStoplightsOn", file: VC },
    Rule { method: "setTrunkLocked", what: "a vehicle's trunk lock", handler: "Commands.setTrunkLocked", file: VC },
    Rule { method: "setTireInflation", what: "a tire's pressure", handler: "Commands.setTirePressure", file: VC },
    Rule { method: "setLightbarLightsMode", what: "a lightbar's lights", handler: "Commands.setLightbarLightsMode", file: VC },
    Rule { method: "setLightbarSirenMode", what: "a lightbar's siren", handler: "Commands.setLightbarSirenMode", file: VC },
    Rule { method: "putKeyInIgnition", what: "a vehicle key in the ignition", handler: "Commands.putKeyInIgnition", file: VC },
    Rule { method: "removeKeyFromIgnition", what: "a vehicle key out of the ignition", handler: "Commands.removeKeyFromIgnition", file: VC },
    Rule { method: "putKeyOnDoor", what: "a vehicle key on the door", handler: "Commands.putKeyOnDoor", file: VC },
    Rule { method: "removeKeyFromDoor", what: "a vehicle key off the door", handler: "Commands.removeKeyFromDoor", file: VC },
    Rule { method: "createVehicleKey", what: "making a vehicle key", handler: "Commands.getKey", file: VC },
    Rule { method: "permanentlyRemove", what: "removing a vehicle", handler: "Commands.remove", file: VC },
    Rule { method: "setBloodIntensity", what: "blood on a vehicle", handler: "Commands.setBloodIntensity", file: VC },
    Rule { method: "setRust", what: "rust on a vehicle", handler: "Commands.setRust", file: VC },
    Rule { method: "setColorHSV", what: "a vehicle's colour", handler: "Commands.setHSV", file: VC },
    Rule { method: "setSkinIndex", what: "a vehicle's skin", handler: "Commands.setSkinIndex", file: VC },
    Rule { method: "setAlarmed", what: "a vehicle's alarm", handler: "Commands.setAlarmed", file: VC },
    Rule { method: "addPointConstraint", what: "attaching a trailer", handler: "Commands.attachTrailer", file: VC },
    Rule { method: "breakConstraint", what: "detaching a trailer", handler: "Commands.detachTrailer", file: VC },
    Rule { method: "cheatHotwire", what: "hotwiring a vehicle", handler: "Commands.cheatHotwire", file: VC },
    Rule { method: "setFocusingUp", what: "a headlight's focus", handler: "Commands.configHeadlight", file: VC },
    Rule { method: "damageObjects", what: "a vehicle damaging objects", handler: "Commands.damageObjects", file: VC },
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MpFinding {
    /// From the mod's root, with forward slashes.
    pub path: String,
    pub line: u32,
    pub method: String,
    pub what: String,
    pub handler: String,
    /// The game file with the handler, under media/lua.
    pub evidence: String,
    /// The line as written, trimmed.
    pub snippet: String,
    /// The call sits in an `if` that mentions isClient/isServer: the author handles multiplayer there.
    pub guarded: bool,
    /// The file mentions isClient() or sendClientCommand somewhere: the author knows about multiplayer.
    pub aware: bool,
    /// The game's own file at the same path makes the same call: the mod copied a vanilla file,
    /// so this line is the game's, not the mod's doing.
    #[serde(default)]
    pub same_as_game: bool,
}

/// Whether the mod would work in multiplayer, as far as the check can see.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MpVerdict {
    /// The author says single player only.
    No,
    /// Some action would stay on one client: an unhandled call in a file that knows nothing of multiplayer.
    Partial,
    /// Nothing seen against it.
    #[default]
    LikelyOk,
    /// The author says it works in multiplayer.
    Ok,
}

/// What the mod's own description says about multiplayer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MpDeclared {
    SpOnly,
    MpOk,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MpReport {
    pub root: String,
    pub files_checked: usize,
    /// The mod ships server-side Lua (media/lua/server), where these calls are right.
    pub has_server_lua: bool,
    pub findings: Vec<MpFinding>,
    pub verdict: MpVerdict,
    pub declared: Option<MpDeclared>,
    /// The words of the description the declaration was read from.
    pub declared_quote: Option<String>,
}

impl MpFinding {
    /// Counts against the mod: not handled, in a file that knows nothing of multiplayer, and
    /// not the game's own line.
    pub fn counts_against(&self) -> bool {
        !self.guarded && !self.aware && !self.same_as_game
    }
}

/// Phrases an author writes when the mod is for single player only. Checked before the
/// positive ones, since "not multiplayer compatible" contains "multiplayer compatible".
const SP_ONLY: &[&str] = &[
    "single player only", "singleplayer only", "single-player only", "sp only", "solo only",
    "not multiplayer", "no multiplayer", "not mp compatible", "not for multiplayer", "not for mp",
    "doesn't work in multiplayer", "does not work in multiplayer", "doesn't work on servers", "does not work on servers",
    "multiplayer not supported", "multiplayer is not supported", "no mp support", "without multiplayer support",
    "not compatible with multiplayer", "incompatible with multiplayer", "not tested in multiplayer",
    "solo un jugador", "solo para un jugador", "no funciona en multijugador", "no es compatible con multijugador",
];

/// Phrases an author writes when the mod is made for multiplayer.
const MP_OK: &[&str] = &[
    "multiplayer compatible", "mp compatible", "works in multiplayer", "works in mp", "works on servers",
    "multiplayer support", "multiplayer supported", "supports multiplayer", "mp ready", "multiplayer ready",
    "server compatible", "compatible with multiplayer", "tested in multiplayer", "multiplayer friendly",
    "compatible con multijugador", "funciona en multijugador",
];

/// What a description says about multiplayer, with the words it says it in.
pub fn declared_mp(description: &str) -> Option<(MpDeclared, String)> {
    let text = description.to_lowercase();
    let find = |phrases: &[&str]| phrases.iter().find(|p| text.contains(*p)).map(|p| p.to_string());
    if let Some(p) = find(SP_ONLY) {
        return Some((MpDeclared::SpOnly, p));
    }
    find(MP_OK).map(|p| (MpDeclared::MpOk, p))
}

/// The game's own path for a mod file: the version folder (`42/`, `42.15/`) or `common/` taken
/// off, so `42/media/lua/client/x.lua` is `media/lua/client/x.lua` under the game's folder.
pub fn game_relative(rel: &str) -> Option<String> {
    if rel.starts_with("media/") {
        return Some(rel.to_string());
    }
    let (first, rest) = rel.split_once('/')?;
    let version_folder = first == "common" || (!first.is_empty() && first.chars().all(|c| c.is_ascii_digit() || c == '.'));
    (version_folder && rest.starts_with("media/")).then(|| rest.to_string())
}

fn verdict(report: &MpReport) -> MpVerdict {
    match report.declared {
        Some(MpDeclared::SpOnly) => MpVerdict::No,
        Some(MpDeclared::MpOk) => MpVerdict::Ok,
        None if report.findings.iter().any(MpFinding::counts_against) => MpVerdict::Partial,
        None => MpVerdict::LikelyOk,
    }
}

fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with("--")
}

/// Whether the nearest enclosing `if`/`elseif` (by indentation, looking up) mentions the
/// multiplayer checks. Lua has no fixed indentation, so this is the shape of the game's own
/// code and of most mods, not a parse.
fn guarded_by_mp_check(lines: &[&str], at: usize) -> bool {
    let indent = |l: &str| l.len() - l.trim_start().len();
    let my = indent(lines[at]);
    let mut i = at;
    while i > 0 {
        i -= 1;
        let l = lines[i];
        if l.trim().is_empty() || is_comment(l) {
            continue;
        }
        if indent(l) < my {
            let t = l.trim_start();
            if t.starts_with("if ") || t.starts_with("elseif ") {
                return t.contains("isClient()") || t.contains("isServer()");
            }
            if t.starts_with("else") {
                // The else of an `if isClient()`: find that if.
                let else_indent = indent(l);
                let mut k = i;
                while k > 0 {
                    k -= 1;
                    let lk = lines[k];
                    if !lk.trim().is_empty() && indent(lk) == else_indent && lk.trim_start().starts_with("if ") {
                        return lk.contains("isClient()") || lk.contains("isServer()");
                    }
                }
                return false;
            }
            if t.starts_with("function") || t.starts_with("local function") || t.starts_with("end") {
                return false;
            }
        }
    }
    false
}

/// Checks one Lua file's text; `path` is only carried into the findings.
pub fn check_text(path: &str, text: &str) -> Vec<MpFinding> {
    let lines: Vec<&str> = text.lines().collect();
    let aware = text.contains("isClient()") || text.contains("sendClientCommand") || text.contains("isServer()");
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if is_comment(line) {
            continue;
        }
        for rule in RULES {
            let needle = format!(":{}(", rule.method);
            if !line.contains(&needle) {
                continue;
            }
            out.push(MpFinding {
                path: path.to_string(),
                line: (i + 1) as u32,
                method: rule.method.to_string(),
                what: rule.what.to_string(),
                handler: rule.handler.to_string(),
                evidence: rule.file.to_string(),
                snippet: line.trim().chars().take(160).collect(),
                guarded: guarded_by_mp_check(&lines, i),
                aware,
                same_as_game: false,
            });
        }
    }
    out
}

/// Every client and shared Lua file of the mod (server Lua is where these calls belong, so it
/// is only noted), checked; no game folder to compare with and no description.
pub fn check_mod(root: &Path) -> Result<MpReport, String> {
    assess(root, None, None)
}

/// The check with a verdict: the findings, each compared with the game's own file at the same
/// path when the game folder is known (a copied vanilla file is not the mod's doing), and what
/// the mod's description says about multiplayer.
pub fn assess(root: &Path, game_dir: Option<&Path>, description: Option<&str>) -> Result<MpReport, String> {
    if !root.is_dir() {
        return Err(format!("{} is not a folder.", root.display()));
    }
    let mut report = MpReport { root: root.to_string_lossy().to_string(), ..Default::default() };
    let mut game_files: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
    for entry in walkdir::WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !path.extension().map(|e| e.eq_ignore_ascii_case("lua")).unwrap_or(false) {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
        let lower = rel.to_lowercase();
        if lower.contains("/media/lua/server/") || lower.starts_with("media/lua/server/") {
            report.has_server_lua = true;
            continue;
        }
        if !(lower.contains("/media/lua/client/") || lower.starts_with("media/lua/client/") || lower.contains("/media/lua/shared/") || lower.starts_with("media/lua/shared/")) {
            continue;
        }
        report.files_checked += 1;
        if let Ok(text) = std::fs::read_to_string(path) {
            let mut found = check_text(&rel, &text);
            if let (Some(game), Some(game_rel)) = (game_dir, game_relative(&rel)) {
                let game_text = game_files
                    .entry(game_rel.clone())
                    .or_insert_with(|| std::fs::read_to_string(game.join(&game_rel)).ok())
                    .clone();
                if let Some(game_text) = game_text {
                    for f in &mut found {
                        f.same_as_game = game_text.contains(&format!(":{}(", f.method));
                    }
                }
            }
            report.findings.extend(found);
        }
    }
    report.findings.sort_by(|a, b| (!a.counts_against(), &a.path, a.line).cmp(&(!b.counts_against(), &b.path, b.line)));
    if let Some((declared, quote)) = description.and_then(declared_mp) {
        report.declared = Some(declared);
        report.declared_quote = Some(quote);
    }
    report.verdict = verdict(&report);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_direct_world_change_is_found_and_the_guarded_one_is_marked() {
        // Indentation matters to the guard check, so the lines are joined, not continued.
        let text = [
            "local function burn(square)",
            "    square:getObjects():get(0):setFuelAmount(10)",
            "end",
            "local function burnMp(square)",
            "    if isClient() then",
            "        sendClientCommand(getPlayer(), 'fireplace', 'setFuel', {})",
            "    else",
            "        square:getObjects():get(0):setFuelAmount(10)",
            "    end",
            "end",
            "-- vehicle:setHeadlightsOn(true) in a comment",
        ]
        .join("\n");
        let text = text.as_str();
        let f = check_text("media/lua/client/x.lua", text);
        assert_eq!(f.len(), 2, "{:#?}", f);
        assert_eq!((f[0].line, f[0].guarded, f[0].aware), (2, false, true));
        assert_eq!((f[1].line, f[1].guarded), (8, true));
        assert_eq!(f[0].handler, "Commands.fireplace.setFuel / Commands.bbq.setFuel");
        let blind = check_text("x.lua", "car:setHeadlightsOn(true)\n");
        assert!(!blind[0].aware && !blind[0].guarded);
    }

    #[test]
    fn server_lua_is_skipped_and_only_client_and_shared_are_checked() {
        let dir = std::env::temp_dir().join(format!("pzms-mplint-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for d in ["42/media/lua/client", "42/media/lua/server", "42/media/lua/shared"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        std::fs::write(dir.join("42/media/lua/client/a.lua"), "v:setRust(1)\n").unwrap();
        std::fs::write(dir.join("42/media/lua/server/b.lua"), "v:setRust(1)\n").unwrap();
        std::fs::write(dir.join("42/media/lua/shared/c.lua"), "local x = 1\n").unwrap();
        let r = check_mod(&dir).unwrap();
        assert_eq!(r.files_checked, 2);
        assert!(r.has_server_lua);
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.findings[0].path, "42/media/lua/client/a.lua");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_copied_vanilla_file_is_the_games_line_and_the_verdict_says_nothing_against() {
        let dir = std::env::temp_dir().join(format!("pzms-mplint-game-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let game = dir.join("game");
        let m = dir.join("mod");
        std::fs::create_dir_all(game.join("media/lua/client/ISUI")).unwrap();
        std::fs::create_dir_all(m.join("42.15/media/lua/client/ISUI")).unwrap();
        std::fs::create_dir_all(m.join("42/media/lua/client")).unwrap();
        std::fs::write(game.join("media/lua/client/ISUI/Menu.lua"), "item:setExplosionTimer(seconds)\n").unwrap();
        std::fs::write(m.join("42.15/media/lua/client/ISUI/Menu.lua"), "-- changed\nitem:setExplosionTimer(seconds)\n").unwrap();
        std::fs::write(m.join("42/media/lua/client/Own.lua"), "car:setRust(1)\n").unwrap();
        let r = assess(&m, Some(&game), None).unwrap();
        let copied = r.findings.iter().find(|f| f.path.ends_with("Menu.lua")).unwrap();
        let own = r.findings.iter().find(|f| f.path.ends_with("Own.lua")).unwrap();
        assert!(copied.same_as_game && !copied.counts_against());
        assert!(!own.same_as_game && own.counts_against());
        assert_eq!(r.verdict, MpVerdict::Partial);
        // The mod's own line comes first; the game's is a note.
        assert_eq!(r.findings[0].path, own.path);
        // Without the game's own line the mod has nothing against it.
        std::fs::remove_file(m.join("42/media/lua/client/Own.lua")).unwrap();
        assert_eq!(assess(&m, Some(&game), None).unwrap().verdict, MpVerdict::LikelyOk);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_the_author_says_decides_before_the_code() {
        assert_eq!(declared_mp("A cozy mod. Singleplayer only!"), Some((MpDeclared::SpOnly, "singleplayer only".to_string())));
        assert_eq!(declared_mp("NOT multiplayer compatible yet"), Some((MpDeclared::SpOnly, "not multiplayer".to_string())));
        assert_eq!(declared_mp("Multiplayer compatible, tested on a 10-player server."), Some((MpDeclared::MpOk, "multiplayer compatible".to_string())));
        assert_eq!(declared_mp("Adds hats."), None);
        let dir = std::env::temp_dir().join(format!("pzms-mplint-said-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("42/media/lua/client")).unwrap();
        std::fs::write(dir.join("42/media/lua/client/a.lua"), "v:setRust(1)\n").unwrap();
        assert_eq!(assess(&dir, None, Some("Singleplayer only")).unwrap().verdict, MpVerdict::No);
        assert_eq!(assess(&dir, None, Some("Works in multiplayer")).unwrap().verdict, MpVerdict::Ok);
        assert_eq!(assess(&dir, None, Some("Adds hats")).unwrap().verdict, MpVerdict::Partial);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_games_path_drops_the_version_folder() {
        assert_eq!(game_relative("42/media/lua/client/x.lua").as_deref(), Some("media/lua/client/x.lua"));
        assert_eq!(game_relative("42.15/media/lua/shared/y.lua").as_deref(), Some("media/lua/shared/y.lua"));
        assert_eq!(game_relative("common/media/lua/client/z.lua").as_deref(), Some("media/lua/client/z.lua"));
        assert_eq!(game_relative("media/lua/client/w.lua").as_deref(), Some("media/lua/client/w.lua"));
        assert_eq!(game_relative("docs/notes.lua"), None);
    }

    #[test]
    fn every_rule_names_a_handler_in_one_of_the_two_files() {
        for r in RULES {
            assert!(r.handler.starts_with("Commands."), "{}", r.handler);
            assert!(r.file == CC || r.file == VC);
            assert!(r.method.len() > 6, "a short name would match too much: {}", r.method);
        }
    }
}
