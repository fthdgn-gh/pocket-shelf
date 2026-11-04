//! Identifying titles from the game database tables
//! (`shelf_core::online::gamedb`) on worker threads: the table, the kept
//! CRCs and `known.tsv` are read from the data folder, RetroArch games'
//! files from the memory card; the CRCs read and what each title was found
//! to be are written back for the next run and the next start.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use shelf_core::host::Event;
use shelf_core::online::gamedb::{self, Crc32, Crcs, Known, RomFiles, Table};

use crate::host::{log_line, now_us, DATA};

/// The current run's number. A run stops once it changes: by
/// `Host::identify_stop` ([`stop`]) or by an asked-for run starting.
static RUN: AtomicU32 = AtomicU32::new(0);

/// Held while a run reads and writes `crcs.tsv` and `known.tsv`: a run
/// that is stopping finishes its writes before the next one reads.
static FILES: Mutex<()> = Mutex::new(());

/// Progress goes to the frame at most this often, in microseconds.
const PROGRESS_EVERY_US: u64 = 200_000;

pub fn stop() {
    RUN.fetch_add(1, Ordering::Relaxed);
}

/// File reads on the memory card.
struct CardFiles {
    buffer: Vec<u8>,
    run: u32,
}

impl RomFiles for CardFiles {
    fn size(&mut self, path: &str) -> Option<u64> {
        fs::metadata(path).ok().filter(|m| m.is_file()).map(|m| m.len())
    }

    fn read_at(&mut self, path: &str, offset: u64, len: usize) -> Option<Vec<u8>> {
        let mut file = fs::File::open(path).ok()?;
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut bytes = vec![0; len];
        file.read_exact(&mut bytes).ok()?;
        Some(bytes)
    }

    fn crc(&mut self, path: &str, skip: u64) -> Option<u32> {
        let mut file = fs::File::open(path).ok()?;
        file.seek(SeekFrom::Start(skip)).ok()?;
        let mut crc = Crc32::default();
        loop {
            match file.read(&mut self.buffer) {
                Ok(0) => return Some(crc.finish()),
                Ok(read) => crc.update(&self.buffer[..read]),
                Err(_) => return None,
            }
            if RUN.load(Ordering::Relaxed) != self.run {
                return None;
            }
        }
    }
}

/// The table's build time from its first line, without reading the rest.
fn table_built(path: &str) -> Option<u64> {
    let mut head = [0u8; 64];
    let read = fs::File::open(path).ok()?.read(&mut head).ok()?;
    let text = String::from_utf8_lossy(&head[..read]);
    gamedb::table_built(text.split('\n').next()?)
}

fn read_text(path: &str) -> Option<String> {
    fs::read(path).ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// Write through a `.part` file, so a reader never sees half of it.
fn write_text(path: &str, text: &str) -> bool {
    let part = format!("{path}.part");
    fs::write(&part, text).is_ok() && fs::rename(&part, path).is_ok()
}

/// `known.tsv`, for the title list (read on the scan thread).
pub fn known() -> Known {
    read_text(&format!("{DATA}/{}", gamedb::KNOWN_PATH)).map(|text| Known::parse(&text)).unwrap_or_default()
}

/// The titles a run asked for (`Host::identify`): every title of the
/// database. Runs on its own thread.
pub fn run(database: String, newer_than_ms: u64, sender: Sender<Event>) {
    let run = RUN.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    if !identify(&database, newer_than_ms, &sender, false, run) {
        let _ = sender.send(Event::Identified(database, None));
    }
}

/// At each start, after the title list: the titles not looked up yet in
/// each database whose table is on the card. No network; a run the user
/// starts stops this one.
pub fn identify_new(sender: &Sender<Event>) {
    let run = RUN.load(Ordering::Relaxed);
    let known = known();
    for database in unsafe { crate::platform::installed::databases() } {
        if RUN.load(Ordering::Relaxed) != run {
            return;
        }
        let Some(built) = table_built(&format!("{DATA}/{}", gamedb::table_path(&database))) else { continue };
        let roms = unsafe { crate::platform::installed::database_roms(&database) };
        if roms.iter().any(|rom| known.wants(&rom.id, &database, built)) {
            identify(&database, 0, sender, true, run);
        }
    }
}

/// Identify a database's titles (with `only_new`, those `known.tsv` does
/// not cover for this table) and answer with `Event::Identified`; false
/// when there is no table built after `newer_than_ms`. A stopped run
/// answers nobody (the run that asked has halted) and keeps the CRCs it
/// read, not what it found.
fn identify(database: &str, newer_than_ms: u64, sender: &Sender<Event>, only_new: bool, run: u32) -> bool {
    let started = now_us();
    let path = format!("{DATA}/{}", gamedb::table_path(database));
    if table_built(&path).is_none_or(|built| built < newer_than_ms) {
        log_line(&format!("gamedb {database}: no table built after {newer_than_ms}"));
        return false;
    }
    let Some(table) = read_text(&path).and_then(|text| Table::parse(&text)) else {
        log_line(&format!("gamedb {database}: the table does not read"));
        return false;
    };
    let read = now_us();
    let _files = FILES.lock();
    let known_path = format!("{DATA}/{}", gamedb::KNOWN_PATH);
    let mut known = read_text(&known_path).map(|text| Known::parse(&text)).unwrap_or_default();
    let mut roms = unsafe { crate::platform::installed::database_roms(database) };
    if only_new {
        roms.retain(|rom| known.wants(&rom.id, database, table.built_ms));
    }
    let crcs_path = format!("{DATA}/{}", gamedb::CRCS_PATH);
    let mut crcs = read_text(&crcs_path).map(|text| Crcs::parse(&text)).unwrap_or_default();
    let mut files = CardFiles { buffer: vec![0; 64 * 1024], run };
    let mut last = 0u64;
    let mut progress = |done: usize, total: usize| {
        let now = now_us();
        if now - last >= PROGRESS_EVERY_US || done == total {
            last = now;
            let _ = sender.send(Event::Identifying { database: database.to_string(), done: done as u32, total: total as u32 });
        }
        RUN.load(Ordering::Relaxed) == run
    };
    let (found, counts) = gamedb::identify(&table, &roms, &mut crcs, &mut files, &mut progress);
    if crcs.changed && !write_text(&crcs_path, &crcs.render()) {
        log_line("gamedb: crcs.tsv not written");
    }
    if !counts.stopped {
        known.record(database, table.built_ms, &roms, &found);
        if !write_text(&known_path, &known.render()) {
            log_line("gamedb: known.tsv not written");
        }
    }
    log_line(&format!(
        "gamedb {database}: {} of {} titles identified ({} by label, {} by serial, {} by set name, {} by kept CRC, {} by CRC read; {} files read{}{}), table of {} read in {} ms, all in {} ms",
        found.len(),
        roms.len(),
        counts.label,
        counts.serial,
        counts.rom,
        counts.kept,
        counts.read,
        counts.files,
        if only_new { ", new titles only" } else { "" },
        if counts.stopped { ", stopped" } else { "" },
        table.len(),
        (read - started) / 1000,
        (now_us() - started) / 1000
    ));
    if !counts.stopped {
        let _ = sender.send(Event::Identified(database.to_string(), Some(found)));
    }
    true
}
