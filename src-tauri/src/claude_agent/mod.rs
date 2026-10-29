//! Claude Agent game-control layer.
//!
//! Talks to the `Z_ClaudeAgent` companion mod through two files in `<Zomboid>/Lua/`:
//! - `claude_cmd.lua`: first line `--id:<n>`, the rest is Lua that may `return` a value.
//! - `claude_out.txt`: written by the mod as `id:<n>\n<ok|err>\n<serialized result>`.
//!
//! Unlike `Z_PZModStudio_Bridge`, the mod polls from OnFETick / OnTickEvenPaused /
//! OnPreUIDraw, so commands also run in the main menu, new-game screens and while paused.
//!
//! Also holds the Win32 helpers for the game window: find, park (bottom of the z-order,
//! no activation), minimize, show, PrintWindow screenshots and focus hand-back.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const CMD_FILE: &str = "claude_cmd.lua";
pub const OUT_FILE: &str = "claude_out.txt";
pub const MOD_ID: &str = "Z_ClaudeAgent";

const MOD_INFO: &str = include_str!("../../../mods/Z_ClaudeAgent/mod.info");
const MOD_INFO_42: &str = include_str!("../../../mods/Z_ClaudeAgent/42/mod.info");
const CLIENT_LUA: &str = include_str!("../../../mods/Z_ClaudeAgent/42/media/lua/client/ClaudeAgent.lua");

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

// ------------------------------------------------------------------ install

/// Installs the Z_ClaudeAgent companion mod into `<user_zomboid_dir>/mods/Z_ClaudeAgent`.
pub fn install_claude_agent_mod(user_zomboid_dir: &str) -> Result<String, String> {
    let target = Path::new(user_zomboid_dir).join("mods").join(MOD_ID);
    let client_dir = target.join("42").join("media").join("lua").join("client");
    std::fs::create_dir_all(&client_dir)
        .map_err(|e| format!("Failed to create directories for {}: {}", MOD_ID, e))?;
    std::fs::create_dir_all(target.join("common"))
        .map_err(|e| format!("Failed to create common directory for {}: {}", MOD_ID, e))?;

    let writes: [(PathBuf, &str); 3] = [
        (target.join("mod.info"), MOD_INFO),
        (target.join("42").join("mod.info"), MOD_INFO_42),
        (client_dir.join("ClaudeAgent.lua"), CLIENT_LUA),
    ];
    for (path, content) in writes.iter() {
        std::fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    }
    Ok(format!(
        "Claude Agent mod installed at {} (enable '{}' in the game's Mods list to activate it)",
        target.display(),
        MOD_ID
    ))
}

/// Brings an installed Z_ClaudeAgent up to this build's scripts. A copy installed by an older
/// Studio keeps its bugs until replaced (one re-ran the last command at every game start), and
/// nothing is installed for someone who never asked for it.
pub fn refresh_if_installed(user_zomboid_dir: &Path) {
    let target = user_zomboid_dir.join("mods").join(MOD_ID);
    if !target.is_dir() {
        return;
    }
    let lua = target.join("42").join("media").join("lua").join("client").join("ClaudeAgent.lua");
    if std::fs::read_to_string(&lua).is_ok_and(|now| now == CLIENT_LUA) {
        return;
    }
    if let Err(e) = install_claude_agent_mod(&user_zomboid_dir.to_string_lossy()) {
        crate::diagnostics::error(format!("[ClaudeAgent] {}", e));
    }
}

// ------------------------------------------------------------------ lua eval

#[derive(Debug, Clone, Serialize)]
pub struct AgentEvalOutcome {
    pub id: String,
    /// "ok", "err" or "timeout"
    pub status: String,
    /// Raw serialized result written by the mod (or a timeout explanation).
    pub raw: String,
    pub elapsed_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn fresh_id() -> String {
    let n = ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}-{}", now_ms(), std::process::id(), n)
}

/// Quotes a Rust string as a Lua double-quoted literal (safe for Kahlua).
pub fn lua_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\{:03}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Optional string argument as a Lua literal, or `nil`.
pub fn lua_opt(s: Option<&str>) -> String {
    match s {
        Some(v) if !v.is_empty() => lua_quote(v),
        _ => "nil".to_string(),
    }
}

/// Writes the command file atomically (temp file + rename) so the mod never reads half a command.
fn write_command(lua_dir: &Path, text: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(lua_dir).map_err(|e| format!("Failed to create {}: {}", lua_dir.display(), e))?;
    let cmd_path = lua_dir.join(CMD_FILE);
    let tmp_path = lua_dir.join(format!("{}.tmp", CMD_FILE));
    std::fs::write(&tmp_path, text).map_err(|e| format!("Failed to write {}: {}", tmp_path.display(), e))?;
    for _ in 0..25 {
        if std::fs::rename(&tmp_path, &cmd_path).is_ok() {
            return Ok(cmd_path);
        }
        thread::sleep(Duration::from_millis(20));
    }
    // The game may hold the file open for reading; fall back to a direct write.
    let _ = std::fs::remove_file(&tmp_path);
    std::fs::write(&cmd_path, text).map_err(|e| format!("Failed to write {}: {}", cmd_path.display(), e))?;
    Ok(cmd_path)
}

/// Returns (status, raw) when `claude_out.txt` answers `id`.
fn read_answer(out_path: &Path, id: &str) -> Option<(String, String)> {
    let bytes = std::fs::read(out_path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let header = format!("id:{}", id);
    let mut parts = text.splitn(3, '\n');
    let first = parts.next()?.trim_end_matches('\r');
    if first != header {
        return None;
    }
    let status = parts.next().unwrap_or("").trim_end_matches('\r').trim().to_string();
    let raw = parts.next().unwrap_or("").to_string();
    Some((status, raw))
}

/// Runs Lua inside the game through the Z_ClaudeAgent mod and waits for its answer.
pub fn eval_lua(user_zomboid_dir: &str, code: &str, timeout_ms: u64) -> Result<AgentEvalOutcome, String> {
    let lua_dir = Path::new(user_zomboid_dir).join("Lua");
    let out_path = lua_dir.join(OUT_FILE);
    let id = fresh_id();
    let start = Instant::now();
    write_command(&lua_dir, &format!("--id:{}\n{}\n", id, code))?;

    let deadline = start + Duration::from_millis(timeout_ms.max(100));
    while Instant::now() < deadline {
        if let Some((status, raw)) = read_answer(&out_path, &id) {
            // The mod writes the file in one call; re-read once to avoid a torn read.
            thread::sleep(Duration::from_millis(30));
            let (status, raw) = read_answer(&out_path, &id).unwrap_or((status, raw));
            return Ok(AgentEvalOutcome {
                id,
                status,
                raw,
                elapsed_ms: start.elapsed().as_millis() as u64,
            });
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(AgentEvalOutcome {
        id,
        status: "timeout".to_string(),
        raw: "No answer from Z_ClaudeAgent (game not running, still loading, or the mod is not installed/enabled).".to_string(),
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

// ------------------------------------------------------------------ window helpers

#[derive(Debug, Clone, Serialize)]
pub struct GameWindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub client_width: i32,
    pub client_height: i32,
    pub foreground: bool,
    pub iconic: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotInfo {
    pub path: String,
    pub source_width: u32,
    pub source_height: u32,
    pub width: u32,
    pub height: u32,
    pub print_window_ok: bool,
    /// True when every pixel is (nearly) the same color, i.e. the capture probably failed.
    pub looks_blank: bool,
}

#[cfg(windows)]
mod win {
    use super::{GameWindowInfo, ScreenshotInfo};
    use std::collections::HashMap;
    use std::path::Path;
    use std::thread;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::System::Threading::{
        AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, MapVirtualKeyW, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, VK_MENU,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, GetClientRect, GetForegroundWindow, GetWindowPlacement, GetWindowRect,
        GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PostMessageW, SetForegroundWindow,
        WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
        SetWindowPos, ShowWindow, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_RESTORE,
        SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, WINDOWPLACEMENT,
    };

    const GAME_EXE: &str = "projectzomboid64.exe";
    const PW_CLIENTONLY_RENDERFULLCONTENT: u32 = 0x1 | 0x2;

    fn to_hwnd(v: isize) -> HWND {
        v as HWND
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> i32 {
        let list = &mut *(lparam as *mut Vec<isize>);
        list.push(hwnd as isize);
        1
    }

    fn process_exe_name(pid: u32) -> Option<String> {
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
            full.rsplit(['\\', '/']).next().map(|s| s.to_lowercase())
        }
    }

    /// Main window of ProjectZomboid64.exe: the largest visible top-level window of that process
    /// (size taken from the normal placement, so a minimized window is still found).
    pub fn find_game_window() -> Option<isize> {
        let mut all: Vec<isize> = Vec::new();
        unsafe {
            EnumWindows(Some(enum_cb), &mut all as *mut Vec<isize> as LPARAM);
        }
        let mut exe_cache: HashMap<u32, bool> = HashMap::new();
        let mut best: Option<(i64, isize)> = None;
        for h in all {
            let hwnd = to_hwnd(h);
            unsafe {
                if IsWindowVisible(hwnd) == 0 {
                    continue;
                }
                let mut pid: u32 = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                let is_game = *exe_cache
                    .entry(pid)
                    .or_insert_with(|| process_exe_name(pid).map(|n| n == GAME_EXE).unwrap_or(false));
                if !is_game {
                    continue;
                }
                let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
                wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
                let r = if GetWindowPlacement(hwnd, &mut wp) != 0 {
                    wp.rcNormalPosition
                } else {
                    let mut r: RECT = std::mem::zeroed();
                    GetWindowRect(hwnd, &mut r);
                    r
                };
                let area = ((r.right - r.left) as i64) * ((r.bottom - r.top) as i64);
                if area <= 0 {
                    continue;
                }
                if best.map(|(a, _)| area > a).unwrap_or(true) {
                    best = Some((area, h));
                }
            }
        }
        best.map(|(_, h)| h)
    }

    pub fn window_info(h: isize) -> GameWindowInfo {
        let hwnd = to_hwnd(h);
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut r);
            let mut c: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut c);
            let mut buf = [0u16; 512];
            let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            GameWindowInfo {
                hwnd: h,
                title: String::from_utf16_lossy(&buf[..n.max(0) as usize]),
                left: r.left,
                top: r.top,
                right: r.right,
                bottom: r.bottom,
                client_width: c.right - c.left,
                client_height: c.bottom - c.top,
                foreground: GetForegroundWindow() == hwnd,
                iconic: IsIconic(hwnd) != 0,
            }
        }
    }

    pub fn foreground_window() -> isize {
        unsafe { GetForegroundWindow() as isize }
    }

    pub fn is_window(h: isize) -> bool {
        h != 0 && unsafe { IsWindow(to_hwnd(h)) != 0 }
    }

    /// Restores if minimized (minimized GL windows stop presenting) and sends the window to the
    /// bottom of the z-order at (0,0) with the given size, without activating it.
    pub fn park(h: isize, width: i32, height: i32) {
        let hwnd = to_hwnd(h);
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, width, height, SWP_NOACTIVATE);
        }
    }

    pub fn minimize(h: isize) {
        unsafe {
            ShowWindow(to_hwnd(h), SW_SHOWMINNOACTIVE);
        }
    }

    /// Brings a window to the foreground despite SetForegroundWindow restrictions: attach to the
    /// current foreground thread's input queue and tap ALT, then SetForegroundWindow.
    pub fn force_foreground(h: isize) -> bool {
        let hwnd = to_hwnd(h);
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            }
            if GetForegroundWindow() == hwnd {
                return true;
            }
            let fg = GetForegroundWindow();
            let cur_tid = GetCurrentThreadId();
            let fg_tid = if fg.is_null() { 0 } else { GetWindowThreadProcessId(fg, std::ptr::null_mut()) };
            let target_tid = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            let attach_fg = fg_tid != 0 && fg_tid != cur_tid;
            let attach_target = target_tid != 0 && target_tid != cur_tid && target_tid != fg_tid;
            if attach_fg {
                AttachThreadInput(cur_tid, fg_tid, 1);
            }
            if attach_target {
                AttachThreadInput(cur_tid, target_tid, 1);
            }
            // ALT tap: the system then treats this process as having had the last input event.
            keybd_event(VK_MENU as u8, 0, KEYEVENTF_EXTENDEDKEY, 0);
            keybd_event(VK_MENU as u8, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            if attach_target {
                AttachThreadInput(cur_tid, target_tid, 0);
            }
            if attach_fg {
                AttachThreadInput(cur_tid, fg_tid, 0);
            }
            GetForegroundWindow() == hwnd
        }
    }

    /// Posts WM_KEYDOWN for each key (modifiers first), waits `hold_ms`, then WM_KEYUP in reverse
    /// order. lParam carries repeat count 1 and the MapVirtualKeyW scan code. Never focuses the window.
    pub fn send_keys(h: isize, keys: &[(u16, bool)], hold_ms: u64) -> bool {
        let hwnd = to_hwnd(h);
        let lparam = |vk: u16, ext: bool, up: bool| -> isize {
            let sc = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as isize;
            let mut l: isize = 1 | (sc << 16);
            if ext {
                l |= 1 << 24;
            }
            if up {
                l |= (1 << 30) | (1 << 31);
            }
            l
        };
        let mut ok = true;
        for &(vk, ext) in keys {
            ok &= unsafe { PostMessageW(hwnd, WM_KEYDOWN, vk as usize, lparam(vk, ext, false)) } != 0;
            thread::sleep(Duration::from_millis(20));
        }
        thread::sleep(Duration::from_millis(hold_ms));
        for &(vk, ext) in keys.iter().rev() {
            ok &= unsafe { PostMessageW(hwnd, WM_KEYUP, vk as usize, lparam(vk, ext, true)) } != 0;
            thread::sleep(Duration::from_millis(20));
        }
        ok
    }

    /// Posts WM_MOUSEMOVE + WM_LBUTTONDOWN/UP at a fraction of the client area. Never focuses.
    /// Returns the client coordinates clicked.
    pub fn click(h: isize, fx: f64, fy: f64) -> Result<(i32, i32), String> {
        let hwnd = to_hwnd(h);
        unsafe {
            let mut c: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut c);
            if c.right <= 0 || c.bottom <= 0 {
                return Err("Game window has no client area (minimized?).".to_string());
            }
            let x = (c.right as f64 * fx.clamp(0.0, 1.0)) as i32;
            let y = (c.bottom as f64 * fy.clamp(0.0, 1.0)) as i32;
            let lp: isize = (((y as u32 & 0xFFFF) << 16) | (x as u32 & 0xFFFF)) as isize;
            const MK_LBUTTON: usize = 0x0001;
            let mut ok = PostMessageW(hwnd, WM_MOUSEMOVE, 0, lp) != 0;
            ok &= PostMessageW(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, lp) != 0;
            thread::sleep(Duration::from_millis(80));
            ok &= PostMessageW(hwnd, WM_LBUTTONUP, 0, lp) != 0;
            if ok {
                Ok((x, y))
            } else {
                Err("PostMessageW failed for the mouse click.".to_string())
            }
        }
    }

    /// PrintWindow(PW_CLIENTONLY | PW_RENDERFULLCONTENT) capture; works while covered by other windows.
    pub fn screenshot(h: isize, path: &Path, max_width: u32) -> Result<ScreenshotInfo, String> {
        let hwnd = to_hwnd(h);
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE);
                thread::sleep(Duration::from_millis(500));
            }
            let mut c: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut c);
            let w = c.right - c.left;
            let hgt = c.bottom - c.top;
            if w <= 0 || hgt <= 0 {
                return Err("Game window has no client area (minimized or not yet created).".to_string());
            }

            let hdc = GetDC(hwnd);
            if hdc.is_null() {
                return Err("GetDC failed for the game window.".to_string());
            }
            let mdc = CreateCompatibleDC(hdc);
            let bmp = CreateCompatibleBitmap(hdc, w, hgt);
            let old = SelectObject(mdc, bmp);
            let ok = PrintWindow(hwnd, mdc, PW_CLIENTONLY_RENDERFULLCONTENT) != 0;

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -hgt, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            };
            let mut bgra = vec![0u8; (w as usize) * (hgt as usize) * 4];
            SelectObject(mdc, old);
            let lines = GetDIBits(
                mdc,
                bmp,
                0,
                hgt as u32,
                bgra.as_mut_ptr() as *mut core::ffi::c_void,
                &mut bmi,
                DIB_RGB_COLORS,
            );
            DeleteObject(bmp);
            DeleteDC(mdc);
            ReleaseDC(hwnd, hdc);
            if lines == 0 {
                return Err("GetDIBits failed to read the captured bitmap.".to_string());
            }
            super::encode_png(path, &bgra, w as u32, hgt as u32, max_width, ok)
        }
    }

    /// After a launch: wait for the game window, then for a while keep it parked at the bottom of
    /// the z-order and hand focus back to `prev_fg` whenever the booting game steals it.
    pub fn park_after_launch(prev_fg: isize, park_it: bool, keep_focus: bool, width: i32, height: i32) {
        thread::spawn(move || {
            let wait_until = Instant::now() + Duration::from_secs(180);
            let mut found = None;
            while Instant::now() < wait_until {
                if let Some(h) = find_game_window() {
                    found = Some(h);
                    break;
                }
                thread::sleep(Duration::from_millis(250));
            }
            if found.is_none() {
                return;
            }
            let guard_until = Instant::now() + Duration::from_secs(45);
            while Instant::now() < guard_until {
                if let Some(h) = find_game_window() {
                    if park_it {
                        park(h, width, height);
                    }
                    if keep_focus && is_window(prev_fg) && foreground_window() == h {
                        force_foreground(prev_fg);
                    }
                }
                thread::sleep(Duration::from_millis(250));
            }
        });
    }
}

/// Converts a top-down BGRA buffer to RGB (box-filter downscaled to `max_width`) and saves a PNG.
#[allow(dead_code)]
fn encode_png(
    path: &Path,
    bgra: &[u8],
    src_w: u32,
    src_h: u32,
    max_width: u32,
    print_window_ok: bool,
) -> Result<ScreenshotInfo, String> {
    let (dst_w, dst_h) = if max_width > 0 && src_w > max_width {
        (max_width, ((src_h as u64 * max_width as u64) / src_w as u64).max(1) as u32)
    } else {
        (src_w, src_h)
    };

    let mut rgb = vec![0u8; (dst_w as usize) * (dst_h as usize) * 3];
    let (mut min_l, mut max_l) = (255u32, 0u32);
    for dy in 0..dst_h {
        let sy0 = (dy as u64 * src_h as u64 / dst_h as u64) as u32;
        let sy1 = (((dy + 1) as u64 * src_h as u64 / dst_h as u64) as u32).max(sy0 + 1).min(src_h);
        for dx in 0..dst_w {
            let sx0 = (dx as u64 * src_w as u64 / dst_w as u64) as u32;
            let sx1 = (((dx + 1) as u64 * src_w as u64 / dst_w as u64) as u32).max(sx0 + 1).min(src_w);
            let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
            for sy in sy0..sy1 {
                let row = (sy as usize) * (src_w as usize) * 4;
                for sx in sx0..sx1 {
                    let i = row + (sx as usize) * 4;
                    b += bgra[i] as u32;
                    g += bgra[i + 1] as u32;
                    r += bgra[i + 2] as u32;
                    n += 1;
                }
            }
            let n = n.max(1);
            let (r, g, b) = (r / n, g / n, b / n);
            let l = (r * 3 + g * 6 + b) / 10;
            min_l = min_l.min(l);
            max_l = max_l.max(l);
            let o = ((dy as usize) * (dst_w as usize) + dx as usize) * 3;
            rgb[o] = r as u8;
            rgb[o + 1] = g as u8;
            rgb[o + 2] = b as u8;
        }
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
        }
    }
    let file = std::fs::File::create(path).map_err(|e| format!("Failed to create {}: {}", path.display(), e))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), dst_w, dst_h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| format!("PNG header error: {}", e))?;
    writer.write_image_data(&rgb).map_err(|e| format!("PNG encode error: {}", e))?;

    Ok(ScreenshotInfo {
        path: path.to_string_lossy().to_string(),
        source_width: src_w,
        source_height: src_h,
        width: dst_w,
        height: dst_h,
        print_window_ok,
        looks_blank: max_l.saturating_sub(min_l) <= 2,
    })
}

#[cfg(not(windows))]
const NOT_WINDOWS: &str = "Game window control is only supported on Windows.";
const NO_WINDOW: &str = "No Project Zomboid game window found (is ProjectZomboid64.exe running?).";

/// Information about the game's main window.
pub fn game_window_info() -> Result<GameWindowInfo, String> {
    #[cfg(windows)]
    {
        let h = win::find_game_window().ok_or_else(|| NO_WINDOW.to_string())?;
        Ok(win::window_info(h))
    }
    #[cfg(not(windows))]
    {
        Err(NOT_WINDOWS.to_string())
    }
}

/// Applies `mode` ("park" | "minimize" | "show" | "info") to the game window; returns a message and the new info.
pub fn game_window_action(mode: &str, width: i32, height: i32) -> Result<(String, GameWindowInfo), String> {
    #[cfg(windows)]
    {
        let h = win::find_game_window().ok_or_else(|| NO_WINDOW.to_string())?;
        let msg = match mode {
            "info" => "Game window info.".to_string(),
            "park" => {
                win::park(h, width, height);
                format!("Parked behind other windows at 0,0 {}x{} without focus.", width, height)
            }
            "minimize" | "min" => {
                win::minimize(h);
                "Minimized without activation (note: a minimized game may stop rendering; prefer 'park').".to_string()
            }
            "show" => {
                let ok = win::force_foreground(h);
                format!("Shown and brought to the foreground (foreground={}).", ok)
            }
            other => return Err(format!("Unknown mode '{}'. Use park, minimize, show or info.", other)),
        };
        thread::sleep(Duration::from_millis(100));
        Ok((msg, win::window_info(h)))
    }
    #[cfg(not(windows))]
    {
        let _ = (mode, width, height);
        Err(NOT_WINDOWS.to_string())
    }
}

/// PrintWindow capture of the game window to a PNG file.
pub fn capture_game_screenshot(path: &Path, max_width: u32) -> Result<ScreenshotInfo, String> {
    #[cfg(windows)]
    {
        let h = win::find_game_window().ok_or_else(|| NO_WINDOW.to_string())?;
        win::screenshot(h, path, max_width)
    }
    #[cfg(not(windows))]
    {
        let _ = (path, max_width);
        Err(NOT_WINDOWS.to_string())
    }
}

/// Parses "O", "SHIFT+O", "F9", "ESC", "CTRL+ALT+DELETE"... into (virtual key, extended) pairs.
pub fn parse_key_combo(combo: &str) -> Result<Vec<(u16, bool)>, String> {
    let mut keys = Vec::new();
    for raw in combo.split('+') {
        let part = raw.trim().to_uppercase();
        let key: (u16, bool) = match part.as_str() {
            "SHIFT" => (0x10, false),
            "CTRL" | "CONTROL" => (0x11, false),
            "ALT" => (0x12, false),
            "ESC" | "ESCAPE" => (0x1B, false),
            "SPACE" => (0x20, false),
            "TAB" => (0x09, false),
            "ENTER" | "RETURN" => (0x0D, false),
            "BACKSPACE" => (0x08, false),
            "DELETE" | "DEL" => (0x2E, true),
            "INSERT" | "INS" => (0x2D, true),
            "HOME" => (0x24, true),
            "END" => (0x23, true),
            "PAGEUP" | "PGUP" => (0x21, true),
            "PAGEDOWN" | "PGDN" => (0x22, true),
            "LEFT" => (0x25, true),
            "UP" => (0x26, true),
            "RIGHT" => (0x27, true),
            "DOWN" => (0x28, true),
            "SEMICOLON" => (0xBA, false),
            "APOSTROPHE" => (0xDE, false),
            "COMMA" => (0xBC, false),
            "PERIOD" => (0xBE, false),
            "MINUS" => (0xBD, false),
            "EQUALS" => (0xBB, false),
            "TILDE" | "GRAVE" => (0xC0, false),
            p if p.len() >= 2 && p.starts_with('F') && p[1..].parse::<u16>().map(|n| (1..=12).contains(&n)).unwrap_or(false) => {
                (0x6F + p[1..].parse::<u16>().unwrap_or(1), false)
            }
            p if p.len() == 1 && p.chars().all(|c| c.is_ascii_alphanumeric()) => (p.as_bytes()[0] as u16, false),
            _ => return Err(format!("Unknown key '{}' in '{}'", raw.trim(), combo)),
        };
        keys.push(key);
    }
    if keys.is_empty() {
        return Err("Empty key combination.".to_string());
    }
    Ok(keys)
}

/// Posts a key combination to the game window without focusing it.
pub fn send_game_key(combo: &str, hold_ms: u64) -> Result<String, String> {
    let keys = parse_key_combo(combo)?;
    #[cfg(windows)]
    {
        let h = win::find_game_window().ok_or_else(|| NO_WINDOW.to_string())?;
        if win::send_keys(h, &keys, hold_ms) {
            Ok(format!("Posted {} to the game window (held {} ms).", combo, hold_ms))
        } else {
            Err(format!("PostMessageW failed for {}", combo))
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (keys, hold_ms);
        Err(NOT_WINDOWS.to_string())
    }
}

/// Posts a left click at (x_frac, y_frac) of the game window's client area without focusing it.
pub fn click_game_window(x_frac: f64, y_frac: f64) -> Result<(i32, i32), String> {
    #[cfg(windows)]
    {
        let h = win::find_game_window().ok_or_else(|| NO_WINDOW.to_string())?;
        win::click(h, x_frac, y_frac)
    }
    #[cfg(not(windows))]
    {
        let _ = (x_frac, y_frac);
        Err(NOT_WINDOWS.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EnterGameOutcome {
    pub in_game: bool,
    pub clicks: u32,
    pub elapsed_ms: u64,
    /// Last CA.state() answer (raw), if the mod answered at all.
    pub last_state: Option<String>,
}

/// After Continue/Play: waits for the world to load and gets past the black "click to start"
/// screen (GameLoadingState suppresses Lua, so the mod cannot answer there). Polls
/// `return CA.state()` with a 2 s timeout; once the mod has been silent for more than 20 s it
/// posts a centre click about every 3 s. Clicks are only sent while the mod is silent, so nothing
/// in game gets clicked. Stops when the state says `"ingame":true`.
pub fn enter_game(user_zomboid_dir: &str, timeout_ms: u64) -> EnterGameOutcome {
    let start = Instant::now();
    let deadline = start + Duration::from_millis(timeout_ms);
    let mut clicks = 0u32;
    let mut silent_since: Option<Instant> = None;
    let mut last_state = None;
    while Instant::now() < deadline {
        match eval_lua(user_zomboid_dir, "return CA.state()", 2000) {
            Ok(o) if o.status == "ok" => {
                silent_since = None;
                let compact: String = o.raw.chars().filter(|c| !c.is_whitespace()).collect();
                let in_game = compact.contains("\"ingame\":true");
                last_state = Some(o.raw);
                if in_game {
                    return EnterGameOutcome { in_game: true, clicks, elapsed_ms: start.elapsed().as_millis() as u64, last_state };
                }
            }
            Ok(o) if o.status == "timeout" => {
                let since = *silent_since.get_or_insert_with(Instant::now);
                if since.elapsed() > Duration::from_secs(20) && click_game_window(0.5, 0.5).is_ok() {
                    clicks += 1;
                }
            }
            _ => silent_since = None,
        }
        let pause = Duration::from_millis(3000).min(deadline.saturating_duration_since(Instant::now()));
        thread::sleep(pause);
    }
    EnterGameOutcome { in_game: false, clicks, elapsed_ms: start.elapsed().as_millis() as u64, last_state }
}

/// Handle of the current foreground window (0 if none / not Windows).
pub fn current_foreground_window() -> isize {
    #[cfg(windows)]
    {
        win::foreground_window()
    }
    #[cfg(not(windows))]
    {
        0
    }
}

/// Background watcher started after a launch: parks the new game window and/or returns focus.
pub fn start_launch_window_guard(prev_fg: isize, park: bool, keep_focus: bool, width: i32, height: i32) {
    #[cfg(windows)]
    {
        if park || keep_focus {
            win::park_after_launch(prev_fg, park, keep_focus, width, height);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (prev_fg, park, keep_focus, width, height);
    }
}

/// Default screenshot path: `<temp>/pz-mod-studio/game_screenshot.png`.
pub fn default_screenshot_path() -> PathBuf {
    std::env::temp_dir().join("pz-mod-studio").join("game_screenshot.png")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_lua_strings() {
        assert_eq!(lua_quote("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
        assert_eq!(lua_quote("\u{1}"), "\"\\001\"");
        assert_eq!(lua_opt(None), "nil");
    }

    #[test]
    fn parses_key_combos() {
        assert_eq!(parse_key_combo("shift+o").unwrap(), vec![(0x10, false), (b'O' as u16, false)]);
        assert_eq!(parse_key_combo("F9").unwrap(), vec![(0x78, false)]);
        assert_eq!(parse_key_combo("ESC").unwrap(), vec![(0x1B, false)]);
        assert!(parse_key_combo("F13").is_err());
        assert!(parse_key_combo("SHIFT+?").is_err());
    }

    #[test]
    fn parses_answer_file() {
        let dir = std::env::temp_dir().join(format!("pz_ca_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(OUT_FILE);
        std::fs::write(&p, "id:42-1\nok\n{\"a\":1}\nmore").unwrap();
        assert_eq!(read_answer(&p, "42-1"), Some(("ok".to_string(), "{\"a\":1}\nmore".to_string())));
        assert_eq!(read_answer(&p, "42"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
