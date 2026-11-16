//! Game functions that two or more active mods redefine, and whether they chain or one cuts the
//! other off.
//!
//! A mod changes a game function (`ISVehicleMenu.showRadialMenu`) either cooperatively — it keeps
//! the original (`local old = ISVehicleMenu.showRadialMenu`) and calls it from its own — or by
//! replacing it outright. Cooperative ones chain: each wraps the one before. A replacing one
//! drops whatever was there: every change made before it, by any mod, never runs, and the game
//! says nothing. Which one is "before" is the order the game runs the files in: the shared, then
//! client, then server phase; within a phase a mod file at a game file's own path runs in the
//! game's turn, then each mod's files in load order; and a definition made inside a function (an
//! event handler, an install() called at OnGameStart) runs after every file has loaded.
//!
//! Read lexically (comments and strings blanked, blocks counted), not with a full parser: fast
//! enough for 15,000 files, and kept per mod in memory until Steam updates it. Measured on a
//! 166-mod list: 58 functions shared, all chaining; the false positives a naive reading gives
//! (a local table named like a game one, `x = x or Original`, a wrap installed at OnGameStart)
//! are each covered by a test below.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// One redefinition of a game function in one mod file.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Redefinition {
    pub mod_id: String,
    pub mod_name: String,
    /// The file, from media/lua (`client/ISUI/ISFoo.lua`).
    pub file: String,
    pub line: u32,
    /// Keeps and calls the original (chains), or replaces it.
    pub keeps_original: bool,
    /// Made inside a function: runs after every file has loaded (from an event, an install()).
    pub deferred: bool,
    /// The file is at a game file's own path: it runs in the game's turn, before the mods'.
    pub replaces_game_file: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SharedFunction {
    /// `ISVehicleMenu.showRadialMenu` (a method `A:b` is written `A.b`).
    pub function: String,
    /// In the order the game runs them.
    pub redefinitions: Vec<Redefinition>,
    /// The mod whose replacement is the last: everything before it is dropped. None: they chain.
    pub winner: Option<String>,
    /// The mods whose change never runs.
    pub cut: Vec<String>,
    /// Moving the winner before these mods would make them wrap it instead (all of them keep the
    /// original, load at file time, in the same phase): the fix is the order alone.
    pub fixed_by_moving_before: Option<String>,
}

// ------------------------------------------------------------------- reading one file

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Punct(char),
    /// `==`, `~=`, `<=`, `>=`, `..`: never an assignment, never a path.
    Op,
}

fn tokens(text: &str) -> Vec<(Tok, u32)> {
    let chars: Vec<char> = crate::api_lint::blank_comments_and_strings(text).chars().collect();
    let mut out = Vec::new();
    let mut line = 1u32;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push((Tok::Ident(chars[start..i].iter().collect()), line));
        } else if c.is_ascii_digit() {
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            out.push((Tok::Op, line));
        } else {
            let next = chars.get(i + 1).copied().unwrap_or(' ');
            if matches!((c, next), ('=', '=') | ('~', '=') | ('<', '=') | ('>', '=') | ('.', '.')) {
                out.push((Tok::Op, line));
                i += 2;
            } else {
                out.push((Tok::Punct(c), line));
                i += 1;
            }
        }
    }
    out
}

/// What one file defines and reads: dotted names it assigns a function to (name, line, inside a
/// function or not), the dotted names it reads as values (keeping an original), the global names
/// it assigns at file level (a game file's tables), and the plain names it declares local.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FileScan {
    pub defs: Vec<(String, u32, bool)>,
    pub reads: BTreeSet<String>,
    pub globals: BTreeSet<String>,
    pub locals: BTreeSet<String>,
}

const KEYWORDS: [&str; 21] = [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until",
];

fn is_kw(s: &str) -> bool {
    KEYWORDS.contains(&s) || s == "while"
}

/// The dotted path starting at `i` (`A`, `.b`, `.c`, `:d`): its parts, whether a `:` closed it,
/// and the index after it.
fn path_at(toks: &[(Tok, u32)], i: usize) -> (Vec<String>, usize) {
    let mut parts = Vec::new();
    let Some((Tok::Ident(first), _)) = toks.get(i) else { return (parts, i) };
    if is_kw(first) {
        return (parts, i);
    }
    parts.push(first.clone());
    let mut j = i + 1;
    while let (Some((Tok::Punct(p), _)), Some((Tok::Ident(n), _))) = (toks.get(j), toks.get(j + 1)) {
        if (*p != '.' && *p != ':') || is_kw(n) {
            break;
        }
        parts.push(n.clone());
        j += 2;
        if *p == ':' {
            break;
        }
    }
    (parts, j)
}

pub fn scan(text: &str) -> FileScan {
    let toks = tokens(text);
    let mut scan = FileScan::default();
    // Open blocks: whether each is a function body, and the name an `if not X then` guards (a
    // fallback defined only when the game's is missing: never a replacement of the game's).
    let mut stack: Vec<(bool, Option<String>)> = Vec::new();
    // `while`/`for` open their block themselves: their `do` opens nothing more.
    let mut loop_do_pending = 0usize;
    let mut i = 0;
    while i < toks.len() {
        let in_function = stack.iter().any(|f| f.0);
        let guarded = |name: &str| stack.iter().filter_map(|b| b.1.as_deref()).any(|g| name == g || name.starts_with(&format!("{}.", g)));
        let prev_is_path_link = i > 0 && matches!(toks[i - 1].0, Tok::Punct('.') | Tok::Punct(':'));
        match &toks[i].0 {
            Tok::Ident(w) if w == "function" => {
                let (parts, after) = path_at(&toks, i + 1);
                if parts.len() >= 2 && matches!(toks.get(after), Some((Tok::Punct('('), _))) {
                    let name = parts.join(".");
                    if !guarded(&name) {
                        scan.defs.push((name, toks[i].1, in_function));
                    }
                } else if parts.len() == 1 && !in_function && !(i > 0 && toks[i - 1].0 == Tok::Ident("local".into())) {
                    // `function Foo()` at file level: a global function (a game file's too).
                    scan.globals.insert(parts[0].clone());
                }
                stack.push((true, None));
                i += 1;
                continue;
            }
            Tok::Ident(w) if w == "local" => {
                // `local a, b = ...` / `local function a`: names that hide a global of that name here.
                let mut j = i + 1;
                if matches!(toks.get(j), Some((Tok::Ident(f), _)) if f == "function") {
                    j += 1;
                }
                while let Some((Tok::Ident(n), _)) = toks.get(j) {
                    if is_kw(n) {
                        break;
                    }
                    scan.locals.insert(n.clone());
                    if matches!(toks.get(j + 1), Some((Tok::Punct(','), _))) {
                        j += 2;
                    } else {
                        break;
                    }
                }
                i += 1;
                continue;
            }
            Tok::Ident(w) if w == "if" => {
                // `if not A.b then` / `if A.b == nil then`: what follows defines A.b only when missing.
                let guard = match toks.get(i + 1) {
                    Some((Tok::Ident(n), _)) if n == "not" => {
                        let (p, end) = path_at(&toks, i + 2);
                        (!p.is_empty() && matches!(toks.get(end), Some((Tok::Ident(t), _)) if t == "then")).then(|| p.join("."))
                    }
                    _ => {
                        let (p, end) = path_at(&toks, i + 1);
                        (!p.is_empty() && matches!(toks.get(end), Some((Tok::Op, _))) && matches!(toks.get(end + 1), Some((Tok::Ident(n), _)) if n == "nil")).then(|| p.join("."))
                    }
                };
                stack.push((false, guard));
            }
            Tok::Ident(w) if w == "repeat" => stack.push((false, None)),
            Tok::Ident(w) if w == "while" || w == "for" => {
                stack.push((false, None));
                loop_do_pending += 1;
            }
            Tok::Ident(w) if w == "do" => {
                if loop_do_pending > 0 {
                    loop_do_pending -= 1;
                } else {
                    stack.push((false, None));
                }
            }
            Tok::Ident(w) if w == "end" || w == "until" => {
                stack.pop();
            }
            Tok::Ident(w) if !is_kw(w) && !prev_is_path_link => {
                let (parts, after) = path_at(&toks, i);
                let assigned = matches!(toks.get(after), Some((Tok::Punct('='), _)));
                if parts.len() >= 2 {
                    let name = parts.join(".");
                    if assigned {
                        // `A.b = function`, or `A.b = myNewFunction` (a local function given its place).
                        match toks.get(after + 1) {
                            Some((Tok::Ident(f), _)) if f == "function" => {
                                if !guarded(&name) {
                                    scan.defs.push((name, toks[i].1, in_function));
                                }
                            }
                            Some((Tok::Ident(v), _)) if !is_kw(v) => {
                                let (value, value_end) = path_at(&toks, after + 1);
                                let next = toks.get(value_end);
                                // `A.b = mine`, not `A.b = x or y`, `A.b = f(...)`, `A.b = t.c`.
                                let continues = matches!(next, Some((Tok::Ident(o), _)) if o == "or" || o == "and");
                                let used_further = matches!(next, Some((Tok::Punct('(' | '[' | '.' | ':'), _)));
                                if value.len() == 1 && !continues && !used_further && !guarded(&name) {
                                    scan.defs.push((name, toks[i].1, in_function));
                                }
                            }
                            _ => {}
                        }
                    } else if !matches!(toks.get(after), Some((Tok::Punct('(' | '{'), _))) {
                        // Read as a value, not called: kept (`local old = A.b`, `x = x or A.b`).
                        scan.reads.insert(name);
                    }
                } else if parts.len() == 1 && assigned && !in_function && stack.is_empty() {
                    scan.globals.insert(parts[0].clone());
                }
                i = after.max(i + 1);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    scan
}

// ------------------------------------------------------------------- the game and the mods

/// What the game's own Lua defines: its functions (dotted names) and its global tables, and the
/// paths of its files (a mod file at one of them replaces it).
#[derive(Debug, Default)]
pub struct GameLua {
    pub functions: HashSet<String>,
    pub globals: HashSet<String>,
    pub files: HashSet<String>,
}

fn lua_files_under(dir: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("lua")))
        .map(|e| e.into_path())
        .collect()
}

/// `client/ISUI/ISFoo.lua` for a file somewhere under a `media/lua` folder.
fn rel_under_lua(path: &Path) -> Option<String> {
    let s = path.to_string_lossy().replace('\\', "/");
    let at = s.to_lowercase().rfind("/media/lua/")?;
    Some(s[at + "/media/lua/".len()..].to_string())
}

type GameKey = (PathBuf, u64);
static GAME: Mutex<Option<(GameKey, Arc<GameLua>)>> = Mutex::new(None);

/// The installed game's Lua, read once per build (the jar's size tells a new one).
pub fn game_lua(install_dir: &Path) -> Arc<GameLua> {
    let key = (install_dir.to_path_buf(), std::fs::metadata(install_dir.join("projectzomboid.jar")).map(|m| m.len()).unwrap_or(0));
    if let Some((k, g)) = GAME.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
        if *k == key {
            return g.clone();
        }
    }
    let mut game = GameLua::default();
    for f in lua_files_under(&install_dir.join("media").join("lua")) {
        let Some(rel) = rel_under_lua(&f) else { continue };
        game.files.insert(rel.to_lowercase());
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let s = scan(&text);
        game.functions.extend(s.defs.into_iter().filter(|d| !d.2).map(|d| d.0));
        game.globals.extend(s.globals);
    }
    let game = Arc::new(game);
    *GAME.lock().unwrap_or_else(|p| p.into_inner()) = Some((key, game.clone()));
    game
}

/// One mod file's redefinitions of game functions: (function, file, line, keeps, deferred, replaces game file).
type ModDef = (String, String, u32, bool, bool, bool);
type ModKey = (String, u64, Option<u64>, Option<u32>);
static MODS: Mutex<Option<HashMap<ModKey, Arc<Vec<ModDef>>>>> = Mutex::new(None);

fn mod_defs(m: &crate::catalog::CatalogMod, game: &GameLua, version: Option<u32>) -> Arc<Vec<ModDef>> {
    let key = m.workshop_state.as_ref().and_then(|s| s.installed_time.map(|t| (m.root_dir.clone(), t, s.size, version)));
    if let Some(key) = &key {
        if let Some(found) = MODS.lock().unwrap_or_else(|p| p.into_inner()).as_ref().and_then(|c| c.get(key)) {
            return found.clone();
        }
    }
    // Kept on disk from an earlier start (mod_cache), until Steam updates the mod or the game changes.
    let disk_key = crate::mod_cache::workshop_key(m, &format!("overrides {:?} {}", version, game.functions.len()));
    if let Some(found) = disk_key.as_deref().and_then(|k| crate::mod_cache::load::<Vec<ModDef>>("overrides", k)) {
        let found = Arc::new(found);
        if let Some(key) = key {
            MODS.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(HashMap::new).insert(key, found.clone());
        }
        return found;
    }
    // The build's layers, the version folder's copy of a path replacing common/'s.
    let mut files: BTreeMap<String, PathBuf> = BTreeMap::new();
    for layer in crate::load_order::mod_info::media_layers_for_build(Path::new(&m.root_dir), version) {
        for f in lua_files_under(&layer.join("media").join("lua")) {
            if let Some(rel) = rel_under_lua(&f) {
                files.insert(rel, f);
            }
        }
    }
    let mut out = Vec::new();
    for (rel, path) in files {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let s = scan(&text);
        for (name, line, deferred) in &s.defs {
            let root = name.split('.').next().unwrap_or("");
            // A game function, on the game's own table (not a local of the same name).
            if !game.functions.contains(name) || !game.globals.contains(root) || s.locals.contains(root) {
                continue;
            }
            out.push((name.clone(), rel.clone(), *line, s.reads.contains(name), *deferred, game.files.contains(&rel.to_lowercase())));
        }
    }
    if let Some(k) = disk_key.as_deref() {
        crate::mod_cache::save("overrides", k, &out);
    }
    let out = Arc::new(out);
    if let Some(key) = key {
        MODS.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(HashMap::new).insert(key, out.clone());
    }
    out
}

fn phase(file: &str) -> u8 {
    match file.split('/').next().unwrap_or("").to_lowercase().as_str() {
        "shared" => 0,
        "client" => 1,
        "server" => 2,
        _ => 3,
    }
}

/// The order the game runs a redefinition in: deferred after everything; then phase; a file at a
/// game path in the game's turn; then load order; then path.
fn run_order(r: &Redefinition, position: usize) -> (bool, u8, usize, String) {
    (r.deferred, phase(&r.file), if r.replaces_game_file { 0 } else { position + 1 }, r.file.to_lowercase())
}

/// Every game function two or more active mods redefine, in run order, with who cuts whom.
pub fn shared_functions(install_dir: &Path, catalog: &crate::catalog::ModCatalog, active: &[String]) -> Vec<SharedFunction> {
    let game = game_lua(install_dir);
    if game.functions.is_empty() {
        return Vec::new();
    }
    let version = catalog.game_version_int();
    let mut by_function: BTreeMap<String, Vec<(usize, Redefinition)>> = BTreeMap::new();
    let mut seen = HashSet::new();
    for (position, entry) in active.iter().enumerate() {
        if entry.starts_with("Z_PZModStudio_") {
            continue;
        }
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if m.loadability != crate::catalog::Loadability::Loadable || !seen.insert(m.id.clone()) {
            continue;
        }
        for (name, file, line, keeps, deferred, replaces) in mod_defs(m, &game, version).iter() {
            by_function.entry(name.clone()).or_default().push((
                position,
                Redefinition {
                    mod_id: m.id.clone(),
                    mod_name: m.name.clone(),
                    file: file.clone(),
                    line: *line,
                    keeps_original: *keeps,
                    deferred: *deferred,
                    replaces_game_file: *replaces,
                },
            ));
        }
    }
    by_function
        .into_iter()
        .filter(|(_, defs)| defs.iter().map(|(_, r)| &r.mod_id).collect::<HashSet<_>>().len() >= 2)
        .map(|(function, mut defs)| {
            defs.sort_by(|a, b| run_order(&a.1, a.0).cmp(&run_order(&b.1, b.0)));
            verdict(function, defs)
        })
        .collect()
}

/// Who cuts whom, from redefinitions already in run order (with their load positions).
pub fn verdict(function: String, defs: Vec<(usize, Redefinition)>) -> SharedFunction {
    let last_replacing = defs.iter().rposition(|(_, r)| !r.keeps_original);
    let (winner, cut, fix) = match last_replacing {
        Some(at) if at > 0 => {
            let w = &defs[at].1;
            // A mod that also redefines it after the winner (in an event, later) wraps the winner: not cut.
            let after: HashSet<&str> = defs[at + 1..].iter().map(|(_, r)| r.mod_id.as_str()).collect();
            let before: Vec<&(usize, Redefinition)> = defs[..at].iter().filter(|(_, r)| r.mod_id != w.mod_id && !after.contains(r.mod_id.as_str())).collect();
            let mut cut: Vec<String> = Vec::new();
            for (_, r) in &before {
                if !cut.contains(&r.mod_id) {
                    cut.push(r.mod_id.clone());
                }
            }
            // The order alone fixes it when the winner runs at file time, not at a game path, and
            // every mod it cuts keeps the original, at file time, in the winner's phase: moved
            // before them, it runs first and they wrap it.
            let movable = !w.deferred
                && !w.replaces_game_file
                && before.iter().all(|(_, r)| r.keeps_original && !r.deferred && !r.replaces_game_file && phase(&r.file) == phase(&w.file));
            let first = before.iter().min_by_key(|(p, _)| *p).map(|(_, r)| r.mod_id.clone());
            if cut.is_empty() {
                (None, cut, None)
            } else {
                (Some(w.mod_id.clone()), cut, if movable { first } else { None })
            }
        }
        _ => (None, Vec::new(), None),
    };
    SharedFunction { function, redefinitions: defs.into_iter().map(|(_, r)| r).collect(), winner, cut, fixed_by_moving_before: fix }
}

#[cfg(test)]
mod live {
    /// The shared functions of a real list. Ignored: needs the PC's game and mods.
    /// PZMS_PATHS=<pz>|<workshop>|<zomboid> PZMS_ORDER=<json array file>
    #[test]
    #[ignore]
    fn shared_functions_of_a_real_list() {
        let (Ok(paths), Ok(order)) = (std::env::var("PZMS_PATHS"), std::env::var("PZMS_ORDER")) else { return };
        let p: Vec<&str> = paths.split('|').collect();
        let active: Vec<String> = serde_json::from_str(&std::fs::read_to_string(order).unwrap()).unwrap();
        let catalog = crate::catalog::build_catalog(p[1], p[2], None);
        let t = std::time::Instant::now();
        let shared = super::shared_functions(std::path::Path::new(p[0]), &catalog, &active);
        eprintln!("{} mods, {} shared functions in {} ms", active.len(), shared.len(), t.elapsed().as_millis());
        let t = std::time::Instant::now();
        let again = super::shared_functions(std::path::Path::new(p[0]), &catalog, &active);
        eprintln!("again: {} in {} ms", again.len(), t.elapsed().as_millis());
        for f in shared.iter().filter(|f| f.winner.is_some()) {
            eprintln!("CUT {}: {} cuts {:?} (fix by moving before: {:?})", f.function, f.winner.as_deref().unwrap_or(""), f.cut, f.fixed_by_moving_before);
            for r in &f.redefinitions {
                eprintln!("    {} {}:{} keeps={} deferred={} gamefile={}", r.mod_id, r.file, r.line, r.keeps_original, r.deferred, r.replaces_game_file);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defs(text: &str) -> Vec<(String, bool)> {
        scan(text).defs.into_iter().map(|d| (d.0, d.2)).collect()
    }

    #[test]
    fn definitions_in_every_shape() {
        let s = scan(
            "local old = ISFoo.bar\nfunction ISFoo:bar(x) return old(self, x) end\nISFoo.baz = function() end\n\
             local function mine() end\nISFoo.qux = mine\nISFoo.call(1)\nlocal t = { a = ISFoo.nope }\n",
        );
        assert_eq!(s.defs.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(), ["ISFoo.bar", "ISFoo.baz", "ISFoo.qux"]);
        assert!(s.reads.contains("ISFoo.bar"), "kept with local old = ...");
        assert!(!s.reads.contains("ISFoo.call"), "a call is not keeping");
        assert!(s.locals.contains("old") && s.locals.contains("mine"));
    }

    #[test]
    fn a_wrap_inside_a_function_is_deferred_and_x_or_original_keeps() {
        let text = "function C.install()\n  local update = ISVehicleMechanics.update\n  function ISVehicleMechanics:update() return update(self) end\nend\n\
                    C.orig = C.orig or ISWorldObjectContextMenu.createMenu\nISWorldObjectContextMenu.createMenu = function(...) return C.orig(...) end\n";
        assert_eq!(
            defs(text),
            [("C.install".to_string(), false), ("ISVehicleMechanics.update".to_string(), true), ("ISWorldObjectContextMenu.createMenu".to_string(), false)]
        );
        let s = scan(text);
        assert!(s.reads.contains("ISWorldObjectContextMenu.createMenu") && s.reads.contains("ISVehicleMechanics.update"));
    }

    #[test]
    fn a_fallback_for_a_missing_game_table_is_no_replacement() {
        let text = "if not ISBaseTimedAction then
  ISBaseTimedAction = {}
  function ISBaseTimedAction:isValid() return true end
end
                    if ISFoo.bar == nil then ISFoo.bar = function() end end
function ISBaseTimedAction:start() end
";
        assert_eq!(defs(text), [("ISBaseTimedAction.start".to_string(), false)]);
    }

    #[test]
    fn strings_comments_and_loops_do_not_fool_it() {
        let text = "-- function ISFoo.a() end\nlocal s = \"ISFoo.b = function\"\nfor i = 1, 3 do\n  if i then end\nend\nISFoo.c = function() end\n";
        assert_eq!(defs(text), [("ISFoo.c".to_string(), false)], "the loop's do opened nothing more: c is at file level");
    }

    fn r(mod_id: &str, keeps: bool, deferred: bool) -> Redefinition {
        Redefinition { mod_id: mod_id.into(), mod_name: mod_id.into(), file: "client/x.lua".into(), line: 1, keeps_original: keeps, deferred, replaces_game_file: false }
    }

    #[test]
    fn chaining_mods_cut_no_one() {
        let v = verdict("F".into(), vec![(0, r("A", true, false)), (1, r("B", true, false))]);
        assert_eq!((v.winner, v.cut), (None, vec![]));
    }

    #[test]
    fn a_replacement_after_others_cuts_them_and_the_order_can_fix_it() {
        let v = verdict("F".into(), vec![(0, r("A", true, false)), (3, r("B", false, false))]);
        assert_eq!((v.winner.as_deref(), v.cut.clone(), v.fixed_by_moving_before.as_deref()), (Some("B"), vec!["A".to_string()], Some("A")));
        // Two replacing: no order makes both run.
        let both = verdict("F".into(), vec![(0, r("A", false, false)), (3, r("B", false, false))]);
        assert_eq!((both.cut, both.fixed_by_moving_before), (vec!["A".to_string()], None));
    }

    #[test]
    fn a_mod_that_wraps_again_later_is_not_cut() {
        let v = verdict("F".into(), vec![(0, r("A", true, false)), (3, r("B", false, false)), (0, r("A", true, true))]);
        assert_eq!(v.winner, None, "A wraps B's version at OnGameStart");
    }

    #[test]
    fn a_replacement_first_and_a_deferred_wrap_after_is_fine() {
        // tsarslib replaces at file time; BlackHawk wraps it at OnGameStart: both run.
        let v = verdict("ISVehicleMechanics.update".into(), vec![(10, r("tsarslib", false, false)), (5, r("BlackHawk", true, true))]);
        assert_eq!(v.winner, None);
    }
}
