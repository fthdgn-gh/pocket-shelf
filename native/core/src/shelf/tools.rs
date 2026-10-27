//! Rescan titles (`components/rescan.tsx`) and Diagnostics
//! (`components/diagnostics.tsx`), both opened from the SELECT menu.

use super::chrome::Button;
use super::navigation::{Direction, Step as Nav};
use super::panel::{self, Drawer, Step};
use super::Shelf;
use crate::gfx::{Gfx, Rect};
use crate::host::{Host, ScanPhase};
use crate::i18n::{fill, scan_phase, upper};
use crate::library::catalog::Game;

/// Diagnostics rows shown at once.
const DIAGNOSTICS_ROWS: usize = 7;

/// The rescan drawer: what the last scan found, or how far the running one
/// got.
pub struct Rescan {
    pub note: String,
    pub running: bool,
    pub progress: Option<(u32, u32, ScanPhase)>,
}

impl Shelf {
    pub(super) fn open_rescan(&mut self) {
        self.rescan = Some(Rescan { note: String::new(), running: false, progress: None });
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    /// Confirm scans; back closes, not while a scan runs.
    pub(super) fn rescan_input(&mut self, host: &mut dyn Host, confirm: bool, cancel: bool) {
        let Some(rescan) = &mut self.rescan else { return };
        if confirm && !rescan.running {
            rescan.running = true;
            rescan.progress = None;
            rescan.note.clear();
            host.scan(true);
        } else if cancel && !rescan.running {
            self.rescan = None;
        }
    }

    /// A running rescan's progress.
    pub fn scan_progress(&mut self, done: u32, total: u32, phase: ScanPhase) {
        if let Some(rescan) = &mut self.rescan {
            rescan.progress = Some((done, total, phase));
        }
    }

    /// A rescan finished: the new list, and the per-title files read again
    /// (one may have been edited by hand).
    pub fn rescanned(&mut self, host: &mut dyn Host, games: Vec<Game>, ms: u32) {
        self.library.overrides.reload(host);
        let count = games.len();
        self.set_titles(games, false);
        let t = self.t();
        if let Some(rescan) = &mut self.rescan {
            rescan.running = false;
            rescan.progress = None;
            let (count, seconds) = (count.to_string(), format!("{:.1}", ms as f32 / 1000.0));
            rescan.note = fill(t.scan_done, &[("count", &count), ("seconds", &seconds)]);
        }
    }

    pub(super) fn draw_rescan(&mut self, gfx: &mut Gfx) {
        let Some(rescan) = &self.rescan else { return };
        let (running, progress, note) = (rescan.running, rescan.progress, rescan.note.clone());
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.rescan, self.settings.language), panel::Size::Wide, slide);
        let count = fill(t.scrape_count_all, &[("count", &self.library.games().len().to_string())]);
        drawer.row(gfx, t.scan, Some(&count), None, true, Step::Right);
        if running {
            let (done, total, phase) = progress.unwrap_or((0, 0, ScanPhase::Apps));
            let line = fill(t.scan_step, &[("phase", scan_phase(t, phase)), ("done", &done.to_string()), ("total", &total.to_string())]);
            drawer.note(gfx, &line, 0);
            // The bar above the step's line.
            let y = drawer.bottom - gfx.line_height(crate::text::Role::Small) - 2.0 - 12.0;
            let bar = Rect::new(drawer.x, y, 240.0, 6.0);
            gfx.round_rect(bar, 3.0, 0.0, theme.line);
            if total > 0 {
                let filled = (240.0 * done.min(total) as f32 / total as f32).round();
                if filled > 0.0 {
                    gfx.round_rect(Rect::new(bar.x, y, filled, 6.0), 3.0, 0.0, theme.accent);
                }
            }
        } else {
            let text = if note.is_empty() { t.scan_help.to_string() } else { note };
            drawer.note(gfx, &text, 1);
            drawer.hints(gfx, &[(Button::confirm(self.settings.confirm), t.scan), (Button::cancel(self.settings.confirm), t.back)]);
        }
    }

    pub(super) fn open_diagnostics(&mut self) {
        self.diagnostics = Some(0);
        self.reset_stats = true;
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    pub fn diagnostics_open(&self) -> bool {
        self.diagnostics.is_some()
    }

    /// Up from the first line stays there; the list wraps going down.
    pub(super) fn diagnostics_input(&mut self, step: Option<Nav>, cancel: bool) {
        let Some(row) = &mut self.diagnostics else { return };
        match step {
            Some(Nav::Press(Direction::Up)) | Some(Nav::Repeat(Direction::Up, _)) => *row = row.saturating_sub(1),
            Some(Nav::Press(Direction::Down)) | Some(Nav::Repeat(Direction::Down, _)) => *row += 1,
            _ => {}
        }
        if cancel {
            self.diagnostics = None;
        }
    }

    pub(super) fn draw_diagnostics(&mut self, gfx: &mut Gfx) {
        let Some(row) = self.diagnostics else { return };
        let lines = self.diagnostics_lines.clone();
        if lines.is_empty() {
            return;
        }
        let row = row % lines.len();
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.diagnostics, self.settings.language), panel::Size::Wide, slide);
        let first = row.saturating_sub(DIAGNOSTICS_ROWS / 2).min(lines.len().saturating_sub(DIAGNOSTICS_ROWS));
        for (i, (label, value)) in lines.iter().enumerate().skip(first).take(DIAGNOSTICS_ROWS) {
            drawer.row(gfx, label, Some(value), None, i == row, Step::None);
        }
        drawer.hints(gfx, &[(Button::DpadVertical, t.navigate), (Button::cancel(self.settings.confirm), t.back)]);
    }
}
