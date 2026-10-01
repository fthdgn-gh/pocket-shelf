//! Installed-title navigation for the Vita host (cargo feature `installed-apps`).
//!
//! Implements the native flavour of the launcher ops (docs/LAUNCHER.md,
//! "Native process navigation"): `appTable()` lists the titles installed under
//! `ux0:/app` and `appLaunch(titleId)` starts one through the system app
//! manager. The table carries `kind: "native"`, so the framework treats the
//! call as a process switch, not as a guest swap inside this process.
//!
//! Titles are read from each `sce_sys/param.sfo`. The list is scanned once per
//! process and cached: apps cannot be installed while this process is in the
//! foreground, so a later scan would return the same set.

use std::ffi::CString;
use std::fs;
use std::string::String;
use std::vec::Vec;

/// One installed title, as read from its `param.sfo`.
struct Title {
    title_id: String,
    title: String,
}

static mut CACHE: Option<Vec<Title>> = None;
/// Icon texture handle per title id (-1 = no usable icon). Handles belong to
/// the process-lifetime `Ui` of a single-app VPK.
static mut ICONS: Vec<(String, i32)> = Vec::new();

/// Title id of this VPK, when the build supplied one. It is omitted from the
/// list so the library never offers to launch itself.
const SELF_TITLE_ID: &str = match option_env!("VITA_DEFAULT_TITLE_ID") {
    Some(id) => id,
    None => "",
};

const APP_ROOT: &str = "ux0:/app";
/// Largest `icon0.png` the host will read; Vita icons are 128x128 and a few
/// tens of KiB.
const ICON_MAX: usize = 2 * 1024 * 1024;
/// `param.sfo` files are a few KiB; anything larger is not a valid SFO.
const SFO_MAX: usize = 256 * 1024;
/// Flags value used by every launcher that calls sceAppMgrLaunchAppByUri.
const LAUNCH_FLAGS: i32 = 0xFFFFF;

/// Read the UTF-8 string entry `key` from an SFO image.
///
/// Layout: 20-byte header (`\0PSF`, version, key table offset, data table
/// offset, entry count) followed by 16-byte entries of
/// (key offset u16, format u16, length u32, max length u32, data offset u32).
fn sfo_string(sfo: &[u8], key: &str) -> Option<String> {
    let u16_at = |at: usize| sfo.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |at: usize| {
        sfo.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if sfo.get(..4)? != b"\0PSF" {
        return None;
    }
    let key_table = u32_at(8)? as usize;
    let data_table = u32_at(12)? as usize;
    let count = u32_at(16)? as usize;
    for index in 0..count.min(256) {
        let entry = 20 + index * 16;
        let key_offset = key_table + u16_at(entry)? as usize;
        let name_end = sfo.get(key_offset..)?.iter().position(|&byte| byte == 0)?;
        if &sfo[key_offset..key_offset + name_end] != key.as_bytes() {
            continue;
        }
        // 0x0204 is the NUL-terminated UTF-8 string format.
        if u16_at(entry + 2)? != 0x0204 {
            return None;
        }
        let length = u32_at(entry + 4)? as usize;
        let start = data_table + u32_at(entry + 12)? as usize;
        let raw = sfo.get(start..start.checked_add(length)?)?;
        let text = raw.split(|&byte| byte == 0).next()?;
        return String::from_utf8(text.to_vec()).ok();
    }
    None
}

/// A title id is exactly nine ASCII letters or digits (e.g. `PCSE00000`).
fn valid_title_id(id: &str) -> bool {
    id.len() == 9 && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn scan() -> Vec<Title> {
    let mut titles = Vec::new();
    let Ok(entries) = fs::read_dir(APP_ROOT) else {
        return titles;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(String::from) else {
            continue;
        };
        if !valid_title_id(&name) || name == SELF_TITLE_ID {
            continue;
        }
        let path = format!("{APP_ROOT}/{name}/sce_sys/param.sfo");
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if meta.len() as usize > SFO_MAX {
            continue;
        }
        let Ok(sfo) = fs::read(&path) else {
            continue;
        };
        // CATEGORY "gd…" is an application; patches, DLC and system content
        // use other prefixes and are not launchable from here.
        if !sfo_string(&sfo, "CATEGORY").is_some_and(|category| category.starts_with("gd")) {
            continue;
        }
        let title = sfo_string(&sfo, "TITLE")
            .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| name.clone());
        titles.push(Title {
            title_id: name,
            title,
        });
    }
    titles.sort_by_key(|item| (item.title.to_lowercase(), item.title_id.clone()));
    titles
}

unsafe fn titles() -> &'static [Title] {
    if CACHE.is_none() {
        CACHE = Some(scan());
    }
    CACHE.as_deref().unwrap_or(&[])
}

/// Whether the scan found a title with this id.
pub unsafe fn listed(title_id: &str) -> bool {
    titles().iter().any(|item| item.title_id == title_id)
}

fn push_json_str(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            character if (character as u32) < 0x20 => {
                let value = character as u32;
                let hex = b"0123456789abcdef";
                output.push_str("\\u00");
                output.push(hex[(value >> 4) as usize] as char);
                output.push(hex[(value & 0x0f) as usize] as char);
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

/// spec op 39 (native flavour): `{ kind, apps: [{output, id, title, installed}],
/// current, resume }`. `output` and `id` are both the title id.
pub unsafe fn table_json() -> String {
    let mut output = String::from("{\"kind\":\"native\",\"apps\":[");
    for (index, item) in titles().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"output\":");
        push_json_str(&mut output, &item.title_id);
        output.push_str(",\"id\":");
        push_json_str(&mut output, &item.title_id);
        output.push_str(",\"title\":");
        push_json_str(&mut output, &item.title);
        output.push_str(",\"installed\":true}");
    }
    output.push_str("],\"current\":");
    push_json_str(&mut output, SELF_TITLE_ID);
    output.push_str(",\"resume\":null}");
    output
}

/// spec op 40 (native flavour): start the title with this id. Only ids found by
/// the scan are accepted, so the URI is never built from guest-controlled text.
/// Returns whether the app manager accepted the request.
pub unsafe fn launch(title_id: &str) -> bool {
    if !valid_title_id(title_id) || !titles().iter().any(|item| item.title_id == title_id) {
        return false;
    }
    let Ok(uri) = CString::new(format!("psgm:play?titleid={title_id}")) else {
        return false;
    };
    vitasdk_sys::sceAppMgrLaunchAppByUri(LAUNCH_FLAGS, uri.as_ptr()) >= 0
}

/// Decode `sce_sys/icon0.png` to tightly packed RGBA. The core only accepts
/// power-of-two textures up to `TEX_MAX_DIM`; other sizes have no icon.
fn decode_icon(title_id: &str) -> Option<(u32, u32, Vec<u8>)> {
    let path = format!("{APP_ROOT}/{title_id}/sce_sys/icon0.png");
    if fs::metadata(&path).ok()?.len() as usize > ICON_MAX {
        return None;
    }
    let bytes = fs::read(&path).ok()?;
    let mut decoder = png::Decoder::new(&bytes[..]);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).ok()?;
    let (width, height) = (info.width, info.height);
    let pow2 = |value: u32| {
        value > 0 && value <= pocketjs_core::spec::TEX_MAX_DIM && value & (value - 1) == 0
    };
    if !pow2(width) || !pow2(height) {
        return None;
    }
    let pixels = buffer.get(..info.buffer_size())?;
    let count = width as usize * height as usize;
    let mut rgba = Vec::with_capacity(count * 4);
    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(pixels.get(..count * 4)?),
        png::ColorType::Rgb => {
            for px in pixels.chunks_exact(3).take(count) {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in pixels.chunks_exact(2).take(count) {
                rgba.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for &value in pixels.iter().take(count) {
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        png::ColorType::Indexed => return None,
    }
    (rgba.len() == count * 4).then_some((width, height, rgba))
}

/// spec op 52: texture handle of a listed title's icon, -1 when the id was
/// not found by the scan or its icon cannot be decoded. The first call for a
/// title decodes and uploads; later calls return the cached handle.
pub unsafe fn icon(ui: &mut pocketjs_core::Ui, title_id: &str) -> i32 {
    if !titles().iter().any(|item| item.title_id == title_id) {
        return -1;
    }
    if let Some((_, handle)) = ICONS.iter().find(|(id, _)| id == title_id) {
        return *handle;
    }
    let handle = match decode_icon(title_id) {
        Some((width, height, rgba)) => ui.upload_texture_flags(
            &rgba,
            width,
            height,
            pocketjs_core::spec::psm::PSM_8888,
            pocketjs_core::spec::img::FLAG_LINEAR,
        ),
        None => -1,
    };
    if handle >= 0 {
        crate::graphics::register_texture(ui, handle);
    }
    ICONS.push((String::from(title_id), handle));
    handle
}
