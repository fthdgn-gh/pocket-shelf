//! Matching titles to Wikidata's games (`shelf_core::online::wikidata`) on
//! worker threads: the table, `labels.tsv`, `found.tsv` and libretro's
//! `known.tsv` (the database names it gave) are read from the data folder;
//! what each title was found to be is written back for the next start.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use shelf_core::host::{Event, TitleInfo};
use shelf_core::online::wikidata::{self, Found, Index, Matches, Names, Title};

use crate::host::{log_line, now_us, DATA};

/// Held while a run reads and writes `found.tsv`.
static FILES: Mutex<()> = Mutex::new(());

fn read_text(path: &str) -> Option<String> {
    fs::read(format!("{DATA}/{path}")).ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// Write through a `.part` file, so a reader never sees half of it.
fn write_text(path: &str, text: &str) -> bool {
    let path = format!("{DATA}/{path}");
    let part = format!("{path}.part");
    fs::write(&part, text).is_ok() && fs::rename(&part, &path).is_ok()
}

/// The table's build time from its first line, without reading the rest.
fn table_built(file: &str) -> Option<u64> {
    let mut head = [0u8; 128];
    let read = fs::File::open(format!("{DATA}/{}", wikidata::table_path(file))).ok()?.read(&mut head).ok()?;
    let text = String::from_utf8_lossy(&head[..read]);
    wikidata::table_built(text.split('\n').next()?)
}

fn labels() -> Option<HashMap<String, Names>> {
    read_text(wikidata::LABELS_PATH).map(|text| wikidata::shared_labels(&text)).filter(|labels| !labels.is_empty())
}

/// Each title's game from `found.tsv`, for the title list (on the scan
/// thread).
pub fn apply(infos: &mut [TitleInfo]) {
    let started = now_us();
    let Some(text) = read_text(wikidata::FOUND_PATH) else { return };
    let matches = Matches::parse(&text);
    matches.apply(infos, &labels().unwrap_or_default());
    log_line(&format!("wikidata: {} titles looked up, read in {} ms", matches.len(), (now_us() - started) / 1000));
}

/// The titles a run asked for (`Host::match_wikidata`): every title of the
/// table. Runs on its own thread.
pub fn run(file: String, newer_than_ms: u64, sender: Sender<Event>) {
    let found = match_file(&file, newer_than_ms, false);
    let _ = sender.send(Event::WikidataMatched(file, found));
}

/// At each start, after libretro's new titles: the titles not looked up yet
/// in each table on the card. No network.
pub fn match_new(sender: &Sender<Event>) {
    let found = read_text(wikidata::FOUND_PATH).map(|text| Matches::parse(&text)).unwrap_or_default();
    for file in unsafe { crate::platform::installed::wikidata_files() } {
        let Some(built) = table_built(&file) else { continue };
        let titles = unsafe { crate::platform::installed::wikidata_titles(&file) };
        if titles.iter().any(|(id, _)| found.wants(id, &file, built)) {
            if let Some(matched) = match_file(&file, 0, true) {
                let _ = sender.send(Event::WikidataMatched(file, Some(matched)));
            }
        }
    }
}

/// Match a table's titles (with `only_new`, those `found.tsv` does not
/// cover for this table) by the database name libretro gave them, else
/// their own; None when the table is missing or built before
/// `newer_than_ms`, or the labels are missing.
fn match_file(file: &str, newer_than_ms: u64, only_new: bool) -> Option<Vec<(String, Arc<Found>)>> {
    let started = now_us();
    if table_built(file).is_none_or(|built| built < newer_than_ms) {
        log_line(&format!("wikidata {file}: no table built after {newer_than_ms}"));
        return None;
    }
    let Some(labels) = labels() else {
        log_line("wikidata: no labels");
        return None;
    };
    let Some(index) = read_text(&wikidata::table_path(file)).and_then(|text| Index::parse(&text)) else {
        log_line(&format!("wikidata {file}: the table does not read"));
        return None;
    };
    let read = now_us();
    let _files = FILES.lock();
    let mut found = read_text(wikidata::FOUND_PATH).map(|text| Matches::parse(&text)).unwrap_or_default();
    let known = crate::platform::gamedb::known();
    let mut titles: Vec<Title> = unsafe { crate::platform::installed::wikidata_titles(file) }
        .into_iter()
        .map(|(id, label)| {
            let details = known.get(&id);
            let names = details.map(|d| d.name.clone()).into_iter().chain([label]).collect();
            Title { year: details.map_or(0, |d| d.year), id, names }
        })
        .collect();
    if only_new {
        titles.retain(|title| found.wants(&title.id, file, index.built_ms));
    }
    let matched = wikidata::match_titles(&index, &titles);
    found.record(file, index.built_ms, &titles, &matched);
    if !write_text(wikidata::FOUND_PATH, &found.render()) {
        log_line("wikidata: found.tsv not written");
    }
    log_line(&format!(
        "wikidata {file}: {} of {} titles matched{}, table of {} read in {} ms, all in {} ms",
        matched.len(),
        titles.len(),
        if only_new { " (new titles only)" } else { "" },
        index.games.len(),
        (read - started) / 1000,
        (now_us() - started) / 1000
    ));
    Some(matched.into_iter().map(|(id, game)| (id, Arc::new(Found::new(game, &labels)))).collect())
}
