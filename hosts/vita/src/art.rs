//! Custom box art (cargo features `installed-apps` and `data-fs`).
//!
//! The user drops PNG files into `<data folder>/art`; the launcher lists them
//! through `globalThis.fs` and asks for one by file name. The host decodes the
//! image, crops it to a centered square, and averages it down to a power-of-two
//! texture (at most 256 px), which is the only shape the core accepts.

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

static mut CACHE: Vec<(String, i32)> = Vec::new();

/// Folder the art files live in.
pub fn dir() -> String {
    format!("{}/art", crate::datafs::data_dir())
}

/// A plain `.png` file name: no separators, no hidden files.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 96
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && name.to_ascii_lowercase().ends_with(".png")
}

/// Decode, crop to a centered square and box-filter down. Returns the texture
/// side and its RGBA pixels.
fn decode(path: &str) -> Option<(u32, Vec<u8>)> {
    if fs::metadata(path).ok()?.len() > FILE_MAX {
        return None;
    }
    let file = fs::File::open(path).ok()?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
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

    let side = width.min(height);
    let mut out = OUT_MAX;
    while out > side {
        out /= 2;
    }
    let origin_x = (width - side) / 2;
    let origin_y = (height - side) / 2;
    let cells = (out * out) as usize;
    let mut sums = vec![0u32; cells * 4];
    let mut counts = vec![0u32; cells];
    for y in origin_y..origin_y + side {
        let start = y as usize * info.line_size;
        let row = buffer.get(start..start + info.line_size)?;
        let dest_y = ((y - origin_y) as u64 * out as u64 / side as u64) as usize;
        for x in origin_x..origin_x + side {
            let at = x as usize * channels;
            let pixel = row.get(at..at + channels)?;
            let (r, g, b, a) = match channels {
                4 => (pixel[0], pixel[1], pixel[2], pixel[3]),
                3 => (pixel[0], pixel[1], pixel[2], 255),
                2 => (pixel[0], pixel[0], pixel[0], pixel[1]),
                _ => (pixel[0], pixel[0], pixel[0], 255),
            };
            let dest_x = ((x - origin_x) as u64 * out as u64 / side as u64) as usize;
            let cell = dest_y * out as usize + dest_x;
            sums[cell * 4] += r as u32;
            sums[cell * 4 + 1] += g as u32;
            sums[cell * 4 + 2] += b as u32;
            sums[cell * 4 + 3] += a as u32;
            counts[cell] += 1;
        }
    }
    let mut rgba = Vec::with_capacity(cells * 4);
    for cell in 0..cells {
        let count = counts[cell].max(1);
        for channel in 0..4 {
            rgba.push((sums[cell * 4 + channel] / count) as u8);
        }
    }
    Some((out, rgba))
}

/// Texture handle for the art file `name`, or -1 when the name is not a plain
/// PNG file name or the file cannot be decoded. The first call decodes and
/// uploads; later calls return the cached handle.
pub unsafe fn texture(ui: &mut pocketjs_core::Ui, name: &str) -> i32 {
    if !valid_name(name) {
        return -1;
    }
    if let Some((_, handle)) = CACHE.iter().find(|(cached, _)| cached == name) {
        return *handle;
    }
    let handle = match decode(&format!("{}/{name}", dir())) {
        Some((side, rgba)) => {
            let handle = ui.upload_texture_flags(
                &rgba,
                side,
                side,
                pocketjs_core::spec::psm::PSM_8888,
                pocketjs_core::spec::img::FLAG_LINEAR,
            );
            crate::accent::record(handle, &rgba);
            handle
        }
        None => -1,
    };
    if handle >= 0 {
        crate::graphics::register_texture(ui, handle);
    }
    CACHE.push((String::from(name), handle));
    handle
}
