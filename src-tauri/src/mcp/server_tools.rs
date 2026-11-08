//! MCP tools to host a dedicated server from a config in `Zomboid/Server/` and to join one with
//! the game: the same launcher as the Servers screen (ZombieBuddy's agent included), with the
//! admin password taken from the Studio's stored value or a file and never sent back.

use super::protocol::McpTool;
use super::protocol::ToolCallResult;
use crate::server_manager::{self, java_mods, mod_sync};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "server_start".to_string(),
            description: "Starts a dedicated server from a config in Zomboid/Server/<name>.ini, the way the Servers screen does: java.exe from the game folder, headless, its log in Zomboid/server-console.txt. When Mods= has ZombieBuddy the JVM gets -agentlib:zbNative=policy=deny-new (only Java jars already approved on this PC load; a server cannot show the approval window). The admin password comes from admin_password_file (its first line) or the value the Studio stored the last time this server was started with one; it is never returned. Returns the PID, whether the agent was added, and what the Mods and sync check says will not load (unapproved jars; Workshop mods linked into Zomboid/mods or copied there and outdated; with nosteam, mods only in the Workshop folder: the Servers screen's Mods & Sync button copies them). One server at a time. Poll server_status until started is true (a big mod list takes minutes).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Server config name (Zomboid/Server/<name>.ini), letters, digits, '_' and '-'" },
                    "memory_gb": { "type": "integer", "description": "Java heap (-Xmx) in GB, 2-32 (default 4)" },
                    "nosteam": { "type": "boolean", "description": "Start without Steam (-nosteam, LAN; players join with -nosteam). Default false" },
                    "admin_password_file": { "type": "string", "description": "A file whose first line is the admin password (it is read, never echoed). Default: the Studio's stored value for this server, else none" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                },
                "required": ["name"]
            }),
        },
        McpTool {
            name: "server_stop".to_string(),
            description: "Stops the dedicated server the Studio started (the PID it keeps in Zomboid/pz_server_state.json): \"quit\" over RCON first (saves the world), killed only if it does not go within 90 s. Never touches any other java.exe. Returns whether a server was stopped.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "server_status".to_string(),
            description: "Whether the Studio's dedicated server runs: PID, config name, memory, start time, nosteam, started (its log says SERVER STARTED since it was launched), the port from its .ini, the address to join it from this PC (the LAN address; a Steam client cannot join 127.0.0.1 or this PC's own public IP), and the last lines of Zomboid/server-console.txt.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "log_lines": { "type": "integer", "description": "Lines of server-console.txt to return (default 20, max 200; 0 for none)" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
        McpTool {
            name: "server_join".to_string(),
            description: "Launches the game straight into a server: ProjectZomboid64.exe +connect <ip:port>, windowed. The game opens on its connect window for that address, where the player types the account username and password (a new username makes a new account) and presses Connect. Default address: the running Studio server on this PC (its .ini's DefaultPort) at this PC's LAN address with Steam, 127.0.0.1 without. nosteam defaults to how that server was started (a NoSteam server needs a -nosteam client, a Steam server a Steam client). A Steam client cannot join 127.0.0.1 or this PC's own public IP: use the LAN address. A -debug client may only join as an admin. Refused while the game already runs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "address": { "type": "string", "description": "ip:port or ip (port 16261). Default: this PC's running Studio server" },
                    "nosteam": { "type": "boolean", "description": "Launch with -nosteam (default: how the running server was started, else false)" },
                    "debug": { "type": "boolean", "description": "Launch with -debug (default false; only admins may join with it)" },
                    "extra_args": { "type": "array", "items": { "type": "string" }, "description": "Optional extra game arguments" },
                    "user_zomboid_dir": { "type": "string", "description": "Optional custom user Zomboid directory" }
                }
            }),
        },
    ]
}

pub fn execute(name: &str, args: &Value, paths: &crate::vfs::StudioPaths) -> Option<ToolCallResult> {
    Some(match name {
        "server_start" => result(start(args, paths)),
        "server_stop" => result(server_manager::stop_dedicated_server(paths.user_zomboid_dir.clone(), None).map(|stopped| json!({ "stopped": stopped }))),
        "server_status" => result(status(args, paths)),
        "server_join" => result(join(args, paths)),
        _ => return None,
    })
}

fn result(r: Result<Value, String>) -> ToolCallResult {
    use super::protocol::ToolContent;
    match r {
        Ok(v) => ToolCallResult {
            content: vec![ToolContent { content_type: "text".to_string(), text: serde_json::to_string_pretty(&v).unwrap_or_default() }],
            is_error: false,
        },
        Err(e) => ToolCallResult { content: vec![ToolContent { content_type: "text".to_string(), text: e }], is_error: true },
    }
}

fn zomboid_dir(paths: &crate::vfs::StudioPaths) -> PathBuf {
    if paths.user_zomboid_dir.is_empty() {
        dirs_next::home_dir().unwrap_or_default().join("Zomboid")
    } else {
        PathBuf::from(&paths.user_zomboid_dir)
    }
}

/// The admin password and where it came from ("file", "stored" or "none").
fn admin_password(server: &str, file: Option<&str>) -> Result<(Option<String>, &'static str), String> {
    if let Some(file) = file.filter(|f| !f.trim().is_empty()) {
        let text = std::fs::read_to_string(file).map_err(|e| format!("Cannot read admin_password_file: {}", e))?;
        let first = text.trim_start_matches('\u{feff}').lines().next().unwrap_or("").trim().to_string();
        if first.is_empty() {
            return Err("admin_password_file is empty.".to_string());
        }
        return Ok((Some(first), "file"));
    }
    match crate::secrets::get(&server_manager::admin_password_secret(server)) {
        Ok(Some(p)) if !p.is_empty() => Ok((Some(p), "stored")),
        _ => Ok((None, "none")),
    }
}

fn start(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    let name = args.get("name").and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
    if name.is_empty() {
        return Err("name is required (a config in Zomboid/Server/<name>.ini).".to_string());
    }
    let zomboid = zomboid_dir(paths);
    let ini = zomboid.join("Server").join(format!("{}.ini", name));
    let content = std::fs::read_to_string(&ini).map_err(|e| format!("Cannot read {}: {}", ini.display(), e))?;
    let memory = args.get("memory_gb").and_then(Value::as_u64).map(|m| m as u32);
    let nosteam = args.get("nosteam").and_then(Value::as_bool).unwrap_or(false);
    let (password, password_source) = admin_password(&name, args.get("admin_password_file").and_then(Value::as_str))?;

    // What will not load, said before the minutes it takes the server to find out.
    let report = server_manager::check_server_mods(paths.clone(), ini.to_string_lossy().to_string(), Some(nosteam))?;
    let will_not_load: Vec<Value> = report
        .issues
        .iter()
        .filter(|i| i.code.starts_with("server_java_") || i.code.starts_with("server_mod_workshop_") || i.code == "server_mod_copy_stale")
        .map(|i| json!({ "code": i.code, "mod": i.mod_id, "message": i.message }))
        .collect();

    crate::claude_agent::keep_std_handles_private();
    let pid = server_manager::launch_dedicated_server(
        paths.pz_install_dir.clone(),
        zomboid.to_string_lossy().to_string(),
        name.clone(),
        memory,
        Some(nosteam),
        password,
    )?;
    Ok(json!({
        "status": "launched",
        "pid": pid,
        "name": name,
        "nosteam": nosteam,
        "zombiebuddy_agent": java_mods::agent_arg_for(&content).filter(|_| Path::new(&paths.pz_install_dir).join("zbNative.dll").is_file()),
        "admin_password": password_source,
        "mods": report.mods,
        "will_not_load": will_not_load,
        "log": zomboid.join("server-console.txt").to_string_lossy(),
        "next": "Poll server_status until started is true, then server_join.",
    }))
}

/// This PC's address on its network: the interface a packet to the internet would leave by
/// (nothing is sent).
pub fn lan_address() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}

fn ini_port(zomboid: &Path, server: &str) -> u16 {
    std::fs::read_to_string(zomboid.join("Server").join(format!("{}.ini", server)))
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.trim().strip_prefix("DefaultPort=").and_then(|v| v.trim().parse::<u16>().ok())))
        .unwrap_or(16261)
}

/// The last `n` lines of the server's console log, and whether it said SERVER STARTED after `since`.
fn console(zomboid: &Path, n: usize, since: Option<u64>) -> (Vec<String>, bool) {
    let path = zomboid.join("server-console.txt");
    let fresh = since.is_none_or(|s| {
        path.metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .is_some_and(|t| t.as_secs() + 5 >= s)
    });
    let Ok(bytes) = std::fs::read(&path) else { return (Vec::new(), false) };
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();
    let started = fresh && lines.iter().any(|l| l.contains("*** SERVER STARTED"));
    let tail = lines[lines.len().saturating_sub(n)..].iter().map(|l| l.to_string()).collect();
    (tail, started)
}

fn status(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    let zomboid = zomboid_dir(paths);
    let st = server_manager::get_dedicated_server_status(zomboid.to_string_lossy().to_string())?;
    let n = args.get("log_lines").and_then(Value::as_u64).unwrap_or(20).min(200) as usize;
    let (tail, started) = if st.is_running { console(&zomboid, n, st.start_timestamp) } else { (console(&zomboid, n, None).0, false) };
    let port = st.server_name.as_deref().map(|s| ini_port(&zomboid, s));
    let join_at = match (st.is_running, port) {
        (true, Some(port)) => Some(if st.nosteam == Some(true) {
            format!("127.0.0.1:{}", port)
        } else {
            format!("{}:{}", lan_address().unwrap_or_else(|| "<this PC's LAN address>".into()), port)
        }),
        _ => None,
    };
    Ok(json!({
        "is_running": st.is_running,
        "pid": st.pid,
        "name": st.server_name,
        "memory_gb": st.memory_gb,
        "start_timestamp": st.start_timestamp,
        "nosteam": st.nosteam,
        "started": started,
        "port": port,
        "join_at": join_at,
        "log_tail": tail,
    }))
}

/// `ip:port` from what the caller wrote (`ip`, `ip:port`), or None when it is not an address.
pub fn parse_address(text: &str) -> Option<(String, u16)> {
    let text = text.trim();
    let (host, port) = match text.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().ok()?),
        None => (text, 16261),
    };
    let ok = !host.is_empty() && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    ok.then(|| (host.to_string(), port))
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn join(args: &Value, paths: &crate::vfs::StudioPaths) -> Result<Value, String> {
    if let Some(pid) = crate::sandbox::find_running_pz_pid() {
        return Err(format!("The game is already running (PID {}): close it first (terminate_game), then join.", pid));
    }
    let zomboid = zomboid_dir(paths);
    let st = server_manager::get_dedicated_server_status(zomboid.to_string_lossy().to_string()).ok();
    let running = st.as_ref().filter(|s| s.is_running);
    let nosteam = args
        .get("nosteam")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| running.and_then(|s| s.nosteam).unwrap_or(false));
    let debug = args.get("debug").and_then(Value::as_bool).unwrap_or(false);

    let (host, port) = match args.get("address").and_then(Value::as_str).filter(|a| !a.trim().is_empty()) {
        Some(a) => parse_address(a).ok_or_else(|| format!("\"{}\" is not an address: give ip:port or ip.", a))?,
        None => {
            let Some(server) = running.and_then(|s| s.server_name.clone()) else {
                return Err("No Studio server runs on this PC: give an address (ip:port).".to_string());
            };
            let port = ini_port(&zomboid, &server);
            let host = if nosteam {
                "127.0.0.1".to_string()
            } else {
                lan_address().ok_or("Could not tell this PC's LAN address: give it as address (ip:port).")?
            };
            (host, port)
        }
    };
    if !nosteam && is_loopback(&host) {
        return Err(format!(
            "A Steam client cannot join {} (Steam refuses this PC's own loopback and public addresses): use the LAN address{}.",
            host,
            lan_address().map(|ip| format!(", {}:{}", ip, port)).unwrap_or_default()
        ));
    }

    let mut extra: Vec<String> = vec!["+connect".to_string(), format!("{}:{}", host, port)];
    if let Some(more) = args.get("extra_args").and_then(Value::as_array) {
        extra.extend(more.iter().filter_map(|v| v.as_str().map(str::to_string)));
    }
    let prev_foreground = crate::claude_agent::current_foreground_window();
    crate::claude_agent::keep_std_handles_private();
    let pid = crate::sandbox::launch_pz_direct(&paths.pz_install_dir, &paths.user_zomboid_dir, debug, true, nosteam, &extra)?;
    crate::claude_agent::start_launch_window_guard(prev_foreground, false, false, true);
    let mut note = Vec::new();
    if debug {
        note.push("A -debug client may only join as an admin.");
    }
    if running.is_some_and(|s| s.nosteam != Some(nosteam)) && running.and_then(|s| s.nosteam).is_some() {
        note.push("This client's Steam mode differs from how the running server was started: it will not get in.");
    }
    Ok(json!({
        "status": "launched",
        "pid": pid,
        "address": format!("{}:{}", host, port),
        "nosteam": nosteam,
        "debug": debug,
        "args": extra,
        "notes": note,
        "mods_line": running.and_then(|s| s.server_name.clone()).map(|s| {
            std::fs::read_to_string(zomboid.join("Server").join(format!("{}.ini", s))).map(|t| mod_sync::ini_list(&t, "Mods")).unwrap_or_default()
        }),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_read_with_the_default_port() {
        assert_eq!(parse_address("192.168.0.192:16261"), Some(("192.168.0.192".into(), 16261)));
        assert_eq!(parse_address(" 10.0.0.5 "), Some(("10.0.0.5".into(), 16261)));
        assert_eq!(parse_address("my-host.lan:16300"), Some(("my-host.lan".into(), 16300)));
        assert_eq!(parse_address("1.2.3.4:99999"), None);
        assert_eq!(parse_address("a b:1"), None);
        assert_eq!(parse_address("\"x\" -debug:1"), None, "nothing but an address reaches the command line");
        assert!(is_loopback("127.0.0.1") && is_loopback("localhost") && !is_loopback("192.168.0.192"));
    }

    #[test]
    fn the_admin_password_comes_from_the_file_first_line_and_is_never_empty() {
        let dir = std::env::temp_dir().join(format!("pzms-mcp-server-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("pw.txt");
        std::fs::write(&file, "\u{feff}secret one \r\nsecond line\r\n").unwrap();
        let (pw, from) = admin_password("x", Some(&file.to_string_lossy())).unwrap();
        assert_eq!((pw.as_deref(), from), (Some("secret one"), "file"));
        std::fs::write(&file, "\n").unwrap();
        assert!(admin_password("x", Some(&file.to_string_lossy())).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_tools_are_listed_and_dispatched() {
        let paths = crate::vfs::studio_paths();
        for t in tools() {
            assert!(t.description.len() > 40, "{} explains itself", t.name);
            if t.name != "server_start" {
                continue;
            }
            // Without a name it refuses before touching anything.
            let r = execute(&t.name, &json!({}), &paths).unwrap();
            assert!(r.is_error);
        }
        assert!(execute("not_a_server_tool", &json!({}), &paths).is_none());
    }
}
