//! RetroArch games (cargo feature `installed-apps`): the entries of the
//! user's RetroArch playlists, each listed as a title of its own and started
//! with the core the playlist names for it.
//!
//! RetroArch for the Vita (title id `RETROVITA`) starts through `eboot.bin`,
//! its "salamander" stub. The stub reads the launch URI's `&param=<core>` and
//! `&param2=<content>` (`frontend_psp_exec` in RetroArch's
//! `frontend/drivers/platform_psp.c`) and runs the core's own executable,
//! `app0:/<name>_libretro.self`, with the content path as its argument. It
//! uses `param2` only when `param` is present, and it cuts each value at the
//! next `&`. Without `param` it opens the last core used and no content, so
//! every listed game needs a core.
//!
//! The playlists are RetroArch's own (`playlists/*.lpl` in its data folder,
//! JSON since RetroArch 1.7.6, six lines per entry before that). With
//! RetroArch's "Compress Playlists" setting on, each file is that text in
//! RZIP form (`decode_rzip`). A game's
//! console is its entry's `db_name`, else the playlist's file name, which
//! RetroArch names after its database (`Nintendo - Game Boy.lpl`); that is
//! also the folder its thumbnails are in.
//!
//! This file uses std, serde_json and miniz_oxide only, so its tests run on
//! the build machine: `bun run test` (platform-tests).

use std::fs;
use std::string::String;
use std::vec::Vec;

/// RetroArch's title id on the Vita.
pub const RETROARCH_ID: &str = "RETROVITA";

/// Where RetroArch is installed and keeps its data. The tests point both at a
/// temporary folder.
pub struct Roots {
    /// The installed app: `eboot.bin`, the cores, `info/`.
    pub app: String,
    /// The data folder: `retroarch.cfg`, `playlists/`, `thumbnails/`.
    pub data: String,
}

impl Roots {
    pub fn vita() -> Roots {
        Roots {
            app: String::from("ux0:/app/RETROVITA"),
            data: String::from("ux0:/data/retroarch"),
        }
    }

    /// Whether RetroArch is installed: its `eboot.bin` is there.
    pub fn installed(&self) -> bool {
        is_file(&format!("{}/eboot.bin", self.app))
    }
}

/// One game from a playlist.
pub struct Game {
    /// `ra` and seven base-36 digits of a hash of `path` (`game_id`).
    pub id: String,
    pub title: String,
    /// The console's database name, `Nintendo - Super Nintendo Entertainment System`.
    pub console: String,
    /// The core's file name in RetroArch's app folder, `snes9x_libretro.self`.
    pub core: String,
    /// The content path RetroArch opens, as the playlist has it.
    pub path: String,
}

/// What a scan found.
pub struct Scan {
    pub games: Vec<Game>,
    /// `Label=value` pairs separated by commas, for the diagnostics screen.
    pub report: String,
}

/// The longest launch URI sent. The system's limit is not known; RetroArch
/// reads the parameters into a buffer of `PATH_MAX_LENGTH` bytes.
pub const URI_MAX: usize = 1000;

/// How many skipped entries the report names.
const SKIPS_SHOWN: usize = 6;

/// The URI that starts `path` with the core `core` (a file name in
/// RetroArch's app folder).
pub fn launch_uri(core: &str, path: &str) -> String {
    format!("psgm:play?titleid={RETROARCH_ID}&param=app0:/{core}&param2={path}")
}

/// The id a game is listed under: `ra` and seven base-36 digits of the FNV-1a
/// hash of its content path. Nine letters and digits like a title id, and
/// lowercase, which the system's own ids never are. The same file keeps its
/// id across scans, so the user's changes to it (category, art) stay.
pub fn game_id(path: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut id = String::from("ra");
    for _ in 0..7 {
        id.push(digits[(hash % 36) as usize] as char);
        hash /= 36;
    }
    id
}

/// Whether `id` has the shape `game_id` gives.
pub fn is_game_id(id: &str) -> bool {
    id.len() == 9
        && id.starts_with("ra")
        && id.bytes().skip(2).all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase())
}

/// The value of `key` in a `retroarch.cfg` text (`key = "value"` lines).
pub fn config_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (name, value) = line.trim().split_once('=')?;
        if name.trim() != key {
            return None;
        }
        let value = value.trim();
        let value = value.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')).unwrap_or(value);
        Some(String::from(value))
    })
}

/// `text` without the prefix `prefix`, compared ignoring ASCII case.
fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix).then(|| &text[prefix.len()..])
}

/// A folder named in `retroarch.cfg`, as a path this process can open.
/// `app0:` and RetroArch's `:` (its own folder) are RetroArch's app folder.
fn resolve(roots: &Roots, path: &str) -> String {
    let path = path.trim_end_matches('/');
    if let Some(rest) = strip_prefix_ci(path, "app0:").or_else(|| path.strip_prefix(':')) {
        let rest = rest.trim_start_matches('/');
        return if rest.is_empty() { roots.app.clone() } else { format!("{}/{rest}", roots.app) };
    }
    String::from(path)
}

/// A folder setting of `retroarch.cfg`, or `default` under the data folder
/// when the file does not set it (RetroArch writes `default` for unset ones).
fn directory(roots: &Roots, config: &str, key: &str, default: &str) -> String {
    match config_value(config, key) {
        Some(value) if !value.is_empty() && value != "default" => resolve(roots, &value),
        _ => format!("{}/{default}", roots.data),
    }
}

fn read_config(roots: &Roots) -> String {
    fs::read_to_string(format!("{}/retroarch.cfg", roots.data)).unwrap_or_default()
}

/// The folder of the playlists.
pub fn playlist_dir(roots: &Roots) -> String {
    directory(roots, &read_config(roots), "playlist_directory", "playlists")
}

/// The folder of the thumbnails.
pub fn thumbnail_dir(roots: &Roots) -> String {
    directory(roots, &read_config(roots), "thumbnails_directory", "thumbnails")
}

fn is_file(path: &str) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.is_file())
}

/// The file name of a core in RetroArch's app folder that exists, from a
/// playlist's core path (`app0:/snes9x_libretro.self`, or the same file by
/// its `ux0:` path). None for `DETECT`, an empty path, a core elsewhere (the
/// stub only runs executables from its own folder) or a missing file.
pub fn core_file(roots: &Roots, core_path: &str) -> Option<String> {
    let app_path = roots.app.trim_end_matches('/');
    let rest = strip_prefix_ci(core_path, "app0:").or_else(|| strip_prefix_ci(core_path, app_path))?;
    let name = rest.trim_start_matches('/');
    if name.is_empty() || name.contains(['/', '\\', '&']) {
        return None;
    }
    is_file(&format!("{app_path}/{name}")).then(|| String::from(name))
}

/// An installed core and the databases (consoles) its info file names.
pub struct CoreInfo {
    pub file: String,
    pub databases: Vec<String>,
}

/// The cores in RetroArch's app folder (`<name>_libretro.self`), by name,
/// each with the `database` list of `info/<name>_libretro.info`.
pub fn installed_cores(roots: &Roots) -> Vec<CoreInfo> {
    let mut files: Vec<String> = fs::read_dir(&roots.app)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(String::from))
                .filter(|name| name.ends_with("_libretro.self"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
        .into_iter()
        .map(|file| {
            let stem = file.trim_end_matches(".self");
            let info = fs::read_to_string(format!("{}/info/{stem}.info", roots.app)).unwrap_or_default();
            let databases = config_value(&info, "database")
                .map(|list| list.split('|').map(|name| String::from(name.trim())).filter(|name| !name.is_empty()).collect())
                .unwrap_or_default();
            CoreInfo { file, databases }
        })
        .collect()
}

/// One entry of a playlist, its fields as RetroArch writes them.
#[derive(Default)]
pub struct Entry {
    pub path: String,
    pub label: String,
    pub core_path: String,
    pub db_name: String,
}

pub struct Playlist {
    /// The core for entries whose own core is `DETECT` or empty.
    pub default_core: String,
    pub entries: Vec<Entry>,
}

/// A playlist file's text: JSON (`{"version": ..., "items": [...]}`), or the
/// old format of six lines per entry (path, label, core path, core name,
/// CRC, database name); old files end with fewer lines of settings, which
/// are not read.
pub fn parse_playlist(text: &str) -> Option<Playlist> {
    let text = text.trim_start_matches('\u{feff}');
    if text.trim_start().starts_with('{') {
        let value: serde_json::Value = serde_json::from_str(text).ok()?;
        let field = |object: &serde_json::Value, key: &str| {
            object.get(key).and_then(|value| value.as_str()).map(String::from).unwrap_or_default()
        };
        let entries = value
            .get("items")
            .and_then(|items| items.as_array())
            .map(|items| {
                items
                    .iter()
                    .map(|item| Entry {
                        path: field(item, "path"),
                        label: field(item, "label"),
                        core_path: field(item, "core_path"),
                        db_name: field(item, "db_name"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        return Some(Playlist { default_core: field(&value, "default_core_path"), entries });
    }
    let lines: Vec<&str> = text.lines().map(|line| line.trim_end_matches('\r')).collect();
    let entries = lines
        .chunks_exact(6)
        .map(|group| Entry {
            path: String::from(group[0]),
            label: String::from(group[1]),
            core_path: String::from(group[2]),
            db_name: String::from(group[5]),
        })
        .collect();
    Some(Playlist { default_core: String::new(), entries })
}

/// The largest playlist text read: a playlist of thousands of games is a few
/// megabytes.
const PLAYLIST_MAX: u64 = 16 * 1024 * 1024;

/// The text of an RZIP file (RetroArch's `libretro-common/streams/rzip_stream.c`):
/// a 20-byte header (`#RZIPv`, the version, `#`, the chunk size as a 32-bit
/// and the whole text's size as a 64-bit little-endian number), then chunks,
/// each a 32-bit little-endian length and that many bytes of one complete
/// zlib stream. Version 1 is zlib; version 2 (zstd) is not read. None when
/// `bytes` is not RZIP; Err with the reason when it is and cannot be read.
pub fn decode_rzip(bytes: &[u8]) -> Option<Result<Vec<u8>, &'static str>> {
    let header = bytes.get(..20)?;
    if &header[..6] != b"#RZIPv" || header[7] != b'#' {
        return None;
    }
    Some(match header[6] {
        1 => inflate_chunks(header, &bytes[20..]),
        2 => Err("rzipZstd"),
        _ => Err("rzipVersion"),
    })
}

fn inflate_chunks(header: &[u8], mut rest: &[u8]) -> Result<Vec<u8>, &'static str> {
    let chunk = u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
    let mut size_bytes = [0u8; 8];
    size_bytes.copy_from_slice(&header[12..20]);
    let size = u64::from_le_bytes(size_bytes);
    if chunk == 0 || size == 0 || size > PLAYLIST_MAX {
        return Err("rzipSize");
    }
    let size = size as usize;
    let mut text = Vec::with_capacity(size);
    while text.len() < size {
        let length = rest.get(..4).ok_or("rzipShort")?;
        let length = u32::from_le_bytes([length[0], length[1], length[2], length[3]]) as usize;
        let data = rest.get(4..4 + length).ok_or("rzipShort")?;
        let part = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, chunk).map_err(|_| "rzipInflate")?;
        if part.is_empty() {
            return Err("rzipInflate");
        }
        text.extend_from_slice(&part);
        rest = &rest[4 + length..];
    }
    if text.len() != size {
        return Err("rzipSize");
    }
    Ok(text)
}

/// A playlist file's text, unpacked when it is RZIP. Err with the reason
/// the report gives when it cannot be read.
pub fn read_playlist(path: &str) -> Result<Playlist, &'static str> {
    if fs::metadata(path).map_err(|_| "open")?.len() > PLAYLIST_MAX {
        return Err("tooLarge");
    }
    let bytes = fs::read(path).map_err(|_| "read")?;
    let bytes = match decode_rzip(&bytes) {
        Some(text) => text?,
        None => bytes,
    };
    let text = String::from_utf8(bytes).map_err(|_| "notUtf8")?;
    parse_playlist(&text).ok_or("notJson")
}

/// A playlist RetroArch keeps for its own menus (history, favorites), not a
/// collection of games of one kind.
fn is_menu_playlist(stem: &str) -> bool {
    stem.starts_with("content_") || stem.starts_with("images_") || stem.starts_with("music_") || stem.starts_with("video_")
}

/// The `.lpl` files of the playlist folder, by name.
fn playlist_files(roots: &Roots) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(playlist_dir(roots))
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(String::from))
                .filter(|name| {
                    name.to_ascii_lowercase().ends_with(".lpl") && !is_menu_playlist(&name[..name.len() - 4])
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// How many steps `scan` reports: one per playlist file.
pub fn item_count(roots: &Roots) -> u32 {
    if roots.installed() {
        playlist_files(roots).len() as u32
    } else {
        0
    }
}

/// A value for a report entry: no `,` or `=`, which separate the entries.
fn report_value(text: &str) -> String {
    text.replace([',', '='], "_")
}

/// The name a title shows: the entry's label, else the content's file name
/// without its extension; whitespace collapsed to single spaces.
fn title_of(entry: &Entry) -> String {
    let text = if entry.label.trim().is_empty() {
        let name = entry.path.rsplit(['/', '#']).next().unwrap_or(&entry.path);
        name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name).to_string()
    } else {
        entry.label.clone()
    };
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Read every playlist and list its games. `step` is called once per
/// playlist file. An entry is left out when it names no file, when no core
/// is found for it, when its path holds `&` (the stub cuts the parameter
/// there), a tab or a line break, when its URI is longer than `URI_MAX`, or
/// when an earlier playlist already listed the same file.
pub fn scan(roots: &Roots, step: &mut dyn FnMut()) -> Scan {
    if !roots.installed() {
        return Scan { games: Vec::new(), report: String::from("RetroArch=notInstalled") };
    }
    let files = playlist_files(roots);
    let mut games: Vec<Game> = Vec::new();
    // The content path of each id listed so far.
    let mut listed: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut skips: Vec<String> = Vec::new();
    let mut skipped = 0usize;
    let mut unreadable = 0usize;
    let mut unreadable_first = String::new();
    // Read only when an entry needs it: every core's info file.
    let mut cores: Option<Vec<CoreInfo>> = None;
    let folder = playlist_dir(roots);
    for file in &files {
        step();
        let stem = &file[..file.len() - 4];
        let playlist = match read_playlist(&format!("{folder}/{file}")) {
            Ok(playlist) => playlist,
            Err(reason) => {
                if unreadable == 0 {
                    unreadable_first = format!("{} {reason}", report_value(file));
                }
                unreadable += 1;
                continue;
            }
        };
        let default_core = core_file(roots, &playlist.default_core);
        for entry in &playlist.entries {
            let mut skip = |reason: &str| {
                skipped += 1;
                if skips.len() < SKIPS_SHOWN {
                    skips.push(format!("RetroArch skip={} {reason}", report_value(&title_of(entry))));
                }
            };
            if entry.path.trim().is_empty() {
                skip("noPath");
                continue;
            }
            if entry.path.contains(['&', '\t', '\n', '\r']) {
                skip("pathChars");
                continue;
            }
            let console = {
                let db = entry.db_name.trim();
                let db = db.strip_suffix(".lpl").unwrap_or(db);
                String::from(if db.is_empty() { stem } else { db })
            };
            if console.contains(['\t', '\n', '\r']) {
                skip("consoleChars");
                continue;
            }
            let core = core_file(roots, &entry.core_path).or_else(|| default_core.clone()).or_else(|| {
                let cores = cores.get_or_insert_with(|| installed_cores(roots));
                cores
                    .iter()
                    .find(|core| core.databases.iter().any(|name| *name == console))
                    .map(|core| core.file.clone())
            });
            let Some(core) = core else {
                skip("noCore");
                continue;
            };
            if launch_uri(&core, &entry.path).len() > URI_MAX {
                skip("long");
                continue;
            }
            let id = game_id(&entry.path);
            if let Some(known) = listed.get(&id) {
                skip(if *known == entry.path { "duplicate" } else { "idTaken" });
                continue;
            }
            listed.insert(id.clone(), entry.path.clone());
            games.push(Game { id, title: title_of(entry), console, core, path: entry.path.clone() });
        }
    }
    let mut report = format!(
        "RetroArch playlists={},RetroArch titles={},RetroArch skipped={skipped}",
        files.len(),
        games.len()
    );
    if unreadable > 0 {
        report.push_str(&format!(",RetroArch unreadable={unreadable},RetroArch firstUnreadable={unreadable_first}"));
    }
    for line in skips {
        report.push(',');
        report.push_str(&line);
    }
    Scan { games, report }
}

/// A thumbnail's file name for a label: RetroArch replaces `&*/:`"<>?\|`
/// with `_` and adds `.png` (`gfx_thumbnail_fill_content_img`).
pub fn thumbnail_name(label: &str) -> String {
    let mut name: String = label.chars().map(|c| if "&*/:`\"<>?\\|".contains(c) { '_' } else { c }).collect();
    name.push_str(".png");
    name
}

/// The thumbnail files to try for a game, in order: each of `kinds`
/// (`Named_Boxarts`, `Named_Snaps`, `Named_Titles`) by the full label, then by
/// the label up to its first ` (` (the short name RetroArch also tries). A
/// MAME game's thumbnails are in `MAME`, whatever its database's full name.
pub fn thumbnail_paths(thumbnails: &str, console: &str, label: &str, kinds: &[&str]) -> Vec<String> {
    let system = if console.starts_with("MAME") { "MAME" } else { console };
    let mut names = vec![thumbnail_name(label)];
    if let Some(at) = label.find(" (").filter(|at| *at > 0) {
        names.push(thumbnail_name(&label[..at]));
    }
    let mut paths = Vec::new();
    for kind in kinds {
        for name in &names {
            paths.push(format!("{thumbnails}/{system}/{kind}/{name}"));
        }
    }
    paths
}

/// The file names, without `.png`, that a thumbnail folder's listing page
/// links to (`<a href="Battleship%20(USA).png">`), percent-decoded, in page
/// order. The page of a console's box art is up to a few megabytes; the names
/// are kept as a short list instead (`http.rs`, `libretro/<console>.txt`).
pub fn index_names(html: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find("href=\"") {
        rest = &rest[at + 6..];
        let Some(end) = rest.find('"') else {
            break;
        };
        let link = &rest[..end];
        rest = &rest[end..];
        let Some(name) = link.strip_suffix(".png") else {
            continue;
        };
        if name.contains('/') || name.starts_with('?') {
            continue;
        }
        if let Some(name) = percent_decode(name) {
            if !name.is_empty() && !name.contains(['\n', '\r']) {
                names.push(name);
            }
        }
    }
    names
}

/// `%XX` escapes decoded as UTF-8; None when the result is not UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = |byte: u8| (byte as char).to_digit(16);
            if let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2])) {
                out.push((high * 16 + low) as u8);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Temp(std::path::PathBuf);

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A RetroArch install and data folder in a temporary folder.
    fn setup(name: &str) -> (Temp, Roots) {
        let base = std::env::temp_dir().join(format!("pocket-retroarch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let roots = Roots {
            app: base.join("app").to_string_lossy().into_owned(),
            data: base.join("data").to_string_lossy().into_owned(),
        };
        fs::create_dir_all(format!("{}/info", roots.app)).unwrap();
        fs::create_dir_all(format!("{}/playlists", roots.data)).unwrap();
        fs::write(format!("{}/eboot.bin", roots.app), b"").unwrap();
        (Temp(base), roots)
    }

    fn add_core(roots: &Roots, name: &str, database: &str) {
        fs::write(format!("{}/{name}_libretro.self", roots.app), b"").unwrap();
        fs::write(
            format!("{}/info/{name}_libretro.info", roots.app),
            format!("corename = \"{name}\"\ndatabase = \"{database}\"\n"),
        )
        .unwrap();
    }

    fn add_playlist(roots: &Roots, name: &str, text: &str) {
        fs::write(format!("{}/playlists/{name}", roots.data), text).unwrap();
    }

    #[test]
    fn ids_are_stable_lowercase_and_nine_long() {
        let id = game_id("ux0:/roms/snes/Game.sfc");
        assert_eq!(id, game_id("ux0:/roms/snes/Game.sfc"));
        assert_ne!(id, game_id("ux0:/roms/snes/Game2.sfc"));
        assert!(is_game_id(&id), "{id}");
        assert!(!is_game_id("PCSE00000"));
    }

    #[test]
    fn reads_config_values() {
        let text = "video_driver = \"gl\"\nplaylist_directory = \"ux0:/data/retroarch/lists\"\nthumbnails_directory = default\n";
        assert_eq!(config_value(text, "playlist_directory").as_deref(), Some("ux0:/data/retroarch/lists"));
        assert_eq!(config_value(text, "thumbnails_directory").as_deref(), Some("default"));
        assert_eq!(config_value(text, "missing"), None);
    }

    #[test]
    fn parses_json_playlists() {
        let text = r#"{
  "version": "1.5",
  "default_core_path": "app0:/snes9x_libretro.self",
  "default_core_name": "Snes9x",
  "items": [
    {"path": "ux0:/roms/snes/A.sfc", "label": "A", "core_path": "DETECT", "core_name": "DETECT", "crc32": "00000000|crc", "db_name": "Nintendo - Super Nintendo Entertainment System.lpl"}
  ]
}"#;
        let playlist = parse_playlist(text).unwrap();
        assert_eq!(playlist.default_core, "app0:/snes9x_libretro.self");
        assert_eq!(playlist.entries.len(), 1);
        assert_eq!(playlist.entries[0].core_path, "DETECT");
        assert_eq!(playlist.entries[0].db_name, "Nintendo - Super Nintendo Entertainment System.lpl");
    }

    #[test]
    fn parses_six_line_playlists() {
        let text = "ux0:/roms/gb/A.gb\nA\napp0:/gambatte_libretro.self\nGambatte\nDETECT\nNintendo - Game Boy.lpl\r\nux0:/roms/gb/B.gb\nB\nDETECT\nDETECT\nDETECT\nNintendo - Game Boy.lpl\n";
        let playlist = parse_playlist(text).unwrap();
        assert_eq!(playlist.entries.len(), 2);
        assert_eq!(playlist.entries[0].core_path, "app0:/gambatte_libretro.self");
        assert_eq!(playlist.entries[1].label, "B");
    }

    #[test]
    fn picks_the_entry_core_then_the_default_then_an_installed_one() {
        let (_temp, roots) = setup("cores");
        add_core(&roots, "snes9x", "Nintendo - Super Nintendo Entertainment System|Nintendo - Satellaview");
        add_core(&roots, "bsnes", "Nintendo - Super Nintendo Entertainment System");
        add_core(&roots, "genesis_plus_gx", "Sega - Game Gear|Sega - Mega Drive - Genesis");
        add_playlist(
            &roots,
            "Nintendo - Super Nintendo Entertainment System.lpl",
            r#"{"default_core_path": "app0:/snes9x_libretro.self", "items": [
              {"path": "ux0:/roms/snes/Own.sfc", "label": "Own", "core_path": "app0:/bsnes_libretro.self", "db_name": ""},
              {"path": "ux0:/roms/snes/Default.sfc", "label": "Default", "core_path": "DETECT", "db_name": ""}
            ]}"#,
        );
        add_playlist(
            &roots,
            "Sega - Game Gear.lpl",
            r#"{"items": [
              {"path": "ux0:/roms/gg/Info.gg", "label": "Info", "core_path": "DETECT", "db_name": "Sega - Game Gear.lpl"},
              {"path": "ux0:/roms/gg/Gone.gg", "label": "Gone", "core_path": "app0:/missing_libretro.self", "db_name": "Sega - Game Gear.lpl"}
            ]}"#,
        );
        let mut steps = 0;
        let scan = scan(&roots, &mut || steps += 1);
        assert_eq!(steps, 2);
        let core = |title: &str| scan.games.iter().find(|game| game.title == title).map(|game| game.core.clone());
        assert_eq!(core("Own").as_deref(), Some("bsnes_libretro.self"));
        assert_eq!(core("Default").as_deref(), Some("snes9x_libretro.self"));
        assert_eq!(core("Info").as_deref(), Some("genesis_plus_gx_libretro.self"));
        // A core that is not installed falls back to one that is.
        assert_eq!(core("Gone").as_deref(), Some("genesis_plus_gx_libretro.self"));
        let console = |title: &str| scan.games.iter().find(|game| game.title == title).unwrap().console.clone();
        assert_eq!(console("Own"), "Nintendo - Super Nintendo Entertainment System");
        assert_eq!(console("Info"), "Sega - Game Gear");
    }

    #[test]
    fn leaves_out_entries_it_cannot_start() {
        let (_temp, roots) = setup("skips");
        add_core(&roots, "gambatte", "Nintendo - Game Boy");
        add_playlist(
            &roots,
            "Nintendo - Game Boy.lpl",
            r#"{"items": [
              {"path": "ux0:/roms/gb/Tom & Jerry.gb", "label": "Tom & Jerry", "core_path": "DETECT"},
              {"path": "ux0:/roms/gb/Ok.gb", "label": "", "core_path": "DETECT"},
              {"path": "", "label": "Empty", "core_path": "DETECT"}
            ]}"#,
        );
        add_playlist(
            &roots,
            "Favorites.lpl",
            r#"{"items": [
              {"path": "ux0:/roms/gb/Ok.gb", "label": "Ok", "core_path": "DETECT", "db_name": "Nintendo - Game Boy.lpl"},
              {"path": "ux0:/roms/nes/Nes.nes", "label": "Nes", "core_path": "DETECT", "db_name": "Nintendo - Nintendo Entertainment System.lpl"}
            ]}"#,
        );
        add_playlist(&roots, "content_history.lpl", r#"{"items": [{"path": "ux0:/x.gb", "core_path": "app0:/gambatte_libretro.self"}]}"#);
        let scan = scan(&roots, &mut || {});
        let titles: Vec<&str> = scan.games.iter().map(|game| game.title.as_str()).collect();
        assert_eq!(titles, vec!["Ok"]);
        assert!(scan.report.starts_with("RetroArch playlists=2,RetroArch titles=1,RetroArch skipped=4"), "{}", scan.report);
        assert!(scan.report.contains("RetroArch skip=Tom & Jerry pathChars"));
        assert!(scan.report.contains("RetroArch skip=Ok duplicate"));
        assert!(scan.report.contains("RetroArch skip=Nes noCore"));
    }

    #[test]
    fn reads_folders_from_the_config() {
        let (_temp, roots) = setup("config");
        add_core(&roots, "gambatte", "Nintendo - Game Boy");
        let lists = format!("{}/lists", roots.data);
        fs::create_dir_all(&lists).unwrap();
        fs::write(format!("{}/retroarch.cfg", roots.data), format!("playlist_directory = \"{lists}/\"\n")).unwrap();
        fs::write(
            format!("{lists}/Nintendo - Game Boy.lpl"),
            r#"{"items": [{"path": "ux0:/roms/gb/A.gb", "label": "A", "core_path": "DETECT"}]}"#,
        )
        .unwrap();
        assert_eq!(scan(&roots, &mut || {}).games.len(), 1);
        assert_eq!(thumbnail_dir(&roots), format!("{}/thumbnails", roots.data));
    }

    /// An RZIP file of `text` in chunks of `chunk` bytes, as RetroArch writes it.
    fn rzip(text: &[u8], chunk: usize) -> Vec<u8> {
        let mut bytes = b"#RZIPv\x01#".to_vec();
        bytes.extend_from_slice(&(chunk as u32).to_le_bytes());
        bytes.extend_from_slice(&(text.len() as u64).to_le_bytes());
        for part in text.chunks(chunk) {
            let packed = miniz_oxide::deflate::compress_to_vec_zlib(part, 6);
            bytes.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&packed);
        }
        bytes
    }

    #[test]
    fn reads_compressed_playlists() {
        let text = r#"{"items": [{"path": "ux0:/roms/gb/A.gb", "label": "A", "core_path": "DETECT"}]}"#;
        assert_eq!(decode_rzip(&rzip(text.as_bytes(), 16)).unwrap().unwrap(), text.as_bytes());
        assert!(decode_rzip(text.as_bytes()).is_none());
        let mut zstd = rzip(text.as_bytes(), 16);
        zstd[6] = 2;
        assert_eq!(decode_rzip(&zstd).unwrap().unwrap_err(), "rzipZstd");
        let packed = rzip(text.as_bytes(), 16);
        assert_eq!(decode_rzip(&packed[..packed.len() - 3]).unwrap().unwrap_err(), "rzipShort");

        let (_temp, roots) = setup("rzip");
        add_core(&roots, "gambatte", "Nintendo - Game Boy");
        fs::write(format!("{}/playlists/Nintendo - Game Boy.lpl", roots.data), rzip(text.as_bytes(), 32)).unwrap();
        fs::write(format!("{}/playlists/Broken.lpl", roots.data), &zstd).unwrap();
        let scan = scan(&roots, &mut || {});
        assert_eq!(scan.games.len(), 1);
        assert!(scan.report.contains("RetroArch unreadable=1,RetroArch firstUnreadable=Broken.lpl rzipZstd"), "{}", scan.report);
    }

    /// The playlists in `RETROARCH_SAMPLES` (set by retroarch-test.sh to
    /// `.samples/retroarch-playlists` when that folder exists) all read.
    #[test]
    fn reads_the_sample_playlists() {
        let Ok(folder) = std::env::var("RETROARCH_SAMPLES") else {
            return;
        };
        // The samples are not in git; a checkout without them (CI) skips.
        let Ok(entries) = fs::read_dir(&folder) else {
            return;
        };
        let mut count = 0;
        for entry in entries.flatten() {
            let path = entry.path().to_string_lossy().into_owned();
            if !path.ends_with(".lpl") {
                continue;
            }
            let playlist = read_playlist(&path).unwrap_or_else(|reason| panic!("{path}: {reason}"));
            assert!(!playlist.entries.is_empty(), "{path}: no entries");
            println!("{path}: {} entries, default core {:?}", playlist.entries.len(), playlist.default_core);
            count += 1;
        }
        assert!(count > 0);
    }

    #[test]
    fn reads_names_from_a_listing_page() {
        let html = r#"<html><body><a href="../">../</a>
<a href="007%20Shitou%20-%20The%20Duel%20(Japan).png">007 Shitou</a>
<a href="Pok%C3%A9mon%20-%20Red%20(USA).png">x</a>
<a href="?C=N;O=D">Name</a><a href="notes.txt">t</a>
<a href="Battleship%20(USA).png">Battleship (USA).png</a></body></html>"#;
        assert_eq!(
            index_names(html),
            vec!["007 Shitou - The Duel (Japan)", "Pokémon - Red (USA)", "Battleship (USA)"]
        );
        assert_eq!(percent_decode("100%").as_deref(), Some("100%"));
        assert_eq!(percent_decode("a%2"), Some(String::from("a%2")));
    }

    /// The listing of a console in `RETROARCH_INDEX` (set by
    /// retroarch-test.sh to an index page downloaded beforehand) reads.
    #[test]
    fn reads_a_real_listing_page() {
        let Ok(path) = std::env::var("RETROARCH_INDEX") else {
            return;
        };
        let names = index_names(&fs::read_to_string(path).unwrap());
        assert!(names.len() > 1000, "{}", names.len());
        assert!(names.iter().any(|name| name == "Battleship (USA)"));
    }

    #[test]
    fn nothing_without_retroarch() {
        let (_temp, roots) = setup("absent");
        fs::remove_file(format!("{}/eboot.bin", roots.app)).unwrap();
        let scan = scan(&roots, &mut || {});
        assert!(scan.games.is_empty());
        assert_eq!(scan.report, "RetroArch=notInstalled");
    }

    #[test]
    fn builds_launch_uris_and_thumbnail_paths() {
        assert_eq!(
            launch_uri("snes9x_libretro.self", "ux0:/roms/A.sfc"),
            "psgm:play?titleid=RETROVITA&param=app0:/snes9x_libretro.self&param2=ux0:/roms/A.sfc"
        );
        assert_eq!(thumbnail_name("Q*bert: Arcade?"), "Q_bert_ Arcade_.png");
        let paths = thumbnail_paths("T", "Nintendo - Game Boy", "Tetris (World)", &["Named_Boxarts"]);
        assert_eq!(paths, vec!["T/Nintendo - Game Boy/Named_Boxarts/Tetris (World).png", "T/Nintendo - Game Boy/Named_Boxarts/Tetris.png"]);
        assert_eq!(thumbnail_paths("T", "MAME 2003-Plus", "Pac", &["Named_Snaps"]), vec!["T/MAME/Named_Snaps/Pac.png"]);
    }
}
