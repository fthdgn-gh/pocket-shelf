//! `Host` on the Vita. The title scan, picture decoding and the log file run
//! on worker threads; the frame loop only sends requests and takes answers.

use std::fs;
use std::io::Write;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use shelf_core::host::{Event, Host, Picture, PictureKey, ScanPhase, Status};

use crate::platform::{backdrop, installed};

/// The current app's data folder. Read only until each file's writer has a
/// round-trip test (native/PLAN.md).
pub const DATA: &str = "ux0:data/PocketShelf";
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
            PictureKey::Backdrop(id) => backdrop::picture(id, "").map(|(w, h, rgba)| Picture { w, h, rgba, accent: None }),
        }
    }
}

pub struct VitaHost {
    answers: Receiver<Event>,
    answer_sender: Sender<Event>,
    pictures: Sender<PictureKey>,
}

impl VitaHost {
    pub fn new() -> Self {
        let (answer_sender, answers) = channel();
        let (pictures, picture_requests) = channel::<PictureKey>();
        let (log, lines) = channel::<String>();
        let sender = answer_sender.clone();
        spawn("shelf-pictures", move || {
            for key in picture_requests {
                let picture = picture(&key);
                if sender.send(Event::Picture(key, picture)).is_err() {
                    return;
                }
            }
        });
        spawn("shelf-log", move || {
            let _ = fs::create_dir_all(OWN);
            for line in lines {
                if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(format!("{OWN}/log.txt")) {
                    let _ = writeln!(file, "{line}");
                }
            }
        });
        let _ = LOG.set(Mutex::new(log));
        VitaHost { answers, answer_sender, pictures }
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

    fn write(&mut self, path: &str, _data: &[u8]) -> bool {
        self.log(&format!("write refused (read-only for now): {path}"));
        false
    }

    fn remove(&mut self, path: &str) -> bool {
        self.log(&format!("remove refused (read-only for now): {path}"));
        false
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

    fn scan(&mut self) {
        let sender = self.answer_sender.clone();
        spawn("shelf-scan", move || unsafe {
            let started = now_us();
            // The saved list when there is one; else a full scan, which
            // saves it (installed.rs).
            if installed::titles_ready() {
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
        let _ = self.pictures.send(key);
    }

    fn poll(&mut self, events: &mut Vec<Event>) {
        events.extend(self.answers.try_iter());
    }

    fn log(&mut self, line: &str) {
        log_line(line);
    }
}
