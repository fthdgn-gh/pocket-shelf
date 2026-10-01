//! Background art of installed titles (cargo feature `installed-apps`).
//!
//! A title ships a full-screen picture next to its icon: `pic0.png`
//! (960x544), or the LiveArea background named by
//! `livearea/contents/template.xml` (840x500). Both are looked for in the
//! title's `sce_sys` folder and in the system's copy of it (see
//! `installed::metadata_dirs`). The launcher draws the picture
//! behind the selected title. The host decodes the file, crops it to the
//! screen's 30:17 shape and averages it down to a 512x256 texture, the
//! largest power-of-two size the core accepts; the guest stretches that back
//! over the screen.
//!
//! Reading and decoding a picture takes long enough to drop frames, so it
//! runs on a worker thread (`jobs`): a request answers `PENDING` until the
//! pixels are ready, and the guest asks again on later frames.
//!
//! A texture this size is 512 KiB, so the guest says when it is done with
//! one (`release`), and the host frees it then. The host does not free a
//! picture by itself: only the guest knows which ones are on screen.

use std::fs;
use std::string::String;
use std::vec::Vec;

use crate::jobs::{self, Pixels, Poll, PENDING};

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
/// Textures held at once. A guest that asks for more without releasing any
/// gets -1 for the extra ones.
const HELD_MAX: usize = 8;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Loaded textures, by picture.
static mut CACHE: Vec<(String, i32)> = Vec::new();
/// Titles found to have no usable picture, so their files are read once.
static mut MISSING: Vec<String> = Vec::new();

/// File name of the LiveArea background, from the `<image>` inside
/// `<livearea-background>` of the template under `base`.
fn livearea_image(base: &str) -> Option<String> {
    let path = format!("{base}/livearea/contents/template.xml");
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

/// Files to try in a title's metadata folders, best first.
fn candidates(dirs: &[String]) -> Vec<String> {
    let mut paths = Vec::new();
    for base in dirs {
        paths.push(format!("{base}/pic0.png"));
        if let Some(name) = livearea_image(base) {
            paths.push(format!("{base}/livearea/contents/{name}"));
        }
        paths.push(format!("{base}/livearea/contents/bg.png"));
        paths.push(format!("{base}/livearea/contents/bg0.png"));
    }
    paths
}

/// The bytes of a PNG file, or None when the file is missing, too large, or
/// not a PNG (an encrypted file is not).
fn read_png(path: &str) -> Option<Vec<u8>> {
    if fs::metadata(path).ok()?.len() > FILE_MAX {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    bytes.starts_with(&PNG_SIGNATURE).then_some(bytes)
}

/// The file of a listed title's own picture. A retail game the home screen
/// has not opened yet has no plain copy, so its own files are read through a
/// decrypting mount (`installed::with_decrypted`). The mount is held for the
/// read only; decoding happens after it is released.
fn read_title(title_id: &str) -> Option<Vec<u8>> {
    let from = |dirs: &[String]| candidates(dirs).iter().find_map(|path| read_png(path));
    from(&crate::installed::metadata_dirs(title_id))
        .or_else(|| unsafe { crate::installed::with_decrypted(title_id, from) })
}

/// Decode, crop to the screen's shape and box-filter down. Returns the
/// texture's width and height and its RGBA pixels.
fn decode(bytes: &[u8]) -> Option<Pixels> {
    let mut decoder = png::Decoder::new(bytes);
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

/// Path of a picture the user supplied: a plain `.png` name in the data
/// folder's `backdrops` directory.
#[cfg(feature = "data-fs")]
fn custom_path(file: &str) -> Option<String> {
    let plain = !file.is_empty()
        && file.len() <= 96
        && !file.starts_with('.')
        && !file.contains(['/', '\\', ':'])
        && file.to_ascii_lowercase().ends_with(".png");
    plain.then(|| format!("{}/backdrops/{file}", crate::datafs::data_dir()))
}

#[cfg(not(feature = "data-fs"))]
fn custom_path(_file: &str) -> Option<String> {
    None
}

/// Texture handle of a background picture: -1 when there is none the host
/// can decode, or `PENDING` while it is being read and decoded; the guest
/// asks again on a later frame. With `file` empty it is the listed title's
/// own picture; otherwise it is that PNG from the data folder's `backdrops`
/// directory. The handle stays valid until it is passed to `release`; asking
/// for the same picture again before that returns the same handle.
pub unsafe fn texture(ui: &mut pocketjs_core::Ui, title_id: &str, file: &str) -> i32 {
    // A file's key cannot collide with a title id, which has no colon.
    let key = if file.is_empty() { String::from(title_id) } else { format!("file:{file}") };
    if file.is_empty() && !crate::installed::listed(title_id) {
        return -1;
    }
    if MISSING.iter().any(|id| *id == key) {
        return -1;
    }
    if let Some((_, handle)) = CACHE.iter().find(|(id, _)| *id == key) {
        return *handle;
    }
    if CACHE.len() >= HELD_MAX {
        return -1;
    }
    let path = if file.is_empty() { None } else { custom_path(file) };
    if !file.is_empty() && path.is_none() {
        return -1;
    }
    let title = String::from(title_id);
    let poll = jobs::poll(&format!("backdrop:{key}"), move || {
        let bytes = match path {
            Some(path) => read_png(&path),
            None => read_title(&title),
        };
        bytes.and_then(|bytes| decode(&bytes))
    });
    let pixels = match poll {
        Poll::Pending => return PENDING,
        Poll::Ready(pixels) => pixels,
    };
    let Some((width, height, rgba)) = pixels else {
        // A title's own files do not change while the launcher runs. A file in
        // the data folder can appear later (a download), so it is tried again.
        if file.is_empty() {
            MISSING.push(key);
        }
        return -1;
    };
    let handle = ui.upload_texture_flags(
        &rgba,
        width,
        height,
        pocketjs_core::spec::psm::PSM_8888,
        pocketjs_core::spec::img::FLAG_LINEAR,
    );
    if handle >= 0 {
        crate::graphics::register_texture(ui, handle);
        CACHE.push((key, handle));
    }
    handle
}

/// Free a picture's texture. The guest calls this once nothing on screen
/// draws the handle any more. Handles this module did not hand out are ignored.
pub unsafe fn release(ui: &mut pocketjs_core::Ui, handle: i32) {
    let Some(index) = CACHE.iter().position(|(_, held)| *held == handle) else {
        return;
    };
    CACHE.remove(index);
    crate::graphics::free_texture(handle);
    ui.free_texture(handle);
}
