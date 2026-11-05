//! Folder categories' games (cargo feature `installed-apps`): the files
//! under each folder the guest names (`Host::rom_folders`), with the console
//! `shelf_core::library::roms` finds for each, and the RetroArch and
//! Emu4Vita++ cores that start it.
//!
//! RetroArch gets the console's recommended core when it is installed,
//! else a core whose info file names the console, else one Emu4Vita++ runs
//! the console with; Emu4Vita++ gets its own preferred core. A standalone
//! emulator installed for the console (DaedalusX64, DSVita, NooDS, Flycast)
//! starts it too. A file no installed app can start, or one a playlist
//! lists already, is left out.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::string::String;
use std::sync::Mutex;
use std::vec::Vec;

use shelf_core::library::roms::{self, Files};

use super::{emu4vita, retroarch};

/// The folders, set by the guest before a scan.
static FOLDERS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn set(folders: Vec<String>) {
    if let Ok(mut set) = FOLDERS.lock() {
        *set = folders;
    }
}

pub fn get() -> Vec<String> {
    FOLDERS.lock().map(|f| f.clone()).unwrap_or_default()
}

/// The memory card through `std::fs`.
struct Card;

impl Files for Card {
    fn list(&mut self, dir: &str) -> Option<Vec<(String, bool)>> {
        let entries = fs::read_dir(dir).ok()?;
        Some(
            entries
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name().to_str()?.to_string();
                    let folder = entry.file_type().ok()?.is_dir();
                    Some((name, folder))
                })
                .collect(),
        )
    }

    fn size(&mut self, path: &str) -> Option<u64> {
        fs::metadata(path).ok().filter(|m| m.is_file()).map(|m| m.len())
    }

    fn read_at(&mut self, path: &str, offset: u64, len: usize) -> Option<Vec<u8>> {
        let mut file = fs::File::open(path).ok()?;
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut out = Vec::with_capacity(len);
        file.take(len as u64).read_to_end(&mut out).ok()?;
        Some(out)
    }
}

/// A game found in a folder, as `installed.rs` lists it.
pub struct Game {
    pub id: String,
    pub title: String,
    pub console: String,
    /// RetroArch's core file; empty when RetroArch cannot start it.
    pub core: String,
    /// Emu4Vita++'s core; empty when it cannot start it.
    pub emu: String,
    pub path: String,
}

/// RetroArch's core for a console: the recommended one when installed,
/// else one whose info file names the console, else one Emu4Vita++ runs the
/// console with.
fn retroarch_core(cores: &[retroarch::CoreInfo], console: &str) -> Option<String> {
    use shelf_core::library::{recommended::recommended, settings::Emulator};
    let installed = |file: &str| cores.iter().any(|c| c.file == file);
    recommended(Emulator::RetroArch, console)
        .filter(|file| installed(file))
        .map(String::from)
        .or_else(|| cores.iter().find(|c| c.databases.iter().any(|d| d == console)).map(|c| c.file.clone()))
        .or_else(|| emu4vita::console_cores(console).iter().map(|name| format!("{name}_libretro.self")).find(|file| installed(file)))
}

/// A folder to read: a folder category's, or a standalone emulator's,
/// which keeps the games of its own consoles alone.
pub struct Root {
    pub path: String,
    pub consoles: Option<&'static [&'static str]>,
}

impl Root {
    pub fn any(path: String) -> Root {
        Root { path, consoles: None }
    }
}

/// The folders the installed standalone emulators keep games in
/// (`shelf_core::library::standalone`), those that exist, each once.
pub fn emulator_roots(installed: u8) -> Vec<Root> {
    use shelf_core::library::{settings::STANDALONE, standalone};
    let mut roots: Vec<Root> = Vec::new();
    for (app, _, consoles) in STANDALONE.iter().filter(|(app, _, _)| installed & app.bit() != 0) {
        let (fixed, config) = standalone::folders(*app);
        let mut paths: Vec<String> = fixed.iter().map(|p| String::from(*p)).collect();
        if let Some(config) = config {
            if let Ok(bytes) = fs::read(config.file()) {
                paths.extend(standalone::config_folders(config, &bytes));
            }
        }
        for path in paths {
            let key = roms::path_key(&path);
            let here = fs::metadata(&path).is_ok_and(|m| m.is_dir());
            if here && !roots.iter().any(|r| roms::path_key(&r.path) == key) {
                roots.push(Root { path, consoles: Some(consoles) });
            }
        }
    }
    roots
}

/// Every game in the folders that an installed app can start, leaving out
/// files whose path key is in `known` (a playlist lists them) and ids in
/// `ids`. `step` is called once per folder read. Returns the games and a
/// report for the diagnostics screen, its entries named after `label`.
pub fn games(
    label: &str,
    folders: &[Root],
    known: &HashSet<String>,
    ids: &mut HashSet<String>,
    emu: Option<&emu4vita::Install>,
    standalone: u8,
    step: &mut dyn FnMut(),
) -> (Vec<Game>, String) {
    let roots = retroarch::Roots::vita();
    let retroarch = roots.installed();
    let cores = if retroarch { retroarch::installed_cores(&roots) } else { Vec::new() };
    // Each console's cores, worked out once.
    let mut by_console: HashMap<&'static str, (String, String)> = HashMap::new();
    let (mut found, mut unknown, mut no_app, mut listed) = (Vec::new(), 0u32, 0u32, 0u32);
    let mut unknown_names: Vec<String> = Vec::new();
    let mut read = 0u32;
    let mut other_console = 0u32;
    for folder in folders {
        // A file a playlist lists is not opened.
        // An emulator's folder for one console is that console's.
        let console = folder.consoles.filter(|c| c.len() == 1).map(|c| c[0]);
        let walk = roms::walk(&mut Card, &folder.path, console, &mut |path| {
            let known = known.contains(&roms::path_key(path));
            listed += u32::from(known);
            known
        }, step);
        read += walk.folders;
        unknown += walk.unknown;
        unknown_names.extend(walk.unknown_names);
        for rom in walk.roms {
            // An emulator's folder keeps its own consoles' games.
            if folder.consoles.is_some_and(|consoles| !consoles.contains(&rom.console)) {
                other_console += 1;
                continue;
            }
            let (core, emu_core) = by_console
                .entry(rom.console)
                .or_insert_with(|| {
                    let core = if retroarch { retroarch_core(&cores, rom.console) } else { None };
                    let emu_core = emu.and_then(|emu| emu.core_for(None, rom.console));
                    (core.unwrap_or_default(), emu_core.unwrap_or_default())
                })
                .clone();
            let core = Some(core)
                .filter(|c| !c.is_empty() && !rom.path.contains('&') && retroarch::launch_uri(c, &rom.path).len() <= retroarch::URI_MAX)
                .unwrap_or_default();
            let emu_core = Some(emu_core).filter(|c| !c.is_empty() && emu4vita::launch_uri(c, &rom.path).len() <= emu4vita::URI_MAX).unwrap_or_default();
            // A standalone emulator installed for the console starts it
            // too (`installed::standalone_for`).
            let other = super::installed::standalone_for(rom.console, standalone, &rom.path) != 0;
            if (core.is_empty() && emu_core.is_empty() && !other) || rom.path.contains(['\t', '\n', '\r']) {
                no_app += 1;
                continue;
            }
            let id = retroarch::game_id(&rom.path);
            if !ids.insert(id.clone()) {
                continue;
            }
            found.push(Game { id, title: rom.title, console: String::from(rom.console), core, emu: emu_core, path: rom.path });
        }
    }
    let mut report = format!(
        "{label} roots={},{label} read={read},{label} titles={},{label} inPlaylists={listed},{label} noApp={no_app},{label} unknown={unknown}",
        folders.len(),
        found.len()
    );
    if other_console > 0 {
        report.push_str(&format!(",{label} otherConsole={other_console}"));
    }
    for name in unknown_names.iter().take(6) {
        report.push_str(&format!(",{label} unknownFile={}", name.replace([',', '='], "_")));
    }
    (found, report)
}
