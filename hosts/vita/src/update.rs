//! The app's own update (cargo feature `self-update`).
//!
//! src/updates.ts downloads a release's VPK to `update/pocket-shelf.vpk` in
//! the data folder (`http::UPDATE_VPK`). `unpack_start` then unpacks it into
//! `update/pkg/` on a worker thread, so the main thread keeps drawing, and
//! checks that it is this app: its `param.sfo` must name this title id.
//!
//! Installing: the system's package installer replaces `ux0:app/<title id>`,
//! which this running app is mounted from, so a separate app does it. The VPK
//! carries that app, Pocket Shelf Updater (`app0:updater/`, built from
//! src/updater/, GPL-3.0). `install_start` installs it with the installer
//! (`__updateInstall`), the guest then starts it (`__updateLaunch`) and this
//! process ends. The updater installs `update/pkg/`, writes
//! `update/result.txt` and starts Pocket Shelf again, which reads the result
//! (`__updateResult`) and removes the updater app.
//!
//! Host extras on `ui` (not spec ops):
//!
//!   __updateUnpack() -> 0 | -1     start unpacking; -1 while a step runs
//!   __updateInstall() -> 0 | -1    install the updater app; -1 unless unpacked
//!   __updateLaunch() -> 0 | -1     start the updater after this frame and exit
//!   __updateState() -> "idle"
//!                    | "busy <files done> <files>"
//!                    | "done <version>"   unpacked: the package's APP_VER
//!                    | "installing"
//!                    | "installed"        the updater app is ready to start
//!                    | "error <reason>"
//!   __updateResult() -> "" | "ok" | "error <reason>"
//!                    the updater's report, read once at start; the update
//!                    files and the updater app are then removed

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::string::String;
use std::sync::Mutex;
use std::vec::Vec;

/// Files in a VPK. Pocket Shelf's has about forty.
const ENTRIES_MAX: usize = 4096;
/// Largest file unpacked. The release eboot.bin is about 2.4 MB.
const ENTRY_MAX: usize = 64 * 1024 * 1024;
/// The end-of-central-directory record is in the file's last 64 KiB + 22 bytes.
const TAIL_MAX: u64 = 65_557;

/// The updater app's title id (src/updater/Makefile).
const UPDATER_ID: &str = "POCKTUPDR";
/// Where the VPK carries the updater's package.
const UPDATER_SOURCE: &str = "app0:updater";

enum State {
    Idle,
    Busy { done: usize, total: usize },
    Done { version: String },
    Installing,
    Installed,
    Failed(String),
}

static STATE: Mutex<State> = Mutex::new(State::Idle);

fn set(state: State) {
    if let Ok(mut current) = STATE.lock() {
        *current = state;
    }
}

/// The folder the package is unpacked into.
pub fn package_dir() -> String {
    format!("{}/update/pkg", crate::datafs::data_dir())
}

/// Start unpacking. False while an unpack is running or no thread starts.
pub fn unpack_start() -> bool {
    {
        let Ok(mut state) = STATE.lock() else {
            return false;
        };
        if matches!(*state, State::Busy { .. } | State::Installing) {
            return false;
        }
        *state = State::Busy { done: 0, total: 0 };
    }
    let spawned = std::thread::Builder::new()
        .name(String::from("pocket-update"))
        .spawn(|| {
            let vpk = format!("{}/{}", crate::datafs::data_dir(), crate::http::UPDATE_VPK);
            let dir = package_dir();
            match unpack(&vpk, &dir).and_then(|()| check(&dir)) {
                Ok(version) => set(State::Done { version }),
                Err(reason) => {
                    let _ = fs::remove_dir_all(&dir);
                    set(State::Failed(reason));
                }
            }
        });
    if spawned.is_err() {
        set(State::Failed(String::from("thread")));
        return false;
    }
    true
}

/// One line describing the unpack; see the module comment.
pub fn state() -> String {
    match STATE.lock().as_deref() {
        Ok(State::Idle) => String::from("idle"),
        Ok(State::Busy { done, total }) => format!("busy {done} {total}"),
        Ok(State::Done { version }) => format!("done {version}"),
        Ok(State::Installing) => String::from("installing"),
        Ok(State::Installed) => String::from("installed"),
        Ok(State::Failed(reason)) => format!("error {reason}"),
        Err(_) => String::from("error state"),
    }
}

/// The package names this app and carries an executable. Returns its APP_VER.
fn check(dir: &str) -> Result<String, String> {
    let sfo = fs::read(format!("{dir}/sce_sys/param.sfo")).map_err(|_| String::from("no param.sfo"))?;
    let title = crate::installed::sfo_string(&sfo, "TITLE_ID").unwrap_or_default();
    if title != crate::dev::TITLE_ID {
        return Err(format!("title id {title}"));
    }
    if fs::metadata(format!("{dir}/eboot.bin")).map(|meta| meta.len()).unwrap_or(0) == 0 {
        return Err(String::from("no eboot.bin"));
    }
    Ok(crate::installed::sfo_string(&sfo, "APP_VER").unwrap_or_default())
}

struct Entry {
    name: String,
    method: u16,
    crc: u32,
    compressed: usize,
    size: usize,
    header: u64,
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    bytes.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    bytes.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// A path inside the package: relative, `/`-separated, no `..` or empty parts.
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('/')
        && !name.contains('\\')
        && !name.contains(':')
        && name.trim_end_matches('/').split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

/// The central directory of a ZIP file: every entry's name, sizes and where its data starts.
fn entries(file: &mut fs::File) -> Result<Vec<Entry>, String> {
    let length = file.metadata().map_err(|_| String::from("vpk stat"))?.len();
    let tail_len = length.min(TAIL_MAX);
    let mut tail = vec![0u8; tail_len as usize];
    file.seek(SeekFrom::Start(length - tail_len)).map_err(|_| String::from("vpk seek"))?;
    file.read_exact(&mut tail).map_err(|_| String::from("vpk read"))?;
    let end = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&at| tail[at..at + 4] == [b'P', b'K', 5, 6])
        .ok_or_else(|| String::from("vpk end record"))?;
    let count = u16_at(&tail, end + 10).unwrap_or(0) as usize;
    let dir_size = u32_at(&tail, end + 12).unwrap_or(0) as usize;
    let dir_offset = u32_at(&tail, end + 16).unwrap_or(0) as u64;
    if count > ENTRIES_MAX || dir_offset + dir_size as u64 > length {
        return Err(String::from("vpk directory"));
    }
    let mut dir = vec![0u8; dir_size];
    file.seek(SeekFrom::Start(dir_offset)).map_err(|_| String::from("vpk seek"))?;
    file.read_exact(&mut dir).map_err(|_| String::from("vpk read"))?;
    let mut list = Vec::with_capacity(count);
    let mut at = 0usize;
    for _ in 0..count {
        if u32_at(&dir, at) != Some(0x0201_4b50) {
            return Err(String::from("vpk directory entry"));
        }
        let field = |offset: usize| u32_at(&dir, at + offset).ok_or_else(|| String::from("vpk directory entry"));
        let short = |offset: usize| u16_at(&dir, at + offset).ok_or_else(|| String::from("vpk directory entry"));
        let name_len = short(28)? as usize;
        let extra_len = short(30)? as usize;
        let comment_len = short(32)? as usize;
        let name = dir
            .get(at + 46..at + 46 + name_len)
            .and_then(|raw| std::str::from_utf8(raw).ok())
            .ok_or_else(|| String::from("vpk file name"))?;
        if !safe_name(name) {
            return Err(format!("vpk file name {name}"));
        }
        list.push(Entry {
            name: String::from(name),
            method: short(10)?,
            crc: field(16)?,
            compressed: field(20)? as usize,
            size: field(24)? as usize,
            header: field(42)? as u64,
        });
        at += 46 + name_len + extra_len + comment_len;
    }
    Ok(list)
}

/// Unpack every file of the ZIP at `vpk` into `dir`, replacing what was there.
fn unpack(vpk: &str, dir: &str) -> Result<(), String> {
    let mut file = fs::File::open(vpk).map_err(|_| String::from("no vpk"))?;
    let list = entries(&mut file)?;
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).map_err(|_| String::from("mkdir"))?;
    let total = list.len();
    set(State::Busy { done: 0, total });
    for (index, entry) in list.iter().enumerate() {
        let path = format!("{dir}/{}", entry.name.trim_end_matches('/'));
        if entry.name.ends_with('/') {
            fs::create_dir_all(&path).map_err(|_| String::from("mkdir"))?;
        } else {
            if entry.size > ENTRY_MAX || entry.compressed > ENTRY_MAX {
                return Err(format!("{} too large", entry.name));
            }
            let data = read_entry(&mut file, entry)?;
            if let Some((parent, _)) = path.rsplit_once('/') {
                fs::create_dir_all(parent).map_err(|_| String::from("mkdir"))?;
            }
            fs::write(&path, &data).map_err(|_| format!("write {}", entry.name))?;
        }
        set(State::Busy { done: index + 1, total });
    }
    Ok(())
}

/// One file's bytes, inflated and checked against its CRC-32.
fn read_entry(file: &mut fs::File, entry: &Entry) -> Result<Vec<u8>, String> {
    let mut local = [0u8; 30];
    file.seek(SeekFrom::Start(entry.header)).map_err(|_| String::from("vpk seek"))?;
    file.read_exact(&mut local).map_err(|_| String::from("vpk read"))?;
    if u32_at(&local, 0) != Some(0x0403_4b50) {
        return Err(format!("{} header", entry.name));
    }
    let skip = u16_at(&local, 26).unwrap_or(0) as i64 + u16_at(&local, 28).unwrap_or(0) as i64;
    file.seek(SeekFrom::Current(skip)).map_err(|_| String::from("vpk seek"))?;
    let mut raw = vec![0u8; entry.compressed];
    file.read_exact(&mut raw).map_err(|_| format!("{} cut short", entry.name))?;
    let data = match entry.method {
        0 => raw,
        8 => miniz_oxide::inflate::decompress_to_vec_with_limit(&raw, entry.size)
            .map_err(|_| format!("{} damaged", entry.name))?,
        method => return Err(format!("{} method {method}", entry.name)),
    };
    if data.len() != entry.size || crc32(&data) != entry.crc {
        return Err(format!("{} damaged", entry.name));
    }
    Ok(data)
}

fn crc32(bytes: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (index, slot) in table.iter_mut().enumerate() {
            let mut value = index as u32;
            for _ in 0..8 {
                value = if value & 1 != 0 { 0xedb8_8320 ^ (value >> 1) } else { value >> 1 };
            }
            *slot = value;
        }
        table
    });
    !bytes
        .iter()
        .fold(!0u32, |crc, &byte| table[((crc ^ byte as u32) & 0xff) as usize] ^ (crc >> 8))
}

// --- Installing ----------------------------------------------------------------

fn update_dir() -> String {
    format!("{}/update", crate::datafs::data_dir())
}

/// Copy the files under `from` into `to`, folders included.
fn copy_tree(from: &str, to: &str) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|_| format!("mkdir {to}"))?;
    for entry in fs::read_dir(from).map_err(|_| format!("read {from}"))? {
        let entry = entry.map_err(|_| format!("read {from}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let source = format!("{from}/{name}");
        let target = format!("{to}/{name}");
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|_| format!("copy {name}"))?;
        }
    }
    Ok(())
}

/// The installer is part of the system's PAF library, which has to be loaded first.
/// The argument block and options are the values the system expects for it.
struct Promoter;

impl Promoter {
    unsafe fn open() -> Result<Self, String> {
        use vitasdk_sys::*;
        let mut argp: [u32; 6] = [0x18_0000, u32::MAX, u32::MAX, 1, u32::MAX, u32::MAX];
        let mut result: i32 = -1;
        let opt = SceSysmoduleOpt { flags: 16, result: &mut result, unused: [-1, -1] };
        let code = sceSysmoduleLoadModuleInternalWithArg(
            SCE_SYSMODULE_INTERNAL_PAF,
            core::mem::size_of_val(&argp) as u32,
            argp.as_mut_ptr().cast(),
            &opt,
        );
        if code < 0 {
            return Err(format!("paf 0x{:08x}", code as u32));
        }
        let code = sceSysmoduleLoadModuleInternal(SCE_SYSMODULE_INTERNAL_PROMOTER_UTIL);
        if code < 0 {
            Self::unload_paf();
            return Err(format!("promoter 0x{:08x}", code as u32));
        }
        let code = scePromoterUtilityInit();
        if code < 0 {
            sceSysmoduleUnloadModuleInternal(SCE_SYSMODULE_INTERNAL_PROMOTER_UTIL);
            Self::unload_paf();
            return Err(format!("promoter init 0x{:08x}", code as u32));
        }
        Ok(Promoter)
    }

    unsafe fn unload_paf() {
        let opt = vitasdk_sys::SceSysmoduleOpt { flags: 0, result: core::ptr::null_mut(), unused: [0, 0] };
        vitasdk_sys::sceSysmoduleUnloadModuleInternalWithArg(
            vitasdk_sys::SCE_SYSMODULE_INTERNAL_PAF,
            0,
            core::ptr::null_mut(),
            &opt,
        );
    }
}

impl Drop for Promoter {
    fn drop(&mut self) {
        unsafe {
            vitasdk_sys::scePromoterUtilityExit();
            vitasdk_sys::sceSysmoduleUnloadModuleInternal(vitasdk_sys::SCE_SYSMODULE_INTERNAL_PROMOTER_UTIL);
            Self::unload_paf();
        }
    }
}

fn install_updater() -> Result<(), String> {
    let dir = format!("{}/helper", update_dir());
    let _ = fs::remove_dir_all(&dir);
    copy_tree(UPDATER_SOURCE, &dir)?;
    let path = std::ffi::CString::new(dir.as_str()).map_err(|_| String::from("path"))?;
    unsafe {
        let _promoter = Promoter::open()?;
        let code = vitasdk_sys::scePromoterUtilityPromotePkgWithRif(path.as_ptr(), 1);
        if code < 0 {
            let _ = fs::remove_dir_all(&dir);
            return Err(format!("install updater 0x{:08x}", code as u32));
        }
    }
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

/// Install the updater app on a worker thread. False unless a package is unpacked.
pub fn install_start() -> bool {
    {
        let Ok(mut state) = STATE.lock() else {
            return false;
        };
        if !matches!(*state, State::Done { .. }) {
            return false;
        }
        *state = State::Installing;
    }
    let spawned = std::thread::Builder::new()
        .name(String::from("pocket-update"))
        .spawn(|| match install_updater() {
            Ok(()) => set(State::Installed),
            Err(reason) => set(State::Failed(reason)),
        });
    if spawned.is_err() {
        set(State::Failed(String::from("thread")));
        return false;
    }
    true
}

/// Start the updater after this frame; this process then ends.
pub fn launch() -> bool {
    let ready = matches!(STATE.lock().as_deref(), Ok(State::Installed));
    ready && unsafe { crate::installed::launch_unlisted(UPDATER_ID) }
}

/// The updater's report from its last run, read once. When there is one, or
/// the updater app is still installed, the update files and the app are
/// removed on a worker thread.
pub fn take_result() -> String {
    let dir = update_dir();
    let path = format!("{dir}/result.txt");
    let report = fs::read_to_string(&path).map(|text| String::from(text.trim())).unwrap_or_default();
    let updater_left = fs::metadata(format!("ux0:/app/{UPDATER_ID}")).is_ok();
    if !report.is_empty() || updater_left {
        let _ = fs::remove_file(&path);
        let _ = std::thread::Builder::new().name(String::from("pocket-update")).spawn(move || {
            let _ = fs::remove_dir_all(&dir);
            if updater_left {
                if let Ok(id) = std::ffi::CString::new(UPDATER_ID) {
                    unsafe {
                        if let Ok(_promoter) = Promoter::open() {
                            vitasdk_sys::scePromoterUtilityDeletePkg(id.as_ptr());
                        }
                    }
                }
            }
        });
    }
    report
}
