//! What the app asks of the platform. The Vita implements it on the
//! system's calls; the desktop fakes it with sample data, as
//! the PocketJS preview did for Pocket Shelf 0.1.
//!
//! Slow work (scans, decoding, network) is asked for and answered later as
//! an [`Event`], so the frame loop never waits on it.

/// What the status bar shows; `None` for a reading the host cannot give.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Status {
    pub hour: u8,
    pub minute: u8,
    /// The system's clock format; `None` when unknown (24-hour is then used).
    pub clock24: Option<bool>,
    /// Battery charge in percent.
    pub battery: Option<u8>,
    pub charging: bool,
    /// Wi-Fi signal in percent, 0 when not connected.
    pub wifi: Option<u8>,
    /// Whether Bluetooth is switched on in Settings.
    pub bluetooth: Option<bool>,
}

pub use crate::library::catalog::Game;

/// One installed title as the host lists it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TitleInfo {
    pub id: String,
    pub title: String,
    /// `vita`, `psm`, `psp`, `psx` or `retro`.
    pub platform: String,
    /// A PSP or PS1 title with no bubble, started through Adrenaline.
    pub adrenaline: bool,
    /// A RetroArch game's console: its database name from the playlist
    /// (`Nintendo - Game Boy`).
    pub console: Option<String>,
}

/// An RGBA picture.
pub struct Picture {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
    /// An icon's strongest color, `ACCENT_HAS_ALPHA | 0xRRGGBB`
    /// (`theme::box_colors`); None for other pictures.
    pub accent: Option<u32>,
    /// The part that shows, `[x, y, w, h]` in texels: for a picture fitted
    /// into a square with clear bars on two opposite sides (a PSP icon,
    /// 144×80; a RetroArch box art), the picture's own rectangle; else the
    /// whole picture. The selection ring and the icon box are drawn around
    /// this part.
    pub content: [u32; 4],
}

/// Texels with less alpha than this count as see-through (an antialiased
/// edge's faint texels do not widen the box).
const SHOWN_ALPHA: u8 = 16;

impl Picture {
    /// A picture with its `content` worked out: call it on the thread that
    /// decoded the picture, not in a frame (it reads every texel).
    pub fn new(w: u32, h: u32, rgba: Vec<u8>, accent: Option<u32>) -> Picture {
        let content = visible_box(&rgba, w, h);
        Picture { w, h, rgba, accent, content }
    }
}

/// `[x, y, w, h]` of the part of an RGBA picture that shows: the box
/// around its texels with alpha of at least `SHOWN_ALPHA` when that box
/// spans the picture's whole width or whole height (clear bars on two
/// opposite sides, as a fitted picture has). A round or shaped icon has
/// clear margins on every side and counts as whole, so its icon box stays
/// square.
pub fn visible_box(rgba: &[u8], w: u32, h: u32) -> [u32; 4] {
    let whole = [0, 0, w, h];
    let [x, y, bw, bh] = opaque_box(rgba, w, h);
    // Within 2% of an edge counts as reaching it (a scaled edge's faint
    // texels).
    let spans = |part: u32, all: u32| part * 50 >= all * 49;
    if spans(bw, w) && !spans(bh, h) {
        [0, y, w, bh]
    } else if spans(bh, h) && !spans(bw, w) {
        [x, 0, bw, h]
    } else {
        whole
    }
}

/// `[x, y, w, h]` around the texels with alpha of at least `SHOWN_ALPHA`;
/// the whole picture when none has.
fn opaque_box(rgba: &[u8], w: u32, h: u32) -> [u32; 4] {
    let (w, h) = (w as usize, h as usize);
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return [0, 0, w as u32, h as u32];
    }
    let shown = |x: usize, y: usize| rgba[(y * w + x) * 4 + 3] >= SHOWN_ALPHA;
    let row_shown = |y: usize| (0..w).any(|x| shown(x, y));
    let Some(top) = (0..h).find(|&y| row_shown(y)) else { return [0, 0, w as u32, h as u32] };
    let bottom = (0..h).rev().find(|&y| row_shown(y)).unwrap_or(top);
    let column_shown = |x: usize| (top..=bottom).any(|y| shown(x, y));
    let left = (0..w).find(|&x| column_shown(x)).unwrap_or(0);
    let right = (0..w).rev().find(|&x| column_shown(x)).unwrap_or(w - 1);
    [left as u32, top as u32, (right - left + 1) as u32, (bottom - top + 1) as u32]
}

#[cfg(test)]
mod visible_tests {
    use super::*;

    #[test]
    fn the_visible_box_skips_clear_bars_and_faint_edges() {
        // 8×6: clear except a 4×2 block at (2, 2), with a faint texel at
        // (0, 0).
        let mut rgba = vec![0u8; 8 * 6 * 4];
        for y in 2..4 {
            for x in 2..6 {
                rgba[(y * 8 + x) * 4 + 3] = 255;
            }
        }
        rgba[3] = 10;
        assert_eq!(opaque_box(&rgba, 8, 6), [2, 2, 4, 2]);
        // Clear on every side: a shaped icon, shown whole.
        assert_eq!(visible_box(&rgba, 8, 6), [0, 0, 8, 6]);
        // Bars above and below only: the rows between show.
        let mut wide = vec![0u8; 8 * 6 * 4];
        for y in 1..4 {
            for x in 0..8 {
                wide[(y * 8 + x) * 4 + 3] = 255;
            }
        }
        assert_eq!(visible_box(&wide, 8, 6), [0, 1, 8, 3]);
        assert_eq!(visible_box(&[0; 16], 2, 2), [0, 0, 2, 2]);
        assert_eq!(visible_box(&[255; 16], 2, 2), [0, 0, 2, 2]);
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum PictureKey {
    /// A title's icon (`icon0.png` and its stand-ins).
    Icon(String),
    /// A box art file in `art/`, which a title shows in place of its icon.
    Art(String),
    /// A title's full-screen picture: the file the user chose in
    /// `backdrops/`, or (`file` empty) `pic0.png` and its stand-ins.
    Backdrop { id: String, file: String },
    /// A picture decoded from memory with [`Host::decode`] (a Find artwork
    /// candidate): its number, and how it was decoded.
    Memory { id: u32, as_: DecodeAs },
}

/// What a running scan is reading (`installed.rs`): one step per app
/// folder, one for the database and PSM, one per PSP folder or disc image,
/// one per RetroArch playlist.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScanPhase {
    Apps,
    System,
    Psm,
    Psp,
    Images,
    Retro,
}

impl ScanPhase {
    /// From the host's `running <done> <total> <phase>` line; Apps when unknown.
    pub fn from_key(key: &str) -> ScanPhase {
        match key {
            "system" => ScanPhase::System,
            "psm" => ScanPhase::Psm,
            "psp" => ScanPhase::Psp,
            "images" => ScanPhase::Images,
            "retro" => ScanPhase::Retro,
            _ => ScanPhase::Apps,
        }
    }
}

/// How a picture in memory is decoded, as the files it would be saved to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DecodeAs {
    /// Box art: fitted whole into a square with clear bars (`art/`).
    Art,
    /// Box art fitted whole into a 128 px square (a libretro box art).
    FittedArt,
    /// A backdrop (`backdrops/`).
    Backdrop,
}

pub enum Event {
    /// A scan is running, with no title list to read: steps done of the
    /// total, and the phase. Only a scan sends it; a start that reads the
    /// saved list goes straight to [`Event::Titles`].
    Scan { done: u32, total: u32, phase: ScanPhase },
    /// The titles, from the saved list or a finished scan, and how long that
    /// took. Hosts make them with [`load_catalog`] on their worker thread:
    /// with thousands of RetroArch games that is tens of milliseconds the
    /// frame loop does not spend.
    ///
    /// [`load_catalog`]: crate::library::catalog::load_catalog
    Titles(Vec<Game>, u32),
    /// A picture asked for with [`Host::request`]; `None` when there is none.
    Picture(PictureKey, Option<Picture>),
    /// A rescan the user asked for could not start (no scan thread); the
    /// titles stay as they were.
    ScanFailed,
    /// A console's libretro names asked for with [`Host::listing`]; `None`
    /// when its file is missing or empty.
    Listing(String, Option<crate::online::libretro::Listing>),
    /// The system's last-launch and install dates by title id, read from the
    /// home screen's database after [`Event::Titles`] at each start (they
    /// change with every launch, so a saved list does not keep them).
    Dates(Vec<(String, crate::library::played::SystemDates)>),
}

/// Whether Adrenaline loads Pocket Shelf's boot plugin, which starts a PSP
/// or PS1 title with no bubble (`pspemu::plugin_state`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PluginState {
    NoAdrenaline,
    /// Not in Adrenaline's plugin lists.
    Missing,
    /// In a list, turned off.
    Off,
    On,
}

/// A network request (`http.rs`): its id while open.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NetId(pub i32);

/// What a request's reply becomes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NetKind {
    /// Text kept in memory (an API reply), with an `Authorization` header
    /// when not empty.
    Text { authorization: String },
    /// A PNG kept in memory, read with [`Host::net_bytes`].
    Bytes,
    /// A file in the data folder: a PNG, a libretro listing
    /// (`libretro/<console>.txt`, the names its page links to), or the
    /// update package.
    Save(String),
    /// A PNG fitted whole into a 128 px square, to `art/<name>.png`.
    SaveIcon(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NetState {
    Busy { received: u64, total: u64 },
    Done { status: u16, received: u64 },
    Error(String),
}

impl NetState {
    /// From the host's `busy <received> <total>`, `done <status>
    /// <received>` or `error <reason>`.
    pub fn parse(line: &str) -> NetState {
        let mut words = line.splitn(3, ' ');
        let kind = words.next().unwrap_or("");
        let a = words.next().unwrap_or("");
        let b = words.next().unwrap_or("");
        match kind {
            "busy" => NetState::Busy { received: a.parse().unwrap_or(0), total: b.parse().unwrap_or(0) },
            "done" => NetState::Done { status: a.parse().unwrap_or(0), received: b.parse().unwrap_or(0) },
            _ => NetState::Error(line.strip_prefix("error ").unwrap_or(line).to_string()),
        }
    }
}

pub trait Host {
    /// Microseconds since the process started.
    fn time_us(&self) -> u64;
    /// A file in the app's data folder, by a path relative to it.
    fn read(&mut self, path: &str) -> Option<Vec<u8>>;
    /// Write a file in the data folder; false when it could not be written.
    /// A host may write later, in order (the Vita writes on a worker thread:
    /// a write to the memory card took 24–30 ms of a frame); `flush` waits
    /// for every write asked for so far.
    fn write(&mut self, path: &str, data: &[u8]) -> bool;
    /// Wait until every write and delete asked for so far is done: before a
    /// launch ends the process.
    fn flush(&mut self) {}
    /// Delete a file in the data folder; true when it is gone (also when it
    /// was not there).
    fn remove(&mut self, path: &str) -> bool;
    /// The names in a folder of the data folder.
    fn list(&mut self, dir: &str) -> Vec<String>;
    fn status(&mut self) -> Status;
    /// Start listing the installed titles; answered with [`Event::Titles`].
    /// The saved list is read when there is one, unless `force` (Rescan
    /// titles), which scans and reports [`Event::Scan`] meanwhile.
    fn scan(&mut self, force: bool);
    /// Ask for a picture; answered with [`Event::Picture`].
    fn request(&mut self, key: PictureKey);
    /// Ask for a backdrop the user may go to next (a neighbor of the
    /// selection), decoded only while no [`request`](Host::request) waits;
    /// answered with [`Event::Picture`]. A host with one queue asks for it
    /// as any other picture.
    fn request_ahead(&mut self, key: PictureKey) {
        self.request(key);
    }
    /// Take back a request not yet started (an icon scrolled past); it may
    /// still be answered if decoding began.
    fn cancel(&mut self, _key: &PictureKey) {}
    /// Answers that arrived since the last call.
    fn poll(&mut self, events: &mut Vec<Event>);
    /// A line for the app's own log.
    fn log(&mut self, line: &str);
    /// Accept a launch of a listed title, sent once the frame that asked is
    /// on screen (the platform's `finish_launch`); false when refused.
    fn launch(&mut self, id: &str) -> bool;
    /// Adrenaline's boot plugin, checked before a title with no bubble
    /// starts.
    fn plugin_state(&mut self) -> PluginState;
    /// Add the plugin to Adrenaline's lists or turn it on there (the user
    /// agreed); true when it is now on.
    fn enable_plugin(&mut self) -> bool;
    /// Start a request on the host's network thread; `None` when it could
    /// not start (no network, too many open). Polled with `net_state`.
    fn net_start(&mut self, _url: &str, _kind: NetKind) -> Option<NetId> {
        None
    }
    fn net_state(&mut self, _id: NetId) -> NetState {
        NetState::Error("no network".into())
    }
    /// A finished text request's reply.
    fn net_text(&mut self, _id: NetId) -> String {
        String::new()
    }
    /// A finished `Bytes` request's reply.
    fn net_bytes(&mut self, _id: NetId) -> Vec<u8> {
        Vec::new()
    }
    /// Cancel and forget a request.
    fn net_close(&mut self, _id: NetId) {}
    /// Decode a PNG in memory; answered with [`Event::Picture`] under `key`
    /// (a [`PictureKey::Memory`]).
    fn decode(&mut self, _key: PictureKey, _png: Vec<u8>) {}
    /// Save a PNG to `art/` or `backdrops/` (fitted into a 128 px square
    /// first with `fit`), in order with the other writes.
    fn save_picture(&mut self, _path: &str, _png: Vec<u8>, _fit: bool) -> bool {
        false
    }
    /// Read `libretro/<console>.txt` and parse it with
    /// [`Listing::parse`](crate::online::libretro::Listing::parse) off the
    /// frame; answered with [`Event::Listing`] under `folder`.
    fn listing(&mut self, _folder: &str) {}

    /// What the last title scan found, as `Label=value` pairs separated by
    /// commas (PSM, PSP and PS1, disc images, RetroArch); "" when this start
    /// read the saved list. Shown on the Diagnostics screen.
    fn scan_report(&mut self) -> String {
        String::new()
    }
    /// The Adrenaline boot plugin's last log lines, newest last.
    fn boot_log(&mut self) -> String {
        String::new()
    }

    // --- The app's own update (`updates.rs`, the Vita's `update.rs`) -------

    /// This build's version, channel and commit.
    fn build(&self) -> crate::updates::BuildInfo {
        crate::updates::BuildInfo::local()
    }
    /// Milliseconds since 1970 (the update check's record); 0 when unknown.
    fn unix_ms(&self) -> u64 {
        0
    }
    /// Whether this build can install its own update. The updater app
    /// installs and starts `POCKTSHLF` only, so a build under another title
    /// id checks but does not download.
    fn can_install_update(&self) -> bool {
        false
    }
    /// Unpack and check the downloaded package on a worker thread; false
    /// while a step runs.
    fn update_unpack(&mut self) -> bool {
        false
    }
    /// Install the updater app on a worker thread; false unless unpacked.
    fn update_install(&mut self) -> bool {
        false
    }
    /// Start the updater once this frame is shown; this process then ends.
    fn update_launch(&mut self) -> bool {
        false
    }
    /// `idle`, `busy <done> <total>`, `done <version>`, `installing`,
    /// `installed` or `error <reason>`.
    fn update_state(&mut self) -> String {
        "error unavailable".into()
    }
    /// The updater's report from its last run, read once at start: "",
    /// `ok` or `error <reason>`.
    fn update_result(&mut self) -> String {
        String::new()
    }
}
