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
    /// A title's full-screen picture (`pic0.png` and its stand-ins).
    Backdrop(String),
}

pub enum Event {
    /// The scan finished: the titles and how long it took.
    Titles(Vec<TitleInfo>, u32),
    /// A picture asked for with [`Host::request`]; `None` when there is none.
    Picture(PictureKey, Option<Picture>),
}

pub trait Host {
    /// Microseconds since the process started.
    fn time_us(&self) -> u64;
    /// A file in the app's data folder, by a path relative to it.
    fn read(&mut self, path: &str) -> Option<Vec<u8>>;
    /// Write a file in the data folder; false when it could not be written.
    fn write(&mut self, path: &str, data: &[u8]) -> bool;
    /// The names in a folder of the data folder.
    fn list(&mut self, dir: &str) -> Vec<String>;
    fn status(&mut self) -> Status;
    /// Start listing the installed titles; answered with [`Event::Titles`].
    fn scan(&mut self);
    /// Ask for a picture; answered with [`Event::Picture`].
    fn request(&mut self, key: PictureKey);
    /// Answers that arrived since the last call.
    fn poll(&mut self, events: &mut Vec<Event>);
    /// A line for the app's own log.
    fn log(&mut self, line: &str);
}
