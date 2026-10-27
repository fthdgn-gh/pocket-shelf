//! The five views: Home, Carousel, Grid, List and Cascade,
//! each drawing the tiles near the selection from the shelf's state, and
//! the title's details under them.

use super::art::{selected_frame, tile_shadow};
use super::chrome::{Button, FOOTER_H};
use super::layout::*;
use super::{Shelf, Slot, TileAnim};
use crate::gfx::{Gfx, Rect};
use crate::library::catalog::Game;
use crate::library::settings::{DetailLevel, ViewMode};
use crate::shapes::Symbol;
use crate::text::Role;
use crate::{H, W};

/// Top of the view: under the top bar.
const TOP: f32 = BAR_H;
/// The view's height: between the bar and the footer.
const VIEW_H: f32 = H - BAR_H - FOOTER_H;

/// Tiles kept on each side of the carousel's selected tile.
const SIDE_TILES: usize = 5;

impl Shelf {
    /// Start over when the view, the details or the tab changed: tiles
    /// appear in their places and the row makes its entrance.
    fn place(&mut self) {
        let key = (self.settings.view, self.settings.detail, self.tab.clone());
        if self.placed.as_ref() == Some(&key) {
            return;
        }
        self.placed = Some(key);
        self.tiles.clear();
        let ms = match self.settings.view {
            ViewMode::Home => 240,
            ViewMode::Cascade => 260,
            ViewMode::Carousel => 220,
            ViewMode::Grid | ViewMode::List => 1,
        };
        self.intro = super::Tween::new(0.0, ms);
        self.intro.set(1.0, self.now);
        let detail = self.settings.detail;
        match self.settings.view {
            ViewMode::Carousel => {
                let (pitch, center) = carousel_layout(W, detail);
                self.strip = super::Tween::new(center - self.selected as f32 * pitch, 220);
            }
            ViewMode::Grid => {
                let (tile, gap) = grid(detail);
                self.top_row = (self.selected / GRID_COLUMNS).saturating_sub(GRID_ROWS - 1);
                self.strip = super::Tween::new(self.top_row as f32 * (tile + gap), 180);
            }
            ViewMode::List => {
                self.top_row = self.selected.saturating_sub(LIST_ROWS - 1);
                self.strip = super::Tween::new(self.top_row as f32 * (LIST_ROW_H + LIST_GAP), 150);
            }
            _ => {}
        }
    }

    /// The tile at `index` eased toward `slot`; a tile new to the screen
    /// starts in its place.
    fn tile(&mut self, index: usize, slot: Slot, ms: u32) -> Slot {
        let now = self.now;
        self.tiles.entry(index).or_insert_with(|| TileAnim::new(slot, ms)).step(slot, now)
    }

    /// Drop the motion of tiles outside `from..to`.
    fn keep_tiles(&mut self, from: usize, to: usize) {
        self.tiles.retain(|&i, _| i >= from && i < to);
    }

    pub(super) fn draw_view(&mut self, gfx: &mut Gfx) {
        if self.list.is_empty() {
            self.draw_empty(gfx);
            return;
        }
        self.place();
        match self.settings.view {
            ViewMode::Home => self.draw_home(gfx),
            ViewMode::Carousel => self.draw_carousel(gfx),
            ViewMode::Grid => self.draw_grid(gfx),
            ViewMode::List => self.draw_list(gfx),
            ViewMode::Cascade => self.draw_cascade(gfx),
        }
    }

    /// A tab with no titles: what is missing and, with titles elsewhere,
    /// how to move one here.
    fn draw_empty(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        if self.tab == crate::library::categories::SEARCH_ID {
            // A search that found nothing: its term, then search again or
            // close.
            let t = self.t();
            let line = gfx.line_height(Role::Title);
            let top = TOP + VIEW_H / 2.0 - (line + 4.0 + 16.0) / 2.0;
            let term = gfx.fit(Role::Title, &self.library.search_term, 180.0);
            gfx.text_top_centered(Role::Title, W / 2.0, top, theme.text, &crate::i18n::fill(t.no_match, &[("term", &term)]));
            let items = [(Button::Face(Symbol::Square), t.search_again), (Button::cancel(self.settings.confirm), t.close)];
            super::chrome::hints_centered(gfx, &theme, &items, W / 2.0, top + line + 4.0, W - 32.0);
            return;
        }
        let has_titles = !self.library.games().is_empty();
        let line = gfx.line_height(Role::Title);
        let cy = TOP + VIEW_H / 2.0;
        let top = if has_titles { cy - (line + 4.0 + 16.0) / 2.0 } else { cy - line / 2.0 };
        let t = self.t();
        let text = if has_titles { t.empty_category } else { t.no_titles };
        gfx.text_top_centered(Role::Title, W / 2.0, top, theme.text, text);
        if has_titles {
            // "Press {button} on a title to move it here", the button inline.
            let (before, after) = t.move_here.split_once("{button}").unwrap_or((t.move_here, ""));
            let width = gfx.measure(Role::Small, before) + 20.0 + gfx.measure(Role::Small, after);
            let y = top + line + 4.0;
            let baseline = (y + 8.0 - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
            let mut x = W / 2.0 - width / 2.0;
            x = gfx.text(Role::Small, x, baseline, theme.dim, before.trim_end()) + 2.0;
            Button::Face(Symbol::Triangle).draw(gfx, x, y);
            gfx.text(Role::Small, x + 18.0, baseline, theme.dim, after.trim_start());
        }
    }

    /// The selected title's name and, at "Detailed", its id and category,
    /// centered on `cx` (or from `left` when given), top at `top`.
    fn draw_info(&mut self, gfx: &mut Gfx, role: Role, top: f32, width: f32, left: Option<f32>) {
        if self.settings.detail == DetailLevel::Basic {
            return;
        }
        let theme = self.theme;
        let Some(game) = self.selected_game().cloned() else { return };
        let title = gfx.fit(role, &game.title, width);
        let line = gfx.line_height(role);
        let detail = (self.settings.detail == DetailLevel::Detailed).then(|| format!("{}  ·  {}", game.shown_id(), self.category_label(&game)));
        match left {
            Some(x) => {
                gfx.text_top(role, x, top, theme.text, &title);
                if let Some(detail) = detail {
                    gfx.text_top(Role::Small, x, top + line + 2.0, theme.dim, &detail);
                }
            }
            None => {
                gfx.text_top_centered(role, W / 2.0, top, theme.text, &title);
                if let Some(detail) = detail {
                    gfx.text_top_centered(Role::Small, W / 2.0, top + line + 2.0, theme.dim, &detail);
                }
            }
        }
    }

    fn category_label(&self, game: &Game) -> String {
        let t = self.t();
        self.library
            .all_categories()
            .iter()
            .find(|c| *c.id == *game.category)
            .map_or(game.category.to_string(), |c| crate::i18n::category_label(t, c))
    }

    /// Draw a tile's art and, for the selected one, its ring.
    fn draw_tile(&mut self, gfx: &mut Gfx, index: usize, rect: Rect, side: f32, opacity: f32, selected: bool, shadow: bool) {
        let Some(&game_index) = self.list.get(index) else { return };
        if opacity <= 0.0 {
            return;
        }
        let k = rect.w / side;
        if selected {
            if shadow {
                tile_shadow(gfx, rect, opacity);
            }
            selected_frame(gfx, rect, k, self.theme.text.alpha(opacity));
        }
        let game = &self.library.games()[game_index];
        self.art.draw(gfx, game, rect, side, opacity, self.settings.icon_box, false);
    }

    /// Home, after the PS5 home screen: the selected
    /// title large at the left, the next ones smaller after it, its name, a
    /// line about it and a start button below.
    fn draw_home(&mut self, gfx: &mut Gfx) {
        let detail = self.settings.detail;
        let Home { tile, .. } = home(detail);
        let row_h = tile + 16.0;
        let intro = self.intro.at(self.now);
        let shift = 24.0 * (1.0 - intro);
        let from = self.selected.saturating_sub(1);
        let to = (self.selected + HOME_AFTER as usize + 1).min(self.list.len());
        self.keep_tiles(from, to);
        // Farthest first: the selected tile is in front.
        let mut order: Vec<usize> = (from..to).collect();
        order.sort_by_key(|&i| std::cmp::Reverse((i as i64 - self.selected as i64).abs()));
        for index in order {
            let offset = index as i32 - self.selected as i32;
            let s = home_slot(detail, offset);
            let slot = self.tile(index, Slot { x: s.x, y: 0.0, scale: s.scale, opacity: s.opacity, shade: 0.0 }, 200);
            let side = tile * slot.scale;
            let rect = Rect::new(slot.x + shift, TOP + 8.0, side, side);
            self.draw_tile(gfx, index, rect, tile, slot.opacity * intro, offset == 0, true);
        }
        if detail == DetailLevel::Basic {
            return;
        }
        let theme = self.theme;
        let Some(game) = self.selected_game().cloned() else { return };
        let text_w = W - HOME_LEFT * 2.0;
        let mut y = TOP + row_h + 6.0;
        let title = gfx.fit(Role::Display, &game.title, text_w);
        gfx.text_top(Role::Display, HOME_LEFT, y, theme.text, &title);
        y += gfx.line_height(Role::Display) + 4.0;
        let mut parts = vec![game.platform.genre().to_string(), self.category_label(&game)];
        if detail == DetailLevel::Detailed {
            parts.push(game.shown_id().to_string());
        }
        let first = parts[0].clone();
        let line: Vec<String> = parts.into_iter().enumerate().filter(|(i, p)| *i == 0 || *p != first).map(|(_, p)| p).collect();
        let line = gfx.fit(Role::Small, &line.join("  ·  "), text_w);
        gfx.text_top(Role::Small, HOME_LEFT, y, theme.dim, &line);
        y += gfx.line_height(Role::Small) + 4.0 + 4.0 + 4.0;
        // The start button: the confirm button and "Launch" on a pill in the
        // text color.
        let launch = self.t().launch;
        let label_w = gfx.measure(Role::BodyBold, launch);
        let pill = Rect::new(HOME_LEFT, y, 6.0 + 16.0 + 6.0 + label_w + 14.0, 26.0);
        gfx.round_rect(pill, 13.0, 0.0, theme.text);
        Button::confirm(self.settings.confirm).draw(gfx, pill.x + 6.0, y + 5.0);
        let baseline = (y + 13.0 - gfx.line_height(Role::BodyBold) / 2.0 + gfx.ascent(Role::BodyBold)).round();
        gfx.text(Role::BodyBold, pill.x + 28.0, baseline, theme.bg_bottom, launch);
    }

    /// The Carousel: one row of tiles on a lit shelf,
    /// the selected tile large in the middle, the title under the shelf.
    fn draw_carousel(&mut self, gfx: &mut Gfx) {
        let detail = self.settings.detail;
        let ShelfRow { tile, row, .. } = shelf(detail);
        let (pitch, center) = carousel_layout(W, detail);
        self.strip.set(center - self.selected as f32 * pitch, self.now);
        let strip = self.strip.at(self.now);
        let intro = self.intro.at(self.now);
        let lift = 14.0 * (1.0 - intro);
        let bottom = TOP + row - 8.0 + lift;
        let from = self.selected.saturating_sub(SIDE_TILES);
        let to = (self.selected + SIDE_TILES + 1).min(self.list.len());
        self.keep_tiles(from, to);
        gfx.clip(Some(Rect::new(0.0, TOP, W, row)));
        // The selected tile last, over its neighbors.
        let order = (from..to).filter(|&i| i != self.selected).chain(std::iter::once(self.selected));
        for index in order.collect::<Vec<_>>() {
            let selected = index == self.selected;
            let (scale, opacity) = if selected { (1.25, 1.0) } else { (0.9, 0.6) };
            let slot = self.tile(index, Slot { scale, opacity, ..Slot::default() }, 200);
            let left = strip + index as f32 * pitch;
            let side = tile * slot.scale;
            // Scaled from its bottom edge's middle.
            let rect = Rect::new(left + (tile - side) / 2.0, bottom - side, side, side);
            self.draw_tile(gfx, index, rect, tile, slot.opacity * intro, selected, false);
        }
        gfx.clip(None);
        // The shelf: a lit edge and its glow on the surface below.
        let theme = self.theme;
        let edge = TOP + row;
        gfx.fill(Rect::new(0.0, edge, W, 2.0), theme.accent.with_a(0x99));
        let ramp = self.ramp(gfx);
        gfx.image_full(ramp, Rect::new(0.0, edge + 2.0, W, 44.0), theme.accent.with_a(0x2e));
        let role = if detail == DetailLevel::Detailed { Role::Heading } else { Role::Title };
        self.draw_info(gfx, role, edge + 10.0, 440.0, None);
    }

    /// The Grid: rows of five, scrolling a row at a time.
    fn draw_grid(&mut self, gfx: &mut Gfx) {
        let detail = self.settings.detail;
        let (tile, gap) = grid(detail);
        let pitch = tile + gap;
        let row = self.selected / GRID_COLUMNS;
        if row < self.top_row {
            self.top_row = row;
        } else if row >= self.top_row + GRID_ROWS {
            self.top_row = row + 1 - GRID_ROWS;
        }
        self.strip.set(self.top_row as f32 * pitch, self.now);
        let scroll = self.strip.at(self.now);
        let area_h = 2.0 * tile + gap + 14.0;
        let strip_w = GRID_COLUMNS as f32 * tile + (GRID_COLUMNS - 1) as f32 * gap;
        let left = ((W - strip_w) / 2.0).round();
        let from = self.top_row.saturating_sub(1) * GRID_COLUMNS;
        let to = ((self.top_row + GRID_ROWS + 1) * GRID_COLUMNS).min(self.list.len());
        self.keep_tiles(from, to);
        gfx.clip(Some(Rect::new(0.0, TOP, W, area_h)));
        let order = (from..to).filter(|&i| i != self.selected).chain(std::iter::once(self.selected));
        for index in order.collect::<Vec<_>>() {
            let selected = index == self.selected;
            let (scale, opacity) = if selected { (1.0, 1.0) } else { (0.9, 0.7) };
            let slot = self.tile(index, Slot { scale, opacity, ..Slot::default() }, 150);
            let side = tile * slot.scale;
            let x = left + (index % GRID_COLUMNS) as f32 * pitch + (tile - side) / 2.0;
            let y = TOP + 6.0 + (index / GRID_COLUMNS) as f32 * pitch - scroll + (tile - side) / 2.0;
            self.draw_tile(gfx, index, Rect::new(x, y, side, side), tile, slot.opacity, selected, false);
        }
        gfx.clip(None);
        self.draw_info(gfx, Role::Title, TOP + area_h + 4.0, 440.0, None);
    }

    /// The List: a title per row, scrolling a row at a
    /// time.
    fn draw_list(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let pitch = LIST_ROW_H + LIST_GAP;
        if self.selected < self.top_row {
            self.top_row = self.selected;
        } else if self.selected >= self.top_row + LIST_ROWS {
            self.top_row = self.selected + 1 - LIST_ROWS;
        }
        self.strip.set(self.top_row as f32 * pitch, self.now);
        let scroll = self.strip.at(self.now);
        let area = Rect::new(BAR_SIDE, TOP + 4.0, W - 2.0 * BAR_SIDE, LIST_ROWS as f32 * pitch - LIST_GAP);
        let from = self.top_row.saturating_sub(2);
        let to = (self.top_row + LIST_ROWS + 2).min(self.list.len());
        let detailed = self.settings.detail == DetailLevel::Detailed;
        gfx.clip(Some(area));
        for index in from..to {
            let y = area.y + index as f32 * pitch - scroll;
            if y + LIST_ROW_H < area.y || y > area.bottom() {
                continue;
            }
            let selected = index == self.selected;
            let row = Rect::new(area.x, y, area.w, LIST_ROW_H);
            if selected {
                gfx.round_rect(row, 8.0, 0.0, theme.text.with_a(0x1c));
            }
            let game = self.library.games()[self.list[index]].clone();
            self.art.draw(gfx, &game, Rect::new(row.x + 12.0, y + 4.0, 22.0, 22.0), 22.0, 1.0, self.settings.icon_box, true);
            let role = if selected { Role::BodyBold } else { Role::Body };
            let title = gfx.fit(role, &game.title, 300.0);
            let baseline = (y + LIST_ROW_H / 2.0 - gfx.line_height(role) / 2.0 + gfx.ascent(role)).round();
            gfx.text(role, row.x + 12.0 + 22.0 + 8.0, baseline, if selected { theme.text } else { theme.dim }, &title);
            if detailed {
                let small = (y + LIST_ROW_H / 2.0 - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
                gfx.text_right(Role::Small, row.right() - 12.0, small, if selected { theme.dim } else { theme.faint }, game.shown_id());
            }
        }
        gfx.clip(None);
    }

    /// The Cascade, after the Xbox 360 dashboard: the
    /// selected title at the left, the next ones receding and darkening to
    /// the right.
    fn draw_cascade(&mut self, gfx: &mut Gfx) {
        let detail = self.settings.detail;
        let (side, row) = cascade(detail);
        let intro = self.intro.at(self.now);
        let shift = 24.0 * (1.0 - intro);
        let from = self.selected.saturating_sub(1);
        let to = (self.selected + CASCADE_DEPTH as usize + 2).min(self.list.len());
        self.keep_tiles(from, to);
        gfx.clip(Some(Rect::new(0.0, TOP, W, row)));
        let mut order: Vec<usize> = (from..to).collect();
        order.sort_by_key(|&i| std::cmp::Reverse((i as i64 - self.selected as i64).abs()));
        let shade_color = self.theme.bg_bottom;
        for index in order {
            let offset = index as i32 - self.selected as i32;
            let s = cascade_slot(detail, offset);
            let slot = self.tile(index, Slot { x: s.x, y: s.y, scale: s.scale, opacity: s.opacity, shade: s.shade }, 240);
            let w = side * slot.scale;
            // Scaled from its left edge's middle.
            let rect = Rect::new(slot.x + shift, TOP + 12.0 + (side - w) / 2.0 + slot.y, w, w);
            let opacity = slot.opacity * intro;
            self.draw_tile(gfx, index, rect, side, opacity, offset == 0, false);
            if slot.shade > 0.0 && opacity > 0.0 {
                gfx.fill(rect, shade_color.alpha(slot.shade * opacity));
            }
        }
        gfx.clip(None);
        let role = if detail == DetailLevel::Detailed { Role::Heading } else { Role::Title };
        self.draw_info(gfx, role, TOP + row, W - CASCADE_LEFT * 2.0, Some(CASCADE_LEFT));
    }
}
