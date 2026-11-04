//! Naming a console's RetroArch games from its game database table
//! (`shelf_core::online::gamedb`) on a worker thread: the table and the kept
//! CRCs are read from the data folder, the games' files from the memory
//! card, and the CRCs read are kept for the next run.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;

use shelf_core::host::Event;
use shelf_core::online::gamedb::{self, Crc32, Crcs, RomFiles, Table};

use crate::host::{log_line, now_us, DATA};

/// The current run's number: a run stops once it changes, by
/// `Host::identify_stop` ([`stop`]) or by the next run starting, so two runs
/// never write `crcs.tsv` at once.
static RUN: AtomicU32 = AtomicU32::new(0);

pub fn stop() {
    RUN.fetch_add(1, Ordering::Relaxed);
}

/// Progress goes to the frame at most this often, in microseconds.
const PROGRESS_EVERY_US: u64 = 200_000;

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

/// Name a database's games and answer with `Event::Identified`; runs on its
/// own thread.
pub fn run(database: String, newer_than_ms: u64, sender: Sender<Event>) {
    let run = RUN.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    let started = now_us();
    let path = format!("{DATA}/{}", gamedb::table_path(&database));
    if table_built(&path).is_none_or(|built| built < newer_than_ms) {
        log_line(&format!("gamedb {database}: no table built after {newer_than_ms}"));
        let _ = sender.send(Event::Identified(database, None));
        return;
    }
    let Some(table) = fs::read(&path).ok().and_then(|bytes| Table::parse(&String::from_utf8_lossy(&bytes))) else {
        log_line(&format!("gamedb {database}: the table does not read"));
        let _ = sender.send(Event::Identified(database, None));
        return;
    };
    let read = now_us();
    let roms = unsafe { crate::platform::installed::retro_roms(&database) };
    let crcs_path = format!("{DATA}/{}", gamedb::CRCS_PATH);
    let mut crcs = fs::read(&crcs_path).map(|bytes| Crcs::parse(&String::from_utf8_lossy(&bytes))).unwrap_or_default();
    let mut files = CardFiles { buffer: vec![0; 64 * 1024], run };
    let mut last = 0u64;
    let mut progress = |done: usize, total: usize| {
        let now = now_us();
        if now - last >= PROGRESS_EVERY_US || done == total {
            last = now;
            let _ = sender.send(Event::Identifying { database: database.clone(), done: done as u32, total: total as u32 });
        }
        RUN.load(Ordering::Relaxed) == run
    };
    let (names, counts) = gamedb::identify(&table, &roms, &mut crcs, &mut files, &mut progress);
    if crcs.changed {
        let part = format!("{crcs_path}.part");
        let written = fs::write(&part, crcs.render()).is_ok() && fs::rename(&part, &crcs_path).is_ok();
        if !written {
            log_line("gamedb: crcs.tsv not written");
        }
    }
    log_line(&format!(
        "gamedb {database}: {} of {} games named ({} by label, {} by set name, {} by kept CRC, {} by CRC read; {} files read{}), table of {} read in {} ms, all in {} ms",
        names.len(),
        roms.len(),
        counts.label,
        counts.rom,
        counts.kept,
        counts.read,
        counts.files,
        if counts.stopped { ", stopped" } else { "" },
        table.len(),
        (read - started) / 1000,
        (now_us() - started) / 1000
    ));
    // A stopped run answers nobody: the run that asked has halted, and the
    // next one names the games again (from the kept CRCs).
    if !counts.stopped {
        let _ = sender.send(Event::Identified(database, Some(names)));
    }
}
