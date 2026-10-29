//! The pictures a package carries: the poster that ships, and a generated one for when it is missing.

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    (s2 << 16) | s1
}

pub fn generate_256x256_png_bytes() -> Vec<u8> {
    let mut png = Vec::new();
    png.extend_from_slice(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&256u32.to_be_bytes());
    ihdr_data.extend_from_slice(&256u32.to_be_bytes());
    ihdr_data.push(8);
    ihdr_data.push(6);
    ihdr_data.push(0);
    ihdr_data.push(0);
    ihdr_data.push(0);

    let mut ihdr_chunk = Vec::new();
    ihdr_chunk.extend_from_slice(b"IHDR");
    ihdr_chunk.extend_from_slice(&ihdr_data);
    let ihdr_crc = crc32(&ihdr_chunk);

    png.extend_from_slice(&(ihdr_data.len() as u32).to_be_bytes());
    png.extend_from_slice(&ihdr_chunk);
    png.extend_from_slice(&ihdr_crc.to_be_bytes());

    let mut raw_data = Vec::with_capacity(262400);
    for _ in 0..256 {
        raw_data.push(0);
        for _ in 0..256 {
            raw_data.extend_from_slice(&[16, 185, 129, 255]);
        }
    }

    let adler = adler32(&raw_data);

    let mut zlib_stream = Vec::new();
    zlib_stream.extend_from_slice(&[0x78, 0x01]);

    let block_size = 65535;
    let mut offset = 0;
    while offset < raw_data.len() {
        let chunk_size = (raw_data.len() - offset).min(block_size);
        let is_final = (offset + chunk_size) == raw_data.len();
        let bfinal_btype = if is_final { 0x01 } else { 0x00 };

        zlib_stream.push(bfinal_btype);
        let len_u16 = chunk_size as u16;
        let nlen_u16 = !len_u16;

        zlib_stream.extend_from_slice(&len_u16.to_le_bytes());
        zlib_stream.extend_from_slice(&nlen_u16.to_le_bytes());
        zlib_stream.extend_from_slice(&raw_data[offset..offset + chunk_size]);

        offset += chunk_size;
    }

    zlib_stream.extend_from_slice(&adler.to_be_bytes());

    let mut idat_chunk = Vec::new();
    idat_chunk.extend_from_slice(b"IDAT");
    idat_chunk.extend_from_slice(&zlib_stream);
    let idat_crc = crc32(&idat_chunk);

    png.extend_from_slice(&(zlib_stream.len() as u32).to_be_bytes());
    png.extend_from_slice(&idat_chunk);
    png.extend_from_slice(&idat_crc.to_be_bytes());

    let mut iend_chunk = Vec::new();
    iend_chunk.extend_from_slice(b"IEND");
    let iend_crc = crc32(&iend_chunk);

    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(&iend_chunk);
    png.extend_from_slice(&iend_crc.to_be_bytes());

    png
}

/// The picture every generated package shows in the game's mod list. Compiled in, so it does not
/// depend on where the Studio was started from: it used to be read from the working directory,
/// which meant a stray file of that name next to the exe became the poster.
static EMBEDDED_PREVIEW_PNG: &[u8] = include_bytes!("../../../assets/package-poster.png");

pub fn get_preview_png_bytes() -> Vec<u8> {
    EMBEDDED_PREVIEW_PNG.to_vec()
}
