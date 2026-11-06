//! The views: Home, Carousel, Grid, List, Cascade, Split and Spotlight,
//! each drawing the tiles near the selection from the shelf's state, and
//! the title's details under them.

use super::art::{selected_frame, tile_shadow};
use super::chrome::{Button, FOOTER_H};
use super::layout::*;
use super::{Shelf, Slot, TileAnim};
use crate::gfx::{Gfx, Rect};
use crate::library::catalog::Game;
use crate::library::settings::{DetailLevel, ViewMode};
use crate::shapes::{Icon, Symbol};
use crate::text::Role;
use crate::touch::Hit;
use crate::{H, W};

/// Top of the view: under the top bar.
const TOP: f32 = BAR_H;
/// The view's height: between the bar and the footer.
const VIEW_H: f32 = H - BAR_H - FOOTER_H;

/// Tiles kept on each side of the carousel's selected tile.
const SIDE_TILES: usize = 5;

/// The details panel's left edge in Home and the Cascade, and its rows'
/// pitch.
const DETAILS_LEFT: f32 = 540.0;
const DETAILS_PITCH: f32 = 28.0;
/// Space from an icon's left edge to its text, and between two items in
/// the details row.
const DETAIL_TEXT: f32 = 30.0;
const DETAIL_GAP: f32 = 28.0;
/// The start button's height.
const LAUNCH_H: f32 = 52.0;

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
            ViewMode::Spotlight => 240,
            ViewMode::Grid | ViewMode::List | ViewMode::Split => 1,
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
            ViewMode::List | ViewMode::Split => {
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
            ViewMode::Split => self.draw_split(gfx),
            ViewMode::Spotlight => self.draw_spotlight(gfx),
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
            let top = TOP + VIEW_H / 2.0 - (line + 8.0 + 32.0) / 2.0;
            let term = gfx.fit(Role::Title, &self.library.search_term, 360.0);
            gfx.text_top_centered(Role::Title, W / 2.0, top, theme.text, &crate::i18n::fill(t.no_match, &[("term", &term)]));
            let items = [(Button::Face(Symbol::Square), t.search_again), (Button::cancel(self.settings.confirm), t.close)];
            super::chrome::hints_centered(gfx, &theme, &items, W / 2.0, top + line + 8.0, W - 64.0);
            return;
        }
        let has_titles = !self.library.games().is_empty();
        let line = gfx.line_height(Role::Title);
        let cy = TOP + VIEW_H / 2.0;
        let top = if has_titles { cy - (line + 8.0 + 32.0) / 2.0 } else { cy - line / 2.0 };
        let t = self.t();
        let text = if has_titles { t.empty_category } else { t.no_titles };
        gfx.text_top_centered(Role::Title, W / 2.0, top, theme.text, text);
        if has_titles {
            // "Press {button} on a title to move it here", the button inline.
            let (before, after) = t.move_here.split_once("{button}").unwrap_or((t.move_here, ""));
            let width = gfx.measure(Role::Small, before) + 40.0 + gfx.measure(Role::Small, after);
            let y = top + line + 8.0;
            let baseline = (y + 16.0 - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
            let mut x = W / 2.0 - width / 2.0;
            x = gfx.text(Role::Small, x, baseline, theme.dim, before.trim_end()) + 4.0;
            Button::Face(Symbol::Triangle).draw(gfx, x, y);
            gfx.text(Role::Small, x + 36.0, baseline, theme.dim, after.trim_start());
        }
    }

    /// Whether the selected title's details panel shows: the setting is on,
    /// the view shows text, and the game database or Wikidata knows the
    /// title.
    fn details_shown(&self, game: &Game) -> bool {
        self.settings.game_info.details && self.settings.detail != DetailLevel::Basic && (game.details.is_some() || game.wiki.is_some())
    }

    /// The genre libretro's database gives, else Wikidata's first in the
    /// app's language.
    fn genre_of(&self, game: &Game) -> Option<String> {
        match game.details.as_deref().filter(|d| !d.genre.is_empty()) {
            Some(d) => Some(d.genre.clone()),
            None => {
                let language = crate::i18n::LANGUAGES.iter().position(|(l, _)| *l == self.settings.language).unwrap_or(0);
                game.wiki.as_ref()?.genre(language)
            }
        }
    }

    /// The release year and genre for a details line, when the panel does
    /// not show them.
    fn line_details(&self, game: &Game) -> Vec<String> {
        if (game.details.is_none() && game.wiki.is_none()) || self.details_shown(game) {
            return Vec::new();
        }
        let year = game.year();
        let year = (year > 0).then(|| year.to_string());
        year.into_iter().chain(self.genre_of(game)).collect()
    }

    /// The panel's items, each with its icon: year, genre, developer, the
    /// publisher when another company (with `publisher`), players, region.
    /// Each is libretro's database's, else Wikidata's.
    fn detail_items(&self, game: &Game, publisher: bool) -> Vec<(Icon, String)> {
        if game.details.is_none() && game.wiki.is_none() {
            return Vec::new();
        }
        let empty = crate::online::gamedb::Details::default();
        let d = game.details.as_deref().unwrap_or(&empty);
        let wiki = game.wiki.as_deref().map(|w| &w.game);
        let either = |own: &str, other: fn(&crate::online::wikidata::Game) -> &[String]| {
            if own.is_empty() {
                wiki.map_or(String::new(), |w| other(w).join(", "))
            } else {
                own.to_string()
            }
        };
        let developer = either(&d.developer, |w| &w.developers);
        let publisher_name = either(&d.publisher, |w| &w.publishers);
        let t = self.t();
        let mut items = Vec::new();
        let year = game.year();
        if year > 0 {
            items.push((Icon::Calendar, year.to_string()));
        }
        if let Some(genre) = self.genre_of(game) {
            items.push((Icon::Genre, genre));
        }
        if !developer.is_empty() {
            items.push((Icon::Developer, developer.clone()));
        }
        if publisher && !publisher_name.is_empty() && publisher_name != developer {
            items.push((Icon::Publisher, publisher_name));
        }
        let players = match d.players {
            0 => game.wiki.as_ref().and_then(|w| w.multiplayer()).map(|multi| if multi { t.players_multi.to_string() } else { t.players_one.to_string() }),
            1 => Some(t.players_one.to_string()),
            n => Some(crate::i18n::fill(t.players_many, &[("count", &n.to_string())])),
        };
        if let Some(text) = players {
            items.push((Icon::Players, text));
        }
        if !d.region.is_empty() {
            items.push((Icon::Region, d.region.clone()));
        }
        items
    }

    /// The details panel as a column of rows from `(x, y)`, `w` wide, down
    /// to `bottom` at most.
    fn draw_details_column(&mut self, gfx: &mut Gfx, game: &Game, x: f32, y: f32, w: f32, bottom: f32) {
        let theme = self.theme;
        let mut y = y;
        for (icon, text) in self.detail_items(game, true) {
            if y + DETAILS_PITCH > bottom {
                break;
            }
            gfx.icon(icon, x, y + (DETAILS_PITCH - 20.0) / 2.0 - 2.0, theme.accent);
            let text = gfx.fit(Role::Small, &text, w - DETAIL_TEXT);
            let baseline = (y + DETAILS_PITCH / 2.0 - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small) - 2.0).round();
            gfx.text(Role::Small, x + DETAIL_TEXT, baseline, theme.dim, &text);
            y += DETAILS_PITCH;
        }
    }

    /// The details panel as one row, top at `y`, from `left` or centered on
    /// the screen: the items that fit in `w`, in order (with the publisher
    /// at "Detailed").
    fn draw_details_row(&mut self, gfx: &mut Gfx, game: &Game, y: f32, w: f32, left: Option<f32>) {
        let theme = self.theme;
        let mut items: Vec<(Icon, String, f32)> = Vec::new();
        let mut width = 0.0;
        let publisher = self.settings.detail == DetailLevel::Detailed;
        for (icon, text) in self.detail_items(game, publisher) {
            let text = gfx.fit(Role::Small, &text, 260.0);
            let item = DETAIL_TEXT + gfx.measure(Role::Small, &text);
            let added = if items.is_empty() { item } else { DETAIL_GAP + item };
            if width + added > w {
                break;
            }
            width += added;
            items.push((icon, text, item));
        }
        let mut x = left.unwrap_or(((W - width) / 2.0).round());
        let baseline = (y + 10.0 - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
        for (icon, text, item) in items {
            gfx.icon(icon, x, y, theme.accent);
            gfx.text(Role::Small, x + DETAIL_TEXT, baseline, theme.dim, &text);
            x += item + DETAIL_GAP;
        }
    }

    /// The selected title's name and, at "Detailed", its id and category,
    /// centered on `cx` (or from `left` when given), top at `top`. Returns
    /// the bottom of the text.
    fn draw_info(&mut self, gfx: &mut Gfx, role: Role, top: f32, width: f32, left: Option<f32>) -> f32 {
        if self.settings.detail == DetailLevel::Basic {
            return top;
        }
        let theme = self.theme;
        let Some(game) = self.selected_game().cloned() else { return top };
        let title = gfx.fit(role, &game.title, width);
        let line = gfx.line_height(role);
        let detail = (self.settings.detail == DetailLevel::Detailed).then(|| {
            let mut parts = vec![self.shown_id(&game).to_string(), self.category_label(&game)];
            parts.extend(self.line_details(&game));
            gfx.fit(Role::Small, &parts.join("  ·  "), width)
        });
        let bottom = top + line + if detail.is_some() { 4.0 + gfx.line_height(Role::Small) } else { 0.0 };
        match left {
            Some(x) => {
                gfx.text_top(role, x, top, theme.text, &title);
                if let Some(detail) = detail {
                    gfx.text_top(Role::Small, x, top + line + 4.0, theme.dim, &detail);
                }
            }
            None => {
                gfx.text_top_centered(role, W / 2.0, top, theme.text, &title);
                if let Some(detail) = detail {
                    gfx.text_top_centered(Role::Small, W / 2.0, top + line + 4.0, theme.dim, &detail);
                }
            }
        }
        bottom
    }

    /// The details row under a centered title (Carousel, Grid), when it
    /// fits above the footer.
    fn draw_details_under(&mut self, gfx: &mut Gfx, bottom: f32) {
        let Some(game) = self.selected_game().cloned() else { return };
        if !self.details_shown(&game) {
            return;
        }
        let y = bottom + 12.0;
        if y + 20.0 <= H - FOOTER_H - 4.0 {
            self.draw_details_row(gfx, &game, y, 880.0, None);
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
        // Tiles are drawn farthest first, so the selected one is on top.
        gfx.hit(rect, Hit::Tile(index));
        let game = &self.library.games()[game_index];
        if selected {
            // Around the part the picture covers: a PSP icon's rectangle,
            // not the square it is fitted into.
            let shown = self.art.shown(game, rect, gfx);
            if shadow {
                tile_shadow(gfx, shown, opacity);
            }
            selected_frame(gfx, shown, k, self.theme.text.alpha(opacity));
        }
        self.art.draw(gfx, game, rect, side, opacity, self.settings.icon_box, false);
    }

    /// Home, after the PS5 home screen: the selected
    /// title large at the left, the next ones smaller after it, its name, a
    /// line about it and a start button below.
    fn draw_home(&mut self, gfx: &mut Gfx) {
        let detail = self.settings.detail;
        let Home { tile, .. } = home(detail);
        let row_h = tile + 32.0;
        let intro = self.intro.at(self.now);
        let shift = 48.0 * (1.0 - intro);
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
            let rect = Rect::new(slot.x + shift, TOP + 16.0, side, side);
            self.draw_tile(gfx, index, rect, tile, slot.opacity * intro, offset == 0, true);
        }
        if detail == DetailLevel::Basic {
            return;
        }
        let theme = self.theme;
        let Some(game) = self.selected_game().cloned() else { return };
        let panel = self.details_shown(&game);
        let text_w = if panel { DETAILS_LEFT - HOME_LEFT - 32.0 } else { W - HOME_LEFT * 2.0 };
        let mut y = TOP + row_h + 12.0;
        if panel {
            self.draw_details_column(gfx, &game, DETAILS_LEFT, y + 4.0, W - DETAILS_LEFT - HOME_LEFT, H - FOOTER_H - 8.0);
        }
        let title = gfx.fit(Role::Display, &game.title, text_w);
        gfx.text_top(Role::Display, HOME_LEFT, y, theme.text, &title);
        y += gfx.line_height(Role::Display) + 8.0;
        let line = self.info_line(gfx, &game, text_w);
        gfx.text_top(Role::Small, HOME_LEFT, y, theme.dim, &line);
        y += gfx.line_height(Role::Small) + 8.0 + 8.0 + 8.0;
        self.draw_launch(gfx, HOME_LEFT, y);
    }

    /// The line under a title in Home and Spotlight: its console or kind
    /// and category, at "Detailed" its id, and the year and genre when the
    /// details panel does not show them; fitted to `width`.
    fn info_line(&self, gfx: &mut Gfx, game: &Game, width: f32) -> String {
        let genre = if game.console.is_some() { self.shown_id(game) } else { game.platform.genre() };
        let mut parts = vec![genre.to_string(), self.category_label(game)];
        if self.settings.detail == DetailLevel::Detailed {
            parts.push(self.shown_id(game).to_string());
        }
        parts.extend(self.line_details(game));
        let first = parts[0].clone();
        let line: Vec<String> = parts.into_iter().enumerate().filter(|(i, p)| *i == 0 || *p != first).map(|(_, p)| p).collect();
        gfx.fit(Role::Small, &line.join("  ·  "), width)
    }

    /// The start button with its top left at `(x, y)`: the confirm button
    /// and "Launch" on a pill in the text color. Returns its height.
    fn draw_launch(&mut self, gfx: &mut Gfx, x: f32, y: f32) -> f32 {
        let theme = self.theme;
        let launch = self.t().launch;
        let label_w = gfx.measure(Role::BodyBold, launch);
        let pill = Rect::new(x, y, 12.0 + 32.0 + 12.0 + label_w + 28.0, LAUNCH_H);
        gfx.round_rect(pill, LAUNCH_H / 2.0, 0.0, theme.text);
        if let Some(bit) = Button::confirm(self.settings.confirm).bit() {
            gfx.hit(pill, Hit::Button(bit));
        }
        Button::confirm(self.settings.confirm).draw(gfx, pill.x + 12.0, y + 10.0);
        let baseline = (y + LAUNCH_H / 2.0 - gfx.line_height(Role::BodyBold) / 2.0 + gfx.ascent(Role::BodyBold)).round();
        gfx.text(Role::BodyBold, pill.x + 56.0, baseline, theme.bg_bottom, launch);
        LAUNCH_H
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
        let lift = 28.0 * (1.0 - intro);
        let bottom = TOP + row - 16.0 + lift;
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
        gfx.fill(Rect::new(0.0, edge, W, 4.0), theme.accent.with_a(0x99));
        let ramp = self.ramp(gfx);
        gfx.image_full(ramp, Rect::new(0.0, edge + 4.0, W, 88.0), theme.accent.with_a(0x2e));
        let role = if detail == DetailLevel::Detailed { Role::Heading } else { Role::Title };
        let bottom = self.draw_info(gfx, role, edge + 20.0, 880.0, None);
        self.draw_details_under(gfx, bottom);
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
        let area_h = 2.0 * tile + gap + 28.0;
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
            let y = TOP + 12.0 + (index / GRID_COLUMNS) as f32 * pitch - scroll + (tile - side) / 2.0;
            self.draw_tile(gfx, index, Rect::new(x, y, side, side), tile, slot.opacity, selected, false);
        }
        gfx.clip(None);
        let bottom = self.draw_info(gfx, Role::Title, TOP + area_h + 8.0, 880.0, None);
        self.draw_details_under(gfx, bottom);
    }

    /// The List: a title per row, scrolling a row at a
    /// time.
    fn draw_list(&mut self, gfx: &mut Gfx) {
        let detailed = self.settings.detail == DetailLevel::Detailed;
        self.draw_rows(gfx, W - 2.0 * BAR_SIDE, detailed);
    }

    /// The list's rows from the left margin, `width` wide, each with the
    /// title's id at the right with `ids`.
    fn draw_rows(&mut self, gfx: &mut Gfx, width: f32, ids: bool) {
        let theme = self.theme;
        let pitch = LIST_ROW_H + LIST_GAP;
        if self.selected < self.top_row {
            self.top_row = self.selected;
        } else if self.selected >= self.top_row + LIST_ROWS {
            self.top_row = self.selected + 1 - LIST_ROWS;
        }
        self.strip.set(self.top_row as f32 * pitch, self.now);
        let scroll = self.strip.at(self.now);
        let area = Rect::new(BAR_SIDE, TOP + 8.0, width, LIST_ROWS as f32 * pitch - LIST_GAP);
        let from = self.top_row.saturating_sub(2);
        let to = (self.top_row + LIST_ROWS + 2).min(self.list.len());
        let text_left = 24.0 + 44.0 + 16.0;
        let title_w = if ids { 600.0 } else { width - text_left - 24.0 };
        gfx.clip(Some(area));
        for index in from..to {
            let y = area.y + index as f32 * pitch - scroll;
            if y + LIST_ROW_H < area.y || y > area.bottom() {
                continue;
            }
            let selected = index == self.selected;
            let row = Rect::new(area.x, y, area.w, LIST_ROW_H);
            gfx.hit(row, Hit::Tile(index));
            if selected {
                gfx.round_rect(row, 16.0, 0.0, theme.text.with_a(0x1c));
            }
            let game = self.library.games()[self.list[index]].clone();
            self.art.draw(gfx, &game, Rect::new(row.x + 24.0, y + 8.0, 44.0, 44.0), 44.0, 1.0, self.settings.icon_box, true);
            let role = if selected { Role::BodyBold } else { Role::Body };
            let title = gfx.fit(role, &game.title, title_w);
            let baseline = (y + LIST_ROW_H / 2.0 - gfx.line_height(role) / 2.0 + gfx.ascent(role)).round();
            gfx.text(role, row.x + text_left, baseline, if selected { theme.text } else { theme.dim }, &title);
            if ids {
                let small = (y + LIST_ROW_H / 2.0 - gfx.line_height(Role::Small) / 2.0 + gfx.ascent(Role::Small)).round();
                gfx.text_right(Role::Small, row.right() - 24.0, small, if selected { theme.dim } else { theme.faint }, self.shown_id(&game));
            }
        }
        gfx.clip(None);
    }

    /// The selected title's art at `rect`, with a shadow; a tap on it
    /// launches like one on the selected tile.
    fn draw_feature_art(&mut self, gfx: &mut Gfx, index: usize, rect: Rect, opacity: f32) {
        let Some(&game_index) = self.list.get(index) else { return };
        if opacity <= 0.0 {
            return;
        }
        gfx.hit(rect, Hit::Tile(index));
        let game = &self.library.games()[game_index];
        let shown = self.art.shown(game, rect, gfx);
        tile_shadow(gfx, shown, opacity);
        self.art.draw(gfx, game, rect, rect.w, opacity, self.settings.icon_box, false);
    }

    /// Split: the List's rows at the left; at the right the selected
    /// title's art, its name and line, and the details panel.
    fn draw_split(&mut self, gfx: &mut Gfx) {
        let detail = self.settings.detail;
        self.draw_rows(gfx, SPLIT_LIST_W, false);
        let side = split_art(detail);
        let width = W - SPLIT_PANE_LEFT - BAR_SIDE;
        let bottom = H - FOOTER_H - 8.0;
        // At "Basic", the art alone, in the middle of the view's height.
        let top = if detail == DetailLevel::Basic { (TOP + (VIEW_H - side) / 2.0).round() } else { TOP + 8.0 };
        self.draw_feature_art(gfx, self.selected, Rect::new(SPLIT_PANE_LEFT, top, side, side), 1.0);
        if detail == DetailLevel::Basic {
            return;
        }
        let theme = self.theme;
        let Some(game) = self.selected_game().cloned() else { return };
        let mut y = top + side + 16.0;
        let title = gfx.fit(Role::Title, &game.title, width);
        gfx.text_top(Role::Title, SPLIT_PANE_LEFT, y, theme.text, &title);
        y += gfx.line_height(Role::Title) + 4.0;
        let line = self.info_line(gfx, &game, width);
        gfx.text_top(Role::Small, SPLIT_PANE_LEFT, y, theme.dim, &line);
        y += gfx.line_height(Role::Small) + 12.0;
        if self.details_shown(&game) {
            self.draw_details_column(gfx, &game, SPLIT_PANE_LEFT, y, width, bottom);
        }
    }

    /// Spotlight: the selected title's backdrop over the screen, its art
    /// at the lower left, sliding in from the side the selection moved
    /// to, and its name, line, details and start button beside it.
    fn draw_spotlight(&mut self, gfx: &mut Gfx) {
        let detail = self.settings.detail;
        let side = spotlight(detail);
        let intro = self.intro.at(self.now);
        let art_bottom = H - FOOTER_H - 16.0;
        let art_top = art_bottom - side;
        let from = self.selected.saturating_sub(1);
        let to = (self.selected + 2).min(self.list.len());
        self.keep_tiles(from, to);
        for index in from..to {
            let offset = index as f32 - self.selected as f32;
            let opacity = if offset == 0.0 { 1.0 } else { 0.0 };
            let slot = self.tile(index, Slot { x: SPOTLIGHT_LEFT + offset * SPOTLIGHT_SLIDE, opacity, ..Slot::default() }, 220);
            let rect = Rect::new(slot.x - 32.0 * (1.0 - intro), art_top, side, side);
            self.draw_feature_art(gfx, index, rect, slot.opacity * intro);
        }
        if detail == DetailLevel::Basic {
            return;
        }
        let theme = self.theme;
        let Some(game) = self.selected_game().cloned() else { return };
        let left = SPOTLIGHT_LEFT + side + 32.0;
        let width = W - left - SPOTLIGHT_LEFT;
        // From the bottom up: the button level with the art's foot, the
        // details row, the line and the name above them.
        let title_h = gfx.line_height(Role::Display);
        let line_h = gfx.line_height(Role::Small);
        let details = self.details_shown(&game);
        let block = |details: bool| title_h + 8.0 + line_h + if details { 16.0 + 20.0 } else { 0.0 } + 24.0 + LAUNCH_H;
        let details = details && art_bottom - block(true) >= TOP + 16.0;
        let mut y = (art_bottom - block(details)).max(TOP + 16.0);
        let title = gfx.fit(Role::Display, &game.title, width);
        gfx.text_top(Role::Display, left, y, theme.text.alpha(intro), &title);
        y += title_h + 8.0;
        let line = self.info_line(gfx, &game, width);
        gfx.text_top(Role::Small, left, y, theme.dim.alpha(intro), &line);
        y += line_h;
        if details {
            y += 16.0;
            self.draw_details_row(gfx, &game, y, width, Some(left));
            y += 20.0;
        }
        self.draw_launch(gfx, left, y + 24.0);
    }

    /// The Cascade, after the Xbox 360 dashboard: the
    /// selected title at the left, the next ones receding and darkening to
    /// the right.
    fn draw_cascade(&mut self, gfx: &mut Gfx) {
        let detail = self.settings.detail;
        let (side, row) = cascade(detail);
        let intro = self.intro.at(self.now);
        let shift = 48.0 * (1.0 - intro);
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
            let rect = Rect::new(slot.x + shift, TOP + 24.0 + (side - w) / 2.0 + slot.y, w, w);
            let opacity = slot.opacity * intro;
            self.draw_tile(gfx, index, rect, side, opacity, offset == 0, false);
            if slot.shade > 0.0 && opacity > 0.0 {
                // Over the picture only, not its clear bars.
                let shown = self.list.get(index).map_or(rect, |&g| self.art.shown(&self.library.games()[g], rect, gfx));
                gfx.fill(shown, shade_color.alpha(slot.shade * opacity));
            }
        }
        gfx.clip(None);
        let role = if detail == DetailLevel::Detailed { Role::Heading } else { Role::Title };
        let game = self.selected_game().cloned();
        let panel = game.as_ref().is_some_and(|g| self.details_shown(g));
        let width = if panel { DETAILS_LEFT - CASCADE_LEFT - 32.0 } else { W - CASCADE_LEFT * 2.0 };
        self.draw_info(gfx, role, TOP + row, width, Some(CASCADE_LEFT));
        if let (true, Some(game)) = (panel, game) {
            // Higher than the title: right of the selected tile, the
            // receding ones end above it.
            self.draw_details_column(gfx, &game, DETAILS_LEFT, TOP + row - 64.0, W - DETAILS_LEFT - CASCADE_LEFT, H - FOOTER_H - 8.0);
        }
    }
}
