//! Background art of installed titles (cargo feature `installed-apps`).
//!
//! A title ships a full-screen picture next to its icon: `pic0.png`
//! (960x544), or the LiveArea background named by
//! `livearea/contents/template.xml` (840x500). Both are looked for in the
//! title's `sce_sys` folder and in the system's copy of it (see
//! `installed::metadata_dirs`). The launcher draws the picture
//! behind the selected title. The host decodes the file, crops it to the
//! screen's 30:17 shape and averages it down to at most 512x256 texels
//! (480x272 for a 960x544 picture; `shrink.rs`); the guest stretches that
//! back over the screen.
//!
//! Reading and decoding a picture takes long enough to drop frames, so it
//! runs on a worker thread (host.rs).

use std::fs;
use std::string::String;
use std::vec::Vec;

use super::shrink::Pixels;

/// Largest file read, in bytes.
const FILE_MAX: u64 = 8 * 1024 * 1024;
/// Largest image decoded, in pixels (1920x1080). The decoded frame is held
/// in memory, so this bounds it to 8 MiB of RGBA.
const PIXELS_MAX: u64 = 2_097_152;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Titles found to have no usable picture, so their files are read once.
static mut MISSING: Option<std::collections::HashSet<String>> = None;

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
    from(&crate::platform::installed::metadata_dirs(title_id))
        .or_else(|| unsafe { crate::platform::installed::with_decrypted(title_id, from) })
}

/// Decode, crop to the screen's shape and box-filter down. Returns the
/// texture's width and height and its RGBA pixels.
pub(crate) fn decode(bytes: &[u8]) -> Option<Pixels> {
    let started = crate::host::now_us();
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    // A damaged file shows as a damaged picture; the CRC and Adler checks
    // cost time on every decode.
    decoder.ignore_checksums(true);
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
    let decoded = crate::host::now_us();
    let channels = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };

    let shrunk = super::shrink::shrink(&buffer, info.line_size, channels, width, height);
    crate::host::log_line(&format!(
        "backdrop {width}x{height}: decode {} ms, scale {} ms",
        (decoded - started) / 1000,
        (crate::host::now_us() - decoded) / 1000
    ));
    shrunk
}

/// Path of a picture the user supplied: a plain `.png` name in the data
/// folder's `backdrops` directory.
#[cfg(feature = "data-fs")]
fn custom_path(file: &str) -> Option<String> {
    // A PNG the title editor's Browse set, read where it is.
    if shelf_core::library::overrides::is_path(file) {
        return Some(file.to_string());
    }
    let plain = !file.is_empty()
        && file.len() <= 96
        && !file.starts_with('.')
        && !file.contains(['/', '\\', ':'])
        && file.to_ascii_lowercase().ends_with(".png");
    plain.then(|| format!("{}/backdrops/{file}", crate::platform::data_dir()))
}

#[cfg(not(feature = "data-fs"))]
fn custom_path(_file: &str) -> Option<String> {
    None
}

/// A background picture, cropped to the screen's shape and scaled down:
/// with `file` empty the listed title's own picture, otherwise that PNG from
/// the data folder's `backdrops` directory. None when there is none that
/// decodes. Called on a worker thread.
pub unsafe fn picture(title_id: &str, file: &str) -> Option<Pixels> {
    let key = if file.is_empty() { String::from(title_id) } else { format!("file:{file}") };
    if file.is_empty() && !crate::platform::installed::listed(title_id) {
        return None;
    }
    if MISSING.as_ref().is_some_and(|missing| missing.contains(&key)) {
        return None;
    }
    let path = if file.is_empty() { None } else { Some(custom_path(file)?) };
    let classic = if file.is_empty() { crate::platform::installed::classic_files(title_id) } else { None };
    // A RetroArch game's screenshot from RetroArch's thumbnails; none when
    // its console has no thumbnail folders.
    let retro = if file.is_empty() { crate::platform::installed::retro_backdrops(title_id) } else { None };
    if retro.as_ref().is_some_and(|paths| paths.is_empty()) {
        MISSING.get_or_insert_with(Default::default).insert(key);
        return None;
    }
    let retro = retro.unwrap_or_default();
    let read = crate::host::now_us();
    let bytes = match (path, &classic) {
        (Some(path), _) => read_png(&path),
        (None, Some(files)) => crate::platform::pspemu::picture(files),
        (None, None) if !retro.is_empty() => retro.iter().find_map(|path| read_png(path)),
        (None, None) => read_title(title_id),
    };
    let pixels = bytes.and_then(|bytes| {
        crate::host::log_line(&format!("backdrop {title_id}: read {} KB in {} ms", bytes.len() / 1024, (crate::host::now_us() - read) / 1000));
        decode(&bytes)
    });
    // A title's own files do not change while the launcher runs. A file in
    // the data folder can appear later (a download), so it is tried again.
    if pixels.is_none() && file.is_empty() {
        MISSING.get_or_insert_with(Default::default).insert(key);
    }
    pixels
}
