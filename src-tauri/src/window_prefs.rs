//! The Studio's own window: where it opens (size, position, monitor, maximized or fullscreen,
//! as it was left) and what the close button does (quit, or stay in the tray next to the clock).
//!
//! Kept in `%APPDATA%/PZModStudio/window.json`, read before the window shows.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPrefs {
    /// Opens where and how it was closed.
    #[serde(default = "yes")]
    pub remember_window: bool,
    /// The close button hides it to the tray instead of quitting.
    #[serde(default)]
    pub close_to_tray: bool,
    /// The tray menu's words, in the Studio's language (set by the window).
    #[serde(default)]
    pub tray_open: String,
    #[serde(default)]
    pub tray_quit: String,
    #[serde(default)]
    pub tray_tip: String,
}

fn yes() -> bool {
    true
}

impl Default for WindowPrefs {
    fn default() -> Self {
        WindowPrefs { remember_window: true, close_to_tray: false, tray_open: String::new(), tray_quit: String::new(), tray_tip: String::new() }
    }
}

/// Windows keeps a window's normal bounds whatever its state (maximized, minimized): its
/// "placement". Read and written in screen pixels, frame included.
#[cfg(windows)]
mod placement {
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowPlacement, SetWindowPlacement, SW_HIDE, SW_SHOWMAXIMIZED, WINDOWPLACEMENT};

    fn empty() -> WINDOWPLACEMENT {
        WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            flags: 0,
            showCmd: 0,
            ptMinPosition: POINT { x: 0, y: 0 },
            ptMaxPosition: POINT { x: 0, y: 0 },
            rcNormalPosition: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        }
    }

    /// (x, y, width, height) of the normal bounds, and whether it is maximized now.
    pub fn read(hwnd: isize) -> Option<((i32, i32, u32, u32), bool)> {
        let mut p = empty();
        if unsafe { GetWindowPlacement(hwnd as HWND, &mut p) } == 0 {
            return None;
        }
        let r = p.rcNormalPosition;
        Some(((r.left, r.top, (r.right - r.left).max(0) as u32, (r.bottom - r.top).max(0) as u32), p.showCmd == SW_SHOWMAXIMIZED as u32))
    }

    /// Sets the normal bounds of a hidden window, leaving it hidden.
    pub fn write(hwnd: isize, x: i32, y: i32, width: u32, height: u32) -> bool {
        let mut p = empty();
        if unsafe { GetWindowPlacement(hwnd as HWND, &mut p) } == 0 {
            return false;
        }
        p.showCmd = SW_HIDE as u32;
        p.rcNormalPosition = RECT { left: x, top: y, right: x + width as i32, bottom: y + height as i32 };
        unsafe { SetWindowPlacement(hwnd as HWND, &p) != 0 }
    }
}

fn hwnd_of<R: Runtime>(window: &WebviewWindow<R>) -> Option<isize> {
    #[cfg(windows)]
    {
        window.hwnd().ok().map(|h| h.0 as isize)
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        None
    }
}

/// How the window was left: its normal bounds (not the maximized ones) and its state.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub fullscreen: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Stored {
    #[serde(default)]
    prefs: WindowPrefs,
    #[serde(default)]
    state: Option<WindowState>,
}

static STORED: Mutex<Option<Stored>> = Mutex::new(None);

fn file() -> Option<std::path::PathBuf> {
    dirs_next::config_dir().map(|d| d.join("PZModStudio").join("window.json"))
}

fn with<T>(f: impl FnOnce(&mut Stored) -> T) -> T {
    let mut guard = STORED.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        // A file saved by another editor may start with a byte-order mark.
        *guard = Some(
            file()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
                .unwrap_or_default(),
        );
    }
    f(guard.as_mut().expect("loaded above"))
}

fn save() {
    let text = with(|s| serde_json::to_string_pretty(s).unwrap_or_default());
    if let Some(p) = file() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = crate::load_order::ini_parser::atomic_write(&p, text);
    }
}

pub fn prefs() -> WindowPrefs {
    with(|s| s.prefs.clone())
}

pub fn set_prefs<R: Runtime>(app: &tauri::AppHandle<R>, prefs: WindowPrefs) {
    with(|s| s.prefs = prefs.clone());
    save();
    if prefs.close_to_tray {
        ensure_tray(app);
    } else if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_visible(false);
    }
}

/// Whether a saved rectangle still lands on a monitor that is there (a monitor unplugged since
/// would open the window off screen): at least 100 x 50 px of its top must be on one.
pub fn on_some_monitor(state: &WindowState, monitors: &[(i32, i32, u32, u32)]) -> bool {
    monitors.iter().any(|&(mx, my, mw, mh)| {
        let left = state.x.max(mx);
        let right = (state.x + state.width as i32).min(mx + mw as i32);
        let top_seen = state.y >= my && state.y < my + mh as i32 - 50;
        right - left >= 100 && top_seen
    })
}

/// Puts the window back where it was left, before it shows; maximized or fullscreen comes after
/// it shows (`restore_state`), or Windows forgets the place to go back to.
pub fn restore<R: Runtime>(window: &WebviewWindow<R>) {
    let p = prefs();
    let Some(state) = with(|s| s.state.clone()) else { return };
    if !p.remember_window || state.width < 400 || state.height < 300 {
        return;
    }
    let monitors: Vec<(i32, i32, u32, u32)> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| (m.position().x, m.position().y, m.size().width, m.size().height))
        .collect();
    if on_some_monitor(&state, &monitors) {
        #[cfg(windows)]
        if let Some(h) = hwnd_of(window) {
            if placement::write(h, state.x, state.y, state.width, state.height) {
                return;
            }
        }
        let _ = window.set_size(PhysicalSize::new(state.width, state.height));
        let _ = window.set_position(PhysicalPosition::new(state.x, state.y));
    }
}

/// Maximized or fullscreen, as it was left (once the window shows).
pub fn restore_state<R: Runtime>(window: &WebviewWindow<R>) {
    let Some(state) = with(|s| s.state.clone()) else { return };
    if !prefs().remember_window {
        return;
    }
    if state.fullscreen {
        let _ = window.set_fullscreen(true);
    } else if state.maximized {
        let _ = window.maximize();
    }
}

/// Notes how the window is now (called when it moves, resizes or closes).
pub fn remember<R: Runtime>(window: &WebviewWindow<R>) {
    if window.is_minimized().unwrap_or(false) || !window.is_visible().unwrap_or(true) {
        return;
    }
    let fullscreen = window.is_fullscreen().unwrap_or(false);
    // Windows' placement: the normal bounds even while maximized (no in-between moment to catch).
    #[cfg(windows)]
    if let Some((bounds, maximized)) = hwnd_of(window).and_then(placement::read) {
        if !fullscreen {
            with(|s| {
                s.state = Some(WindowState { x: bounds.0, y: bounds.1, width: bounds.2, height: bounds.3, maximized, fullscreen });
            });
        } else {
            // Fullscreen: the bounds stay those it had before.
            with(|s| s.state.get_or_insert_with(WindowState::default).fullscreen = true);
        }
        return;
    }
    let maximized = window.is_maximized().unwrap_or(false);
    with(|s| {
        let mut state = s.state.clone().unwrap_or_default();
        state.maximized = maximized;
        state.fullscreen = fullscreen;
        // Only the normal bounds: a maximized window restored to its maximized size would cover
        // the screen without being maximized.
        if !maximized && !fullscreen {
            if let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) {
                state.x = pos.x;
                state.y = pos.y;
                state.width = size.width;
                state.height = size.height;
            }
        }
        s.state = Some(state);
    });
}

pub fn persist() {
    save();
}

const TRAY_ID: &str = "pzms-tray";

fn show<R: Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// The window in front of the game, asked for from the game's Mods screen. Windows gives the
/// foreground to a program in the background only right after a key press, so an Alt tap goes
/// first (it does nothing on its own).
pub fn bring_to_front<R: Runtime>(app: &tauri::AppHandle<R>) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_MENU};
        keybd_event(VK_MENU as u8, 0, 0, 0);
        keybd_event(VK_MENU as u8, 0, KEYEVENTF_KEYUP, 0);
    }
    show(app);
}

/// The tray icon next to the clock, made the first time it is needed: click to open, menu to
/// open or quit.
pub fn ensure_tray<R: Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_visible(true);
        return;
    }
    let p = prefs();
    let open_text = if p.tray_open.is_empty() { "Open PZ Mod Studio".to_string() } else { p.tray_open.clone() };
    let quit_text = if p.tray_quit.is_empty() { "Quit".to_string() } else { p.tray_quit.clone() };
    let build = || -> tauri::Result<()> {
        let open = tauri::menu::MenuItem::with_id(app, "open", open_text, true, None::<&str>)?;
        let quit = tauri::menu::MenuItem::with_id(app, "quit", quit_text, true, None::<&str>)?;
        let menu = tauri::menu::Menu::with_items(app, &[&open, &quit])?;
        let mut builder = tauri::tray::TrayIconBuilder::with_id(TRAY_ID)
            .menu(&menu)
            .show_menu_on_left_click(false)
            .tooltip(if p.tray_tip.is_empty() { "PZ Mod Studio" } else { p.tray_tip.as_str() })
            .on_menu_event(|app, event| match event.id.as_ref() {
                "open" => show(app),
                "quit" => {
                    persist();
                    app.exit(0);
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, button_state: tauri::tray::MouseButtonState::Up, .. } = event {
                    show(tray.app_handle());
                }
            });
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        builder.build(app)?;
        Ok(())
    };
    if let Err(e) = build() {
        crate::diagnostics::error(format!("[Window] the tray icon could not be made: {}", e));
    }
}

/// The close button: hides to the tray when the player chose that; otherwise the window closes
/// (and the Studio with it). Either way how it was left is kept.
pub fn on_close_requested<R: Runtime>(window: &WebviewWindow<R>, api: &tauri::CloseRequestApi) {
    remember(window);
    persist();
    if prefs().close_to_tray {
        api.prevent_close();
        ensure_tray(window.app_handle());
        let _ = window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_left_on_an_unplugged_monitor_is_not_put_back_there() {
        let one = [(0, 0, 1920, 1080)];
        let two = [(0, 0, 1920, 1080), (1920, 0, 2560, 1440)];
        let on_second = WindowState { x: 2200, y: 100, width: 1280, height: 800, ..Default::default() };
        assert!(on_some_monitor(&on_second, &two));
        assert!(!on_some_monitor(&on_second, &one));
        let title_off_top = WindowState { x: 100, y: -500, width: 1280, height: 800, ..Default::default() };
        assert!(!on_some_monitor(&title_off_top, &one), "its title bar would be out of reach");
    }
}
