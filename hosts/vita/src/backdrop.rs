//! Background art of installed titles (cargo feature `installed-apps`).
//!
//! A title ships a full-screen picture next to its icon: `sce_sys/pic0.png`
//! (960x544), or the LiveArea background named by
//! `sce_sys/livearea/contents/template.xml` (840x500). The launcher draws it
//! behind the selected title. The host decodes the file, crops it to the
//! screen's 30:17 shape and averages it down to a 512x256 texture, the
//! largest power-of-two size the core accepts; the guest stretches that back
//! over the screen.
//!
//! A texture this size is 512 KiB, so only the last few are kept: asking for
//! one more frees the least recently used.

use std::fs;
use std::string::String;
use std::vec::Vec;

const APP_ROOT: &str = "ux0:/app";
/// Largest file read, in bytes.
const FILE_MAX: u64 = 8 * 1024 * 1024;
/// Largest image decoded, in pixels (1920x1080). The decoded frame is held
/// in memory, so this bounds it to 8 MiB of RGBA.
const PIXELS_MAX: u64 = 2_097_152;
const OUT_W: u32 = 512;
const OUT_H: u32 = 256;
/// Smallest texture produced for a small source image.
const OUT_W_MIN: u32 = 64;
/// Screen shape the picture is cropped to (480:272).
const ASPECT_W: u64 = 30;
const ASPECT_H: u64 = 17;
/// Textures kept at once.
const KEEP: usize = 4;

/// Loaded textures, least recently used first.
static mut CACHE: Vec<(String, i32)> = Vec::new();
/// Titles found to have no usable picture, so their files are read once.
static mut MISSING: Vec<String> = Vec::new();

/// File name of the LiveArea background, from the `<image>` inside
/// `<livearea-background>` of the title's template.
fn livearea_image(title_id: &str) -> Option<String> {
    let path = format!("{APP_ROOT}/{title_id}/sce_sys/livearea/contents/template.xml");
    if fs::metadata(&path).ok()?.len() > 64 * 1024 {
        return None;
    }
    let xml = fs::read_to_string(&path).ok()?;
    let section = xml.split("<livearea-background>").nth(1)?;
    let section = section.split("</livearea-background>").next()?;
    let name = section.split("<image>").nth(1)?.split("</image>").next()?.trim();
    let plain = !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && name.to_ascii_lowercase().ends_with(".png");
    plain.then(|| String::from(name))
}

/// Files to try for a title, best first.
fn candidates(title_id: &str) -> Vec<String> {
    let base = format!("{APP_ROOT}/{title_id}/sce_sys");
    let mut paths = vec![format!("{base}/pic0.png")];
    if let Some(name) = livearea_image(title_id) {
        paths.push(format!("{base}/livearea/contents/{name}"));
    }
    paths.push(format!("{base}/livearea/contents/bg.png"));
    paths.push(format!("{base}/livearea/contents/bg0.png"));
    paths
}

/// Decode, crop to the screen's shape and box-filter down. Returns the
/// texture's width and height and its RGBA pixels.
fn decode(path: &str) -> Option<(u32, u32, Vec<u8>)> {
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
    if width as u64 * height as u64 > PIXELS_MAX {
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

    // Largest centered 30:17 rectangle inside the picture.
    let (crop_w, crop_h) = if width as u64 * ASPECT_H > height as u64 * ASPECT_W {
        ((height as u64 * ASPECT_W / ASPECT_H) as u32, height)
    } else {
        (width, (width as u64 * ASPECT_H / ASPECT_W) as u32)
    };
    // The filter averages source pixels into each texel, so the texture is
    // never larger than the cropped picture.
    let (mut out_w, mut out_h) = (OUT_W, OUT_H);
    while out_w > crop_w || out_h > crop_h {
        out_w /= 2;
        out_h /= 2;
    }
    if out_w < OUT_W_MIN {
        return None;
    }
    let origin_x = (width - crop_w) / 2;
    let origin_y = (height - crop_h) / 2;
    let cells = (out_w * out_h) as usize;
    let mut sums = vec![0u32; cells * 4];
    let mut counts = vec![0u32; cells];
    for y in origin_y..origin_y + crop_h {
        let start = y as usize * info.line_size;
        let row = buffer.get(start..start + info.line_size)?;
        let dest_y = ((y - origin_y) as u64 * out_h as u64 / crop_h as u64) as usize;
        for x in origin_x..origin_x + crop_w {
            let at = x as usize * channels;
            let pixel = row.get(at..at + channels)?;
            let (r, g, b) = match channels {
                4 | 3 => (pixel[0], pixel[1], pixel[2]),
                _ => (pixel[0], pixel[0], pixel[0]),
            };
            let dest_x = ((x - origin_x) as u64 * out_w as u64 / crop_w as u64) as usize;
            let cell = dest_y * out_w as usize + dest_x;
            sums[cell * 4] += r as u32;
            sums[cell * 4 + 1] += g as u32;
            sums[cell * 4 + 2] += b as u32;
            counts[cell] += 1;
        }
    }
    // A background is drawn opaque, so the picture's own alpha is dropped.
    let mut rgba = Vec::with_capacity(cells * 4);
    for cell in 0..cells {
        let count = counts[cell].max(1);
        for channel in 0..3 {
            rgba.push((sums[cell * 4 + channel] / count) as u8);
        }
        rgba.push(255);
    }
    Some((out_w, out_h, rgba))
}

/// Texture handle of a listed title's background picture, or -1 when the id
/// was not found by the scan or the title has no picture the host can decode.
/// The handle stays valid until `KEEP` other titles have been asked for;
/// asking again reloads it under a new handle.
pub unsafe fn texture(ui: &mut pocketjs_core::Ui, title_id: &str) -> i32 {
    if !crate::installed::listed(title_id) || MISSING.iter().any(|id| id == title_id) {
        return -1;
    }
    if let Some(index) = CACHE.iter().position(|(id, _)| id == title_id) {
        let entry = CACHE.remove(index);
        let handle = entry.1;
        CACHE.push(entry);
        return handle;
    }
    let decoded = candidates(title_id).iter().find_map(|path| decode(path));
    let handle = match decoded {
        Some((width, height, rgba)) => ui.upload_texture_flags(
            &rgba,
            width,
            height,
            pocketjs_core::spec::psm::PSM_8888,
            pocketjs_core::spec::img::FLAG_LINEAR,
        ),
        None => -1,
    };
    if handle < 0 {
        MISSING.push(String::from(title_id));
        return -1;
    }
    crate::graphics::register_texture(ui, handle);
    while CACHE.len() >= KEEP {
        let (_, old) = CACHE.remove(0);
        crate::graphics::free_texture(old);
        ui.free_texture(old);
    }
    CACHE.push((String::from(title_id), handle));
    handle
}
