//! Names a mod's Lua calls that the installed build does not have, before the game is launched.
//!
//! The facts are the game's own: `projectzomboid.jar` holds every Java class with its methods
//! (the classes `LuaManager$Exposer` hands to Lua, and the rest, since objects of any class come
//! back from them), `LuaManager$GlobalObject`'s methods are the global functions (`getPlayer`,
//! `getCore`...), the game's `media/lua` files define the Lua globals and methods of the vanilla
//! UI and systems, and the events are the `On...` names the Java triggers plus what
//! `LuaEventManager.AddEvent` adds. Read once per build and kept in
//! `PZModStudio_Cache/api/<jar>.json`.
//!
//! The check is by name only, on the text of each Lua file with comments and strings blanked:
//! a global call `foo(` whose name no one defines (not the game, not the mod, not another
//! active mod, not Lua itself, not a local or parameter of the mod), a method call `:foo(`
//! whose name no Java class and no Lua file defines, an `Events.X.Add` whose X nothing triggers
//! or adds. Kahlua accepts a nil global until it is called, so every row here is a call that
//! would fail at that line. What the check cannot see: a name built at run time (`_G[n]`,
//! `obj[n]()`), and whether a method exists on *that* object (a name that some class has
//! passes). A mod written for Build 41 against the 132 Lua files and the events Build 42
//! dropped is the first case this is for.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

/// What the installed build has, by name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameApi {
    /// The jar's size and date: the cache is for this one.
    pub jar_key: String,
    pub classes: BTreeSet<String>,
    /// Every method name of every class in the jar, plus the Java collection basics.
    pub methods: BTreeSet<String>,
    /// Global functions and tables: `LuaManager$GlobalObject`'s methods, the exposed classes'
    /// simple names, the game Lua's top-level definitions, Lua's own.
    pub globals: BTreeSet<String>,
    pub events: BTreeSet<String>,
    pub class_files: usize,
    pub lua_files: usize,
}

/// One class as the jar has it.
struct ClassInfo {
    name: String,
    /// Method names: the Java ones and, where Kahlua's `@LuaMethod(name = "x")` renames one for
    /// Lua, that name too (`openURl` is `openUrl` to Lua).
    methods: Vec<String>,
    /// Every Utf8 constant: string literals among them.
    strings: Vec<String>,
}

/// Skips one `element_value` of an annotation, returning the string it names when it is a
/// plain string constant.
fn element_value(bytes: &[u8], i: &mut usize, pool: &[Option<String>]) -> Option<Option<String>> {
    let tag = *bytes.get(*i)?;
    *i += 1;
    match tag {
        b's' => {
            let idx = u16_at(bytes, *i)?;
            *i += 2;
            Some(pool.get(idx).cloned().flatten())
        }
        b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b'c' => {
            *i += 2;
            Some(None)
        }
        b'e' => {
            *i += 4;
            Some(None)
        }
        b'@' => {
            annotation(bytes, i, pool)?;
            Some(None)
        }
        b'[' => {
            let n = u16_at(bytes, *i)?;
            *i += 2;
            for _ in 0..n {
                element_value(bytes, i, pool)?;
            }
            Some(None)
        }
        _ => None,
    }
}

/// Reads one annotation; returns the `name` element of a `LuaMethod` annotation when it has one.
fn annotation(bytes: &[u8], i: &mut usize, pool: &[Option<String>]) -> Option<Option<String>> {
    let type_idx = u16_at(bytes, *i)?;
    let pairs = u16_at(bytes, *i + 2)?;
    *i += 4;
    let is_lua = pool.get(type_idx).cloned().flatten().map(|t| t.ends_with("/LuaMethod;")).unwrap_or(false);
    let mut found = None;
    for _ in 0..pairs {
        let name_idx = u16_at(bytes, *i)?;
        *i += 2;
        let value = element_value(bytes, i, pool)?;
        if is_lua && pool.get(name_idx).cloned().flatten().as_deref() == Some("name") {
            found = value;
        }
    }
    Some(found)
}

/// The Lua names a method's `RuntimeVisibleAnnotations` attribute gives it.
fn lua_names(attr: &[u8], pool: &[Option<String>]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(n) = u16_at(attr, 0) else { return out };
    let mut i = 2;
    for _ in 0..n {
        match annotation(attr, &mut i, pool) {
            Some(Some(name)) => out.push(name),
            Some(None) => {}
            None => break,
        }
    }
    out
}

fn u16_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]) as usize)
}

/// The class file's name, method names and constants.
fn parse_class(bytes: &[u8]) -> Option<ClassInfo> {
    if bytes.len() < 10 || bytes[..4] != [0xCA, 0xFE, 0xBA, 0xBE] {
        return None;
    }
    let count = u16_at(bytes, 8)?;
    let mut pool: Vec<Option<String>> = vec![None; count];
    let mut classes: Vec<(usize, usize)> = Vec::new(); // (slot, name index)
    let mut i = 10;
    let mut slot = 1;
    while slot < count {
        let tag = *bytes.get(i)?;
        i += 1;
        let size = match tag {
            1 => {
                let len = u16_at(bytes, i)?;
                let s = bytes.get(i + 2..i + 2 + len)?;
                pool[slot] = Some(String::from_utf8_lossy(s).to_string());
                2 + len
            }
            3 | 4 => 4,
            5 | 6 => {
                slot += 1;
                8
            }
            7 => {
                classes.push((slot, u16_at(bytes, i)?));
                2
            }
            8 | 16 | 19 | 20 => 2,
            9 | 10 | 11 | 12 | 17 | 18 => 4,
            15 => 3,
            _ => return None,
        };
        i += size;
        slot += 1;
    }
    let _access = u16_at(bytes, i)?;
    let this = u16_at(bytes, i + 2)?;
    let name = classes.iter().find(|(s, _)| *s == this).and_then(|(_, n)| pool.get(*n).cloned().flatten())?;
    let ifaces = u16_at(bytes, i + 6)?;
    i += 8 + 2 * ifaces;
    // fields, then methods: access, name, descriptor, attributes
    let mut methods = Vec::new();
    for pass in 0..2 {
        let n = u16_at(bytes, i)?;
        i += 2;
        for _ in 0..n {
            let name_idx = u16_at(bytes, i + 2)?;
            let attrs = u16_at(bytes, i + 6)?;
            i += 8;
            for _ in 0..attrs {
                let attr_name = u16_at(bytes, i)?;
                let len = u32::from_be_bytes([*bytes.get(i + 2)?, *bytes.get(i + 3)?, *bytes.get(i + 4)?, *bytes.get(i + 5)?]) as usize;
                if pass == 1 && pool.get(attr_name).cloned().flatten().as_deref() == Some("RuntimeVisibleAnnotations") {
                    if let Some(attr) = bytes.get(i + 6..i + 6 + len) {
                        methods.extend(lua_names(attr, &pool));
                    }
                }
                i += 6 + len;
            }
            if pass == 1 {
                if let Some(Some(m)) = pool.get(name_idx) {
                    if !m.starts_with('<') {
                        methods.push(m.clone());
                    }
                }
            }
        }
    }
    Some(ClassInfo { name, methods, strings: pool.into_iter().flatten().collect() })
}

/// What Lua itself gives every script (Kahlua's stdlib and the game's `stdlib.lua`).
const LUA_BUILTINS: &[&str] = &[
    "assert", "collectgarbage", "dofile", "error", "getmetatable", "ipairs", "load", "loadstring", "next", "pairs", "pcall", "print",
    "rawequal", "rawget", "rawset", "rawlen", "require", "select", "setmetatable", "tonumber", "tostring", "type", "unpack", "xpcall",
    "string", "table", "math", "os", "io", "coroutine", "debug", "_G", "_VERSION", "newproxy", "setfenv", "getfenv", "module",
    "luanet", "java", "kahlua", "Events", "LuaEventManager", "triggerEvent", "ZombRand", "ZombRandFloat", "ZombRandBetween",
];

/// Methods of the Java standard library that game objects return (lists, maps, strings, enums).
const JAVA_BASICS: &[&str] = &[
    "size", "get", "add", "remove", "contains", "isEmpty", "iterator", "hasNext", "next", "toString", "equals", "hashCode", "getClass",
    "getName", "getSimpleName", "clear", "put", "keySet", "values", "containsKey", "containsValue", "entrySet", "getKey", "getValue",
    "length", "charAt", "substring", "indexOf", "lastIndexOf", "split", "trim", "toLowerCase", "toUpperCase", "startsWith", "endsWith",
    "replace", "replaceAll", "matches", "intValue", "floatValue", "doubleValue", "longValue", "booleanValue", "compareTo", "ordinal",
    "name", "valueOf", "stream", "forEach", "isPresent", "orElse", "addAll", "removeAll", "indexOfKey", "set", "insert", "push", "pop",
    "peek", "toArray", "sort", "reverse", "first", "last", "clone", "compare", "max", "min", "abs", "floor", "ceil", "round", "format",
    "getBytes", "contentEquals", "equalsIgnoreCase", "concat", "isBlank", "strip", "join", "chars", "nextInt", "nextFloat", "nextBoolean",
    "getX", "getY", "getZ", "setX", "setY", "setZ", "x", "y", "z", "width", "height", "getWidth", "getHeight", "getTime", "getSeconds",
];

const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in", "local", "nil", "not", "or", "repeat",
    "return", "then", "true", "until", "while",
];

fn jar_key(jar: &Path) -> Option<String> {
    let meta = std::fs::metadata(jar).ok()?;
    let mtime = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(format!("{}-{}", meta.len(), mtime))
}

/// Blank comments and string literals (same length, line breaks kept) so names inside them
/// are not read as calls.
pub fn blank_comments_and_strings(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let blank = |out: &mut String, c: char| out.push(if c == '\n' { '\n' } else { ' ' });
    while i < chars.len() {
        let c = chars[i];
        // --[==[ ... ]==] and -- to the end of the line
        if c == '-' && chars.get(i + 1) == Some(&'-') {
            let mut j = i + 2;
            let mut level = None;
            if chars.get(j) == Some(&'[') {
                let mut k = j + 1;
                let mut eq = 0;
                while chars.get(k) == Some(&'=') {
                    eq += 1;
                    k += 1;
                }
                if chars.get(k) == Some(&'[') {
                    level = Some(eq);
                    j = k + 1;
                }
            }
            match level {
                Some(eq) => {
                    let close: String = format!("]{}]", "=".repeat(eq));
                    let close: Vec<char> = close.chars().collect();
                    let mut k = j;
                    while k < chars.len() && chars[k..].starts_with(&close) == false {
                        k += 1;
                    }
                    let end = (k + close.len()).min(chars.len());
                    for &ch in &chars[i..end] {
                        blank(&mut out, ch);
                    }
                    i = end;
                }
                None => {
                    while i < chars.len() && chars[i] != '\n' {
                        out.push(' ');
                        i += 1;
                    }
                }
            }
            continue;
        }
        if c == '"' || c == '\'' {
            out.push(c);
            i += 1;
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    blank(&mut out, chars[i]);
                    i += 1;
                }
                if chars[i] == '\n' {
                    break;
                }
                blank(&mut out, chars[i]);
                i += 1;
            }
            if i < chars.len() {
                out.push(chars[i]);
                i += 1;
            }
            continue;
        }
        if c == '[' && (chars.get(i + 1) == Some(&'[') || (chars.get(i + 1) == Some(&'=') )) {
            let mut k = i + 1;
            let mut eq = 0;
            while chars.get(k) == Some(&'=') {
                eq += 1;
                k += 1;
            }
            if chars.get(k) == Some(&'[') {
                let close: Vec<char> = format!("]{}]", "=".repeat(eq)).chars().collect();
                let mut m = k + 1;
                while m < chars.len() && !chars[m..].starts_with(&close) {
                    m += 1;
                }
                let end = (m + close.len()).min(chars.len());
                for &ch in &chars[i..end] {
                    blank(&mut out, ch);
                }
                i = end;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// What one Lua file defines and declares, by name.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Defs {
    /// Global functions and tables: `function Foo(`, `Foo = `, `function Foo.bar` (Foo).
    pub globals: BTreeSet<String>,
    /// Methods and fields: `function X:foo`, `function X.foo`, `.foo =`, `foo = function`.
    pub methods: BTreeSet<String>,
    /// Locals and parameters: `local x`, `function(a, b)`, `for k, v in`.
    pub locals: BTreeSet<String>,
    /// `LuaEventManager.AddEvent("X")`.
    pub events: BTreeSet<String>,
}

impl Defs {
    pub fn absorb(&mut self, other: &Defs) {
        self.globals.extend(other.globals.iter().cloned());
        self.methods.extend(other.methods.iter().cloned());
        self.locals.extend(other.locals.iter().cloned());
        self.events.extend(other.events.iter().cloned());
    }
}

/// The identifier at `i` (`chars[i]` starts it) and where it ends.
fn ident_at(chars: &[char], i: usize) -> (String, usize) {
    let mut j = i;
    while j < chars.len() && is_ident(chars[j]) {
        j += 1;
    }
    (chars[i..j].iter().collect(), j)
}

fn skip_ws(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// The previous word before `i`, skipping spaces (for "function foo(" and "local foo").
fn word_before(chars: &[char], i: usize) -> Option<String> {
    let mut j = i;
    while j > 0 && chars[j - 1].is_whitespace() {
        j -= 1;
    }
    if j == 0 || !is_ident(chars[j - 1]) {
        return None;
    }
    let mut k = j;
    while k > 0 && is_ident(chars[k - 1]) {
        k -= 1;
    }
    Some(chars[k..j].iter().collect())
}

/// Definitions in a (raw) Lua text; the events are read from the raw text, strings included.
pub fn definitions(raw: &str) -> Defs {
    let mut d = Defs::default();
    // AddEvent("X") and triggerEvent("X") before the strings go: both make X an event.
    for word in ["AddEvent", "triggerEvent"] {
        let mut rest = raw;
        while let Some(p) = rest.find(word) {
            rest = &rest[p + word.len()..];
            let after = rest.trim_start().strip_prefix('(').unwrap_or("").trim_start();
            if let Some(q) = after.strip_prefix('"').or_else(|| after.strip_prefix('\'')) {
                let name: String = q.chars().take_while(|c| is_ident(*c)).collect();
                if !name.is_empty() {
                    d.events.insert(name);
                }
            }
        }
    }
    let text = blank_comments_and_strings(raw);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !is_ident_start(c) || (i > 0 && is_ident(chars[i - 1])) {
            i += 1;
            continue;
        }
        let (word, end) = ident_at(&chars, i);
        match word.as_str() {
            "local" => {
                // local function name / local a, b = ...
                let mut j = skip_ws(&chars, end);
                if chars[j..].starts_with(&['f', 'u', 'n', 'c', 't', 'i', 'o', 'n']) {
                    j = skip_ws(&chars, j + 8);
                    if j < chars.len() && is_ident_start(chars[j]) {
                        let (n, _) = ident_at(&chars, j);
                        d.locals.insert(n);
                    }
                } else {
                    loop {
                        if j >= chars.len() || !is_ident_start(chars[j]) {
                            break;
                        }
                        let (n, e) = ident_at(&chars, j);
                        d.locals.insert(n);
                        j = skip_ws(&chars, e);
                        if chars.get(j) == Some(&',') {
                            j = skip_ws(&chars, j + 1);
                        } else {
                            break;
                        }
                    }
                }
                i = end;
            }
            "function" => {
                // function Name(, function Name.sub(, function Name:sub(, function(params)
                let mut j = skip_ws(&chars, end);
                let mut first: Option<String> = None;
                let mut last: Option<String> = None;
                let mut dotted = false;
                while j < chars.len() && is_ident_start(chars[j]) {
                    let (n, e) = ident_at(&chars, j);
                    if first.is_none() {
                        first = Some(n.clone());
                    }
                    last = Some(n);
                    j = e;
                    if chars.get(j) == Some(&'.') || chars.get(j) == Some(&':') {
                        dotted = true;
                        j += 1;
                    } else {
                        break;
                    }
                }
                let is_local = word_before(&chars, i).as_deref() == Some("local");
                if let (Some(f), Some(l)) = (first, last) {
                    if dotted {
                        d.methods.insert(l);
                        if !is_local {
                            d.globals.insert(f);
                        }
                    } else if !is_local {
                        d.globals.insert(f);
                    }
                }
                // params
                j = skip_ws(&chars, j);
                if chars.get(j) == Some(&'(') {
                    let mut k = j + 1;
                    while k < chars.len() && chars[k] != ')' {
                        if is_ident_start(chars[k]) && (k == 0 || !is_ident(chars[k - 1])) {
                            let (n, e) = ident_at(&chars, k);
                            d.locals.insert(n);
                            k = e;
                        } else {
                            k += 1;
                        }
                    }
                }
                i = end;
            }
            "for" => {
                let mut j = skip_ws(&chars, end);
                while j < chars.len() && is_ident_start(chars[j]) {
                    let (n, e) = ident_at(&chars, j);
                    if n == "in" {
                        break;
                    }
                    d.locals.insert(n);
                    j = skip_ws(&chars, e);
                    if chars.get(j) == Some(&',') {
                        j = skip_ws(&chars, j + 1);
                    } else {
                        break;
                    }
                }
                i = end;
            }
            _ => {
                // Name = ..., Name.sub = ..., Name:sub = (rare), .sub = function
                let j = skip_ws(&chars, end);
                let prev = if i > 0 { chars[i - 1] } else { ' ' };
                if chars.get(j) == Some(&'=') && chars.get(j + 1) != Some(&'=') {
                    if prev == '.' || prev == ':' {
                        d.methods.insert(word.clone());
                    } else if !KEYWORDS.contains(&word.as_str()) {
                        // `foo = function` inside a table, or a global assignment at the start of
                        // a statement; both make `foo` callable.
                        let line_start = chars[..i].iter().rposition(|&c| c == '\n').map(|p| p + 1).unwrap_or(0);
                        let before: String = chars[line_start..i].iter().collect();
                        let before = before.trim();
                        let in_table = before.ends_with('{') || before.ends_with(',') || before.is_empty();
                        let k = skip_ws(&chars, j + 1);
                        let is_function = chars[k..].starts_with(&['f', 'u', 'n', 'c', 't', 'i', 'o', 'n']);
                        if is_function || before.ends_with('{') || before.ends_with(',') {
                            d.methods.insert(word.clone());
                        }
                        if in_table && !before.ends_with('{') && !before.ends_with(',') && word_before(&chars, i).as_deref() != Some("local") {
                            d.globals.insert(word.clone());
                        }
                    }
                }
                i = end;
            }
        }
    }
    d
}

/// A call the check looks at.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Call {
    /// `global`, `method` or `event`.
    pub kind: String,
    pub name: String,
    pub line: u32,
    /// The file also names it without calling it (`if foo then`, `type(foo) == "function"`,
    /// `foo and foo()`, `if obj.foo then obj:foo()`): the author knows it may be missing.
    #[serde(default)]
    pub guarded: bool,
}

/// The calls in a (raw) Lua text: global `foo(`, method `:foo(`, `Events.X.Add`.
pub fn calls(raw: &str) -> Vec<Call> {
    let text = blank_comments_and_strings(raw);
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut bare: BTreeSet<(&str, String)> = BTreeSet::new();
    let mut line = 1u32;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if !is_ident_start(c) || (i > 0 && is_ident(chars[i - 1])) {
            i += 1;
            continue;
        }
        let (word, end) = ident_at(&chars, i);
        let prev = if i > 0 { chars[i - 1] } else { ' ' };
        if word == "Events" && chars.get(end) == Some(&'.') && prev != '.' {
            let (ev, e2) = ident_at(&chars, end + 1);
            if !ev.is_empty() && chars.get(e2) == Some(&'.') {
                let (op, _) = ident_at(&chars, e2 + 1);
                if op == "Add" || op == "Remove" {
                    out.push(Call { kind: "event".into(), name: ev, line, guarded: false });
                }
            }
            i = end;
            continue;
        }
        let j = skip_ws(&chars, end);
        let called = chars.get(j) == Some(&'(') || chars.get(j) == Some(&'{') || chars.get(j) == Some(&'"');
        if called && !KEYWORDS.contains(&word.as_str()) {
            if prev == ':' {
                out.push(Call { kind: "method".into(), name: word, line, guarded: false });
            } else if prev != '.' && word_before(&chars, i).as_deref() != Some("function") {
                out.push(Call { kind: "global".into(), name: word, line, guarded: false });
            }
        } else if !called && !KEYWORDS.contains(&word.as_str()) && word_before(&chars, i).as_deref() != Some("function") && word_before(&chars, i).as_deref() != Some("local") {
            // A bare mention: `if foo then`, `type(foo)`, `foo and foo()`, `obj.foo` before `obj:foo()`.
            let next = chars.get(j).copied().unwrap_or(' ');
            if next != '=' && next != '.' && next != ':' {
                bare.insert((if prev == '.' { "method" } else { "global" }, word));
            }
        }
        i = end;
    }
    for c in &mut out {
        if c.kind != "event" && bare.contains(&(c.kind.as_str(), c.name.clone())) {
            c.guarded = true;
        }
    }
    out
}

fn lua_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false).into_iter().flatten() {
        let p = entry.path();
        if p.is_file() && p.extension().map(|e| e.eq_ignore_ascii_case("lua")).unwrap_or(false) {
            out.push(p.to_path_buf());
        }
    }
    out.sort();
    out
}

fn api_cache_dir(user_zomboid_dir: &Path) -> PathBuf {
    user_zomboid_dir.join("PZModStudio_Cache").join("api")
}

/// The installed build's API, read from its jar and Lua once and kept.
pub fn game_api(pz_install_dir: &Path, user_zomboid_dir: &Path) -> Result<GameApi, String> {
    let jar = pz_install_dir.join("projectzomboid.jar");
    let key = jar_key(&jar).ok_or_else(|| format!("{} is not there: no game to read the API from.", jar.display()))?;
    let cache = api_cache_dir(user_zomboid_dir).join(format!("{}.json", key));
    if let Some(api) = std::fs::read_to_string(&cache).ok().and_then(|t| serde_json::from_str::<GameApi>(&t).ok()) {
        if api.jar_key == key {
            return Ok(api);
        }
    }
    let mut api = GameApi { jar_key: key, ..Default::default() };
    for b in LUA_BUILTINS {
        api.globals.insert(b.to_string());
    }
    for m in JAVA_BASICS {
        api.methods.insert(m.to_string());
    }
    // The jar.
    let bytes = std::fs::read(&jar).map_err(|e| format!("Could not read {}: {}", jar.display(), e))?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("{} is not a jar: {}", jar.display(), e))?;
    let mut exposed: Vec<String> = Vec::new();
    for idx in 0..zip.len() {
        let Ok(mut file) = zip.by_index(idx) else { continue };
        if !file.name().ends_with(".class") {
            continue;
        }
        let mut buf = Vec::with_capacity(file.size() as usize);
        if file.read_to_end(&mut buf).is_err() {
            continue;
        }
        let Some(class) = parse_class(&buf) else { continue };
        api.class_files += 1;
        let simple = class.name.rsplit('/').next().unwrap_or(&class.name).to_string();
        api.classes.insert(simple.clone());
        if class.name == "zombie/Lua/LuaManager$GlobalObject" {
            api.globals.extend(class.methods.iter().cloned());
        }
        if class.name == "zombie/Lua/LuaManager$Exposer" {
            exposed.extend(class.strings.iter().filter(|s| s.starts_with("zombie/") || s.starts_with("se/") || s.starts_with("java/")).cloned());
        }
        api.methods.extend(class.methods.into_iter());
        // The event names: the engine triggers them by string, and LuaEventManager holds the
        // list it registers (`OnTick`, `EveryTenMinutes`, `LoadGridsquare`...); elsewhere only
        // the `On...` shape is taken, so a method name does not pass for an event.
        let all_events = class.name == "zombie/Lua/LuaEventManager";
        for s in class.strings {
            let shape = s.len() > 3 && s.chars().all(is_ident) && s.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) && s.chars().any(|c| c.is_ascii_lowercase());
            if shape && (all_events || (s.starts_with("On") && s.as_bytes()[2].is_ascii_uppercase())) {
                api.events.insert(s);
            }
        }
    }
    // Exposed classes are reachable as globals by their simple name (`IsoPlayer`, `ItemPicker`).
    for c in exposed {
        let simple = c.rsplit('/').next().unwrap_or(&c);
        let simple = simple.rsplit('$').next().unwrap_or(simple);
        api.globals.insert(simple.to_string());
    }
    // The game's Lua, and stdlib.lua.
    let mut files = lua_files(&pz_install_dir.join("media").join("lua"));
    let stdlib = pz_install_dir.join("stdlib.lua");
    if stdlib.is_file() {
        files.push(stdlib);
    }
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        api.lua_files += 1;
        let d = definitions(&text);
        api.globals.extend(d.globals);
        api.methods.extend(d.methods);
        api.events.extend(d.events);
        // Events the game's own Lua listens to exist.
        for c in calls(&text) {
            if c.kind == "event" {
                api.events.insert(c.name);
            }
        }
    }
    if let Some(parent) = cache.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(&api) {
        let _ = std::fs::write(&cache, text);
    }
    Ok(api)
}

/// One mod's Lua, read: what it defines and what it calls, per file. Cached by the folder's
/// fingerprint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModLua {
    pub fingerprint: String,
    pub defs: Defs,
    /// Relative path -> calls.
    pub calls: BTreeMap<String, Vec<Call>>,
}

/// The mod's Lua files the installed build reads: its layers for that build, else the root.
fn mod_lua_files(root: &Path, game_version: Option<u32>) -> Vec<PathBuf> {
    let layers = crate::load_order::mod_info::media_layers_for_build(root, game_version);
    let mut out = Vec::new();
    for layer in &layers {
        out.extend(lua_files(&layer.join("media").join("lua")));
    }
    if out.is_empty() {
        out = lua_files(&root.join("media").join("lua"));
    }
    out
}

pub fn read_mod(user_zomboid_dir: &Path, mod_id: &str, root: &Path, game_version: Option<u32>) -> ModLua {
    let (fp, _) = crate::security::fingerprint(root);
    let fp = format!("{}-{}", fp, game_version.unwrap_or(0));
    let cache = api_cache_dir(user_zomboid_dir).join("mods").join(format!("{}.json", crate::security::safe_name(mod_id)));
    if let Some(m) = std::fs::read_to_string(&cache).ok().and_then(|t| serde_json::from_str::<ModLua>(&t).ok()) {
        if m.fingerprint == fp {
            return m;
        }
    }
    let mut m = ModLua { fingerprint: fp, ..Default::default() };
    for f in mod_lua_files(root, game_version) {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let rel = f.strip_prefix(root).unwrap_or(&f).to_string_lossy().replace('\\', "/");
        m.defs.absorb(&definitions(&text));
        let c = calls(&text);
        if !c.is_empty() {
            m.calls.insert(rel, c);
        }
    }
    if let Some(parent) = cache.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(&m) {
        let _ = std::fs::write(&cache, text);
    }
    m
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiFinding {
    /// `global`, `method` or `event`.
    pub kind: String,
    pub name: String,
    pub file: String,
    pub line: u32,
    /// A known name one or two letters away, when there is one.
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModApiReport {
    pub mod_id: String,
    pub files_checked: usize,
    pub calls_checked: usize,
    pub findings: Vec<ApiFinding>,
    /// The mod ships Java (a jar or class files): names its Java gives Lua are not seen here.
    #[serde(default)]
    pub has_java: bool,
    /// The jar the names were read from (size and date), for the record.
    #[serde(default)]
    pub api_key: String,
}

fn ships_java(root: &Path) -> bool {
    walkdir::WalkDir::new(root)
        .into_iter()
        .flatten()
        .any(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("jar") || x.eq_ignore_ascii_case("class")).unwrap_or(false))
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = if a[i - 1].eq_ignore_ascii_case(&b[j - 1]) { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn nearest(name: &str, known: &BTreeSet<String>) -> Option<String> {
    if name.len() < 5 {
        return None;
    }
    let lower = name.to_lowercase();
    if let Some(k) = known.iter().find(|k| k.to_lowercase() == lower && *k != name) {
        return Some(k.clone());
    }
    let mut best: Option<(usize, &String)> = None;
    for k in known {
        if k.len().abs_diff(name.len()) > 2 || !k.chars().next().map(|c| c.eq_ignore_ascii_case(&name.chars().next().unwrap_or(' '))).unwrap_or(false) {
            continue;
        }
        let d = edit_distance(name, k);
        if d <= 2 && best.map(|(bd, _)| d < bd).unwrap_or(true) {
            best = Some((d, k));
        }
    }
    best.map(|(_, k)| k.clone())
}

/// The calls of `this` that nothing defines: not the game, not the mod, not `others` (the other
/// active mods' definitions), not a local of the mod.
pub fn check(api: &GameApi, this: &ModLua, others: &Defs) -> ModApiReport {
    let mut report = ModApiReport { mod_id: String::new(), files_checked: this.calls.len(), calls_checked: 0, findings: Vec::new(), has_java: false, api_key: api.jar_key.clone() };
    let known_global = |n: &str| api.globals.contains(n) || this.defs.globals.contains(n) || this.defs.locals.contains(n) || others.globals.contains(n) || this.defs.methods.contains(n);
    let known_method = |n: &str| api.methods.contains(n) || this.defs.methods.contains(n) || others.methods.contains(n) || this.defs.locals.contains(n) || this.defs.globals.contains(n);
    let known_event = |n: &str| api.events.contains(n) || this.defs.events.contains(n) || others.events.contains(n);
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for (file, calls) in &this.calls {
        for c in calls {
            report.calls_checked += 1;
            let known = match c.kind.as_str() {
                "global" => known_global(&c.name),
                "method" => known_method(&c.name),
                _ => known_event(&c.name),
            };
            if known || c.guarded || !seen.insert((file.clone(), format!("{}:{}", c.kind, c.name))) {
                continue;
            }
            let pool = match c.kind.as_str() {
                "global" => &api.globals,
                "method" => &api.methods,
                _ => &api.events,
            };
            report.findings.push(ApiFinding { kind: c.kind.clone(), name: c.name.clone(), file: file.clone(), line: c.line, suggestion: nearest(&c.name, pool) });
        }
    }
    report.findings.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.cmp(&b.name)).then(a.file.cmp(&b.file)).then(a.line.cmp(&b.line)));
    report
}

/// One mod against the installed build, with the other active mods' definitions counted as
/// known (a mod may call what another gives it).
pub fn check_mod(paths: &crate::vfs::StudioPaths, catalog: &crate::catalog::ModCatalog, mod_id: &str, root: &Path, active: &[String]) -> Result<ModApiReport, String> {
    let z = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next().ok_or("No Zomboid folder.")?;
    let api = game_api(Path::new(&paths.pz_install_dir), &z)?;
    let game = catalog.game_version_int();
    let this = read_mod(&z, mod_id, root, game);
    let mut others = Defs::default();
    for entry in active {
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if m.id == mod_id {
            continue;
        }
        others.absorb(&read_mod(&z, &m.id, Path::new(&m.root_dir), game).defs);
    }
    let mut report = check(&api, &this, &others);
    report.mod_id = mod_id.to_string();
    report.has_java = ships_java(root);
    Ok(report)
}

/// Several mods at once (the active list by default), each against the build with the others'
/// definitions known.
pub fn check_mods(paths: &crate::vfs::StudioPaths, catalog: &crate::catalog::ModCatalog, ids: &[String], active: &[String]) -> Result<Vec<ModApiReport>, String> {
    let z = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&paths.user_zomboid_dir).into_iter().next().ok_or("No Zomboid folder.")?;
    let api = game_api(Path::new(&paths.pz_install_dir), &z)?;
    let game = catalog.game_version_int();
    let pool: Vec<(String, String, ModLua)> = active
        .iter()
        .chain(ids.iter())
        .filter_map(|e| catalog.resolve(e.trim()))
        .map(|m| (m.id.clone(), m.root_dir.clone(), read_mod(&z, &m.id, Path::new(&m.root_dir), game)))
        .collect();
    let mut out = Vec::new();
    for id in ids {
        let Some(m) = catalog.resolve(id.trim()) else { continue };
        let Some((_, root, this)) = pool.iter().find(|(pid, _, _)| *pid == m.id) else { continue };
        let mut others = Defs::default();
        for (pid, _, o) in &pool {
            if *pid != m.id {
                others.absorb(&o.defs);
            }
        }
        let mut report = check(&api, this, &others);
        report.mod_id = m.id.clone();
        report.has_java = ships_java(Path::new(root));
        out.push(report);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_strings_are_blanked_with_lines_kept() {
        let t = "a = \"x(\" -- call(\nb = [[\nfoo(\n]] c()\n--[[ bar( ]] d()";
        let b = blank_comments_and_strings(t);
        assert_eq!(b.lines().count(), t.lines().count());
        assert!(!b.contains("call(") && !b.contains("foo(") && !b.contains("bar("));
        assert!(b.contains("c()") && b.contains("d()"));
    }

    #[test]
    fn definitions_and_calls_are_read_by_shape() {
        let t = r#"
local util = require "x"
local function helper(a, b) return a end
function MyMod.setup(opts)
    local p = getPlayer()
    p:Say("hi")
    p:getInventory():AddItem("Base.Axe")
    Events.OnGameStart.Add(MyMod.setup)
    Events.OnMyThing.Remove(helper)
    for k, v in pairs(opts) do helper(k, v) end
    obj.handler = function(self) self:run() end
    LuaEventManager.AddEvent("OnMyThing")
end
MyGlobal = 1
"#;
        let d = definitions(t);
        assert!(d.globals.contains("MyMod") && d.globals.contains("MyGlobal"));
        assert!(d.methods.contains("setup") && d.methods.contains("handler"));
        assert!(d.locals.contains("util") && d.locals.contains("helper") && d.locals.contains("opts") && d.locals.contains("k"));
        assert!(d.events.contains("OnMyThing"));
        let c = calls(t);
        let names: Vec<(String, String)> = c.iter().map(|c| (c.kind.clone(), c.name.clone())).collect();
        assert!(names.contains(&("global".into(), "getPlayer".into())));
        assert!(names.contains(&("method".into(), "Say".into())));
        assert!(names.contains(&("method".into(), "AddItem".into())));
        assert!(names.contains(&("event".into(), "OnGameStart".into())));
        assert!(names.contains(&("event".into(), "OnMyThing".into())));
        assert!(names.contains(&("global".into(), "helper".into())));
        assert!(!names.iter().any(|(_, n)| n == "setup"), "a definition is not a call");
        assert!(!names.iter().any(|(_, n)| n == "require" && false));
        assert_eq!(c.iter().find(|c| c.name == "Say").unwrap().line, 6);
    }

    #[test]
    fn only_what_nothing_defines_is_reported_with_a_near_name() {
        let mut api = GameApi::default();
        api.globals.extend(["getPlayer", "getCore"].map(String::from));
        api.methods.extend(["getInventory", "AddItem", "Say"].map(String::from));
        api.events.insert("OnGameStart".into());
        let text = "local p = getPlayer()\np:getInvenotry():AddItem(\"x\")\ngetPlayr():Say(\"a\")\nEvents.OnGameStart.Add(f)\nEvents.OnGone.Add(f)\nmine()\nfunction mine() end\nother()";
        let text = &format!("{}\nif getCacheDir then getCacheDir() end\nif obj.getFoo then obj:getFoo() end", text);
        let this = ModLua { fingerprint: String::new(), defs: definitions(text), calls: [("a.lua".to_string(), calls(text))].into_iter().collect() };
        let mut others = Defs::default();
        others.globals.insert("other".into());
        let r = check(&api, &this, &others);
        let got: Vec<(String, String, Option<String>)> = r.findings.iter().map(|f| (f.kind.clone(), f.name.clone(), f.suggestion.clone())).collect();
        assert_eq!(
            got,
            vec![
                ("event".into(), "OnGone".into(), None),
                ("global".into(), "getPlayr".into(), Some("getPlayer".into())),
                ("method".into(), "getInvenotry".into(), Some("getInventory".into())),
            ]
        );
    }

    #[test]
    fn a_class_file_gives_its_name_and_methods() {
        // The smallest class: `class A { void m() {} }`, assembled by hand.
        let mut b: Vec<u8> = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52];
        b.extend([0, 6]); // 5 constants
        b.extend([7, 0, 2]); // #1 Class -> #2
        b.extend([1, 0, 1, b'A']); // #2 Utf8 "A"
        b.extend([1, 0, 1, b'm']); // #3 Utf8 "m"
        b.extend([1, 0, 3, b'(', b')', b'V']); // #4 Utf8 "()V"
        b.extend([1, 0, 4, b'C', b'o', b'd', b'e']); // #5 Utf8 "Code"
        b.extend([0, 1, 0, 1, 0, 0, 0, 0]); // access, this=#1, super=0, interfaces=0
        b.extend([0, 0]); // fields
        b.extend([0, 1, 0, 1, 0, 3, 0, 4, 0, 0]); // 1 method: access, name=#3, desc=#4, 0 attrs
        b.extend([0, 0]); // attributes
        let c = parse_class(&b).unwrap();
        assert_eq!(c.name, "A");
        assert_eq!(c.methods, vec!["m".to_string()]);
    }

    /// Live: `cargo test --lib -- --ignored live_api_index --nocapture`. Reads the installed game.
    #[test]
    #[ignore]
    fn live_api_index() {
        let paths = crate::vfs::studio_paths();
        let z = PathBuf::from(&paths.user_zomboid_dir);
        let started = std::time::Instant::now();
        let api = game_api(Path::new(&paths.pz_install_dir), &z).unwrap();
        println!(
            "{} classes ({} files), {} methods, {} globals, {} events, {} lua files in {:?}",
            api.classes.len(), api.class_files, api.methods.len(), api.globals.len(), api.events.len(), api.lua_files, started.elapsed()
        );
        for n in ["getPlayer", "getCore", "IsoPlayer", "ISPanel", "triggerEvent", "getSpecificPlayer", "ISTimedActionQueue"] {
            println!("global {} {}", n, api.globals.contains(n));
        }
        for n in ["getInventory", "AddItem", "getModData", "transmitModData", "getCategories"] {
            println!("method {} {}", n, api.methods.contains(n));
        }
        for n in ["OnGameStart", "OnTick", "OnFillWorldObjectContextMenu", "OnPlayerUpdate", "OnZombieUpdate", "OnCreatePlayer"] {
            println!("event {} {}", n, api.events.contains(n));
        }
    }

    /// Live probe of the jar: `cargo test --lib -- --ignored live_api_probe --nocapture`.
    #[test]
    #[ignore]
    fn live_api_probe() {
        let paths = crate::vfs::studio_paths();
        let jar = Path::new(&paths.pz_install_dir).join("projectzomboid.jar");
        let bytes = std::fs::read(&jar).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut failed = 0;
        let mut failed_names: Vec<String> = Vec::new();
        for idx in 0..zip.len() {
            let mut file = zip.by_index(idx).unwrap();
            let name = file.name().to_string();
            if !name.ends_with(".class") {
                continue;
            }
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).unwrap();
            match parse_class(&buf) {
                None => {
                    failed += 1;
                    if failed_names.len() < 8 {
                        failed_names.push(name.clone());
                    }
                }
                Some(c) => {
                    if name == "zombie/Lua/LuaManager$GlobalObject.class" {
                        println!("GlobalObject: {} methods; openUrl {} getCacheDir {} getAverageFPS {}", c.methods.len(), c.methods.iter().any(|m| m == "openUrl"), c.methods.iter().any(|m| m == "getCacheDir"), c.methods.iter().any(|m| m == "getAverageFPS"));
                        println!("  like Url/Cache/FPS: {:?}", c.methods.iter().filter(|m| m.contains("Url") || m.contains("URL") || m.contains("Cache") || m.contains("FPS")).collect::<Vec<_>>());
                    }
                    if name == "zombie/core/properties/PropertyContainer.class" {
                        println!("PropertyContainer: {:?}", c.methods);
                    }
                    for m in ["openurl", "cachedir", "averagefps", "zomboiddir", "getprofession", "dirtyness", "hastrait", "boredomlevel"] {
                        for x in c.methods.iter().filter(|x| x.to_lowercase().contains(m)) {
                            println!("method {} ({}) in {}", x, m, name);
                        }
                    }
                    for s in ["LoadChunk", "ReuseGridsquare", "OnGameEnd", "OnDestroyIsoCode", "OnLoadGridsquare"] {
                        if c.strings.iter().any(|x| x == s) {
                            println!("string {} in {}", s, name);
                        }
                    }
                    if name == "zombie/Lua/LuaManager$Exposer.class" {
                        let classes: Vec<&String> = c.strings.iter().filter(|s| s.contains('/') && !s.contains('(') && !s.starts_with('L') && !s.starts_with('[')).collect();
                        println!("Exposer: {} class-like strings; first 40: {:?}", classes.len(), classes.iter().take(40).collect::<Vec<_>>());
                        println!("Exposer methods: {:?}", c.methods);
                    }
                    if c.strings.iter().any(|s| s == "EveryTenMinutes") {
                        println!("EveryTenMinutes in {} ({} strings): {:?}", name, c.strings.len(), c.strings.iter().filter(|s| s.chars().next().map(|ch| ch.is_ascii_uppercase()).unwrap_or(false) && s.chars().all(is_ident)).take(60).collect::<Vec<_>>());
                    }
                }
            }
        }
        println!("failed to parse: {} {:?}", failed, failed_names);
    }

    /// Live: `cargo test --lib -- --ignored live_api_check --nocapture`: every active mod against the
    /// installed build, the findings counted per mod (the noise to look at).
    #[test]
    #[ignore]
    fn live_api_check() {
        let paths = crate::vfs::studio_paths();
        let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
        let z = PathBuf::from(&paths.user_zomboid_dir);
        let active = crate::load_order::ini_parser::parse_default_txt(&z.join("mods").join("default.txt"));
        let started = std::time::Instant::now();
        let api = game_api(Path::new(&paths.pz_install_dir), &z).unwrap();
        let game = catalog.game_version_int();
        let mods: Vec<(String, ModLua)> = active.iter().filter_map(|e| catalog.resolve(e.trim())).map(|m| (m.id.clone(), read_mod(&z, &m.id, Path::new(&m.root_dir), game))).collect();
        println!("{} mods read in {:?}", mods.len(), started.elapsed());
        let mut total = 0;
        for (id, this) in &mods {
            let mut others = Defs::default();
            for (oid, o) in &mods {
                if oid != id {
                    others.absorb(&o.defs);
                }
            }
            let r = check(&api, this, &others);
            if r.findings.is_empty() {
                continue;
            }
            total += r.findings.len();
            let show: Vec<String> = r.findings.iter().take(6).map(|f| format!("{}:{}{}@{}:{}", f.kind, f.name, f.suggestion.as_ref().map(|s| format!("->{}", s)).unwrap_or_default(), f.file.rsplit('/').next().unwrap_or(""), f.line)).collect();
            println!("{:40} {:3} {}", id, r.findings.len(), show.join("  "));
        }
        println!("total {} findings in {:?}", total, started.elapsed());
    }
}
