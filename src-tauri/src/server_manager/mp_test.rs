//! A multiplayer test on one PC: the Studio's dedicated server and one or two games joined to it.
//!
//! What a NoSteam test needs, learned by doing it by hand (KnoxLives, November 2026):
//! - Two games on one PC cannot both be Steam clients of one account, so the test runs without
//!   Steam: the server with `-nosteam`, the games with `-nosteam +connect 127.0.0.1:<port>`.
//! - A NoSteam server and a NoSteam game read mods only from `<cachedir>/mods`. The second game
//!   gets a Zomboid folder of its own (`-cachedir=<home>/Zomboid2`), so its options, saves and
//!   logs do not mix with the player's; its `mods` needs every mod of the server's `Mods=`. Real
//!   copies, with the Studio's marker (a link into `Zomboid/mods` failed to load: "Invalid prefix
//!   found for ...\Zomboid\mods\..."); a mod in development that is itself a link elsewhere is
//!   linked to the same place.
//! - In multiplayer each game loads the server's list, not the player's profile.
//! - A game is "connecting" from `ConnectToServerState: Start` in its console.txt and in the
//!   world from `game loading took N seconds`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::java_mods::{self, CopyNeed, WorkshopCopy};
use crate::catalog::{ModCatalog, ModSource};

/// One process of the test, as the dashboard shows it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MpProcess {
    /// "server" or "player".
    pub role: String,
    pub pid: u32,
    /// The server's config name, or "1" / "2" for the games (2: its own Zomboid folder).
    pub name: String,
    pub zomboid_dir: String,
    pub log_path: String,
    pub ram_mb: u64,
    /// Its Java heap limit (-Xmx), when known.
    pub ram_max_mb: Option<u64>,
    pub cpu_percent: f64,
    /// server: starting / started; player: menu / connecting / in_game.
    pub state: String,
    /// `ERROR` lines in its log (the log starts again with each launch).
    pub errors: u32,
    /// The address a game was started to join (`+connect`).
    pub connect: Option<String>,
    /// What it is doing now: the last line of its log, its prefix taken off.
    pub step: Option<String>,
    /// Seconds since the process started.
    pub uptime_s: u64,
    /// A game: the screen its Studio Link last said it is on (menu, spawn_select,
    /// character_profession, character_look, in_game).
    pub screen: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MpStatus {
    pub pc_total_mb: u64,
    pub pc_free_mb: u64,
    pub processes: Vec<MpProcess>,
    /// The second player's Zomboid folder, and whether it is there yet.
    pub second_dir: String,
    pub second_dir_ready: bool,
    /// What the started test is doing now ("waiting for the server", "players launched", ...).
    pub phase: Option<String>,
    /// The Java heap each game asks for (-Xmx in ProjectZomboid64.json), for the memory check.
    pub game_heap_mb: Option<u64>,
}

/// What getting the second player's folder ready did.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PrepareReport {
    pub second_dir: String,
    pub copied: Vec<String>,
    pub linked: Vec<String>,
    /// Up to date already, or a folder someone put there by hand (never replaced).
    pub kept: Vec<String>,
    /// In the server's list but installed nowhere on this PC.
    pub missing: Vec<String>,
}

/// `<home>/Zomboid2` beside the player's Zomboid folder.
pub fn second_dir(primary: &Path) -> PathBuf {
    let name = primary.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Zomboid".into());
    primary.parent().map(|p| p.join(format!("{}2", name))).unwrap_or_else(|| PathBuf::from(format!("{}2", name)))
}

fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| {
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            m.file_type().is_symlink() || m.file_attributes() & 0x400 != 0
        }
        #[cfg(not(windows))]
        {
            m.file_type().is_symlink()
        }
    })
}

/// A directory junction (Windows) or symbolic link at `link` to `target`.
fn make_link(link: &Path, target: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link).map_err(|e| e.to_string())
    }
}

/// Puts the second player's Zomboid folder in order for the server `server`: the folder itself,
/// the player's options.ini the first time (resolution, keys), and in its `mods` every mod of the
/// server's `Mods=`: copied with the Studio's marker (again when the source got newer), or linked
/// when the mod is a link to a mod in development. A folder there without the Studio's marker is kept
/// while it still holds what the server runs, and copied again once it differs.
pub fn prepare_second(catalog: &ModCatalog, primary: &Path, server: &str) -> Result<PrepareReport, String> {
    let ini = primary.join("Server").join(format!("{}.ini", server));
    let content = std::fs::read_to_string(&ini).map_err(|e| format!("Cannot read {}: {}", ini.display(), e))?;
    let second = second_dir(primary);
    let mods_dir = second.join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| format!("Cannot create {}: {}", mods_dir.display(), e))?;
    let options = second.join("options.ini");
    if !options.exists() {
        let _ = std::fs::copy(primary.join("options.ini"), &options);
    }
    // The Studio Link (it fills in the account on the connect window), and in the folder's own
    // list the server's mods: before it connects a game runs the mods of its own folder's list,
    // so player 2's main menu is player 1's.
    crate::studio_link::install_link_files_at(&second)?;
    let mut menu_mods = super::mod_sync::ini_list(&content, "Mods");
    if !menu_mods.iter().any(|m| m == crate::studio_link::LINK_MOD_ID) {
        menu_mods.push(crate::studio_link::LINK_MOD_ID.to_string());
    }
    std::fs::write(mods_dir.join("default.txt"), crate::load_order::ini_parser::build_default_txt(&menu_mods, &[]))
        .map_err(|e| format!("Cannot write {}: {}", mods_dir.join("default.txt").display(), e))?;
    let mut report = PrepareReport { second_dir: second.to_string_lossy().to_string(), ..Default::default() };
    for id in super::mod_sync::ini_list(&content, "Mods") {
        // The copy the game on this PC uses: Zomboid/mods first, then the Workshop.
        let local = catalog.mods.iter().find(|m| m.id == id && matches!(m.source, ModSource::Local));
        let workshop = catalog.mods.iter().find_map(|m| match &m.source {
            ModSource::Workshop { workshop_id } if m.id == id => Some((m, workshop_id.clone())),
            _ => None,
        });
        let (source, workshop_id) = match (local, &workshop) {
            (Some(l), _) => (PathBuf::from(&l.root_dir), workshop.as_ref().map(|(_, w)| w.clone()).unwrap_or_default()),
            (None, Some((w, wid))) => (PathBuf::from(&w.root_dir), wid.clone()),
            (None, None) => {
                report.missing.push(id);
                continue;
            }
        };
        let folder = source.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if folder.is_empty() {
            continue;
        }
        let dest = mods_dir.join(&folder);
        // A mod in development, linked into Zomboid/mods from elsewhere: linked the same way.
        if is_link(&source) && !source.to_string_lossy().to_ascii_lowercase().contains("workshop") {
            if std::fs::symlink_metadata(&dest).is_ok() {
                report.kept.push(id);
                continue;
            }
            let target = std::fs::canonicalize(&source).map_err(|e| format!("Cannot follow the link {}: {}", source.display(), e))?;
            make_link(&dest, &target).map_err(|e| format!("Could not link {}: {}", dest.display(), e))?;
            report.linked.push(id);
            continue;
        }
        // A copy: missing, or the Studio's own copy outgrown by its source.
        let need = if std::fs::symlink_metadata(&dest).is_err() {
            CopyNeed::Missing
        } else if java_mods::marker_is_stale(&dest, &source) == Some(true) {
            CopyNeed::Stale
        } else if !is_link(&dest) && java_mods::unmarked_differs(&dest, &source) {
            // Put there by a script or by hand, and no longer what the server runs (the mod was
            // updated, or got a compatible copy): this folder exists only for the test, so it is
            // made the Studio's and copied again. One that still matches is left as it is.
            java_mods::adopt_unmarked(&dest, &id, &workshop_id, &source)?;
            CopyNeed::Stale
        } else {
            report.kept.push(id);
            continue;
        };
        let real_source = std::fs::canonicalize(&source).unwrap_or(source.clone());
        let copy = WorkshopCopy {
            id: id.clone(),
            name: id.clone(),
            source: real_source.to_string_lossy().to_string(),
            folder,
            workshop_id,
            need,
            required_by: vec![],
        };
        java_mods::copy_into_mods(&second, &[copy])?;
        report.copied.push(id);
    }
    Ok(report)
}

/// Whether the second player's folder has what the server needs, without copying anything.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SecondCheck {
    pub ready: bool,
    /// Server mods not in the second folder yet.
    pub missing: Vec<String>,
    /// The Studio's copies there that their source outgrew.
    pub outdated: Vec<String>,
    /// The Studio Link there is not this app's (missing or another version).
    pub link_outdated: bool,
}

/// What "Get player 2 ready" would do now, for the button to say whether it is needed.
pub fn check_second(catalog: &ModCatalog, primary: &Path, server: &str) -> SecondCheck {
    let second = second_dir(primary);
    let mods_dir = second.join("mods");
    let mut check = SecondCheck::default();
    let content = std::fs::read_to_string(primary.join("Server").join(format!("{}.ini", server))).unwrap_or_default();
    for id in super::mod_sync::ini_list(&content, "Mods") {
        let local = catalog.mods.iter().find(|m| m.id == id && matches!(m.source, ModSource::Local));
        let workshop = catalog.mods.iter().find(|m| m.id == id && matches!(m.source, ModSource::Workshop { .. }));
        let Some(source) = local.or(workshop).map(|m| PathBuf::from(&m.root_dir)) else { continue };
        let Some(folder) = source.file_name().map(|n| n.to_string_lossy().to_string()) else { continue };
        let dest = mods_dir.join(&folder);
        if std::fs::symlink_metadata(&dest).is_err() {
            check.missing.push(id);
        } else if java_mods::marker_is_stale(&dest, &std::fs::canonicalize(&source).unwrap_or(source.clone())) == Some(true)
            || (!is_link(&dest) && !is_link(&source) && java_mods::unmarked_differs(&dest, &source))
        {
            check.outdated.push(id);
        }
    }
    let link_info = std::fs::read_to_string(mods_dir.join(crate::studio_link::LINK_MOD_ID).join("42").join("mod.info")).unwrap_or_default();
    check.link_outdated = crate::studio_link::mod_version(&link_info) != crate::studio_link::bundled_version();
    check.ready = second.join("mods").is_dir() && check.missing.is_empty() && check.outdated.is_empty() && !check.link_outdated;
    check
}

/// One running process, read from Windows.
#[derive(Debug, Clone, Deserialize, Default)]
struct ProcRow {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "Cmd", default)]
    cmd: Option<String>,
    #[serde(rename = "Ram", default)]
    ram: u64,
    #[serde(rename = "Cpu", default)]
    cpu: f64,
    #[serde(rename = "Up", default)]
    up: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct Snapshot {
    #[serde(rename = "Total", default)]
    total_kb: u64,
    #[serde(rename = "Free", default)]
    free_kb: u64,
    #[serde(rename = "Procs", default)]
    procs: Vec<ProcRow>,
}

/// The game's and servers' processes with their command lines, memory and CPU, and the PC's
/// memory, in one PowerShell call (CIM; the CPU is the performance counter's last sample).
fn snapshot() -> Snapshot {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CPU: two readings of each process's CPU time half a second apart (the performance
        // counter class reads 0 on its first sample of a new process, and is slower).
        let script = r#"$os = Get-CimInstance Win32_OperatingSystem
$cores = [Environment]::ProcessorCount
$list = @(Get-CimInstance Win32_Process -Filter "Name='ProjectZomboid64.exe' OR Name='java.exe'")
$t0 = @{}; foreach ($p in $list) { $g = Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue; if ($g) { $t0[[int]$p.ProcessId] = $g.TotalProcessorTime.TotalSeconds } }
$w = [Diagnostics.Stopwatch]::StartNew(); Start-Sleep -Milliseconds 500; $secs = $w.Elapsed.TotalSeconds
$procs = @($list | ForEach-Object { $g = Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue; $cpu = 0; if ($g -and $t0.ContainsKey([int]$_.ProcessId)) { $cpu = ($g.TotalProcessorTime.TotalSeconds - $t0[[int]$_.ProcessId]) / $secs / $cores * 100 }; [pscustomobject]@{ Id = [int]$_.ProcessId; Name = $_.Name; Cmd = $_.CommandLine; Ram = [uint64]$_.WorkingSetSize; Cpu = [math]::Round($cpu, 1); Up = [uint64][math]::Max(0, ((Get-Date) - $_.CreationDate).TotalSeconds) } })
[pscustomobject]@{ Total = [uint64]$os.TotalVisibleMemorySize; Free = [uint64]$os.FreePhysicalMemory; Procs = $procs } | ConvertTo-Json -Depth 3 -Compress"#;
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(0x08000000)
            .output();
        if let Ok(out) = out {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Ok(s) = serde_json::from_str::<Snapshot>(text.trim()) {
                return s;
            }
        }
        Snapshot::default()
    }
    #[cfg(not(windows))]
    {
        Snapshot::default()
    }
}

/// The value of `-name=value` or `-name value` in a command line.
fn arg_value(cmd: &str, name: &str) -> Option<String> {
    let mut parts = split_args(cmd).into_iter();
    while let Some(p) = parts.next() {
        if let Some(v) = p.strip_prefix(&format!("{}=", name)) {
            return Some(v.to_string());
        }
        if p == name {
            return parts.next();
        }
    }
    None
}

/// A Windows command line in its arguments (quotes kept together, quotes removed).
fn split_args(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in cmd.chars() {
        match c {
            '"' => quoted = !quoted,
            ' ' | '\t' if !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// `-Xmx6144m` / `-Xmx4g` in megabytes.
fn xmx_mb(text: &str) -> Option<u64> {
    let start = text.find("-Xmx")? + 4;
    let rest: String = text[start..].chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
    let (num, unit) = rest.split_at(rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len()));
    let n: u64 = num.parse().ok()?;
    Some(match unit.to_ascii_lowercase().as_str() {
        "g" => n * 1024,
        "k" => n / 1024,
        "" => n / (1024 * 1024),
        _ => n,
    })
}

/// What a log says about where its game or server is, and how many ERROR lines it has.
pub fn read_log_state(text: &str, server: bool) -> (String, u32) {
    let errors = text.lines().filter(|l| l.starts_with("ERROR")).count() as u32;
    let state = if server {
        if text.contains("*** SERVER STARTED") { "started" } else { "starting" }
    } else if text.contains("game loading took") {
        "in_game"
    } else if text.contains("ConnectToServerState: Start") {
        "connecting"
    } else {
        "menu"
    };
    (state.to_string(), errors)
}

/// The last line of a log that says something, without its `LOG  : General      f:0 st:...>` prefix.
pub fn last_step(text: &str) -> Option<String> {
    let line = text.lines().rev().map(str::trim).find(|l| !l.is_empty())?;
    let body = line.split_once("> ").map(|(_, b)| b).unwrap_or(line).trim();
    let body: String = body.chars().take(140).collect();
    (!body.is_empty()).then_some(body)
}

fn read_text(path: &Path) -> String {
    std::fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

/// Where the test's phase is kept between the start and the dashboard's polls.
fn phase_file(primary: &Path) -> PathBuf {
    primary.join("PZModStudio_MpTest.json")
}

fn set_phase(primary: &Path, phase: &str) {
    let _ = std::fs::write(phase_file(primary), serde_json::json!({ "phase": phase }).to_string());
}

fn read_phase(primary: &Path) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(phase_file(primary)).ok()?).ok()?;
    v.get("phase").and_then(|p| p.as_str()).map(str::to_string)
}

/// The test as it is now: the server and every game on this PC, with memory, CPU, folder, log,
/// state and errors, and the PC's free memory.
pub fn status(primary: &Path, pz_install_dir: &Path) -> MpStatus {
    let snap = snapshot();
    let game_xmx = std::fs::read_to_string(pz_install_dir.join("ProjectZomboid64.json")).ok().and_then(|t| xmx_mb(&t));
    let second = second_dir(primary);
    let mut processes = Vec::new();
    for p in &snap.procs {
        let cmd = p.cmd.clone().unwrap_or_default();
        let is_server = p.name.eq_ignore_ascii_case("java.exe") && (cmd.contains("-servername") || cmd.contains("GameServer"));
        let is_game = p.name.eq_ignore_ascii_case("ProjectZomboid64.exe");
        if !is_server && !is_game {
            continue;
        }
        let dir = arg_value(&cmd, "-cachedir").map(PathBuf::from).unwrap_or_else(|| primary.to_path_buf());
        let log = if is_server { dir.join("server-console.txt") } else { dir.join("console.txt") };
        let text = read_text(&log);
        let (state, errors) = read_log_state(&text, is_server);
        let step = last_step(&text);
        let same_dir = |a: &Path, b: &Path| a.to_string_lossy().replace('/', "\\").trim_end_matches('\\').eq_ignore_ascii_case(b.to_string_lossy().replace('/', "\\").trim_end_matches('\\'));
        let name = if is_server {
            arg_value(&cmd, "-servername").unwrap_or_else(|| "?".into())
        } else if same_dir(&dir, &second) {
            "2".into()
        } else {
            "1".into()
        };
        let connect = split_args(&cmd).windows(2).find(|w| w[0] == "+connect").map(|w| w[1].clone());
        processes.push(MpProcess {
            role: if is_server { "server" } else { "player" }.to_string(),
            pid: p.id,
            name,
            zomboid_dir: dir.to_string_lossy().to_string(),
            log_path: log.to_string_lossy().to_string(),
            ram_mb: p.ram / (1024 * 1024),
            ram_max_mb: if is_server { xmx_mb(&cmd) } else { game_xmx },
            cpu_percent: p.cpu,
            state,
            errors,
            connect,
            step,
            uptime_s: p.up,
            screen: if is_server { None } else { read_game_state(&dir) },
        });
    }
    processes.sort_by(|a, b| (a.role != "server", a.name.clone()).cmp(&(b.role != "server", b.name.clone())));
    MpStatus {
        pc_total_mb: snap.total_kb / 1024,
        pc_free_mb: snap.free_kb / 1024,
        processes,
        second_dir: second.to_string_lossy().to_string(),
        second_dir_ready: second.join("mods").is_dir(),
        phase: read_phase(primary),
        game_heap_mb: game_xmx,
    }
}

/// The memory the test asks for, in MB: the server's heap and each game's.
pub fn memory_needed(server_gb: u32, players: u32, game_xmx_mb: u64) -> u64 {
    server_gb as u64 * 1024 + players as u64 * game_xmx_mb
}

/// Leaves the account for the Studio Link mod in this game's own folder
/// (`Lua/PZModStudio_MpLogin.txt`): once the connect window shows this address, the Link fills
/// it in, connects, and empties the file. (The game's own way, its saved server list, never fills
/// the window with +connect: it compares the saved port, a number, with the given one, text.)
pub fn write_login(zomboid: &Path, port: u16, username: &str, password: &str) -> Result<(), String> {
    let dir = zomboid.join("Lua");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let text = format!("ip=127.0.0.1\nport={}\nuser={}\npassword={}\nconnect=1\n", port, username, password);
    std::fs::write(dir.join("PZModStudio_MpLogin.txt"), text).map_err(|e| format!("Cannot write the login file: {}", e))
}

/// The account each player of the test signs in with: player 1 the server's admin (its admin
/// password, as the Studio keeps it), player 2 a player of its own whose password the Studio
/// makes once and keeps in Windows Credential Manager. None when it cannot be known.
pub fn test_account(server: &str, slot: u32, admin_password: Option<&str>) -> Option<(String, String)> {
    if let Some(p) = admin_password.filter(|p| slot == 1 && !p.is_empty()) {
        return Some(("admin".to_string(), p.to_string()));
    }
    let key = format!("mp_test_password:{}:player{}", server, slot);
    let stored = crate::secrets::get(&key).ok().flatten().filter(|p| !p.is_empty());
    let password = match stored {
        Some(p) => p,
        None => {
            let p: String = {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) ^ (std::process::id() as u128) << 32;
                let alphabet = b"abcdefghjkmnpqrstuvwxyz23456789";
                let mut x = seed;
                (0..12)
                    .map(|_| {
                        x ^= x << 13;
                        x ^= x >> 7;
                        x ^= x << 17;
                        alphabet[(x % alphabet.len() as u128) as usize] as char
                    })
                    .collect()
            };
            crate::secrets::set(&key, &p).ok()?;
            p
        }
    };
    Some((format!("jugador{}", slot), password))
}

/// Launches one game joined to `address` without Steam; player 2 with its own Zomboid folder.
pub fn launch_player(pz_install_dir: &Path, primary: &Path, slot: u32, address: &str) -> Result<u32, String> {
    let exe = pz_install_dir.join("ProjectZomboid64.exe");
    if !exe.is_file() {
        return Err(format!("ProjectZomboid64.exe not found at {}", exe.display()));
    }
    let dir = if slot == 2 { second_dir(primary) } else { primary.to_path_buf() };
    crate::sandbox::configure_options_ini(&dir.to_string_lossy(), true);
    let mut cmd = std::process::Command::new(&exe);
    cmd.current_dir(pz_install_dir);
    if slot == 2 {
        cmd.arg(format!("-cachedir={}", dir.to_string_lossy().replace('\\', "/")));
    }
    cmd.args(["-nosteam", "+connect", address, "-modfolders", "mods,workshop,steam"]);
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    let child = cmd.spawn().map_err(|e| format!("Could not start the game: {}", e))?;
    Ok(child.id())
}

/// What the test does by itself, as the player ticked it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MpAutomation {
    /// Fill in the account on the connect window and press Connect (the Studio Link).
    pub login: bool,
    /// Click the black "click to start" screen once the world has loaded (the app: Lua does not
    /// run there).
    pub start_click: bool,
    /// A new character: "random", "first" (what each screen shows first) or "manual".
    pub character: String,
    /// Take the first spawn place the server offers (the Studio Link presses Next there).
    #[serde(default = "yes")]
    pub spawn: bool,
    /// Every player of the test an admin (player 1 signs in as admin already; player 2 is given
    /// the access level over RCON once its account exists).
    #[serde(default = "yes")]
    pub admin_all: bool,
}

fn yes() -> bool {
    true
}

impl Default for MpAutomation {
    fn default() -> Self {
        MpAutomation { login: true, start_click: true, character: "random".to_string(), spawn: true, admin_all: true }
    }
}

/// Where a game of the test is, as its Studio Link last said (`Lua/PZModStudio_MpState.txt`).
pub fn read_game_state(zomboid: &Path) -> Option<String> {
    let text = std::fs::read_to_string(zomboid.join("Lua").join("PZModStudio_MpState.txt")).ok()?;
    text.lines().find_map(|l| l.trim().strip_prefix("state=").map(|v| v.trim().to_string())).filter(|v| !v.is_empty())
}

/// The choices for the Studio Link inside this game (`Lua/PZModStudio_MpAuto.txt`), and a fresh
/// state file so an old "in_game" is not read as this launch's.
fn write_automation(zomboid: &Path, auto: &MpAutomation, name: &str) -> Result<(), String> {
    let dir = zomboid.join("Lua");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let character = match auto.character.as_str() {
        "random" | "first" => auto.character.as_str(),
        _ => "manual",
    };
    std::fs::write(dir.join("PZModStudio_MpAuto.txt"), format!("character={}\nspawn={}\nname={}\n", character, if auto.spawn { 1 } else { 0 }, name)).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("PZModStudio_MpState.txt"), "state=launching\n").map_err(|e| e.to_string())
}

/// The Studio Link in the server's `Mods=` (after it connects a game runs the server's mods, and
/// the Link is what builds the character there), with a backup of the .ini. Returns whether it
/// had to be added.
pub fn ensure_link_on_server(ini: &Path) -> Result<bool, String> {
    let text = std::fs::read_to_string(ini).map_err(|e| format!("Cannot read {}: {}", ini.display(), e))?;
    if super::mod_sync::ini_list(&text, "Mods").iter().any(|m| m == crate::studio_link::LINK_MOD_ID) {
        return Ok(false);
    }
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut found = false;
    let lines: Vec<String> = text
        .lines()
        .map(|l| match l.strip_prefix("Mods=") {
            Some(v) if !found => {
                found = true;
                let backslash = v.trim_start().starts_with('\\');
                let id = if backslash { format!("\\{}", crate::studio_link::LINK_MOD_ID) } else { crate::studio_link::LINK_MOD_ID.to_string() };
                if v.trim().is_empty() { format!("Mods={}", id) } else { format!("Mods={};{}", v.trim_end_matches(';'), id) }
            }
            _ => l.to_string(),
        })
        .collect();
    if !found {
        return Err(format!("{} has no Mods= line.", ini.display()));
    }
    let _ = std::fs::copy(ini, ini.with_extension("ini.bak-before-studio-link"));
    std::fs::write(ini, lines.join(eol) + eol).map_err(|e| format!("Cannot write {}: {}", ini.display(), e))?;
    Ok(true)
}

/// One game of the test, as the watcher follows it.
struct Watched {
    slot: u32,
    pid: u32,
    dir: PathBuf,
    clicks: u32,
    /// The account it signs in with, and whether it was made an admin yet.
    user: Option<String>,
    made_admin: bool,
}

/// The test as it was started, kept beside its phase so one game can be started again on its own.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestSettings {
    server: String,
    port: u16,
    automation: MpAutomation,
}

fn settings_file(primary: &Path) -> PathBuf {
    primary.join("PZModStudio_MpTestSettings.json")
}

/// Leaves one game's choices and account in its folder and starts it; the game to watch.
fn launch_one(pz_install_dir: &Path, primary: &Path, settings: &TestSettings, slot: u32, admin_password: Option<&str>) -> Result<Watched, String> {
    let dir = if slot == 2 { second_dir(primary) } else { primary.to_path_buf() };
    let account = test_account(&settings.server, slot, admin_password);
    let surname = account.as_ref().map(|(u, _)| u.clone()).unwrap_or_else(|| format!("Player{}", slot));
    let _ = write_automation(&dir, &settings.automation, &surname);
    if settings.automation.login {
        // The account the Studio Link fills in and connects with, for this player.
        if let Some((user, password)) = &account {
            let _ = write_login(&dir, settings.port, user, password);
        }
    }
    let game_pid = launch_player(pz_install_dir, primary, slot, &format!("127.0.0.1:{}", settings.port))?;
    Ok(Watched { slot, pid: game_pid, dir, clicks: 0, user: account.map(|(u, _)| u), made_admin: false })
}

/// The watcher: every two seconds, up to half an hour, until each watched game is in the world.
/// Their logs say where they are, their Studio Link says which screen; a game past the connect
/// window is made an admin when asked; a game whose world loaded and waits on the black "click to
/// start" screen gets a click in its own window.
fn watch_games(primary: PathBuf, settings: TestSettings, mut watched: Vec<Watched>) {
    let auto = &settings.automation;
    set_phase(&primary, "watching");
    let watch_until = std::time::Instant::now() + std::time::Duration::from_secs(1800);
    while std::time::Instant::now() < watch_until {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if read_phase(&primary).as_deref() == Some("stopped") {
            return;
        }
        let mut all_in = true;
        // A game that is gone before it got in: the test says so (and why, when its log does)
        // instead of counting it as arrived.
        let mut gone: Option<String> = None;
        for w in watched.iter_mut() {
            if !super::server_pid_alive(w.pid) {
                if read_game_state(&w.dir).as_deref() != Some("in_game") || closed_by_error(&w.dir) {
                    let why = if read_text(&w.dir.join("console.txt")).contains("OutOfMemoryError") { "out_of_memory" } else { "closed" };
                    gone.get_or_insert(format!("player_{}_{}", w.slot, why));
                }
                continue;
            }
            let state = read_game_state(&w.dir);
            // Past the connect window the account exists on the server: an admin, when asked
            // (player 1 is the admin account already). Tried until the server says yes.
            let signed_in = matches!(state.as_deref(), Some("spawn_select" | "character_profession" | "character_look" | "in_game"));
            if auto.admin_all && w.slot > 1 && signed_in && !w.made_admin {
                if let Some(user) = &w.user {
                    w.made_admin = super::rcon_command(&primary, &settings.server, &format!("setaccesslevel \"{}\" admin", user)).is_ok();
                }
            }
            if state.as_deref() == Some("in_game") {
                continue;
            }
            all_in = false;
            let loaded = read_text(&w.dir.join("console.txt")).contains("game loading took");
            // The world is loaded and the Link has not said "in game": the black screen.
            if auto.start_click && loaded && w.clicks < 40 {
                if crate::claude_agent::click_window_of_pid(w.pid, 0.5, 0.5).is_ok() {
                    w.clicks += 1;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        if all_in {
            set_phase(&primary, gone.as_deref().unwrap_or("all_in_game"));
            return;
        }
    }
    set_phase(&primary, "watch_ended");
}

/// Whether a game's log ends on an error that closed it (the Link's last word may still be
/// "in game" when the game dies in the world).
fn closed_by_error(dir: &Path) -> bool {
    let log = read_text(&dir.join("console.txt"));
    let start = (log.len().saturating_sub(4000)..log.len()).find(|i| log.is_char_boundary(*i)).unwrap_or(0);
    let tail = &log[start..];
    tail.contains("OutOfMemoryError") || tail.contains("uncaughtException")
}

/// Starts the test: the server without Steam, then, once its log says SERVER STARTED, the games
/// (15 s apart), watched until each is in the world (`watch_games`). The phase and what each game
/// is doing are kept for `status`.
pub fn start(pz_install_dir: PathBuf, primary: PathBuf, server: String, memory_gb: u32, players: u32, admin_password: Option<String>, auto: MpAutomation) -> Result<u32, String> {
    let ini = primary.join("Server").join(format!("{}.ini", server));
    let port = std::fs::read_to_string(&ini)
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.trim().strip_prefix("DefaultPort=").and_then(|v| v.trim().parse::<u16>().ok())))
        .unwrap_or(16261);
    // The Link has to be where a NoSteam server and game read it, and in the server's list for
    // what it does after connecting.
    crate::studio_link::install_link_files_at(&primary)?;
    if auto.character != "manual" || auto.spawn {
        ensure_link_on_server(&ini)?;
    }
    let settings = TestSettings { server: server.clone(), port, automation: auto };
    let _ = std::fs::write(settings_file(&primary), serde_json::to_string_pretty(&settings).unwrap_or_default());
    let admin_for_player = admin_password.clone();
    let pid = super::launch_dedicated_server(
        pz_install_dir.to_string_lossy().to_string(),
        primary.to_string_lossy().to_string(),
        server,
        Some(memory_gb),
        Some(true),
        admin_password,
    )?;
    set_phase(&primary, "waiting_for_server");
    let started_at = std::time::Instant::now();
    std::thread::spawn(move || {
        let log = primary.join("server-console.txt");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3));
            if read_text(&log).contains("*** SERVER STARTED") {
                break;
            }
            if started_at.elapsed().as_secs() > 600 || !super::server_pid_alive(pid) {
                set_phase(&primary, "server_did_not_start");
                return;
            }
        }
        let mut watched: Vec<Watched> = Vec::new();
        for slot in 1..=players.clamp(1, 2) {
            if slot > 1 {
                std::thread::sleep(std::time::Duration::from_secs(15));
            }
            match launch_one(&pz_install_dir, &primary, &settings, slot, admin_for_player.as_deref()) {
                Ok(w) => watched.push(w),
                Err(e) => {
                    set_phase(&primary, &format!("player_{}_failed: {}", slot, e));
                    return;
                }
            }
            if slot == 1 {
                // Every game window maximized while it boots, never focus taken from the player.
                crate::claude_agent::start_launch_window_guard(crate::claude_agent::current_foreground_window(), false, false, true);
            }
        }
        watch_games(primary, settings, watched);
    });
    Ok(pid)
}

/// Starts one game of the running test again (closed first when it is open), with the same
/// account and choices, and watches it in: the server and the other game keep running.
pub fn restart_player(pz_install_dir: PathBuf, primary: PathBuf, slot: u32, admin_password: Option<String>) -> Result<u32, String> {
    if slot != 1 && slot != 2 {
        return Err("Player 1 or 2.".to_string());
    }
    let settings: TestSettings = std::fs::read_to_string(settings_file(&primary))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .ok_or("No multiplayer test was started from this app: start one first.")?;
    let st = status(&primary, &pz_install_dir);
    if !st.processes.iter().any(|p| p.role == "server") {
        return Err("The test's server is not running: start the test again.".to_string());
    }
    for p in st.processes.iter().filter(|p| p.role == "player" && p.name == slot.to_string()) {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("taskkill").args(["/PID", &p.pid.to_string(), "/F"]).creation_flags(0x08000000).output();
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    let w = launch_one(&pz_install_dir, &primary, &settings, slot, admin_password.as_deref())?;
    let pid = w.pid;
    crate::claude_agent::start_launch_window_guard(crate::claude_agent::current_foreground_window(), false, false, true);
    std::thread::spawn(move || watch_games(primary, settings, vec![w]));
    Ok(pid)
}

/// Ends the test: the server quits over RCON (it saves) and the games are closed.
pub fn stop(primary: &Path) -> Result<Vec<u32>, String> {
    let st = status(primary, Path::new(&crate::vfs::trusted_install_dir()));
    let mut closed = Vec::new();
    for p in st.processes.iter().filter(|p| p.role == "player") {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("taskkill").args(["/PID", &p.pid.to_string(), "/F"]).creation_flags(0x08000000).output();
        }
        closed.push(p.pid);
    }
    let _ = super::stop_dedicated_server(primary.to_string_lossy().to_string(), None);
    // The Link's choices go with the test: player 1 plays in the player's own folder.
    crate::sandbox::forget_mp_test_automation(&primary.to_string_lossy());
    crate::sandbox::forget_mp_test_automation(&second_dir(primary).to_string_lossy());
    set_phase(primary, "stopped");
    Ok(closed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_login_is_left_where_the_link_reads_it() {
        let dir = std::env::temp_dir().join(format!("pzms-mplogin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_login(&dir, 16261, "jugador2", "pw").unwrap();
        let text = std::fs::read_to_string(dir.join("Lua").join("PZModStudio_MpLogin.txt")).unwrap();
        assert_eq!(text, "ip=127.0.0.1\nport=16261\nuser=jugador2\npassword=pw\nconnect=1\n");
        let _ = std::fs::remove_dir_all(&dir);
        // Player 1 is the admin when the admin password is known.
        assert_eq!(test_account("x", 1, Some("adm")), Some(("admin".to_string(), "adm".to_string())));
    }

    #[test]
    fn the_link_joins_the_servers_list_once_and_the_games_say_where_they_are() {
        let dir = std::env::temp_dir().join(format!("pzms-mpauto-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ini = dir.join("t.ini");
        std::fs::write(&ini, "PVP=true\r\nMods=\\A;\\B\r\nMap=Muldraugh, KY\r\n").unwrap();
        assert!(ensure_link_on_server(&ini).unwrap());
        assert!(!ensure_link_on_server(&ini).unwrap(), "only once");
        let text = std::fs::read_to_string(&ini).unwrap();
        assert!(text.contains("Mods=\\A;\\B;\\Z_StudioLink\r\n") && text.contains("PVP=true\r\n"), "{}", text);
        assert!(dir.join("t.ini.bak-before-studio-link").is_file());

        let auto = MpAutomation { login: true, start_click: true, character: "first".into(), spawn: true, admin_all: true };
        write_automation(&dir, &auto, "jugador2").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("Lua").join("PZModStudio_MpAuto.txt")).unwrap(), "character=first\nspawn=1\nname=jugador2\n");
        assert_eq!(read_game_state(&dir).as_deref(), Some("launching"));
        std::fs::write(dir.join("Lua").join("PZModStudio_MpState.txt"), "state=in_game\n").unwrap();
        assert_eq!(read_game_state(&dir).as_deref(), Some("in_game"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_second_folder_sits_beside_the_first() {
        assert_eq!(second_dir(Path::new(r"C:\Users\x\Zomboid")), PathBuf::from(r"C:\Users\x\Zomboid2"));
    }

    #[test]
    fn command_lines_and_logs_are_read() {
        let cmd = r#""D:\PZ\ProjectZomboid64.exe" -nosteam "-cachedir=C:/Users/x/Zomboid2" +connect 127.0.0.1:16261 -modfolders mods,workshop,steam"#;
        assert_eq!(arg_value(cmd, "-cachedir").as_deref(), Some("C:/Users/x/Zomboid2"));
        assert_eq!(split_args(cmd).windows(2).find(|w| w[0] == "+connect").map(|w| w[1].as_str()), Some("127.0.0.1:16261"));
        let server = r#"java.exe -Xmx4g -cp ./;projectzomboid.jar zombie.network.GameServer -servername knoxtest -cachedir=C:\Users\x\Zomboid -nosteam"#;
        assert_eq!(arg_value(server, "-servername").as_deref(), Some("knoxtest"));
        assert_eq!(xmx_mb(server), Some(4096));
        assert_eq!(xmx_mb(r#""-Xmx6144m","#), Some(6144));
        assert_eq!(read_log_state("LOG x\nConnectToServerState: Start\nERROR: General\n", false), ("connecting".to_string(), 1));
        assert_eq!(read_log_state("ConnectToServerState: Start\ngame loading took 92 seconds\n", false).0, "in_game");
        assert_eq!(read_log_state("LOG\n", false).0, "menu");
        assert_eq!(read_log_state("*** SERVER STARTED ****\n", true).0, "started");
        assert_eq!(memory_needed(4, 2, 6144), 4096 + 12288);
        assert_eq!(last_step("LOG  : General      f:0> Loading Tiles2x texture pack\n\n").as_deref(), Some("Loading Tiles2x texture pack"));
        assert_eq!(last_step(""), None);
    }

    #[test]
    fn the_second_folder_gets_the_servers_mods() {
        let base = std::env::temp_dir().join(format!("pzms-mptest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let primary = base.join("Zomboid");
        let workshop = base.join("108600");
        let write = |root: &Path, info: &str| {
            std::fs::create_dir_all(root.join("42")).unwrap();
            std::fs::write(root.join("42").join("mod.info"), info).unwrap();
            std::fs::write(root.join("mod.info"), info).unwrap();
        };
        write(&workshop.join("111").join("mods").join("Wsh"), "id=Wsh\nname=Wsh\n");
        write(&primary.join("mods").join("Loc"), "id=Loc\nname=Loc\n");
        std::fs::create_dir_all(primary.join("Server")).unwrap();
        std::fs::write(primary.join("Server").join("t.ini"), "Mods=\\Wsh;\\Loc;\\Nowhere\nDefaultPort=16261\n").unwrap();
        std::fs::write(primary.join("options.ini"), "fullscreen=false\n").unwrap();
        let catalog = crate::catalog::build_catalog(&workshop.to_string_lossy(), &primary.to_string_lossy(), None);

        let r = prepare_second(&catalog, &primary, "t").unwrap();
        assert_eq!(r.copied, vec!["Wsh".to_string(), "Loc".to_string()]);
        assert_eq!(r.missing, vec!["Nowhere".to_string()]);
        let second = base.join("Zomboid2");
        assert!(second.join("mods").join("Wsh").join("42").join("mod.info").is_file());
        assert!(second.join("mods").join("Loc").join(java_mods::COPY_MARKER).is_file());
        assert!(second.join("options.ini").is_file());
        // Again: nothing to do.
        let again = prepare_second(&catalog, &primary, "t").unwrap();
        assert!(again.copied.is_empty() && again.kept.len() == 2, "{:?}", again);
        let _ = std::fs::remove_dir_all(&base);
    }
}
