//! User secrets (API keys, a server's admin password) in the operating system's credential store:
//! Windows Credential Manager. Never written to logs, localStorage or the repo, and never sent
//! back to the UI after being saved (the UI only learns whether a key is set).
//!
//! On Linux there is no credential store the app can count on without a desktop keyring service
//! running (and a heavy dependency to reach it), so the secrets live in one file,
//! `<config dir>/PZModStudio/secrets.json` (`~/.config/PZModStudio/secrets.json`), created with
//! permissions 0600 (only the user can read it) and rewritten whole through a temporary file of
//! the same permissions.

#[cfg(windows)]
use std::sync::OnceLock;

const SERVICE: &str = "PZ Mod Studio";

pub const STEAM_WEB_API_KEY: &str = "steam_web_api_key";

#[cfg(windows)]
fn ensure_store() -> Result<(), String> {
    static INIT: OnceLock<Result<(), String>> = OnceLock::new();
    INIT.get_or_init(|| {
        let store = windows_native_keyring_store::Store::new()
            .map_err(|e| format!("Windows Credential Manager is not available: {}", e))?;
        keyring_core::set_default_store(store);
        Ok(())
    })
    .clone()
}

#[cfg(windows)]
fn entry(name: &str) -> Result<keyring_core::Entry, String> {
    ensure_store()?;
    keyring_core::Entry::new(SERVICE, name).map_err(|e| format!("Cannot open the stored key: {}", e))
}

#[cfg(windows)]
pub fn set(name: &str, value: &str) -> Result<(), String> {
    entry(name)?.set_password(value).map_err(|e| format!("Cannot save the key: {}", e))
}

/// None when nothing is stored.
#[cfg(windows)]
pub fn get(name: &str) -> Result<Option<String>, String> {
    match entry(name)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Cannot read the stored key: {}", e)),
    }
}

#[cfg(windows)]
pub fn delete(name: &str) -> Result<(), String> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Cannot remove the stored key: {}", e)),
    }
}

#[cfg(not(windows))]
mod file_store {
    use std::collections::BTreeMap;
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    fn file() -> Result<PathBuf, String> {
        if let Some(dir) = std::env::var_os("PZMS_SECRETS_DIR") {
            return Ok(PathBuf::from(dir).join("secrets.json"));
        }
        dirs_next::config_dir().map(|d| d.join("PZModStudio").join("secrets.json")).ok_or_else(|| "No configuration folder for the secrets file.".to_string())
    }

    /// Every secret, by service and name.
    fn load() -> Result<BTreeMap<String, String>, String> {
        let path = file()?;
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{} is not readable: {}", path.display(), e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(format!("Cannot read {}: {}", path.display(), e)),
        }
    }

    fn save(all: &BTreeMap<String, String>) -> Result<(), String> {
        let path = file()?;
        let dir = path.parent().ok_or("No folder for the secrets file.")?;
        std::fs::create_dir_all(dir).map_err(|e| format!("Cannot make {}: {}", dir.display(), e))?;
        let tmp = path.with_extension("json.tmp");
        let _ = std::fs::remove_file(&tmp);
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp).map_err(|e| format!("Cannot write {}: {}", tmp.display(), e))?;
        f.write_all(serde_json::to_string_pretty(all).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        drop(f);
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("Cannot save {}: {}", path.display(), e))
    }

    fn key(name: &str) -> String {
        format!("{}/{}", super::SERVICE, name)
    }

    pub fn set(name: &str, value: &str) -> Result<(), String> {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut all = load()?;
        all.insert(key(name), value.to_string());
        save(&all).map_err(|e| format!("Cannot save the key: {}", e))
    }

    pub fn get(name: &str) -> Result<Option<String>, String> {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        Ok(load()?.get(&key(name)).cloned())
    }

    pub fn delete(name: &str) -> Result<(), String> {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut all = load()?;
        if all.remove(&key(name)).is_some() {
            save(&all).map_err(|e| format!("Cannot remove the key: {}", e))?;
        }
        Ok(())
    }
}

#[cfg(not(windows))]
pub use file_store::{delete, get, set};

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_credential_manager() {
        let name = format!("test_secret_{}", std::process::id());
        assert_eq!(get(&name).unwrap(), None);
        set(&name, "value-1").unwrap();
        assert_eq!(get(&name).unwrap().as_deref(), Some("value-1"));
        set(&name, "value-2").unwrap();
        assert_eq!(get(&name).unwrap().as_deref(), Some("value-2"));
        delete(&name).unwrap();
        assert_eq!(get(&name).unwrap(), None);
        delete(&name).unwrap(); // deleting twice is fine
    }
}


#[cfg(all(test, not(windows)))]
mod file_tests {
    use super::*;

    #[test]
    fn round_trips_through_a_private_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("pzms_secrets_{}", std::process::id()));
        std::env::set_var("PZMS_SECRETS_DIR", &dir);
        let name = "test_secret";
        assert_eq!(get(name).unwrap(), None);
        set(name, "value-1").unwrap();
        set(name, "value-2").unwrap();
        assert_eq!(get(name).unwrap().as_deref(), Some("value-2"));
        let mode = std::fs::metadata(dir.join("secrets.json")).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        delete(name).unwrap();
        assert_eq!(get(name).unwrap(), None);
        delete(name).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
