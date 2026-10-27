//! What the app asks of the platform. The Vita implements it on the
//! system's calls; the desktop fakes it with sample data, as
//! `src/tools/preview.ts` does for the current app.
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

    /// The English name (`scanPhases` in `locales/en.ts`).
    pub fn label(self) -> &'static str {
        match self {
            ScanPhase::Apps => "Apps and games",
            ScanPhase::System => "System applications",
            ScanPhase::Psm => "PlayStation Mobile",
            ScanPhase::Psp => "PSP and PSX games",
            ScanPhase::Images => "Disc images",
            ScanPhase::Retro => "RetroArch playlists",
        }
    }
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
    fn scan(&mut self);
    /// Ask for a picture; answered with [`Event::Picture`].
    fn request(&mut self, key: PictureKey);
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
}
