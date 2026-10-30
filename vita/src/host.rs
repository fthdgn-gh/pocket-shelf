//! `Host` on the Vita. The title scan, picture decoding and the log file run
//! on worker threads; the frame loop only sends requests and takes answers.

use std::fs;
use std::io::Write;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use shelf_core::host::{DecodeAs, Event, Host, NetId, NetKind, NetState, Picture, PictureKey, PluginState, ScanPhase, Status};

use crate::platform::{backdrop, installed};

/// The data folder, the same one Pocket Shelf 0.1 used.
pub const DATA: &str = "ux0:data/PocketShelf";

/// Whether this build may write `path` in the data folder: only the files
/// whose writers reproduce Pocket Shelf 0.1's output byte for byte
/// (`core/src/library/round_trip.rs`; `steamgriddb.txt` is the key and a
/// newline), and this build's own `status-bar.json`, `text-size.json`,
/// `plays.json` and `sort.json` (0.1 did not read them). Downloads go to their files through `http.rs`.
fn writable(path: &str) -> bool {
    matches!(
        path,
        "settings.json" | "categories.json" | "recent.json" | "titles.cache.json" | "status-bar.json" | "text-size.json" | "plays.json" | "sort.json" | "steamgriddb.txt" | "update.json" | "update/version.txt"
    )
        || path.strip_prefix("titles/").is_some_and(|_| removable(path))
}

/// The files this build deletes: a title's changes, `titles/<id>.json` (a
/// title reset to its defaults), and a PNG in `art/` or `backdrops/` (a
/// candidate not chosen, Clean up artwork). A name with a `/` is refused.
fn removable(path: &str) -> bool {
    if let Some(name) = path.strip_prefix("titles/") {
        return name.strip_suffix(".json").is_some_and(shelf_core::library::overrides::valid_title_id);
    }
    let picture = |name: &str| !name.contains('/') && !name.starts_with('.') && name.to_lowercase().ends_with(".png");
    path.strip_prefix("art/").or_else(|| path.strip_prefix("backdrops/")).is_some_and(picture)
}
/// The log, in the data folder. At `LOG_LIMIT` bytes it becomes
/// `log.old.txt` (replacing the one before) and a new one starts, so the two
/// stay under twice that on the card.
const LOG_FILE: &str = "log.txt";
const LOG_OLD: &str = "log.old.txt";
const LOG_LIMIT: u64 = 256 * 1024;

pub fn now_us() -> u64 {
    unsafe { vitasdk_sys::sceKernelGetProcessTimeWide() }
}

/// The log thread's queue, for code without the host at hand (platform/).
static LOG: OnceLock<Mutex<Sender<String>>> = OnceLock::new();

/// Queue a line for the log file, stamped with the time.
pub fn log_line(line: &str) {
    if let Some(sender) = LOG.get().and_then(|sender| sender.lock().ok()) {
        let _ = sender.send(format!("{:>8} ms  {line}", now_us() / 1000));
    }
}

/// A picture asked for, decoded. Runs on the picture thread.
fn picture(key: &PictureKey) -> Option<Picture> {
    unsafe {
        match key {
            PictureKey::Icon(id) => {
                installed::icon(id).map(|(w, h, rgba, accent)| Picture::new(w, h, rgba, Some(accent as u32)))
            }
            PictureKey::Art(name) => crate::platform::art::picture(name).map(|(w, h, rgba)| {
                let accent = Some(crate::platform::accent::of(&rgba) as u32);
                Picture::new(w, h, rgba, accent)
            }),
            PictureKey::Backdrop { id, file } => {
                backdrop::picture(id, file).map(|(w, h, rgba)| Picture::new(w, h, rgba, None))
            }
            // Pictures in memory come with their bytes (`decode_memory`).
            PictureKey::Memory { .. } => None,
        }
    }
}

/// A change to the data folder, done in order on the file thread.
enum FileOp {
    Write(String, Vec<u8>),
    /// A picture to `art/` or `backdrops/`, fitted into a square first.
    Picture(String, Vec<u8>, bool),
    Remove(String),
    /// Answered once every earlier change is done.
    Flush(Sender<()>),
}

/// A picture in memory decoded as the file it would be saved to.
fn decode_memory(as_: DecodeAs, png: &[u8]) -> Option<Picture> {
    let with_accent = |w, h, rgba: Vec<u8>| {
        let accent = Some(crate::platform::accent::of(&rgba) as u32);
        Picture::new(w, h, rgba, accent)
    };
    match as_ {
        DecodeAs::Art => crate::platform::art::decode_bytes(png).map(|(side, rgba)| with_accent(side, side, rgba)),
        DecodeAs::FittedArt => crate::platform::pspemu::fit_png(png).map(|(w, h, rgba)| with_accent(w, h, rgba)),
        DecodeAs::Backdrop => backdrop::decode(png).map(|(w, h, rgba)| Picture::new(w, h, rgba, None)),
    }
}

/// A picture file this build saves: `art/` or `backdrops/` and a plain PNG
/// name.
fn picture_path(path: &str) -> bool {
    let Some((dir, name)) = path.split_once('/') else { return false };
    (dir == "art" || dir == "backdrops")
        && !name.starts_with('.')
        && name.to_lowercase().ends_with(".png")
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// Do one change; a failure goes to the log.
fn file_op(op: FileOp) {
    match op {
        FileOp::Picture(path, png, fit) => {
            let png = if fit { crate::platform::http::shrink_bytes(&png) } else { Ok(png) };
            match png {
                Ok(png) => file_op(FileOp::Write(path, png)),
                Err(error) => log_line(&format!("save {path}: {error}")),
            }
        }
        FileOp::Write(path, data) => {
            let full = format!("{DATA}/{path}");
            if let Some((dir, _)) = full.rsplit_once('/') {
                let _ = fs::create_dir_all(dir);
            }
            if let Err(error) = fs::write(&full, data) {
                log_line(&format!("write {path}: {error}"));
            }
        }
        FileOp::Remove(path) => match fs::remove_file(format!("{DATA}/{path}")) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => log_line(&format!("remove {path}: {error}")),
        },
        FileOp::Flush(done) => {
            let _ = done.send(());
        }
    }
}

/// Picture requests waiting for a decoding thread: icons and box art in
/// order, which the app can take back once scrolled past, and the newest
/// backdrop alone. Backdrops have their own thread: behind one queue they
/// waited for every icon asked for while scrolling, and the previous
/// title's backdrop stayed up meanwhile. The neighbors' backdrops
/// (`request_ahead`) wait behind the selected one.
#[derive(Default)]
struct Waiting {
    icons: VecDeque<PictureKey>,
    backdrop: Option<PictureKey>,
    ahead: VecDeque<PictureKey>,
    /// Pictures in memory to decode, by key; taken with their request.
    bytes: std::collections::HashMap<PictureKey, Vec<u8>>,
    closed: bool,
}

#[derive(Default)]
struct Requests {
    waiting: Mutex<Waiting>,
    wake: Condvar,
}

impl Requests {
    /// The next request for a thread: an icon (`backdrops` false) or the
    /// newest backdrop, else a neighbor's; `None` once the host is gone.
    fn next(&self, backdrops: bool) -> Option<(PictureKey, Option<Vec<u8>>)> {
        let mut waiting = self.waiting.lock().ok()?;
        loop {
            if waiting.closed {
                return None;
            }
            let next = if backdrops { waiting.backdrop.take().or_else(|| waiting.ahead.pop_front()) } else { waiting.icons.pop_front() };
            if let Some(key) = next {
                let bytes = waiting.bytes.remove(&key);
                return Some((key, bytes));
            }
            waiting = self.wake.wait(waiting).ok()?;
        }
    }
}

/// A decoding thread: takes requests of its kind and answers them.
fn decoder(requests: Arc<Requests>, sender: Sender<Event>, backdrops: bool) {
    while let Some((key, bytes)) = requests.next(backdrops) {
        let started = now_us();
        let picture = match (&key, bytes) {
            (PictureKey::Memory { as_, .. }, Some(png)) => decode_memory(*as_, &png),
            _ => picture(&key),
        };
        if let (true, PictureKey::Backdrop { id, .. }) = (backdrops, &key) {
            log_line(&format!("backdrop {id} in {} ms{}", (now_us() - started) / 1000, if picture.is_none() { " (none)" } else { "" }));
        }
        if sender.send(Event::Picture(key, picture)).is_err() {
            return;
        }
    }
}

pub struct VitaHost {
    answers: Receiver<Event>,
    answer_sender: Sender<Event>,
    requests: Arc<Requests>,
    files: Sender<FileOp>,
}

impl Drop for VitaHost {
    fn drop(&mut self) {
        if let Ok(mut waiting) = self.requests.waiting.lock() {
            waiting.closed = true;
        }
        self.requests.wake.notify_all();
    }
}

impl VitaHost {
    pub fn new() -> Self {
        let (answer_sender, answers) = channel();
        let requests = Arc::new(Requests::default());
        let (log, lines) = channel::<String>();
        for (name, backdrops) in [("shelf-icons", false), ("shelf-backdrops", true)] {
            let (requests, sender) = (requests.clone(), answer_sender.clone());
            spawn(name, move || decoder(requests, sender, backdrops));
        }
        spawn("shelf-log", move || {
            let _ = fs::create_dir_all(DATA);
            let (path, old) = (format!("{DATA}/{LOG_FILE}"), format!("{DATA}/{LOG_OLD}"));
            let open = || fs::OpenOptions::new().create(true).append(true).open(&path).ok();
            let mut size = fs::metadata(&path).map_or(0, |m| m.len());
            let mut file = open();
            for line in lines {
                if size > 0 && size + line.len() as u64 + 1 > LOG_LIMIT {
                    drop(file.take());
                    let _ = fs::rename(&path, &old);
                    file = open();
                    size = 0;
                }
                if let Some(file) = &mut file {
                    if writeln!(file, "{line}").is_ok() {
                        size += line.len() as u64 + 1;
                    }
                }
            }
        });
        let (files, file_ops) = channel::<FileOp>();
        spawn("shelf-files", move || file_ops.into_iter().for_each(file_op));
        let _ = LOG.set(Mutex::new(log));
        VitaHost { answers, answer_sender, requests, files }
    }
}

fn spawn(name: &str, run: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new().name(name.into()).stack_size(256 * 1024).spawn(run).expect("worker thread");
}

impl Host for VitaHost {
    fn time_us(&self) -> u64 {
        now_us()
    }

    fn read(&mut self, path: &str) -> Option<Vec<u8>> {
        fs::read(format!("{DATA}/{path}")).ok()
    }

    fn write(&mut self, path: &str, data: &[u8]) -> bool {
        if !writable(path) {
            self.log(&format!("write refused (no round-trip test): {path}"));
            return false;
        }
        self.files.send(FileOp::Write(path.into(), data.to_vec())).is_ok()
    }

    fn flush(&mut self) {
        let (done, wait) = channel();
        if self.files.send(FileOp::Flush(done)).is_ok() {
            let _ = wait.recv();
        }
    }

    fn remove(&mut self, path: &str) -> bool {
        if !removable(path) {
            self.log(&format!("remove refused: {path}"));
            return false;
        }
        self.files.send(FileOp::Remove(path.into())).is_ok()
    }

    fn list(&mut self, dir: &str) -> Vec<String> {
        let Ok(entries) = fs::read_dir(format!("{DATA}/{dir}")) else { return Vec::new() };
        let mut names: Vec<String> = entries.flatten().filter_map(|e| e.file_name().to_str().map(String::from)).collect();
        names.sort();
        names
    }

    fn listing(&mut self, folder: &str) {
        use shelf_core::online::libretro::{listing_path, Listing};
        let (folder, sender) = (folder.to_string(), self.answer_sender.clone());
        spawn("shelf-listing", move || {
            let started = now_us();
            let listing = fs::read(format!("{DATA}/{}", listing_path(&folder))).ok().and_then(|b| Listing::parse(&String::from_utf8_lossy(&b)));
            log_line(&format!("listing {folder}: {} names in {} ms", listing.as_ref().map_or(0, |l| l.len()), (now_us() - started) / 1000));
            let _ = sender.send(Event::Listing(folder, listing));
        });
    }

    fn build(&self) -> shelf_core::updates::BuildInfo {
        shelf_core::updates::BuildInfo {
            version: env!("SHELF_VERSION").into(),
            channel: env!("SHELF_CHANNEL").into(),
            commit: env!("SHELF_COMMIT").into(),
        }
    }

    fn unix_ms(&self) -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
    }

    fn can_install_update(&self) -> bool {
        // The updater app (updater/main.c) installs and starts
        // POCKTSHLF only; the side-by-side build checks but does not
        // download.
        crate::platform::installed::SELF_TITLE_ID == "POCKTSHLF"
    }

    fn update_unpack(&mut self) -> bool {
        crate::platform::update::unpack_start()
    }

    fn update_install(&mut self) -> bool {
        crate::platform::update::install_start()
    }

    fn update_launch(&mut self) -> bool {
        crate::platform::update::launch()
    }

    fn update_state(&mut self) -> String {
        crate::platform::update::state()
    }

    fn update_result(&mut self) -> String {
        crate::platform::update::take_result()
    }

    fn scan_report(&mut self) -> String {
        use crate::platform::{psm, pspemu};
        let parts = unsafe { [psm::report(), pspemu::report(), installed::retro_report()] };
        parts.iter().filter(|part| !part.is_empty()).cloned().collect::<Vec<_>>().join(",")
    }

    fn boot_log(&mut self) -> String {
        crate::platform::pspemu::boot_log()
    }

    fn status(&mut self) -> Status {
        crate::status::read()
    }

    fn scan(&mut self, force: bool) {
        let sender = self.answer_sender.clone();
        spawn("shelf-scan", move || unsafe {
            let started = now_us();
            // The saved list when there is one; else a full scan, which
            // saves it (installed.rs).
            if !force && installed::titles_ready() {
                log_line("titles: read the saved list");
            } else if !installed::scan_start() {
                // A rescan the user asked for says so; at a start, the saved
                // list (if any) is used.
                log_line("titles: the scan could not start");
                if force {
                    let _ = sender.send(Event::ScanFailed);
                    return;
                }
            } else {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let state = installed::scan_state();
                    if state.starts_with("done") {
                        log_line(&format!("titles: scanned ({state})"));
                        break;
                    }
                    // `running <done> <total> <phase>`: the scan page's bar.
                    let mut words = state.split(' ');
                    if words.next() == Some("running") {
                        let done = words.next().and_then(|w| w.parse().ok()).unwrap_or(0);
                        let total = words.next().and_then(|w| w.parse().ok()).unwrap_or(0);
                        let phase = ScanPhase::from_key(words.next().unwrap_or(""));
                        let _ = sender.send(Event::Scan { done, total, phase });
                    }
                }
            }
            let titles = shelf_core::library::catalog::load_catalog(&installed::infos());
            let _ = sender.send(Event::Titles(titles, ((now_us() - started) / 1000) as u32));
            // The system's last-launch and install dates, for the sort orders.
            let read = now_us();
            let dates = crate::platform::appdb::dates().unwrap_or_default();
            log_line(&format!("dates: {} titles from app.db in {} ms", dates.len(), (now_us() - read) / 1000));
            let _ = sender.send(Event::Dates(dates));
        });
    }

    fn request(&mut self, key: PictureKey) {
        if let Ok(mut waiting) = self.requests.waiting.lock() {
            match key {
                // Only the newest backdrop matters.
                PictureKey::Backdrop { .. } | PictureKey::Memory { as_: DecodeAs::Backdrop, .. } => waiting.backdrop = Some(key),
                _ => waiting.icons.push_back(key),
            }
        }
        self.requests.wake.notify_all();
    }

    fn request_ahead(&mut self, key: PictureKey) {
        if let Ok(mut waiting) = self.requests.waiting.lock() {
            if !waiting.ahead.contains(&key) {
                waiting.ahead.push_back(key);
            }
        }
        self.requests.wake.notify_all();
    }

    fn cancel(&mut self, key: &PictureKey) {
        if let Ok(mut waiting) = self.requests.waiting.lock() {
            waiting.icons.retain(|k| k != key);
            waiting.ahead.retain(|k| k != key);
            waiting.bytes.remove(key);
            if waiting.backdrop.as_ref() == Some(key) {
                waiting.backdrop = None;
            }
        }
    }

    fn poll(&mut self, events: &mut Vec<Event>) {
        events.extend(self.answers.try_iter());
    }

    fn log(&mut self, line: &str) {
        log_line(line);
    }

    fn launch(&mut self, id: &str) -> bool {
        unsafe { installed::launch(id) }
    }

    fn plugin_state(&mut self) -> PluginState {
        match crate::platform::pspemu::plugin_state() {
            "noAdrenaline" => PluginState::NoAdrenaline,
            "missing" => PluginState::Missing,
            "off" => PluginState::Off,
            _ => PluginState::On,
        }
    }

    fn enable_plugin(&mut self) -> bool {
        crate::platform::pspemu::enable_plugin()
    }

    fn net_start(&mut self, url: &str, kind: NetKind) -> Option<NetId> {
        use crate::platform::http;
        let id = match &kind {
            NetKind::Text { authorization } => http::start(url, authorization, None, false),
            NetKind::Bytes => http::start_bytes(url),
            NetKind::Save(path) => http::start(url, "", Some(path), false),
            NetKind::SaveIcon(path) => http::start(url, "", Some(path), true),
        };
        (id >= 0).then_some(NetId(id))
    }

    fn net_state(&mut self, id: NetId) -> NetState {
        NetState::parse(&crate::platform::http::state(id.0))
    }

    fn net_text(&mut self, id: NetId) -> String {
        crate::platform::http::text(id.0)
    }

    fn net_close(&mut self, id: NetId) {
        crate::platform::http::close(id.0);
    }

    fn net_bytes(&mut self, id: NetId) -> Vec<u8> {
        crate::platform::http::take_bytes(id.0)
    }

    fn decode(&mut self, key: PictureKey, png: Vec<u8>) {
        if let Ok(mut waiting) = self.requests.waiting.lock() {
            waiting.bytes.insert(key.clone(), png);
        }
        self.request(key);
    }

    fn save_picture(&mut self, path: &str, png: Vec<u8>, fit: bool) -> bool {
        picture_path(path) && self.files.send(FileOp::Picture(path.into(), png, fit)).is_ok()
    }
}
