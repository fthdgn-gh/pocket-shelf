//! The app: on a first start the scan page, then the shelf. It loads the
//! saved settings and the library on its first frame and takes the host's
//! answers.

use crate::gfx::{Color, Gfx, Rect, TexId};
use crate::host::{Event, Host, ScanPhase};
use crate::library::settings::{Settings, UpdateChannel, SETTINGS_FILE};
use crate::library::Library;
use crate::shelf::{Request, Shelf};
use crate::stats::FrameStats;
use crate::text::Role;
use crate::theme::{theme_by_id, Theme};
use crate::{H, W};

/// Button bits, filled in by each platform from its own input.
pub mod buttons {
    pub const UP: u32 = 1 << 0;
    pub const DOWN: u32 = 1 << 1;
    pub const LEFT: u32 = 1 << 2;
    pub const RIGHT: u32 = 1 << 3;
    pub const CROSS: u32 = 1 << 4;
    pub const CIRCLE: u32 = 1 << 5;
    pub const SQUARE: u32 = 1 << 6;
    pub const TRIANGLE: u32 = 1 << 7;
    pub const START: u32 = 1 << 8;
    pub const SELECT: u32 = 1 << 9;
    pub const L: u32 = 1 << 10;
    pub const R: u32 = 1 << 11;
}

/// Frames the full scan bar stays before the shelf replaces the page.
const SCAN_DONE_FRAMES: u32 = 8;
const SCAN_BAR_W: f32 = 240.0;

/// The scan page's state: steps done of the total, the phase, and frames
/// since the titles arrived.
struct Scan {
    done: u32,
    total: u32,
    phase: Option<ScanPhase>,
    finished: Option<u32>,
}

pub struct App {
    shelf: Option<Shelf>,
    /// Whether the host's titles have arrived.
    titled: bool,
    scan: Option<Scan>,
    /// The scan page's theme and language, from the saved settings.
    theme: Theme,
    language: crate::library::settings::Language,
    background: Option<TexId>,
    /// The mark's three tile gradients, made on first use.
    mark: Option<[TexId; 3]>,
    events: Vec<Event>,
    frame: u32,
    /// What the last frame asks the platform to do once it is shown.
    pub request: Option<Request>,
    /// Startup marks for Diagnostics, µs of process time and ms.
    loaded_ms: u64,
    titles_ms: u32,
    titles_at: u64,
    rebuild_us: u64,
    first_frame_at: u64,
    pub stats: FrameStats,
    pub quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        App {
            shelf: None,
            titled: false,
            scan: None,
            theme: theme_by_id(Settings::defaults(UpdateChannel::Off).theme),
            language: crate::library::settings::Language::En,
            background: None,
            mark: None,
            events: Vec::new(),
            frame: 0,
            request: None,
            loaded_ms: 0,
            titles_ms: 0,
            titles_at: 0,
            rebuild_us: 0,
            first_frame_at: 0,
            stats: FrameStats::default(),
            quit: false,
        }
    }

    /// Titles in the library.
    pub fn len(&self) -> usize {
        self.shelf.as_ref().map_or(0, |s| s.games().len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The shelf, once the library is loaded.
    pub fn shelf(&mut self) -> Option<&mut Shelf> {
        self.shelf.as_mut()
    }

    /// Take the host's answers. Returns how many there were.
    pub fn events(&mut self, host: &mut dyn Host, gfx: &mut Gfx) -> usize {
        let mut events = std::mem::take(&mut self.events);
        host.poll(&mut events);
        let count = events.len();
        for event in events.drain(..) {
            match event {
                Event::Scan { done, total, phase } => match &mut self.shelf {
                    // With titles on screen it is a rescan: its drawer shows it.
                    Some(shelf) if self.titled => shelf.scan_progress(done, total, phase),
                    _ => self.scan = Some(Scan { done, total, phase: Some(phase), finished: None }),
                },
                Event::Titles(titles, ms) => {
                    host.log(&format!("titles {} in {ms} ms", titles.len()));
                    if let Some(scan) = &mut self.scan {
                        scan.done = scan.total.max(1);
                        scan.total = scan.done;
                        scan.finished = Some(0);
                    }
                    if let Some(shelf) = &mut self.shelf {
                        let started = host.time_us();
                        if self.titled {
                            shelf.rescanned(host, titles, ms);
                        } else {
                            shelf.set_titles(titles, true);
                            self.titles_ms = ms;
                            self.titles_at = host.time_us();
                        }
                        self.rebuild_us = host.time_us() - started;
                        host.log(&format!("library: {} titles in {} us", shelf.games().len(), self.rebuild_us));
                    }
                    self.titled = true;
                }
                Event::Picture(key, picture) => {
                    if let Some(shelf) = &mut self.shelf {
                        shelf.picture(gfx, key, picture);
                    }
                }
                Event::Listing(folder, listing) => {
                    if let Some(shelf) = &mut self.shelf {
                        shelf.listing(host, folder, listing);
                    }
                }
            }
        }
        self.events = events;
        count
    }

    pub fn update(&mut self, host: &mut dyn Host, pad: u32) {
        let now = host.time_us();
        if self.frame == 0 {
            // A fresh install checks the channel it was released on.
            let build = host.build();
            let mut settings = Settings::defaults(build.default_channel());
            if let Some(bytes) = host.read(SETTINGS_FILE) {
                settings = Settings::parse(&bytes, settings);
            }
            if let Some(bytes) = host.read(crate::library::settings::STATUS_BAR_FILE) {
                settings.layout = crate::library::settings::StatusLayout::parse(&bytes);
            }
            self.theme = theme_by_id(settings.theme);
            self.language = settings.language;
            let library = Library::load(host);
            self.loaded_ms = (host.time_us() - now) / 1000;
            self.first_frame_at = now;
            host.log(&format!("library loaded in {} ms", self.loaded_ms));
            // The first status reading is slow on hardware (about 25 ms), so
            // it is taken here, in the frame that loads the library.
            let status = host.status();
            let mut shelf = Shelf::new(settings, library, status);
            shelf.build = build;
            self.shelf = Some(shelf);
            host.scan(false);
        }
        self.frame += 1;
        self.request = None;
        if let Some(scan) = &mut self.scan {
            if let Some(frames) = &mut scan.finished {
                *frames += 1;
                if *frames >= SCAN_DONE_FRAMES {
                    self.scan = None;
                }
            }
            // The page takes no buttons.
            return;
        }
        // Until the titles arrive the shelf shows "Reading titles".
        if !self.titled {
            return;
        }
        if let Some(shelf) = &mut self.shelf {
            self.request = shelf.update(host, pad, now);
            // Diagnostics counts frames from when it opens.
            if std::mem::take(&mut shelf.reset_stats) {
                self.stats.reset();
            }
        }
    }

    /// Diagnostics' lines: frame timing
    /// since the screen opened, then this start's timing and the library.
    fn diagnostics(&self, gfx: &Gfx) -> Vec<(String, String)> {
        let s = &self.stats;
        let ms = |us: u64| format!("{} ms", us / 1000);
        vec![
            ("Frames".into(), s.frames.to_string()),
            ("Frames over 20 ms".into(), s.slow.to_string()),
            ("CPU per frame".into(), format!("{:.1} / {:.1} ms", s.cpu_ms(), s.longest_cpu_us as f32 / 1000.0)),
            ("Frame time".into(), format!("{:.1} / {:.1} ms", s.frame_ms(), s.longest_frame_us as f32 / 1000.0)),
            ("Textures".into(), format!("{} KB", gfx.textures.live_bytes() / 1024)),
            ("Glyph atlas restarts".into(), gfx.atlas_resets().to_string()),
            ("Titles".into(), self.len().to_string()),
            ("App code begins".into(), ms(self.first_frame_at)),
            ("Library loaded".into(), format!("{} ms", self.loaded_ms)),
            ("Title list".into(), format!("{} ms", self.titles_ms)),
            ("Titles on screen".into(), ms(self.titles_at)),
            ("Library rebuild".into(), format!("{:.1} ms", self.rebuild_us as f32 / 1000.0)),
            ("Build".into(), format!("{} (next)", env!("CARGO_PKG_VERSION"))),
        ]
    }

    pub fn draw(&mut self, gfx: &mut Gfx) {
        gfx.begin();
        if self.scan.is_some() || !self.titled {
            let theme = self.theme;
            let background = *self.background.get_or_insert_with(|| gfx.gradient(theme.bg_top, theme.bg_bottom));
            gfx.image_full(background, Rect::new(0.0, 0.0, W, H), Color::rgb(0xffffff));
            if self.scan.is_some() {
                self.draw_scan(gfx);
            }
            return;
        }
        let lines = match &self.shelf {
            Some(shelf) if shelf.diagnostics_open() => Some(self.diagnostics(gfx)),
            _ => None,
        };
        if let Some(shelf) = &mut self.shelf {
            if let Some(lines) = lines {
                shelf.diagnostics_lines = lines;
            }
            shelf.draw(gfx);
        }
    }

    /// The first start's page: the mark, the
    /// name, what is happening, a bar and the phase with its count.
    fn draw_scan(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let Some(scan) = &self.scan else { return };
        let (done, total, phase, finished) = (scan.done, scan.total, scan.phase, scan.finished);
        // A column of: mark 48, a 2 unit gap, heading, body, bar 6, small
        // line, 10 apart, centered.
        let lines = [gfx.line_height(Role::Heading), gfx.line_height(Role::Body), gfx.line_height(Role::Small)];
        let height = 48.0 + 2.0 + lines.iter().sum::<f32>() + 6.0 + 10.0 * 5.0;
        let mut y = ((H - height) / 2.0).round();
        self.draw_mark(gfx, (W - 110.0) / 2.0, y);
        y += 48.0 + 10.0 + 2.0 + 10.0;
        gfx.text_top_centered(Role::Heading, W / 2.0, y, theme.text, "Pocket Shelf");
        y += lines[0] + 10.0;
        let t = crate::i18n::messages(self.language);
        gfx.text_top_centered(Role::Body, W / 2.0, y, theme.dim, t.scan_title);
        y += lines[1] + 10.0;
        let bar = Rect::new((W - SCAN_BAR_W) / 2.0, y, SCAN_BAR_W, 6.0);
        gfx.round_rect(bar, 3.0, 0.0, theme.line);
        if total > 0 {
            let filled = (SCAN_BAR_W * done.min(total) as f32 / total as f32).round();
            if filled > 0.0 {
                gfx.round_rect(Rect::new(bar.x, y, filled, 6.0), 3.0, 0.0, theme.accent);
            }
        }
        y += 6.0 + 10.0;
        if let (Some(phase), None) = (phase, finished) {
            let (done, total) = (done.to_string(), total.to_string());
            let step = crate::i18n::fill(t.scan_step, &[("phase", crate::i18n::scan_phase(t, phase)), ("done", &done), ("total", &total)]);
            gfx.text_top_centered(Role::Small, W / 2.0, y, theme.faint, &step);
        }
    }

    /// The app's mark: the Home view in small, a selected tile in its ring
    /// and two smaller tiles after it (`MARK` in `tools/src/art.rs`).
    fn draw_mark(&mut self, gfx: &mut Gfx, x: f32, y: f32) {
        let [amber, emerald, violet] = *self.mark.get_or_insert_with(|| {
            [
                gfx.gradient(Color::rgb(0xfbbf24), Color::rgb(0xdc2626)),
                gfx.gradient(Color::rgb(0x34d399), Color::rgb(0x0f766e)),
                gfx.gradient(Color::rgb(0xa78bfa), Color::rgb(0x5b21b6)),
            ]
        });
        let white = Color::rgb(0xffffff);
        gfx.image_full(amber, Rect::new(x + 52.0, y + 4.0, 26.0, 26.0), white.alpha(0.8));
        gfx.image_full(emerald, Rect::new(x + 84.0, y + 4.0, 26.0, 26.0), white.alpha(0.8));
        gfx.image_full(violet, Rect::new(x + 4.0, y + 4.0, 40.0, 40.0), white);
        gfx.round_rect(Rect::new(x, y, 48.0, 48.0), 3.0, 2.0, self.theme.text);
    }
}
