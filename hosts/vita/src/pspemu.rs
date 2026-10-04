//! PSP and PS1 Classics for the installed-title list (cargo feature
//! `installed-apps`).
//!
//! The Vita's own PSP emulator keeps each installed PSP or PS1 title in
//! `ux0:/pspemu/PSP/GAME/<id>/EBOOT.PBP`. A title installed as an official
//! package has a bubble on the home screen, and so a row in the home screen's
//! database (`appdb.rs`) with the folder's id: that gives its name and icon,
//! and `psgm:play?titleid=<id>` starts it in the Vita's emulator, without
//! Adrenaline. Folders with no bubble (homebrew, games installed for
//! Adrenaline) are left out. The `CATEGORY` in the PBP's `PARAM.SFO` tells a
//! PS1 title (`ME`) from a PSP one.

use crate::appdb::{self, report_value};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::string::String;
use std::vec::Vec;

const GAME_ROOT: &str = "ux0:/pspemu/PSP/GAME";
/// `PARAM.SFO` sections are a few KiB; anything larger is not one.
const SFO_MAX: u32 = 256 * 1024;

/// What the last scan found, for the guest's diagnostics screen.
static mut REPORT: String = String::new();

/// A listed PSP or PS1 title.
pub struct Classic {
    pub title_id: String,
    pub title: String,
    /// `"ps1"` or `"psp"`.
    pub platform: &'static str,
}

/// The `PARAM.SFO` inside an `EBOOT.PBP`: the header is `\0PBP`, a version,
/// then eight section offsets, the SFO first and the icon second; the SFO
/// runs from the first offset to the second.
fn pbp_sfo(path: &str) -> Option<Vec<u8>> {
    let mut file = fs::File::open(path).ok()?;
    let mut header = [0u8; 40];
    file.read_exact(&mut header).ok()?;
    if &header[..4] != b"\0PBP" {
        return None;
    }
    let offset = |index: usize| u32::from_le_bytes(header[8 + index * 4..12 + index * 4].try_into().unwrap());
    let (start, end) = (offset(0), offset(1));
    if end <= start || end - start > SFO_MAX {
        return None;
    }
    file.seek(SeekFrom::Start(start as u64)).ok()?;
    let mut sfo = vec![0u8; (end - start) as usize];
    file.read_exact(&mut sfo).ok()?;
    Some(sfo)
}

/// The PSP and PS1 titles to list. `rows` is the home screen's database (None
/// when it could not be read: then nothing is listed, since a title cannot be
/// started without its bubble); `known` holds the ids already listed.
pub unsafe fn scan(rows: Option<&[appdb::Bubble]>, known: &[String]) -> Vec<Classic> {
    let mut ids: Vec<String> = fs::read_dir(GAME_ROOT)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.metadata().is_ok_and(|meta| meta.is_dir()))
                .filter_map(|entry| entry.file_name().to_str().map(String::from))
                .filter(|name| crate::installed::valid_title_id(name))
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    let Some(rows) = rows else {
        REPORT = format!("PSP folders={},PSP source=none", ids.len());
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut without_bubble = Vec::new();
    for id in &ids {
        if known.contains(id) {
            continue;
        }
        let Some(row) = rows.iter().find(|row| &row.title_id == id) else {
            without_bubble.push(id.clone());
            continue;
        };
        let sfo = pbp_sfo(&format!("{GAME_ROOT}/{id}/EBOOT.PBP"));
        let category = sfo.as_deref().and_then(|sfo| crate::installed::sfo_string(sfo, "CATEGORY"));
        let title = if row.title.is_empty() {
            sfo.as_deref()
                .and_then(|sfo| crate::installed::sfo_string(sfo, "TITLE"))
                .map(|title| appdb::one_line(&title))
                .filter(|title| !title.is_empty())
                .unwrap_or_else(|| id.clone())
        } else {
            row.title.clone()
        };
        let platform = if category.as_deref() == Some("ME") { "ps1" } else { "psp" };
        found.push(Classic { title_id: id.clone(), title, platform });
    }
    let ps1 = found.iter().filter(|item| item.platform == "ps1").count();
    REPORT = format!(
        "PSP folders={},PSP bubbles={},PSP ps1={ps1},PSP psp={}",
        ids.len(),
        found.len(),
        found.len() - ps1
    );
    // When folders and bubbles do not meet, the ids say why: the first folder
    // with no bubble, and the first bubble that is none of the listed titles.
    if let Some(first) = without_bubble.first() {
        REPORT.push_str(&format!(",PSP noBubble={},PSP firstNoBubble={}", without_bubble.len(), report_value(first)));
    }
    let listed = |id: &str| known.iter().any(|known| known == id) || found.iter().any(|item| item.title_id == id);
    let others: Vec<&str> = rows
        .iter()
        .map(|row| row.title_id.as_str())
        .filter(|id| !id.starts_with("NPXS") && !listed(id))
        .collect();
    if let Some(first) = others.first() {
        REPORT.push_str(&format!(",PSP otherBubbles={},PSP firstOther={}", others.len(), report_value(first)));
    }
    crate::vita_log(format_args!("[PocketJS pspemu] {}", REPORT));
    found
}

/// What the last scan found among PSP and PS1 titles. Empty before a scan.
pub unsafe fn report() -> String {
    REPORT.clone()
}
