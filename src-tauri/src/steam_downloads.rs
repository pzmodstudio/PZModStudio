//! What Steam is downloading for Project Zomboid right now, from Steam's own logs: which Workshop
//! items (`logs/workshop_log.txt`), in which phase and how many bytes (`logs/content_log.txt`).
//! Steam does not log progress, and it preallocates every file before writing, so the folder's
//! size says nothing either; the bytes Steam's process has written since the job started, against
//! the job's staged size, do. No Steam API: starting it would show the player as in-game.

use serde::Serialize;
use std::path::{Path, PathBuf};

const APP: &str = "108600";

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SteamDownloads {
    /// `idle`, `queued` (subscribed, the job has not started), `preparing`, `downloading` or
    /// `installing` (Steam moves the files into the Workshop folder).
    pub phase: String,
    /// The Workshop ids of the running (or queued) job.
    pub items: Vec<String>,
    /// When the running job started, as the log writes it ("2026-10-30 17:16:08").
    pub started: Option<String>,
    /// What the job unpacks to disk, and what Steam has written of it so far (approximate).
    pub total_bytes: Option<u64>,
    pub done_bytes: Option<u64>,
    pub rate_mbps: Option<f64>,
    /// The last job that ended, so the app can say it is done and read the mods again.
    pub finished: Option<FinishedJob>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FinishedJob {
    pub at: String,
    pub items: Vec<String>,
    pub ok: bool,
    pub result: String,
}

/// The log line's time, `[2026-10-30 17:16:08]`.
fn stamp(line: &str) -> Option<&str> {
    line.strip_prefix('[').and_then(|l| l.get(..19))
}

/// Everything but the bytes, from the two logs' last lines.
pub fn parse(workshop_log: &str, content_log: &str) -> SteamDownloads {
    let tag = format!("[AppID {}]", APP);
    let mut pending: Vec<String> = Vec::new();
    let mut job: Option<(String, Vec<String>)> = None;
    let mut finished: Option<FinishedJob> = None;
    let push = |list: &mut Vec<String>, id: &str| {
        if !id.is_empty() && !list.iter().any(|i| i == id) {
            list.push(id.to_string());
        }
    };
    for line in workshop_log.lines().filter(|l| l.contains(&tag)) {
        let Some(time) = stamp(line) else { continue };
        let item = line
            .split("subscribed item ")
            .nth(1)
            .or_else(|| line.split("Subscribed to item ").nth(1))
            .or_else(|| line.split("changed cached item ").nth(1))
            .map(|rest| rest.split(|c: char| !c.is_ascii_digit()).next().unwrap_or(""));
        if let Some(id) = item {
            match job.as_mut() {
                Some((_, items)) => push(items, id),
                None => push(&mut pending, id),
            }
        } else if line.contains("Starting Workshop download job") {
            let items = std::mem::take(&mut pending);
            job = Some((time.to_string(), items));
        } else if let Some(result) = line.split("Finished Workshop download job : ").nth(1) {
            let items = job.take().map(|(_, items)| items).unwrap_or_else(|| std::mem::take(&mut pending));
            let result = result.trim().to_string();
            finished = Some(FinishedJob { at: time.to_string(), items, ok: result == "No Error", result });
        }
    }
    let mut out = SteamDownloads { phase: "idle".into(), finished, ..Default::default() };
    if let Some((started, items)) = job {
        out.phase = "preparing".into();
        out.items = items;
        let app = format!("AppID {} ", APP);
        for line in content_log.lines() {
            let Some(time) = stamp(line) else { continue };
            if time < started.as_str() {
                continue;
            }
            if let Some(rate) = line.split("Current download rate: ").nth(1) {
                out.rate_mbps = rate.split_whitespace().next().and_then(|r| r.parse().ok());
            }
            if !line.contains(&app) {
                continue;
            }
            if let Some(state) = line.split("Workshop update changed : ").nth(1) {
                out.phase = if state.contains("Committing") {
                    "installing"
                } else if state.contains("Downloading") || state.contains("Staging") {
                    "downloading"
                } else if state.trim() == "None" {
                    // Done on Steam's side; the workshop log says so a moment later.
                    "installing"
                } else {
                    "preparing"
                }
                .into();
            } else if let Some(sizes) = line.split("update started : ").nth(1) {
                // "download 0/244576912, store 0/0, reuse 0/163181, delta 0/0, stage 0/877956846"
                out.total_bytes = sizes
                    .split(", ")
                    .find_map(|part| part.strip_prefix("stage "))
                    .and_then(|s| s.split('/').nth(1))
                    .and_then(|n| n.trim().parse().ok())
                    .filter(|n: &u64| *n > 0);
            }
        }
        out.started = Some(started);
    } else if !pending.is_empty() {
        out.phase = "queued".into();
        out.items = pending;
    }
    out
}

/// Where Steam itself is installed (its `logs` folder): the registry, else above the Workshop folder.
fn steam_root(workshop_dir: &str) -> Option<PathBuf> {
    let has_logs = |p: &Path| p.join("logs").join("workshop_log.txt").is_file();
    if let Some(p) = crate::vfs::steam_path().filter(|p| has_logs(p)) {
        return Some(p);
    }
    Path::new(workshop_dir).ancestors().find(|p| has_logs(p)).map(Path::to_path_buf)
}

/// The last `bytes` of a log, from a line start.
fn tail(path: &Path, bytes: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else { return String::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = f.seek(SeekFrom::Start(len.saturating_sub(bytes)));
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf).into_owned();
    if len > bytes {
        text.split_once('\n').map(|(_, rest)| rest.to_string()).unwrap_or_default()
    } else {
        text
    }
}

/// The job whose bytes are being counted, and what Steam had written when it was first seen.
static BASELINE: std::sync::Mutex<Option<(String, u64)>> = std::sync::Mutex::new(None);
/// What Steam had written at the last reading with no job running.
static BEFORE_JOB: std::sync::Mutex<Option<u64>> = std::sync::Mutex::new(None);

pub fn status(workshop_dir: &str) -> SteamDownloads {
    let Some(root) = steam_root(workshop_dir) else { return SteamDownloads { phase: "idle".into(), ..Default::default() } };
    let logs = root.join("logs");
    let mut out = parse(&tail(&logs.join("workshop_log.txt"), 64 * 1024), &tail(&logs.join("content_log.txt"), 256 * 1024));
    // A subscription with nothing to download (already on disk) never gets a job.
    if out.phase == "queued" {
        out.items.retain(|id| !Path::new(workshop_dir).join(id).is_dir());
        if out.items.is_empty() {
            out.phase = "idle".into();
        }
    }
    // A job Steam never finished (Steam closed mid-download) is not running hours later.
    if out.started.is_some() && !steam_running() {
        out = SteamDownloads { phase: "idle".into(), finished: out.finished, ..Default::default() };
    }
    let mut base = BASELINE.lock().unwrap_or_else(|e| e.into_inner());
    let mut before = BEFORE_JOB.lock().unwrap_or_else(|e| e.into_inner());
    let written = steam_bytes_written();
    match &out.started {
        Some(started) => {
            // Counted from the last reading before the job, not from when the app first saw it
            // (up to one idle poll later), so a big download does not end short.
            if base.as_ref().map(|(s, _)| s != started).unwrap_or(true) {
                if let Some(w) = before.or(written) {
                    *base = Some((started.clone(), w));
                }
            }
            match out.phase.as_str() {
                "downloading" => {
                    if let (Some(w), Some((_, b))) = (written, base.as_ref()) {
                        let since = w.saturating_sub(*b);
                        out.done_bytes = Some(out.total_bytes.map(|t| since.min(t)).unwrap_or(since));
                    }
                }
                "installing" => out.done_bytes = out.total_bytes,
                _ => {}
            }
        }
        None => {
            *base = None;
            *before = written;
        }
    }
    out
}

#[cfg(windows)]
fn steam_pid() -> Option<u32> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = None;
        let mut ok = Process32FirstW(snap, &mut entry) != 0;
        while ok {
            let len = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..len]).eq_ignore_ascii_case("steam.exe") {
                found = Some(entry.th32ProcessID);
                break;
            }
            ok = Process32NextW(snap, &mut entry) != 0;
        }
        CloseHandle(snap);
        found
    }
}

#[cfg(windows)]
fn steam_running() -> bool {
    steam_pid().is_some()
}

/// Everything Steam's process has written to disk since it started.
#[cfg(windows)]
fn steam_bytes_written() -> Option<u64> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{GetProcessIoCounters, OpenProcess, IO_COUNTERS, PROCESS_QUERY_LIMITED_INFORMATION};
    let pid = steam_pid()?;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut io: IO_COUNTERS = std::mem::zeroed();
        let ok = GetProcessIoCounters(handle, &mut io) != 0;
        CloseHandle(handle);
        ok.then_some(io.WriteTransferCount)
    }
}

#[cfg(not(windows))]
fn steam_running() -> bool {
    true
}

#[cfg(not(windows))]
fn steam_bytes_written() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::parse;

    const WORKSHOP: &str = "\
[2026-10-30 17:16:08] [AppID 108600] Subscribed to item 2335368829 @ Fri Oct 30 17:16:08 2026
[2026-10-30 17:16:08] [AppID 108600] Detected workshop change : added subscribed item 2335368829
[2026-10-30 17:16:08] [AppID 108600] Detected workshop change (latest from server) : changed cached item 2335368829, new manifest 2914114881058941615
[2026-10-30 17:16:08] [AppID 108600] Starting Workshop download job (downloading  )
[2026-10-30 17:16:08] [AppID 108600] Workshop update job started while app is running, commit : yes
";
    const CONTENT: &str = "\
[2026-10-30 17:10:00] AppID 108600 update started : download 0/1, store 0/0, reuse 0/0, delta 0/0, stage 0/5
[2026-10-30 17:16:11] AppID 108600 Workshop update changed : Running Update,Preallocating,
[2026-10-30 17:16:12] AppID 108600 update started : download 0/244576912, store 0/0, reuse 0/163181, delta 0/0, stage 0/877956846
[2026-10-30 17:16:12] AppID 108600 Workshop update changed : Running Update,Downloading,Staging,
[2026-10-30 17:16:25] Current download rate: 72.381 Mbps
";

    #[test]
    fn a_running_job_has_its_item_phase_and_size() {
        let s = parse(WORKSHOP, CONTENT);
        assert_eq!(s.phase, "downloading");
        assert_eq!(s.items, vec!["2335368829".to_string()]);
        assert_eq!(s.total_bytes, Some(877956846));
        assert_eq!(s.rate_mbps, Some(72.381));
        assert_eq!(s.started.as_deref(), Some("2026-10-30 17:16:08"));
        let committing = format!("{}[2026-10-30 17:17:58] AppID 108600 Workshop update changed : Running Update,Committing,\n", CONTENT);
        assert_eq!(parse(WORKSHOP, &committing).phase, "installing");
    }

    #[test]
    fn a_finished_job_is_idle_and_remembered() {
        let done = format!("{}[2026-10-30 17:18:18] [AppID 108600] Finished Workshop download job : No Error\n", WORKSHOP);
        let s = parse(&done, CONTENT);
        assert_eq!(s.phase, "idle");
        let f = s.finished.unwrap();
        assert!(f.ok);
        assert_eq!(f.items, vec!["2335368829".to_string()]);
        assert_eq!(f.at, "2026-10-30 17:18:18");
    }

    #[test]
    fn a_subscription_before_its_job_is_queued() {
        let s = parse("[2026-10-30 17:13:58] [AppID 108600] Subscribed to item 3821185516 @ Fri\n[2026-10-30 17:13:58] [AppID 431960] Subscribed to item 1 @ Fri\n", "");
        assert_eq!(s.phase, "queued");
        assert_eq!(s.items, vec!["3821185516".to_string()]);
    }
}
