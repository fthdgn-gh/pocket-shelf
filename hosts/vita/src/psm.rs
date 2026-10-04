//! PlayStation Mobile titles for the installed-title list (cargo feature
//! `installed-apps`).
//!
//! A PSM title is a folder in `ux0:/psm` named after its id: `NPNA`, `NPOA`,
//! `NPPA` or `NPQA` and five digits (the region). It has no `param.sfo`. The
//! name comes from the home screen's database, `ur0:shell/db/app.db`, which
//! has a row for every bubble; a title with no row has no bubble, cannot be
//! started by URI, and is left out. The database is read with the system's
//! SQLite module. When it cannot be read, every folder is listed, named from
//! the home screen's copy of its `param.sfo` when there is one, else by id.
//! RetroFlow lists and starts PSM titles the same way.

use core::ffi::{c_char, c_int, c_void};
use std::ffi::CStr;
use std::fs;
use std::sync::Mutex;
use std::string::String;
use std::vec::Vec;

const PSM_ROOT: &str = "ux0:/psm";
const APP_DB: &CStr = c"ur0:shell/db/app.db";

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_OPEN_READONLY: c_int = 0x1;

// The system's SQLite (SceSqlite); vitasdk-sys links the stub with the
// `SceSqlite_stub` feature but has no bindings for these.
extern "C" {
    fn sqlite3_open_v2(filename: *const c_char, db: *mut *mut c_void, flags: c_int, vfs: *const c_char) -> c_int;
    fn sqlite3_prepare_v2(
        db: *mut c_void,
        sql: *const c_char,
        bytes: c_int,
        statement: *mut *mut c_void,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_step(statement: *mut c_void) -> c_int;
    fn sqlite3_column_text(statement: *mut c_void, column: c_int) -> *const u8;
    fn sqlite3_finalize(statement: *mut c_void) -> c_int;
    fn sqlite3_close(db: *mut c_void) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

// The system's SQLite has no allocator of its own: without these, opening a
// database fails with SQLITE_NOMEM (seen in Vita3K). They hand it the C heap.
unsafe extern "C" fn sqlite_malloc(size: c_int) -> *mut c_void {
    malloc(size.max(0) as usize)
}
unsafe extern "C" fn sqlite_realloc(pointer: *mut c_void, size: c_int) -> *mut c_void {
    realloc(pointer, size.max(0) as usize)
}
unsafe extern "C" fn sqlite_free(pointer: *mut c_void) {
    free(pointer)
}
static mut SQLITE_READY: bool = false;

/// Load the system's SQLite module and give it the allocator, once.
unsafe fn sqlite_ready() -> bool {
    if SQLITE_READY {
        return true;
    }
    let loaded = vitasdk_sys::sceSysmoduleLoadModule(vitasdk_sys::SCE_SYSMODULE_SQLITE as _);
    if loaded < 0 && vitasdk_sys::sceSysmoduleIsLoaded(vitasdk_sys::SCE_SYSMODULE_SQLITE as _) != 0 {
        crate::vita_log(format_args!("[PocketJS psm] SQLite module: {:#x}", loaded as u32));
        return false;
    }
    // SQLite keeps the pointer, so the table lives for the process.
    static mut METHODS: vitasdk_sys::SceSqliteMallocMethods = vitasdk_sys::SceSqliteMallocMethods {
        xMalloc: Some(sqlite_malloc),
        xRealloc: Some(sqlite_realloc),
        xFree: Some(sqlite_free),
    };
    let code = vitasdk_sys::sceSqliteConfigMallocMethods(core::ptr::addr_of_mut!(METHODS));
    if code < 0 {
        crate::vita_log(format_args!("[PocketJS psm] SQLite allocator: {:#x}", code as u32));
        return false;
    }
    SQLITE_READY = true;
    true
}

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

/// One line per item, whitespace collapsed (bubble names can hold line breaks).
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A bubble's row in the home screen's database.
struct Bubble {
    title_id: String,
    title: String,
    /// Where the home screen reads the bubble's icon (`iconPath`).
    icon_path: String,
}

/// One user of the database at a time: the scan runs on the main thread, an
/// icon lookup on a decoding thread.
static DB_LOCK: Mutex<()> = Mutex::new(());

/// The rows of the home screen's database for ids that start with `NP`.
/// None when the database cannot be opened or read.
unsafe fn bubbles() -> Option<Vec<Bubble>> {
    let _guard = DB_LOCK.lock().ok()?;
    if !sqlite_ready() {
        return None;
    }
    let mut db: *mut c_void = core::ptr::null_mut();
    let code = sqlite3_open_v2(APP_DB.as_ptr(), &mut db, SQLITE_OPEN_READONLY, core::ptr::null());
    if code != SQLITE_OK {
        crate::vita_log(format_args!("[PocketJS psm] open app.db: {code}"));
        if !db.is_null() {
            sqlite3_close(db);
        }
        return None;
    }
    let sql = c"SELECT titleId, title, iconPath FROM tbl_appinfo_icon WHERE titleId LIKE 'NP%'";
    let mut statement: *mut c_void = core::ptr::null_mut();
    let code = sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut statement, core::ptr::null_mut());
    if code != SQLITE_OK {
        crate::vita_log(format_args!("[PocketJS psm] query app.db: {code}"));
        sqlite3_close(db);
        return None;
    }
    let text = |column: c_int| {
        let pointer = sqlite3_column_text(statement, column);
        if pointer.is_null() {
            String::new()
        } else {
            CStr::from_ptr(pointer as *const c_char).to_string_lossy().into_owned()
        }
    };
    let mut rows = Vec::new();
    while sqlite3_step(statement) == SQLITE_ROW {
        rows.push(Bubble {
            title_id: text(0),
            title: one_line(&text(1)),
            icon_path: text(2).trim().to_string(),
        });
    }
    sqlite3_finalize(statement);
    sqlite3_close(db);
    Some(rows)
}

/// Icon paths per PSM title id, from the last read of the database. Filled by
/// a scan, or by the first icon lookup on a start that read the list file.
static ICON_PATHS: Mutex<Option<Vec<(String, String)>>> = Mutex::new(None);

fn keep_icon_paths(rows: &[Bubble]) {
    let paths = rows
        .iter()
        .filter(|row| is_psm_id(&row.title_id) && !row.icon_path.is_empty())
        .map(|row| (row.title_id.clone(), row.icon_path.clone()))
        .collect();
    if let Ok(mut kept) = ICON_PATHS.lock() {
        *kept = Some(paths);
    }
}

/// The file the home screen shows as this PSM title's icon, from its
/// database: on hardware `ur0:appmeta/<id>/icon0.dds`, a 128x128 DXT1 DDS
/// (`dds.rs`). Callable from any thread.
pub unsafe fn icon_path(title_id: &str) -> Option<String> {
    if !is_psm_id(title_id) {
        return None;
    }
    let loaded = ICON_PATHS.lock().ok()?.is_some();
    if !loaded {
        keep_icon_paths(&bubbles().unwrap_or_default());
    }
    let kept = ICON_PATHS.lock().ok()?;
    kept.as_ref()?
        .iter()
        .find(|(id, _)| id == title_id)
        .map(|(_, path)| path.clone())
}

/// Names and sizes in bytes of a folder's entries, sorted; a folder's name
/// ends in `/` and has no size.
fn folder_files(path: &str) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name().to_str()?.to_string();
                    let meta = entry.metadata().ok()?;
                    Some(if meta.is_dir() {
                        (format!("{name}/"), String::new())
                    } else {
                        (name, meta.len().to_string())
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// The name in the home screen's copy of the title's `param.sfo`, if any.
fn appmeta_title(title_id: &str) -> Option<String> {
    let sfo = fs::read(format!("ur0:appmeta/{title_id}/param.sfo")).ok()?;
    crate::installed::sfo_string(&sfo, "TITLE")
        .map(|title| one_line(&title))
        .filter(|title| !title.is_empty())
}

/// The PSM titles to list, as `(title id, name)`.
pub unsafe fn scan() -> Vec<(String, String)> {
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
        REPORT = String::from("folders=0");
        return Vec::new();
    }
    let rows = bubbles();
    let found: Vec<(String, String)> = match &rows {
        Some(rows) => {
            keep_icon_paths(rows);
            let listed: Vec<(String, String)> = ids
                .iter()
                .filter_map(|id| {
                    let row = rows.iter().find(|row| &row.title_id == id)?;
                    Some((id.clone(), if row.title.is_empty() { id.clone() } else { row.title.clone() }))
                })
                .collect();
            REPORT = format!("folders={},source=app.db,bubbles={}", ids.len(), listed.len());
            listed
        }
        None => {
            let listed: Vec<(String, String)> = ids
                .iter()
                .map(|id| (id.clone(), appmeta_title(id).unwrap_or_else(|| id.clone())))
                .collect();
            let named = listed.iter().filter(|(id, title)| id != title).count();
            REPORT = format!("folders={},source=folders,named={named}", ids.len());
            listed
        }
    };
    // Where the icons are: `icon0.png` or `icon0.dds` in ur0:appmeta, and the
    // file the database names. The files of the first title's ur0:appmeta
    // folder that has any are listed, one line each.
    let appmeta = found
        .iter()
        .filter(|(id, _)| {
            ["png", "dds"].iter().any(|kind| fs::metadata(format!("ur0:appmeta/{id}/icon0.{kind}")).is_ok())
        })
        .count();
    let paths: Vec<String> = found.iter().filter_map(|(id, _)| icon_path(id)).collect();
    let files = paths.iter().filter(|path| fs::metadata(path.as_str()).is_ok_and(|meta| meta.is_file())).count();
    REPORT.push_str(&format!(",appmetaIcons={appmeta},dbIconFiles={files}/{}", paths.len()));
    if let Some(first) = paths.first() {
        // The part after the folder of ids, so the line stays short.
        let tail = first.rsplit('/').next().unwrap_or(first);
        REPORT.push_str(&format!(",iconPathEnd={}", tail.replace([',', '='], "_")));
    }
    let listing = found
        .iter()
        .map(|(id, _)| folder_files(&format!("ur0:appmeta/{id}")))
        .find(|files| !files.is_empty())
        .unwrap_or_default();
    for (name, size) in listing.into_iter().take(16) {
        REPORT.push_str(&format!(",file={} {size}", name.replace([',', '='], "_")));
    }
    crate::vita_log(format_args!("[PocketJS psm] {}", REPORT));
    found
}

/// What the last scan found: `folders=N`, where the names came from, and
/// where the icons are. Empty before a scan (the list file was read instead).
pub unsafe fn report() -> String {
    REPORT.clone()
}
