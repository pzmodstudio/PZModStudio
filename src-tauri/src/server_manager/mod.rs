use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::load_order::mod_info::get_all_user_zomboid_dirs;

pub mod mod_sync;
mod rcon;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PZServerConfig {
    pub name: String,
    pub file_path: String,
    pub mods: Vec<String>,
    pub workshop_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedicatedServerStatus {
    pub is_running: bool,
    pub pid: Option<u32>,
    pub server_name: Option<String>,
    pub memory_gb: Option<u32>,
    pub start_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedPlayer {
    pub username: String,
    pub role: String,
    pub ping: u32,
    pub health: f32,
    pub is_godmode: bool,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub steam_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerQuickSettings {
    pub public_name: String,
    pub public_description: String,
    pub password: String,
    pub max_players: u32,
    pub pvp: bool,
    pub pause_empty: bool,
    pub open: bool,
    pub port: u32,
    pub rcon_port: u32,
    pub rcon_password: String,
    pub map_names: String,
}

#[tauri::command(async)]
pub fn list_server_configs(user_zomboid_dir: String) -> Result<Vec<PZServerConfig>, String> {
    let mut configs = Vec::new();

    for user_dir in get_all_user_zomboid_dirs(&user_zomboid_dir) {
        let server_dir = user_dir.join("Server");
        if server_dir.exists() {
            if let Ok(entries) = fs::read_dir(&server_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("ini") {
                        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        if let Ok(content) = fs::read_to_string(&path) {
                            let mut mods = Vec::new();
                            let mut workshop_items = Vec::new();

                            for line in content.lines() {
                                let trimmed = line.trim();
                                if trimmed.starts_with("Mods=") {
                                    mods = trimmed[5..]
                                        .split(';')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                } else if trimmed.starts_with("WorkshopItems=") {
                                    workshop_items = trimmed[14..]
                                        .split(';')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                }
                            }

                            configs.push(PZServerConfig {
                                name,
                                file_path: path.to_string_lossy().to_string(),
                                mods,
                                workshop_items,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(configs)
}

/// Writes this mod list into a server's `.ini`: `Mods=`, `WorkshopItems=` **and** `Map=`, the
/// third of which the old sync left alone, which is why map mods never showed up on servers set
/// up with it. The ids come from the catalog of what is really installed, not from the caller, so
/// a mod held by several Workshop items (or an item holding several mods) lands in both lines
/// correctly. Everything else in the file keeps its place, and the previous file is backed up.
#[tauri::command(async)]
pub fn sync_client_to_server(
    paths: crate::vfs::StudioPaths,
    file_path: String,
    active_mod_ids: Vec<String>,
) -> Result<mod_sync::ServerLines, String> {
    let path = Path::new(&file_path);
    if !path.is_file() {
        return Err(format!("{} does not exist.", file_path));
    }
    let content = fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {}", file_path, e))?;

    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    let lines = mod_sync::lines_for(
        &catalog,
        &active_mod_ids,
        &mod_sync::ini_list(&content, "Map"),
        &paths.pz_install_dir,
    );
    let updated = mod_sync::with_mod_lines(&content, &lines);

    let backups = Path::new(&paths.user_zomboid_dir).join("PZModStudio_Backups");
    crate::load_order::ini_parser::write_file_with_backup(path, &updated, &backups)?;
    Ok(lines)
}

/// What the server's `.ini` says about mods, and everything in it that will not do what the admin
/// meant: a mod that is never downloaded, an item that is never loaded, a map that is ignored.
#[tauri::command(async)]
pub fn check_server_mods(
    paths: crate::vfs::StudioPaths,
    file_path: String,
) -> Result<mod_sync::ServerModReport, String> {
    let content = fs::read_to_string(Path::new(&file_path))
        .map_err(|e| format!("Cannot read {}: {}", file_path, e))?;
    let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
    Ok(mod_sync::check(&catalog, &content, &paths.pz_install_dir))
}

/// 32 hex characters from the process's random hash keys (no RNG crate needed for this).
/// Sets a random `RCONPassword` in a server's `.ini` when it has none (backed up first).
/// Returns whether it was set.
pub fn ensure_rcon_password(ini_path: &Path, backups: &Path) -> Result<bool, String> {
    let Ok(text) = fs::read_to_string(ini_path) else { return Ok(false) };
    let has = text.lines().any(|l| l.trim().strip_prefix("RCONPassword=").is_some_and(|v| !v.trim().is_empty()));
    if has {
        return Ok(false);
    }
    let text = crate::discord::ini_set(&text, "RCONPassword", &random_password());
    crate::load_order::ini_parser::write_file_with_backup(ini_path, &text, backups)?;
    Ok(true)
}

fn random_password() -> String {
    use std::hash::{BuildHasher, Hasher};
    (0..2)
        .map(|i| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) + i);
            format!("{:016x}", h.finish())
        })
        .collect()
}

/// A server name the game, the file system and cmd.exe all take as it is.
fn is_valid_server_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Whether `Server/` already holds a server by this name (Windows file names ignore case).
fn server_name_taken(server_dir: &Path, name: &str) -> bool {
    let wanted = format!("{}.ini", name).to_lowercase();
    fs::read_dir(server_dir)
        .map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().to_lowercase() == wanted))
        .unwrap_or(false)
}

fn primary_dir(user_zomboid_dir: &str) -> PathBuf {
    if !user_zomboid_dir.is_empty() {
        PathBuf::from(user_zomboid_dir)
    } else if let Some(home) = dirs_next::home_dir() {
        home.join("Zomboid")
    } else {
        PathBuf::from(".")
    }
}

#[cfg(target_os = "windows")]
fn server_pid_alive(pid: u32) -> bool {
    is_tracked_server_alive(pid)
}

#[cfg(not(target_os = "windows"))]
fn server_pid_alive(_pid: u32) -> bool {
    false
}

#[tauri::command]
pub fn create_new_server_config(user_zomboid_dir: String, server_name: String) -> Result<PZServerConfig, String> {
    let clean_name = server_name.trim().replace(' ', "_");
    if !is_valid_server_name(&clean_name) {
        return Err(format!(
            "Invalid server name '{}': use only letters, digits, '_' and '-'.",
            server_name.trim()
        ));
    }

    let user_path = if user_zomboid_dir.is_empty() {
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Zomboid")
    } else {
        PathBuf::from(user_zomboid_dir)
    };

    let server_dir = user_path.join("Server");
    fs::create_dir_all(&server_dir).map_err(|e| e.to_string())?;

    let ini_path = server_dir.join(format!("{}.ini", clean_name));
    // Never replace a server that is already there: its .ini is every setting the admin made.
    if server_name_taken(&server_dir, &clean_name) {
        return Err(format!("A server named '{}' already exists.", clean_name));
    }

    let default_content = format!(
        "PVP=true\r\n\
PauseEmpty=true\r\n\
GlobalChat=true\r\n\
Open=true\r\n\
Public=true\r\n\
PublicName={}\r\n\
PublicDescription=Server managed by Project Zomboid Mod Studio\r\n\
MaxPlayers=16\r\n\
Mods=\r\n\
WorkshopItems=\r\n\
RCONPort=27015\r\n\
RCONPassword={}\r\n",
        clean_name,
        // With RCON on, Stop can tell the server to save and quit instead of killing it.
        random_password()
    );

    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&ini_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, default_content.as_bytes()))
        .map_err(|e| e.to_string())?;

    Ok(PZServerConfig {
        name: clean_name,
        file_path: ini_path.to_string_lossy().to_string(),
        mods: Vec::new(),
        workshop_items: Vec::new(),
    })
}

#[tauri::command]
pub fn delete_server_config(
    user_zomboid_dir: String,
    file_path: String,
    server_name: Option<String>,
) -> Result<bool, String> {
    let p = Path::new(&file_path);
    let Some(server_dir) = p.parent().filter(|d| d.file_name().is_some_and(|n| n == "Server")) else {
        return Err(format!("{} is not a server config.", file_path));
    };
    let srv_name = server_name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
    if srv_name.is_empty() || srv_name.contains(['/', '\\']) || srv_name.contains("..") {
        return Err("Invalid server name.".to_string());
    }

    let primary_user_dir = primary_dir(&user_zomboid_dir);
    if let Some(state) = read_server_state(&primary_user_dir.join("pz_server_state.json")) {
        if state.server_name.as_deref() == Some(srv_name.as_str()) && state.pid.is_some_and(server_pid_alive) {
            return Err(format!("The server '{}' is running: stop it before deleting it.", srv_name));
        }
    }

    // Only this config's own folder, and into a backup rather than away: the .ini is every
    // setting the admin made.
    let zomboid_dir = server_dir.parent().unwrap_or(server_dir);
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let backup = zomboid_dir
        .join("PZModStudio_Backups")
        .join("deleted_servers")
        .join(format!("{}.{}", srv_name, stamp));
    for name in [
        format!("{}.ini", srv_name),
        format!("{}_SandboxVars.lua", srv_name),
        format!("{}_spawnregions.lua", srv_name),
    ] {
        let src = server_dir.join(&name);
        if !src.is_file() {
            continue;
        }
        fs::create_dir_all(&backup).map_err(|e| e.to_string())?;
        if fs::rename(&src, backup.join(&name)).is_err() {
            fs::copy(&src, backup.join(&name)).map_err(|e| format!("Cannot back up {}: {}", name, e))?;
            fs::remove_file(&src).map_err(|e| format!("Cannot delete {}: {}", name, e))?;
        }
    }

    Ok(true)
}

#[tauri::command(async)]
pub fn save_server_log_snapshot(
    user_zomboid_dir: String,
    server_name: String,
    custom_lines: Option<Vec<String>>,
) -> Result<String, String> {
    let user_dirs = get_all_user_zomboid_dirs(&user_zomboid_dir);
    if user_dirs.is_empty() {
        return Err("Invalid Zomboid directory.".to_string());
    }

    let primary_dir = &user_dirs[0];
    let logs_dir = primary_dir.join("Logs");
    let _ = fs::create_dir_all(&logs_dir);

    // Format: "servertest - 2026-08-24_10-48-00.txt"
    let now = chrono::Local::now();
    let timestamp_str = now.format("%Y-%m-%d_%H-%M-%S").to_string();
    let clean_srv = if server_name.trim().is_empty() { "server" } else { server_name.trim() };
    let filename = format!("{} - {}.txt", clean_srv, timestamp_str);
    let target_path = logs_dir.join(&filename);

    let content_to_write = if let Some(lines) = custom_lines {
        lines.join("\r\n")
    } else {
        let recent_lines = get_dedicated_server_logs(user_zomboid_dir.clone(), Some(5000))
            .unwrap_or_default();
        recent_lines.join("\r\n")
    };

    fs::write(&target_path, content_to_write)
        .map_err(|e| format!("Failed to write server log snapshot: {}", e))?;

    Ok(target_path.to_string_lossy().to_string())
}

#[tauri::command(async)]
pub fn launch_dedicated_server(
    pz_install_dir: String,
    user_zomboid_dir: String,
    server_name: String,
    memory_gb: Option<u32>,
    nosteam: Option<bool>,
    admin_password: Option<String>,
) -> Result<u32, String> {
    // The server is started from the game folder the Studio saved, not one the page names.
    let _ = pz_install_dir;
    let pz_install_dir = crate::vfs::trusted_install_dir();
    let install_path = Path::new(&pz_install_dir);
    if !install_path.exists() {
        return Err(format!("Install directory not found: {}", pz_install_dir));
    }

    let mem = memory_gb.unwrap_or(4).max(2).min(32);
    // Not rewritten: "My Server" launched as My_Server would start a fresh vanilla config.
    let clean_name = server_name.trim().to_string();
    if let Ok(mut last) = LAST_LAUNCH.lock() {
        *last = Some((clean_name.clone(), memory_gb, nosteam));
    }
    let is_nosteam = nosteam.unwrap_or(false);
    let clean_admin_pass = admin_password.unwrap_or_default().trim().to_string();

    // The name becomes a file name and, with the .bat fallbacks, part of a cmd.exe command
    // line, so only allow characters that are inert in both.
    if !is_valid_server_name(&clean_name) {
        return Err(format!(
            "Invalid server name '{}': use only letters, digits, '_' and '-' (rename its .ini in Zomboid/Server).",
            server_name
        ));
    }
    if clean_admin_pass.chars().any(|c| "&|<>^%\"!()".contains(c) || c.is_control()) {
        return Err("The admin password cannot contain any of: & | < > ^ % \" ! ( )".to_string());
    }

    // A server the game made has no RCON password: give it one, so Stop saves before quitting and
    // the Discord commands can talk to it.
    let ini_path = Path::new(&user_zomboid_dir).join("Server").join(format!("{}.ini", clean_name));
    if let Err(e) = ensure_rcon_password(&ini_path, &Path::new(&user_zomboid_dir).join("PZModStudio_Backups")) {
        crate::diagnostics::error(format!("[Servers] Could not set an RCON password for {}: {}", clean_name, e));
    }

    let java_exe = install_path.join("jre64").join("bin").join("java.exe");
    let pz_server_bat = install_path.join("ProjectZomboidServer.bat");
    let start_server_bat = install_path.join("StartServer64.bat");
    let pz_exe = install_path.join("ProjectZomboid64.exe");

    let primary_user_dir = if !user_zomboid_dir.is_empty() {
        Path::new(&user_zomboid_dir).to_path_buf()
    } else if let Some(home) = dirs_next::home_dir() {
        home.join("Zomboid")
    } else {
        Path::new(".").to_path_buf()
    };

    // A second server on the same port, with the first one's PID forgotten, could never be stopped.
    if let Some(state) = read_server_state(&primary_user_dir.join("pz_server_state.json")) {
        if let Some(pid) = state.pid.filter(|&p| server_pid_alive(p)) {
            return Err(format!(
                "A server is already running (PID {}): stop it before starting another.",
                pid
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let mut cmd = if java_exe.exists() {
            let mut c = Command::new(&java_exe);
            c.arg("-Djava.awt.headless=true");
            c.arg("--enable-native-access=ALL-UNNAMED");
            c.arg("--add-exports=java.base/jdk.internal.misc=ALL-UNNAMED");
            if is_nosteam {
                c.arg("-Dzomboid.steam=0");
                c.arg("-Dzomboid.znetlog=0");
            } else {
                c.arg("-Dzomboid.steam=1");
                c.arg("-Dzomboid.znetlog=1");
            }
            c.arg("-XX:+UseZGC");
            c.arg("-XX:-CreateCoredumpOnCrash");
            c.arg("-XX:-OmitStackTraceInFastThrow");
            c.arg(format!("-Xmx{}g", mem));
            c.arg("-Djava.library.path=./natives/;./natives/win64/;./");
            c.arg("-cp");
            c.arg("./;projectzomboid.jar");
            c.arg("zombie.network.GameServer");
            c.arg("-servername");
            c.arg(&clean_name);
            c.arg(format!("-cachedir={}", primary_user_dir.display()));
            if !clean_admin_pass.is_empty() {
                c.arg("-adminpassword");
                c.arg(&clean_admin_pass);
            }
            if is_nosteam {
                c.arg("-nosteam");
            }
            c
        } else if pz_server_bat.exists() {
            let mut c = Command::new("cmd.exe");
            let mut args = vec!["/c".to_string(), pz_server_bat.to_string_lossy().to_string(), "-servername".to_string(), clean_name.clone()];
            if !clean_admin_pass.is_empty() {
                args.push("-adminpassword".to_string());
                args.push(clean_admin_pass.clone());
            }
            c.args(&args);
            c
        } else if start_server_bat.exists() {
            let mut c = Command::new("cmd.exe");
            let mut args = vec!["/c".to_string(), start_server_bat.to_string_lossy().to_string(), "-servername".to_string(), clean_name.clone()];
            if !clean_admin_pass.is_empty() {
                args.push("-adminpassword".to_string());
                args.push(clean_admin_pass.clone());
            }
            c.args(&args);
            c
        } else if pz_exe.exists() {
            let mut c = Command::new(&pz_exe);
            let mut args = vec!["-servername".to_string(), clean_name.clone()];
            if !clean_admin_pass.is_empty() {
                args.push("-adminpassword".to_string());
                args.push(clean_admin_pass.clone());
            }
            c.args(&args);
            c
        } else {
            return Err("Could not find java.exe, ProjectZomboidServer.bat, or ProjectZomboid64.exe in install directory.".to_string());
        };

        cmd.current_dir(install_path);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW: runs 100% headless inside PZ Mod Studio!

        let child = cmd.spawn().map_err(|e| format!("Failed to spawn dedicated server: {}", e))?;
        let child_pid = child.id();

        // Clean stale player telemetry from prior sessions
        let _ = fs::remove_file(primary_user_dir.join("Lua").join("pz_server_players.json"));
        let _ = fs::remove_file(primary_user_dir.join("pz_server_players.json"));

        // Save server state to JSON
        let state = DedicatedServerStatus {
            is_running: true,
            pid: Some(child_pid),
            server_name: Some(clean_name),
            memory_gb: Some(mem),
            start_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .ok(),
        };

        let state_file = primary_user_dir.join("pz_server_state.json");
        let _ = fs::write(&state_file, serde_json::to_string_pretty(&state).unwrap_or_default());

        return Ok(child_pid);
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Dedicated server launcher currently implemented for Windows.".to_string())
    }
}

/// Images the launcher can start: java.exe directly, cmd.exe for ProjectZomboidServer.bat,
/// or ProjectZomboid64.exe. A PID read back from the state file is only trusted if it still
/// belongs to one of these (PIDs are reused after a process exits).
#[cfg(target_os = "windows")]
const SERVER_PROCESS_IMAGES: &[&str] = &["java.exe", "cmd.exe", "projectzomboid64.exe"];

/// Returns the lowercase image name of a running process, or None if no process has that PID.
#[cfg(target_os = "windows")]
fn process_image_name(pid: u32) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
        .creation_flags(0x08000000)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().find(|l| l.starts_with('"'))?;
    let mut cols = line.split(',').map(|c| c.trim_matches('"'));
    let image = cols.next()?.to_lowercase();
    let listed_pid = cols.next()?.parse::<u32>().ok()?;
    (listed_pid == pid).then_some(image)
}

#[cfg(target_os = "windows")]
fn is_tracked_server_alive(pid: u32) -> bool {
    process_image_name(pid).is_some_and(|img| SERVER_PROCESS_IMAGES.contains(&img.as_str()))
}

/// Sends "quit" to the server over RCON when its .ini has a password for it. True when sent.
#[cfg(target_os = "windows")]
/// One RCON command to a server this Studio runs (its .ini's RCONPort and RCONPassword).
pub fn rcon_command(zomboid_dir: &Path, server_name: &str, command: &str) -> Result<(), String> {
    rcon_query(zomboid_dir, server_name, command).map(|_| ())
}

/// A moderation action on the players screen, as the server console command it becomes.
pub enum PlayerAction<'a> {
    Kick { user: &'a str, reason: &'a str },
    Ban { user: &'a str, reason: &'a str },
    Message { text: &'a str },
}

/// Text that fits between the console's quotes: one line, no quotes or backslashes.
fn quoted_arg(text: &str, max: usize) -> String {
    text.chars().filter(|c| !matches!(c, '"' | '\\' | '\r' | '\n')).take(max).collect::<String>().trim().to_string()
}

/// The console command for `action` (`kickuser`, `banuser`, `servermsg`), or why there is none.
pub fn player_command(action: &PlayerAction) -> Result<String, String> {
    let with_reason = |verb: &str, user: &str, reason: &str| {
        let user = quoted_arg(user, 64);
        if user.is_empty() {
            return Err("No player name.".to_string());
        }
        let reason = quoted_arg(reason, 200);
        Ok(if reason.is_empty() { format!("{} \"{}\"", verb, user) } else { format!("{} \"{}\" -r \"{}\"", verb, user, reason) })
    };
    match action {
        PlayerAction::Kick { user, reason } => with_reason("kickuser", user, reason),
        PlayerAction::Ban { user, reason } => with_reason("banuser", user, reason),
        PlayerAction::Message { text } => {
            let text = quoted_arg(text, 200);
            if text.is_empty() {
                return Err("The message is empty.".to_string());
            }
            Ok(format!("servermsg \"{}\"", text))
        }
    }
}

/// Sends a moderation action to a server this Studio runs; the server's answer comes back.
pub fn player_action(zomboid_dir: &Path, server_name: &str, action: &PlayerAction) -> Result<String, String> {
    rcon_query(zomboid_dir, server_name, &player_command(action)?)
}

/// One RCON command, with what the server answered.
pub fn rcon_query(zomboid_dir: &Path, server_name: &str, command: &str) -> Result<String, String> {
    let ini = zomboid_dir.join("Server").join(format!("{}.ini", server_name));
    let text = fs::read_to_string(&ini).map_err(|e| format!("Cannot read {}: {}", ini.display(), e))?;
    let value = |key: &str| {
        text.lines()
            .find_map(|l| l.trim().strip_prefix(&format!("{}=", key)).map(|v| v.trim().to_string()))
            .unwrap_or_default()
    };
    let password = value("RCONPassword");
    if password.is_empty() {
        return Err("The server has no RCONPassword.".to_string());
    }
    rcon::send(value("RCONPort").parse::<u16>().unwrap_or(27015), &password, command)
}

/// How the server was last started from the Studio (name, memory, without Steam): a restart
/// asked from Discord starts it the same way.
pub static LAST_LAUNCH: std::sync::Mutex<Option<(String, Option<u32>, Option<bool>)>> = std::sync::Mutex::new(None);

fn quit_over_rcon(zomboid_dir: &Path, server_name: &str) -> bool {
    let ini = zomboid_dir.join("Server").join(format!("{}.ini", server_name));
    let Ok(text) = fs::read_to_string(ini) else { return false };
    let value = |key: &str| {
        text.lines()
            .find_map(|l| l.trim().strip_prefix(&format!("{}=", key)).map(|v| v.trim().to_string()))
            .unwrap_or_default()
    };
    let password = value("RCONPassword");
    let port = value("RCONPort").parse::<u16>().unwrap_or(27015);
    !password.is_empty() && rcon::send(port, &password, "quit").is_ok()
}

fn read_server_state(state_file: &Path) -> Option<DedicatedServerStatus> {
    fs::read_to_string(state_file)
        .ok()
        .and_then(|c| serde_json::from_str::<DedicatedServerStatus>(&c).ok())
}

/// Stops the dedicated server this app launched (the PID saved in pz_server_state.json),
/// together with its child processes. Never touches unrelated java.exe processes.
/// Returns Ok(false) when there is no tracked server to stop.
#[tauri::command(async)]
pub fn stop_dedicated_server(user_zomboid_dir: String, _pid: Option<u32>) -> Result<bool, String> {
    // Asked for: the Discord channel says "stopped", not "crashed".
    crate::discord::note_stop_asked();
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let primary_user_dir = if !user_zomboid_dir.is_empty() {
            Path::new(&user_zomboid_dir).to_path_buf()
        } else if let Some(home) = dirs_next::home_dir() {
            home.join("Zomboid")
        } else {
            Path::new(".").to_path_buf()
        };

        let state_file = primary_user_dir.join("pz_server_state.json");
        let Some(state) = read_server_state(&state_file) else {
            return Ok(false);
        };
        let srv_name = state.server_name.clone().unwrap_or_else(|| "servertest".to_string());

        let mut stopped = false;
        if let Some(pid) = state.pid.filter(|&p| is_tracked_server_alive(p)) {
            // Automatically archive the full server session log with timestamp
            let _ = save_server_log_snapshot(user_zomboid_dir.clone(), srv_name.clone(), None);

            // First the way an admin would: "quit" over RCON saves the world and exits. Only a
            // server that does not answer, or does not go, is killed.
            if quit_over_rcon(&primary_user_dir, &srv_name) {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
                while std::time::Instant::now() < deadline && is_tracked_server_alive(pid) {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
            if !is_tracked_server_alive(pid) {
                let _ = fs::remove_file(primary_user_dir.join("Lua").join("pz_server_players.json"));
                let _ = fs::remove_file(primary_user_dir.join("pz_server_players.json"));
                let _ = fs::remove_file(&state_file);
                return Ok(true);
            }

            // /T also ends the java.exe child when the server was started through the .bat
            let output = Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output()
                .map_err(|e| format!("Failed to run taskkill: {}", e))?;
            if !output.status.success() && is_tracked_server_alive(pid) {
                return Err(format!(
                    "Could not stop server process {}: {}",
                    pid,
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            stopped = true;
        }

        let _ = fs::remove_file(primary_user_dir.join("Lua").join("pz_server_players.json"));
        let _ = fs::remove_file(primary_user_dir.join("pz_server_players.json"));
        let _ = fs::remove_file(&state_file);

        Ok(stopped)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}

#[tauri::command(async)]
pub fn get_dedicated_server_status(user_zomboid_dir: String) -> Result<DedicatedServerStatus, String> {
    let primary_user_dir = if !user_zomboid_dir.is_empty() {
        Path::new(&user_zomboid_dir).to_path_buf()
    } else if let Some(home) = dirs_next::home_dir() {
        home.join("Zomboid")
    } else {
        Path::new(".").to_path_buf()
    };

    let state_file = primary_user_dir.join("pz_server_state.json");
    let mut state = if state_file.exists() {
        fs::read_to_string(&state_file)
            .ok()
            .and_then(|c| serde_json::from_str::<DedicatedServerStatus>(&c).ok())
            .unwrap_or(DedicatedServerStatus {
                is_running: false,
                pid: None,
                server_name: None,
                memory_gb: None,
                start_timestamp: None,
            })
    } else {
        DedicatedServerStatus {
            is_running: false,
            pid: None,
            server_name: None,
            memory_gb: None,
            start_timestamp: None,
        }
    };

    #[cfg(target_os = "windows")]
    {
        // Only the process this app launched counts; any other java.exe is not our server.
        if state.pid.is_some_and(is_tracked_server_alive) {
            state.is_running = true;
            if state.server_name.is_none() {
                state.server_name = Some("servertest".to_string());
            }
            return Ok(state);
        }

        if state_file.exists() {
            let srv_name = state.server_name.clone().unwrap_or_else(|| "servertest".to_string());
            let _ = save_server_log_snapshot(user_zomboid_dir.clone(), srv_name, None);
            let _ = fs::remove_file(&state_file);
        }
        state.is_running = false;
        state.pid = None;
    }

    Ok(state)
}

#[tauri::command(async)]
pub fn get_dedicated_server_logs(
    user_zomboid_dir: String,
    max_lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let primary_user_dir = if !user_zomboid_dir.is_empty() {
        Path::new(&user_zomboid_dir).to_path_buf()
    } else if let Some(home) = dirs_next::home_dir() {
        home.join("Zomboid")
    } else {
        Path::new(".").to_path_buf()
    };

    let limit = max_lines.unwrap_or(300).max(20).min(2000);

    // Look for latest server debug log file in Zomboid/Logs/
    let logs_dir = primary_user_dir.join("Logs");
    let mut server_log_paths = Vec::new();

    if logs_dir.exists() {
        if let Ok(entries) = fs::read_dir(&logs_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    if name.contains("DebugLog-server.txt") || name.contains("DebugLog.txt") {
                        server_log_paths.push(p);
                    }
                }
            }
        }
    }

    server_log_paths.sort_by(|a, b| {
        let meta_a = a.metadata().and_then(|m| m.modified()).ok();
        let meta_b = b.metadata().and_then(|m| m.modified()).ok();
        meta_a.cmp(&meta_b)
    });

    let target_file = server_log_paths.last().cloned().unwrap_or_else(|| {
        primary_user_dir.join("console.txt")
    });

    if !target_file.exists() {
        return Ok(vec!["[PZ Mod Studio] Waiting for dedicated server to write log file...".to_string()]);
    }

    let file = fs::File::open(&target_file)
        .map_err(|e| format!("Failed to open log file '{}': {}", target_file.display(), e))?;
    
    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        lines.push(line);
    }

    let total = lines.len();
    if total > limit {
        Ok(lines[total - limit..].to_vec())
    } else {
        Ok(lines)
    }
}

fn is_clean_player_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") || trimmed.len() < 2 || trimmed.len() > 32 {
        return false;
    }
    if trimmed.contains('[') || trimmed.contains(']') || trimmed.contains('"') || trimmed.contains('=') {
        return false;
    }
    trimmed.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ')
}

#[tauri::command(async)]
pub fn get_connected_players(user_zomboid_dir: String) -> Result<Vec<ConnectedPlayer>, String> {
    let primary_user_dir = if !user_zomboid_dir.is_empty() {
        Path::new(&user_zomboid_dir).to_path_buf()
    } else if let Some(home) = dirs_next::home_dir() {
        home.join("Zomboid")
    } else {
        Path::new(".").to_path_buf()
    };

    // 1. Check pz_server_players.json from Bridge Companion Mod (both in Lua/ and root)
    let players_file_lua = primary_user_dir.join("Lua").join("pz_server_players.json");
    let players_file_root = primary_user_dir.join("pz_server_players.json");

    let target_players_file = if players_file_lua.exists() {
        Some(players_file_lua)
    } else if players_file_root.exists() {
        Some(players_file_root)
    } else {
        None
    };

    if let Some(pf) = target_players_file {
        // Check telemetry freshness: if file was not updated in the last 10 seconds, the client is offline
        let is_fresh = pf.metadata()
            .and_then(|m| m.modified())
            .map(|mod_time| {
                if let Ok(elapsed) = std::time::SystemTime::now().duration_since(mod_time) {
                    elapsed.as_secs() <= 10
                } else {
                    false
                }
            })
            .unwrap_or(false);

        if is_fresh {
            if let Ok(content) = fs::read_to_string(&pf) {
                if let Ok(mut list) = serde_json::from_str::<Vec<ConnectedPlayer>>(&content) {
                    if !list.is_empty() {
                        let roles_file = primary_user_dir.join("Lua").join("pz_player_roles.json");
                        let fallback_roles_file = primary_user_dir.join("pz_player_roles.json");
                        let roles_map: std::collections::HashMap<String, String> = if roles_file.exists() {
                            fs::read_to_string(&roles_file)
                                .ok()
                                .and_then(|s| serde_json::from_str(&s).ok())
                                .unwrap_or_default()
                        } else if fallback_roles_file.exists() {
                            fs::read_to_string(&fallback_roles_file)
                                .ok()
                                .and_then(|s| serde_json::from_str(&s).ok())
                                .unwrap_or_default()
                        } else {
                            std::collections::HashMap::new()
                        };

                        for player in list.iter_mut() {
                            if let Some(r) = roles_map.get(&player.username) {
                                if !r.is_empty() && r.to_lowercase() != "none" {
                                    player.role = r.clone();
                                }
                            }
                        }
                        return Ok(list);
                    }
                }
            }
        }
    }

    let state_file = primary_user_dir.join("pz_server_state.json");
    let (is_server_running, server_start_ts) = if state_file.exists() {
        if let Ok(content) = fs::read_to_string(&state_file) {
            if let Ok(st) = serde_json::from_str::<DedicatedServerStatus>(&content) {
                (st.is_running, st.start_timestamp)
            } else {
                (false, None)
            }
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    // 2. Parse connection logs from Logs/ (only if server is running and logs belong to current session)
    let logs_dir = primary_user_dir.join("Logs");
    let mut online_map: std::collections::HashMap<String, ConnectedPlayer> = std::collections::HashMap::new();

    if is_server_running && logs_dir.exists() {
        if let Ok(entries) = fs::read_dir(&logs_dir) {
            let mut user_log_paths = Vec::new();
            let mut conn_log_paths = Vec::new();

            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(srv_start) = server_start_ts {
                        let file_mod_ts = p.metadata()
                            .and_then(|m| m.modified())
                            .ok()
                            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        if file_mod_ts + 5 < srv_start {
                            continue;
                        }
                    }

                    let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if name.contains("user.txt") {
                        user_log_paths.push(p.clone());
                    }
                    if name.contains("connections.txt") {
                        conn_log_paths.push(p);
                    }
                }
            }

            user_log_paths.sort_by(|a, b| {
                let meta_a = a.metadata().and_then(|m| m.modified()).ok();
                let meta_b = b.metadata().and_then(|m| m.modified()).ok();
                meta_a.cmp(&meta_b)
            });
            conn_log_paths.sort_by(|a, b| {
                let meta_a = a.metadata().and_then(|m| m.modified()).ok();
                let meta_b = b.metadata().and_then(|m| m.modified()).ok();
                meta_a.cmp(&meta_b)
            });

            // Process latest user.txt
            if let Some(latest_user) = user_log_paths.last() {
                if let Ok(content) = fs::read_to_string(latest_user) {
                    for line in content.lines() {
                        let is_connect = line.contains("fully connected")
                            || (line.contains("connected")
                                && !line.contains("disconnected")
                                && !line.contains("Connection closed")
                                && !line.contains("Connection disconnect")
                                && !line.contains("Connection remove"));
                        let is_disconnect = line.contains("disconnected")
                            || line.contains("Connection closed")
                            || line.contains("Connection disconnect")
                            || line.contains("Connection remove");
                        let is_death = line.contains(" died at ");

                        if is_connect {
                            let mut username = String::new();
                            let mut steam_id = String::new();
                            let mut x = 0;
                            let mut y = 0;
                            let mut z = 0;

                            // Pattern: [25-08-26 01:39:14.144] 76561190000000000 "player" fully connected (11902,6894,0).
                            let quote_parts: Vec<&str> = line.split('"').collect();
                            if quote_parts.len() >= 3 {
                                username = quote_parts[1].trim().to_string();
                                if let Some(first_part) = quote_parts.first() {
                                    steam_id = first_part
                                        .split_whitespace()
                                        .last()
                                        .unwrap_or("")
                                        .chars()
                                        .filter(|c| c.is_ascii_digit())
                                        .collect();
                                }
                            } else if let Some(pos) = line.find("user ") {
                                let rest = &line[pos + 5..];
                                username = rest.split_whitespace().next().unwrap_or("").trim_matches('"').to_string();
                            } else if let Some(pos) = line.find("player ") {
                                let rest = &line[pos + 7..];
                                username = rest.split_whitespace().next().unwrap_or("").trim_matches('"').to_string();
                            }

                            if let Some(coord_start) = line.rfind('(') {
                                if let Some(coord_end) = line[coord_start..].find(')') {
                                    let coords_str = &line[coord_start + 1..coord_start + coord_end];
                                    let parts: Vec<&str> = coords_str.split(',').map(|s| s.trim()).collect();
                                    if parts.len() == 3 {
                                        x = parts[0].parse::<i32>().unwrap_or(0);
                                        y = parts[1].parse::<i32>().unwrap_or(0);
                                        z = parts[2].parse::<i32>().unwrap_or(0);
                                    }
                                }
                            }

                            if is_clean_player_name(&username) {
                                online_map.insert(username.clone(), ConnectedPlayer {
                                    username,
                                    role: "User".to_string(),
                                    ping: 35,
                                    health: 1.0,
                                    is_godmode: false,
                                    x,
                                    y,
                                    z,
                                    steam_id,
                                });
                            }
                        } else if is_disconnect {
                            let quote_parts: Vec<&str> = line.split('"').collect();
                            if quote_parts.len() >= 3 {
                                let username = quote_parts[1].trim();
                                online_map.remove(username);
                            } else if let Some(pos) = line.find("player ") {
                                let rest = &line[pos + 7..];
                                let username = rest.split_whitespace().next().unwrap_or("").trim_matches('"');
                                online_map.remove(username);
                            } else if let Some(pos) = line.find("user ") {
                                let rest = &line[pos + 5..];
                                let username = rest.split_whitespace().next().unwrap_or("").trim_matches('"');
                                online_map.remove(username);
                            }
                        } else if is_death {
                            if let Some(pos) = line.find("user ") {
                                if let Some(died_pos) = line.find(" died at ") {
                                    let username = line[pos + 5..died_pos].trim().trim_matches('"');
                                    if let Some(player) = online_map.get_mut(username) {
                                        player.health = 0.0;
                                        if let Some(coord_start) = line.rfind('(') {
                                            if let Some(coord_end) = line[coord_start..].find(')') {
                                                let coords_str = &line[coord_start + 1..coord_start + coord_end];
                                                let parts: Vec<&str> = coords_str.split(',').map(|s| s.trim()).collect();
                                                if parts.len() >= 3 {
                                                    player.x = parts[0].parse::<i32>().unwrap_or(player.x);
                                                    player.y = parts[1].parse::<i32>().unwrap_or(player.y);
                                                    player.z = parts[2].parse::<i32>().unwrap_or(player.z);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Also enhance with latest connections.txt (roles, steam_id)
            if let Some(latest_conn) = conn_log_paths.last() {
                if let Ok(content) = fs::read_to_string(latest_conn) {
                    for line in content.lines() {
                        if line.contains("username=") {
                            let mut username = String::new();
                            let mut role = String::new();
                            let mut steam_id = String::new();

                            for part in line.split_whitespace() {
                                if part.starts_with("username=\"") {
                                    username = part.trim_start_matches("username=\"").trim_end_matches('"').trim_end_matches('.').to_string();
                                } else if part.starts_with("role=\"") {
                                    role = part.trim_start_matches("role=\"").trim_end_matches('"').to_string();
                                } else if part.starts_with("steam-id=\"") {
                                    steam_id = part.trim_start_matches("steam-id=\"").trim_end_matches('"').to_string();
                                }
                            }

                            if is_clean_player_name(&username) {
                                if line.contains("event=\"disconnected\"") || line.contains("connection-type=\"Disconnected\"") {
                                    online_map.remove(&username);
                                } else if line.contains("fully-connected") || line.contains("client-connect") || line.contains("player-connect") {
                                    if let Some(player) = online_map.get_mut(&username) {
                                        if !role.is_empty() && role != "null" {
                                            player.role = role;
                                        }
                                        if !steam_id.is_empty() && steam_id != "0" {
                                            player.steam_id = steam_id;
                                        }
                                    } else if line.contains("fully-connected") {
                                        online_map.insert(username.clone(), ConnectedPlayer {
                                            username,
                                            role: if !role.is_empty() && role != "null" { role } else { "User".to_string() },
                                            ping: 35,
                                            health: 1.0,
                                            is_godmode: false,
                                            x: 0,
                                            y: 0,
                                            z: 0,
                                            steam_id: if steam_id != "0" { steam_id } else { String::new() },
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Enhance with dynamically assigned persistent roles
    let roles_file = primary_user_dir.join("Lua").join("pz_player_roles.json");
    let fallback_roles_file = primary_user_dir.join("pz_player_roles.json");
    let roles_map: std::collections::HashMap<String, String> = if roles_file.exists() {
        fs::read_to_string(&roles_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else if fallback_roles_file.exists() {
        fs::read_to_string(&fallback_roles_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };

    for player in online_map.values_mut() {
        if let Some(r) = roles_map.get(&player.username) {
            player.role = r.clone();
        }
    }

    let result: Vec<ConnectedPlayer> = online_map.into_values().collect();
    Ok(result)
}

#[tauri::command]
pub fn send_server_command(
    user_zomboid_dir: String,
    action: String,
    target: Option<String>,
    message: Option<String>,
    reason: Option<String>,
) -> Result<bool, String> {
    let primary_user_dir = if !user_zomboid_dir.is_empty() {
        Path::new(&user_zomboid_dir).to_path_buf()
    } else if let Some(home) = dirs_next::home_dir() {
        home.join("Zomboid")
    } else {
        Path::new(".").to_path_buf()
    };

    let payload = serde_json::json!({
        "action": action,
        "target": target,
        "message": message,
        "reason": reason,
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
    });

    let json_text = serde_json::to_string_pretty(&payload).unwrap_or_default();

    // 1. Write to Zomboid/Lua/ (where PZ getFileReader reads)
    let lua_dir = primary_user_dir.join("Lua");
    let _ = fs::create_dir_all(&lua_dir);
    let _ = fs::write(lua_dir.join("pz_server_commands.json"), &json_text);
    let _ = fs::write(lua_dir.join("pz_ipc_queue.json"), &json_text);

    // 2. Write to root Zomboid/ (fallback)
    let _ = fs::write(primary_user_dir.join("pz_server_commands.json"), &json_text);
    let _ = fs::write(primary_user_dir.join("pz_ipc_queue.json"), &json_text);

    // 3. If setting role / access level, persist to pz_player_roles.json
    if action == "set_access_level" || action == "grant_admin" || action == "set_role" {
        if let Some(ref t) = target {
            let role_val = message.clone().unwrap_or_else(|| "admin".to_string());
            let roles_file = lua_dir.join("pz_player_roles.json");
            let mut roles_map: std::collections::HashMap<String, String> = if roles_file.exists() {
                fs::read_to_string(&roles_file)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default()
            } else {
                std::collections::HashMap::new()
            };

            if role_val == "none" || role_val.is_empty() || role_val.to_lowercase() == "user" {
                roles_map.remove(t);
            } else {
                roles_map.insert(t.clone(), role_val);
            }

            if let Ok(serialized) = serde_json::to_string_pretty(&roles_map) {
                let _ = fs::write(&roles_file, &serialized);
                let _ = fs::write(primary_user_dir.join("pz_player_roles.json"), &serialized);
            }
        }
    }

    Ok(true)
}

#[tauri::command(async)]
pub fn get_server_quick_settings(file_path: String) -> Result<ServerQuickSettings, String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("Server .ini file not found.".to_string());
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;

    let mut settings = ServerQuickSettings {
        public_name: String::new(),
        public_description: String::new(),
        password: String::new(),
        max_players: 16,
        pvp: true,
        pause_empty: true,
        open: true,
        port: 16261,
        rcon_port: 27015,
        rcon_password: String::new(),
        map_names: "Muldraugh, KY".to_string(),
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("PublicName=") {
            settings.public_name = trimmed[11..].trim().to_string();
        } else if trimmed.starts_with("PublicDescription=") {
            settings.public_description = trimmed[18..].trim().to_string();
        } else if trimmed.starts_with("Password=") {
            settings.password = trimmed[9..].trim().to_string();
        } else if trimmed.starts_with("MaxPlayers=") {
            if let Ok(v) = trimmed[11..].trim().parse::<u32>() {
                settings.max_players = v;
            }
        } else if trimmed.starts_with("PVP=") {
            settings.pvp = trimmed[4..].trim().eq_ignore_ascii_case("true");
        } else if trimmed.starts_with("PauseEmpty=") {
            settings.pause_empty = trimmed[11..].trim().eq_ignore_ascii_case("true");
        } else if trimmed.starts_with("Open=") {
            settings.open = trimmed[5..].trim().eq_ignore_ascii_case("true");
        } else if trimmed.starts_with("DefaultPort=") {
            if let Ok(v) = trimmed[12..].trim().parse::<u32>() {
                settings.port = v;
            }
        } else if trimmed.starts_with("RCONPort=") {
            if let Ok(v) = trimmed[9..].trim().parse::<u32>() {
                settings.rcon_port = v;
            }
        } else if trimmed.starts_with("RCONPassword=") {
            settings.rcon_password = trimmed[13..].trim().to_string();
        } else if trimmed.starts_with("Map=") {
            settings.map_names = trimmed[4..].trim().to_string();
        }
    }

    Ok(settings)
}

#[tauri::command]
pub fn save_server_quick_settings(file_path: String, settings: ServerQuickSettings) -> Result<bool, String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("Server .ini file not found.".to_string());
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut new_lines = Vec::new();

    let mut keys_found = std::collections::HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("PublicName=") {
            new_lines.push(format!("PublicName={}", settings.public_name));
            keys_found.insert("PublicName");
        } else if trimmed.starts_with("PublicDescription=") {
            new_lines.push(format!("PublicDescription={}", settings.public_description));
            keys_found.insert("PublicDescription");
        } else if trimmed.starts_with("Password=") {
            new_lines.push(format!("Password={}", settings.password));
            keys_found.insert("Password");
        } else if trimmed.starts_with("MaxPlayers=") {
            new_lines.push(format!("MaxPlayers={}", settings.max_players));
            keys_found.insert("MaxPlayers");
        } else if trimmed.starts_with("PVP=") {
            new_lines.push(format!("PVP={}", settings.pvp));
            keys_found.insert("PVP");
        } else if trimmed.starts_with("PauseEmpty=") {
            new_lines.push(format!("PauseEmpty={}", settings.pause_empty));
            keys_found.insert("PauseEmpty");
        } else if trimmed.starts_with("Open=") {
            new_lines.push(format!("Open={}", settings.open));
            keys_found.insert("Open");
        } else if trimmed.starts_with("DefaultPort=") {
            new_lines.push(format!("DefaultPort={}", settings.port));
            keys_found.insert("DefaultPort");
        } else if trimmed.starts_with("RCONPort=") {
            new_lines.push(format!("RCONPort={}", settings.rcon_port));
            keys_found.insert("RCONPort");
        } else if trimmed.starts_with("RCONPassword=") {
            new_lines.push(format!("RCONPassword={}", settings.rcon_password));
            keys_found.insert("RCONPassword");
        } else if trimmed.starts_with("Map=") {
            new_lines.push(format!("Map={}", settings.map_names));
            keys_found.insert("Map");
        } else {
            new_lines.push(line.to_string());
        }
    }

    // Append any missing keys
    if !keys_found.contains("PublicName") { new_lines.push(format!("PublicName={}", settings.public_name)); }
    if !keys_found.contains("PublicDescription") { new_lines.push(format!("PublicDescription={}", settings.public_description)); }
    if !keys_found.contains("Password") { new_lines.push(format!("Password={}", settings.password)); }
    if !keys_found.contains("MaxPlayers") { new_lines.push(format!("MaxPlayers={}", settings.max_players)); }
    if !keys_found.contains("PVP") { new_lines.push(format!("PVP={}", settings.pvp)); }
    if !keys_found.contains("PauseEmpty") { new_lines.push(format!("PauseEmpty={}", settings.pause_empty)); }
    if !keys_found.contains("Open") { new_lines.push(format!("Open={}", settings.open)); }
    if !keys_found.contains("DefaultPort") { new_lines.push(format!("DefaultPort={}", settings.port)); }
    if !keys_found.contains("RCONPort") { new_lines.push(format!("RCONPort={}", settings.rcon_port)); }
    if !keys_found.contains("RCONPassword") { new_lines.push(format!("RCONPassword={}", settings.rcon_password)); }
    if !keys_found.contains("Map") && !settings.map_names.is_empty() { new_lines.push(format!("Map={}", settings.map_names)); }

    let eol = if content.contains("\r\n") || !content.contains('\n') { "\r\n" } else { "\n" };
    let mut updated = new_lines.join(eol);
    if content.is_empty() || content.ends_with('\n') {
        updated.push_str(eol);
    }
    let backups = path
        .parent()
        .and_then(Path::parent)
        .map(|z| z.join("PZModStudio_Backups"))
        .unwrap_or_else(|| PathBuf::from("PZModStudio_Backups"));
    crate::load_order::ini_parser::write_file_with_backup(path, &updated, &backups)?;

    Ok(true)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_actions_become_console_commands() {
        let kick = PlayerAction::Kick { user: "Bob", reason: "" };
        assert_eq!(player_command(&kick).unwrap(), r#"kickuser "Bob""#);
        let ban = PlayerAction::Ban { user: "Bob \"x\"", reason: "griefing\nthe base" };
        assert_eq!(player_command(&ban).unwrap(), r#"banuser "Bob x" -r "griefingthe base""#, "quotes and new lines cannot break out");
        let msg = PlayerAction::Message { text: "Restart in 5 minutes" };
        assert_eq!(player_command(&msg).unwrap(), r#"servermsg "Restart in 5 minutes""#);
        assert!(player_command(&PlayerAction::Kick { user: "  ", reason: "" }).is_err());
        assert!(player_command(&PlayerAction::Message { text: "\"" }).is_err());
    }

    fn temp_zomboid(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pzms-server-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Server")).unwrap();
        dir
    }

    #[test]
    fn creating_a_server_never_replaces_one_that_is_there() {
        let z = temp_zomboid("create");
        let ini = z.join("Server").join("servertest.ini");
        fs::write(&ini, "Mods=MyMods\r\n").unwrap();
        assert!(create_new_server_config(z.to_string_lossy().to_string(), "servertest".into()).is_err());
        assert!(create_new_server_config(z.to_string_lossy().to_string(), "ServerTest".into()).is_err());
        assert_eq!(fs::read_to_string(&ini).unwrap(), "Mods=MyMods\r\n");
        assert!(create_new_server_config(z.to_string_lossy().to_string(), "../x".into()).is_err());
        let made = create_new_server_config(z.to_string_lossy().to_string(), "My Server".into()).unwrap();
        assert_eq!(made.name, "My_Server");
        assert!(fs::read_to_string(&made.file_path).unwrap().contains("RCONPassword="));
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn deleting_a_server_keeps_its_files_in_the_backups() {
        let z = temp_zomboid("delete");
        let ini = z.join("Server").join("srv.ini");
        fs::write(&ini, "PVP=true\r\n").unwrap();
        fs::write(z.join("Server").join("srv_SandboxVars.lua"), "SandboxVars = {}").unwrap();
        delete_server_config(z.to_string_lossy().to_string(), ini.to_string_lossy().to_string(), None).unwrap();
        assert!(!ini.exists());
        let kept: Vec<_> = fs::read_dir(z.join("PZModStudio_Backups").join("deleted_servers")).unwrap().flatten().collect();
        assert_eq!(kept.len(), 1);
        assert!(kept[0].path().join("srv.ini").is_file() && kept[0].path().join("srv_SandboxVars.lua").is_file());
        let _ = fs::remove_dir_all(&z);
    }

    #[test]
    fn quick_settings_keep_line_endings_and_back_up() {
        let z = temp_zomboid("quick");
        let ini = z.join("Server").join("srv.ini");
        fs::write(&ini, "PVP=true\nCustom=1\n").unwrap();
        let mut s = get_server_quick_settings(ini.to_string_lossy().to_string()).unwrap();
        s.pvp = false;
        save_server_quick_settings(ini.to_string_lossy().to_string(), s).unwrap();
        let text = fs::read_to_string(&ini).unwrap();
        assert!(text.starts_with("PVP=false\nCustom=1\n") && !text.contains('\r') && text.ends_with('\n'), "{:?}", text);
        assert!(z.join("PZModStudio_Backups").read_dir().unwrap().count() >= 1);
        let _ = fs::remove_dir_all(&z);
    }
}
