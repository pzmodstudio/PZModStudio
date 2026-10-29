//! The real map under the grid: one image per map folder that ships an image pyramid
//! (`pyramid.zip`, the tiles the game's world map draws), put together from the tiles of the
//! level that gives about the asked size per cell.
//!
//! `pyramid.zip` holds `pyramid.txt` (`bounds=x0 y0 x1 y1` in squares, `imageSize=w h` of level
//! 0) and `<level>/tile<X>x<Y>.png`, 256 px each; every level halves the one before. The game's
//! own world is `bounds=0 0 19968 16128` (78 x 63 cells) at one pixel per square on level 0.
//! The images made are kept in `<Zomboid>/PZModStudio_Cache/maps/`, made again only when the
//! zip changes.

use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

/// One map image, placed on the world in squares.
#[derive(Debug, Clone, Serialize)]
pub struct MapImage {
    /// Asks for its bytes (`map_image_bytes`).
    pub id: String,
    /// The map folder it is of ("Muldraugh, KY" for the game's world).
    pub folder: String,
    /// x0, y0, x1, y1 in squares (a cell is 256).
    pub bounds: [i64; 4],
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PyramidInfo {
    pub bounds: [i64; 4],
    pub image_size: [u32; 2],
}

pub fn parse_pyramid_txt(text: &str) -> Option<PyramidInfo> {
    let field = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string()));
    let b: Vec<i64> = field("bounds=")?.split_whitespace().filter_map(|n| n.parse().ok()).collect();
    let s: Vec<u32> = field("imageSize=")?.split_whitespace().filter_map(|n| n.parse().ok()).collect();
    (b.len() == 4 && s.len() == 2 && b[2] > b[0] && b[3] > b[1] && s[0] > 0).then(|| PyramidInfo { bounds: [b[0], b[1], b[2], b[3]], image_size: [s[0], s[1]] })
}

/// The level whose pixels per cell is closest to `px_per_cell`, among `levels` present.
pub fn pick_level(info: &PyramidInfo, px_per_cell: u32, levels: &[u32]) -> Option<u32> {
    let squares = (info.bounds[2] - info.bounds[0]) as f64;
    let px_per_square_l0 = info.image_size[0] as f64 / squares;
    levels.iter().copied().min_by(|a, b| {
        let at = |l: u32| (px_per_square_l0 * 256.0 / f64::from(1u32 << l) - px_per_cell as f64).abs();
        at(*a).partial_cmp(&at(*b)).unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// FNV-1a, for cache names.
fn hash(text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)
}

fn decode_rgba(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (w, h) = (frame.width, frame.height);
    let data = &buf[..frame.buffer_size()];
    let rgba = match frame.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::GrayscaleAlpha => data.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Rgb => data.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        other => return Err(format!("unexpected tile colours {:?}", other)),
    };
    Ok((w, h, rgba))
}

/// Puts one level of a pyramid together into a PNG at `out`. Returns its size.
fn compose(zip_path: &Path, level: u32, info: &PyramidInfo, out: &Path) -> Result<(u32, u32), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|e| e.to_string())?;
    let width = info.image_size[0].div_ceil(1 << level);
    let height = info.image_size[1].div_ceil(1 << level);
    if u64::from(width) * u64::from(height) > 40_000_000 {
        return Err("The map image would be too big at this size.".to_string());
    }
    let mut canvas = vec![0u8; (width * height * 4) as usize];
    let prefix = format!("{}/tile", level);
    let names: Vec<String> = zip.file_names().filter(|n| n.starts_with(&prefix) && n.ends_with(".png")).map(String::from).collect();
    for name in names {
        let coords = &name[prefix.len()..name.len() - 4];
        let Some((tx, ty)) = coords.split_once('x') else { continue };
        let (Ok(tx), Ok(ty)) = (tx.parse::<u32>(), ty.parse::<u32>()) else { continue };
        let mut bytes = Vec::new();
        zip.by_name(&name).map_err(|e| e.to_string())?.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        let (tw, th, rgba) = decode_rgba(&bytes)?;
        for row in 0..th {
            let y = ty * tw.max(th) + row;
            if y >= height {
                break;
            }
            let x0 = tx * tw;
            if x0 >= width {
                continue;
            }
            let n = (tw.min(width - x0) * 4) as usize;
            let src = (row * tw * 4) as usize;
            let dst = ((y * width + x0) * 4) as usize;
            canvas[dst..dst + n].copy_from_slice(&rgba[src..src + n]);
        }
    }
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = out.with_extension("png.part");
    {
        let f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&canvas).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, out).map_err(|e| e.to_string())?;
    Ok((width, height))
}

/// The map image of one map folder with a `pyramid.zip`, made (or taken from the cache) at about
/// `px_per_cell` pixels per cell.
pub fn folder_image(map_dir: &Path, folder: &str, px_per_cell: u32, cache_dir: &Path) -> Result<Option<MapImage>, String> {
    let zip_path = map_dir.join("pyramid.zip");
    if !zip_path.is_file() {
        return Ok(None);
    }
    let file = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|e| e.to_string())?;
    let mut text = String::new();
    zip.by_name("pyramid.txt").map_err(|e| e.to_string())?.read_to_string(&mut text).map_err(|e| e.to_string())?;
    let Some(info) = parse_pyramid_txt(&text) else { return Ok(None) };
    let mut levels: Vec<u32> = zip.file_names().filter_map(|n| n.split('/').next()?.parse().ok()).collect();
    levels.sort();
    levels.dedup();
    drop(zip);
    let Some(level) = pick_level(&info, px_per_cell, &levels) else { return Ok(None) };
    let stamp = std::fs::metadata(&zip_path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let id = hash(&format!("{}|{}|{}", zip_path.display(), level, stamp));
    let out = cache_dir.join(format!("{}.png", id));
    let (width, height) = if out.is_file() {
        let bytes = std::fs::read(&out).map_err(|e| e.to_string())?;
        let d = png::Decoder::new(bytes.as_slice()).read_info().map_err(|e| e.to_string())?;
        (d.info().width, d.info().height)
    } else {
        compose(&zip_path, level, &info, &out)?
    };
    Ok(Some(MapImage { id, folder: folder.to_string(), bounds: info.bounds, width, height }))
}

pub fn cache_dir(user_zomboid_dir: &Path) -> PathBuf {
    user_zomboid_dir.join("PZModStudio_Cache").join("maps")
}

/// The bytes of a map image made before (ids are 16 hex digits; nothing else is read).
pub fn image_bytes(user_zomboid_dir: &Path, id: &str) -> Result<Vec<u8>, String> {
    if id.len() != 16 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Not a map image.".to_string());
    }
    std::fs::read(cache_dir(user_zomboid_dir).join(format!("{}.png", id))).map_err(|e| e.to_string())
}

/// The images of the game's world and of every installed map folder that has one.
pub fn map_images(pz_install_dir: &str, maps: &[(PathBuf, String)], user_zomboid_dir: &Path, px_per_cell: u32) -> Vec<MapImage> {
    let cache = cache_dir(user_zomboid_dir);
    let mut out = Vec::new();
    let vanilla = Path::new(pz_install_dir).join("media").join("maps").join(super::VANILLA_MAP);
    let mut dirs = vec![(vanilla, super::VANILLA_MAP.to_string())];
    dirs.extend(maps.iter().cloned());
    for (dir, folder) in dirs {
        match folder_image(&dir, &folder, px_per_cell, &cache) {
            Ok(Some(img)) => out.push(img),
            Ok(None) => {}
            Err(e) => crate::diagnostics::error(format!("[Maps] the image of {} could not be made: {}", folder, e)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_is_the_one_closest_to_the_size_asked() {
        let info = parse_pyramid_txt("VERSION=1\nbounds=0 0 19968 16128\nimageSize=19968 16128\n").unwrap();
        assert_eq!(info.bounds, [0, 0, 19968, 16128]);
        // Level 0 is 256 px per cell; each level halves it: 16 px is level 4, 32 px level 3.
        assert_eq!(pick_level(&info, 16, &[0, 1, 2, 3, 4]), Some(4));
        assert_eq!(pick_level(&info, 32, &[0, 1, 2, 3, 4]), Some(3));
        assert_eq!(pick_level(&info, 4, &[0, 1, 2, 3, 4]), Some(4), "the smallest there is");
        assert!(parse_pyramid_txt("bounds=0 0 0 0\nimageSize=1 1").is_none());
    }

    /// Live: `cargo test -- --ignored live_map_images --nocapture`: the game's world image.
    #[test]
    #[ignore]
    fn live_map_images() {
        let p = crate::vfs::studio_paths();
        let started = std::time::Instant::now();
        let imgs = map_images(&p.pz_install_dir, &[], Path::new(&p.user_zomboid_dir), 16);
        for i in &imgs {
            println!("{} {} {:?} {}x{} in {:?}", i.id, i.folder, i.bounds, i.width, i.height, started.elapsed());
        }
        assert!(!imgs.is_empty());
    }
}
