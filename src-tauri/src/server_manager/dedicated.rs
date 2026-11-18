//! The official "Project Zomboid Dedicated Server" program (Steam app 380870), installed and kept
//! up to date with Valve's SteamCMD in a folder the player picks, and how a server is started from
//! it instead of from the game's own folder.
//!
//! Nothing here downloads on its own: SteamCMD is fetched from Valve's address only when the
//! player presses the button (or an assistant passes `allow_download`), its signature is checked
//! before it runs, and the server program comes from Steam through it. The game's folder stays the
//! default way to start a server.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};

/// Steam's app id of the dedicated server program (anonymous login is enough for it).
pub const APP_ID: u32 = 380870;
/// Valve's own address for SteamCMD on Windows.
pub const STEAMCMD_URL: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";
/// The zip is about 3 MB; anything far bigger is not it.
const MAX_ZIP_BYTES: u64 = 64 * 1024 * 1024;
/// Where SteamCMD puts the program when the folder is not chosen.
pub const DEFAULT_FOLDER_NAME: &str = "Project Zomboid Dedicated Server";

/// Which program starts the Studio's servers, and where the dedicated one lives.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct DedicatedConfig {
    /// "game" (default: the game's own java.exe and files) or "dedicated".
    pub source: String,
    /// The dedicated server's folder.
    pub dir: String,
    /// The Steam branch: "public", "unstable" (Build 42) or another branch name.
    pub branch: String,
    /// steamcmd.exe the player pointed at; empty for the Studio's own copy.
    pub steamcmd: String,
}

impl DedicatedConfig {
    pub fn uses_dedicated(&self) -> bool {
        self.source == "dedicated"
    }
}

fn config_file() -> Option<PathBuf> {
    if cfg!(test) {
        // Only a live test (run by hand against a real install) names the real file.
        return std::env::var_os("PZMS_LIVE_DEDICATED_CONFIG").map(PathBuf::from);
    }
    dirs_next::config_dir().map(|d| d.join("PZModStudio").join("dedicated_server.json"))
}

pub fn load_config() -> DedicatedConfig {
    config_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &DedicatedConfig) -> Result<(), String> {
    let Some(file) = config_file() else { return Ok(()) };
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(cfg).unwrap_or_default()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &file).map_err(|e| e.to_string())
}

/// A branch name SteamCMD takes as one word (and nothing a command line could read as more).
pub fn valid_branch(branch: &str) -> bool {
    let b = branch.trim();
    !b.is_empty() && b.len() <= 64 && b.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// SteamCMD's arguments to install or update the dedicated server in `dir` on `branch`.
/// `-beta public` is said too: without it SteamCMD stays on the last beta the folder had.
pub fn steamcmd_args(dir: &Path, branch: &str, validate: bool) -> Vec<String> {
    let folder = dir.to_string_lossy().trim_end_matches(['\\', '/']).to_string();
    let mut args = vec![
        "+force_install_dir".to_string(),
        folder,
        "+login".to_string(),
        "anonymous".to_string(),
        "+app_update".to_string(),
        APP_ID.to_string(),
    ];
    let branch = branch.trim();
    args.push("-beta".to_string());
    args.push(if branch.is_empty() { "public".to_string() } else { branch.to_string() });
    if validate {
        args.push("validate".to_string());
    }
    args.push("+quit".to_string());
    args
}

/// Whether SteamCMD should check every file: for a folder without a complete install (a first
/// install, or one cut short), or when its record says so (it then checks them anyway). An update of a complete one only fetches what changed, in seconds;
/// checking its ~7 GB takes many minutes.
pub fn validate_by_default(dir: &Path) -> bool {
    !inspect(dir).is_some_and(|i| i.ready && !i.incomplete && !i.check_due)
}

/// The Steam branch the installed game is on ("public" when it is on none), which a server has to
/// match for that game to join it. None when the game is not a Steam install.
pub fn game_branch(game_dir: &Path) -> Option<String> {
    let game = crate::catalog::read_game_install(&game_dir.to_string_lossy())?;
    Some(if game.branch == "stable" { "public".to_string() } else { game.branch })
}

/// The command as a person would type it, for the screen.
pub fn command_line(steamcmd: &Path, args: &[String]) -> String {
    let quote = |s: &str| if s.contains(' ') { format!("\"{}\"", s) } else { s.to_string() };
    std::iter::once(quote(&steamcmd.to_string_lossy())).chain(args.iter().map(|a| quote(a))).collect::<Vec<_>>().join(" ")
}

// ------------------------------------------------------------------ what is on this PC

/// The Studio's own SteamCMD folder (in the local app data), where the button downloads it.
pub fn studio_steamcmd_dir() -> PathBuf {
    dirs_next::data_local_dir().unwrap_or_else(std::env::temp_dir).join("PZModStudio").join("steamcmd")
}

/// SteamCMD's program: steamcmd.exe on Windows; on Linux its steamcmd.sh (Valve's tarball) or the
/// `steamcmd` of the distribution's package.
pub const STEAMCMD_EXE: &str = if cfg!(windows) { "steamcmd.exe" } else { "steamcmd.sh" };

/// Whether a file the player points at is SteamCMD.
fn is_steamcmd_name(name: &str) -> bool {
    if cfg!(windows) {
        name.eq_ignore_ascii_case("steamcmd.exe")
    } else {
        name == "steamcmd.sh" || name == "steamcmd"
    }
}

/// Places steamcmd.exe usually is, the player's choice first.
pub fn steamcmd_candidates(cfg: &DedicatedConfig) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !cfg.steamcmd.trim().is_empty() {
        out.push(PathBuf::from(cfg.steamcmd.trim()));
    }
    out.push(studio_steamcmd_dir().join(STEAMCMD_EXE));
    // Linux: Valve's tarball unpacked in the usual places, or the distribution's package.
    #[cfg(not(windows))]
    {
        if let Some(home) = dirs_next::home_dir() {
            for folder in ["steamcmd", "Steam", ".steam/steamcmd", ".local/share/Steam/steamcmd"] {
                out.push(home.join(folder).join("steamcmd.sh"));
            }
        }
        for p in ["/usr/games/steamcmd", "/usr/bin/steamcmd", "/usr/local/bin/steamcmd", "/opt/steamcmd/steamcmd.sh"] {
            out.push(PathBuf::from(p));
        }
    }
    #[cfg(windows)]
    {
        for drive in ["C", "D", "E", "F"] {
            for folder in ["steamcmd", "SteamCMD", "Program Files\\steamcmd", "Program Files (x86)\\steamcmd", "Apps\\steamcmd", "Games\\steamcmd"] {
                out.push(PathBuf::from(format!("{}:\\{}\\steamcmd.exe", drive, folder)));
            }
        }
        if let Some(home) = dirs_next::home_dir() {
            out.push(home.join("steamcmd").join("steamcmd.exe"));
        }
    }
    out
}

pub fn find_steamcmd(cfg: &DedicatedConfig) -> Option<PathBuf> {
    steamcmd_candidates(cfg).into_iter().find(|p| p.is_file())
}

/// What a folder holds of the dedicated server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DedicatedInstall {
    pub dir: String,
    /// java.exe and the server's files are there: a server can start from it.
    pub ready: bool,
    /// From SteamCMD's record (`steamapps/appmanifest_380870.acf`).
    pub build_id: Option<String>,
    /// The branch it is on ("public" when the record names none).
    pub branch: Option<String>,
    pub size_on_disk: Option<u64>,
    /// Unix seconds of its last update.
    pub last_updated: Option<u64>,
    /// SteamCMD stopped half way (the record says so, or files are still being downloaded).
    pub incomplete: bool,
    /// The game build its projectzomboid.jar starts as ("42.22"): a client joins only the same.
    pub version: Option<String>,
    /// SteamCMD's record says the next update checks every file first (a check was stopped).
    pub check_due: bool,
}

/// Reads one folder; None when it holds nothing of the dedicated server.
pub fn inspect(dir: &Path) -> Option<DedicatedInstall> {
    let java = dir.join("jre64").join("bin").join(crate::platform::JAVA_EXE);
    let has_server_files = dir.join("StartServer64.bat").is_file()
        || dir.join("start-server.sh").is_file()
        || dir.join("projectzomboid.jar").is_file()
        || dir.join("java").join("projectzomboid.jar").is_file()
        || dir.join("java").join("zombie").is_dir();
    let name = format!("appmanifest_{}.acf", APP_ID);
    // SteamCMD keeps its record inside the folder; the Steam client in the library around it.
    let library_record = dir.parent().filter(|p| p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("common"))).and_then(Path::parent).map(|steamapps| steamapps.join(&name));
    let manifest = std::fs::read_to_string(dir.join("steamapps").join(&name)).ok().or_else(|| library_record.filter(|_| has_server_files).and_then(|p| std::fs::read_to_string(p).ok()));
    if manifest.is_none() && !has_server_files {
        return None;
    }
    let mut out = DedicatedInstall { dir: dir.to_string_lossy().to_string(), ready: java.is_file() && has_server_files, ..Default::default() };
    if let Some(text) = manifest {
        let (build, branch, size, updated, state_flags) = read_manifest(&text);
        out.build_id = build;
        out.branch = branch;
        out.size_on_disk = size;
        out.last_updated = updated;
        // StateFlags 4 is "fully installed"; other bits mean an update is due or under way.
        out.incomplete = state_flags.is_some_and(|f| f != 4);
        out.check_due = crate::vdf::parse(&text).get("AppState").and_then(|a| a.u64("FullValidateBeforeNextUpdate")) == Some(1);
    }
    // SteamCMD leaves this folder behind empty after a check of the files is stopped: only files
    // in it are a download still to finish.
    let downloading = dir.join("steamapps").join("downloading").join(APP_ID.to_string());
    if walkdir::WalkDir::new(&downloading).min_depth(1).into_iter().flatten().any(|e| e.file_type().is_file()) {
        out.incomplete = true;
    }
    if out.ready && !out.incomplete {
        out.version = ["projectzomboid.jar", "java/projectzomboid.jar"]
            .iter()
            .find_map(|j| crate::load_order::mod_info::jar_game_version(&dir.join(j)))
            .map(crate::catalog::modinfo::format_game_version);
    }
    Some(out)
}

/// buildid, branch, SizeOnDisk, LastUpdated and StateFlags of an `appmanifest_380870.acf`.
pub fn read_manifest(text: &str) -> (Option<String>, Option<String>, Option<u64>, Option<u64>, Option<u64>) {
    let v = crate::vdf::parse(text);
    let Some(app) = v.get("AppState") else { return (None, None, None, None, None) };
    let branch = app
        .get("UserConfig")
        .and_then(|u| u.str("BetaKey"))
        .or_else(|| app.get("MountedConfig").and_then(|u| u.str("BetaKey")))
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .unwrap_or("public")
        .to_string();
    (app.str("buildid").map(str::to_string), Some(branch), app.u64("SizeOnDisk"), app.u64("LastUpdated"), app.u64("StateFlags"))
}

/// Dedicated servers already on this PC: the chosen folder, the Steam libraries' tools folder
/// (the Steam client installs it there from Library > Tools), and beside a SteamCMD.
pub fn find_installs(cfg: &DedicatedConfig) -> Vec<DedicatedInstall> {
    let mut places: Vec<PathBuf> = Vec::new();
    if !cfg.dir.trim().is_empty() {
        places.push(PathBuf::from(cfg.dir.trim()));
    }
    for lib in crate::vfs::steam_library_dirs() {
        places.push(lib.join("steamapps").join("common").join(DEFAULT_FOLDER_NAME));
    }
    for steamcmd in steamcmd_candidates(cfg).into_iter().filter(|p| p.is_file()) {
        if let Some(parent) = steamcmd.parent() {
            places.push(parent.join("steamapps").join("common").join(DEFAULT_FOLDER_NAME));
        }
    }
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for p in places {
        let key = p.to_string_lossy().to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        if let Some(found) = inspect(&p) {
            out.push(found);
        }
    }
    out
}

// ------------------------------------------------------------------ starting a server from it

/// How java.exe is started for the server: from the folder's own start script when it has one.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub java: PathBuf,
    pub classpath: String,
    pub library_path: String,
}

/// The class path and library path the server's own `StartServer64.bat` uses (its `SET
/// PZ_CLASSPATH=` line and the java line's `-cp` and `-Djava.library.path=`).
pub fn parse_start_script(text: &str) -> (Option<String>, Option<String>) {
    let mut set_cp: Option<String> = None;
    let mut cp: Option<String> = None;
    let mut lib: Option<String> = None;
    for line in text.lines() {
        let l = line.trim();
        let lower = l.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("set pz_classpath=") {
            let start = l.len() - rest.len();
            set_cp = Some(l[start..].trim().to_string());
            continue;
        }
        if !lower.contains("java.exe") {
            continue;
        }
        let tokens: Vec<&str> = l.split_whitespace().collect();
        for (i, t) in tokens.iter().enumerate() {
            if *t == "-cp" || *t == "-classpath" {
                cp = tokens.get(i + 1).map(|s| s.trim_matches('"').to_string());
            } else if let Some(v) = t.strip_prefix("-Djava.library.path=") {
                lib = Some(v.trim_matches('"').to_string());
            }
        }
    }
    let cp = cp.map(|c| if c.eq_ignore_ascii_case("%PZ_CLASSPATH%") { set_cp.clone().unwrap_or_default() } else { c }).filter(|c| !c.is_empty()).or(set_cp);
    (cp, lib)
}

/// Linux: the class path and native library folders of a game or server folder, from its
/// `ProjectZomboid64.json` (what the ProjectZomboid64 launcher reads: `classpath` and the
/// `-Djava.library.path=` of `vmArgs`), else the folder's usual layout.
pub fn linux_java_paths(dir: &Path) -> (String, String) {
    let json: Option<serde_json::Value> = std::fs::read_to_string(dir.join("ProjectZomboid64.json")).ok().and_then(|t| serde_json::from_str(&t).ok());
    let cp = json
        .as_ref()
        .and_then(|j| j.get("classpath")?.as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(":")))
        .filter(|c| !c.is_empty());
    let lib = json.as_ref().and_then(|j| {
        j.get("vmArgs")?.as_array()?.iter().filter_map(|v| v.as_str()).find_map(|a| a.strip_prefix("-Djava.library.path=").map(str::to_string))
    });
    let cp = cp.unwrap_or_else(|| {
        if dir.join("projectzomboid.jar").is_file() {
            "./:projectzomboid.jar".to_string()
        } else if dir.join("java").join("projectzomboid.jar").is_file() {
            "java/:java/projectzomboid.jar".to_string()
        } else {
            "java/:java/*".to_string()
        }
    });
    (cp, lib.unwrap_or_else(|| "linux64/:natives/:.".to_string()))
}

/// How to start java.exe in a dedicated server folder, or None when it cannot start from there.
pub fn layout(dir: &Path) -> Option<Layout> {
    let java = dir.join("jre64").join("bin").join(crate::platform::JAVA_EXE);
    if !java.is_file() {
        return None;
    }
    if cfg!(windows) {
        layout_from_start_script(dir, java)
    } else {
        let has_server = dir.join("projectzomboid.jar").is_file() || dir.join("java").is_dir() || dir.join("ProjectZomboid64.json").is_file();
        let (classpath, library_path) = linux_java_paths(dir);
        has_server.then_some(Layout { java, classpath, library_path })
    }
}

/// Windows: from the folder's StartServer64.bat, else its usual layout.
fn layout_from_start_script(dir: &Path, java: PathBuf) -> Option<Layout> {
    let script = ["StartServer64.bat", "StartServer64_nosteam.bat", "ProjectZomboidServer.bat"]
        .iter()
        .find_map(|n| std::fs::read_to_string(dir.join(n)).ok())
        .unwrap_or_default();
    let (cp, lib) = parse_start_script(&script);
    let classpath = cp.or_else(|| {
        if dir.join("projectzomboid.jar").is_file() {
            Some("./;projectzomboid.jar".to_string())
        } else if dir.join("java").join("projectzomboid.jar").is_file() {
            Some("java/;java/projectzomboid.jar".to_string())
        } else if dir.join("java").is_dir() {
            Some("java/;java/*".to_string())
        } else {
            None
        }
    })?;
    Some(Layout { java, classpath, library_path: lib.unwrap_or_else(|| "natives/;natives/win64/;.".to_string()) })
}

/// The folder the Studio's server starts from: the dedicated server when chosen and ready there,
/// else None (the game's folder).
pub fn chosen_server_dir() -> Option<PathBuf> {
    let cfg = load_config();
    if !cfg.uses_dedicated() || cfg.dir.trim().is_empty() {
        return None;
    }
    let dir = PathBuf::from(cfg.dir.trim());
    layout(&dir).map(|_| dir)
}

// ------------------------------------------------------------------ SteamCMD's output

/// One line of SteamCMD's output, as what it says about the job.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineInfo {
    pub phase: String,
    pub percent: Option<f32>,
    pub done_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    /// Set for the line that ends the job: true when it worked.
    pub finished: Option<bool>,
}

fn number(text: &str) -> Option<u64> {
    let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// What a SteamCMD line says: its own update (`[ 40%] Downloading update (1,234 of 39,483 KB)`),
/// the login, the app's progress (`Update state (0x61) downloading, progress: 2.07 (61287133 /
/// 2959812457)`), and the end (`Success! App '380870' fully installed.`, `Error! App '380870'
/// state is 0x202 after update job.`).
pub fn parse_line(line: &str) -> Option<LineInfo> {
    let l = line.trim();
    if l.is_empty() {
        return None;
    }
    let lower = l.to_ascii_lowercase();
    if lower.starts_with("success!") {
        return Some(LineInfo { phase: "done".into(), percent: Some(100.0), finished: Some(true), ..Default::default() });
    }
    if lower.starts_with("error!") || lower.starts_with("failed to") || lower.contains("failed to install app") {
        return Some(LineInfo { phase: "error".into(), finished: Some(false), ..Default::default() });
    }
    if let Some(rest) = l.strip_prefix('[') {
        // SteamCMD updating itself.
        let (pct, text) = rest.split_once(']')?;
        let percent = pct.trim().trim_end_matches('%').trim().parse::<f32>().ok();
        let mut info = LineInfo { phase: "steamcmd_update".into(), percent, ..Default::default() };
        if let Some(open) = text.find('(') {
            let inner = &text[open + 1..text[open..].find(')').map(|c| open + c).unwrap_or(text.len())];
            if let Some((a, b)) = inner.split_once(" of ") {
                let kb = |s: &str| number(s).map(|n| n * 1024);
                info.done_bytes = kb(a);
                info.total_bytes = kb(b);
            }
        }
        return Some(info);
    }
    if lower.contains("update state") {
        let after = l.split_once(')').map(|(_, r)| r).unwrap_or(l);
        let word = after.split(',').next().unwrap_or("").trim().to_ascii_lowercase();
        let phase = if word.contains("download") {
            "downloading"
        } else if word.contains("verif") {
            "verifying"
        } else if word.contains("prealloc") {
            "preallocating"
        } else if word.contains("commit") {
            "committing"
        } else if word.contains("reconfig") {
            "reconfiguring"
        } else {
            "updating"
        };
        let mut info = LineInfo { phase: phase.into(), ..Default::default() };
        if let Some((_, p)) = l.split_once("progress:") {
            let p = p.trim();
            info.percent = p.split_whitespace().next().and_then(|n| n.parse::<f32>().ok());
            if let (Some(open), Some(close)) = (p.find('('), p.rfind(')')) {
                if let Some((a, b)) = p[open + 1..close].split_once('/') {
                    info.done_bytes = number(a);
                    info.total_bytes = number(b);
                }
            }
        }
        return Some(info);
    }
    if lower.contains("logging in") || lower.contains("connecting anonymously") || lower.contains("waiting for user info") {
        return Some(LineInfo { phase: "login".into(), ..Default::default() });
    }
    if lower.contains("loading steam api") || lower.contains("steam console client") {
        return Some(LineInfo { phase: "starting".into(), ..Default::default() });
    }
    None
}

/// A line of SteamCMD's own log (`logs/console_log.txt`) without the time in front of it
/// (`[2026-11-18 06:07:32] Waiting for user info...OK`).
pub fn strip_log_time(line: &str) -> &str {
    let l = line.trim_start();
    let b = l.as_bytes();
    if b.len() >= 21 && b[0] == b'[' && b[20] == b']' && l.get(1..5).is_some_and(|y| y.chars().all(|c| c.is_ascii_digit())) {
        l[21..].trim_start()
    } else {
        l
    }
}

/// The branch SteamCMD's content log says it could not use because Steam keeps it behind a
/// password (`WARNING! AppID 380870 has missing cached password for private beta "unstable" set,
/// falling back to public`). SteamCMD itself only prints "Missing configuration".
pub fn private_beta_in_log(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let at = lower.find("password for private beta \"")? + "password for private beta \"".len();
    let rest = &text[at..];
    rest.find('"').map(|end| rest[..end].to_string()).filter(|b| !b.is_empty())
}

/// The branch of `ERROR! Failed to set beta 'unstable'`, SteamCMD's word for a branch it may not use.
pub fn refused_beta(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let at = lower.find("failed to set beta '")? + "failed to set beta '".len();
    let rest = &line[at..];
    rest.find('\'').map(|end| rest[..end].to_string()).filter(|b| !b.is_empty())
}

/// What an error line means in words, when SteamCMD says it with a state code.
pub fn explain_error(line: &str) -> Option<&'static str> {
    let lower = line.to_ascii_lowercase();
    if lower.contains("missing configuration") {
        Some("Steam gave no details of the server on that branch: the branch may be private or gone; pick the branch your game is on")
    } else if lower.contains("0x202") {
        Some("not enough free space on the disk")
    } else if lower.contains("0x602") || lower.contains("0x402") {
        Some("the connection to Steam dropped; try again")
    } else if lower.contains("invalid beta") || lower.contains("invalid branch") {
        Some("Steam has no branch by that name")
    } else if lower.contains("no subscription") {
        Some("Steam refused the anonymous download")
    } else {
        None
    }
}

// ------------------------------------------------------------------ the install job

/// How the install or update is going, kept in a file so the window and an assistant both see it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct InstallProgress {
    pub running: bool,
    /// download_steamcmd, starting, steamcmd_update, login, preallocating, downloading,
    /// verifying, committing, done, error, cancelled.
    pub phase: String,
    pub percent: Option<f32>,
    pub done_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    /// The folder's size now (SteamCMD does not always print its progress through a pipe).
    pub disk_bytes: Option<u64>,
    pub line: String,
    pub error: Option<String>,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub dir: String,
    pub branch: String,
    pub command: String,
    pub pid: Option<u32>,
    /// Set when it worked: true when SteamCMD said it was already up to date (nothing downloaded).
    pub up_to_date: Option<bool>,
}

fn progress_file() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    dirs_next::config_dir().map(|d| d.join("PZModStudio").join("dedicated_install.json"))
}

static PROGRESS: std::sync::Mutex<Option<InstallProgress>> = std::sync::Mutex::new(None);

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn set_progress(p: &InstallProgress) {
    if let Ok(mut g) = PROGRESS.lock() {
        *g = Some(p.clone());
    }
    if let Some(f) = progress_file() {
        let tmp = f.with_extension("json.tmp");
        if std::fs::write(&tmp, serde_json::to_string(p).unwrap_or_default()).is_ok() {
            let _ = std::fs::rename(&tmp, &f);
        }
    }
}

/// The install job as it is now (this process's, else the one another process left in the file).
/// A job whose SteamCMD is gone without saying how it ended is told as cut short.
pub fn progress() -> Option<InstallProgress> {
    let mine = PROGRESS.lock().ok().and_then(|g| g.clone());
    let from_file = mine.is_none();
    let mut p = mine.or_else(|| progress_file().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()))?;
    // Another process's job (or one from before the app was closed): only as alive as its SteamCMD.
    if from_file && p.running {
        let gone = match p.pid {
            Some(pid) => !steamcmd_alive(pid),
            // Still downloading SteamCMD (3 MB): not after ten minutes.
            None => now_secs().saturating_sub(p.started_at) > 600,
        };
        if gone {
            p.running = false;
            p.phase = "error".into();
            p.error.get_or_insert_with(|| "SteamCMD stopped before it said it had finished (the app that ran it was closed?).".into());
        }
    }
    Some(p)
}

#[cfg(windows)]
fn steamcmd_alive(pid: u32) -> bool {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
        .creation_flags(0x08000000)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase().contains("steamcmd"))
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn steamcmd_alive(pid: u32) -> bool {
    crate::platform::process(pid).is_some_and(|p| p.name.to_ascii_lowercase().contains("steamcmd") || p.cmd.contains("steamcmd"))
}

/// Stops a running install (SteamCMD resumes from where it was the next time).
pub fn cancel() -> Result<bool, String> {
    let Some(mut p) = progress().filter(|p| p.running) else { return Ok(false) };
    if let Some(pid) = p.pid {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("taskkill").args(["/F", "/T", "/PID", &pid.to_string()]).creation_flags(0x08000000).output();
        }
        #[cfg(not(windows))]
        if steamcmd_alive(pid) {
            let _ = crate::platform::kill(pid, true, true);
        }
        let _ = pid;
    }
    CANCELLED.store(true, std::sync::atomic::Ordering::Relaxed);
    p.running = false;
    p.phase = "cancelled".into();
    p.finished_at = Some(now_secs());
    set_progress(&p);
    Ok(true)
}

static CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// This job was stopped, here or by another process (the app's window while an assistant's MCP
/// server runs the job, or the other way round): the file then says "cancelled" for it.
fn cancelled(started_at: u64) -> bool {
    CANCELLED.load(std::sync::atomic::Ordering::Relaxed)
        || progress_file()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|t| serde_json::from_str::<InstallProgress>(&t).ok())
            .is_some_and(|p| p.started_at == started_at && p.phase == "cancelled")
}

/// Why a folder cannot take the dedicated server, or None when it can: an absolute path, not a
/// drive's root, not the game's or the Zomboid folder, and empty or already a dedicated server.
pub fn folder_problem(dir: &Path, game_dir: &Path, zomboid: &Path) -> Option<String> {
    if !dir.is_absolute() {
        return Some("bad_folder: Pick a full folder path.".into());
    }
    if dir.parent().is_none() || dir.components().count() <= 1 {
        return Some("bad_folder: Not the root of a drive: pick or make a folder for the server.".into());
    }
    let inside = |a: &Path, b: &Path| {
        let (a, b) = (a.to_string_lossy().to_lowercase(), b.to_string_lossy().to_lowercase());
        !b.is_empty() && (a == b || a.starts_with(&format!("{}\\", b.trim_end_matches('\\'))) || a.starts_with(&format!("{}/", b.trim_end_matches('/'))))
    };
    if inside(dir, game_dir) || inside(game_dir, dir) {
        return Some("bad_folder: Not the game's folder: the dedicated server goes in a folder of its own.".into());
    }
    if inside(dir, zomboid) || inside(zomboid, dir) {
        return Some("bad_folder: Not the Zomboid folder (saves, mods, settings): pick another one.".into());
    }
    if dir.is_dir() && inspect(dir).is_none() {
        // A SteamCMD run that stopped before the first file leaves only its `steamapps` folder.
        let steamcmds_own = |e: &std::fs::DirEntry| e.file_name().to_string_lossy().eq_ignore_ascii_case("steamapps") && e.path().is_dir();
        let has_files = std::fs::read_dir(dir).map(|r| r.flatten().any(|e| !steamcmds_own(&e))).unwrap_or(false);
        if has_files {
            return Some("bad_folder: That folder has other files: pick an empty folder or one with the dedicated server.".into());
        }
    }
    None
}

/// Downloads steamcmd.zip from Valve into `into` and unpacks steamcmd.exe, checking that Valve
/// signed it. `on_bytes(done, total)` is called as it arrives.
pub fn download_steamcmd(into: &Path, on_bytes: &mut dyn FnMut(u64, Option<u64>)) -> Result<PathBuf, String> {
    std::fs::create_dir_all(into).map_err(|e| format!("Cannot make {}: {}", into.display(), e))?;
    let tls = ureq::tls::TlsConfig::builder().provider(ureq::tls::TlsProvider::NativeTls).build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_connect(Some(std::time::Duration::from_secs(30)))
        .timeout_global(Some(std::time::Duration::from_secs(10 * 60)))
        .user_agent(format!("PZModStudio/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut resp = agent.get(STEAMCMD_URL).call().map_err(|e| format!("download_failed: Could not download SteamCMD from Valve: {}", e))?;
    let total = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
    if total.is_some_and(|t| t > MAX_ZIP_BYTES) {
        return Err("download_failed: Valve's address answered with a file far bigger than SteamCMD.".into());
    }
    let mut reader = resp.body_mut().as_reader();
    let zip_path = into.join("steamcmd.zip.part");
    let mut file = std::fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("download_failed: {}", e))?;
        if n == 0 {
            break;
        }
        done += n as u64;
        if done > MAX_ZIP_BYTES {
            drop(file);
            let _ = std::fs::remove_file(&zip_path);
            return Err("download_failed: The download from Valve's address is far bigger than SteamCMD.".into());
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        on_bytes(done, total);
    }
    drop(file);
    let exe = unpack_steamcmd(&zip_path, into);
    let _ = std::fs::remove_file(&zip_path);
    let exe = exe?;
    if let Err(e) = check_valve_signature(&exe) {
        let _ = std::fs::remove_file(&exe);
        return Err(e);
    }
    Ok(exe)
}

/// Only `steamcmd.exe` comes out of the zip.
fn unpack_steamcmd(zip_path: &Path, into: &Path) -> Result<PathBuf, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("download_failed: Not a zip: {}", e))?;
    let mut entry = zip.by_name("steamcmd.exe").map_err(|_| "download_failed: The zip from Valve has no steamcmd.exe.".to_string())?;
    let exe = into.join("steamcmd.exe");
    let mut out = std::fs::File::create(&exe).map_err(|e| e.to_string())?;
    std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    Ok(exe)
}

/// Windows' own check of the file's signature: valid, and signed by Valve.
#[cfg(windows)]
pub fn check_valve_signature(exe: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$s = Get-AuthenticodeSignature -LiteralPath $env:PZMS_SIGNED_FILE; [string]$s.Status + '|' + [string]$s.SignerCertificate.Subject",
        ])
        .env("PZMS_SIGNED_FILE", exe)
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("signature: Could not ask Windows for steamcmd.exe's signature: {}", e))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    signature_ok(&text).then_some(()).ok_or_else(|| format!("signature: steamcmd.exe is not signed by Valve as Windows sees it ({}): it was deleted.", text))
}

#[cfg(not(windows))]
pub fn check_valve_signature(_exe: &Path) -> Result<(), String> {
    Err("signature: Only checked on Windows.".into())
}

/// `Status|Subject` as PowerShell prints it: valid, and Valve's certificate.
pub fn signature_ok(text: &str) -> bool {
    let (status, subject) = text.split_once('|').unwrap_or((text, ""));
    status.trim() == "Valid" && subject.contains("Valve")
}

fn dir_size(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum()
}

/// What to install and how.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InstallRequest {
    pub dir: String,
    pub branch: String,
    /// Check every file (slower; mends a broken install).
    pub validate: bool,
    /// Download SteamCMD from Valve when it is not on this PC.
    pub allow_download: bool,
}

/// Starts installing or updating the dedicated server in the background. Refused while one runs,
/// while the Studio's server runs from that folder, for a folder that cannot take it, and, without
/// `allow_download`, when SteamCMD is not on this PC (`steamcmd_missing`).
pub fn start_install(req: InstallRequest, game_dir: &Path, zomboid: &Path, server_running_from: Option<&Path>) -> Result<InstallProgress, String> {
    if progress().is_some_and(|p| p.running) {
        return Err("install_running: An install or update is already running.".into());
    }
    let dir = PathBuf::from(req.dir.trim());
    if let Some(problem) = folder_problem(&dir, game_dir, zomboid) {
        return Err(problem);
    }
    // No branch said: the game's, which is what a client of this PC can join.
    let game_branch = game_branch(game_dir);
    let branch = if req.branch.trim().is_empty() { game_branch.clone().unwrap_or_else(|| "public".to_string()) } else { req.branch.trim().to_string() };
    if !valid_branch(&branch) {
        return Err("bad_branch: A branch name is one word: letters, digits, '_', '-' or '.'.".into());
    }
    if server_running_from.is_some_and(|r| r.to_string_lossy().eq_ignore_ascii_case(&dir.to_string_lossy())) {
        return Err("server_running: The Studio's server runs from that folder: stop it before updating.".into());
    }
    let mut cfg = load_config();
    let steamcmd = find_steamcmd(&cfg);
    // Valve's zip holds the Windows program: on Linux SteamCMD comes from the distribution.
    if steamcmd.is_none() && !cfg!(windows) {
        return Err("steamcmd_missing: SteamCMD is not on this PC. Install it with your distribution's package (steamcmd) or Valve's steamcmd_linux.tar.gz, then point the Studio at steamcmd.sh.".into());
    }
    if steamcmd.is_none() && !req.allow_download {
        return Err(format!("steamcmd_missing: SteamCMD is not on this PC. Downloading it (about 3 MB, from {}) needs your go-ahead.", STEAMCMD_URL));
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot make {}: {}", dir.display(), e))?;
    let args = steamcmd_args(&dir, &branch, req.validate);
    let p = InstallProgress {
        running: true,
        phase: if steamcmd.is_some() { "starting".into() } else { "download_steamcmd".into() },
        started_at: now_secs(),
        dir: dir.to_string_lossy().to_string(),
        branch: branch.clone(),
        command: command_line(steamcmd.as_deref().unwrap_or(&studio_steamcmd_dir().join(STEAMCMD_EXE)), &args),
        ..Default::default()
    };
    CANCELLED.store(false, std::sync::atomic::Ordering::Relaxed);
    set_progress(&p);
    cfg.dir = p.dir.clone();
    cfg.branch = branch;
    let _ = save_config(&cfg);
    let job = p.clone();
    std::thread::spawn(move || {
        let mut p = job;
        let result = run_install(&mut p, steamcmd, &args, &dir, game_branch.as_deref());
        if cancelled(p.started_at) {
            return;
        }
        p.running = false;
        p.pid = None;
        p.finished_at = Some(now_secs());
        p.disk_bytes = Some(dir_size(&dir));
        match result {
            Ok(()) => {
                p.phase = "done".into();
                p.percent = Some(100.0);
            }
            Err(e) => {
                p.phase = "error".into();
                // A bar at 100% from SteamCMD updating itself would read as done.
                p.percent = None;
                p.done_bytes = None;
                p.total_bytes = None;
                p.error = Some(e);
            }
        }
        set_progress(&p);
    });
    Ok(p)
}

fn run_install(p: &mut InstallProgress, steamcmd: Option<PathBuf>, args: &[String], dir: &Path, game_branch: Option<&str>) -> Result<(), String> {
    let exe = match steamcmd {
        Some(e) => e,
        None => {
            let into = studio_steamcmd_dir();
            let mut last = std::time::Instant::now();
            let exe = download_steamcmd(&into, &mut |done, total| {
                if last.elapsed().as_millis() >= 300 {
                    last = std::time::Instant::now();
                    p.done_bytes = Some(done);
                    p.total_bytes = total;
                    p.percent = total.filter(|t| *t > 0).map(|t| done as f32 * 100.0 / t as f32);
                    set_progress(p);
                }
            })?;
            let mut cfg = load_config();
            cfg.steamcmd = exe.to_string_lossy().to_string();
            let _ = save_config(&cfg);
            exe
        }
    };
    p.phase = "starting".into();
    p.percent = None;
    p.done_bytes = None;
    p.total_bytes = None;
    p.command = command_line(&exe, args);
    set_progress(p);

    // SteamCMD holds its output back while it writes into a pipe (it all comes out when it ends), but
    // writes the same lines as they happen to its own logs/console_log.txt: progress is read there.
    let logs = exe.parent().unwrap_or(Path::new(".")).join("logs");
    let mut console_log = LogTail::new(logs.join("console_log.txt"));
    let mut content_log = LogTail::new(logs.join("content_log.txt"));

    let mut cmd = std::process::Command::new(&exe);
    cmd.args(args).current_dir(exe.parent().unwrap_or(Path::new("."))).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.spawn().map_err(|e| format!("Could not start SteamCMD: {}", e))?;
    p.pid = Some(child.id());
    set_progress(p);

    // The pipe, read on its own thread (SteamCMD ends some lines with \r only: split on both).
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let stdout = child.stdout.take().ok_or("No output from SteamCMD")?;
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        let mut pending = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            let n = reader.read(&mut byte).unwrap_or(0);
            if n > 0 && byte[0] != b'\n' && byte[0] != b'\r' {
                pending.push(byte[0]);
                continue;
            }
            if !pending.is_empty() {
                let _ = tx.send(String::from_utf8_lossy(&pending).trim().to_string());
                pending.clear();
            }
            if n == 0 {
                break;
            }
        }
    });
    let stderr = child.stderr.take();
    let stderr_job = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut e) = stderr {
            let _ = e.read_to_string(&mut text);
        }
        text
    });

    let mut outcome = Outcome::default();
    let mut log_live = false;
    let mut last_disk: Option<std::time::Instant> = None;
    let mut pipe_closed = false;
    loop {
        let exited = matches!(child.try_wait(), Ok(Some(_)));
        let mut from_pipe = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(line) => from_pipe.push(line),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    pipe_closed = true;
                    break;
                }
            }
        }
        let from_log = console_log.lines();
        log_live |= !from_log.is_empty();
        for line in &from_log {
            apply_line(p, strip_log_time(line), &mut outcome, true);
        }
        // Once the log is read, the pipe's late copy of the same lines would only set the bar back.
        for line in &from_pipe {
            apply_line(p, line, &mut outcome, !log_live);
        }
        if last_disk.is_none_or(|t| t.elapsed().as_secs() >= 4) {
            last_disk = Some(std::time::Instant::now());
            p.disk_bytes = Some(dir_size(dir)).filter(|b| *b > 0);
        }
        // Stopped by another process: its "cancelled" is not written over.
        if cancelled(p.started_at) {
            break;
        }
        set_progress(p);
        if pipe_closed && exited {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(700));
    }
    for line in console_log.lines() {
        apply_line(p, strip_log_time(&line), &mut outcome, true);
    }
    let err_text = stderr_job.join().unwrap_or_default();
    let status = child.wait().map_err(|e| e.to_string())?;
    // A branch Steam keeps behind a password (or has no more): SteamCMD says only "Missing
    // configuration" or "Failed to set beta"; its content log names the branch.
    if outcome.finished != Some(true) {
        let refused = refused_beta(&outcome.error_line).or_else(|| private_beta_in_log(&content_log.lines().join("\n")));
        if let Some(beta) = refused {
            return Err(format!(
                "bad_branch: Steam would not give the \"{}\" branch of the dedicated server: it is private (it needs a password) or gone.{}",
                beta,
                game_branch.map(|g| format!(" Your game is on \"{}\": pick that one.", g)).unwrap_or_default()
            ));
        }
    }
    match outcome.finished {
        Some(true) => Ok(()),
        Some(false) => {
            p.line = outcome.error_line.chars().take(300).collect();
            Err(match explain_error(&outcome.error_line) {
                Some(why) => format!("{} ({})", outcome.error_line, why),
                None => outcome.error_line,
            })
        }
        None if status.success() && inspect(dir).is_some_and(|i| i.ready) => Ok(()),
        None => Err(format!(
            "SteamCMD ended (exit code {}) without saying it installed the server{}",
            status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
            err_text.lines().find(|l| !l.trim().is_empty()).map(|l| format!(": {}", l.trim())).unwrap_or_default()
        )),
    }
}

/// How SteamCMD said the job ended, from whichever of its outputs said it first.
#[derive(Default)]
struct Outcome {
    finished: Option<bool>,
    error_line: String,
}

/// One line of SteamCMD's output into the job's progress. `progress: false` only looks for the end.
fn apply_line(p: &mut InstallProgress, line: &str, outcome: &mut Outcome, progress: bool) {
    let line = line.trim();
    let info = parse_line(line);
    if info.as_ref().is_some_and(|i| i.finished == Some(true)) {
        p.up_to_date = Some(line.to_ascii_lowercase().contains("already up to date"));
    }
    if let Some(ok) = info.as_ref().and_then(|i| i.finished) {
        // An error said once stays said: a later "Success!" of another step does not undo it.
        if outcome.finished != Some(false) {
            outcome.finished = Some(ok);
            if !ok {
                outcome.error_line = line.to_string();
            }
        }
    }
    if !progress {
        return;
    }
    if let Some(info) = info {
        if info.phase != p.phase && info.percent.is_none() {
            // A new step: the last one's percent and bytes are not this one's.
            p.percent = None;
            p.done_bytes = None;
            p.total_bytes = None;
        }
        p.phase = info.phase.clone();
        if info.percent.is_some() {
            p.percent = info.percent;
        }
        if info.total_bytes.is_some() {
            p.done_bytes = info.done_bytes;
            p.total_bytes = info.total_bytes;
        }
    }
    if !line.is_empty() {
        p.line = line.chars().take(300).collect();
    }
}

/// The lines a log file gained since the last look (from where it was when the job began).
struct LogTail {
    path: PathBuf,
    offset: u64,
    rest: String,
}

impl LogTail {
    fn new(path: PathBuf) -> Self {
        let offset = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        LogTail { path, offset, rest: String::new() }
    }

    fn lines(&mut self) -> Vec<String> {
        use std::io::Seek;
        let Ok(mut f) = std::fs::File::open(&self.path) else { return Vec::new() };
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            // Started anew.
            self.offset = 0;
            self.rest.clear();
        }
        if len == self.offset || f.seek(std::io::SeekFrom::Start(self.offset)).is_err() {
            return Vec::new();
        }
        let mut buf = Vec::new();
        if f.take(8 * 1024 * 1024).read_to_end(&mut buf).is_err() {
            return Vec::new();
        }
        self.offset += buf.len() as u64;
        let text = std::mem::take(&mut self.rest) + &String::from_utf8_lossy(&buf);
        let mut parts: Vec<&str> = text.split(['\n', '\r']).collect();
        // The last piece has no line end yet: kept for the next look.
        self.rest = parts.pop().unwrap_or("").to_string();
        parts.into_iter().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect()
    }
}

/// Everything the screen shows about the dedicated server program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedicatedView {
    pub config: DedicatedConfig,
    pub steamcmd: Option<String>,
    pub steamcmd_url: String,
    pub installs: Vec<DedicatedInstall>,
    /// The chosen folder as it is now.
    pub chosen: Option<DedicatedInstall>,
    /// The folder the next server start uses: the dedicated one, or None for the game's.
    pub starts_from: Option<String>,
    pub progress: Option<InstallProgress>,
    /// The command an update would run with the chosen folder and branch.
    pub update_command: Option<String>,
    pub default_dir: String,
    /// The branch the installed game is on ("public" when none): the one to install for it to join.
    pub game_branch: Option<String>,
    /// The installed game's build ("42.22"), to set beside the dedicated server's `version`.
    pub game_version: Option<String>,
}

pub fn view() -> DedicatedView {
    let config = load_config();
    let steamcmd = find_steamcmd(&config);
    let chosen = (!config.dir.trim().is_empty()).then(|| inspect(Path::new(config.dir.trim()))).flatten();
    let update_command = (!config.dir.trim().is_empty()).then(|| {
        command_line(steamcmd.as_deref().unwrap_or(&studio_steamcmd_dir().join(STEAMCMD_EXE)), &steamcmd_args(Path::new(config.dir.trim()), &config.branch, validate_by_default(Path::new(config.dir.trim()))))
    });
    DedicatedView {
        steamcmd: steamcmd.map(|p| p.to_string_lossy().to_string()),
        steamcmd_url: STEAMCMD_URL.to_string(),
        installs: find_installs(&config),
        chosen,
        starts_from: chosen_server_dir().map(|d| d.to_string_lossy().to_string()),
        progress: progress(),
        update_command,
        game_branch: game_branch(Path::new(&crate::vfs::trusted_install_dir())),
        game_version: crate::load_order::mod_info::installed_jar_game_version().map(crate::catalog::modinfo::format_game_version),
        default_dir: studio_steamcmd_dir().parent().unwrap_or(Path::new(".")).join(DEFAULT_FOLDER_NAME).to_string_lossy().to_string(),
        config,
    }
}

/// Changes which program starts servers, the folder or the branch. Choosing "dedicated" needs a
/// folder a server can start from.
pub fn set(source: Option<String>, dir: Option<String>, branch: Option<String>, steamcmd: Option<String>) -> Result<DedicatedConfig, String> {
    let mut cfg = load_config();
    if let Some(d) = dir {
        cfg.dir = d.trim().to_string();
    }
    if let Some(b) = branch {
        let b = b.trim().to_string();
        if !b.is_empty() && !valid_branch(&b) {
            return Err("bad_branch: A branch name is one word: letters, digits, '_', '-' or '.'.".into());
        }
        cfg.branch = b;
    }
    if let Some(s) = steamcmd {
        let s = s.trim().to_string();
        if !s.is_empty() && !(Path::new(&s).is_file() && Path::new(&s).file_name().is_some_and(|n| is_steamcmd_name(&n.to_string_lossy()))) {
            return Err(format!("bad_steamcmd: That is not a {}.", STEAMCMD_EXE));
        }
        cfg.steamcmd = s;
    }
    if let Some(src) = source {
        match src.as_str() {
            "game" => cfg.source = "game".into(),
            "dedicated" => {
                if layout(Path::new(cfg.dir.trim())).is_none() {
                    return Err("not_installed: The dedicated server is not installed in that folder yet: install it first.".into());
                }
                cfg.source = "dedicated".into();
            }
            _ => return Err("source is \"game\" or \"dedicated\".".into()),
        }
    }
    save_config(&cfg)?;
    Ok(cfg)
}

/// Reads SteamCMD's whole output (for tests and for a finished job's log).
pub fn parse_output(text: &str) -> Vec<LineInfo> {
    std::io::BufReader::new(text.as_bytes()).split(b'\n').flatten().flat_map(|l| String::from_utf8_lossy(&l).split('\r').filter_map(parse_line).collect::<Vec<_>>()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steamcmd_command_installs_the_chosen_branch() {
        let dir = Path::new("D:\\Servers\\PZ Dedicated\\");
        let a = steamcmd_args(dir, "unstable", true);
        assert_eq!(a, ["+force_install_dir", "D:\\Servers\\PZ Dedicated", "+login", "anonymous", "+app_update", "380870", "-beta", "unstable", "validate", "+quit"]);
        let b = steamcmd_args(Path::new("C:\\pz"), "", false);
        assert_eq!(b.join(" "), "+force_install_dir C:\\pz +login anonymous +app_update 380870 -beta public +quit", "public is said, or a folder stays on its last beta");
        assert_eq!(
            command_line(Path::new("C:\\Program Files\\steamcmd\\steamcmd.exe"), &a),
            "\"C:\\Program Files\\steamcmd\\steamcmd.exe\" +force_install_dir \"D:\\Servers\\PZ Dedicated\" +login anonymous +app_update 380870 -beta unstable validate +quit"
        );
        assert!(valid_branch("unstable") && valid_branch("public") && valid_branch("b41.78"));
        assert!(!valid_branch("x +quit") && !valid_branch("") && !valid_branch("a\"b"));
    }

    #[test]
    fn steamcmd_output_becomes_progress() {
        let out = "Redirecting stderr to 'C:\\steamcmd\\logs\\stderr.txt'\r\n\
[  0%] Checking for available updates...\r\n\
[ 40%] Downloading update (15,792 of 39,483 KB)...\r\n\
[----] Verifying installation...\r\n\
Steam Console Client (c) Valve Corporation - version 1730853794\r\n\
Connecting anonymously to Steam Public...OK\r\n\
 Update state (0x11) preallocating, progress: 0.00 (0 / 0)\r\n\
 Update state (0x61) downloading, progress: 2.07 (61287133 / 2959812457)\r\n\
 Update state (0x81) verifying update, progress: 99.10 (2933000000 / 2959812457)\r\n\
Success! App '380870' fully installed.\r\n";
        let lines = parse_output(out);
        let phases: Vec<&str> = lines.iter().map(|l| l.phase.as_str()).collect();
        assert_eq!(phases, ["steamcmd_update", "steamcmd_update", "steamcmd_update", "starting", "login", "preallocating", "downloading", "verifying", "done"]);
        assert_eq!(lines[1].percent, Some(40.0));
        assert_eq!(lines[1].done_bytes, Some(15_792 * 1024));
        assert_eq!(lines[1].total_bytes, Some(39_483 * 1024));
        assert_eq!(lines[2].percent, None);
        assert_eq!((lines[6].percent, lines[6].done_bytes, lines[6].total_bytes), (Some(2.07), Some(61_287_133), Some(2_959_812_457)));
        assert_eq!(lines[8].finished, Some(true));

        let err = parse_line("Error! App '380870' state is 0x202 after update job.").unwrap();
        assert_eq!((err.phase.as_str(), err.finished), ("error", Some(false)));
        assert_eq!(explain_error("Error! App '380870' state is 0x202 after update job."), Some("not enough free space on the disk"));
        assert_eq!(parse_line("ERROR! Failed to install app '380870' (Invalid platform)").and_then(|l| l.finished), Some(false));
        assert!(parse_line("Success! App '380870' already up to date.").is_some_and(|l| l.finished == Some(true)));
        let mut p = InstallProgress::default();
        apply_line(&mut p, "Success! App '380870' already up to date.", &mut Outcome::default(), false);
        assert_eq!(p.up_to_date, Some(true), "said from either output");
        apply_line(&mut p, "Success! App '380870' fully installed.", &mut Outcome::default(), true);
        assert_eq!(p.up_to_date, Some(false));
        assert!(parse_line("   ").is_none() && parse_line("Loading Steam API...OK").is_some());
    }

    #[test]
    fn the_servers_own_start_script_gives_the_class_path() {
        let b42 = "@setlocal enableextensions\r\n@cd /d \"%~dp0\"\r\nSET _JAVA_OPTIONS=\r\nSET PZ_CLASSPATH=./;projectzomboid.jar\r\n\".\\jre64\\bin\\java.exe\" --enable-native-access=ALL-UNNAMED -XX:+UseZGC -Xmx3072m -Djava.library.path=./natives/;./natives/win64/;./ -cp %PZ_CLASSPATH% zombie.network.GameServer\r\nPAUSE\r\n";
        assert_eq!(parse_start_script(b42), (Some("./;projectzomboid.jar".into()), Some("./natives/;./natives/win64/;./".into())));
        let b41 = "SET PZ_CLASSPATH=java/istack-commons-runtime.jar;java/jassimp.jar;java/\r\n\".\\jre64\\bin\\java.exe\" -Djava.awt.headless=true -Xms16g -Xmx16g -Djava.library.path=natives/;natives/win64/;. -cp %PZ_CLASSPATH% zombie.network.GameServer -statistic 0 %1 %2\r\n";
        assert_eq!(parse_start_script(b41).0.as_deref(), Some("java/istack-commons-runtime.jar;java/jassimp.jar;java/"));
        let inline = "\"jre64\\bin\\java.exe\" -cp java/;java/projectzomboid.jar zombie.network.GameServer";
        assert_eq!(parse_start_script(inline), (Some("java/;java/projectzomboid.jar".into()), None));
        assert_eq!(parse_start_script(""), (None, None));
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pzms-dedicated-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_folder_is_read_for_what_it_holds_of_the_server() {
        let d = temp("inspect");
        assert!(inspect(&d).is_none(), "an empty folder holds nothing");
        assert!(layout(&d).is_none());
        std::fs::create_dir_all(d.join("steamapps")).unwrap();
        std::fs::write(
            d.join("steamapps").join("appmanifest_380870.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\t\"380870\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"buildid\"\t\t\"20512345\"\n\t\"LastUpdated\"\t\t\"1795000000\"\n\t\"SizeOnDisk\"\t\t\"3100000000\"\n\t\"UserConfig\"\n\t{\n\t\t\"BetaKey\"\t\t\"unstable\"\n\t}\n}\n",
        )
        .unwrap();
        let found = inspect(&d).unwrap();
        assert_eq!((found.build_id.as_deref(), found.branch.as_deref(), found.size_on_disk, found.ready, found.incomplete), (Some("20512345"), Some("unstable"), Some(3_100_000_000), false, false));
        std::fs::create_dir_all(d.join("jre64").join("bin")).unwrap();
        std::fs::write(d.join("jre64").join("bin").join(crate::platform::JAVA_EXE), "").unwrap();
        std::fs::write(d.join("projectzomboid.jar"), "").unwrap();
        assert!(inspect(&d).unwrap().ready);
        assert!(!validate_by_default(&d), "an update of a complete install only fetches what changed");
        let downloading = d.join("steamapps").join("downloading").join("380870");
        std::fs::create_dir_all(&downloading).unwrap();
        assert!(!inspect(&d).unwrap().incomplete, "the empty folder a stopped check leaves");
        std::fs::write(downloading.join("chunk"), "x").unwrap();
        assert!(inspect(&d).unwrap().incomplete && validate_by_default(&d), "a download still to finish");
        std::fs::remove_dir_all(d.join("steamapps").join("downloading")).unwrap();
        let record = d.join("steamapps").join("appmanifest_380870.acf");
        let text = std::fs::read_to_string(&record).unwrap();
        std::fs::write(&record, text.replace("	\"StateFlags\"", "	\"FullValidateBeforeNextUpdate\"		\"1\"
	\"StateFlags\"")).unwrap();
        assert!(inspect(&d).unwrap().check_due && validate_by_default(&d), "a stopped check is done again");
        assert!(validate_by_default(&d.join("elsewhere")), "a first install checks every file");
        let l = layout(&d).unwrap();
        #[cfg(windows)]
        assert_eq!((l.classpath.as_str(), l.library_path.as_str()), ("./;projectzomboid.jar", "natives/;natives/win64/;."));
        #[cfg(not(windows))]
        assert_eq!((l.classpath.as_str(), l.library_path.as_str()), ("./:projectzomboid.jar", "linux64/:natives/:."));
        let (_, branch, ..) = read_manifest("\"AppState\" { \"buildid\" \"1\" }");
        assert_eq!(branch.as_deref(), Some("public"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_server_goes_in_a_folder_of_its_own() {
        let d = temp("folder");
        let game = PathBuf::from("D:\\Apps\\Steam\\steamapps\\common\\ProjectZomboid");
        let zomboid = PathBuf::from("C:\\Users\\x\\Zomboid");
        assert!(folder_problem(&d.join("new"), &game, &zomboid).is_none(), "a folder still to be made");
        assert!(folder_problem(&d, &game, &zomboid).is_none(), "an empty one");
        std::fs::write(d.join("notes.txt"), "x").unwrap();
        assert!(folder_problem(&d, &game, &zomboid).unwrap().starts_with("bad_folder"), "one with other files");
        assert!(folder_problem(Path::new("C:\\"), &game, &zomboid).is_some());
        assert!(folder_problem(&game.join("server"), &game, &zomboid).is_some());
        assert!(folder_problem(Path::new("C:\\Users\\x\\Zomboid\\server"), &game, &zomboid).is_some());
        assert!(folder_problem(Path::new("relative\\dir"), &game, &zomboid).is_some());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_folder_a_stopped_steamcmd_run_left_can_be_tried_again() {
        let d = temp("leftover");
        let game = PathBuf::from("D:\\Apps\\Steam\\steamapps\\common\\ProjectZomboid");
        let zomboid = PathBuf::from("C:\\Users\\x\\Zomboid");
        std::fs::create_dir_all(d.join("steamapps")).unwrap();
        assert!(folder_problem(&d, &game, &zomboid).is_none(), "only SteamCMD's own steamapps folder");
        std::fs::write(d.join("steamapps").join("libraryfolders.vdf"), "x").unwrap();
        assert!(folder_problem(&d, &game, &zomboid).is_none());
        std::fs::write(d.join("notes.txt"), "x").unwrap();
        assert!(folder_problem(&d, &game, &zomboid).is_some(), "with a file of someone else's");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn steamcmds_own_log_gives_the_progress_and_the_reason() {
        assert_eq!(strip_log_time("[2026-11-18 06:07:32] Waiting for user info...OK"), "Waiting for user info...OK");
        assert_eq!(strip_log_time("[ 40%] Downloading update (15,792 of 39,483 KB)..."), "[ 40%] Downloading update (15,792 of 39,483 KB)...", "SteamCMD's own percent stays");
        assert_eq!(strip_log_time("[2026-11-18 06:09:33]  Update state (0x61) downloading, progress: 2.07 (61287133 / 2959812457)").starts_with("Update state"), true);
        assert_eq!(strip_log_time(""), "");
        let content = "[2026-11-18 06:07:32] AppID 380870 App update changed : Running Update,Reconfiguring,\n[2026-11-18 06:07:32] WARNING! AppID 380870 has missing cached password for private beta \"unstable\" set, falling back to public\n[2026-11-18 06:07:32] AppID 380870 beta changed: public\n";
        assert_eq!(private_beta_in_log(content).as_deref(), Some("unstable"));
        assert_eq!(private_beta_in_log("AppID 380870 beta changed: unstable"), None);
        assert_eq!(refused_beta("ERROR! Failed to set beta 'unstable'").as_deref(), Some("unstable"));
        assert_eq!(refused_beta("ERROR! Failed to install app '380870' (Missing configuration)"), None);
        assert!(explain_error("ERROR! Failed to install app '380870' (Missing configuration)").is_some_and(|w| w.contains("branch")));

        // The log is read as it grows, a half-written line kept for the next look.
        let d = temp("tail");
        let f = d.join("console_log.txt");
        std::fs::write(&f, "[2026-11-18 06:00:00] from an earlier run\n").unwrap();
        let mut tail = LogTail::new(f.clone());
        assert!(tail.lines().is_empty(), "what was there before the job is not read");
        let mut file = std::fs::OpenOptions::new().append(true).open(&f).unwrap();
        file.write_all(b"[2026-11-18 06:07:31] Connecting anonymously to Steam Public...OK\r\n[2026-11-18 06:07:32] Update state (0x61) downloading, progress: 50.00 (100 / 200").unwrap();
        assert_eq!(tail.lines(), ["[2026-11-18 06:07:31] Connecting anonymously to Steam Public...OK"]);
        file.write_all(b")\n").unwrap();
        let l = tail.lines();
        assert_eq!(l.len(), 1);
        let mut p = InstallProgress::default();
        let mut outcome = Outcome::default();
        apply_line(&mut p, strip_log_time(&l[0]), &mut outcome, true);
        assert_eq!((p.phase.as_str(), p.percent, p.done_bytes, p.total_bytes), ("downloading", Some(50.0), Some(100), Some(200)));
        // The pipe's late copy of SteamCMD updating itself does not set the bar back...
        apply_line(&mut p, "[100%] Downloading update (43,472 of 43,472 KB)...", &mut outcome, false);
        assert_eq!((p.phase.as_str(), p.percent), ("downloading", Some(50.0)));
        // ...but how the job ended counts from either.
        apply_line(&mut p, "ERROR! Failed to install app '380870' (Missing configuration)", &mut outcome, false);
        apply_line(&mut p, "Success! App '380870' fully installed.", &mut outcome, true);
        assert_eq!(outcome.finished, Some(false), "an error said once stays said");
        // A new step drops the last one's numbers.
        let mut q = InstallProgress { phase: "steamcmd_update".into(), percent: Some(100.0), done_bytes: Some(1), total_bytes: Some(1), ..Default::default() };
        apply_line(&mut q, "Logging in user 'anonymous' to Steam Public...", &mut Outcome::default(), true);
        assert_eq!((q.phase.as_str(), q.percent, q.total_bytes), ("login", None, None));
        drop(file);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_game_s_branch_is_read_from_its_manifest() {
        let d = temp("gamebranch");
        let game = d.join("steamapps").join("common").join("ProjectZomboid");
        std::fs::create_dir_all(&game).unwrap();
        assert_eq!(game_branch(&game), None, "not a Steam install");
        std::fs::write(d.join("steamapps").join("appmanifest_108600.acf"), "\"AppState\"\n{\n\t\"buildid\"\t\t\"26486631\"\n\t\"UserConfig\"\n\t{\n\t\t\"language\"\t\t\"english\"\n\t}\n}\n").unwrap();
        assert_eq!(game_branch(&game).as_deref(), Some("public"));
        std::fs::write(d.join("steamapps").join("appmanifest_108600.acf"), "\"AppState\"\n{\n\t\"buildid\"\t\t\"1\"\n\t\"UserConfig\"\n\t{\n\t\t\"BetaKey\"\t\t\"unstable\"\n\t}\n}\n").unwrap();
        assert_eq!(game_branch(&game).as_deref(), Some("unstable"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn only_valves_signature_lets_steamcmd_run() {
        assert!(signature_ok("Valid|CN=Valve Corp., O=Valve Corp., L=Bellevue, S=Washington, C=US"));
        assert!(!signature_ok("NotSigned|"));
        assert!(!signature_ok("Valid|CN=Someone Else"));
        assert!(!signature_ok("HashMismatch|CN=Valve Corp."));
    }
}

// ------------------------------------------------------------------ the window's calls

#[tauri::command(async)]
pub fn dedicated_server_view() -> DedicatedView {
    view()
}

#[tauri::command(async)]
pub fn dedicated_server_set(source: Option<String>, dir: Option<String>, branch: Option<String>, steamcmd: Option<String>) -> Result<DedicatedConfig, String> {
    set(source, dir, branch, steamcmd)
}

#[tauri::command(async)]
pub fn dedicated_server_install(paths: crate::vfs::StudioPaths, request: InstallRequest) -> Result<InstallProgress, String> {
    let zomboid = super::primary_dir(&paths.user_zomboid_dir);
    let status = super::get_dedicated_server_status(zomboid.to_string_lossy().to_string()).ok().filter(|s| s.is_running);
    let running_from = status.and_then(|s| s.program_dir).map(PathBuf::from);
    start_install(request, Path::new(&crate::vfs::trusted_install_dir()), &zomboid, running_from.as_deref())
}

#[tauri::command(async)]
pub fn dedicated_server_cancel() -> Result<bool, String> {
    cancel()
}
