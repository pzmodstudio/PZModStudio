//! Every option of a server's settings, edited the way the game's own screen edits them.
//!
//! What the options are comes from the game itself: the Studio Link writes, at the main menu, the
//! game's list of server options (ServerOptions) and the pages of its sandbox screen with the mods'
//! own options (`Lua/PZModStudio_LinkSettings.txt`: name, type, default, range, label, help, the words
//! of each choice). The values are a server's two files in `Zomboid/Server`: `<name>.ini`
//! (`Key=Value` lines) and `<name>_SandboxVars.lua` (`SandboxVars = { Key = value, Group = { ... } }`,
//! two levels). Writing changes only the lines of the options changed, keeps everything else as
//! it was (comments, order, line endings), adds an option missing from the file where it belongs,
//! and backs both files up first. A server the Studio is running with those settings is refused.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const SCHEMA_FILE: &str = "PZModStudio_LinkSettings.txt";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OptionDef {
    /// `server` (the .ini) or `sandbox` (the _SandboxVars.lua).
    pub kind: String,
    /// `PVP`, `ZombieLore.Speed`.
    pub name: String,
    /// `boolean`, `integer`, `double`, `enum`, `string`, `text`.
    #[serde(rename = "type")]
    pub typ: String,
    pub default: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub label: String,
    pub help: String,
    /// An enum's choices, in order (the value is the choice's number, from 1).
    pub values: Vec<String>,
    /// The page it is on.
    pub page: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schema {
    /// What the game wrote it for: build | language | mods.
    pub key: String,
    /// Pages in the game's order, each with its kind.
    pub pages: Vec<(String, String)>,
    pub options: Vec<OptionDef>,
}

/// The options as the game last listed them, or None before the Link ever did.
pub fn schema(zomboid: &Path) -> Option<Schema> {
    let text = std::fs::read_to_string(zomboid.join("Lua").join(SCHEMA_FILE)).ok()?;
    let mut s = Schema::default();
    let mut page = (String::new(), String::new());
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    for line in text.lines() {
        if let Some(k) = line.strip_prefix("key=") {
            s.key = k.to_string();
        } else if let Some(rest) = line.strip_prefix("page=") {
            let (kind, name) = rest.split_once('\t').unwrap_or((rest, ""));
            page = (kind.to_string(), name.to_string());
            s.pages.push(page.clone());
        } else if let Some(rest) = line.strip_prefix("opt=") {
            let f: Vec<&str> = rest.split('\t').collect();
            if f.len() < 8 {
                continue;
            }
            let num = |v: &str| v.trim().parse::<f64>().ok();
            let o = OptionDef {
                kind: f[0].to_string(),
                name: f[1].to_string(),
                typ: f[2].to_string(),
                default: f[3].to_string(),
                min: num(f[4]),
                max: num(f[5]),
                label: f[6].to_string(),
                help: f[7].replace("\\n", "\n"),
                values: Vec::new(),
                page: page.1.clone(),
            };
            index.insert(format!("{}|{}", o.kind, o.name), s.options.len());
            s.options.push(o);
        } else if let Some(rest) = line.strip_prefix("val=") {
            let f: Vec<&str> = rest.splitn(3, '\t').collect();
            if f.len() == 3 {
                // An enum's choices follow its option: the last option of that name.
                if let Some(o) = s.options.iter_mut().rev().find(|o| o.name == f[0]) {
                    o.values.push(f[2].to_string());
                }
            }
        }
    }
    (!s.options.is_empty()).then_some(s)
}

// ------------------------------------------------------------------ the files

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerValues {
    /// The .ini's values by key.
    pub server: BTreeMap<String, String>,
    /// The sandbox file's values by dotted name (`ZombieLore.Speed`), as written there.
    pub sandbox: BTreeMap<String, String>,
    pub ini_path: String,
    pub sandbox_path: String,
}

fn files_of(zomboid: &Path, name: &str) -> Result<(PathBuf, PathBuf), String> {
    if name.is_empty() || name.contains(['/', '\\', ':']) || name.contains("..") {
        return Err(format!("\"{}\" is not a server settings name", name));
    }
    let dir = zomboid.join("Server");
    Ok((dir.join(format!("{}.ini", name)), dir.join(format!("{}_SandboxVars.lua", name))))
}

/// A sandbox file's values by dotted name, with the line each is on.
fn sandbox_values(text: &str) -> BTreeMap<String, (String, usize)> {
    let mut out = BTreeMap::new();
    let mut path: Vec<String> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("--") {
            continue;
        }
        if line.starts_with('}') {
            path.pop();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        let value = value.trim().trim_end_matches(',').trim();
        if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || key.is_empty() {
            continue;
        }
        if value.starts_with('{') {
            path.push(key.to_string());
            if value.ends_with('}') {
                path.pop(); // `X = {},` on one line
            }
            continue;
        }
        // Under the root table: `Key` or `Group.Key`.
        if path.len() >= 2 {
            out.insert(format!("{}.{}", path[1..].join("."), key), (value.to_string(), n));
        } else if path.len() == 1 {
            out.insert(key.to_string(), (value.to_string(), n));
        }
    }
    out
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        v[1..v.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
        v.to_string()
    }
}

/// One server's values, read from its two files (an absent file reads as no values).
pub fn read_values(zomboid: &Path, name: &str) -> Result<ServerValues, String> {
    let (ini, sandbox) = files_of(zomboid, name)?;
    let mut v = ServerValues { ini_path: ini.display().to_string(), sandbox_path: sandbox.display().to_string(), ..Default::default() };
    if let Ok(text) = std::fs::read_to_string(&ini) {
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with('#') || t.is_empty() {
                continue;
            }
            if let Some((k, val)) = t.split_once('=') {
                v.server.insert(k.trim().to_string(), val.to_string());
            }
        }
    }
    if let Ok(text) = std::fs::read_to_string(&sandbox) {
        v.sandbox = sandbox_values(&text).into_iter().map(|(k, (val, _))| (k, unquote(&val))).collect();
    }
    Ok(v)
}

/// A value as the file wants it, checked against the option: a number within its range, a choice
/// that exists, true/false.
pub fn format_value(def: Option<&OptionDef>, value: &str, for_lua: bool) -> Result<String, String> {
    let value = value.trim();
    let Some(d) = def else {
        return Ok(if for_lua && value.parse::<f64>().is_err() && value != "true" && value != "false" { quote(value) } else { value.to_string() });
    };
    let range = |n: f64| -> Result<(), String> {
        if d.min.is_some_and(|m| n < m) || d.max.is_some_and(|m| n > m) {
            return Err(format!("{} must be between {} and {}", d.name, d.min.map(|m| m.to_string()).unwrap_or_default(), d.max.map(|m| m.to_string()).unwrap_or_default()));
        }
        Ok(())
    };
    match d.typ.as_str() {
        "boolean" => match value.to_ascii_lowercase().as_str() {
            "true" => Ok("true".into()),
            "false" => Ok("false".into()),
            _ => Err(format!("{} is true or false", d.name)),
        },
        "integer" => {
            let n: i64 = value.parse().map_err(|_| format!("{} is a whole number", d.name))?;
            range(n as f64)?;
            Ok(n.to_string())
        }
        "double" => {
            let n: f64 = value.parse().map_err(|_| format!("{} is a number", d.name))?;
            range(n)?;
            let s = n.to_string();
            Ok(if s.contains('.') || s.contains('e') { s } else { format!("{}.0", s) })
        }
        "enum" => {
            let n: usize = value.parse().map_err(|_| format!("{} is the number of a choice", d.name))?;
            if n == 0 || (!d.values.is_empty() && n > d.values.len()) {
                return Err(format!("{} has choices 1 to {}", d.name, d.values.len()));
            }
            Ok(n.to_string())
        }
        _ => Ok(if for_lua { quote(value) } else { value.replace(['\r', '\n'], " ") }),
    }
}

fn quote(v: &str) -> String {
    format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\"").replace(['\r', '\n'], " "))
}

/// The .ini with these keys set: each line changed in place, a missing key added at the end.
pub fn apply_ini(text: &str, changes: &BTreeMap<String, String>) -> String {
    let crlf = text.contains("\r\n");
    let nl = if crlf { "\r\n" } else { "\n" };
    let mut done: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut out: Vec<String> = text
        .lines()
        .map(|line| {
            let t = line.trim_start();
            if !t.starts_with('#') {
                if let Some((k, _)) = t.split_once('=') {
                    if let Some((key, v)) = changes.get_key_value(k.trim()) {
                        done.insert(key.as_str());
                        return format!("{}={}", key, v);
                    }
                }
            }
            line.to_string()
        })
        .collect();
    for (k, v) in changes {
        if !done.contains(k.as_str()) {
            out.push(format!("{}={}", k, v));
        }
    }
    let mut s = out.join(nl);
    if text.ends_with('\n') || text.is_empty() {
        s.push_str(nl);
    }
    s
}

/// The sandbox file with these values set (already formatted for Lua): each value's line changed in
/// place; one missing is added at the end of its table, a group missing at the end of the root.
pub fn apply_sandbox(text: &str, changes: &BTreeMap<String, String>) -> String {
    let crlf = text.contains("\r\n");
    let nl = if crlf { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let found = sandbox_values(text);
    let mut missing: Vec<(&String, &String)> = Vec::new();
    for (name, value) in changes {
        match found.get(name) {
            Some((_, n)) => {
                let line = &lines[*n];
                let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                let key = name.rsplit('.').next().unwrap_or(name);
                lines[*n] = format!("{}{} = {},", indent, key, value);
            }
            None => missing.push((name, value)),
        }
    }
    // Missing ones: by their group (the root for plain names).
    let mut by_group: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (name, value) in missing {
        let (group, key) = match name.rsplit_once('.') {
            Some((g, k)) => (g.to_string(), k.to_string()),
            None => (String::new(), name.clone()),
        };
        by_group.entry(group).or_default().push((key, value.clone()));
    }
    for (group, items) in by_group {
        // Where the table closes: the line `}` that ends it.
        let close = table_close(&lines, &group);
        let indent = if group.is_empty() { "    " } else { "        " };
        let mut add: Vec<String> = items.iter().map(|(k, v)| format!("{}{} = {},", indent, k, v)).collect();
        match close {
            Some(at) => {
                for (i, l) in add.drain(..).enumerate() {
                    lines.insert(at + i, l);
                }
            }
            None => {
                // The group itself is missing: a new table at the end of the root.
                let Some(root_close) = table_close(&lines, "") else { continue };
                let mut block = vec![format!("    {} = {{", group)];
                block.extend(add);
                block.push("    },".to_string());
                for (i, l) in block.into_iter().enumerate() {
                    lines.insert(root_close + i, l);
                }
            }
        }
    }
    let mut s = lines.join(nl);
    if text.ends_with('\n') {
        s.push_str(nl);
    }
    s
}

/// The line of the `}` closing a table: "" is the root (SandboxVars), else a group under it.
fn table_close(lines: &[String], group: &str) -> Option<usize> {
    let mut path: Vec<String> = Vec::new();
    for (n, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if line.starts_with("--") || line.is_empty() {
            continue;
        }
        if line.starts_with('}') {
            if path.is_empty() {
                continue;
            }
            let closing = path[1..].join(".");
            if path.len() >= 1 && closing == group && (!group.is_empty() || path.len() == 1) {
                return Some(n);
            }
            path.pop();
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim();
            if value.starts_with('{') && !value.trim_end_matches(',').ends_with('}') {
                path.push(key.trim().to_string());
            }
        }
    }
    None
}

/// Sets these options of one server, both files backed up first. Values are checked against the
/// game's list; an option it does not list is written as given. Returns the options written.
pub fn write_values(zomboid: &Path, name: &str, server: &BTreeMap<String, String>, sandbox: &BTreeMap<String, String>) -> Result<Vec<String>, String> {
    let status = crate::server_manager::get_dedicated_server_status(zomboid.display().to_string()).ok();
    if status.as_ref().is_some_and(|s| s.is_running && s.server_name.as_deref() == Some(name)) {
        return Err(format!("The server \"{}\" is running: stop it first, it would write its own settings over these.", name));
    }
    let (ini, sandbox_file) = files_of(zomboid, name)?;
    if !ini.is_file() {
        return Err(format!("{} does not exist", ini.display()));
    }
    let schema = schema(zomboid);
    let def = |kind: &str, n: &str| schema.as_ref().and_then(|s| s.options.iter().find(|o| o.kind == kind && o.name == n)).cloned();
    let mut ini_changes = BTreeMap::new();
    for (k, v) in server {
        ini_changes.insert(k.clone(), format_value(def("server", k).as_ref(), v, false)?);
    }
    let mut lua_changes = BTreeMap::new();
    for (k, v) in sandbox {
        lua_changes.insert(k.clone(), format_value(def("sandbox", k).as_ref(), v, true)?);
    }
    let backups = zomboid.join("PZModStudio_Backups").join("server_settings");
    std::fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S%.3f");
    let mut written = Vec::new();
    if !ini_changes.is_empty() {
        let text = std::fs::read_to_string(&ini).map_err(|e| e.to_string())?;
        std::fs::copy(&ini, backups.join(format!("{}.{}.ini", name, stamp))).map_err(|e| e.to_string())?;
        crate::load_order::ini_parser::atomic_write(&ini, apply_ini(&text, &ini_changes)).map_err(|e| e.to_string())?;
        written.extend(ini_changes.keys().cloned());
    }
    if !lua_changes.is_empty() {
        let text = std::fs::read_to_string(&sandbox_file).map_err(|_| format!("{} does not exist: open the server's settings in the game once to make it", sandbox_file.display()))?;
        std::fs::copy(&sandbox_file, backups.join(format!("{}_SandboxVars.{}.lua", name, stamp))).map_err(|e| e.to_string())?;
        crate::load_order::ini_parser::atomic_write(&sandbox_file, apply_sandbox(&text, &lua_changes)).map_err(|e| e.to_string())?;
        written.extend(lua_changes.keys().cloned());
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LUA: &str = "SandboxVars = {\n    VERSION = 6,\n    -- comment = 3,\n    Zombies = 4,\n    Name = \"x\",\n    ZombieLore = {\n        -- Default = Fast\n        Speed = 2,\n    },\n    Empty = {},\n}\n";

    #[test]
    fn sandbox_values_are_read_by_dotted_name() {
        let v = sandbox_values(LUA);
        assert_eq!(v.get("Zombies").map(|x| x.0.as_str()), Some("4"));
        assert_eq!(v.get("ZombieLore.Speed").map(|x| x.0.as_str()), Some("2"));
        assert_eq!(unquote(&v.get("Name").unwrap().0), "x");
        assert!(!v.contains_key("comment"));
    }

    #[test]
    fn changes_go_in_place_and_missing_ones_where_they_belong() {
        let changes: BTreeMap<String, String> = [
            ("Zombies".to_string(), "2".to_string()),
            ("ZombieLore.Speed".to_string(), "1".to_string()),
            ("ZombieLore.Strength".to_string(), "3".to_string()),
            ("NewTop".to_string(), "true".to_string()),
            ("NewGroup.Opt".to_string(), "5".to_string()),
        ]
        .into();
        let out = apply_sandbox(LUA, &changes);
        let v = sandbox_values(&out);
        for (k, want) in [("Zombies", "2"), ("ZombieLore.Speed", "1"), ("ZombieLore.Strength", "3"), ("NewTop", "true"), ("NewGroup.Opt", "5"), ("VERSION", "6")] {
            assert_eq!(v.get(k).map(|x| x.0.as_str()), Some(want), "{} in\n{}", k, out);
        }
        assert!(out.contains("        -- Default = Fast\n        Speed = 1,"), "comments kept:\n{}", out);
        assert!(out.trim_end().ends_with('}'));
    }

    #[test]
    fn the_ini_changes_lines_in_place_and_keeps_its_endings() {
        let ini = "# PVP\r\nPVP=true\r\nPublicName=My server\r\n";
        let out = apply_ini(ini, &[("PVP".to_string(), "false".to_string()), ("MaxPlayers".to_string(), "8".to_string())].into());
        assert_eq!(out, "# PVP\r\nPVP=false\r\nPublicName=My server\r\nMaxPlayers=8\r\n");
    }

    #[test]
    fn values_are_checked_against_the_option() {
        let int = OptionDef { name: "MaxPlayers".into(), typ: "integer".into(), min: Some(1.0), max: Some(100.0), ..Default::default() };
        assert_eq!(format_value(Some(&int), "8", false).unwrap(), "8");
        assert!(format_value(Some(&int), "500", false).is_err());
        let dbl = OptionDef { name: "X".into(), typ: "double".into(), ..Default::default() };
        assert_eq!(format_value(Some(&dbl), "2", true).unwrap(), "2.0");
        let en = OptionDef { name: "Speed".into(), typ: "enum".into(), values: vec!["a".into(), "b".into()], ..Default::default() };
        assert!(format_value(Some(&en), "3", true).is_err());
        let st = OptionDef { name: "S".into(), typ: "string".into(), ..Default::default() };
        assert_eq!(format_value(Some(&st), "a \"b\"", true).unwrap(), "\"a \\\"b\\\"\"");
    }
}
