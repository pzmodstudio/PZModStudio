//! A mod's own poster/icon, handed to the webview as a `data:` URL.
//!
//! The webview cannot open files on disk, so the picture travels inside the answer. Only image
//! files inside the folders the user configured are read, so a path from the UI cannot reach
//! anywhere else.

use crate::vfs::StudioPaths;
use std::path::{Path, PathBuf};

/// Biggest picture worth sending to the webview (mod posters are a few hundred KB).
const MAX_BYTES: u64 = 8 * 1024 * 1024;

fn media_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// Base64 (RFC 4648), the only encoding a `data:` URL takes for binary.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = u32::from_be_bytes([0, b[0], b[1], b[2]]);
        let digits = [(n >> 18) & 63, (n >> 12) & 63, (n >> 6) & 63, n & 63];
        for (i, d) in digits.iter().enumerate() {
            // The last chunk is padded with '=' for the bytes that are not there.
            out.push(if i > chunk.len() { '=' } else { ALPHABET[*d as usize] as char });
        }
    }
    out
}

/// The folders a picture may come from: the ones the user pointed the Studio at.
fn allowed_roots(paths: &StudioPaths) -> Vec<PathBuf> {
    [&paths.workshop_dir, &paths.user_zomboid_dir, &paths.pz_install_dir]
        .iter()
        .filter(|d| !d.is_empty())
        .filter_map(|d| std::fs::canonicalize(d).ok())
        .collect()
}

/// Reads an image inside the configured folders and returns it as a `data:` URL.
pub fn read_as_data_url(paths: &StudioPaths, path: &str) -> Result<String, String> {
    let file = std::fs::canonicalize(path).map_err(|_| format!("No image at {}", path))?;
    let media = media_type(&file).ok_or("Only PNG, JPEG, GIF and WebP images can be shown.")?;
    let roots = allowed_roots(paths);
    if roots.is_empty() {
        return Err("Set your game and Zomboid folders in App Settings first.".to_string());
    }
    if !roots.iter().any(|root| file.starts_with(root)) {
        return Err("That image is outside your game and Zomboid folders.".to_string());
    }
    let size = std::fs::metadata(&file).map_err(|e| e.to_string())?.len();
    if size > MAX_BYTES {
        return Err(format!("That image is too big to show ({} MB).", size / 1024 / 1024));
    }
    let bytes = std::fs::read(&file).map_err(|e| format!("The image could not be read: {}", e))?;
    Ok(format!("data:{};base64,{}", media, base64(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(dir: &Path) -> StudioPaths {
        StudioPaths {
            pz_install_dir: String::new(),
            workshop_dir: dir.to_string_lossy().to_string(),
            user_zomboid_dir: String::new(),
            mod_list_ini_path: String::new(),
            carrier_workshop_id: None,
            is_valid: true,
        }
    }

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn reads_only_images_inside_the_configured_folders() {
        let root = std::env::temp_dir().join(format!("pzms-img-{}", std::process::id()));
        let inside = root.join("mods");
        std::fs::create_dir_all(&inside).unwrap();
        let png = inside.join("poster.png");
        std::fs::write(&png, b"fake png bytes").unwrap();
        let p = paths(&root);

        let url = read_as_data_url(&p, png.to_str().unwrap()).unwrap();
        assert_eq!(url, format!("data:image/png;base64,{}", base64(b"fake png bytes")));

        let text = inside.join("mod.info");
        std::fs::write(&text, b"name=x").unwrap();
        assert!(read_as_data_url(&p, text.to_str().unwrap()).unwrap_err().contains("Only PNG"));

        let outside = std::env::temp_dir().join(format!("pzms-img-out-{}.png", std::process::id()));
        std::fs::write(&outside, b"x").unwrap();
        assert!(read_as_data_url(&p, outside.to_str().unwrap()).unwrap_err().contains("outside"));

        // Climbing out of the folder with .. is caught too.
        let climb = inside.join("..").join("..").join(outside.file_name().unwrap());
        assert!(read_as_data_url(&p, &climb.to_string_lossy()).is_err());

        assert!(read_as_data_url(&p, &inside.join("missing.png").to_string_lossy()).unwrap_err().contains("No image"));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&outside);
    }
}
