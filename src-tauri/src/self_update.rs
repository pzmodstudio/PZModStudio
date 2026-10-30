//! "Update now": the app puts a newer release in its own place and starts it again.
//!
//! Only what the release itself proves is taken. `SHA256SUMS.txt` must carry the project's
//! ed25519 signature (`SHA256SUMS.txt.sig`, the key in `keys/release-signing.pub.pem`, built into
//! the app), and the portable zip must have the SHA-256 that signed list gives it; anything else
//! stops before a file is touched. Then the new exe (and the MCP server beside it, when there is
//! one) is written next to the running one, the running exe is renamed aside (Windows lets a
//! running exe be renamed, not replaced) and the new one takes its name. The old copy is removed
//! the next time the app starts. A folder the app cannot write to (an install for every user, in
//! Program Files) is refused before downloading: download the new zip from the release page instead.

use base64::Engine;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

const REPO: &str = "pzmodstudio/PZModStudio";
const PUBLIC_KEY_PEM: &str = include_str!("../../keys/release-signing.pub.pem");
const SUMS: &str = "SHA256SUMS.txt";
const MCP_EXE: &str = "pz-mcp-server.exe";
/// Inside the portable zip.
const ZIP_APP_EXE: &str = "Project-Zomboid-Mod-Studio.exe";
/// The renamed-aside copy of a replaced exe, removed at the next start.
const OLD_MARK: &str = ".old-update";
const MAX_DOWNLOAD: u64 = 128 << 20;

/// The 32 bytes of the project's ed25519 public key (the end of its SPKI DER).
fn public_key() -> Result<[u8; 32], String> {
    let body: String = PUBLIC_KEY_PEM.lines().filter(|l| !l.starts_with("-----")).collect();
    let der = base64::engine::general_purpose::STANDARD.decode(body.trim()).map_err(|e| e.to_string())?;
    der.get(der.len().saturating_sub(32)..).and_then(|k| k.try_into().ok()).ok_or_else(|| "The built-in release key is not readable.".to_string())
}

/// The signature (base64, as `SHA256SUMS.txt.sig` holds it) is the project's over `sums`.
pub fn verify_sums(sums: &[u8], signature_b64: &str, key: &[u8; 32]) -> Result<(), String> {
    let sig = base64::engine::general_purpose::STANDARD.decode(signature_b64.trim()).map_err(|_| "The release's signature is not readable.".to_string())?;
    let sig = ed25519_dalek::Signature::from_slice(&sig).map_err(|_| "The release's signature is not readable.".to_string())?;
    let key = ed25519_dalek::VerifyingKey::from_bytes(key).map_err(|e| e.to_string())?;
    key.verify_strict(sums, &sig).map_err(|_| "The release's checksums are not signed by the project key: nothing was changed.".to_string())
}

/// The SHA-256 the sums list for `name`.
pub fn listed_hash(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.trim().split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name && hash.len() == 64).then(|| hash.to_lowercase())
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{:02x}", b)).collect()
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    let tls = ureq::tls::TlsConfig::builder().provider(ureq::tls::TlsProvider::NativeTls).build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(std::time::Duration::from_secs(300)))
        .user_agent(format!("PZModStudio/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|e| format!("Could not download {}: {}", url.rsplit('/').next().unwrap_or(url), e))?;
    response.body_mut().with_config().limit(MAX_DOWNLOAD).read_to_vec().map_err(|e| e.to_string())
}

fn asset_url(version: &str, file: &str) -> String {
    format!("https://github.com/{}/releases/download/v{}/{}", REPO, version, file)
}

/// The files of `zip` named `names`, by name (top level of the zip only).
fn from_zip(zip: &[u8], names: &[&str]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).map_err(|e| format!("The downloaded zip is not readable: {}", e))?;
    let mut out = Vec::new();
    for name in names {
        if let Ok(mut f) = archive.by_name(name) {
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            out.push((name.to_string(), bytes));
        }
    }
    Ok(out)
}

/// Whether the app may write next to itself.
fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".pzms-write-test-{}", std::process::id()));
    let ok = std::fs::write(&probe, b"x").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

fn aside_name(target: &Path) -> PathBuf {
    let stem = target.file_stem().unwrap_or_default().to_string_lossy();
    let mut aside = target.with_file_name(format!("{}{}.exe", stem, OLD_MARK));
    if aside.exists() && std::fs::remove_file(&aside).is_err() {
        aside = target.with_file_name(format!("{}{}-{}.exe", stem, OLD_MARK, std::process::id()));
    }
    aside
}

/// Puts `bytes` in place of `target`: written beside it first, then the old one renamed aside
/// and the new one renamed in; on a failure the old one goes back.
fn replace(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let new = target.with_extension("exe.new");
    std::fs::write(&new, bytes).map_err(|e| format!("Cannot write {}: {}", new.display(), e))?;
    let aside = aside_name(target);
    if target.exists() {
        if let Err(e) = std::fs::rename(target, &aside) {
            let _ = std::fs::remove_file(&new);
            return Err(format!("Cannot move {} aside: {}", target.display(), e));
        }
    }
    if let Err(e) = std::fs::rename(&new, target) {
        let _ = std::fs::rename(&aside, target);
        let _ = std::fs::remove_file(&new);
        return Err(format!("Cannot put the new {} in place: {}", target.display(), e));
    }
    Ok(())
}

/// Downloads `version`, checks it and puts it in place of the running app. Returns the exe to
/// start. Nothing is changed unless the signature and the checksum both hold.
pub fn install(version: &str) -> Result<PathBuf, String> {
    if !version.chars().all(|c| c.is_ascii_digit() || c == '.') || version.is_empty() {
        return Err(format!("\"{}\" is not a version.", version));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("The app's folder is not known.")?.to_path_buf();
    if !writable(&dir) {
        return Err(format!("The app cannot write to its own folder ({}): move the app to a folder you can write to, or download the new zip from the release page.", dir.display()));
    }
    let sums = download(&asset_url(version, SUMS))?;
    let sig = download(&asset_url(version, &format!("{}.sig", SUMS)))?;
    verify_sums(&sums, &String::from_utf8_lossy(&sig), &public_key()?)?;
    let sums_text = String::from_utf8_lossy(&sums).to_string();
    let zip_name = format!("PZ-Mod-Studio-Portable-v{}.zip", version);
    let expected = listed_hash(&sums_text, &zip_name).ok_or_else(|| format!("The signed checksums do not list {}.", zip_name))?;
    let zip = download(&asset_url(version, &zip_name))?;
    if sha256_hex(&zip) != expected {
        return Err(format!("{} is not the file the release signed (its SHA-256 differs): nothing was changed.", zip_name));
    }
    let files = from_zip(&zip, &[ZIP_APP_EXE, MCP_EXE])?;
    let app = files.iter().find(|(n, _)| n == ZIP_APP_EXE).map(|(_, b)| b).ok_or_else(|| format!("{} has no {}.", zip_name, ZIP_APP_EXE))?;
    if !app.starts_with(b"MZ") {
        return Err(format!("{} in {} is not a Windows program.", ZIP_APP_EXE, zip_name));
    }
    replace(&exe, app)?;
    // The MCP server beside it, when this copy has one (an agent may be running it: then it stays).
    let mcp = dir.join(MCP_EXE);
    if mcp.exists() {
        if let Some((_, bytes)) = files.iter().find(|(n, _)| n == MCP_EXE) {
            if let Err(e) = replace(&mcp, bytes) {
                crate::diagnostics::error(format!("[Update] the MCP server stayed as it was: {}", e));
            }
        }
    }
    Ok(exe)
}

/// The copies an update renamed aside, removed once the app runs from the new one.
pub fn remove_old_copies() {
    let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) else { return };
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.contains(OLD_MARK) && name.ends_with(".exe") {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    #[test]
    fn the_built_in_key_is_the_projects() {
        assert_eq!(base64::engine::general_purpose::STANDARD.encode(public_key().unwrap()), "n/cV3Kqh9txGTSl+PkpfYNuIvE/75bh7Otn+fSrQWns=");
    }

    #[test]
    fn only_sums_signed_by_the_key_and_a_listed_hash_pass() {
        let signer = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let key = signer.verifying_key().to_bytes();
        let zip = b"the app".to_vec();
        let sums = format!("{}  PZ-Mod-Studio-Portable-v9.9.9.zip\n{}  other.exe\n", sha256_hex(&zip), "0".repeat(64));
        let sig = base64::engine::general_purpose::STANDARD.encode(signer.sign(sums.as_bytes()).to_bytes());
        assert!(verify_sums(sums.as_bytes(), &sig, &key).is_ok());
        assert!(verify_sums(format!("{} ", sums).as_bytes(), &sig, &key).is_err(), "a changed list");
        assert!(verify_sums(sums.as_bytes(), &sig, &public_key().unwrap()).is_err(), "another key");
        assert_eq!(listed_hash(&sums, "PZ-Mod-Studio-Portable-v9.9.9.zip"), Some(sha256_hex(&zip)));
        assert_eq!(listed_hash(&sums, "missing.zip"), None);
    }

    #[test]
    fn a_replaced_exe_is_moved_aside_and_the_new_one_takes_its_name() {
        let dir = std::env::temp_dir().join(format!("pzms-selfupdate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("App.exe");
        std::fs::write(&target, b"old").unwrap();
        replace(&target, b"new").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert_eq!(std::fs::read(dir.join("App.old-update.exe")).unwrap(), b"old");
        assert!(!dir.join("App.exe.new").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Live: `cargo test --lib -- --ignored live_release_checks_out --nocapture` (downloads, writes nothing).
    #[test]
    #[ignore]
    fn live_release_checks_out() {
        let version = "1.7.1";
        let sums = download(&asset_url(version, SUMS)).unwrap();
        let sig = download(&asset_url(version, &format!("{}.sig", SUMS))).unwrap();
        verify_sums(&sums, &String::from_utf8_lossy(&sig), &public_key().unwrap()).unwrap();
        let name = format!("PZ-Mod-Studio-Portable-v{}.zip", version);
        let zip = download(&asset_url(version, &name)).unwrap();
        assert_eq!(Some(sha256_hex(&zip)), listed_hash(&String::from_utf8_lossy(&sums), &name));
        let files = from_zip(&zip, &[ZIP_APP_EXE, MCP_EXE]).unwrap();
        println!("{:?}", files.iter().map(|(n, b)| (n.clone(), b.len())).collect::<Vec<_>>());
        assert!(files.iter().any(|(n, b)| n == ZIP_APP_EXE && b.starts_with(b"MZ")));
    }
}
