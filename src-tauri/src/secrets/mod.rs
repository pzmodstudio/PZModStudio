//! User secrets (API keys) in the operating system's credential store: Windows Credential
//! Manager. Never written to files, logs, localStorage or the repo, and never sent back to the
//! UI after being saved (the UI only learns whether a key is set).

use std::sync::OnceLock;

const SERVICE: &str = "PZ Mod Studio";

pub const STEAM_WEB_API_KEY: &str = "steam_web_api_key";

fn ensure_store() -> Result<(), String> {
    static INIT: OnceLock<Result<(), String>> = OnceLock::new();
    INIT.get_or_init(|| {
        #[cfg(windows)]
        {
            let store = windows_native_keyring_store::Store::new()
                .map_err(|e| format!("Windows Credential Manager is not available: {}", e))?;
            keyring_core::set_default_store(store);
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err("Saving API keys is only supported on Windows for now.".to_string())
        }
    })
    .clone()
}

fn entry(name: &str) -> Result<keyring_core::Entry, String> {
    ensure_store()?;
    keyring_core::Entry::new(SERVICE, name).map_err(|e| format!("Cannot open the stored key: {}", e))
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    entry(name)?.set_password(value).map_err(|e| format!("Cannot save the key: {}", e))
}

/// None when nothing is stored.
pub fn get(name: &str) -> Result<Option<String>, String> {
    match entry(name)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Cannot read the stored key: {}", e)),
    }
}

pub fn delete(name: &str) -> Result<(), String> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Cannot remove the stored key: {}", e)),
    }
}

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
