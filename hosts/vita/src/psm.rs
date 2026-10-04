//! PlayStation Mobile titles for the installed-title list (cargo feature
//! `installed-apps`).
//!
//! A PSM title is a folder in `ux0:/psm` named after its id: `NPNA`, `NPOA`,
//! `NPPA` or `NPQA` and five digits (the region). It has no `param.sfo`. The
//! name comes from the home screen's database (`appdb.rs`); a title with no
//! row there has no bubble, cannot be started by URI, and is left out. When
//! the database cannot be read, every folder is listed, named from the home
//! screen's copy of its `param.sfo` when there is one, else by id. RetroFlow
//! lists and starts PSM titles the same way. The home screen keeps the icon
//! as `ur0:appmeta/<id>/icon0.dds`, a 128x128 DXT1 DDS (`dds.rs`).

use crate::appdb::{self, one_line, report_value};
use std::fs;
use std::string::String;
use std::vec::Vec;

const PSM_ROOT: &str = "ux0:/psm";

/// What the last scan found, for the guest's diagnostics screen.
static mut REPORT: String = String::new();

/// `NPNA00001`: a PSM title id.
pub fn is_psm_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 9
        && bytes.starts_with(b"NP")
        && matches!(bytes[2], b'N' | b'O' | b'P' | b'Q')
        && bytes[3] == b'A'
        && bytes[4..].iter().all(u8::is_ascii_digit)
}

/// The name in the home screen's copy of the title's `param.sfo`, if any.
fn appmeta_title(title_id: &str) -> Option<String> {
    let sfo = fs::read(format!("ur0:appmeta/{title_id}/param.sfo")).ok()?;
    crate::installed::sfo_string(&sfo, "TITLE")
        .map(|title| one_line(&title))
        .filter(|title| !title.is_empty())
}

/// The PSM titles to list, as `(title id, name)`. `rows` is the home screen's
/// database, None when it could not be read.
pub unsafe fn scan(rows: Option<&[appdb::Bubble]>) -> Vec<(String, String)> {
    let mut ids: Vec<String> = fs::read_dir(PSM_ROOT)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(String::from))
                .filter(|name| is_psm_id(name))
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    if ids.is_empty() {
        REPORT = String::from("PSM folders=0");
        return Vec::new();
    }
    let found: Vec<(String, String)> = match rows {
        Some(rows) => {
            let listed: Vec<(String, String)> = ids
                .iter()
                .filter_map(|id| {
                    let row = rows.iter().find(|row| &row.title_id == id)?;
                    Some((id.clone(), if row.title.is_empty() { id.clone() } else { row.title.clone() }))
                })
                .collect();
            REPORT = format!("PSM folders={},PSM source=app.db,PSM bubbles={}", ids.len(), listed.len());
            listed
        }
        None => {
            let listed: Vec<(String, String)> = ids
                .iter()
                .map(|id| (id.clone(), appmeta_title(id).unwrap_or_else(|| id.clone())))
                .collect();
            let named = listed.iter().filter(|(id, title)| id != title).count();
            REPORT = format!("PSM folders={},PSM source=folders,PSM named={named}", ids.len());
            listed
        }
    };
    // Where the icons are: `icon0.png` or `icon0.dds` in ur0:appmeta, and the
    // file the database names.
    let appmeta = found
        .iter()
        .filter(|(id, _)| {
            ["png", "dds"].iter().any(|kind| fs::metadata(format!("ur0:appmeta/{id}/icon0.{kind}")).is_ok())
        })
        .count();
    let paths: Vec<String> = found.iter().filter_map(|(id, _)| appdb::icon_path(id)).collect();
    let files = paths.iter().filter(|path| fs::metadata(path.as_str()).is_ok_and(|meta| meta.is_file())).count();
    REPORT.push_str(&format!(",PSM appmetaIcons={appmeta},PSM dbIconFiles={files}/{}", paths.len()));
    if let Some(first) = paths.first() {
        let tail = first.rsplit('/').next().unwrap_or(first);
        REPORT.push_str(&format!(",PSM iconPathEnd={}", report_value(tail)));
    }
    crate::vita_log(format_args!("[PocketJS psm] {}", REPORT));
    found
}

/// What the last scan found among PSM titles. Empty before a scan (the list
/// file was read instead).
pub unsafe fn report() -> String {
    REPORT.clone()
}
