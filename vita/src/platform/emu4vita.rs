//! Emu4Vita++ (noword/Emu4VitaPlus, title id `EMU4VPLUS`): a libretro
//! frontend that ships every core in one app, `app0:eboot_<core>.self`.
//!
//! **Launch.** Its core picker reads the launch URI's `&core=<core>` and
//! `&rom=<path>` (`arch/source/main.cpp`, since June 2026, release 0.81),
//! skips its menu and runs `app0:eboot_<core>.self --arch --rom <path>`.
//! Each value ends at the next `&` and is then percent-decoded, `+` as a
//! space, so the path is sent with `%`, `&` and `+` escaped
//! (`launch_uri`). The core loads the file only when it exists.
//!
//! **Games** come from the places it reads names from
//! (`frontend/source/ra_lpl.cpp`): its own `playlists/` folder, RetroArch's
//! playlist folder on `ux0:` and on `uma0:`; and from each core's
//! favourites, `<data>/<short name>/favourite.ini`, which it writes
//! (`frontend/source/favorite.cpp`). It writes no playlists of its own.
//!
//! **Cores and consoles.** Which core opens which console is in its core
//! picker's code (`arch/source/app.cpp`), not in a file, so the tables here
//! copy it: `CORES` from `cmake/cores.cmake`, `DATABASES` from the picker's
//! buttons, its preferred core of each console first.
//!
//! This file uses std only, so its tests run on the build machine: `make
//! test` (platform-tests).

use std::fs;
use std::string::String;
use std::vec::Vec;

/// Emu4Vita++'s title id on the Vita.
pub const EMU4VITA_ID: &str = "EMU4VPLUS";

/// The longest launch URI sent (`retroarch::URI_MAX`); Emu4Vita++ reads the
/// URI into 2048 bytes.
pub const URI_MAX: usize = 1000;

/// Where Emu4Vita++ is installed and keeps its data, and the second
/// RetroArch data folder it reads playlists from. The tests point them at a
/// temporary folder.
pub struct Roots {
    /// The installed app: `eboot.bin` (the core picker), `eboot_<core>.self`.
    pub app: String,
    /// `ux0:data/EMU4VITAPLUS`: `playlists/`, `<short name>/favourite.ini`,
    /// `cache/thumbnails/`.
    pub data: String,
    /// RetroArch's data folder on `uma0:`, whose playlists it reads too.
    pub uma0_retroarch: String,
}

impl Roots {
    pub fn vita() -> Roots {
        Roots {
            app: String::from("ux0:/app/EMU4VPLUS"),
            data: String::from("ux0:/data/EMU4VITAPLUS"),
            uma0_retroarch: String::from("uma0:/data/retroarch"),
        }
    }

    /// Whether the app is installed: its `eboot.bin` is there.
    pub fn installed(&self) -> bool {
        fs::metadata(format!("{}/eboot.bin", self.app)).is_ok_and(|meta| meta.is_file())
    }
}

/// Every core Emu4Vita++ builds (`cmake/cores.cmake`): its name (the
/// `&core=` value and `eboot_<name>.self`), its short name (its data
/// folder) and its console.
const CORES: &[(&str, &str, &str)] = &[
    ("gpsp", "gpSP", "GBA"),
    ("vba_next", "VBANext", "GBA"),
    ("mgba", "mGBA", "GBA"),
    ("gambatte", "Gambatte", "GBC"),
    ("tgbdual", "TGBDual", "GBC"),
    ("fba_lite", "FBALite", "ARC"),
    ("fbneo", "FBNeo", "ARC"),
    ("fbalpha2012", "FBA2012", "ARC"),
    ("km_fbneo_xtreme_amped", "FBNeoXtreme", "ARC"),
    ("mame2000", "MAME2000", "ARC"),
    ("mame2003", "MAME2003", "ARC"),
    ("mame2003_plus", "MAME2003Plus", "ARC"),
    ("km_mame2003_xtreme_amped", "MAME2003Xtreme", "ARC"),
    ("snes9x2005_plus", "Snes9x2005Plus", "SNES"),
    ("snes9x", "Snes9x", "SNES"),
    ("mednafen_supafaust", "Supafaust", "SNES"),
    ("chimerasnes", "ChimeraSNES", "SNES"),
    ("fceumm", "FCEUmm", "NES"),
    ("nestopia", "Nestopia", "NES"),
    ("genesis_plus_gx", "GenesisPlusGX", "MD"),
    ("genesis_plus_gx_wide", "GenesisPlusGXWide", "MD"),
    ("picodrive", "PicoDrive", "MD"),
    ("mednafen_pce_fast", "MednafenPCEFast", "PCE"),
    ("mednafen_supergrafx", "MednafenPCESuperGrafx", "PCE"),
    ("mednafen_ngp", "MednafenNgp", "NGP"),
    ("mednafen_wswan", "MednafenWswan", "WSC"),
    ("pcsx_rearmed", "PCSXReARMed", "PS"),
    ("stella2014", "Stella2014", "ATARI2600"),
    ("prosystem", "ProSystem", "ATARI7800"),
    ("atari800", "Atari800", "ATARI5200"),
    ("dosbox_pure", "DOSBoxPure", "DOS"),
    ("vecx", "vecx", "VECTREX"),
    ("uae4arm", "uae4arm", "AMIGA"),
    ("fuse", "fuse", "ZXS"),
    ("neocd", "neocd", "NEOGEOCD"),
    ("nekop2", "nekop2", "PC98"),
    ("np2kai", "np2kai", "PC98"),
    ("fmsx", "fMSX", "MSX"),
    ("bluemsx", "blueMSX", "MSX"),
    ("vice", "vice", "C64"),
    ("px68k", "px68k", "X68000"),
    ("mednafen_lynx", "Lynx", "LYNX"),
    ("handy", "Handy", "LYNX"),
    ("cap32", "Caprice32", "CPC"),
    ("crocods", "CrocoDS", "CPC"),
    ("quasi88", "quasi88", "PC88"),
    ("mednafen_vb", "mednafen_vb", "VB"),
    ("gw", "gw", "GW"),
    ("hatari", "Hatari", "ATARIST"),
];

/// RetroArch's database names (consoles) and the cores that open them, the
/// core picker's preferred one (starred) first.
const DATABASES: &[(&str, &[&str])] = &[
    ("Nintendo - Nintendo Entertainment System", &["fceumm", "nestopia"]),
    ("Nintendo - Family Computer Disk System", &["fceumm", "nestopia"]),
    ("Nintendo - Super Nintendo Entertainment System", &["snes9x2005_plus", "chimerasnes", "snes9x", "mednafen_supafaust"]),
    ("Nintendo - Game Boy", &["gambatte", "tgbdual"]),
    ("Nintendo - Game Boy Color", &["gambatte", "tgbdual"]),
    ("Nintendo - Game Boy Advance", &["gpsp", "mgba", "vba_next"]),
    ("Nintendo - Virtual Boy", &["mednafen_vb"]),
    ("Sega - Mega Drive - Genesis", &["genesis_plus_gx", "genesis_plus_gx_wide", "picodrive"]),
    ("Sega - Master System - Mark III", &["genesis_plus_gx", "picodrive"]),
    ("Sega - Game Gear", &["genesis_plus_gx"]),
    ("Sega - SG-1000", &["genesis_plus_gx"]),
    ("Sega - Mega-CD - Sega CD", &["genesis_plus_gx", "picodrive"]),
    ("Sega - 32X", &["picodrive"]),
    ("Sony - PlayStation", &["pcsx_rearmed"]),
    ("NEC - PC Engine - TurboGrafx 16", &["mednafen_pce_fast", "mednafen_supergrafx"]),
    ("NEC - PC Engine CD - TurboGrafx-CD", &["mednafen_pce_fast", "mednafen_supergrafx"]),
    ("NEC - PC Engine SuperGrafx", &["mednafen_supergrafx"]),
    ("SNK - Neo Geo Pocket", &["mednafen_ngp"]),
    ("SNK - Neo Geo Pocket Color", &["mednafen_ngp"]),
    ("SNK - Neo Geo CD", &["neocd"]),
    ("Bandai - WonderSwan", &["mednafen_wswan"]),
    ("Bandai - WonderSwan Color", &["mednafen_wswan"]),
    ("Atari - 2600", &["stella2014"]),
    ("Atari - 5200", &["atari800"]),
    ("Atari - 7800", &["prosystem"]),
    ("Atari - Lynx", &["mednafen_lynx", "handy"]),
    ("Atari - ST", &["hatari"]),
    ("GCE - Vectrex", &["vecx"]),
    ("Handheld Electronic Game", &["gw"]),
    ("DOS", &["dosbox_pure"]),
    ("NEC - PC-98", &["nekop2", "np2kai"]),
    ("NEC - PC-8001 - PC-8801", &["quasi88"]),
    ("Microsoft - MSX", &["bluemsx", "fmsx"]),
    ("Microsoft - MSX2", &["bluemsx", "fmsx"]),
    ("Commodore - 64", &["vice"]),
    ("Commodore - Amiga", &["uae4arm"]),
    ("Sinclair - ZX Spectrum", &["fuse"]),
    ("Sinclair - ZX Spectrum +3", &["fuse"]),
    ("Amstrad - CPC", &["cap32", "crocods"]),
    ("Sharp - X68000", &["px68k"]),
    ("FBNeo - Arcade Games", &["fbneo", "km_fbneo_xtreme_amped", "fba_lite"]),
    ("FB Alpha - Arcade Games", &["fbalpha2012", "fbneo"]),
    ("MAME 2000", &["mame2000"]),
    ("MAME 2003", &["mame2003", "km_mame2003_xtreme_amped"]),
    ("MAME 2003-Plus", &["mame2003_plus"]),
    ("MAME", &["mame2003_plus", "mame2003", "mame2000"]),
    ("SNK - Neo Geo", &["fbneo", "km_fbneo_xtreme_amped", "fba_lite", "mame2003_plus"]),
    ("Neo Geo", &["fbneo", "km_fbneo_xtreme_amped", "fba_lite", "mame2003_plus"]),
];

/// RetroArch core names that Emu4Vita++ builds under another name.
const ALIASES: &[(&str, &str)] = &[
    ("vice_x64", "vice"),
    ("vice_x64sc", "vice"),
    ("snes9x2005", "snes9x2005_plus"),
    ("puae", "uae4arm"),
    ("mame2003_xtreme", "km_mame2003_xtreme_amped"),
    ("fbneo_xtreme", "km_fbneo_xtreme_amped"),
];

/// Emu4Vita++ installed, with the cores it carries.
pub struct Install {
    pub roots: Roots,
    /// Core names, from the app folder's `eboot_<core>.self` files, sorted.
    pub cores: Vec<String>,
}

impl Install {
    /// The install at `roots`; None when it is not installed.
    pub fn find(roots: Roots) -> Option<Install> {
        if !roots.installed() {
            return None;
        }
        let mut cores: Vec<String> = fs::read_dir(&roots.app)
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|entry| entry.file_name().to_str().map(String::from))
                    .filter_map(|name| Some(String::from(name.strip_prefix("eboot_")?.strip_suffix(".self")?)))
                    .filter(|core| !core.is_empty() && core.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
                    .collect()
            })
            .unwrap_or_default();
        cores.sort();
        Some(Install { roots, cores })
    }

    fn has(&self, core: &str) -> bool {
        self.cores.iter().any(|own| own == core)
    }

    /// The core that opens a playlist entry: the RetroArch core it names
    /// (`snes9x` of `snes9x_libretro.self`) when Emu4Vita++ carries it, else
    /// its preferred installed core for the console. None when it has none.
    pub fn core_for(&self, retroarch_core: Option<&str>, console: &str) -> Option<String> {
        if let Some(name) = retroarch_core {
            let name = ALIASES.iter().find(|(from, _)| *from == name).map_or(name, |(_, to)| to);
            if self.has(name) {
                return Some(String::from(name));
            }
        }
        let (_, cores) = DATABASES.iter().find(|(database, _)| *database == console)?;
        cores.iter().find(|core| self.has(core)).map(|core| String::from(*core))
    }

    /// Its installed cores that run a console, its preferred one first.
    pub fn cores_for(&self, console: &str) -> Vec<String> {
        let Some((_, cores)) = DATABASES.iter().find(|(database, _)| *database == console) else { return Vec::new() };
        cores.iter().filter(|core| self.has(core)).map(|core| String::from(*core)).collect()
    }

    /// The playlist folders it reads besides RetroArch's on `ux0:`: its own,
    /// then RetroArch's on `uma0:` (from that folder's `retroarch.cfg`).
    pub fn playlist_dirs(&self) -> Vec<String> {
        let uma0 = super::retroarch::Roots {
            app: super::retroarch::Roots::vita().app,
            data: self.roots.uma0_retroarch.clone(),
        };
        vec![format!("{}/playlists", self.roots.data), super::retroarch::playlist_dir(&uma0)]
    }
}

/// The cores Emu4Vita++ runs a console with, its preferred one first (its
/// names are libretro's: RetroArch's `snes9x_libretro.self` is `snes9x`).
pub fn console_cores(console: &str) -> &'static [&'static str] {
    DATABASES.iter().find(|(database, _)| *database == console).map_or(&[], |(_, cores)| cores)
}

/// A core's name from a RetroArch playlist's core path
/// (`app0:/snes9x_libretro.self`, `ux0:/app/RETROVITA/snes9x_libretro.self`);
/// None for `DETECT` or an empty path.
pub fn retroarch_core_name(core_path: &str) -> Option<&str> {
    let file = core_path.rsplit(['/', ':']).next()?;
    let name = file.strip_suffix(".self").unwrap_or(file).strip_suffix("_libretro")?;
    (!name.is_empty()).then_some(name)
}

/// The URI that starts `path` with Emu4Vita++'s core `core`.
pub fn launch_uri(core: &str, path: &str) -> String {
    format!("psgm:play?titleid={EMU4VITA_ID}&core={core}&rom={}", escape(path))
}

/// `%`, `&` and `+` as `%XX`: Emu4Vita++ ends a value at `&` and decodes
/// `%XX` and `+` (a space) in it.
fn escape(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        match c {
            '%' => out.push_str("%25"),
            '&' => out.push_str("%26"),
            '+' => out.push_str("%2B"),
            _ => out.push(c),
        }
    }
    out
}

/// A path as a key for finding the same file under another spelling: a
/// device's `:` followed by `/` (`ux0:roms` is `ux0:/roms`, as Emu4Vita++
/// reads it), no repeated `/`, ASCII lowercase (the memory card ignores
/// case).
pub fn path_key(path: &str) -> String {
    let path = path.trim();
    let (device, rest) = match path.find(':') {
        Some(at) => (&path[..=at], &path[at + 1..]),
        None => ("", path),
    };
    let mut key = String::with_capacity(path.len() + 1);
    key.push_str(device);
    for part in rest.split('/').filter(|part| !part.is_empty()) {
        key.push('/');
        key.push_str(part);
    }
    key.make_ascii_lowercase();
    key
}

/// A favourite as `favourite.ini` keeps it: the folder, the file in it,
/// and the name Emu4Vita++ found for it (its database name; empty when it
/// found none).
#[derive(Debug, PartialEq, Eq)]
pub struct Saved {
    pub folder: String,
    pub file: String,
    pub rom_name: String,
}

/// The entries of a `favourite.ini` (SimpleIni: a `[0000]` section per
/// favourite with `path`, `rom_name`, `name`, `entry` and `crc32`).
pub fn parse_favourites(text: &str) -> Vec<Saved> {
    let mut saved: Vec<Saved> = Vec::new();
    let mut current: Option<Saved> = None;
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            saved.extend(current.take());
            current = Some(Saved { folder: String::new(), file: String::new(), rom_name: String::new() });
            continue;
        }
        let (Some(entry), Some((key, value))) = (current.as_mut(), line.split_once('=')) else { continue };
        let value = String::from(value.trim());
        match key.trim() {
            "path" => entry.folder = value,
            "name" => entry.file = value,
            "rom_name" => entry.rom_name = value,
            _ => {}
        }
    }
    saved.extend(current);
    saved.retain(|entry| !entry.folder.is_empty() && !entry.file.is_empty());
    saved
}

/// A favourite as the launcher lists it.
#[derive(Debug, PartialEq, Eq)]
pub struct Favourite {
    /// The file Emu4Vita++ opens.
    pub path: String,
    pub title: String,
    /// The core whose favourite it is, which opens it.
    pub core: String,
    /// RetroArch's database name for its console, from the core and the
    /// file's extension.
    pub console: String,
    /// The box art Emu4Vita++ downloaded for it (it may not exist).
    pub art: String,
}

/// The console of a file opened with `core`, as RetroArch's database
/// names it. Cores that open several consoles go by the extension.
pub fn console_of(core: &str, path: &str) -> Option<&'static str> {
    let system = CORES.iter().find(|(name, _, _)| *name == core)?.2;
    let ext = path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default();
    let disc = matches!(ext.as_str(), "cue" | "chd" | "iso" | "ccd" | "m3u");
    Some(match system {
        "GBA" => "Nintendo - Game Boy Advance",
        "GBC" if ext == "gb" => "Nintendo - Game Boy",
        "GBC" => "Nintendo - Game Boy Color",
        "NES" if ext == "fds" => "Nintendo - Family Computer Disk System",
        "NES" => "Nintendo - Nintendo Entertainment System",
        "SNES" => "Nintendo - Super Nintendo Entertainment System",
        "MD" if ext == "sms" => "Sega - Master System - Mark III",
        "MD" if ext == "gg" => "Sega - Game Gear",
        "MD" if ext == "sg" => "Sega - SG-1000",
        "MD" if ext == "32x" => "Sega - 32X",
        "MD" if disc => "Sega - Mega-CD - Sega CD",
        "MD" => "Sega - Mega Drive - Genesis",
        "PCE" if disc => "NEC - PC Engine CD - TurboGrafx-CD",
        "PCE" if ext == "sgx" => "NEC - PC Engine SuperGrafx",
        "PCE" => "NEC - PC Engine - TurboGrafx 16",
        "NGP" if ext == "ngc" => "SNK - Neo Geo Pocket Color",
        "NGP" => "SNK - Neo Geo Pocket",
        "WSC" if ext == "wsc" => "Bandai - WonderSwan Color",
        "WSC" => "Bandai - WonderSwan",
        "PS" => "Sony - PlayStation",
        "ATARI2600" => "Atari - 2600",
        "ATARI5200" => "Atari - 5200",
        "ATARI7800" => "Atari - 7800",
        "LYNX" => "Atari - Lynx",
        "ATARIST" => "Atari - ST",
        "VECTREX" => "GCE - Vectrex",
        "GW" => "Handheld Electronic Game",
        "DOS" => "DOS",
        "AMIGA" => "Commodore - Amiga",
        "C64" => "Commodore - 64",
        "ZXS" => "Sinclair - ZX Spectrum",
        "MSX" => "Microsoft - MSX",
        "CPC" => "Amstrad - CPC",
        "X68000" => "Sharp - X68000",
        "PC98" => "NEC - PC-98",
        "PC88" => "NEC - PC-8001 - PC-8801",
        "VB" => "Nintendo - Virtual Boy",
        "NEOGEOCD" => "SNK - Neo Geo CD",
        "ARC" => match core {
            "fbalpha2012" => "FB Alpha - Arcade Games",
            "mame2000" => "MAME 2000",
            "mame2003" | "km_mame2003_xtreme_amped" => "MAME 2003",
            "mame2003_plus" => "MAME 2003-Plus",
            _ => "FBNeo - Arcade Games",
        },
        _ => return None,
    })
}

/// The folder of `cache/thumbnails` a core's box art is downloaded to
/// (`THUMBNAILS_PATH` in `frontend/source/core_spec/defines.cpp`).
fn thumbnail_folder(core: &str, system: &str) -> &'static str {
    match core {
        "fba_lite" | "fbalpha2012" | "fbneo" | "km_fbneo_xtreme_amped" => "FBA",
        "mame2000" | "mame2003" | "mame2003_plus" | "km_mame2003_xtreme_amped" => "MAME",
        _ => CORES.iter().find(|(_, _, own)| *own == system).map_or("", |(_, _, own)| own),
    }
}

/// The favourites of every installed core whose file still exists, in
/// `CORES` order, and a report (`Label=value` pairs separated by commas).
pub fn favourites(install: &Install) -> (Vec<Favourite>, String) {
    let mut found = Vec::new();
    let mut missing = 0usize;
    for (core, short, system) in CORES {
        if !install.has(core) {
            continue;
        }
        let Ok(text) = fs::read_to_string(format!("{}/{short}/favourite.ini", install.roots.data)) else { continue };
        for saved in parse_favourites(&text) {
            let path = format!("{}/{}", saved.folder.trim_end_matches('/'), saved.file.trim_start_matches('/'));
            if path.contains(['\t', '\n', '\r']) || launch_uri(core, &path).len() > URI_MAX {
                missing += 1;
                continue;
            }
            if !fs::metadata(&path).is_ok_and(|meta| meta.is_file()) {
                missing += 1;
                continue;
            }
            let Some(console) = console_of(core, &path) else { continue };
            let stem = {
                let name = saved.file.rsplit('/').next().unwrap_or(&saved.file);
                name.rsplit_once('.').map_or(name, |(stem, _)| stem)
            };
            let title = if saved.rom_name.is_empty() { stem } else { saved.rom_name.as_str() };
            let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
            let art = if saved.rom_name.is_empty() {
                String::new()
            } else {
                let folder = thumbnail_folder(core, system);
                format!("{}/cache/thumbnails/{folder}/{}.png", install.roots.data, saved.rom_name.replace('/', "_"))
            };
            found.push(Favourite { path, title, core: String::from(*core), console: String::from(console), art });
        }
    }
    let report = format!("Emu4Vita cores={},Emu4Vita favourites={},Emu4Vita favouritesMissing={missing}", install.cores.len(), found.len());
    (found, report)
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

    /// An install with `cores` in a temporary folder.
    fn setup(name: &str, cores: &[&str]) -> (Temp, Install) {
        let base = std::env::temp_dir().join(format!("pocket-emu4vita-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let roots = Roots {
            app: base.join("app").to_string_lossy().into_owned(),
            data: base.join("data").to_string_lossy().into_owned(),
            uma0_retroarch: base.join("uma0").to_string_lossy().into_owned(),
        };
        fs::create_dir_all(&roots.app).unwrap();
        fs::create_dir_all(&roots.data).unwrap();
        fs::write(format!("{}/eboot.bin", roots.app), b"").unwrap();
        for core in cores {
            fs::write(format!("{}/eboot_{core}.self", roots.app), b"").unwrap();
        }
        (Temp(base), Install::find(roots).unwrap())
    }

    #[test]
    fn finds_the_install_and_its_cores() {
        let (_temp, install) = setup("find", &["mgba", "gpsp", "snes9x2005_plus"]);
        assert_eq!(install.cores, ["gpsp", "mgba", "snes9x2005_plus"]);
        fs::remove_file(format!("{}/eboot.bin", install.roots.app)).unwrap();
        assert!(Install::find(Roots { app: install.roots.app.clone(), data: String::new(), uma0_retroarch: String::new() }).is_none());
    }

    #[test]
    fn picks_the_named_core_then_the_console_default() {
        let (_temp, install) = setup("pick", &["gpsp", "mgba", "snes9x", "vice", "fbneo"]);
        let gba = "Nintendo - Game Boy Advance";
        assert_eq!(install.core_for(Some("mgba"), gba).as_deref(), Some("mgba"));
        // Not carried: the console's preferred core.
        assert_eq!(install.core_for(Some("vba_next"), gba).as_deref(), Some("gpsp"));
        assert_eq!(install.core_for(None, gba).as_deref(), Some("gpsp"));
        // snes9x2005_plus is preferred, but not installed.
        assert_eq!(install.core_for(None, "Nintendo - Super Nintendo Entertainment System").as_deref(), Some("snes9x"));
        assert_eq!(install.core_for(Some("vice_x64"), "Commodore - 64").as_deref(), Some("vice"));
        assert_eq!(install.core_for(None, "Neo Geo").as_deref(), Some("fbneo"));
        assert_eq!(install.core_for(None, "Nintendo - Nintendo 64"), None);
        assert_eq!(install.core_for(None, "Unknown"), None);
    }

    #[test]
    fn reads_core_names_from_core_paths() {
        assert_eq!(retroarch_core_name("app0:/snes9x_libretro.self"), Some("snes9x"));
        assert_eq!(retroarch_core_name("ux0:/app/RETROVITA/mame2003_plus_libretro.self"), Some("mame2003_plus"));
        assert_eq!(retroarch_core_name("app0:gpsp_libretro.self"), Some("gpsp"));
        assert_eq!(retroarch_core_name("DETECT"), None);
        assert_eq!(retroarch_core_name(""), None);
    }

    #[test]
    fn escapes_the_path_in_the_launch_uri() {
        assert_eq!(launch_uri("gpsp", "ux0:/roms/gba/A.gba"), "psgm:play?titleid=EMU4VPLUS&core=gpsp&rom=ux0:/roms/gba/A.gba");
        assert_eq!(
            launch_uri("fceumm", "ux0:/roms/Tom & Jerry + 100%.nes"),
            "psgm:play?titleid=EMU4VPLUS&core=fceumm&rom=ux0:/roms/Tom %26 Jerry %2B 100%25.nes"
        );
    }

    #[test]
    fn path_keys_match_spellings_of_one_file() {
        assert_eq!(path_key("ux0:roms/GBA//A.gba"), "ux0:/roms/gba/a.gba");
        assert_eq!(path_key("ux0:/roms/gba/A.gba"), path_key("UX0:/Roms/GBA/A.gba"));
        assert_ne!(path_key("ux0:/roms/a.gba"), path_key("uma0:/roms/a.gba"));
    }

    #[test]
    fn parses_favourite_files() {
        let text = "\u{feff}[0000]\npath = ux0:/roms/gba\nrom_name = Advance Wars (USA)\nname = aw.gba\nentry = \ncrc32 = 123\n\n[0001]\npath=ux0:/roms/gba/\nname=Plain.gba\n\n[0002]\nrom_name = No File\n";
        assert_eq!(
            parse_favourites(text),
            vec![
                Saved { folder: "ux0:/roms/gba".into(), file: "aw.gba".into(), rom_name: "Advance Wars (USA)".into() },
                Saved { folder: "ux0:/roms/gba/".into(), file: "Plain.gba".into(), rom_name: String::new() },
            ]
        );
        assert!(parse_favourites("name = x\n").is_empty());
    }

    #[test]
    fn lists_the_favourites_of_installed_cores() {
        let (temp, install) = setup("favourites", &["gpsp", "gambatte"]);
        let roms = temp.0.join("roms").to_string_lossy().into_owned();
        fs::create_dir_all(&roms).unwrap();
        for file in ["aw.gba", "Tetris.gb", "Zelda.gbc", "Mario.nes"] {
            fs::write(format!("{roms}/{file}"), b"").unwrap();
        }
        let write = |short: &str, text: String| {
            fs::create_dir_all(format!("{}/{short}", install.roots.data)).unwrap();
            fs::write(format!("{}/{short}/favourite.ini", install.roots.data), text).unwrap();
        };
        write("gpSP", format!("[0000]\npath = {roms}\nrom_name = Advance Wars (USA)\nname = aw.gba\n[0001]\npath = {roms}\nname = gone.gba\n"));
        write("Gambatte", format!("[0000]\npath = {roms}/\nname = Tetris.gb\n[0001]\npath = {roms}\nname = Zelda.gbc\nrom_name = A/B\n"));
        // fceumm is not installed: its favourites are not listed.
        write("FCEUmm", format!("[0000]\npath = {roms}\nname = Mario.nes\n"));
        let (found, report) = favourites(&install);
        let summary: Vec<(&str, &str, &str)> = found.iter().map(|f| (f.title.as_str(), f.core.as_str(), f.console.as_str())).collect();
        assert_eq!(
            summary,
            [
                ("Advance Wars (USA)", "gpsp", "Nintendo - Game Boy Advance"),
                ("Tetris", "gambatte", "Nintendo - Game Boy"),
                ("A/B", "gambatte", "Nintendo - Game Boy Color"),
            ]
        );
        assert_eq!(found[1].path, format!("{roms}/Tetris.gb"));
        assert_eq!(found[0].art, format!("{}/cache/thumbnails/GBA/Advance Wars (USA).png", install.roots.data));
        assert_eq!(found[1].art, "");
        assert_eq!(found[2].art, format!("{}/cache/thumbnails/GBC/A_B.png", install.roots.data));
        assert_eq!(report, "Emu4Vita cores=2,Emu4Vita favourites=3,Emu4Vita favouritesMissing=1");
    }

    #[test]
    fn names_consoles_by_core_and_extension() {
        assert_eq!(console_of("genesis_plus_gx", "a.sms"), Some("Sega - Master System - Mark III"));
        assert_eq!(console_of("genesis_plus_gx", "a.MD"), Some("Sega - Mega Drive - Genesis"));
        assert_eq!(console_of("mednafen_pce_fast", "a.cue"), Some("NEC - PC Engine CD - TurboGrafx-CD"));
        assert_eq!(console_of("mame2003_plus", "pacman.zip"), Some("MAME 2003-Plus"));
        assert_eq!(console_of("fba_lite", "mslug.zip"), Some("FBNeo - Arcade Games"));
        assert_eq!(console_of("nope", "a.zip"), None);
        // Every core has a console, and every database's cores exist.
        for (core, _, _) in CORES {
            assert!(console_of(core, "a.bin").is_some(), "{core}");
        }
        for (database, cores) in DATABASES {
            for core in *cores {
                assert!(CORES.iter().any(|(name, _, _)| name == core), "{database}: {core}");
            }
        }
    }
}
