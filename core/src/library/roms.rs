//! Folder categories' games: the files under a folder, each with the
//! console it is for, as RetroArch's databases name it.
//!
//! A file's console comes from, in order:
//! - its extension, when only one console uses it (`.sfc`, `.gba`, `.md`);
//! - a zip's file names (its central directory), when one has such an
//!   extension;
//! - a disc image's header: `SEGADISCSYSTEM` (Mega-CD) and `SEGA SEGASATURN`
//!   (Saturn) in the first sector, `PLAYSTATION` as the ISO 9660 volume's
//!   system id (sector 16, in 2048- or 2352-byte sectors), `SEGA` at 0x100 of
//!   a Mega Drive cartridge dump, `PSISOIMG` in a PS1 `EBOOT.PBP`; a `.cue`
//!   is read through its first track, an `.m3u` through its first disc;
//! - the names of the folders it is in, nearest first (`snes`, `psx`,
//!   `Mega Drive`, a database name).
//!
//! Files a `.cue`, `.gdi` or `.m3u` in the same folder names are its parts and
//! are not listed of their own. The host walks the folders on a worker
//! thread ([`walk`]) through [`Files`]; the Vita reads them with `std::fs`.

/// File access for [`walk`].
pub trait Files {
    /// A folder's entries, `(name, is a folder)`; None when it does not read.
    fn list(&mut self, dir: &str) -> Option<Vec<(String, bool)>>;
    fn size(&mut self, path: &str) -> Option<u64>;
    /// Up to `len` bytes from `offset`; fewer at the end of the file.
    fn read_at(&mut self, path: &str, offset: u64, len: usize) -> Option<Vec<u8>>;
}

/// A game found in a folder.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rom {
    /// The file's path, with `/` after the device (`ux0:/roms/a.sfc`).
    pub path: String,
    /// The file's name without its extension, spaces collapsed.
    pub title: String,
    /// RetroArch's database name for its console.
    pub console: &'static str,
}

/// What a walk found.
#[derive(Default, Debug)]
pub struct Walk {
    pub roms: Vec<Rom>,
    pub folders: u32,
    /// Files with a game's extension whose console was not found.
    pub unknown: u32,
    /// The first few of them, for the diagnostics report.
    pub unknown_names: Vec<String>,
}

/// How deep below a folder the walk goes.
const DEPTH_MAX: usize = 8;
/// The most entries one walk looks at, and the most games it lists.
const ENTRIES_MAX: usize = 60_000;
pub const ROMS_MAX: usize = 20_000;
const UNKNOWN_SHOWN: usize = 6;

const NES: &str = "Nintendo - Nintendo Entertainment System";
const FDS: &str = "Nintendo - Family Computer Disk System";
const SNES: &str = "Nintendo - Super Nintendo Entertainment System";
const GB: &str = "Nintendo - Game Boy";
const GBC: &str = "Nintendo - Game Boy Color";
const GBA: &str = "Nintendo - Game Boy Advance";
const N64: &str = "Nintendo - Nintendo 64";
const NDS: &str = "Nintendo - Nintendo DS";
const VB: &str = "Nintendo - Virtual Boy";
const MD: &str = "Sega - Mega Drive - Genesis";
const SMS: &str = "Sega - Master System - Mark III";
const GG: &str = "Sega - Game Gear";
const SG: &str = "Sega - SG-1000";
const S32X: &str = "Sega - 32X";
const MCD: &str = "Sega - Mega-CD - Sega CD";
const SATURN: &str = "Sega - Saturn";
const DC: &str = "Sega - Dreamcast";
const PS1: &str = "Sony - PlayStation";
const PCE: &str = "NEC - PC Engine - TurboGrafx 16";
const PCECD: &str = "NEC - PC Engine CD - TurboGrafx-CD";
const ARCADE: &str = "FBNeo - Arcade Games";

/// Extensions only one console uses.
const EXTENSIONS: &[(&str, &str)] = &[
    ("nes", NES),
    ("unf", NES),
    ("unif", NES),
    ("fds", FDS),
    ("sfc", SNES),
    ("smc", SNES),
    ("swc", SNES),
    ("fig", SNES),
    ("bs", "Nintendo - Satellaview"),
    ("gb", GB),
    ("gbc", GBC),
    ("gba", GBA),
    ("n64", N64),
    ("z64", N64),
    ("v64", N64),
    ("nds", NDS),
    ("vb", VB),
    ("vboy", VB),
    ("min", "Nintendo - Pokemon Mini"),
    ("gen", MD),
    ("smd", MD),
    ("sms", SMS),
    ("gg", GG),
    ("sg", SG),
    ("32x", S32X),
    ("gdi", DC),
    ("cdi", DC),
    ("pce", PCE),
    ("sgx", "NEC - PC Engine SuperGrafx"),
    ("ngp", "SNK - Neo Geo Pocket"),
    ("ngc", "SNK - Neo Geo Pocket Color"),
    ("ws", "Bandai - WonderSwan"),
    ("wsc", "Bandai - WonderSwan Color"),
    ("a26", "Atari - 2600"),
    ("a52", "Atari - 5200"),
    ("a78", "Atari - 7800"),
    ("lnx", "Atari - Lynx"),
    ("st", "Atari - ST"),
    ("msa", "Atari - ST"),
    ("col", "Coleco - ColecoVision"),
    ("int", "Mattel - Intellivision"),
    ("vec", "GCE - Vectrex"),
    ("d64", "Commodore - 64"),
    ("t64", "Commodore - 64"),
    ("crt", "Commodore - 64"),
    ("adf", "Commodore - Amiga"),
    ("mx1", "Microsoft - MSX"),
    ("mx2", "Microsoft - MSX2"),
    ("tzx", "Sinclair - ZX Spectrum"),
    ("dim", "Sharp - X68000"),
];

/// Extensions several consoles use (and `.md`, Markdown's too): the
/// console comes from the file's contents or its folders.
const SHARED: &[&str] = &["md", "zip", "7z", "bin", "rom", "iso", "img", "cue", "chd", "m3u", "ccd", "pbp", "dsk", "mdf"];

/// Folder names (lowercase, letters and digits only) and their consoles.
const FOLDERS: &[(&str, &str)] = &[
    ("nes", NES),
    ("famicom", NES),
    ("fc", NES),
    ("nintendoentertainmentsystem", NES),
    ("fds", FDS),
    ("famicomdisksystem", FDS),
    ("snes", SNES),
    ("sfc", SNES),
    ("superfamicom", SNES),
    ("supernintendo", SNES),
    ("supernintendoentertainmentsystem", SNES),
    ("gb", GB),
    ("gameboy", GB),
    ("gbc", GBC),
    ("gameboycolor", GBC),
    ("gba", GBA),
    ("gameboyadvance", GBA),
    ("n64", N64),
    ("nintendo64", N64),
    ("nds", NDS),
    ("ds", NDS),
    ("nintendods", NDS),
    ("vb", VB),
    ("virtualboy", VB),
    ("md", MD),
    ("megadrive", MD),
    ("genesis", MD),
    ("megadrivegenesis", MD),
    ("sms", SMS),
    ("mastersystem", SMS),
    ("gg", GG),
    ("gamegear", GG),
    ("sg1000", SG),
    ("32x", S32X),
    ("sega32x", S32X),
    ("segacd", MCD),
    ("megacd", MCD),
    ("mcd", MCD),
    ("megacdsegacd", MCD),
    ("saturn", SATURN),
    ("segasaturn", SATURN),
    ("dc", DC),
    ("dreamcast", DC),
    ("psx", PS1),
    ("ps1", PS1),
    ("psone", PS1),
    ("playstation", PS1),
    ("pce", PCE),
    ("pcengine", PCE),
    ("tg16", PCE),
    ("turbografx", PCE),
    ("turbografx16", PCE),
    ("pcenginecd", PCECD),
    ("pcecd", PCECD),
    ("tgcd", PCECD),
    ("turbografxcd", PCECD),
    ("ngp", "SNK - Neo Geo Pocket"),
    ("ngpc", "SNK - Neo Geo Pocket Color"),
    ("neogeo", "SNK - Neo Geo"),
    ("neogeocd", "SNK - Neo Geo CD"),
    ("arcade", ARCADE),
    ("fbneo", ARCADE),
    ("fba", ARCADE),
    ("fbalpha", ARCADE),
    ("mame", "MAME"),
    ("mame2000", "MAME 2000"),
    ("mame2003", "MAME 2003"),
    ("mame2003plus", "MAME 2003-Plus"),
    ("a2600", "Atari - 2600"),
    ("atari2600", "Atari - 2600"),
    ("atari5200", "Atari - 5200"),
    ("atari7800", "Atari - 7800"),
    ("lynx", "Atari - Lynx"),
    ("atarilynx", "Atari - Lynx"),
    ("atarist", "Atari - ST"),
    ("ws", "Bandai - WonderSwan"),
    ("wonderswan", "Bandai - WonderSwan"),
    ("wsc", "Bandai - WonderSwan Color"),
    ("wonderswancolor", "Bandai - WonderSwan Color"),
    ("coleco", "Coleco - ColecoVision"),
    ("colecovision", "Coleco - ColecoVision"),
    ("intellivision", "Mattel - Intellivision"),
    ("vectrex", "GCE - Vectrex"),
    ("msx", "Microsoft - MSX"),
    ("msx2", "Microsoft - MSX2"),
    ("c64", "Commodore - 64"),
    ("commodore64", "Commodore - 64"),
    ("amiga", "Commodore - Amiga"),
    ("zxspectrum", "Sinclair - ZX Spectrum"),
    ("spectrum", "Sinclair - ZX Spectrum"),
    ("cpc", "Amstrad - CPC"),
    ("amstradcpc", "Amstrad - CPC"),
    ("dos", "DOS"),
    ("x68000", "Sharp - X68000"),
    ("pc98", "NEC - PC-98"),
    ("pokemini", "Nintendo - Pokemon Mini"),
];

/// A file name's extension, lowercase; empty when it has none.
fn extension(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// The console only `ext` names.
pub fn console_of_extension(ext: &str) -> Option<&'static str> {
    EXTENSIONS.iter().find(|(e, _)| *e == ext).map(|(_, console)| *console)
}

/// Whether a file with this name may be a game.
pub fn is_candidate(name: &str) -> bool {
    let ext = extension(name);
    console_of_extension(&ext).is_some() || SHARED.contains(&ext.as_str())
}

/// The console a folder's name stands for: one of [`FOLDERS`], or a
/// database name as RetroArch spells it.
pub fn console_of_folder(name: &str) -> Option<&'static str> {
    let key: String = name.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect();
    if key.is_empty() {
        return None;
    }
    FOLDERS
        .iter()
        .find(|(alias, _)| *alias == key)
        .map(|(_, console)| *console)
        .or_else(|| FOLDERS.iter().map(|(_, c)| *c).find(|c| c.eq_ignore_ascii_case(name.trim())))
}

/// `path` with a `/` after its device's `:` and no repeated or trailing
/// `/`: `ux0:data//roms/` is `ux0:/data/roms`. Emulators get paths in this
/// form, as RetroArch's playlists write them.
pub fn canonical(path: &str) -> String {
    let path = path.trim();
    let (device, rest) = match path.find(':') {
        Some(at) => (&path[..=at], &path[at + 1..]),
        None => ("", path),
    };
    let mut out = String::with_capacity(path.len() + 1);
    out.push_str(device);
    for part in rest.split('/').filter(|p| !p.is_empty()) {
        out.push('/');
        out.push_str(part);
    }
    if out.is_empty() || out.ends_with(':') {
        out.push('/');
    }
    out
}

/// A path as a key for comparing spellings of it: [`canonical`], ASCII
/// lowercase (the memory card ignores case).
pub fn path_key(path: &str) -> String {
    let mut key = canonical(path);
    key.make_ascii_lowercase();
    key
}

/// Whether the file `path_key` is somewhere under the folder `folder_key`
/// (both [`path_key`]s).
pub fn is_under(path_key: &str, folder_key: &str) -> bool {
    let folder = folder_key.trim_end_matches('/');
    path_key.len() > folder.len() + 1 && path_key.starts_with(folder) && path_key.as_bytes()[folder.len()] == b'/'
}

fn join(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

fn title_of(name: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    stem.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn at(bytes: &[u8], offset: usize, text: &[u8]) -> bool {
    bytes.get(offset..offset + text.len()) == Some(text)
}

/// The ISO 9660 primary volume descriptor's system id, when there is one,
/// in an image of 2048-byte or raw 2352-byte sectors.
fn system_id(files: &mut dyn Files, path: &str, first: &[u8]) -> Option<String> {
    const SYNC: [u8; 12] = [0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0];
    let offset = if first.starts_with(&SYNC) {
        // Mode 1 data follows a 16-byte header, mode 2 a 24-byte one.
        let mode = first.get(15).copied().unwrap_or(1);
        16 * 2352 + if mode == 2 { 24 } else { 16 }
    } else {
        16 * 2048
    };
    let pvd = files.read_at(path, offset, 40)?;
    if !at(&pvd, 0, b"\x01CD001") {
        return None;
    }
    let id = String::from_utf8_lossy(&pvd[8..40]).trim_matches([' ', '\0']).to_string();
    Some(id)
}

/// The console a disc image or cartridge dump is for, from its first bytes.
fn sniff_image(files: &mut dyn Files, path: &str, ext: &str) -> Option<&'static str> {
    let first = files.read_at(path, 0, 0x200)?;
    for offset in [0, 0x10] {
        if at(&first, offset, b"SEGADISCSYSTEM") {
            return Some(MCD);
        }
        if at(&first, offset, b"SEGA SEGASATURN") {
            return Some(SATURN);
        }
    }
    if ext == "bin" || ext == "md" {
        if at(&first, 0x100, b"SEGA 32X") {
            return Some(S32X);
        }
        if at(&first, 0x100, b"SEGA") || at(&first, 0x101, b"SEGA") {
            return Some(MD);
        }
    }
    let id = system_id(files, path, &first)?;
    (id == "PLAYSTATION").then_some(PS1)
}

/// A PS1 `EBOOT.PBP` (a PSP one holds no disc image).
fn sniff_pbp(files: &mut dyn Files, path: &str) -> Option<&'static str> {
    let header = files.read_at(path, 0, 0x28)?;
    if !at(&header, 0, b"\0PBP") {
        return None;
    }
    let psar = u32::from_le_bytes(header.get(0x24..0x28)?.try_into().ok()?) as u64;
    let magic = files.read_at(path, psar, 8)?;
    (magic == b"PSISOIMG" || magic == b"PSTITLEI").then_some(PS1)
}

/// The file names in a zip's central directory.
pub fn zip_names(files: &mut dyn Files, path: &str) -> Option<Vec<String>> {
    let size = files.size(path)?;
    // The end record is 22 bytes after up to 64 KiB of comment.
    let tail_len = size.min(22 + 65_535) as usize;
    let tail = files.read_at(path, size - tail_len as u64, tail_len)?;
    let end = (0..tail.len().saturating_sub(21)).rev().find(|&i| tail[i..].starts_with(b"PK\x05\x06"))?;
    let record = &tail[end..];
    let entries = u16::from_le_bytes([record[10], record[11]]) as usize;
    let dir_size = u32::from_le_bytes(record[12..16].try_into().ok()?) as usize;
    let dir_at = u32::from_le_bytes(record[16..20].try_into().ok()?) as u64;
    // A zip64 archive or a directory too large to be a game's.
    if dir_size > 1 << 20 || dir_at == u32::MAX as u64 {
        return None;
    }
    let dir = files.read_at(path, dir_at, dir_size)?;
    let mut names = Vec::new();
    let mut i = 0;
    while names.len() < entries && i + 46 <= dir.len() && dir[i..].starts_with(b"PK\x01\x02") {
        let name_len = u16::from_le_bytes([dir[i + 28], dir[i + 29]]) as usize;
        let extra = u16::from_le_bytes([dir[i + 30], dir[i + 31]]) as usize;
        let comment = u16::from_le_bytes([dir[i + 32], dir[i + 33]]) as usize;
        let name = dir.get(i + 46..i + 46 + name_len)?;
        names.push(String::from_utf8_lossy(name).into_owned());
        i += 46 + name_len + extra + comment;
    }
    Some(names)
}

/// The files a `.cue`, `.gdi` or `.m3u` names, in order.
fn listed_files(text: &str, ext: &str) -> Vec<String> {
    let text = text.trim_start_matches('\u{feff}');
    match ext {
        "cue" => text
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                line.get(..5).filter(|w| w.eq_ignore_ascii_case("FILE "))?;
                let rest = line[5..].trim();
                let name = match rest.strip_prefix('"') {
                    Some(quoted) => quoted.split('"').next()?,
                    None => rest.split_whitespace().next()?,
                };
                Some(name.to_string())
            })
            .collect(),
        "gdi" => text
            .lines()
            .skip(1)
            .filter_map(|line| {
                // `<track> <lba> <type> <sector size> <file> <offset>`; the
                // file may be quoted.
                let line = line.trim();
                let rest = line.splitn(5, char::is_whitespace).nth(4)?.trim();
                match rest.strip_prefix('"') {
                    Some(quoted) => quoted.split('"').next().map(String::from),
                    None => rest.split_whitespace().next().map(String::from),
                }
            })
            .collect(),
        "m3u" => text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from).collect(),
        _ => Vec::new(),
    }
}

/// A list file's text (`.cue`, `.gdi`, `.m3u`), when it is small enough to
/// be one.
fn read_list(files: &mut dyn Files, path: &str) -> Option<String> {
    let size = files.size(path)?;
    if size > 64 * 1024 {
        return None;
    }
    let bytes = files.read_at(path, 0, size as usize)?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// The console of one file, from its extension and contents; `hint` is
/// its folders' (nearest first). `depth` stops a list naming lists.
fn detect(files: &mut dyn Files, dir: &str, name: &str, hint: Option<&'static str>, depth: u8) -> Option<&'static str> {
    let ext = extension(name);
    let path = join(dir, name);
    let found = match ext.as_str() {
        "zip" => zip_names(files, &path).and_then(|names| {
            names.iter().find_map(|inner| console_of_extension(&extension(inner.rsplit('/').next().unwrap_or(inner))))
        }),
        "pbp" => sniff_pbp(files, &path),
        "iso" | "bin" | "img" | "md" | "mdf" => sniff_image(files, &path, &ext),
        "cue" | "m3u" if depth < 2 => {
            let first = read_list(files, &path).and_then(|text| listed_files(&text, &ext).into_iter().next());
            first.and_then(|track| {
                // The track's own folder, when the list names one.
                let (sub, file) = match track.rsplit_once(['/', '\\']) {
                    Some((sub, file)) => (join(dir, &sub.replace('\\', "/")), file.to_string()),
                    None => (dir.to_string(), track),
                };
                detect(files, &sub, &file, hint, depth + 1)
            })
        }
        _ => None,
    };
    // `.md` is Markdown as often: a Mega Drive game only with its header
    // or in a Mega Drive folder.
    if ext == "md" {
        return found.or(hint.filter(|h| *h == MD));
    }
    found.or_else(|| console_of_extension(&ext)).or(hint)
}

/// Every game under `root` and its subfolders. `step` is called once per
/// folder read; a file `skip` takes (by its path) is not opened or listed
/// (a playlist lists it).
pub fn walk(files: &mut dyn Files, root: &str, skip: &mut dyn FnMut(&str) -> bool, step: &mut dyn FnMut()) -> Walk {
    let mut walk = Walk::default();
    let mut seen = 0usize;
    // Folders to read: path, depth, its folders' console.
    let root = canonical(root);
    let hint = root.trim_end_matches('/').rsplit(['/', ':']).find(|p| !p.is_empty()).and_then(console_of_folder);
    let mut stack: Vec<(String, usize, Option<&'static str>)> = vec![(root, 0, hint)];
    while let Some((dir, depth, hint)) = stack.pop() {
        step();
        walk.folders += 1;
        let Some(mut entries) = files.list(&dir) else { continue };
        entries.retain(|(name, _)| !name.starts_with('.'));
        entries.sort_by_cached_key(|(name, _)| name.to_lowercase());
        seen += entries.len();
        if seen > ENTRIES_MAX {
            break;
        }
        // The parts the folder's lists name.
        let mut parts: Vec<String> = Vec::new();
        for (name, folder) in &entries {
            let ext = extension(name);
            if !folder && matches!(ext.as_str(), "cue" | "gdi" | "m3u") {
                if let Some(text) = read_list(files, &join(&dir, name)) {
                    parts.extend(listed_files(&text, &ext).into_iter().map(|p| p.rsplit(['/', '\\']).next().unwrap_or(&p).to_lowercase()));
                }
            }
        }
        let mut subfolders = Vec::new();
        for (name, folder) in entries {
            if folder {
                if depth < DEPTH_MAX {
                    let hint = console_of_folder(&name).or(hint);
                    subfolders.push((join(&dir, &name), depth + 1, hint));
                }
                continue;
            }
            if !is_candidate(&name) || parts.contains(&name.to_lowercase()) || walk.roms.len() >= ROMS_MAX || skip(&join(&dir, &name)) {
                continue;
            }
            match detect(files, &dir, &name, hint, 0) {
                Some(console) => walk.roms.push(Rom { path: join(&dir, &name), title: title_of(&name), console }),
                // A `.md` that is not a Mega Drive game is a text.
                None if extension(&name) == "md" => {}
                None => {
                    walk.unknown += 1;
                    if walk.unknown_names.len() < UNKNOWN_SHOWN {
                        walk.unknown_names.push(name);
                    }
                }
            }
        }
        // In name order: the stack takes the last one first.
        stack.extend(subfolders.into_iter().rev());
    }
    walk
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Files in memory, by path.
    #[derive(Default)]
    pub(crate) struct Memory(pub BTreeMap<String, Vec<u8>>);

    impl Memory {
        pub(crate) fn add(&mut self, path: &str, bytes: &[u8]) {
            self.0.insert(path.to_string(), bytes.to_vec());
        }
    }

    impl Files for Memory {
        fn list(&mut self, dir: &str) -> Option<Vec<(String, bool)>> {
            let prefix = format!("{}/", dir.trim_end_matches('/'));
            let mut out: Vec<(String, bool)> = Vec::new();
            for path in self.0.keys() {
                let Some(rest) = path.strip_prefix(&prefix) else { continue };
                let (name, folder) = match rest.split_once('/') {
                    Some((name, _)) => (name, true),
                    None => (rest, false),
                };
                if !out.iter().any(|(n, _)| n == name) {
                    out.push((name.to_string(), folder));
                }
            }
            (!out.is_empty()).then_some(out)
        }
        fn size(&mut self, path: &str) -> Option<u64> {
            self.0.get(path).map(|b| b.len() as u64)
        }
        fn read_at(&mut self, path: &str, offset: u64, len: usize) -> Option<Vec<u8>> {
            let bytes = self.0.get(path)?;
            let start = (offset as usize).min(bytes.len());
            Some(bytes[start..(start + len).min(bytes.len())].to_vec())
        }
    }

    /// A zip holding empty files with these names (stored, no data).
    pub(crate) fn zip(names: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut dir = Vec::new();
        for name in names {
            let at = out.len() as u32;
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&[0; 22]);
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(name.as_bytes());
            dir.extend_from_slice(b"PK\x01\x02");
            dir.extend_from_slice(&[0; 24]);
            dir.extend_from_slice(&(name.len() as u16).to_le_bytes());
            dir.extend_from_slice(&[0; 12]);
            dir.extend_from_slice(&at.to_le_bytes());
            dir.extend_from_slice(name.as_bytes());
        }
        let dir_at = out.len() as u32;
        out.extend_from_slice(&dir);
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(names.len() as u16).to_le_bytes());
        out.extend_from_slice(&(names.len() as u16).to_le_bytes());
        out.extend_from_slice(&(dir.len() as u32).to_le_bytes());
        out.extend_from_slice(&dir_at.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }

    /// A PS1 disc in 2352-byte mode 2 sectors.
    fn ps1_bin() -> Vec<u8> {
        let mut bytes = vec![0u8; 17 * 2352];
        bytes[..12].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
        bytes[15] = 2;
        let pvd = 16 * 2352 + 24;
        bytes[pvd..pvd + 6].copy_from_slice(b"\x01CD001");
        bytes[pvd + 8..pvd + 19].copy_from_slice(b"PLAYSTATION");
        bytes
    }

    fn consoles(walk: &Walk) -> Vec<(&str, &str)> {
        walk.roms.iter().map(|r| (r.path.as_str(), r.console)).collect()
    }

    #[test]
    fn consoles_come_from_extensions_zips_headers_and_folders() {
        let mut files = Memory::default();
        files.add("ux0:/roms/Mario (USA).sfc", b"");
        files.add("ux0:/roms/notes.txt", b"");
        files.add("ux0:/roms/README.md", b"# Notes");
        files.add("ux0:/roms/Listed.sfc", b"");
        files.add("ux0:/roms/.hidden.gba", b"");
        files.add("ux0:/roms/Tetris.zip", &zip(&["readme.txt", "Tetris (World).gb"]));
        files.add("ux0:/roms/sonic.bin", &{
            let mut b = vec![0u8; 0x200];
            b[0x100..0x104].copy_from_slice(b"SEGA");
            b
        });
        files.add("ux0:/roms/Crash/Crash.cue", b"FILE \"Crash (Track 1).bin\" BINARY\n  TRACK 01 MODE2/2352\n");
        files.add("ux0:/roms/Crash/Crash (Track 1).bin", &ps1_bin());
        files.add("ux0:/roms/arcade/sf2.zip", &zip(&["sf2.030", "sf2.031"]));
        files.add("ux0:/roms/Mystery.zip", &zip(&["a.dat"]));
        files.add("ux0:/roms/PSX/Spyro/EBOOT.PBP", &{
            let mut b = vec![0u8; 0x40];
            b[..4].copy_from_slice(b"\0PBP");
            b[0x24..0x28].copy_from_slice(&0x30u32.to_le_bytes());
            b[0x30..0x38].copy_from_slice(b"PSISOIMG");
            b
        });
        let mut folders = 0;
        let mut skipped = Vec::new();
        let walk = walk(&mut files, "ux0:roms/", &mut |path| {
            skipped.push(path.to_string());
            path == "ux0:/roms/Listed.sfc"
        }, &mut || folders += 1);
        assert_eq!(
            consoles(&walk),
            [
                ("ux0:/roms/Mario (USA).sfc", SNES),
                ("ux0:/roms/sonic.bin", MD),
                ("ux0:/roms/Tetris.zip", GB),
                ("ux0:/roms/arcade/sf2.zip", ARCADE),
                ("ux0:/roms/Crash/Crash.cue", PS1),
                ("ux0:/roms/PSX/Spyro/EBOOT.PBP", PS1),
            ]
        );
        assert_eq!(walk.roms[0].title, "Mario (USA)");
        assert_eq!((walk.unknown, walk.unknown_names.as_slice()), (1, &["Mystery.zip".to_string()][..]));
        assert_eq!(folders, 5, "roms, Crash, PSX, Spyro, arcade");
        assert!(skipped.contains(&"ux0:/roms/Listed.sfc".to_string()) && !skipped.iter().any(|p| p.ends_with(".txt")));
    }

    #[test]
    fn a_folder_name_decides_shared_extensions_and_lists_hide_their_parts() {
        let mut files = Memory::default();
        files.add("ux0:/data/Mega Drive/Streets.bin", &[0; 0x200]);
        files.add("ux0:/data/Mega Drive/Streets.zip", &zip(&["Streets.bin"]));
        files.add("ux0:/data/Dreamcast/Sonic/disc.gdi", b"3\n1 0 4 2352 track01.bin 0\n2 600 0 2352 \"track 02.raw\" 0\n");
        files.add("ux0:/data/Dreamcast/Sonic/track01.bin", &[0; 16]);
        files.add("ux0:/data/Dreamcast/Sonic/track 02.raw", &[0; 16]);
        files.add("ux0:/data/FF7/FF7.m3u", b"#EXTM3U\nFF7 (Disc 1).cue\nFF7 (Disc 2).cue\n");
        files.add("ux0:/data/FF7/FF7 (Disc 1).cue", b"FILE \"FF7 (Disc 1).bin\" BINARY\n");
        files.add("ux0:/data/FF7/FF7 (Disc 1).bin", &ps1_bin());
        files.add("ux0:/data/FF7/FF7 (Disc 2).cue", b"FILE \"FF7 (Disc 2).bin\" BINARY\n");
        files.add("ux0:/data/FF7/FF7 (Disc 2).bin", &ps1_bin());
        let walk = walk(&mut files, "ux0:/data", &mut |_| false, &mut || {});
        assert_eq!(
            consoles(&walk),
            [
                ("ux0:/data/Dreamcast/Sonic/disc.gdi", DC),
                ("ux0:/data/FF7/FF7.m3u", PS1),
                ("ux0:/data/Mega Drive/Streets.bin", MD),
                ("ux0:/data/Mega Drive/Streets.zip", MD),
            ]
        );
    }

    #[test]
    fn paths_compare_by_key_and_folders_by_name() {
        assert_eq!(canonical("ux0:data//roms/"), "ux0:/data/roms");
        assert_eq!(canonical("ux0:"), "ux0:/");
        assert_eq!(path_key("UX0:Data/ROMS/a.SFC"), "ux0:/data/roms/a.sfc");
        assert!(is_under("ux0:/data/roms/a.sfc", "ux0:/data/roms"));
        assert!(is_under("ux0:/data/roms/snes/a.sfc", "ux0:/data/roms/"));
        assert!(!is_under("ux0:/data/romsx/a.sfc", "ux0:/data/roms"));
        assert!(!is_under("ux0:/data/roms", "ux0:/data/roms"));
        assert!(is_under("ux0:/a.sfc", "ux0:/"));
        assert_eq!(console_of_folder("Super Nintendo"), Some(SNES));
        assert_eq!(console_of_folder("Sony - PlayStation"), Some(PS1));
        assert_eq!(console_of_folder("PS1"), Some(PS1));
        assert_eq!(console_of_folder("homework"), None);
    }
}
