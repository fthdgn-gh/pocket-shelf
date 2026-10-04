//! PSP disc images (`.iso` and `.cso`) for the installed-title list (cargo
//! feature `installed-apps`).
//!
//! An image is an ISO 9660 file system in 2048-byte sectors. A CSO holds the
//! same sectors in blocks, each stored plain or compressed with raw deflate:
//! a 24-byte header (`CISO`, header size, total bytes as u64, block size,
//! version, index shift), then one u32 per block plus one, each the block's
//! offset shifted right by the index shift. In version 1 the top bit marks a
//! plain block; in version 2 it marks an LZ4 block unless the stored size is a
//! whole block (then plain), and LZ4 is not read here. ZSO (all LZ4) and DAX
//! images are not read either.
//!
//! The scan reads `PSP_GAME/PARAM.SFO` for the id and name, the icon and
//! picture come from `PSP_GAME/ICON0.PNG` and `PIC1.PNG`, and a launch asks
//! whether `PSP_GAME/SYSDIR/EBOOT.OLD` (a patched game boots that) and
//! `BOOT.BIN` exist.
//! The file system is pure text and byte work; its tests run natively with
//! `src/tools/iso-test.sh`.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::string::String;
use std::vec::Vec;

const SECTOR: usize = 2048;
/// The primary volume descriptor's sector.
const PVD_SECTOR: u32 = 16;
/// Largest file read out of an image: `PARAM.SFO` is a few KiB, `PIC1.PNG`
/// a few hundred.
const FILE_MAX: u32 = 4 * 1024 * 1024;
/// Largest CSO block accepted; images use 2048, some tools larger powers of two.
const BLOCK_MAX: u32 = 64 * 1024;

/// Sector access to an image, plain or compressed.
pub trait Sectors {
    /// Read `count` sectors starting at `first` into `out`.
    fn read(&mut self, first: u32, count: u32, out: &mut Vec<u8>) -> Result<(), String>;
}

/// A plain image file, or any reader of one.
pub struct Plain<R: Read + Seek>(pub R);

impl<R: Read + Seek> Sectors for Plain<R> {
    fn read(&mut self, first: u32, count: u32, out: &mut Vec<u8>) -> Result<(), String> {
        self.0
            .seek(SeekFrom::Start(first as u64 * SECTOR as u64))
            .map_err(|_| String::from("seek"))?;
        let at = out.len();
        out.resize(at + count as usize * SECTOR, 0);
        self.0.read_exact(&mut out[at..]).map_err(|_| String::from("shortIso"))
    }
}

/// A CSO image: its index and the one block last read.
pub struct Cso<R: Read + Seek> {
    file: R,
    version: u8,
    shift: u32,
    block_size: u32,
    index: Vec<u32>,
    cached: Option<(u32, Vec<u8>)>,
}

impl<R: Read + Seek> Cso<R> {
    pub fn open(mut file: R) -> Result<Self, String> {
        let mut header = [0u8; 24];
        file.read_exact(&mut header).map_err(|_| String::from("shortCso"))?;
        if &header[..4] != b"CISO" {
            return Err(String::from("notCso"));
        }
        let total = u64::from_le_bytes(header[8..16].try_into().unwrap());
        let block_size = u32::from_le_bytes(header[16..20].try_into().unwrap());
        let (version, shift) = (header[20], header[21] as u32);
        if block_size < SECTOR as u32 || block_size > BLOCK_MAX || !block_size.is_power_of_two() || shift > 16 {
            return Err(format!("csoBlock {block_size}"));
        }
        if version > 2 {
            return Err(format!("csoVersion {version}"));
        }
        let blocks = total.div_ceil(block_size as u64);
        // A UMD holds under 2 GB: about a million 2048-byte blocks.
        if blocks == 0 || blocks > 1 << 21 {
            return Err(format!("csoSize {total}"));
        }
        let mut raw = vec![0u8; (blocks as usize + 1) * 4];
        file.read_exact(&mut raw).map_err(|_| String::from("csoIndex"))?;
        let index = raw.chunks_exact(4).map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap())).collect();
        Ok(Cso { file, version, shift, block_size, index, cached: None })
    }

    /// The uncompressed bytes of one block.
    fn block(&mut self, block: u32) -> Result<&[u8], String> {
        if self.cached.as_ref().is_some_and(|(cached, _)| *cached == block) {
            return Ok(&self.cached.as_ref().unwrap().1);
        }
        let entry = *self.index.get(block as usize).ok_or("csoRange")?;
        let next = *self.index.get(block as usize + 1).ok_or("csoRange")?;
        let start = ((entry & 0x7FFF_FFFF) as u64) << self.shift;
        let end = ((next & 0x7FFF_FFFF) as u64) << self.shift;
        // The last block's stored bytes can run past the next entry by the
        // alignment; read what the entries span.
        let stored = end.checked_sub(start).ok_or("csoIndex")? as usize;
        if stored > self.block_size as usize * 2 {
            return Err(String::from("csoIndex"));
        }
        let mut bytes = vec![0u8; stored];
        self.file.seek(SeekFrom::Start(start)).map_err(|_| String::from("seek"))?;
        self.file.read_exact(&mut bytes).map_err(|_| String::from("shortCso"))?;
        let flagged = entry & 0x8000_0000 != 0;
        let size = self.block_size as usize;
        let plain = if self.version < 2 { flagged } else { stored >= size };
        let data = if plain {
            bytes.truncate(size);
            bytes
        } else if self.version >= 2 && flagged {
            return Err(String::from("csoLz4"));
        } else {
            let mut out = vec![0u8; size];
            let mut inflater = miniz_oxide::inflate::core::DecompressorOxide::new();
            let (status, _, written) = miniz_oxide::inflate::core::decompress(
                &mut inflater,
                &bytes,
                &mut out,
                0,
                miniz_oxide::inflate::core::inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
            );
            if (status as i32) < 0 || written == 0 {
                return Err(String::from("csoInflate"));
            }
            out.truncate(written);
            out
        };
        self.cached = Some((block, data));
        Ok(&self.cached.as_ref().unwrap().1)
    }
}

impl<R: Read + Seek> Sectors for Cso<R> {
    fn read(&mut self, first: u32, count: u32, out: &mut Vec<u8>) -> Result<(), String> {
        let per_block = self.block_size / SECTOR as u32;
        for sector in first..first + count {
            let (block, within) = (sector / per_block, (sector % per_block) as usize * SECTOR);
            let data = self.block(block)?;
            let bytes = data.get(within..within + SECTOR).ok_or("csoShortBlock")?;
            out.extend_from_slice(bytes);
        }
        Ok(())
    }
}

/// Open an image by its extension: `.cso` compressed, anything else plain.
pub fn open(path: &str) -> Result<Box<dyn Sectors>, String> {
    let file = fs::File::open(path).map_err(|_| String::from("open"))?;
    if path.to_ascii_lowercase().ends_with(".cso") {
        Ok(Box::new(Cso::open(std::io::BufReader::new(file))?))
    } else {
        Ok(Box::new(Plain(file)))
    }
}

/// A directory record: where its data starts, how long it is, and whether it
/// is a directory.
#[derive(Clone, Copy)]
struct Entry {
    sector: u32,
    size: u32,
    directory: bool,
}

/// The root directory, from the primary volume descriptor.
fn root(image: &mut dyn Sectors) -> Result<Entry, String> {
    let mut pvd = Vec::new();
    image.read(PVD_SECTOR, 1, &mut pvd)?;
    if pvd[0] != 1 || &pvd[1..6] != b"CD001" {
        return Err(String::from("notIso9660"));
    }
    let record = &pvd[156..190];
    Ok(Entry {
        sector: u32::from_le_bytes(record[2..6].try_into().unwrap()),
        size: u32::from_le_bytes(record[10..14].try_into().unwrap()),
        directory: true,
    })
}

/// The entry named `name` in a directory, compared without case and without
/// a `;1` version.
fn find(image: &mut dyn Sectors, directory: Entry, name: &str) -> Result<Option<Entry>, String> {
    if directory.size > FILE_MAX {
        return Err(String::from("dirSize"));
    }
    let mut data = Vec::new();
    image.read(directory.sector, directory.size.div_ceil(SECTOR as u32), &mut data)?;
    data.truncate(directory.size as usize);
    let mut at = 0;
    while at < data.len() {
        let length = data[at] as usize;
        if length == 0 {
            // Records do not cross sectors: the rest of this one is padding.
            at = (at / SECTOR + 1) * SECTOR;
            continue;
        }
        let record = data.get(at..at + length).ok_or("dirRecord")?;
        if record.len() < 34 {
            return Err(String::from("dirRecord"));
        }
        let name_length = record[32] as usize;
        let raw = record.get(33..33 + name_length).ok_or("dirRecord")?;
        let found = raw.split(|&byte| byte == b';').next().unwrap_or(raw);
        if found.eq_ignore_ascii_case(name.as_bytes()) {
            return Ok(Some(Entry {
                sector: u32::from_le_bytes(record[2..6].try_into().unwrap()),
                size: u32::from_le_bytes(record[10..14].try_into().unwrap()),
                directory: record[25] & 2 != 0,
            }));
        }
        at += length;
    }
    Ok(None)
}

/// The entry at a path such as `PSP_GAME/PARAM.SFO`.
fn lookup(image: &mut dyn Sectors, path: &str) -> Result<Option<Entry>, String> {
    let mut entry = root(image)?;
    for part in path.split('/') {
        if !entry.directory {
            return Ok(None);
        }
        match find(image, entry, part)? {
            Some(next) => entry = next,
            None => return Ok(None),
        }
    }
    Ok(Some(entry))
}

/// The bytes of the file at `path` in the image, None when it has none.
pub fn read_file(image: &mut dyn Sectors, path: &str) -> Result<Option<Vec<u8>>, String> {
    let Some(entry) = lookup(image, path)? else {
        return Ok(None);
    };
    if entry.directory || entry.size > FILE_MAX {
        return Ok(None);
    }
    let mut data = Vec::new();
    image.read(entry.sector, entry.size.div_ceil(SECTOR as u32), &mut data)?;
    data.truncate(entry.size as usize);
    Ok(Some(data))
}

/// Whether the image has a file at `path`.
pub fn has_file(image: &mut dyn Sectors, path: &str) -> Result<bool, String> {
    Ok(lookup(image, path)?.is_some_and(|entry| !entry.directory))
}

/// The game's `PARAM.SFO`, or why it cannot be read.
pub fn param_sfo(path: &str) -> Result<Vec<u8>, String> {
    let mut image = open(path)?;
    read_file(image.as_mut(), "PSP_GAME/PARAM.SFO")?.ok_or_else(|| String::from("noParamSfo"))
}

/// One of the game's pictures (`ICON0.PNG`, `PIC1.PNG`). Callable from any thread.
pub fn picture(path: &str, name: &str) -> Option<Vec<u8>> {
    let mut image = open(path).ok()?;
    read_file(image.as_mut(), &format!("PSP_GAME/{name}")).ok().flatten()
}

/// What a launch boots: `EBOOT.OLD` when the image has one (a patched game
/// keeps the original there), else `EBOOT.BIN`; and whether the image has a
/// `BOOT.BIN`, which the plugin boots instead when the user's Adrenaline
/// setting asks for it.
pub fn boot_files(path: &str) -> (&'static str, bool) {
    let Ok(mut image) = open(path) else {
        return ("EBOOT.BIN", false);
    };
    let has = |image: &mut Box<dyn Sectors>, name: &str| {
        has_file(image.as_mut(), &format!("PSP_GAME/SYSDIR/{name}")).unwrap_or(false)
    };
    let old = has(&mut image, "EBOOT.OLD");
    let boot_bin = has(&mut image, "BOOT.BIN");
    (if old { "EBOOT.OLD" } else { "EBOOT.BIN" }, boot_bin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// A small ISO 9660 image: root, `PSP_GAME/` with `PARAM.SFO` and
    /// `SYSDIR/` with `EBOOT.BIN` (and `EBOOT.OLD` when asked).
    fn image(old: bool) -> Vec<u8> {
        let mut disc = vec![0u8; SECTOR * 24];
        let record = |sector: u32, size: u32, dir: bool, name: &[u8]| {
            let mut out = vec![0u8; 33 + name.len() + (name.len() + 1) % 2];
            out[0] = out.len() as u8;
            out[2..6].copy_from_slice(&sector.to_le_bytes());
            out[10..14].copy_from_slice(&size.to_le_bytes());
            out[25] = if dir { 2 } else { 0 };
            out[32] = name.len() as u8;
            out[33..33 + name.len()].copy_from_slice(name);
            out
        };
        let put = |disc: &mut Vec<u8>, sector: usize, records: &[Vec<u8>]| {
            let mut at = sector * SECTOR;
            for item in records {
                disc[at..at + item.len()].copy_from_slice(item);
                at += item.len();
            }
        };
        let pvd = 16 * SECTOR;
        disc[pvd] = 1;
        disc[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
        let root = record(18, SECTOR as u32, true, &[0]);
        disc[pvd + 156..pvd + 156 + root.len()].copy_from_slice(&root);
        put(&mut disc, 18, &[record(18, 2048, true, &[0]), record(18, 2048, true, &[1]), record(19, 2048, true, b"PSP_GAME")]);
        put(&mut disc, 19, &[record(20, 2048, true, b"SYSDIR"), record(21, 5, false, b"PARAM.SFO;1")]);
        let mut sysdir = vec![record(22, 4, false, b"EBOOT.BIN;1")];
        if old {
            sysdir.push(record(23, 4, false, b"EBOOT.OLD;1"));
        }
        put(&mut disc, 20, &sysdir);
        disc[21 * SECTOR..21 * SECTOR + 5].copy_from_slice(b"\0PSF!");
        disc
    }

    /// The image as a CSO with 2048-byte blocks: version 1 marks plain blocks
    /// with the top bit; the rest are raw deflate.
    fn cso(disc: &[u8], version: u8) -> Vec<u8> {
        let blocks = disc.len() / SECTOR;
        let mut header = vec![0u8; 24];
        header[..4].copy_from_slice(b"CISO");
        header[4..8].copy_from_slice(&24u32.to_le_bytes());
        header[8..16].copy_from_slice(&(disc.len() as u64).to_le_bytes());
        header[16..20].copy_from_slice(&(SECTOR as u32).to_le_bytes());
        header[20] = version;
        let mut body = Vec::new();
        let mut index = Vec::new();
        let base = 24 + (blocks + 1) * 4;
        for (number, block) in disc.chunks(SECTOR).enumerate() {
            let offset = (base + body.len()) as u32;
            if number % 3 == 0 {
                // Stored plain.
                index.push(if version < 2 { offset | 0x8000_0000 } else { offset });
                body.extend_from_slice(block);
            } else {
                index.push(offset);
                body.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(block, 6));
            }
        }
        index.push((base + body.len()) as u32);
        let mut out = header;
        for entry in index {
            out.extend_from_slice(&entry.to_le_bytes());
        }
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn reads_files_from_a_plain_image() {
        let mut disc = Plain(Cursor::new(image(false)));
        assert_eq!(read_file(&mut disc, "PSP_GAME/PARAM.SFO").unwrap().unwrap(), b"\0PSF!");
        assert_eq!(read_file(&mut disc, "psp_game/param.sfo").unwrap().unwrap(), b"\0PSF!");
        assert!(read_file(&mut disc, "PSP_GAME/ICON0.PNG").unwrap().is_none());
        assert!(has_file(&mut disc, "PSP_GAME/SYSDIR/EBOOT.BIN").unwrap());
        assert!(!has_file(&mut disc, "PSP_GAME/SYSDIR/EBOOT.OLD").unwrap());
        assert!(!has_file(&mut disc, "PSP_GAME/SYSDIR").unwrap());
        let mut patched = Plain(Cursor::new(image(true)));
        assert!(has_file(&mut patched, "PSP_GAME/SYSDIR/EBOOT.OLD").unwrap());
    }

    #[test]
    fn reads_the_same_files_from_cso_images() {
        for version in [1, 2] {
            let mut disc = Cso::open(Cursor::new(cso(&image(true), version))).unwrap();
            assert_eq!(read_file(&mut disc, "PSP_GAME/PARAM.SFO").unwrap().unwrap(), b"\0PSF!");
            assert!(has_file(&mut disc, "PSP_GAME/SYSDIR/EBOOT.OLD").unwrap());
        }
    }

    #[test]
    fn refuses_what_is_not_an_image() {
        let mut empty = Plain(Cursor::new(vec![0u8; SECTOR * 20]));
        assert_eq!(read_file(&mut empty, "PSP_GAME/PARAM.SFO").unwrap_err(), "notIso9660");
        assert_eq!(Cso::open(Cursor::new(b"ZISO and more bytes here".to_vec())).err().unwrap(), "notCso");
        let mut lz4 = cso(&image(false), 2);
        // Mark block 1 as LZ4.
        let entry = 24 + 4;
        let value = u32::from_le_bytes(lz4[entry..entry + 4].try_into().unwrap()) | 0x8000_0000;
        lz4[entry..entry + 4].copy_from_slice(&value.to_le_bytes());
        let mut disc = Cso::open(Cursor::new(lz4)).unwrap();
        let mut out = Vec::new();
        assert_eq!(disc.read(1, 1, &mut out).unwrap_err(), "csoLz4");
    }
}
