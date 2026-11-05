//! The folders standalone emulators keep their games in, read from their own
//! files, so their games are listed with no setup (they join the console
//! tabs as playlist games do). Checked in each emulator's source:
//!
//! - DaedalusX64 (`Source/SysVita/UI/MainMenuScreen.cpp`) lists `Roms/` in
//!   `<device>/data/DaedalusX64/` and `<device>/pspemu/PSP/GAME/DaedalusX64/`
//!   on `ux0:` and `uma0:`, and the folder set in its menu, which
//!   `loadCustomRomPath` reads as the bytes of
//!   `ux0:/data/DaedalusX64/Configs/path.ini` (up to 256, no line end).
//! - DSVita (`src/presenter/vita.rs`, `ROM_PATH`) lists `ux0:data/dsvita`.
//! - Flycast (`core/cfg/option.cpp`) keeps its content folders as
//!   `Dreamcast.ContentPath` in `ux0:data/flycast/emu.cfg`: `;` between
//!   folders, a folder holding `;` or starting with `"` in quotes, `""` for a
//!   quote (`core/cfg/option.h`).
//! - NooDS keeps none: its file browser starts at `ux0:` each time.

use crate::library::settings::Emulator;

/// A file of the emulator's that names more folders.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Config {
    /// DaedalusX64's custom ROM folder: the file's text.
    DaedalusPath(&'static str),
    /// Flycast's `emu.cfg`.
    FlycastCfg(&'static str),
}

/// An emulator's fixed folders, and the file that names more.
pub fn folders(app: Emulator) -> (&'static [&'static str], Option<Config>) {
    match app {
        Emulator::DaedalusX64 => (
            &[
                "ux0:/data/DaedalusX64/Roms",
                "uma0:/data/DaedalusX64/Roms",
                "ux0:/pspemu/PSP/GAME/DaedalusX64/Roms",
                "uma0:/pspemu/PSP/GAME/DaedalusX64/Roms",
            ],
            Some(Config::DaedalusPath("ux0:/data/DaedalusX64/Configs/path.ini")),
        ),
        Emulator::DsVita => (&["ux0:/data/dsvita"], None),
        Emulator::Flycast => (&[], Some(Config::FlycastCfg("ux0:/data/flycast/emu.cfg"))),
        _ => (&[], None),
    }
}

/// The folders a config file's bytes name.
pub fn config_folders(config: Config, bytes: &[u8]) -> Vec<String> {
    match config {
        Config::DaedalusPath(_) => {
            // `fread` of up to 256 bytes into a zeroed buffer: the path ends
            // at the first NUL; a line end typed by hand is left out too.
            let bytes = &bytes[..bytes.len().min(256)];
            let end = bytes.iter().position(|b| matches!(b, 0 | b'\r' | b'\n')).unwrap_or(bytes.len());
            let path = String::from_utf8_lossy(&bytes[..end]).trim().to_string();
            if path.len() > 1 { vec![path] } else { Vec::new() }
        }
        Config::FlycastCfg(_) => flycast_content_paths(&String::from_utf8_lossy(bytes)),
    }
}

/// `Dreamcast.ContentPath` in a Flycast `emu.cfg` text, as Flycast splits it.
pub fn flycast_content_paths(text: &str) -> Vec<String> {
    let Some(value) = text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "Dreamcast.ContentPath").then(|| value.trim().to_string())
    }) else {
        return Vec::new();
    };
    let chars: Vec<char> = value.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let mut path = String::new();
        if chars[i] == '"' {
            i += 1;
            while i < chars.len() {
                if chars[i] == '"' {
                    if chars.get(i + 1) == Some(&'"') {
                        path.push('"');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                path.push(chars[i]);
                i += 1;
            }
            // Past the `;` after the quotes.
            while i < chars.len() && chars[i] != ';' {
                i += 1;
            }
            i += 1;
        } else {
            while i < chars.len() && chars[i] != ';' {
                path.push(chars[i]);
                i += 1;
            }
            i += 1;
        }
        let path = path.trim().to_string();
        if !path.is_empty() {
            out.push(path);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_come_from_each_emulators_own_files() {
        assert_eq!(folders(Emulator::DsVita).0, ["ux0:/data/dsvita"]);
        assert_eq!(folders(Emulator::NooDs), (&[][..], None));
        let daedalus = Config::DaedalusPath("path.ini");
        let mut bytes = b"ux0:/n64/".to_vec();
        bytes.resize(256, 0);
        assert_eq!(config_folders(daedalus, &bytes), ["ux0:/n64/"]);
        assert_eq!(config_folders(daedalus, b"ux0:/roms/n64\r\n"), ["ux0:/roms/n64"]);
        assert!(config_folders(daedalus, b"\0\0").is_empty());
        let cfg = "[config]\nDreamcast.Broadcast = 4\nDreamcast.ContentPath = ux0:/data/flycast/roms;\"ux0:/dc;games\";\"say \"\"hi\"\"\"\n";
        assert_eq!(flycast_content_paths(cfg), ["ux0:/data/flycast/roms", "ux0:/dc;games", "say \"hi\""]);
        assert!(flycast_content_paths("[config]\nDreamcast.ContentPath = \n").is_empty());
        assert!(flycast_content_paths("").is_empty());
    }
}
