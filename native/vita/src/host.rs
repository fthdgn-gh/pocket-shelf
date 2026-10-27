//! `Host` on the Vita. The title scan, picture decoding and the log file run
//! on worker threads; the frame loop only sends requests and takes answers.

use std::fs;
use std::io::Write;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use shelf_core::host::{Event, Host, Picture, PictureKey, PluginState, ScanPhase, Status};

use crate::platform::{backdrop, installed};

/// The current app's data folder, shared with it.
pub const DATA: &str = "ux0:data/PocketShelf";

/// Whether this build may write `path` in the data folder: only the files
/// whose writers reproduce the current app's output byte for byte
/// (`core/src/library/round_trip.rs`).
fn writable(path: &str) -> bool {
    matches!(path, "settings.json" | "categories.json" | "recent.json" | "titles.cache.json") || removable(path)
}

/// A title's changes, `titles/<id>.json`: the one kind of file this build
/// deletes (a title reset to its defaults).
fn removable(path: &str) -> bool {
    path.strip_prefix("titles/")
        .and_then(|name| name.strip_suffix(".json"))
        .is_some_and(shelf_core::library::overrides::valid_title_id)
}
/// This build's own folder: its log.
pub const OWN: &str = "ux0:data/PocketShelfNext";

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
                installed::icon(id).map(|(w, h, rgba, accent)| Picture { w, h, rgba, accent: Some(accent as u32) })
            }
            PictureKey::Art(name) => crate::platform::art::picture(name).map(|(w, h, rgba)| {
                let accent = Some(crate::platform::accent::of(&rgba) as u32);
                Picture { w, h, rgba, accent }
            }),
            PictureKey::Backdrop { id, file } => {
                backdrop::picture(id, file).map(|(w, h, rgba)| Picture { w, h, rgba, accent: None })
            }
        }
    }
}

/// A change to the data folder, done in order on the file thread.
enum FileOp {
    Write(String, Vec<u8>),
    Remove(String),
    /// Answered once every earlier change is done.
    Flush(Sender<()>),
}

/// Do one change; a failure goes to the log.
fn file_op(op: FileOp) {
    match op {
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
/// title's backdrop stayed up meanwhile.
#[derive(Default)]
struct Waiting {
    icons: VecDeque<PictureKey>,
    backdrop: Option<PictureKey>,
    closed: bool,
}

#[derive(Default)]
struct Requests {
    waiting: Mutex<Waiting>,
    wake: Condvar,
}

impl Requests {
    /// The next request for a thread: an icon (`backdrops` false) or the
    /// newest backdrop; `None` once the host is gone.
    fn next(&self, backdrops: bool) -> Option<PictureKey> {
        let mut waiting = self.waiting.lock().ok()?;
        loop {
            if waiting.closed {
                return None;
            }
            let next = if backdrops { waiting.backdrop.take() } else { waiting.icons.pop_front() };
            if next.is_some() {
                return next;
            }
            waiting = self.wake.wait(waiting).ok()?;
        }
    }
}

/// A decoding thread: takes requests of its kind and answers them.
fn decoder(requests: Arc<Requests>, sender: Sender<Event>, backdrops: bool) {
    while let Some(key) = requests.next(backdrops) {
        let started = now_us();
        let picture = picture(&key);
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
            let _ = fs::create_dir_all(OWN);
            for line in lines {
                if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(format!("{OWN}/log.txt")) {
                    let _ = writeln!(file, "{line}");
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
            } else if installed::scan_start() {
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
        });
    }

    fn request(&mut self, key: PictureKey) {
        if let Ok(mut waiting) = self.requests.waiting.lock() {
            match key {
                // Only the newest backdrop matters.
                PictureKey::Backdrop { .. } => waiting.backdrop = Some(key),
                _ => waiting.icons.push_back(key),
            }
        }
        self.requests.wake.notify_all();
    }

    fn cancel(&mut self, key: &PictureKey) {
        if let Ok(mut waiting) = self.requests.waiting.lock() {
            waiting.icons.retain(|k| k != key);
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
}
