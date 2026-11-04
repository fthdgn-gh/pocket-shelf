//! RetroArch's game databases, from libretro-database on GitHub
//! (`rdb/<database>.rdb`, one per console, named as RetroArch names the
//! console: `Nintendo - Game Boy`). Each record ties a ROM's CRC-32 (or a
//! disc's serial, or an arcade set's zip name) to its No-Intro, Redump or
//! MAME name, which is the name libretro's thumbnails are saved under.
//!
//! **The file is libretrodb**: `RARCHDB\0`, a big-endian u64 offset of the
//! metadata, then one MessagePack map per record from byte 16 up to a nil.
//! Records come from several dat files merged by CRC, so one game can be
//! several records (NES: 29,646 records, 4,328 sharing a CRC); they are
//! merged here. The download is turned into a table on the download thread
//! ([`table_from_rdb`], `gamedb/<database>.tsv`), and the `.rdb` itself is
//! never written to the card: NES's 6.3 MB becomes about 1.5 MB of text.
//!
//! **A game is identified** ([`identify`]) by its playlist label when that
//! is a database name already (RetroArch's own scanner wrote it), an arcade
//! set by its zip's name, then by CRC-32: a zip's directory holds each
//! file's CRC, a loose file is read once. The CRCs are kept in
//! `gamedb/crcs.tsv`, so a file is read on the first run only.

use std::collections::HashSet;

use super::encode_uri_component;
use crate::library::fxhash::FxHashMap;

/// Where libretro-database keeps the built databases.
pub const SERVER: &str = "https://raw.githubusercontent.com/libretro/libretro-database/master/rdb";

/// A table older than this is downloaded again at the next Fetch artwork
/// (libretro rebuilds the databases every few weeks).
pub const MAX_AGE_MS: u64 = 90 * 24 * 60 * 60 * 1000;

/// The file of the CRCs worked out so far, by content path.
pub const CRCS_PATH: &str = "gamedb/crcs.tsv";

/// The first line of a table: this text, the table's version, and when it
/// was built in milliseconds since 1970.
const TABLE_MARK: &str = "#pocket-shelf-gamedb";
const TABLE_VERSION: u32 = 1;

/// The database for a RetroArch console: its database name, or for the
/// user's "Neo Geo" playlist (no `db_name`), `SNK - Neo Geo`. Every MAME
/// version has a database of its own (`MAME 2003-Plus`).
pub fn database_name(console: &str) -> String {
    match console.trim() {
        "Neo Geo" => "SNK - Neo Geo".into(),
        name => name.into(),
    }
}

pub fn rdb_url(database: &str) -> String {
    format!("{SERVER}/{}.rdb", encode_uri_component(database))
}

/// Where a database's table is kept under the data folder.
pub fn table_path(database: &str) -> String {
    format!("gamedb/{}.tsv", super::libretro::slug(database))
}

// --- libretrodb ---------------------------------------------------------------

/// A MessagePack value as a record holds it; maps and arrays are skipped.
enum Value<'a> {
    Int(i64),
    Text(&'a [u8]),
    Other,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let part = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(part)
    }

    fn uint(&mut self, n: usize) -> Option<u64> {
        Some(self.take(n)?.iter().fold(0u64, |value, &byte| (value << 8) | byte as u64))
    }

    /// A map's length when the next value is a map; None at a nil (the end
    /// of the records) or anything else.
    fn map_len(&mut self) -> Option<usize> {
        let tag = *self.bytes.get(self.at)?;
        self.at += 1;
        match tag {
            0x80..=0x8f => Some((tag & 0x0f) as usize),
            0xde => self.uint(2).map(|n| n as usize),
            0xdf => self.uint(4).map(|n| n as usize),
            _ => None,
        }
    }

    /// The next value; `depth` bounds nested maps and arrays.
    fn value(&mut self, depth: u32) -> Option<Value<'a>> {
        let tag = *self.bytes.get(self.at)?;
        self.at += 1;
        let sized = |reader: &mut Reader<'a>, n: usize| -> Option<Value<'a>> {
            let len = reader.uint(n)? as usize;
            reader.take(len).map(Value::Text)
        };
        let skip = |reader: &mut Reader<'a>, items: u64| -> Option<Value<'a>> {
            if depth >= 4 {
                return None;
            }
            for _ in 0..items {
                reader.value(depth + 1)?;
            }
            Some(Value::Other)
        };
        match tag {
            0x00..=0x7f => Some(Value::Int(tag as i64)),
            0xe0..=0xff => Some(Value::Int(tag as i8 as i64)),
            0xa0..=0xbf => self.take((tag & 0x1f) as usize).map(Value::Text),
            0xc0 | 0xc2 | 0xc3 => Some(Value::Other),
            // Strings and binary data are both text here (a serial is binary).
            0xc4 | 0xd9 => sized(self, 1),
            0xc5 | 0xda => sized(self, 2),
            0xc6 | 0xdb => sized(self, 4),
            0xcc => self.uint(1).map(|n| Value::Int(n as i64)),
            0xcd => self.uint(2).map(|n| Value::Int(n as i64)),
            0xce => self.uint(4).map(|n| Value::Int(n as i64)),
            0xcf => self.uint(8).map(|n| Value::Int(n as i64)),
            0xd0 => self.uint(1).map(|n| Value::Int(n as u8 as i8 as i64)),
            0xd1 => self.uint(2).map(|n| Value::Int(n as u16 as i16 as i64)),
            0xd2 => self.uint(4).map(|n| Value::Int(n as u32 as i32 as i64)),
            0xd3 => self.uint(8).map(|n| Value::Int(n as i64)),
            0xca => self.take(4).map(|_| Value::Other),
            0xcb => self.take(8).map(|_| Value::Other),
            0x90..=0x9f => skip(self, (tag & 0x0f) as u64),
            0xdc => self.uint(2).and_then(|n| skip(self, n)),
            0xdd => self.uint(4).and_then(|n| skip(self, n)),
            0x80..=0x8f => skip(self, 2 * (tag & 0x0f) as u64),
            0xde => self.uint(2).and_then(|n| skip(self, 2 * n)),
            0xdf => self.uint(4).and_then(|n| skip(self, 2 * n)),
            _ => None,
        }
    }
}

/// One game of a table.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Row {
    pub crc: Option<u32>,
    pub serial: String,
    /// An arcade set's zip name without `.zip`, lowercase; empty for the
    /// rest (their file names are not the database's).
    pub rom: String,
    pub name: String,
    pub year: u32,
    pub genre: String,
    pub developer: String,
    pub publisher: String,
    pub users: u32,
}

impl Row {
    /// Fill what this row lacks from another record of the same game.
    fn merge(&mut self, other: Row) {
        let fill = |mine: &mut String, theirs: String| {
            if mine.is_empty() {
                *mine = theirs;
            }
        };
        fill(&mut self.serial, other.serial);
        fill(&mut self.rom, other.rom);
        fill(&mut self.name, other.name);
        fill(&mut self.genre, other.genre);
        fill(&mut self.developer, other.developer);
        fill(&mut self.publisher, other.publisher);
        if self.year == 0 {
            self.year = other.year;
        }
        if self.users == 0 {
            self.users = other.users;
        }
    }
}

/// Text from a record, on one line and with no tabs.
fn clean(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().trim().to_string()
}

/// The records of a libretrodb file, merged by CRC (else by serial); those
/// with neither, or with no name once merged, are left out.
pub fn read_rdb(bytes: &[u8]) -> Result<Vec<Row>, &'static str> {
    if bytes.len() < 16 || &bytes[..8] != b"RARCHDB\0" {
        return Err("not a game database");
    }
    let end = u64::from_be_bytes(bytes[8..16].try_into().map_err(|_| "header")?);
    let end = usize::try_from(end).unwrap_or(usize::MAX).min(bytes.len());
    let mut reader = Reader { bytes: &bytes[..end], at: 16 };
    let mut rows: Vec<Row> = Vec::new();
    let mut by_key: FxHashMap<String, usize> = FxHashMap::default();
    while reader.at < end {
        let Some(len) = reader.map_len() else { break };
        let mut row = Row::default();
        for _ in 0..len {
            let key = match reader.value(0) {
                Some(Value::Text(key)) => key,
                Some(_) => &[][..],
                None => return Err("a record is cut short"),
            };
            let value = reader.value(0).ok_or("a record is cut short")?;
            match (key, value) {
                (b"name", Value::Text(text)) => row.name = clean(text),
                (b"serial", Value::Text(text)) => row.serial = clean(text),
                (b"crc", Value::Text(crc)) if crc.len() == 4 => row.crc = Some(u32::from_be_bytes(crc.try_into().unwrap_or_default())),
                (b"rom_name", Value::Text(text)) => {
                    let lower = clean(text).to_lowercase();
                    if let Some(stem) = lower.strip_suffix(".zip").or_else(|| lower.strip_suffix(".7z")) {
                        row.rom = stem.to_string();
                    }
                }
                (b"releaseyear", Value::Int(year)) => row.year = year.clamp(0, 9999) as u32,
                (b"genre", Value::Text(text)) => row.genre = clean(text),
                (b"developer", Value::Text(text)) => row.developer = clean(text),
                (b"publisher", Value::Text(text)) => row.publisher = clean(text),
                (b"users", Value::Int(users)) => row.users = users.clamp(0, 255) as u32,
                _ => {}
            }
        }
        let key = match (row.crc, row.serial.is_empty()) {
            (Some(crc), _) => format!("{crc:08x}"),
            (None, false) => format!("s:{}", row.serial),
            (None, true) => continue,
        };
        match by_key.get(&key) {
            Some(&at) => rows[at].merge(row),
            None => {
                by_key.insert(key, rows.len());
                rows.push(row);
            }
        }
    }
    rows.retain(|row| !row.name.is_empty());
    Ok(rows)
}

/// A downloaded `.rdb` as the table kept on the card: a first line with the
/// version and `built_ms`, then one game per line, `crc`, `serial`, `rom`,
/// `name`, `year`, `genre`, `developer`, `publisher`, `users` separated by
/// tabs (empty when unknown; the crc as eight hex digits).
pub fn table_from_rdb(bytes: &[u8], built_ms: u64) -> Result<String, &'static str> {
    let rows = read_rdb(bytes)?;
    if rows.is_empty() {
        return Err("no games in the database");
    }
    let mut out = format!("{TABLE_MARK} {TABLE_VERSION} {built_ms}\n");
    for row in rows {
        let crc = row.crc.map(|crc| format!("{crc:08x}")).unwrap_or_default();
        let number = |n: u32| if n == 0 { String::new() } else { n.to_string() };
        let fields = [crc, row.serial, row.rom, row.name, number(row.year), row.genre, row.developer, row.publisher, number(row.users)];
        out.push_str(&fields.join("\t"));
        out.push('\n');
    }
    Ok(out)
}

/// When a table was built, from its first line; None for anything else.
pub fn table_built(first_line: &str) -> Option<u64> {
    let mut words = first_line.split_whitespace();
    (words.next()? == TABLE_MARK && words.next()?.parse::<u32>().ok()? == TABLE_VERSION).then_some(())?;
    words.next()?.parse().ok()
}

/// A table read for matching: names by CRC, by arcade set and by name.
pub struct Table {
    pub built_ms: u64,
    names: Vec<String>,
    by_crc: FxHashMap<u32, usize>,
    by_rom: FxHashMap<String, usize>,
    by_name: HashSet<String>,
}

impl Table {
    pub fn parse(text: &str) -> Option<Table> {
        let mut lines = text.split('\n');
        let built_ms = table_built(lines.next()?)?;
        let mut table = Table { built_ms, names: Vec::new(), by_crc: FxHashMap::default(), by_rom: FxHashMap::default(), by_name: HashSet::new() };
        for line in lines {
            let mut fields = line.split('\t');
            let (Some(crc), Some(_serial), Some(rom), Some(name)) = (fields.next(), fields.next(), fields.next(), fields.next()) else { continue };
            if name.is_empty() {
                continue;
            }
            let at = table.names.len();
            if let Ok(crc) = u32::from_str_radix(crc, 16) {
                table.by_crc.entry(crc).or_insert(at);
            }
            if !rom.is_empty() {
                table.by_rom.entry(rom.to_string()).or_insert(at);
            }
            table.by_name.insert(name.to_string());
            table.names.push(name.to_string());
        }
        (!table.names.is_empty()).then_some(table)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn by_crc(&self, crc: u32) -> Option<&str> {
        self.by_crc.get(&crc).map(|&at| self.names[at].as_str())
    }

    fn by_rom(&self, stem: &str) -> Option<&str> {
        self.by_rom.get(stem).map(|&at| self.names[at].as_str())
    }

    fn has_name(&self, name: &str) -> bool {
        self.by_name.contains(name)
    }
}

// --- CRC-32 -----------------------------------------------------------------------

/// The CRC-32 tables for eight bytes a step (zlib's polynomial).
const CRC_TABLES: [[u32; 256]; 8] = {
    let mut tables = [[0u32; 256]; 8];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        tables[0][n] = c;
        n += 1;
    }
    let mut n = 0;
    while n < 256 {
        let mut t = 1;
        while t < 8 {
            let previous = tables[t - 1][n];
            tables[t][n] = (previous >> 8) ^ tables[0][(previous & 0xff) as usize];
            t += 1;
        }
        n += 1;
    }
    tables
};

/// A running CRC-32, as zip and libretro's databases write it.
#[derive(Clone, Copy)]
pub struct Crc32(u32);

impl Default for Crc32 {
    fn default() -> Crc32 {
        Crc32(!0)
    }
}

impl Crc32 {
    pub fn update(&mut self, bytes: &[u8]) {
        let t = &CRC_TABLES;
        let mut c = self.0;
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let a = c ^ u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            c = t[7][(a & 0xff) as usize]
                ^ t[6][((a >> 8) & 0xff) as usize]
                ^ t[5][((a >> 16) & 0xff) as usize]
                ^ t[4][(a >> 24) as usize]
                ^ t[3][chunk[4] as usize]
                ^ t[2][chunk[5] as usize]
                ^ t[1][chunk[6] as usize]
                ^ t[0][chunk[7] as usize];
        }
        for &byte in chunks.remainder() {
            c = (c >> 8) ^ t[0][((c ^ byte as u32) & 0xff) as usize];
        }
        self.0 = c;
    }

    pub fn finish(self) -> u32 {
        !self.0
    }
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = Crc32::default();
    crc.update(bytes);
    crc.finish()
}

// --- Zip ----------------------------------------------------------------------------

/// How much of a zip's end is read to find its directory: the end record
/// and the longest comment it may have.
pub const ZIP_TAIL: u64 = 22 + 65_535;

/// Where a zip's directory is, `(offset, size)`, from the file's last
/// `tail.len()` bytes; None when they hold no end record.
pub fn zip_directory(tail: &[u8], file_len: u64) -> Option<(u64, usize)> {
    let at = (0..=tail.len().checked_sub(22)?).rev().find(|&at| tail[at..at + 4] == [0x50, 0x4b, 0x05, 0x06])?;
    let size = u32::from_le_bytes(tail[at + 12..at + 16].try_into().ok()?) as u64;
    let offset = u32::from_le_bytes(tail[at + 16..at + 20].try_into().ok()?) as u64;
    (offset.checked_add(size)? <= file_len && size <= 1 << 20).then_some((offset, size as usize))
}

/// The CRC-32 and uncompressed size of each file a zip's directory lists
/// (folders left out).
pub fn zip_entries(directory: &[u8]) -> Vec<(u32, u64)> {
    let mut entries = Vec::new();
    let mut at = 0;
    while at + 46 <= directory.len() && directory[at..at + 4] == [0x50, 0x4b, 0x01, 0x02] {
        let word = |offset: usize| u32::from_le_bytes(directory[at + offset..at + offset + 4].try_into().unwrap_or_default());
        let half = |offset: usize| u16::from_le_bytes(directory[at + offset..at + offset + 2].try_into().unwrap_or_default()) as usize;
        let (name, extra, comment) = (half(28), half(30), half(32));
        let folder = directory.get(at + 46 + name.saturating_sub(1)) == Some(&b'/');
        if !folder && name > 0 {
            entries.push((word(16), word(24) as u64));
        }
        at += 46 + name + extra + comment;
    }
    entries
}

// --- Identifying games ---------------------------------------------------------------

/// A RetroArch game to identify.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rom {
    pub id: String,
    /// Its playlist label.
    pub label: String,
    /// Its content path.
    pub path: String,
}

/// The file reads identifying needs; the Vita host reads the memory card.
pub trait RomFiles {
    fn size(&mut self, path: &str) -> Option<u64>;
    fn read_at(&mut self, path: &str, offset: u64, len: usize) -> Option<Vec<u8>>;
    /// The CRC-32 of a file from byte `skip` to its end.
    fn crc(&mut self, path: &str, skip: u64) -> Option<u32>;
}

/// Loose files larger than this are not read: disc images, which the
/// databases name by serial, not CRC (the largest cartridge, GBA, is 32 MB).
pub const CRC_MAX: u64 = 64 * 1024 * 1024;

/// Disc images and playlists of discs, which are not read for a CRC.
const NOT_READ: [&str; 13] = ["chd", "cue", "iso", "cso", "pbp", "m3u", "ccd", "img", "gdi", "cdi", "mds", "7z", "rvz"];

/// The CRCs worked out so far, by content path (`gamedb/crcs.tsv`, one
/// file per line: the CRCs as hex separated by commas, a tab, the path).
/// A file whose CRCs match nothing keeps them too, so it is not read again.
#[derive(Default)]
pub struct Crcs {
    by_path: FxHashMap<String, Vec<u32>>,
    pub changed: bool,
}

/// One spelling of a path: device as written, lowercase, single slashes.
pub fn path_key(path: &str) -> String {
    let path = path.trim();
    let (device, rest) = match path.find(':') {
        Some(at) => (&path[..=at], &path[at + 1..]),
        None => ("", path),
    };
    let mut key = String::from(device);
    for part in rest.split('/').filter(|part| !part.is_empty()) {
        key.push('/');
        key.push_str(part);
    }
    key.to_lowercase()
}

impl Crcs {
    pub fn parse(text: &str) -> Crcs {
        let mut crcs = Crcs::default();
        for line in text.split('\n') {
            let Some((list, path)) = line.split_once('\t') else { continue };
            let values = list.split(',').filter(|v| !v.is_empty()).filter_map(|v| u32::from_str_radix(v, 16).ok()).collect();
            crcs.by_path.insert(path_key(path), values);
        }
        crcs
    }

    pub fn render(&self) -> String {
        let mut lines: Vec<String> = self
            .by_path
            .iter()
            .map(|(path, values)| format!("{}\t{path}", values.iter().map(|v| format!("{v:08x}")).collect::<Vec<_>>().join(",")))
            .collect();
        lines.sort();
        lines.into_iter().map(|line| line + "\n").collect()
    }

    pub fn len(&self) -> usize {
        self.by_path.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_path.is_empty()
    }
}

/// What a file's CRCs are: a zip's files', else the whole file's and, for
/// a size 512 bytes over a multiple of 1 KiB (a SNES copier header), the
/// CRC after those 512 bytes. None when it could not be read.
fn read_crcs(path: &str, files: &mut dyn RomFiles) -> Option<Vec<u32>> {
    let lower = path.to_lowercase();
    let extension = lower.rsplit_once('.').map_or("", |(_, e)| e);
    if NOT_READ.contains(&extension) {
        return None;
    }
    let size = files.size(path)?;
    if extension == "zip" {
        let start = size.saturating_sub(ZIP_TAIL);
        let tail = files.read_at(path, start, (size - start) as usize)?;
        let (offset, len) = zip_directory(&tail, size)?;
        let directory = files.read_at(path, offset, len)?;
        return Some(zip_entries(&directory).into_iter().map(|(crc, _)| crc).take(64).collect());
    }
    if size == 0 || size > CRC_MAX {
        return None;
    }
    let mut crcs = vec![files.crc(path, 0)?];
    if size % 1024 == 512 {
        crcs.extend(files.crc(path, 512));
    }
    Some(crcs)
}

/// How a run named its games, for the log.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Counts {
    /// The label is a database name.
    pub label: usize,
    /// An arcade set's zip name.
    pub rom: usize,
    /// A CRC kept from an earlier run.
    pub kept: usize,
    /// A CRC read this run.
    pub read: usize,
    /// Files read this run (matched or not).
    pub files: usize,
    /// Stopped before the end.
    pub stopped: bool,
}

/// Name each game from the table: `(title id, database name)` for those
/// found. `progress` is told the games done of the total after each and
/// stops the run when it answers false; CRCs read are added to `crcs`.
pub fn identify(table: &Table, roms: &[Rom], crcs: &mut Crcs, files: &mut dyn RomFiles, progress: &mut dyn FnMut(usize, usize) -> bool) -> (Vec<(String, String)>, Counts) {
    let mut names = Vec::new();
    let mut counts = Counts::default();
    for (done, rom) in roms.iter().enumerate() {
        if !progress(done, roms.len()) {
            counts.stopped = true;
            break;
        }
        let label = rom.label.trim();
        if table.has_name(label) {
            names.push((rom.id.clone(), label.to_string()));
            counts.label += 1;
            continue;
        }
        let file = rom.path.rsplit(['/', ':']).next().unwrap_or("").to_lowercase();
        let stem = file.rsplit_once('.').map_or(file.as_str(), |(stem, _)| stem);
        if let Some(name) = table.by_rom(stem) {
            names.push((rom.id.clone(), name.to_string()));
            counts.rom += 1;
            continue;
        }
        let key = path_key(&rom.path);
        let kept = crcs.by_path.contains_key(&key);
        if !kept {
            let Some(found) = read_crcs(&rom.path, files) else { continue };
            counts.files += 1;
            crcs.by_path.insert(key.clone(), found);
            crcs.changed = true;
        }
        let found = crcs.by_path.get(&key).and_then(|values| values.iter().find_map(|&crc| table.by_crc(crc)));
        if let Some(name) = found {
            names.push((rom.id.clone(), name.to_string()));
            if kept {
                counts.kept += 1;
            } else {
                counts.read += 1;
            }
        }
    }
    if !counts.stopped {
        progress(roms.len(), roms.len());
    }
    (names, counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MessagePack for a record: a map of keys to strings, binary or uints.
    enum V<'a> {
        S(&'a str),
        B(&'a [u8]),
        U(u32),
    }

    fn record(fields: &[(&str, V)]) -> Vec<u8> {
        let mut out = vec![0x80 | fields.len() as u8];
        for (key, value) in fields {
            out.push(0xa0 | key.len() as u8);
            out.extend(key.as_bytes());
            match value {
                V::S(text) => {
                    out.extend([0xd9, text.len() as u8]);
                    out.extend(text.as_bytes());
                }
                V::B(bytes) => {
                    out.extend([0xc4, bytes.len() as u8]);
                    out.extend(*bytes);
                }
                V::U(n) if *n < 128 => out.push(*n as u8),
                V::U(n) => {
                    out.push(0xcd);
                    out.extend((*n as u16).to_be_bytes());
                }
            }
        }
        out
    }

    fn rdb(records: &[Vec<u8>]) -> Vec<u8> {
        let mut body: Vec<u8> = records.concat();
        body.push(0xc0);
        let mut out = b"RARCHDB\0".to_vec();
        out.extend((16 + body.len() as u64).to_be_bytes());
        out.extend(body);
        // The metadata map, {"count": n}.
        out.extend([0x81, 0xa5]);
        out.extend(b"count");
        out.push(records.len() as u8);
        out
    }

    fn sample() -> Vec<u8> {
        rdb(&[
            record(&[("name", V::S("Tetris (World) (Rev 1)")), ("rom_name", V::S("Tetris (World) (Rev 1).gb")), ("size", V::U(32768)), ("crc", V::B(&[0x46, 0xdf, 0x91, 0xad]))]),
            // A second record of the same game: details only.
            record(&[("crc", V::B(&[0x46, 0xdf, 0x91, 0xad])), ("genre", V::S("Puzzle")), ("releaseyear", V::U(1989)), ("users", V::U(2)), ("developer", V::S("Bullet-Proof\tSoftware"))]),
            record(&[("name", V::S("Metal Slug - Super Vehicle-001")), ("rom_name", V::S("mslug.zip")), ("crc", V::B(&[1, 2, 3, 4]))]),
            record(&[("name", V::S("Daxter (USA)")), ("serial", V::B(b"UCUS-98618"))]),
            // No CRC and no serial: left out.
            record(&[("name", V::S("Nothing"))]),
        ])
    }

    #[test]
    fn records_merge_by_crc_into_a_table() {
        let rows = read_rdb(&sample()).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[0],
            Row {
                crc: Some(0x46df91ad),
                name: "Tetris (World) (Rev 1)".into(),
                year: 1989,
                genre: "Puzzle".into(),
                developer: "Bullet-Proof Software".into(),
                users: 2,
                ..Default::default()
            }
        );
        assert_eq!((rows[1].rom.as_str(), rows[2].serial.as_str()), ("mslug", "UCUS-98618"));
        let text = table_from_rdb(&sample(), 1_762_000_000_000).unwrap();
        assert_eq!(text.lines().next(), Some("#pocket-shelf-gamedb 1 1762000000000"));
        assert_eq!(text.lines().nth(1), Some("46df91ad\t\t\tTetris (World) (Rev 1)\t1989\tPuzzle\tBullet-Proof Software\t\t2"));
        let table = Table::parse(&text).unwrap();
        assert_eq!((table.built_ms, table.len()), (1_762_000_000_000, 3));
        assert_eq!(table.by_crc(0x46df91ad), Some("Tetris (World) (Rev 1)"));
        assert_eq!(table.by_rom("mslug"), Some("Metal Slug - Super Vehicle-001"));
        assert!(read_rdb(b"PK\x03\x04").is_err());
        assert!(table_from_rdb(&rdb(&[]), 0).is_err());
        assert_eq!(table_built("#pocket-shelf-gamedb 2 5"), None);
    }

    #[test]
    fn crc32_matches_zlib() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
        let long: Vec<u8> = (0..10_000u32).map(|n| (n * 7 % 251) as u8).collect();
        let mut parts = Crc32::default();
        parts.update(&long[..3]);
        parts.update(&long[3..]);
        let mut slow = !0u32;
        for &byte in &long {
            slow ^= byte as u32;
            for _ in 0..8 {
                slow = if slow & 1 != 0 { 0xedb8_8320 ^ (slow >> 1) } else { slow >> 1 };
            }
        }
        assert_eq!((crc32(&long), parts.finish()), (!slow, !slow));
    }

    /// A stored (uncompressed) zip of `files`, as zip tools write one.
    fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let (mut out, mut directory) = (Vec::new(), Vec::new());
        for (name, data) in files {
            let offset = out.len() as u32;
            let crc = crc32(data);
            out.extend([0x50, 0x4b, 3, 4, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend(crc.to_le_bytes());
            out.extend((data.len() as u32).to_le_bytes());
            out.extend((data.len() as u32).to_le_bytes());
            out.extend((name.len() as u16).to_le_bytes());
            out.extend([0, 0]);
            out.extend(name.as_bytes());
            out.extend(*data);
            directory.extend([0x50, 0x4b, 1, 2, 20, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            directory.extend(crc.to_le_bytes());
            directory.extend((data.len() as u32).to_le_bytes());
            directory.extend((data.len() as u32).to_le_bytes());
            directory.extend((name.len() as u16).to_le_bytes());
            directory.extend([0; 12]);
            directory.extend(offset.to_le_bytes());
            directory.extend(name.as_bytes());
        }
        let start = out.len() as u32;
        out.extend(&directory);
        out.extend([0x50, 0x4b, 5, 6, 0, 0, 0, 0]);
        out.extend((files.len() as u16).to_le_bytes());
        out.extend((files.len() as u16).to_le_bytes());
        out.extend((directory.len() as u32).to_le_bytes());
        out.extend(start.to_le_bytes());
        out.extend([2, 0, b'h', b'i']);
        out
    }

    #[test]
    fn a_zips_directory_gives_each_files_crc() {
        let bytes = zip(&[("docs/", b""), ("Game.gb", b"ROM DATA"), ("readme.txt", b"hello")]);
        let (offset, len) = zip_directory(&bytes, bytes.len() as u64).unwrap();
        let entries = zip_entries(&bytes[offset as usize..offset as usize + len]);
        assert_eq!(entries, [(crc32(b"ROM DATA"), 8), (crc32(b"hello"), 5)]);
        assert_eq!(zip_directory(b"not a zip at all, no end record here", 36), None);
    }

    #[derive(Default)]
    struct Files {
        files: std::collections::HashMap<String, Vec<u8>>,
        reads: usize,
    }

    impl RomFiles for Files {
        fn size(&mut self, path: &str) -> Option<u64> {
            self.files.get(path).map(|f| f.len() as u64)
        }
        fn read_at(&mut self, path: &str, offset: u64, len: usize) -> Option<Vec<u8>> {
            self.reads += 1;
            self.files.get(path)?.get(offset as usize..offset as usize + len).map(<[u8]>::to_vec)
        }
        fn crc(&mut self, path: &str, skip: u64) -> Option<u32> {
            self.reads += 1;
            self.files.get(path).map(|f| crc32(&f[skip as usize..]))
        }
    }

    #[test]
    fn games_are_named_by_label_set_name_or_crc_and_crcs_are_kept() {
        let snes_body = vec![7u8; 2048];
        let mut headered = vec![0u8; 512];
        headered.extend(&snes_body);
        let table_text = format!(
            "#pocket-shelf-gamedb 1 5\n{:08x}\t\t\tPixel Quest (USA)\n{:08x}\t\t\tKnight (Japan)\n{:08x}\t\t\tHeadered (USA)\n01020304\t\tmslug\tMetal Slug\n\t\t\tStar Saga (USA)\n",
            crc32(b"PIXEL"),
            crc32(b"KNIGHT"),
            crc32(&snes_body)
        );
        let table = Table::parse(&table_text).unwrap();
        let mut files = Files::default();
        files.files.insert("ux0:/roms/pq.zip".into(), zip(&[("pq.gb", b"PIXEL")]));
        files.files.insert("ux0:/roms/k.nes".into(), b"KNIGHT".to_vec());
        files.files.insert("ux0:/roms/h.smc".into(), headered);
        files.files.insert("ux0:/roms/other.gb".into(), b"OTHER".to_vec());
        let rom = |id: &str, label: &str, path: &str| Rom { id: id.into(), label: label.into(), path: path.into() };
        let roms = [
            rom("ra1", "Star Saga (USA)", "ux0:/roms/missing.gb"),
            rom("ra2", "Pixel Quest (U)", "ux0:/roms/pq.zip"),
            rom("ra3", "Knight (J) [!]", "ux0:/roms/k.nes"),
            rom("ra4", "mslug", "ux0:/neogeo/MSLUG.zip"),
            rom("ra5", "Other", "ux0:/roms/other.gb"),
            rom("ra6", "Disc", "ux0:/roms/disc.chd"),
            rom("ra7", "Headered", "ux0:/roms/h.smc"),
        ];
        let mut crcs = Crcs::default();
        let mut seen = Vec::new();
        let (names, counts) = identify(&table, &roms, &mut crcs, &mut files, &mut |done, total| {
            seen.push((done, total));
            true
        });
        let expected: Vec<(String, String)> = [("ra1", "Star Saga (USA)"), ("ra2", "Pixel Quest (USA)"), ("ra3", "Knight (Japan)"), ("ra4", "Metal Slug"), ("ra7", "Headered (USA)")]
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        assert_eq!(names, expected);
        assert_eq!(counts, Counts { label: 1, rom: 1, kept: 0, read: 3, files: 4, stopped: false });
        assert_eq!(seen.last(), Some(&(7, 7)));
        assert!(crcs.changed);

        // A second run reads no file.
        let mut kept = Crcs::parse(&crcs.render());
        assert_eq!(kept.len(), 4);
        let reads = files.reads;
        let (again, counts) = identify(&table, &roms, &mut kept, &mut files, &mut |_, _| true);
        assert_eq!((again, files.reads, counts.kept, kept.changed), (expected, reads, 3, false));

        // Stopping keeps what was found.
        let (some, counts) = identify(&table, &roms, &mut Crcs::default(), &mut files, &mut |done, _| done < 2);
        assert_eq!((some.len(), counts.stopped), (2, true));
    }

    #[test]
    fn databases_are_named_as_retroarch_names_them() {
        assert_eq!(database_name("Neo Geo"), "SNK - Neo Geo");
        assert_eq!(database_name(" MAME 2003-Plus "), "MAME 2003-Plus");
        assert_eq!(rdb_url("Nintendo - Game Boy"), format!("{SERVER}/Nintendo%20-%20Game%20Boy.rdb"));
        assert_eq!(table_path("Nintendo - Game Boy"), "gamedb/Nintendo-Game-Boy.tsv");
        assert_eq!(path_key("UX0:roms//GB/A.gb"), "ux0:/roms/gb/a.gb");
    }

    /// The databases in `.samples/gamedb/` when present (downloaded from
    /// libretro-database): each reads, and the table is smaller.
    #[test]
    fn sample_databases_read() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.samples/gamedb");
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let bytes = std::fs::read(entry.path()).unwrap();
            let started = std::time::Instant::now();
            let text = table_from_rdb(&bytes, 1).unwrap();
            let table = Table::parse(&text).unwrap();
            eprintln!("{:?}: {} games, {} KB of {} KB in {:?}", entry.file_name(), table.len(), text.len() / 1024, bytes.len() / 1024, started.elapsed());
            assert!(table.len() > 100 && text.len() < bytes.len());
        }
    }
}
