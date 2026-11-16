//! Who has a file open: for "Windows would not allow that", the program that holds the file.
//!
//! Windows refuses to delete, move or overwrite a file another process has open, and the error
//! names no one. The Restart Manager (the API behind Explorer's "the file is open in ...") says
//! which processes hold a set of files. The paths come from the error text itself: the backend's
//! messages name the file or folder they failed on.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One process that holds one of the files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileHolder {
    pub pid: u32,
    /// The program's file name (`java.exe`), or the name Windows gives it when that cannot be read.
    pub exe: String,
    /// "server" (a Project Zomboid server), "game" (the game) or "other".
    pub role: String,
    /// For a server its name; for the game "2" when it is player 2's of a multiplayer test.
    pub label: String,
}

/// How many files of a folder are asked about at most (a mod folder can hold thousands).
const MAX_FILES: usize = 4000;

/// The paths named in an error text that exist on disk. A path runs from its drive letter to
/// the first place where what is left no longer exists: the texts put ": ", " (" or a quote after it.
pub fn paths_in_text(text: &str) -> Vec<PathBuf> {
    let bytes = text.as_bytes();
    let mut out: Vec<PathBuf> = Vec::new();
    let mut i = 0;
    while i + 2 < bytes.len() {
        let is_start = bytes[i].is_ascii_alphabetic() && bytes[i + 1] == b':' && (bytes[i + 2] == b'\\' || bytes[i + 2] == b'/') && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric());
        if !is_start {
            i += 1;
            continue;
        }
        let rest = &text[i..];
        let line_end = rest.find(['\n', '\r', '"', '\'', '`', '<', '>', '|', '*', '?']).unwrap_or(rest.len());
        let candidate = &rest[..line_end];
        // The longest prefix that ends where a path may end and exists.
        let mut cuts: Vec<usize> = candidate.match_indices([':', ' ', ',', ')', ';']).map(|(k, _)| k).filter(|&k| k > 2).collect();
        cuts.push(candidate.len());
        cuts.sort_unstable();
        cuts.dedup();
        let found = cuts.iter().rev().map(|&k| candidate[..k].trim_end_matches(['.', ' '])).find(|p| p.len() > 3 && Path::new(p).exists());
        if let Some(p) = found {
            let advance = p.len();
            let p = PathBuf::from(p);
            if !out.contains(&p) {
                out.push(p);
            }
            i += advance;
        } else {
            i += 3;
        }
    }
    out
}

/// The files to ask about for a path: the file itself, or the files inside a folder.
fn files_of(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .take(MAX_FILES)
        .map(|e| e.into_path())
        .collect()
}

/// The processes holding any of the paths named in `text`, each once.
pub fn holders_in_text(text: &str) -> Vec<FileHolder> {
    let files: Vec<PathBuf> = paths_in_text(text).iter().flat_map(|p| files_of(p)).collect();
    if files.is_empty() {
        return Vec::new();
    }
    let found = processes_holding(&files);
    if found.is_empty() {
        return Vec::new();
    }
    let roles = crate::server_manager::mp_test::roles_of(&found.iter().map(|(pid, _)| *pid).collect::<Vec<_>>());
    found
        .into_iter()
        .map(|(pid, app)| {
            let exe = process_exe_name(pid).unwrap_or(app);
            let (role, label) = roles.get(&pid).cloned().unwrap_or_else(|| ("other".to_string(), String::new()));
            FileHolder { pid, exe, role, label }
        })
        .collect()
}

#[cfg(windows)]
fn processes_holding(files: &[PathBuf]) -> Vec<(u32, String)> {
    use windows_sys::Win32::System::RestartManager::{RmEndSession, RmGetList, RmRegisterResources, RmStartSession, CCH_RM_SESSION_KEY, RM_PROCESS_INFO};
    const ERROR_MORE_DATA: u32 = 234;
    let wide: Vec<Vec<u16>> = files.iter().map(|f| f.as_os_str().to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect()).collect();
    let ptrs: Vec<*const u16> = wide.iter().map(|w| w.as_ptr()).collect();
    let mut out: Vec<(u32, String)> = Vec::new();
    unsafe {
        let mut session: u32 = 0;
        let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
        if RmStartSession(&mut session, 0, key.as_mut_ptr()) != 0 {
            return out;
        }
        if RmRegisterResources(session, ptrs.len() as u32, ptrs.as_ptr(), 0, std::ptr::null(), 0, std::ptr::null()) == 0 {
            let mut needed: u32 = 0;
            let mut count: u32 = 0;
            let mut reasons: u32 = 0;
            let mut infos: Vec<RM_PROCESS_INFO> = Vec::new();
            // Asked once for the count, then with room for it (twice more if more appeared meanwhile).
            for _ in 0..3 {
                count = infos.len() as u32;
                let r = RmGetList(session, &mut needed, &mut count, if infos.is_empty() { std::ptr::null_mut() } else { infos.as_mut_ptr() }, &mut reasons);
                if r == 0 {
                    infos.truncate(count as usize);
                    break;
                }
                if r != ERROR_MORE_DATA {
                    infos.clear();
                    break;
                }
                infos = vec![std::mem::zeroed(); needed as usize + 2];
            }
            let me = std::process::id();
            for info in infos.iter().take(count as usize) {
                let pid = info.Process.dwProcessId;
                if pid == me || out.iter().any(|(p, _)| *p == pid) {
                    continue;
                }
                let end = info.strAppName.iter().position(|&c| c == 0).unwrap_or(info.strAppName.len());
                out.push((pid, String::from_utf16_lossy(&info.strAppName[..end])));
            }
        }
        RmEndSession(session);
    }
    out
}

#[cfg(not(windows))]
fn processes_holding(_files: &[PathBuf]) -> Vec<(u32, String)> {
    Vec::new()
}

#[cfg(windows)]
fn process_exe_name(pid: u32) -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len: u32 = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        full.rsplit(['\\', '/']).next().map(str::to_string)
    }
}

#[cfg(not(windows))]
fn process_exe_name(_pid: u32) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_paths_an_error_names_are_found_with_their_spaces() {
        let dir = std::env::temp_dir().join(format!("pzms holders {}", std::process::id()));
        std::fs::create_dir_all(dir.join("a mod")).unwrap();
        let file = dir.join("a mod").join("mod.info");
        std::fs::write(&file, "id=x").unwrap();
        let text = format!("Cannot delete {}: Access is denied. (os error 5)", file.display());
        assert_eq!(paths_in_text(&text), vec![file.clone()]);
        let quoted = format!("Could not move '{}' to the backups (os error 5)", dir.join("a mod").display());
        assert_eq!(paths_in_text(&quoted), vec![dir.join("a mod")]);
        assert!(paths_in_text("Access is denied. (os error 5)").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Live: a PowerShell of its own holds a file. Ignored in the full run, where PowerShell can be
    /// too slow to start beside the other tests: cargo test --lib a_file_held_open -- --ignored
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn a_file_held_open_names_who_holds_it() {
        let dir = std::env::temp_dir().join(format!("pzms-held-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("held.txt");
        std::fs::write(&file, "x").unwrap();
        // A process of its own holds it (this test's own process is left out on purpose).
        let script = format!("$f=[System.IO.File]::Open('{}','Open','Read','None'); Start-Sleep -Seconds 60; $f.Close()", file.display());
        let mut child = std::process::Command::new("powershell").args(["-NoProfile", "-Command", &script]).spawn().unwrap();
        let mut held = Vec::new();
        // PowerShell can take many seconds to start while the other tests run beside this one.
        for _ in 0..150 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            held = processes_holding(&[file.clone()]);
            if held.iter().any(|(pid, _)| *pid == child.id()) {
                break;
            }
        }
        let state = child.try_wait();
        let _ = child.kill();
        let _ = child.wait();
        assert!(held.iter().any(|(pid, _)| *pid == child.id()), "{:?} child {:?}", held, state);
        assert_eq!(process_exe_name(std::process::id()).map(|n| n.ends_with(".exe")), Some(true));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
