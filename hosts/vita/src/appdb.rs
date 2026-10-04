//! The home screen's database, `ur0:shell/db/app.db` (cargo feature
//! `installed-apps`). It has a row for every bubble in `tbl_appinfo_icon`:
//! the title id, the name under the bubble, and the icon file. Titles that do
//! not live in `ux0:/app` (PSM, PSP and PS1 Classics) take their name and icon
//! from here, and a title with no row has no bubble and cannot be started by
//! URI. Read with the system's SQLite module (SceSqlite).

use core::ffi::{c_char, c_int, c_void};
use std::ffi::CStr;
use std::fs;
use std::string::String;
use std::sync::Mutex;
use std::vec::Vec;

const APP_DB: &CStr = c"ur0:shell/db/app.db";

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_OPEN_READONLY: c_int = 0x1;

// The system's SQLite; vitasdk-sys links the stub with the `SceSqlite_stub`
// feature but has no bindings for these.
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

/// Load the system's SQLite module and give it the allocator, once. Called
/// with `DB_LOCK` held.
unsafe fn sqlite_ready() -> bool {
    if SQLITE_READY {
        return true;
    }
    let loaded = vitasdk_sys::sceSysmoduleLoadModule(vitasdk_sys::SCE_SYSMODULE_SQLITE as _);
    if loaded < 0 && vitasdk_sys::sceSysmoduleIsLoaded(vitasdk_sys::SCE_SYSMODULE_SQLITE as _) != 0 {
        crate::vita_log(format_args!("[PocketJS appdb] SQLite module: {:#x}", loaded as u32));
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
        crate::vita_log(format_args!("[PocketJS appdb] SQLite allocator: {:#x}", code as u32));
        return false;
    }
    SQLITE_READY = true;
    true
}

/// A bubble's row in the home screen's database.
pub struct Bubble {
    pub title_id: String,
    /// The name under the bubble, on one line.
    pub title: String,
    /// The file the home screen draws as the bubble's icon (`iconPath`); for
    /// PSM titles `ur0:appmeta/<id>/icon0.dds`.
    pub icon_path: String,
}

/// One user of the database at a time: a scan runs on the main thread, an
/// icon lookup on a decoding thread.
static DB_LOCK: Mutex<()> = Mutex::new(());

/// One line per item, whitespace collapsed (bubble names can hold line breaks).
pub fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every bubble's row. None when the database cannot be opened or read.
pub unsafe fn bubbles() -> Option<Vec<Bubble>> {
    let _guard = DB_LOCK.lock().ok()?;
    if !sqlite_ready() {
        return None;
    }
    let mut db: *mut c_void = core::ptr::null_mut();
    let code = sqlite3_open_v2(APP_DB.as_ptr(), &mut db, SQLITE_OPEN_READONLY, core::ptr::null());
    if code != SQLITE_OK {
        crate::vita_log(format_args!("[PocketJS appdb] open app.db: {code}"));
        if !db.is_null() {
            sqlite3_close(db);
        }
        return None;
    }
    let sql = c"SELECT titleId, title, iconPath FROM tbl_appinfo_icon";
    let mut statement: *mut c_void = core::ptr::null_mut();
    let code = sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut statement, core::ptr::null_mut());
    if code != SQLITE_OK {
        crate::vita_log(format_args!("[PocketJS appdb] query app.db: {code}"));
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
            title_id: text(0).trim().to_string(),
            title: one_line(&text(1)),
            icon_path: text(2).trim().to_string(),
        });
    }
    sqlite3_finalize(statement);
    sqlite3_close(db);
    Some(rows)
}

/// Icon paths per title id, from the last read of the database. Filled by a
/// scan, or by the first icon lookup on a start that read the list file.
static ICON_PATHS: Mutex<Option<Vec<(String, String)>>> = Mutex::new(None);

/// Keep the icon paths of these rows for `icon_path`.
pub fn keep_icon_paths(rows: &[Bubble]) {
    let paths = rows
        .iter()
        .filter(|row| !row.icon_path.is_empty())
        .map(|row| (row.title_id.clone(), row.icon_path.clone()))
        .collect();
    if let Ok(mut kept) = ICON_PATHS.lock() {
        *kept = Some(paths);
    }
}

/// The file the home screen shows as this title's icon. Callable from any
/// thread.
pub unsafe fn icon_path(title_id: &str) -> Option<String> {
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
pub fn folder_files(path: &str) -> Vec<(String, String)> {
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

/// A value for a scan report line: no `,` or `=`, which separate the entries.
pub fn report_value(text: &str) -> String {
    text.replace([',', '='], "_")
}
