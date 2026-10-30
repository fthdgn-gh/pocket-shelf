//! Custom box art (cargo features `installed-apps` and `data-fs`).
//!
//! The user drops PNG files into `<data folder>/art`; the launcher lists them
//! and asks for one by file name. The host decodes the image and fits it
//! whole into a power-of-two square texture (at most 256 px) with clear
//! bars, keeping its shape.

use std::fs;
use std::string::String;
use std::vec::Vec;

/// Largest file read, in bytes.
const FILE_MAX: u64 = 8 * 1024 * 1024;
/// Largest image decoded, in pixels (about 1920x1080). The decoded frame is
/// held in memory, so this bounds it to 8 MiB of RGBA.
const PIXELS_MAX: u64 = 2_097_152;
const SIDE_MIN: u32 = 16;
const OUT_MAX: u32 = 256;

/// Folder the art files live in.
pub fn dir() -> String {
    format!("{}/art", crate::platform::data_dir())
}

/// A plain `.png` file name: no separators, no hidden files.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 96
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && name.to_ascii_lowercase().ends_with(".png")
}

/// Decode and fit whole into a square, box-filtered down. Returns the
/// texture side and its RGBA pixels.
pub(crate) fn decode(path: &str) -> Option<(u32, Vec<u8>)> {
    if fs::metadata(path).ok()?.len() > FILE_MAX {
        return None;
    }
    decode_bytes(&fs::read(path).ok()?)
}

/// `decode` for a PNG in memory (a Find artwork candidate not yet saved).
pub(crate) fn decode_bytes(bytes: &[u8]) -> Option<(u32, Vec<u8>)> {
    if bytes.len() as u64 > FILE_MAX {
        return None;
    }
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (width, height) = {
        let info = reader.info();
        (info.width, info.height)
    };
    if width < SIDE_MIN || height < SIDE_MIN || width as u64 * height as u64 > PIXELS_MAX {
        return None;
    }
    let size = reader.output_buffer_size();
    let mut buffer: Vec<u8> = Vec::new();
    buffer.try_reserve_exact(size).ok()?;
    buffer.resize(size, 0);
    let info = reader.next_frame(&mut buffer).ok()?;
    let channels = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };

    // The whole picture fitted into the largest power-of-two square (at
    // most `OUT_MAX`) its long side fills, with clear bars: a box art keeps
    // its shape, and the selection ring follows it (`Picture::content`).
    let long = width.max(height);
    let mut out = OUT_MAX;
    while out > long {
        out /= 2;
    }
    let rgba = crate::platform::pspemu::fit_square(&buffer, width, height, info.line_size, channels, out)?;
    Some((out, rgba))
}

/// The art file `name` fitted into a square and scaled down, RGBA; None when
/// the name is not a plain PNG file name or the file does not decode.
/// Called on a worker thread.
pub fn picture(name: &str) -> Option<(u32, u32, Vec<u8>)> {
    if !valid_name(name) {
        return None;
    }
    decode(&format!("{}/{name}", dir())).map(|(side, rgba)| (side, side, rgba))
}
