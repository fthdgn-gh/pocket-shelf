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
//! - ScummVM's games are the ones added to it, in
//!   `ux0:data/scummvm/scummvm.ini` (`OSystem_PSP2::getDefaultConfigFileName`):
//!   a section per game, named by its target, with its `path`, `gameid` and
//!   `description` ([`scummvm_games`]). Its Vita build starts one from
//!   `psgm:play?titleid=VSCU00001&path=<folder>&game_id=<target>`
//!   (`backends/platform/sdl/psp2/psp2-main.cpp`), into buffers of 256 and
//!   64 bytes.

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

/// ScummVM's list of games.
pub const SCUMMVM_INI: &str = "ux0:/data/scummvm/scummvm.ini";

/// A game added to ScummVM.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScummGame {
    /// The section's name, which the launch names the game by: ScummVM uses
    /// its settings (`monkey`, `monkey-1`).
    pub target: String,
    pub description: String,
    pub path: String,
}

/// The longest folder and target ScummVM's launch takes (its buffers, with
/// the closing NUL).
pub const SCUMMVM_PATH_MAX: usize = 255;
pub const SCUMMVM_TARGET_MAX: usize = 63;

/// The games in a `scummvm.ini` text: every section but ScummVM's own
/// (`scummvm`, `keymapper`, `cloud`) with a `gameid` and a `path`. Targets
/// and folders the launch could not carry are left out.
pub fn scummvm_games(text: &str) -> Vec<ScummGame> {
    let mut games = Vec::new();
    let mut section: Option<(String, Vec<(String, String)>)> = None;
    let mut finish = |section: Option<(String, Vec<(String, String)>)>, games: &mut Vec<ScummGame>| {
        let Some((target, keys)) = section else { return };
        let get = |key: &str| keys.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).unwrap_or_default();
        let (path, gameid) = (get("path"), get("gameid"));
        let usable = !matches!(target.as_str(), "scummvm" | "keymapper" | "cloud")
            && !gameid.is_empty()
            && !path.is_empty()
            && !target.is_empty()
            && target.len() <= SCUMMVM_TARGET_MAX
            && path.len() <= SCUMMVM_PATH_MAX
            && !target.contains(['&', '\t'])
            && !path.contains(['&', '\t', '\n']);
        if usable {
            let description = Some(get("description")).filter(|d| !d.trim().is_empty()).unwrap_or_else(|| target.clone());
            games.push(ScummGame { target, description: description.split_whitespace().collect::<Vec<_>>().join(" "), path });
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            finish(section.take(), &mut games);
            section = Some((name.trim().to_string(), Vec::new()));
        } else if let (Some((_, keys)), Some((key, value))) = (section.as_mut(), line.split_once('=')) {
            keys.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    finish(section, &mut games);
    games
}

/// The URI that starts a ScummVM game.
pub fn scummvm_uri(path: &str, target: &str) -> String {
    format!("psgm:play?titleid=VSCU00001&path={path}&game_id={target}")
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

    #[test]
    fn scummvm_lists_the_games_added_to_it() {
        let ini = "[scummvm]\nversioninfo=2.9.0\n\n[monkey]\ndescription=The Secret of Monkey Island (CD/DOS/English)\npath=ux0:/data/scummvm/games/monkey\ngameid=monkey\nengineid=scumm\n\n\
                   [keymapper]\nkeymap=x\n[sky]\ngameid=sky\npath=ux0:/games/BASS\n[broken]\ngameid=x\n[cloud]\npath=y\ngameid=z\n";
        let games = scummvm_games(ini);
        assert_eq!(
            games,
            [
                ScummGame { target: "monkey".into(), description: "The Secret of Monkey Island (CD/DOS/English)".into(), path: "ux0:/data/scummvm/games/monkey".into() },
                ScummGame { target: "sky".into(), description: "sky".into(), path: "ux0:/games/BASS".into() },
            ]
        );
        assert_eq!(scummvm_uri(&games[0].path, "monkey"), "psgm:play?titleid=VSCU00001&path=ux0:/data/scummvm/games/monkey&game_id=monkey");
        assert_eq!(Emulator::ScummVm.title_id(), Some("VSCU00001"));
        assert_eq!(folders(Emulator::ScummVm), (&[][..], None));
    }
}
