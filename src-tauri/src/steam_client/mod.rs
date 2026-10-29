//! Subscribing through the Steam client that is already running, with no API key and no login.
//!
//! Project Zomboid ships `steam_api64.dll`, the same library the game uses to talk to Steam. The
//! Studio loads that copy (nothing is bundled or downloaded), asks the running Steam client to
//! subscribe or unsubscribe, and lets go of it again. Steam itself downloads the files, exactly as
//! if the user had pressed Subscribe on the Workshop page.
//!
//! While the library is loaded Steam counts Project Zomboid as running, so the user shows up as
//! in-game for the second or two a subscription takes. Only one of these may run at a time.

use serde::{Deserialize, Serialize};
use std::ffi::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const PZ_APP_ID: &str = "108600";
/// Steam's own cap on how long a subscription call may take before we stop waiting.
const CALL_TIMEOUT: Duration = Duration::from_secs(20);

/// Only one Steam session at a time, however many windows ask for one.
static STEAM_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscribeOutcome {
    /// Items Steam accepted.
    pub done: Vec<String>,
    /// Items it refused, with the reason.
    pub failed: Vec<(String, String)>,
}

type InitFlat = unsafe extern "C" fn(*mut c_char) -> i32;
type Accessor = unsafe extern "C" fn() -> *mut c_void;
type ItemCall = unsafe extern "C" fn(*mut c_void, u64) -> u64;
type Shutdown = unsafe extern "C" fn();
type RunCallbacks = unsafe extern "C" fn();
type IsCallDone = unsafe extern "C" fn(*mut c_void, u64, *mut bool) -> bool;
type CallResult = unsafe extern "C" fn(*mut c_void, u64, *mut c_void, i32, i32, *mut bool) -> bool;
type CountItems = unsafe extern "C" fn(*mut c_void, bool) -> u32;

/// `steam_api64.dll` as the game installs it.
fn library_path(pz_install_dir: &str) -> Result<PathBuf, String> {
    let file = Path::new(pz_install_dir).join("steam_api64.dll");
    if !file.is_file() {
        return Err(format!(
            "steam_api64.dll was not found in {}. Check your game folder in App Settings.",
            pz_install_dir
        ));
    }
    Ok(file)
}

/// A loaded `steam_api64.dll` with Steam initialised; closes itself when dropped.
struct SteamSession {
    module: isize,
    ugc: *mut c_void,
    utils: *mut c_void,
    shutdown: Shutdown,
    run_callbacks: RunCallbacks,
    is_call_done: IsCallDone,
}

#[cfg(windows)]
mod ffi {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    pub fn load(path: &std::path::Path) -> Result<isize, String> {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
        if handle.is_null() {
            return Err(format!("{} could not be loaded ({}).", path.display(), std::io::Error::last_os_error()));
        }
        Ok(handle as isize)
    }

    pub fn symbol(module: isize, name: &str) -> Result<*const c_void, String> {
        let c_name = std::ffi::CString::new(name).map_err(|_| "bad symbol name".to_string())?;
        let address = unsafe { GetProcAddress(module as _, c_name.as_ptr() as *const u8) };
        match address {
            Some(a) => Ok(a as *const c_void),
            None => Err(format!("{} is missing from steam_api64.dll; your game install may be older.", name)),
        }
    }

    pub fn unload(module: isize) {
        unsafe { FreeLibrary(module as _) };
    }

    use std::os::windows::ffi::OsStrExt;
}

#[cfg(not(windows))]
mod ffi {
    use std::ffi::c_void;
    pub fn load(_path: &std::path::Path) -> Result<isize, String> {
        Err("Subscribing through the Steam client only works on Windows.".to_string())
    }
    pub fn symbol(_module: isize, _name: &str) -> Result<*const c_void, String> {
        Err("not Windows".to_string())
    }
    pub fn unload(_module: isize) {}
}

impl SteamSession {
    /// Loads the game's own Steam library and attaches to the running Steam client.
    fn open(pz_install_dir: &str) -> Result<Self, String> {
        let path = library_path(pz_install_dir)?;
        let module = ffi::load(&path)?;

        let close = |module: isize, e: String| {
            ffi::unload(module);
            e
        };
        let get = |name: &str| ffi::symbol(module, name);

        // Steam has to know which game is asking. Set before init, as the SDK expects.
        std::env::set_var("SteamAppId", PZ_APP_ID);
        std::env::set_var("SteamGameId", PZ_APP_ID);

        let init: InitFlat = match get("SteamAPI_InitFlat") {
            Ok(a) => unsafe { std::mem::transmute(a) },
            Err(e) => return Err(close(module, e)),
        };
        let mut message = [0 as c_char; 1024];
        let result = unsafe { init(message.as_mut_ptr()) };
        if result != 0 {
            let text = unsafe { std::ffi::CStr::from_ptr(message.as_ptr()) }.to_string_lossy().to_string();
            ffi::unload(module);
            return Err(match result {
                // 2 = no Steam client, 3 = this account does not own the game.
                2 => "Steam is not running. Start Steam, sign in, and try again.".to_string(),
                3 => "This Steam account does not own Project Zomboid.".to_string(),
                _ if !text.is_empty() => format!("Steam refused the connection: {}", text),
                _ => format!("Steam refused the connection (code {}).", result),
            });
        }

        let session = |name: &str| -> Result<*mut c_void, String> {
            let accessor: Accessor = unsafe { std::mem::transmute(get(name)?) };
            let pointer = unsafe { accessor() };
            if pointer.is_null() {
                return Err(format!("Steam did not hand over {}.", name));
            }
            Ok(pointer)
        };
        let shutdown: Shutdown = match get("SteamAPI_Shutdown") {
            Ok(a) => unsafe { std::mem::transmute(a) },
            Err(e) => return Err(close(module, e)),
        };
        let finish = |e: String| {
            unsafe { shutdown() };
            ffi::unload(module);
            e
        };
        let ugc = match session("SteamAPI_SteamUGC_v021") {
            Ok(p) => p,
            Err(e) => return Err(finish(e)),
        };
        let utils = match session("SteamAPI_SteamUtils_v010") {
            Ok(p) => p,
            Err(e) => return Err(finish(e)),
        };
        let run_callbacks: RunCallbacks = match get("SteamAPI_RunCallbacks") {
            Ok(a) => unsafe { std::mem::transmute(a) },
            Err(e) => return Err(finish(e)),
        };
        let is_call_done: IsCallDone = match get("SteamAPI_ISteamUtils_IsAPICallCompleted") {
            Ok(a) => unsafe { std::mem::transmute(a) },
            Err(e) => return Err(finish(e)),
        };
        Ok(SteamSession { module, ugc, utils, shutdown, run_callbacks, is_call_done })
    }

    /// Waits for one Steam call to come back, pumping callbacks meanwhile.
    fn wait_for(&self, call: u64) -> Result<(), String> {
        if call == 0 {
            return Err("Steam would not take the request.".to_string());
        }
        let deadline = Instant::now() + CALL_TIMEOUT;
        while Instant::now() < deadline {
            unsafe { (self.run_callbacks)() };
            let mut failed = false;
            if unsafe { (self.is_call_done)(self.utils, call, &mut failed) } {
                return if failed { Err("Steam reported the request failed.".to_string()) } else { Ok(()) };
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err("Steam did not answer in time.".to_string())
    }

    /// Subscribes or unsubscribes one item and reads Steam's verdict: a call that completes can
    /// still be refused (a removed or private item), and that is not a success.
    fn call_per_item(&self, name: &str, id: u64, callback_id: i32) -> Result<(), String> {
        let function: ItemCall = unsafe { std::mem::transmute(ffi::symbol(self.module, name)?) };
        let call = unsafe { function(self.ugc, id) };
        self.wait_for(call)?;
        let Ok(get) = ffi::symbol(self.module, "SteamAPI_ISteamUtils_GetAPICallResult") else {
            return Ok(());
        };
        let get: CallResult = unsafe { std::mem::transmute(get) };
        // RemoteStorage(Un)SubscribePublishedFileResult_t: EResult, then the item id (pack 8).
        let mut result = [0u8; 16];
        let mut failed = false;
        let read = unsafe { get(self.utils, call, result.as_mut_ptr().cast(), result.len() as i32, callback_id, &mut failed) };
        if !read || failed {
            return Ok(()); // Steam completed it and would not say more: as before.
        }
        let e_result = i32::from_le_bytes([result[0], result[1], result[2], result[3]]);
        match e_result {
            1 => Ok(()),
            9 => Err("Steam says this item does not exist (removed or private).".to_string()),
            15 => Err("Steam refused it: access denied (private or hidden item).".to_string()),
            n => Err(format!("Steam refused it (result {}).", n)),
        }
    }

    /// How many Workshop items this account is subscribed to (a read, to prove the link works).
    fn subscribed_count(&self) -> Result<u32, String> {
        let count: CountItems = unsafe { std::mem::transmute(ffi::symbol(self.module, "SteamAPI_ISteamUGC_GetNumSubscribedItems")?) };
        Ok(unsafe { count(self.ugc, true) })
    }
}

impl Drop for SteamSession {
    fn drop(&mut self) {
        unsafe { (self.shutdown)() };
        ffi::unload(self.module);
    }
}

/// Subscribes or unsubscribes the signed-in Steam account, through the running Steam client.
/// Steam downloads or removes the files itself afterwards.
pub fn set_subscriptions(pz_install_dir: &str, ids: &[String], subscribe: bool) -> Result<SubscribeOutcome, String> {
    let _only_one = STEAM_LOCK.lock().map_err(|_| "Another subscription is still running.".to_string())?;
    let numbers: Vec<(String, u64)> = ids
        .iter()
        .filter_map(|id| id.trim().parse::<u64>().ok().map(|n| (id.clone(), n)))
        .collect();
    if numbers.is_empty() {
        return Err("No Workshop item to subscribe to.".to_string());
    }
    let session = SteamSession::open(pz_install_dir)?;
    // Callback ids: k_iSteamRemoteStorageCallbacks (1300) + 13 / + 15.
    let (name, callback_id) = if subscribe {
        ("SteamAPI_ISteamUGC_SubscribeItem", 1313)
    } else {
        ("SteamAPI_ISteamUGC_UnsubscribeItem", 1315)
    };
    let mut outcome = SubscribeOutcome::default();
    for (text, id) in numbers {
        match session.call_per_item(name, id, callback_id) {
            Ok(()) => outcome.done.push(text),
            Err(e) => outcome.failed.push((text, e)),
        }
    }
    Ok(outcome)
}

/// What an upload to the Workshop carries.
pub struct PublishRequest {
    /// None: a new item is created first.
    pub item_id: Option<u64>,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    /// 0 public, 1 friends only, 2 private, 3 unlisted (ERemoteStoragePublishedFileVisibility).
    pub visibility: i32,
    /// The folder whose contents Steam uploads.
    pub content_dir: PathBuf,
    pub preview: Option<PathBuf>,
    pub change_note: String,
}

/// `SteamParamStringArray_t`.
#[repr(C)]
struct SteamParamStringArray {
    strings: *const *const c_char,
    count: i32,
}

type CreateItem = unsafe extern "C" fn(*mut c_void, u32, i32) -> u64;
type StartUpdate = unsafe extern "C" fn(*mut c_void, u32, u64) -> u64;
type SetText = unsafe extern "C" fn(*mut c_void, u64, *const c_char) -> bool;
type SetVisibility = unsafe extern "C" fn(*mut c_void, u64, i32) -> bool;
type SetTags = unsafe extern "C" fn(*mut c_void, u64, *const SteamParamStringArray, bool) -> bool;
type Submit = unsafe extern "C" fn(*mut c_void, u64, *const c_char) -> u64;
type UpdateProgress = unsafe extern "C" fn(*mut c_void, u64, *mut u64, *mut u64) -> i32;

const APP_ID: u32 = 108600;
/// An upload of a big mod on a slow line.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(45 * 60);
/// Steam's limit on a Workshop description.
const DESCRIPTION_LIMIT: usize = 8000;

/// Steam's `EResult`, in words.
fn e_result_text(n: i32) -> String {
    match n {
        1 => "OK".to_string(),
        2 => "a generic failure on Steam's side".to_string(),
        3 => "no connection to Steam".to_string(),
        8 => "an invalid parameter (a tag, the visibility or a path Steam does not take)".to_string(),
        9 => "a file was not found".to_string(),
        10 => "Steam is busy".to_string(),
        12 => "an invalid name".to_string(),
        14 => "a duplicate name".to_string(),
        15 => "access denied (not your item, or the account may not publish)".to_string(),
        16 => "a timeout".to_string(),
        17 => "the account is banned from the Workshop".to_string(),
        20 => "the Workshop service is unavailable right now".to_string(),
        21 => "the Steam client is not logged on".to_string(),
        24 => "insufficient privilege (a limited account cannot publish)".to_string(),
        25 => "a limit exceeded (the item is too big, or too many items)".to_string(),
        26 => "the item was revoked".to_string(),
        n => format!("Steam result {}", n),
    }
}

impl SteamSession {
    /// The result struct of a completed call, `size` bytes of it.
    fn call_result(&self, call: u64, callback_id: i32, size: usize) -> Result<Vec<u8>, String> {
        let get: CallResult = unsafe { std::mem::transmute(ffi::symbol(self.module, "SteamAPI_ISteamUtils_GetAPICallResult")?) };
        let mut buf = vec![0u8; size];
        let mut failed = false;
        let read = unsafe { get(self.utils, call, buf.as_mut_ptr().cast(), size as i32, callback_id, &mut failed) };
        if !read || failed {
            return Err("Steam completed the request but would not hand over the result.".to_string());
        }
        Ok(buf)
    }

    /// Waits for a long call (an upload), pumping callbacks and calling `tick` between looks.
    fn wait_long(&self, call: u64, timeout: Duration, mut tick: impl FnMut()) -> Result<(), String> {
        if call == 0 {
            return Err("Steam would not take the request.".to_string());
        }
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            unsafe { (self.run_callbacks)() };
            let mut failed = false;
            if unsafe { (self.is_call_done)(self.utils, call, &mut failed) } {
                return if failed { Err("Steam reported the request failed.".to_string()) } else { Ok(()) };
            }
            tick();
            std::thread::sleep(Duration::from_millis(250));
        }
        Err("Steam did not finish in time.".to_string())
    }
}

fn c_string(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s.replace('\0', "")).unwrap_or_default()
}

/// Publishes an item through the running Steam client: `CreateItem` when it has no id yet
/// (`on_created` gets the new id at once, before the upload), then `StartItemUpdate`, the
/// title, description, visibility, tags, content folder and preview, and `SubmitItemUpdate`,
/// with `progress(stage, bytes done, bytes in all)` while it runs. Returns the item id,
/// whether Steam wants the Workshop legal agreement accepted on its site, and whether the item
/// was made now.
pub fn publish_item(pz_install_dir: &str, req: PublishRequest, on_created: &mut dyn FnMut(u64), mut progress: impl FnMut(&str, u64, u64)) -> Result<(u64, bool, bool), String> {
    let _only_one = STEAM_LOCK.lock().map_err(|_| "Another Steam request is still running.".to_string())?;
    if req.description.chars().count() > DESCRIPTION_LIMIT {
        return Err(format!("The description is {} characters; Steam takes at most {}.", req.description.chars().count(), DESCRIPTION_LIMIT));
    }
    if req.title.trim().is_empty() {
        return Err("The item has no title.".to_string());
    }
    if !req.content_dir.is_dir() {
        return Err("The content folder is not there.".to_string());
    }
    let session = SteamSession::open(pz_install_dir)?;
    let get = |name: &str| ffi::symbol(session.module, name);
    let mut legal = false;
    let (id, created) = match req.item_id {
        Some(id) => (id, false),
        None => {
            progress("creating", 0, 0);
            let create: CreateItem = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_CreateItem")?) };
            // k_EWorkshopFileTypeCommunity = 0
            let call = unsafe { create(session.ugc, APP_ID, 0) };
            session.wait_for(call)?;
            // CreateItemResult_t: EResult, (pad), PublishedFileId_t at 8, legal-agreement bool at 16 (pack 8).
            let r = session.call_result(call, 1303, 24)?;
            let e = i32::from_le_bytes([r[0], r[1], r[2], r[3]]);
            if e != 1 {
                return Err(format!("Steam would not create the item: {}.", e_result_text(e)));
            }
            let id = u64::from_le_bytes([r[8], r[9], r[10], r[11], r[12], r[13], r[14], r[15]]);
            legal = r[16] != 0;
            if id == 0 {
                return Err("Steam created no item.".to_string());
            }
            on_created(id);
            (id, true)
        }
    };
    progress("preparing", 0, 0);
    let start: StartUpdate = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_StartItemUpdate")?) };
    let handle = unsafe { start(session.ugc, APP_ID, id) };
    if handle == u64::MAX || handle == 0 {
        return Err("Steam would not start the update (is the item yours?).".to_string());
    }
    let set_text = |name: &str, what: &str, value: &str| -> Result<(), String> {
        let f: SetText = unsafe { std::mem::transmute(get(name)?) };
        let c = c_string(value);
        if unsafe { f(session.ugc, handle, c.as_ptr()) } {
            Ok(())
        } else {
            Err(format!("Steam refused the {}.", what))
        }
    };
    set_text("SteamAPI_ISteamUGC_SetItemTitle", "title", req.title.trim())?;
    set_text("SteamAPI_ISteamUGC_SetItemDescription", "description", &req.description)?;
    set_text("SteamAPI_ISteamUGC_SetItemContent", "content folder", &req.content_dir.to_string_lossy())?;
    if let Some(preview) = &req.preview {
        set_text("SteamAPI_ISteamUGC_SetItemPreview", "preview picture", &preview.to_string_lossy())?;
    }
    let set_visibility: SetVisibility = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_SetItemVisibility")?) };
    if !unsafe { set_visibility(session.ugc, handle, req.visibility) } {
        return Err("Steam refused the visibility.".to_string());
    }
    let tags: Vec<std::ffi::CString> = req.tags.iter().map(|t| c_string(t)).collect();
    let pointers: Vec<*const c_char> = tags.iter().map(|t| t.as_ptr()).collect();
    let array = SteamParamStringArray { strings: pointers.as_ptr(), count: pointers.len() as i32 };
    let set_tags: SetTags = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_SetItemTags")?) };
    if !unsafe { set_tags(session.ugc, handle, &array, false) } {
        return Err("Steam refused the tags (only letters, digits and spaces; no commas).".to_string());
    }
    let submit: Submit = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_SubmitItemUpdate")?) };
    let note = c_string(&req.change_note);
    let call = unsafe { submit(session.ugc, handle, note.as_ptr()) };
    let update_progress: UpdateProgress = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_GetItemUpdateProgress")?) };
    session.wait_long(call, UPLOAD_TIMEOUT, || {
        let (mut done, mut total) = (0u64, 0u64);
        let status = unsafe { update_progress(session.ugc, handle, &mut done, &mut total) };
        // EItemUpdateStatus: 1 preparing config, 2 preparing content, 3 uploading content, 4 uploading preview, 5 committing.
        let stage = match status {
            1 | 2 => "preparing",
            3 => "uploading",
            4 => "preview",
            5 => "committing",
            _ => "uploading",
        };
        progress(stage, done, total);
    })?;
    // SubmitItemUpdateResult_t: EResult, legal-agreement bool at 4, PublishedFileId_t at 8 (pack 8).
    let r = session.call_result(call, 1304, 16)?;
    let e = i32::from_le_bytes([r[0], r[1], r[2], r[3]]);
    legal |= r[4] != 0;
    if e != 1 {
        return Err(format!("Steam refused the upload: {}.", e_result_text(e)));
    }
    Ok((id, legal, created))
}

#[cfg(test)]
mod publish_tests {
    use super::*;

    /// Live, nothing sent: `cargo test --lib -- --ignored live_steam_update_handle --nocapture`.
    /// Opens the Steam session, starts an update on the player's own item and sets the title,
    /// tags and visibility, then drops it without SubmitItemUpdate: Steam changes nothing, and
    /// every flat-API symbol the upload needs is proven to be in the DLL.
    #[test]
    #[ignore]
    fn live_steam_update_handle() {
        let paths = crate::vfs::studio_paths();
        let session = SteamSession::open(&paths.pz_install_dir).unwrap();
        let get = |name: &str| ffi::symbol(session.module, name);
        for name in [
            "SteamAPI_ISteamUGC_CreateItem", "SteamAPI_ISteamUGC_StartItemUpdate", "SteamAPI_ISteamUGC_SetItemTitle", "SteamAPI_ISteamUGC_SetItemDescription",
            "SteamAPI_ISteamUGC_SetItemContent", "SteamAPI_ISteamUGC_SetItemPreview", "SteamAPI_ISteamUGC_SetItemVisibility", "SteamAPI_ISteamUGC_SetItemTags",
            "SteamAPI_ISteamUGC_SubmitItemUpdate", "SteamAPI_ISteamUGC_GetItemUpdateProgress",
        ] {
            assert!(get(name).is_ok(), "{} missing", name);
        }
        let start: StartUpdate = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_StartItemUpdate").unwrap()) };
        let handle = unsafe { start(session.ugc, APP_ID, 3824256253) };
        println!("update handle {:#x}", handle);
        assert!(handle != u64::MAX && handle != 0);
        let set_title: SetText = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_SetItemTitle").unwrap()) };
        let t = c_string("PZ Mod Manager Studio Link");
        assert!(unsafe { set_title(session.ugc, handle, t.as_ptr()) });
        let set_visibility: SetVisibility = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_SetItemVisibility").unwrap()) };
        assert!(unsafe { set_visibility(session.ugc, handle, 0) });
        let tags: Vec<std::ffi::CString> = ["Build 42", "Framework"].iter().map(|t| c_string(t)).collect();
        let pointers: Vec<*const c_char> = tags.iter().map(|t| t.as_ptr()).collect();
        let array = SteamParamStringArray { strings: pointers.as_ptr(), count: pointers.len() as i32 };
        let set_tags: SetTags = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_SetItemTags").unwrap()) };
        assert!(unsafe { set_tags(session.ugc, handle, &array, false) });
        let update_progress: UpdateProgress = unsafe { std::mem::transmute(get("SteamAPI_ISteamUGC_GetItemUpdateProgress").unwrap()) };
        let (mut done, mut total) = (0u64, 0u64);
        let status = unsafe { update_progress(session.ugc, handle, &mut done, &mut total) };
        println!("status before submit: {} ({} / {})", status, done, total);
        // dropped here: nothing submitted
    }
}

/// `servernetadr_t` + `gameserveritem_t` of the Steamworks SDK (matchmakingtypes.h), x64 MSVC
/// layout: 376 bytes. Only read, never written.
#[repr(C)]
struct GameServerItem {
    connection_port: u16,
    query_port: u16,
    ip: u32,
    ping: i32,
    had_successful_response: bool,
    do_not_refresh: bool,
    game_dir: [u8; 32],
    map: [u8; 32],
    game_description: [u8; 64],
    app_id: u32,
    players: i32,
    max_players: i32,
    bot_players: i32,
    password: bool,
    secure: bool,
    time_last_played: u32,
    server_version: i32,
    server_name: [u8; 64],
    game_tags: [u8; 128],
    steam_id: u64,
}

fn c_text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

/// One server Steam's list returned, as the browser shows it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListedServer {
    pub name: String,
    pub ip: String,
    pub game_port: u16,
    pub query_port: u16,
    pub ping: i32,
    pub players: i32,
    pub max_players: i32,
    pub map: String,
    pub tags: String,
    pub password: bool,
    pub secure: bool,
}

/// The do-nothing listener Steam's server list calls back into (ISteamMatchmakingServerListResponse:
/// ServerResponded, ServerFailedToRespond, RefreshComplete). The list is read by polling instead.
#[repr(C)]
struct ListListener {
    vtable: *const [unsafe extern "C" fn(*mut c_void, *mut c_void, i32); 3],
}
unsafe extern "C" fn ignore(_this: *mut c_void, _request: *mut c_void, _value: i32) {}
static LISTENER_VTABLE: [unsafe extern "C" fn(*mut c_void, *mut c_void, i32); 3] = [ignore, ignore, ignore];

/// Steam's internet list of Project Zomboid servers, through the running Steam client: no key.
/// Steam lists every address at once (up to 10 000) and the servers answer over the next
/// minute or so, as in the game's own browser; each second the ones that answered since are
/// handed to `batch`. Ends when Steam is done, when nothing new came for 6 s, or after 90 s.
pub fn internet_servers(pz_install_dir: &str, mut batch: impl FnMut(Vec<ListedServer>)) -> Result<usize, String> {
    assert_eq!(std::mem::size_of::<GameServerItem>(), 376);
    let _only_one = STEAM_LOCK.lock().map_err(|_| "Another Steam request is still running.".to_string())?;
    let session = SteamSession::open(pz_install_dir)?;
    let get = |name: &str| ffi::symbol(session.module, name);
    let accessor: Accessor = unsafe { std::mem::transmute(get("SteamAPI_SteamMatchmakingServers_v002")?) };
    let servers = unsafe { accessor() };
    if servers.is_null() {
        return Err("Steam did not hand over its server list.".to_string());
    }
    type Request = unsafe extern "C" fn(*mut c_void, u32, *mut *mut c_void, u32, *mut c_void) -> *mut c_void;
    type Count = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
    type Details = unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> *const GameServerItem;
    type Refreshing = unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool;
    type Release = unsafe extern "C" fn(*mut c_void, *mut c_void);
    let request: Request = unsafe { std::mem::transmute(get("SteamAPI_ISteamMatchmakingServers_RequestInternetServerList")?) };
    let count: Count = unsafe { std::mem::transmute(get("SteamAPI_ISteamMatchmakingServers_GetServerCount")?) };
    let details: Details = unsafe { std::mem::transmute(get("SteamAPI_ISteamMatchmakingServers_GetServerDetails")?) };
    let refreshing: Refreshing = unsafe { std::mem::transmute(get("SteamAPI_ISteamMatchmakingServers_IsRefreshing")?) };
    let release: Release = unsafe { std::mem::transmute(get("SteamAPI_ISteamMatchmakingServers_ReleaseRequest")?) };

    let mut listener = Box::new(ListListener { vtable: &LISTENER_VTABLE });
    let app: u32 = PZ_APP_ID.parse().unwrap_or(108600);
    let handle = unsafe { request(servers, app, std::ptr::null_mut(), 0, &mut *listener as *mut ListListener as *mut c_void) };
    if handle.is_null() {
        return Err("Steam would not start the server list.".to_string());
    }
    let mut sent = vec![false; 0];
    let mut total = 0usize;
    let started = Instant::now();
    let mut last_new = Instant::now();
    let mut next_look = Instant::now();
    loop {
        unsafe { (session.run_callbacks)() };
        let still = unsafe { refreshing(servers, handle) };
        if Instant::now() >= next_look || !still {
            next_look = Instant::now() + Duration::from_secs(1);
            let n = unsafe { count(servers, handle) }.max(0) as usize;
            if sent.len() < n {
                sent.resize(n, false);
            }
            let mut fresh = Vec::new();
            for i in 0..n {
                if sent[i] {
                    continue;
                }
                let item = unsafe { details(servers, handle, i as i32) };
                if item.is_null() {
                    continue;
                }
                let s = unsafe { &*item };
                if !s.had_successful_response {
                    continue;
                }
                sent[i] = true;
                if s.app_id != 0 && s.app_id != app {
                    continue;
                }
                fresh.push(ListedServer {
                    name: c_text(&s.server_name),
                    ip: std::net::Ipv4Addr::from(s.ip).to_string(),
                    game_port: s.connection_port,
                    query_port: s.query_port,
                    ping: s.ping,
                    players: s.players,
                    max_players: s.max_players,
                    map: c_text(&s.map),
                    tags: c_text(&s.game_tags),
                    password: s.password,
                    secure: s.secure,
                });
            }
            if !fresh.is_empty() {
                total += fresh.len();
                last_new = Instant::now();
                batch(fresh);
            }
            let quiet = total > 0 && last_new.elapsed() > Duration::from_secs(6);
            if !still || quiet || started.elapsed() > Duration::from_secs(90) {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    unsafe { release(servers, handle) };
    drop(listener);
    Ok(total)
}

/// Whether the Steam client can be reached right now, and how many items it has subscribed.
/// Starts Steam's API, so it belongs in a process that does nothing else afterwards.
pub fn check(pz_install_dir: &str) -> Result<u32, String> {
    let _only_one = STEAM_LOCK.lock().map_err(|_| "Another subscription is still running.".to_string())?;
    SteamSession::open(pz_install_dir)?.subscribed_count()
}

/// Is subscribing from here possible: the game's library is there and Steam is signed in?
///
/// This only asks the library whether a Steam client is running. It deliberately does not start
/// Steam's API, both because that would show the user as in-game for no reason and because the
/// API may only be started once in a process: that one start belongs to the subscription itself.
pub fn is_ready(pz_install_dir: &str) -> bool {
    let Ok(path) = library_path(pz_install_dir) else { return false };
    let Ok(module) = ffi::load(&path) else { return false };
    let running = match ffi::symbol(module, "SteamAPI_IsSteamRunning") {
        Ok(address) => {
            let is_running: unsafe extern "C" fn() -> bool = unsafe { std::mem::transmute(address) };
            unsafe { is_running() }
        }
        Err(_) => false,
    };
    ffi::unload(module);
    running
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn says_where_the_library_should_be() {
        let err = library_path("Z:\\nowhere").unwrap_err();
        assert!(err.contains("steam_api64.dll") && err.contains("App Settings"));
    }

    #[test]
    fn ids_must_be_numbers() {
        let err = set_subscriptions("Z:\\nowhere", &["not an id".to_string()], true).unwrap_err();
        assert!(err.contains("No Workshop item"), "{}", err);
    }

    /// Reads from the running Steam client without changing anything. Steam must be signed in.
    /// PZMS_PZ_DIR=... cargo test steam_client_reads -- --ignored --nocapture
    #[test]
    #[ignore]
    fn steam_client_reads_subscriptions() {
        let dir = std::env::var("PZMS_PZ_DIR").expect("Set PZMS_PZ_DIR to your Project Zomboid folder.");
        match check(&dir) {
            Ok(count) => println!("Steam answered: {} subscribed Workshop items.", count),
            Err(e) => panic!("Could not reach Steam: {e}"),
        }
    }

    /// Subscribes to one small mod and unsubscribes again, through the Steam client.
    /// PZMS_PZ_DIR=... PZMS_TEST_ITEM=3041733782 cargo test steam_client_subscribes -- --ignored --nocapture
    #[test]
    #[ignore]
    fn steam_client_subscribes_and_undoes_it() {
        let dir = std::env::var("PZMS_PZ_DIR").expect("Set PZMS_PZ_DIR to your Project Zomboid folder.");
        let item = std::env::var("PZMS_TEST_ITEM").unwrap_or_else(|_| "3041733782".to_string());
        let ids = vec![item.clone()];
        println!("Subscribing to {item}: {:?}", set_subscriptions(&dir, &ids, true).unwrap());
        std::thread::sleep(Duration::from_secs(2));
        println!("Unsubscribing again: {:?}", set_subscriptions(&dir, &ids, false).unwrap());
    }
}
