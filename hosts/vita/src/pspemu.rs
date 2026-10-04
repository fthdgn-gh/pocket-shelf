//! PSP and PS1 Classics for the installed-title list (cargo feature
//! `installed-apps`).
//!
//! The Vita's own PSP emulator keeps each installed PSP or PS1 title in
//! `ux0:/pspemu/PSP/GAME/<id>/EBOOT.PBP`. A title installed as an official
//! package has a bubble on the home screen, and so a row in the home screen's
//! database (`appdb.rs`) with the folder's id: that gives its name and icon,
//! and `psgm:play?titleid=<id>` starts it in the Vita's emulator, without
//! Adrenaline. A folder with no bubble (a game installed for Adrenaline) is
//! listed too: its name comes from the PBP's `PARAM.SFO`, its icon and
//! picture from the PBP's `ICON0.PNG` and `PIC1.PNG`, and it is started
//! through Adrenaline. The `CATEGORY` in the `PARAM.SFO` tells a PS1 title
//! (`ME`) from a PSP one.
//!
//! PSP games in disc images (`ux0:/pspemu/ISO`, `.iso` and `.cso`, and one
//! level of subfolders) are listed too, by the `DISC_ID` and `TITLE` of the
//! image's `PSP_GAME/PARAM.SFO` (`iso.rs`), and started through Adrenaline.

use crate::appdb::{self, report_value};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::string::String;
use std::vec::Vec;

/// Adrenaline's memory stick, `ms0:` to the PSP side: `ux0:/pspemu` unless
/// Adrenaline's settings name another place (`adrenaline_config.rs`). Read
/// once per process: Adrenaline cannot run, and change it, while this does.
fn ms0() -> &'static str {
    static STICK: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    STICK.get_or_init(|| {
        ["ux0:/data/PSPEMUCFW/adrenaline.bin", "ux0:/app/PSPEMUCFW/adrenaline.bin"]
            .iter()
            .find_map(|path| fs::read(path).ok())
            .and_then(|bytes| crate::adrenaline_config::stick_folder(&bytes))
            .unwrap_or(crate::adrenaline_config::DEFAULT_STICK)
    })
}

/// A path on the memory stick, such as `ISO/Game.cso`, as the Vita sees it.
fn on_stick(path: &str) -> String {
    format!("{}/{path}", ms0())
}
/// `PARAM.SFO` sections are a few KiB; anything larger is not one.
const SFO_MAX: u32 = 256 * 1024;
/// `ICON0.PNG` (144x80) and `PIC1.PNG` (480x272) are tens to hundreds of KiB.
const PNG_MAX: u32 = 4 * 1024 * 1024;
/// Left-out folders and files of the first one named in the scan report.
const SKIPPED_SHOWN: usize = 6;
const FILES_SHOWN: usize = 10;
/// Side of the square icon texture a PBP icon is fitted into.
const ICON_SIDE: u32 = 128;

/// Sections of an `EBOOT.PBP`, in header order.
const SECTION_SFO: usize = 0;
const SECTION_ICON0: usize = 1;
const SECTION_PIC1: usize = 4;

/// What the last scan found, for the guest's diagnostics screen.
static mut REPORT: String = String::new();

/// A listed PSP or PS1 title.
pub struct Classic {
    pub title_id: String,
    pub title: String,
    /// `"ps1"` or `"psp"`.
    pub platform: &'static str,
    /// Whether the title has a bubble on the home screen. One without is
    /// started through Adrenaline.
    pub bubble: bool,
    /// A disc image's path under Adrenaline's memory stick (`ISO/<name>.cso`);
    /// None for a title in `PSP/GAME`.
    pub image: Option<String>,
}

/// Where a title without a bubble keeps its icon and picture.
pub enum Files {
    /// `PSP/GAME/<title id>/EBOOT.PBP`.
    Pbp(String),
    /// A disc image, by its path under the memory stick (`ISO/<name>.iso`).
    Image(String),
}

/// Section `index` of an `EBOOT.PBP`: the header is `\0PBP`, a version,
/// then eight section offsets (`PARAM.SFO`, `ICON0.PNG`, `ICON1.PMF`,
/// `PIC0.PNG`, `PIC1.PNG`, `SND0.AT3`, `DATA.PSP`, `DATA.PSAR`); a section
/// runs from its offset to the next one. None when the section is empty or
/// larger than `max`.
fn pbp_section(path: &str, index: usize, max: u32) -> Option<Vec<u8>> {
    let mut file = fs::File::open(path).ok()?;
    let mut header = [0u8; 40];
    file.read_exact(&mut header).ok()?;
    if &header[..4] != b"\0PBP" {
        return None;
    }
    let offset = |index: usize| u32::from_le_bytes(header[8 + index * 4..12 + index * 4].try_into().unwrap());
    let (start, end) = (offset(index), offset(index + 1));
    if end <= start || end - start > max {
        return None;
    }
    file.seek(SeekFrom::Start(start as u64)).ok()?;
    let mut bytes = vec![0u8; (end - start) as usize];
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

/// The `PARAM.SFO` of a folder's `EBOOT.PBP`, or why there is none, for the
/// scan report: `noEboot`, `short`, `notPbp <first bytes>`, `sfoSpan
/// <start>-<end>`, `sfoRead` or `notSfo`.
fn read_sfo(title_id: &str) -> Result<Vec<u8>, String> {
    let mut file = fs::File::open(eboot_path(title_id)).map_err(|_| String::from("noEboot"))?;
    let mut header = [0u8; 40];
    file.read_exact(&mut header).map_err(|_| String::from("short"))?;
    if &header[..4] != b"\0PBP" {
        let bytes: Vec<String> = header[..4].iter().map(|byte| format!("{byte:02x}")).collect();
        return Err(format!("notPbp {}", bytes.join("")));
    }
    let offset = |index: usize| u32::from_le_bytes(header[8 + index * 4..12 + index * 4].try_into().unwrap());
    let (start, end) = (offset(SECTION_SFO), offset(SECTION_SFO + 1));
    if end <= start || end - start > SFO_MAX {
        return Err(format!("sfoSpan {start}-{end}"));
    }
    file.seek(SeekFrom::Start(start as u64)).map_err(|_| String::from("sfoRead"))?;
    let mut sfo = vec![0u8; (end - start) as usize];
    file.read_exact(&mut sfo).map_err(|_| String::from("sfoRead"))?;
    if !sfo.starts_with(b"\0PSF") {
        return Err(String::from("notSfo"));
    }
    Ok(sfo)
}

fn eboot_path(title_id: &str) -> String {
    on_stick(&format!("PSP/GAME/{title_id}/EBOOT.PBP"))
}

/// The game's path as Adrenaline sees it: `ms0:` is `ux0:/pspemu`.
pub fn ms0_eboot_path(title_id: &str) -> String {
    format!("ms0:/PSP/GAME/{title_id}/EBOOT.PBP")
}

/// The picture a title without a bubble shows behind the list: the PBP's or
/// the disc's `PIC1.PNG`. Callable from any thread.
pub fn picture(files: &Files) -> Option<Vec<u8>> {
    match files {
        Files::Pbp(title_id) => pbp_section(&eboot_path(title_id), SECTION_PIC1, PNG_MAX),
        Files::Image(path) => crate::iso::picture(&on_stick(path), "PIC1.PNG"),
    }
}

/// The icon of a title without a bubble: the PBP's or the disc's `ICON0.PNG`
/// (144x80 for a PSP game, 80x80 for most PS1 ones), fitted whole into a
/// square texture with clear bars, as RGBA. Callable from any thread.
pub fn icon(files: &Files) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = match files {
        Files::Pbp(title_id) => pbp_section(&eboot_path(title_id), SECTION_ICON0, PNG_MAX)?,
        Files::Image(path) => crate::iso::picture(&on_stick(path), "ICON0.PNG")?,
    };
    let mut decoder = png::Decoder::new(&bytes[..]);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).ok()?;
    let channels = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };
    Some((ICON_SIDE, ICON_SIDE, fit_square(&buffer, info.width, info.height, info.line_size, channels)?))
}

/// Scale a picture to fit `ICON_SIDE` square, centered, each texel the
/// average of the source pixels it covers; the bars are clear.
fn fit_square(pixels: &[u8], width: u32, height: u32, line_size: usize, channels: usize) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let side = ICON_SIDE as u64;
    let long = width.max(height) as u64;
    let (out_w, out_h) = ((width as u64 * side / long).max(1), (height as u64 * side / long).max(1));
    let (left, top) = ((side - out_w) / 2, (side - out_h) / 2);
    let mut rgba = vec![0u8; (side * side * 4) as usize];
    for y in 0..out_h {
        let (y0, y1) = (y * height as u64 / out_h, ((y + 1) * height as u64 / out_h).max(y * height as u64 / out_h + 1));
        for x in 0..out_w {
            let (x0, x1) = (x * width as u64 / out_w, ((x + 1) * width as u64 / out_w).max(x * width as u64 / out_w + 1));
            let mut sum = [0u64; 4];
            for sy in y0..y1 {
                let row = pixels.get(sy as usize * line_size..(sy as usize + 1) * line_size)?;
                for sx in x0..x1 {
                    let pixel = row.get(sx as usize * channels..(sx as usize + 1) * channels)?;
                    let (r, g, b, a) = match channels {
                        4 => (pixel[0], pixel[1], pixel[2], pixel[3]),
                        3 => (pixel[0], pixel[1], pixel[2], 255),
                        2 => (pixel[0], pixel[0], pixel[0], pixel[1]),
                        _ => (pixel[0], pixel[0], pixel[0], 255),
                    };
                    for (total, value) in sum.iter_mut().zip([r, g, b, a]) {
                        *total += value as u64;
                    }
                }
            }
            let count = (y1 - y0) * (x1 - x0);
            let at = (((top + y) * side + left + x) * 4) as usize;
            for channel in 0..4 {
                rgba[at + channel] = (sum[channel] / count) as u8;
            }
        }
    }
    Some(rgba)
}

/// The PSP and PS1 titles to list. `rows` is the home screen's database (None
/// when it could not be read: then every title counts as having no bubble);
/// `known` holds the ids already listed.
pub unsafe fn scan(rows: Option<&[appdb::Bubble]>, known: &[String]) -> Vec<Classic> {
    let mut folders: Vec<String> = fs::read_dir(on_stick("PSP/GAME"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.metadata().is_ok_and(|meta| meta.is_dir()))
                .filter_map(|entry| entry.file_name().to_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    folders.sort();
    // Folders not named by a title id (homebrew) are left out.
    let (ids, other_names): (Vec<String>, Vec<String>) =
        folders.into_iter().partition(|name| crate::installed::valid_title_id(name));
    let mut found = Vec::new();
    let mut without_bubble = Vec::new();
    // Folders left out, with the reason, for the report.
    let mut skipped: Vec<(String, String)> = Vec::new();
    for id in &ids {
        if known.contains(id) {
            skipped.push((id.clone(), String::from("listedElsewhere")));
            continue;
        }
        // A folder with no readable PBP (an update or DLC folder, a
        // half-copied game) is not a title.
        let sfo = match read_sfo(id) {
            Ok(sfo) => sfo,
            Err(reason) => {
                skipped.push((id.clone(), reason));
                continue;
            }
        };
        let row = rows.and_then(|rows| rows.iter().find(|row| &row.title_id == id));
        if row.is_none() {
            without_bubble.push(id.clone());
        }
        let title = row
            .map(|row| row.title.clone())
            .filter(|title| !title.is_empty())
            .or_else(|| crate::installed::sfo_string(&sfo, "TITLE").map(|title| appdb::one_line(&title)))
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| id.clone());
        let category = crate::installed::sfo_string(&sfo, "CATEGORY");
        let platform = if category.as_deref() == Some("ME") { "ps1" } else { "psp" };
        found.push(Classic { title_id: id.clone(), title, platform, bubble: row.is_some(), image: None });
    }
    let ps1 = found.iter().filter(|item| item.platform == "ps1").count();
    REPORT = format!(
        "PSP stick={},PSP folders={},PSP titles={},PSP ps1={ps1},PSP psp={},PSP noBubble={}",
        report_value(ms0()),
        ids.len(),
        found.len(),
        found.len() - ps1,
        without_bubble.len()
    );
    if rows.is_none() {
        REPORT.push_str(",PSP source=none");
    }
    if let Some(first) = without_bubble.first() {
        REPORT.push_str(&format!(",PSP firstNoBubble={}", report_value(first)));
    }
    if let Some(first) = other_names.first() {
        REPORT.push_str(&format!(",PSP otherNames={},PSP firstOtherName={}", other_names.len(), report_value(first)));
    }
    // Why folders were left out: the first few, then the files in the first
    // one, which show what it holds in place of an EBOOT.PBP.
    if !skipped.is_empty() {
        REPORT.push_str(&format!(",PSP skipped={}", skipped.len()));
        for (id, reason) in skipped.iter().take(SKIPPED_SHOWN) {
            REPORT.push_str(&format!(",PSP skip={} {}", report_value(id), report_value(reason)));
        }
        let files = appdb::folder_files(&on_stick(&format!("PSP/GAME/{}", skipped[0].0)));
        for (name, size) in files.iter().take(FILES_SHOWN) {
            REPORT.push_str(&format!(",PSP file={} {}", report_value(name), report_value(size)));
        }
    }
    // A bubble that is none of the listed titles says where else titles live.
    let rows = rows.unwrap_or(&[]);
    let listed = |id: &str| known.iter().any(|known| known == id) || found.iter().any(|item| item.title_id == id);
    let others: Vec<&str> = rows
        .iter()
        .map(|row| row.title_id.as_str())
        .filter(|id| !id.starts_with("NPXS") && !listed(id))
        .collect();
    if let Some(first) = others.first() {
        REPORT.push_str(&format!(",PSP otherBubbles={},PSP firstOther={}", others.len(), report_value(first)));
    }
    scan_images(known, &mut found);
    crate::vita_log(format_args!("[PocketJS pspemu] {}", REPORT));
    found
}

/// Folder of disc images under the memory stick; its subfolders are read too,
/// one level deep, as Adrenaline's XMB does.
const IMAGE_DIR: &str = "ISO";
/// Image extensions: read here, and known but not read.
const IMAGE_EXTENSIONS: [&str; 2] = ["iso", "cso"];
const OTHER_IMAGE_EXTENSIONS: [&str; 3] = ["zso", "dax", "jso"];

/// Paths under the memory stick of the files in `ISO/` and its subfolders.
fn image_files() -> Vec<String> {
    let mut files = Vec::new();
    let mut folders = vec![String::from(IMAGE_DIR)];
    let mut depth = 0;
    while depth < 2 && !folders.is_empty() {
        let mut next = Vec::new();
        for folder in folders {
            let Ok(entries) = fs::read_dir(on_stick(&folder)) else {
                continue;
            };
            for entry in entries.flatten() {
                let Some(name) = entry.file_name().to_str().map(String::from) else {
                    continue;
                };
                let path = format!("{folder}/{name}");
                if entry.metadata().is_ok_and(|meta| meta.is_dir()) {
                    next.push(path);
                } else {
                    files.push(path);
                }
            }
        }
        folders = next;
        depth += 1;
    }
    files.sort();
    files
}

fn extension(path: &str) -> String {
    path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default()
}

/// Add the games in disc images to `found`. A game's id is its `DISC_ID`; an
/// image whose id is already listed (a second copy, or a game also installed
/// in `PSP/GAME`) is left out. The findings go to the report as `ISO` lines.
unsafe fn scan_images(known: &[String], found: &mut Vec<Classic>) {
    let files = image_files();
    let mut skipped: Vec<(String, String)> = Vec::new();
    let mut images = 0;
    for path in files {
        let ext = extension(&path);
        if OTHER_IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            skipped.push((path, format!("format {ext}")));
            continue;
        }
        if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        images += 1;
        // The path goes into the list file and the boot request, one line each.
        if path.contains(['\t', '\n', '\r']) || path.len() > 200 {
            skipped.push((path, String::from("name")));
            continue;
        }
        let sfo = match crate::iso::param_sfo(&on_stick(&path)) {
            Ok(sfo) => sfo,
            Err(reason) => {
                skipped.push((path, reason));
                continue;
            }
        };
        let Some(id) = crate::installed::sfo_string(&sfo, "DISC_ID").filter(|id| crate::installed::valid_title_id(id)) else {
            skipped.push((path, String::from("noDiscId")));
            continue;
        };
        if known.contains(&id) || found.iter().any(|item| item.title_id == id) {
            skipped.push((path, format!("duplicate {id}")));
            continue;
        }
        let title = crate::installed::sfo_string(&sfo, "TITLE")
            .map(|title| appdb::one_line(&title))
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| id.clone());
        found.push(Classic { title_id: id, title, platform: "psp", bubble: false, image: Some(path) });
    }
    let listed = found.iter().filter(|item| item.image.is_some()).count();
    REPORT.push_str(&format!(",ISO files={images},ISO titles={listed}"));
    if !skipped.is_empty() {
        REPORT.push_str(&format!(",ISO skipped={}", skipped.len()));
        for (path, reason) in skipped.iter().take(SKIPPED_SHOWN) {
            REPORT.push_str(&format!(",ISO skip={} {}", report_value(path), report_value(reason)));
        }
    }
}

/// What the last scan found among PSP and PS1 titles. Empty before a scan.
pub unsafe fn report() -> String {
    REPORT.clone()
}

// Starting a title without a bubble. Adrenaline loads plugins from
// `ms0:/seplugins/` each time its XMB starts; Pocket Shelf ships one
// (`src/psp-boot/`) that boots the title named in a request file. The guest
// asks for the plugin's state (`plugin_state`) before a launch and, with the
// user's consent, adds it to Adrenaline's lists or turns it back on
// (`enable_plugin`). A launch writes the request and starts Adrenaline. The
// plugin deletes the request before booting, and drops one older than 30
// seconds.

/// Adrenaline's title id.
pub const ADRENALINE_ID: &str = "PSPEMUCFW";
/// The plugin as the VPK carries it.
const PLUGIN_SOURCE: &str = "app0:psp/pocketshelf.prx";
/// The plugin as Adrenaline loads it, and where on the stick the host puts it.
const PLUGIN_MODULE: &str = "ms0:/seplugins/pocketshelf.prx";
const PLUGIN_TARGET: &str = "seplugins/pocketshelf.prx";
/// TheOfficialFloW's Adrenaline reads this list.
const FLOW_LIST: &str = "seplugins/vsh.txt";
/// The request file; the plugin reads it as `ms0:/PocketShelf/boot.txt`.
const REQUEST_DIR: &str = "PocketShelf";
const REQUEST_FILE: &str = "PocketShelf/boot.txt";

/// The list isage's Adrenaline reads: `EPIplugins.txt` when it exists.
fn isage_list() -> String {
    let epi = on_stick("seplugins/EPIplugins.txt");
    if fs::metadata(&epi).is_ok() {
        epi
    } else {
        on_stick("seplugins/plugins.txt")
    }
}

fn adrenaline_installed() -> bool {
    fs::metadata(format!("ux0:/app/{ADRENALINE_ID}/eboot.bin")).is_ok()
}

/// Whether Adrenaline will load the plugin at its XMB: `noAdrenaline`,
/// `missing` (a list does not name it), `off` (a list names it turned off) or
/// `on`. Which Adrenaline is installed is not known, so both versions' lists
/// count: `on` needs both on, and one turned off is `off`.
pub fn plugin_state() -> &'static str {
    use crate::seplugins::{flow_state, isage_state, ListState};
    if !adrenaline_installed() {
        return "noAdrenaline";
    }
    let read = |path: &str| fs::read_to_string(path).unwrap_or_default();
    let states = [
        flow_state(&read(&on_stick(FLOW_LIST)), PLUGIN_MODULE),
        isage_state(&read(&isage_list()), PLUGIN_MODULE),
    ];
    if states.contains(&ListState::Off) {
        "off"
    } else if states.iter().all(|state| *state == ListState::On) {
        "on"
    } else {
        "missing"
    }
}

/// Copy the plugin to `ms0:/seplugins/` when it is missing or differs from
/// the VPK's, so an update of this app updates the plugin.
fn copy_plugin() -> Result<(), String> {
    let plugin = fs::read(PLUGIN_SOURCE).map_err(|error| format!("read {PLUGIN_SOURCE}: {error}"))?;
    let target = on_stick(PLUGIN_TARGET);
    if fs::read(&target).ok().as_deref() != Some(&plugin[..]) {
        fs::create_dir_all(on_stick("seplugins")).map_err(|error| format!("create seplugins: {error}"))?;
        fs::write(&target, &plugin).map_err(|error| format!("write {target}: {error}"))?;
    }
    Ok(())
}

/// Copy the plugin and turn it on in both versions' lists: a list without it
/// gets a line, a line that turns it off is changed to on. The guest calls
/// this once the user agreed. Returns whether the plugin is now on.
pub fn enable_plugin() -> bool {
    use crate::seplugins::{flow_enable, flow_state, isage_enable, isage_state, ListState};
    let change = |path: &str, state: fn(&str, &str) -> ListState, enable: fn(&str, &str) -> String| {
        let text = fs::read_to_string(path).unwrap_or_default();
        if state(&text, PLUGIN_MODULE) == ListState::On {
            return Ok(());
        }
        fs::write(path, enable(&text, PLUGIN_MODULE)).map_err(|error| format!("write {path}: {error}"))
    };
    let result = copy_plugin()
        .and_then(|_| change(&on_stick(FLOW_LIST), flow_state, flow_enable))
        .and_then(|_| change(&isage_list(), isage_state, isage_enable));
    if let Err(error) = result {
        crate::vita_log(format_args!("[PocketJS pspemu] plugin not turned on: {error}"));
        return false;
    }
    plugin_state() == "on"
}

/// Write the request for this title, once the plugin is on (`plugin_state`).
/// `platform` is `psp` or `ps1`; `image` is a disc image's path under the
/// memory stick, for a game in one. Called once the launch is accepted, before the frame that
/// shows it; the request goes to Adrenaline in `installed::finish_launch`.
pub unsafe fn request_boot(title_id: &str, platform: &str, image: Option<&str>) -> bool {
    if plugin_state() != "on" {
        return false;
    }
    if let Err(error) = copy_plugin() {
        crate::vita_log(format_args!("[PocketJS pspemu] plugin not copied: {error}"));
        return false;
    }
    let mut tick = vitasdk_sys::SceRtcTick { tick: 0 };
    if vitasdk_sys::sceRtcGetCurrentTick(&mut tick) < 0 {
        return false;
    }
    let request = match image {
        // The plugin mounts the image and boots the file named on the fourth
        // line, or BOOT.BIN when the fifth says the image has one and the
        // user's Adrenaline setting asks for it.
        Some(path) => {
            let (boot, boot_bin) = crate::iso::boot_files(&on_stick(path));
            format!(
                "{}\niso\nms0:/{path}\n{boot}\n{}\n",
                tick.tick,
                if boot_bin { "BOOT.BIN" } else { "-" }
            )
        }
        None => format!("{}\n{platform}\n{}\n", tick.tick, ms0_eboot_path(title_id)),
    };
    let written = fs::create_dir_all(on_stick(REQUEST_DIR)).and_then(|_| fs::write(on_stick(REQUEST_FILE), request));
    if let Err(error) = written {
        crate::vita_log(format_args!("[PocketJS pspemu] request not written: {error}"));
        return false;
    }
    true
}

/// What the plugin wrote to `ms0:/PocketShelf/boot.log`: its last lines,
/// newest last, for the diagnostics screen.
pub fn boot_log() -> String {
    const LINES: usize = 6;
    let text = fs::read_to_string(on_stick(&format!("{REQUEST_DIR}/boot.log"))).unwrap_or_default();
    let lines: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();
    lines[lines.len().saturating_sub(LINES)..].join("\n")
}

/// Delete a request the plugin did not take: after a launch the system
/// refused, and when this app starts (a launch that reached Adrenaline's XMB
/// ended this process, so a request still there never got to the plugin).
pub fn forget_request() {
    let _ = fs::remove_file(on_stick(REQUEST_FILE));
}
