//! What differs between Windows and Linux: program names, the processes running, ending them,
//! folder links and the PC's memory. On Windows each helper does what the Studio always did
//! (tasklist / taskkill / junctions); on Linux it reads `/proc` and uses signals and symlinks.
//! Things only Windows can do (the game window, the event log) say so where they are asked for.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The game's own program in its folder: `ProjectZomboid64.exe` on Windows, the `ProjectZomboid64`
/// launcher on Linux (what `projectzomboid.sh` starts; it runs the JVM inside itself).
pub const GAME_EXE: &str = if cfg!(windows) { "ProjectZomboid64.exe" } else { "ProjectZomboid64" };

/// Java's program name inside `jre64/bin`.
pub const JAVA_EXE: &str = if cfg!(windows) { "java.exe" } else { "java" };

/// The separator of a Java class path or library path.
pub const PATH_LIST_SEP: &str = if cfg!(windows) { ";" } else { ":" };

/// What to say when a feature needs Windows.
pub fn windows_only(what: &str) -> String {
    format!("{} is only available on Windows.", what)
}

/// The program that starts the game from its folder. On Linux the `projectzomboid.sh` script when
/// it is there (it sets the library paths the launcher needs), else the launcher itself.
pub fn game_launcher(install_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        return install_dir.join(GAME_EXE);
    }
    let script = install_dir.join("projectzomboid.sh");
    if script.is_file() {
        script
    } else {
        install_dir.join(GAME_EXE)
    }
}

/// A command started from the Studio opens no console window (Windows' CREATE_NO_WINDOW).
pub fn no_window(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

/// One running process.
#[derive(Debug, Clone, Default)]
pub struct Proc {
    pub pid: u32,
    /// The program's file name ("ProjectZomboid64.exe", "java").
    pub name: String,
    /// The command line, arguments separated by spaces.
    pub cmd: String,
    /// Resident memory in bytes.
    pub rss: u64,
    /// CPU time used so far, in 100-ns units (Windows' unit, which `resources` counts in).
    pub cpu_100ns: u64,
    /// Seconds since it started.
    pub up_secs: u64,
    pub ppid: u32,
}

#[cfg(unix)]
mod linux {
    use super::Proc;
    use std::path::Path;

    fn clock_ticks() -> u64 {
        // sysconf(_SC_CLK_TCK) is 100 on every Linux the game runs on.
        100
    }

    fn boot_uptime() -> f64 {
        std::fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|t| t.split_whitespace().next().and_then(|v| v.parse().ok()))
            .unwrap_or(0.0)
    }

    pub fn read(pid: u32) -> Option<Proc> {
        let base = Path::new("/proc").join(pid.to_string());
        let stat = std::fs::read_to_string(base.join("stat")).ok()?;
        // "pid (comm) state ppid ..." — comm may hold spaces and parentheses: split at the last ')'.
        let close = stat.rfind(')')?;
        let comm = stat[stat.find('(')? + 1..close].to_string();
        let fields: Vec<&str> = stat[close + 1..].split_whitespace().collect();
        let num = |i: usize| fields.get(i).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        // fields[0] is the state (stat field 3): ppid = 4, utime = 14, stime = 15, starttime = 22.
        let ppid = num(1) as u32;
        let ticks = clock_ticks();
        let cpu_100ns = (num(11) + num(12)) * 10_000_000 / ticks;
        let up_secs = (boot_uptime() - num(19) as f64 / ticks as f64).max(0.0) as u64;
        let rss_pages = std::fs::read_to_string(base.join("statm"))
            .ok()
            .and_then(|t| t.split_whitespace().nth(1).and_then(|v| v.parse::<u64>().ok()))
            .unwrap_or(0);
        let cmd = std::fs::read(base.join("cmdline"))
            .map(|b| b.split(|c| *c == 0).filter(|a| !a.is_empty()).map(|a| String::from_utf8_lossy(a).to_string()).collect::<Vec<_>>().join(" "))
            .unwrap_or_default();
        // The program's file name: the executable's, else the first argument's, else comm (15 chars).
        let name = std::fs::read_link(base.join("exe"))
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().trim_end_matches(" (deleted)").to_string()))
            .or_else(|| cmd.split(' ').next().and_then(|a| Path::new(a).file_name()).map(|n| n.to_string_lossy().to_string()))
            .filter(|n| !n.is_empty())
            .unwrap_or(comm);
        Some(Proc { pid, name, cmd, rss: rss_pages * 4096, cpu_100ns, up_secs, ppid })
    }

    pub fn all() -> Vec<Proc> {
        std::fs::read_dir("/proc")
            .map(|d| d.flatten().filter_map(|e| e.file_name().to_str().and_then(|n| n.parse::<u32>().ok())).filter_map(read).collect())
            .unwrap_or_default()
    }
}

/// Every process on this PC (Linux: from /proc). On Windows only the game's and Java's are
/// listed, through the callers' own tools; this returns nothing there.
pub fn processes() -> Vec<Proc> {
    #[cfg(unix)]
    {
        linux::all()
    }
    #[cfg(not(unix))]
    {
        Vec::new()
    }
}

/// The process with that id, when it runs (Linux only; None on Windows).
pub fn process(pid: u32) -> Option<Proc> {
    #[cfg(unix)]
    {
        linux::read(pid)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        None
    }
}

/// Whether a process is the game's client: the launcher, or a Java running its main class.
pub fn is_game_process(name: &str, cmd: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "projectzomboid64.exe"
        || n == "projectzomboid64"
        || n == "projectzomboid32"
        || ((n == "java" || n == "java.exe") && cmd.contains("zombie.gameStates.MainScreenState"))
}

/// The ids of the running game clients, oldest first.
pub fn game_pids() -> Vec<u32> {
    #[cfg(windows)]
    {
        tasklist_pids(GAME_EXE)
    }
    #[cfg(not(windows))]
    {
        let mut games: Vec<Proc> = processes().into_iter().filter(|p| is_game_process(&p.name, &p.cmd)).collect();
        games.sort_by(|a, b| b.up_secs.cmp(&a.up_secs));
        games.into_iter().map(|p| p.pid).collect()
    }
}

/// The ids of processes whose program is `image` ("ProjectZomboid64.exe").
#[cfg(windows)]
fn tasklist_pids(image: &str) -> Vec<u32> {
    let mut cmd = Command::new("tasklist");
    cmd.args(["/FI", &format!("IMAGENAME eq {}", image), "/FO", "CSV", "/NH"]);
    no_window(&mut cmd);
    let Ok(out) = cmd.output() else { return Vec::new() };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut cols = l.split(',').map(|c| c.trim().trim_matches('"'));
            let name = cols.next()?;
            let pid = cols.next()?.parse::<u32>().ok()?;
            name.eq_ignore_ascii_case(image).then_some(pid)
        })
        .collect()
}

/// The lowercase program name of a running process, or None when no process has that id.
pub fn process_image_name(pid: u32) -> Option<String> {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("tasklist");
        cmd.args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"]);
        no_window(&mut cmd);
        let output = cmd.output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let line = text.lines().find(|l| l.starts_with('"'))?;
        let mut cols = line.split(',').map(|c| c.trim_matches('"'));
        let image = cols.next()?.to_lowercase();
        let listed_pid = cols.next()?.parse::<u32>().ok()?;
        (listed_pid == pid).then_some(image)
    }
    #[cfg(not(windows))]
    {
        process(pid).map(|p| p.name.to_lowercase())
    }
}

/// Ends a process; `tree` also ends the processes it started, `force` without asking it first
/// (taskkill /F; SIGKILL instead of SIGTERM). Ok when it is gone or was not running.
pub fn kill(pid: u32, tree: bool, force: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("taskkill");
        if force {
            cmd.arg("/F");
        }
        if tree {
            cmd.arg("/T");
        }
        cmd.args(["/PID", &pid.to_string()]);
        no_window(&mut cmd);
        let out = cmd.output().map_err(|e| format!("Failed to run taskkill: {}", e))?;
        if out.status.success() || process_image_name(pid).is_none() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
    #[cfg(not(windows))]
    {
        let mut targets = vec![pid];
        if tree {
            // Children first found breadth-first from the whole process list.
            let all = processes();
            let mut i = 0;
            while i < targets.len() {
                let parent = targets[i];
                targets.extend(all.iter().filter(|p| p.ppid == parent && !targets.contains(&p.pid)).map(|p| p.pid).collect::<Vec<_>>());
                i += 1;
            }
        }
        let signal = if force { "-KILL" } else { "-TERM" };
        let out = Command::new("kill")
            .arg(signal)
            .args(targets.iter().map(|p| p.to_string()))
            .output()
            .map_err(|e| format!("Failed to run kill: {}", e))?;
        if out.status.success() || process(pid).is_none() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

/// A folder link at `link` to `target`: a directory junction on Windows (no admin rights needed),
/// a symbolic link on Linux.
pub fn link_dir(link: &Path, target: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("cmd.exe");
        cmd.args(["/c", "mklink", "/J"]).arg(link).arg(target);
        no_window(&mut cmd);
        let out = cmd.output().map_err(|e| e.to_string())?;
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

/// The PC's physical memory and what is free of it, in bytes; (0, 0) when unknown.
pub fn memory_bytes() -> (u64, u64) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        // SAFETY: a zeroed MEMORYSTATUSEX with its length set, as the call asks.
        if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
            return (0, 0);
        }
        (status.ullTotalPhys, status.ullAvailPhys)
    }
    #[cfg(not(windows))]
    {
        let Ok(text) = std::fs::read_to_string("/proc/meminfo") else { return (0, 0) };
        let kb = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(key).map(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().unwrap_or(0)))
                .unwrap_or(0)
        };
        (kb("MemTotal:") * 1024, kb("MemAvailable:") * 1024)
    }
}

/// Free and total bytes of the drive holding `path` (Linux: `df`); None when unknown.
#[cfg(not(windows))]
pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
    let out = Command::new("df").args(["-P", "-B1"]).arg(path).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let cols: Vec<&str> = text.lines().nth(1)?.split_whitespace().collect();
    let total = cols.get(1)?.parse::<u64>().ok()?;
    let free = cols.get(3)?.parse::<u64>().ok()?;
    Some((free, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_game_is_the_launcher_or_java_running_its_main_class() {
        assert!(is_game_process("ProjectZomboid64.exe", ""));
        assert!(is_game_process("ProjectZomboid64", "./ProjectZomboid64 -debug"));
        assert!(is_game_process("java", "java -cp projectzomboid.jar zombie.gameStates.MainScreenState"));
        assert!(!is_game_process("java", "java -cp projectzomboid.jar zombie.network.GameServer"));
        assert!(!is_game_process("bash", "bash projectzomboid.sh"));
    }

    #[test]
    fn this_process_is_listed_with_its_memory() {
        if cfg!(windows) {
            return;
        }
        let me = process(std::process::id()).expect("own process");
        assert!(me.rss > 0);
        assert!(process_image_name(std::process::id()).is_some());
        assert!(memory_bytes().0 > 0);
    }
}
