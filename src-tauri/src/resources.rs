//! What the game (and the Studio's server) take from the PC right now: memory against its Java
//! limit, CPU, and what the PC has left. Read natively (Toolhelp, process counters), cheap enough
//! to ask every few seconds for the meter in the top bar; the multiplayer test's fuller status
//! (logs, states) stays its own.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProcessUsage {
    pub pid: u32,
    /// "game" (ProjectZomboid64.exe) or "server" (the Studio's dedicated server, a java.exe).
    pub kind: String,
    pub ram_mb: u64,
    /// The Java heap limit it runs with (-Xmx), when known: the game's from ProjectZomboid64.json,
    /// the server's from the memory it was started with.
    pub ram_max_mb: Option<u64>,
    /// Share of the whole PC (all cores), since the last reading; None on the first one.
    pub cpu_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Resources {
    pub pc_total_mb: u64,
    pub pc_free_mb: u64,
    pub processes: Vec<ProcessUsage>,
}

/// CPU time of each process at the last reading: (when, 100-ns units of kernel + user time).
static LAST: Mutex<Option<HashMap<u32, (Instant, u64)>>> = Mutex::new(None);

/// CPU share from two readings of a process's CPU time: its time used over the wall time of all cores.
pub fn cpu_share(cpu_before: u64, cpu_now: u64, wall_secs: f64, cores: u32) -> Option<f64> {
    if wall_secs <= 0.0 || cores == 0 || cpu_now < cpu_before {
        return None;
    }
    let used_secs = (cpu_now - cpu_before) as f64 / 10_000_000.0;
    Some(((used_secs / (wall_secs * cores as f64)) * 1000.0).round() / 10.0)
}

pub fn read(paths: &crate::vfs::StudioPaths) -> Resources {
    let (pc_total_mb, pc_free_mb) = pc_memory();
    let game_xmx = std::fs::read_to_string(std::path::Path::new(&paths.pz_install_dir).join("ProjectZomboid64.json"))
        .ok()
        .and_then(|t| crate::server_manager::mp_test::xmx_mb(&t));
    let server = crate::server_manager::get_dedicated_server_status(paths.user_zomboid_dir.clone()).ok().filter(|s| s.is_running);
    let server_pid = server.as_ref().and_then(|s| s.pid);
    let server_max = server.as_ref().and_then(|s| s.memory_gb).map(|gb| gb as u64 * 1024);

    let mut wanted: Vec<(u32, &str, Option<u64>)> = processes_named("ProjectZomboid64.exe").into_iter().map(|pid| (pid, "game", game_xmx)).collect();
    if let Some(pid) = server_pid {
        wanted.push((pid, "server", server_max));
    }
    let cores = std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1);
    let now = Instant::now();
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let before = last.take().unwrap_or_default();
    let mut seen = HashMap::new();
    let mut processes = Vec::new();
    for (pid, kind, ram_max_mb) in wanted {
        let Some((ram, cpu_time)) = process_counters(pid) else { continue };
        let cpu_percent = before.get(&pid).and_then(|(at, t)| cpu_share(*t, cpu_time, now.duration_since(*at).as_secs_f64(), cores));
        seen.insert(pid, (now, cpu_time));
        processes.push(ProcessUsage { pid, kind: kind.to_string(), ram_mb: ram / (1024 * 1024), ram_max_mb, cpu_percent });
    }
    *last = Some(seen);
    Resources { pc_total_mb, pc_free_mb, processes }
}

#[cfg(windows)]
fn pc_memory() -> (u64, u64) {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: a zeroed MEMORYSTATUSEX with its length set, as the call asks.
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return (0, 0);
    }
    (status.ullTotalPhys / (1024 * 1024), status.ullAvailPhys / (1024 * 1024))
}

#[cfg(windows)]
fn processes_named(exe: &str) -> Vec<u32> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    let mut out = Vec::new();
    // SAFETY: the snapshot handle is checked and closed; the entry has its size set before the first call.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snap, &mut entry) != 0;
        while ok {
            let len = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..len]).eq_ignore_ascii_case(exe) {
                out.push(entry.th32ProcessID);
            }
            ok = Process32NextW(snap, &mut entry) != 0;
        }
        CloseHandle(snap);
    }
    out
}

/// A process's working set (bytes) and its CPU time so far (100-ns units).
#[cfg(windows)]
fn process_counters(pid: u32) -> Option<(u64, u64)> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    // SAFETY: the handle is checked and closed; the structs are zeroed with their sizes set.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut mem: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        mem.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let got_mem = GetProcessMemoryInfo(handle, &mut mem, mem.cb) != 0;
        let (mut created, mut exited, mut kernel, mut user): (FILETIME, FILETIME, FILETIME, FILETIME) = std::mem::zeroed();
        let got_times = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) != 0;
        CloseHandle(handle);
        if !got_mem || !got_times {
            return None;
        }
        let ft = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
        Some((mem.WorkingSetSize as u64, ft(kernel) + ft(user)))
    }
}

#[cfg(not(windows))]
fn pc_memory() -> (u64, u64) {
    (0, 0)
}
#[cfg(not(windows))]
fn processes_named(_exe: &str) -> Vec<u32> {
    Vec::new()
}
#[cfg(not(windows))]
fn process_counters(_pid: u32) -> Option<(u64, u64)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_is_the_share_of_every_core() {
        // 1 s of CPU in 1 s of wall time on 4 cores: a quarter of the PC.
        assert_eq!(cpu_share(0, 10_000_000, 1.0, 4), Some(25.0));
        assert_eq!(cpu_share(5, 5, 2.0, 8), Some(0.0));
        assert_eq!(cpu_share(10, 5, 1.0, 4), None, "a restarted process with the same pid");
        assert_eq!(cpu_share(0, 1, 0.0, 4), None);
    }

    #[test]
    fn this_process_can_be_read() {
        let pid = std::process::id();
        let (ram, _) = process_counters(pid).expect("own counters");
        assert!(ram > 0);
        assert!(pc_memory().0 > 0);
    }
}
