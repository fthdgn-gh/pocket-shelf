//! Touch on the shelf and its panels: a tap acts on what the last frame drew
//! under it (`crate::touch::Hit`), a drag moves the selection or the open
//! panel's highlight.

use super::layout::{grid, home, shelf, BAR_H, GRID_COLUMNS, LIST_GAP, LIST_ROW_H};
use super::chrome::FOOTER_H;
use super::navigation::{Direction, Step};
use super::{panel, Shelf};
use crate::gfx::Rect;
use crate::host::Host;
use crate::library::settings::ViewMode;
use crate::touch::{hit_at, Gesture, Hit};

/// A drawer row's height with the space after it: a drag this long moves
/// the highlight one row.
const ROW_PITCH: f32 = panel::ROW_H + 4.0;

/// What a frame's touch asks of the shelf, besides the buttons.
pub struct TouchFrame<'a> {
    pub gesture: Gesture,
    /// The last frame's places to tap.
    pub hits: &'a [(Rect, Hit)],
}

impl TouchFrame<'_> {
    /// No touch.
    pub const NONE: TouchFrame<'static> = TouchFrame { gesture: Gesture { tap: None, moved: (0.0, 0.0), from: None, down: false, hold: None }, hits: &[] };
}

impl Shelf {
    /// Whether a panel, the menu or the keyboard is over the shelf.
    pub(super) fn panel_open(&self) -> bool {
        self.keyboard.is_some()
            || self.update.open
            || self.online.is_some()
            || self.fetch.is_some()
            || self.cleanup.is_some()
            || self.rescan.is_some()
            || self.diagnostics.is_some()
            || self.picker.is_some()
            || self.editor.is_some()
            || self.categories.is_some()
            || self.adrenaline.is_some()
            || self.menu.open
    }

    /// Apply a frame's touch: a tap becomes the buttons `pressed` and the
    /// `step` it stands for, after moving a highlight or a selection; a drag
    /// moves the selection or the open panel's highlight.
    pub(super) fn apply_touch(&mut self, host: &mut dyn Host, touch: &TouchFrame, pressed: &mut u32, step: &mut Option<Step>, confirm: u32, cancel: u32) {
        let gesture = touch.gesture;
        if let Some((x, y)) = gesture.tap {
            match hit_at(touch.hits, x, y).cloned() {
                Some(Hit::Button(bit)) => *pressed |= bit,
                Some(Hit::Row(row)) => {
                    if self.touch_row(host, row) {
                        *pressed |= confirm;
                    }
                }
                Some(Hit::Arrow(row, delta)) => {
                    self.set_panel_row(host, row);
                    *step = Some(Step::Press(if delta < 0 { Direction::Left } else { Direction::Right }));
                }
                Some(Hit::Tile(index)) if !self.panel_open() => {
                    // The selected tile starts its title; another is
                    // selected first.
                    if index == self.selected {
                        *pressed |= confirm;
                    } else if index < self.list.len() {
                        self.select(index);
                    }
                }
                Some(Hit::Tab(id)) if !self.panel_open() => self.open_tab(&id),
                Some(Hit::Key(row, col)) => {
                    if let Some(keyboard) = &mut self.keyboard {
                        // A tap beside a letter's accents closes them.
                        if !keyboard.close_accents() {
                            keyboard.focus(row, col);
                            *pressed |= confirm;
                        }
                    }
                }
                Some(Hit::Accent(index)) => {
                    if let Some(accents) = self.keyboard.as_mut().and_then(|k| k.accents.as_mut()) {
                        accents.index = index.min(accents.keys.len() - 1);
                        *pressed |= confirm;
                    }
                }
                Some(Hit::Back) => *pressed |= cancel,
                _ => {}
            }
        }
        // A finger held on a letter: the letter's accents (typed, then
        // replaced, as a held confirm button does).
        if let (Some((x, y)), Some(keyboard)) = (gesture.hold, &mut self.keyboard) {
            if let Some(Hit::Key(row, col)) = hit_at(touch.hits, x, y).cloned() {
                keyboard.focus(row, col);
                if keyboard.accents.is_none() && keyboard.key().action == super::keyboard::Action::Char {
                    keyboard.press();
                    keyboard.hold();
                }
            }
        }
        if gesture.down && gesture.moved == (0.0, 0.0) && gesture.from.is_none() {
            // A finger touching down: what it drags starts over.
            self.drag_left = 0.0;
        }
        let Some(from) = gesture.from else { return };
        if gesture.moved == (0.0, 0.0) {
            return;
        }
        if self.keyboard.is_some() {
            return;
        }
        if self.panel_open() {
            // Up the screen shows the rows below: the highlight goes down.
            self.drag_left -= gesture.moved.1;
            let rows = (self.drag_left / ROW_PITCH).trunc();
            if rows != 0.0 {
                self.drag_left -= rows * ROW_PITCH;
                if let Some((row, count)) = self.panel_row() {
                    let target = (row as i64 + rows as i64).clamp(0, count.saturating_sub(1) as i64) as usize;
                    if target != row {
                        self.set_panel_row(host, target);
                    }
                }
            }
            return;
        }
        // A drag along the tab names moves between tabs.
        if from.1 < BAR_H {
            self.drag_tabs(gesture.moved.0);
            return;
        }
        // The titles: a drag that starts between the bar and the footer.
        if from.1 > crate::H - FOOTER_H {
            return;
        }
        let (along, pitch, unit) = self.drag_axis(gesture.moved);
        self.drag_left -= along;
        let steps = (self.drag_left / pitch).trunc();
        if steps != 0.0 && !self.list.is_empty() {
            self.drag_left -= steps * pitch;
            let target = (self.selected as i64 + steps as i64 * unit).clamp(0, self.list.len() as i64 - 1) as usize;
            self.select(target);
        }
    }

    /// The movement along the view's axis, the distance that moves the
    /// selection one step, and how many titles a step is.
    fn drag_axis(&self, moved: (f32, f32)) -> (f32, f32, i64) {
        let detail = self.settings.detail;
        match self.settings.view {
            ViewMode::Home => {
                let h = home(detail);
                (moved.0, h.small + h.gap, 1)
            }
            ViewMode::Carousel => {
                let s = shelf(detail);
                (moved.0, s.tile + s.gap, 1)
            }
            // The receding tiles are narrow: a step a third of the front
            // tile.
            ViewMode::Cascade => (moved.0, super::layout::cascade(detail).0 / 3.0, 1),
            ViewMode::Grid => {
                let (tile, gap) = grid(detail);
                (moved.1, tile + gap, GRID_COLUMNS as i64)
            }
            ViewMode::List => (moved.1, LIST_ROW_H + LIST_GAP, 1),
        }
    }

    /// A drag along the top bar: a tab further each time the finger moves
    /// the distance between two names (leftward, the next tab). Stops at
    /// the first and the last tab, where L and R wrap.
    fn drag_tabs(&mut self, moved: f32) {
        self.drag_left -= moved;
        loop {
            let delta = if self.drag_left > 0.0 { 1 } else if self.drag_left < 0.0 { -1 } else { return };
            let Some(pitch) = self.header.pitch(&self.tab, delta).filter(|p| *p > 0.0) else {
                // An end: the rest of the drag is dropped.
                self.drag_left = 0.0;
                return;
            };
            if self.drag_left.abs() < pitch {
                return;
            }
            self.drag_left -= delta as f32 * pitch;
            self.change_category(delta);
        }
    }

    /// The tab with this id, as L and R reach it.
    fn open_tab(&mut self, id: &str) {
        let ids: Vec<&str> = self.tabs.iter().map(|c| c.id.as_str()).collect();
        let (Some(to), Some(from)) = (ids.iter().position(|t| *t == id), ids.iter().position(|t| *t == self.tab)) else { return };
        if to != from {
            self.change_category(to as i32 - from as i32);
        }
    }

    /// A tapped drawer row: highlight it; true when it is then confirmed. A
    /// picker's row is only highlighted by a first tap, which shows its
    /// picture, and confirmed by a second.
    fn touch_row(&mut self, host: &mut dyn Host, row: usize) -> bool {
        if self.keyboard.is_none() && self.picker.is_some() && !self.update.open && self.online.is_none() {
            let current = self.picker.as_ref().map_or(0, |p| p.row);
            if current != row {
                self.set_panel_row(host, row);
                return false;
            }
            return true;
        }
        self.set_panel_row(host, row);
        true
    }

    /// The open panel's highlighted row and its number of rows; `None` for
    /// a panel without a list. In the order `update` gives panels the
    /// buttons.
    fn panel_row(&self) -> Option<(usize, usize)> {
        if self.keyboard.is_some() || self.update.open {
            return None;
        }
        if let Some(online) = &self.online {
            return match online.step {
                crate::online::flow::Step::Games => Some((online.row, online.games.len() + 1)),
                crate::online::flow::Step::Assets => Some((online.row, 2)),
                crate::online::flow::Step::Key => None,
            };
        }
        if let Some(panel) = &self.fetch {
            return (panel.flow.step == crate::online::fetch::Step::Setup).then_some((panel.flow.row, crate::online::fetch::SETUP_ROWS));
        }
        if let Some(clean) = &self.cleanup {
            return Some((clean.row(), 2));
        }
        if self.rescan.is_some() {
            return None;
        }
        if let Some(row) = self.diagnostics {
            let count = self.diagnostics_lines.len().max(1);
            return Some((row % count, count));
        }
        if let Some(picker) = &self.picker {
            return Some((picker.row, self.picker_count() + 2));
        }
        if let Some(editor) = &self.editor {
            return Some((editor.row, super::editor::EDITOR_ROWS));
        }
        if let Some(manager) = &self.categories {
            return Some((manager.row, self.library.all_categories().len() + 1));
        }
        if self.adrenaline.is_some() {
            return None;
        }
        if self.menu.open {
            return Some((self.menu.row, super::menu::items(self.menu.page).len()));
        }
        None
    }

    /// Highlight a row of the open panel as the buttons would, with what a
    /// new row brings (a picker's preview, a Find artwork candidate).
    fn set_panel_row(&mut self, host: &mut dyn Host, row: usize) {
        if self.keyboard.is_some() || self.update.open {
            return;
        }
        if let Some(online) = &mut self.online {
            let t = crate::i18n::messages(self.settings.language);
            let dy = row as i32 - online.row as i32;
            if dy != 0 {
                let actions = online.step_by(host, t, 0, dy);
                self.online_actions(host, actions);
            }
            return;
        }
        if let Some(panel) = &mut self.fetch {
            if panel.flow.step == crate::online::fetch::Step::Setup {
                panel.flow.row = row.min(crate::online::fetch::SETUP_ROWS - 1);
            }
            return;
        }
        if let Some(clean) = &mut self.cleanup {
            clean.set_row(row.min(1));
            return;
        }
        if self.rescan.is_some() {
            return;
        }
        if let Some(current) = &mut self.diagnostics {
            *current = row;
            return;
        }
        if self.picker.is_some() {
            self.picker_focus(row);
            return;
        }
        if let Some(editor) = &mut self.editor {
            editor.row = row.min(super::editor::EDITOR_ROWS - 1);
            return;
        }
        if self.categories.is_some() {
            let count = self.library.all_categories().len() + 1;
            if let Some(manager) = &mut self.categories {
                if manager.row != row {
                    manager.row = row.min(count - 1);
                    manager.armed = None;
                    manager.note.clear();
                }
            }
            return;
        }
        if self.adrenaline.is_some() {
            return;
        }
        if self.menu.open {
            self.menu.row = row.min(super::menu::items(self.menu.page).len() - 1);
        }
    }
}
