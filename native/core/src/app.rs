//! The development screen: a grid of the installed titles, moved with the
//! D-pad, the selected title's backdrop behind it. It exercises the
//! foundations (host events, pictures loaded near the screen, backdrops
//! freed, text, shapes, themes) until the real views replace it.

use std::collections::{HashMap, HashSet};

use crate::gfx::{Color, Format, Gfx, Rect, TexId, BLACK};
use crate::host::{Event, Host, PictureKey, Status, TitleInfo};
use crate::shapes::{round_corners, Symbol};
use crate::stats::FrameStats;
use crate::text::Role;
use crate::theme::{theme_by_id, Theme, ThemeId, DEFAULT_THEME};
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
use buttons::*;

const COLS: usize = 5;
const TILE: f32 = 64.0;
const GAP: f32 = 18.0;
const GRID_LEFT: f32 = (W - (COLS as f32 * TILE + (COLS as f32 - 1.0) * GAP)) / 2.0;
const TOP: f32 = 44.0;
const PITCH: f32 = TILE + GAP;
const TILE_RADIUS: f32 = 6.0;
const GRID_CLIP: Rect = Rect::new(0.0, 34.0, W, 168.0);
/// Rows asked for beyond the two on screen, each way.
const ICON_MARGIN_ROWS: usize = 1;
/// Synthetic titles added by TRIANGLE, to measure a large library.
const STRESS_TITLES: usize = 5000;
/// A held direction repeats after this many frames, then every `REPEAT`.
const HOLD_DELAY: u32 = 24;
const REPEAT: u32 = 6;
/// Frames the selection rests before its backdrop is asked for.
const BACKDROP_REST: u32 = 8;
/// Frames between status readings.
const STATUS_EVERY: u32 = 60;

struct Entry {
    info: TitleInfo,
    icon: Option<TexId>,
    /// The icon has no see-through pixels, so it gets a shadow.
    opaque: bool,
    initials: String,
}

impl Entry {
    fn new(info: TitleInfo) -> Self {
        let initials = info
            .title
            .split_whitespace()
            .filter_map(|word| word.chars().find(|c| c.is_alphanumeric()))
            .take(2)
            .flat_map(char::to_uppercase)
            .collect();
        Entry { info, icon: None, opaque: true, initials }
    }
}

struct Backdrop {
    id: String,
    tex: TexId,
    /// 0 to 1 while it fades in.
    shown: f32,
}

pub struct App {
    theme: Theme,
    entries: Vec<Entry>,
    by_id: HashMap<String, usize>,
    real_count: usize,
    scanned: Option<u32>,
    selected: usize,
    /// First visible row, animated toward `top_row`.
    scroll: f32,
    top_row: usize,
    ring: Option<Rect>,
    previous: u32,
    held: u32,
    rested: u32,
    requested: HashSet<PictureKey>,
    backdrop: Option<Backdrop>,
    /// The background gradient and the theme it was made for.
    background: Option<(ThemeId, TexId)>,
    status: Status,
    frame: u32,
    events: Vec<Event>,
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
            theme: theme_by_id(DEFAULT_THEME),
            entries: Vec::new(),
            by_id: HashMap::new(),
            real_count: 0,
            scanned: None,
            selected: 0,
            scroll: 0.0,
            top_row: 0,
            ring: None,
            previous: 0,
            held: 0,
            rested: 0,
            requested: HashSet::new(),
            backdrop: None,
            background: None,
            status: Status::default(),
            frame: 0,
            events: Vec::new(),
            stats: FrameStats::default(),
            quit: false,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Take the host's answers. Returns how many there were.
    pub fn events(&mut self, host: &mut dyn Host, gfx: &mut Gfx) -> usize {
        let mut events = std::mem::take(&mut self.events);
        host.poll(&mut events);
        let count = events.len();
        for event in events.drain(..) {
            match event {
                Event::Titles(titles, ms) => {
                    host.log(&format!("titles {} in {ms} ms", titles.len()));
                    self.set_titles(titles);
                    self.scanned = Some(ms);
                }
                Event::Picture(PictureKey::Icon(id), picture) => {
                    let (Some(picture), Some(&index)) = (picture, self.by_id.get(&id)) else { continue };
                    let mut rgba = picture.rgba;
                    let entry = &mut self.entries[index];
                    entry.opaque = rgba.chunks_exact(4).all(|p| p[3] >= 250);
                    round_corners(&mut rgba, picture.w, picture.h, TILE_RADIUS * picture.w as f32 / TILE);
                    entry.icon = Some(gfx.add_texture(picture.w, picture.h, Format::Rgba8, rgba, true));
                }
                Event::Picture(PictureKey::Backdrop(id), picture) => {
                    self.requested.remove(&PictureKey::Backdrop(id.clone()));
                    // Only the selected title's backdrop is kept.
                    if self.selected_id() != Some(id.as_str()) {
                        continue;
                    }
                    if let Some(old) = self.backdrop.take() {
                        gfx.textures.free(old.tex);
                    }
                    if let Some(picture) = picture {
                        let tex = gfx.add_texture(picture.w, picture.h, Format::Rgba8, picture.rgba, true);
                        self.backdrop = Some(Backdrop { id, tex, shown: 0.0 });
                    }
                }
            }
        }
        self.events = events;
        count
    }

    fn set_titles(&mut self, titles: Vec<TitleInfo>) {
        self.entries = titles.into_iter().map(Entry::new).collect();
        self.by_id = self.entries.iter().enumerate().map(|(i, e)| (e.info.id.clone(), i)).collect();
        self.real_count = self.entries.len();
        self.selected = 0;
        self.top_row = 0;
        self.scroll = 0.0;
        self.ring = None;
    }

    fn selected_id(&self) -> Option<&str> {
        self.entries.get(self.selected).map(|e| e.info.id.as_str())
    }

    fn rows(&self) -> usize {
        self.entries.len().div_ceil(COLS)
    }

    fn move_by(&mut self, step: isize, wrap: bool) {
        let count = self.entries.len() as isize;
        if count == 0 {
            return;
        }
        let target = self.selected as isize + step;
        self.selected = if (0..count).contains(&target) {
            target as usize
        } else if !wrap {
            return;
        } else if step.abs() == 1 {
            target.rem_euclid(count) as usize
        } else {
            // Up and down past an end keep the column.
            let column = self.selected as isize % COLS as isize;
            let last_row = (count - 1) / COLS as isize;
            let row = if step > 0 { 0 } else { last_row };
            (row * COLS as isize + column).min(count - 1) as usize
        };
    }

    pub fn update(&mut self, host: &mut dyn Host, pad: u32) {
        if self.frame == 0 {
            host.scan();
        }
        if self.frame % STATUS_EVERY == 0 {
            self.status = host.status();
        }
        self.frame += 1;

        let pressed = pad & !self.previous;
        let directions = pad & (UP | DOWN | LEFT | RIGHT);
        self.held = if directions != 0 && directions == self.previous & (UP | DOWN | LEFT | RIGHT) { self.held + 1 } else { 0 };
        let repeat = self.held >= HOLD_DELAY && (self.held - HOLD_DELAY) % REPEAT == 0;
        let fire = |bit: u32| pressed & bit != 0 || (repeat && pad & bit != 0);
        let wrap = pressed & directions != 0;
        let before = self.selected;
        if fire(LEFT) {
            self.move_by(-1, wrap);
        }
        if fire(RIGHT) {
            self.move_by(1, wrap);
        }
        if fire(UP) {
            self.move_by(-(COLS as isize), wrap);
        }
        if fire(DOWN) {
            self.move_by(COLS as isize, wrap);
        }
        if pressed & TRIANGLE != 0 {
            self.toggle_stress();
        }
        if pressed & (L | R) != 0 {
            let at = ThemeId::ALL.iter().position(|&id| id == self.theme.id).unwrap_or(0);
            let step = if pressed & R != 0 { 1 } else { ThemeId::ALL.len() - 1 };
            self.theme = theme_by_id(ThemeId::ALL[(at + step) % ThemeId::ALL.len()]);
        }
        if pressed & SELECT != 0 {
            self.stats.reset();
        }
        if pressed & START != 0 {
            self.quit = true;
        }
        self.previous = pad;
        self.rested = if self.selected == before { self.rested + 1 } else { 0 };

        // Keep the selected row in the two visible rows.
        let row = self.selected / COLS;
        if row < self.top_row {
            self.top_row = row;
        } else if row > self.top_row + 1 {
            self.top_row = row - 1;
        }
        let target = self.top_row as f32;
        self.scroll += (target - self.scroll) * 0.3;
        if (target - self.scroll).abs() < 0.002 {
            self.scroll = target;
        }

        self.request_pictures(host);
    }

    /// Icons for the rows on screen and one more each way; the backdrop of a
    /// selection that has rested.
    fn request_pictures(&mut self, host: &mut dyn Host) {
        if self.entries.is_empty() {
            return;
        }
        let first = self.top_row.saturating_sub(ICON_MARGIN_ROWS);
        let last = (self.top_row + 2 + ICON_MARGIN_ROWS).min(self.rows());
        // The selected tile first, then the rest in reading order.
        let order = std::iter::once(self.selected).chain(first * COLS..(last * COLS).min(self.entries.len()));
        for index in order {
            let key = PictureKey::Icon(self.entries[index].info.id.clone());
            if !self.requested.contains(&key) {
                host.request(key.clone());
                self.requested.insert(key);
            }
        }
        if self.rested == BACKDROP_REST {
            let id = self.entries[self.selected].info.id.clone();
            let key = PictureKey::Backdrop(id.clone());
            if self.backdrop.as_ref().is_none_or(|b| b.id != id) && !self.requested.contains(&key) {
                host.request(key.clone());
                self.requested.insert(key);
            }
        }
    }

    fn toggle_stress(&mut self) {
        if self.entries.len() > self.real_count {
            for entry in self.entries.drain(self.real_count..) {
                self.by_id.remove(&entry.info.id);
            }
        } else {
            for n in 1..=STRESS_TITLES {
                let id = format!("TEST{n:05}");
                self.by_id.insert(id.clone(), self.entries.len());
                self.entries.push(Entry::new(TitleInfo { id, title: format!("Test Title {n}"), platform: "vita".into(), adrenaline: false, console: None }));
            }
        }
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }

    fn tile_rect(&self, index: usize) -> Rect {
        let (row, col) = (index / COLS, index % COLS);
        Rect::new(GRID_LEFT + col as f32 * PITCH, TOP + (row as f32 - self.scroll) * PITCH, TILE, TILE)
    }

    pub fn draw(&mut self, gfx: &mut Gfx) {
        gfx.begin();
        let theme = self.theme;
        let background = match self.background {
            Some((id, tex)) if id == theme.id => tex,
            old => {
                if let Some((_, tex)) = old {
                    gfx.textures.free(tex);
                }
                let tex = gfx.gradient(theme.bg_top, theme.bg_bottom);
                self.background = Some((theme.id, tex));
                tex
            }
        };
        gfx.image_full(background, Rect::new(0.0, 0.0, W, H), Color::rgb(0xffffff));
        if let Some(backdrop) = &mut self.backdrop {
            // It fades in once it is on the GPU.
            if !gfx.textures.ready(backdrop.tex) {
                backdrop.shown = 0.0;
            } else {
                backdrop.shown = (backdrop.shown + 0.12).min(1.0);
            }
            gfx.image_cover(backdrop.tex, Rect::new(0.0, 0.0, W, H), Color::rgb(0xffffff).alpha(backdrop.shown));
            // A wash in the theme's color keeps text readable over it.
            gfx.fill(Rect::new(0.0, 0.0, W, H), theme.bg_bottom.with_a(150).alpha(backdrop.shown));
        }

        self.draw_header(gfx);
        if self.entries.is_empty() {
            let line = if self.scanned.is_some() { "No titles found" } else { "Reading titles…" };
            gfx.text_centered(Role::Body, W / 2.0, H / 2.0, theme.dim, line);
            return;
        }
        self.draw_grid(gfx);
        self.draw_details(gfx);
        self.draw_footer(gfx);
    }

    fn draw_header(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let end = gfx.text(Role::Title, 12.0, 20.0, theme.text, "Pocket Shelf");
        let end = gfx.text(Role::Small, end + 5.0, 20.0, theme.accent, "Next");
        gfx.text(Role::Small, end + 8.0, 20.0, theme.faint, theme.name);
        let clock = format!("{:02}:{:02}", self.status.hour, self.status.minute);
        gfx.text_right(Role::Title, W - 12.0, 20.0, theme.text, &clock);
        let width = gfx.measure(Role::Title, &clock);
        gfx.text_right(Role::Small, W - 24.0 - width, 19.0, theme.dim, &self.stats.short());
    }

    fn draw_grid(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        gfx.clip(Some(GRID_CLIP));
        let first_row = self.scroll.floor().max(0.0) as usize;
        let last_row = (first_row + 3).min(self.rows());
        let end = (last_row * COLS).min(self.entries.len());
        for index in first_row * COLS..end {
            // Whole screen pixels, so a 128 px icon in a 64-unit tile is 1:1.
            let rect = self.tile_rect(index).scale(gfx.scale).round().scale(1.0 / gfx.scale);
            let entry = &self.entries[index];
            match entry.icon.filter(|&icon| gfx.textures.ready(icon)) {
                Some(icon) => {
                    if entry.opaque {
                        gfx.shadow(rect, TILE_RADIUS, 6.0, BLACK.with_a(110));
                    } else {
                        gfx.round_rect(rect, TILE_RADIUS, 0.0, theme.tile);
                    }
                    gfx.image_full(icon, rect, Color::rgb(0xffffff));
                }
                None => {
                    let initials = entry.initials.clone();
                    gfx.round_rect(rect, TILE_RADIUS, 0.0, theme.tile);
                    gfx.text_centered(Role::Heading, rect.x + TILE / 2.0, rect.y + TILE / 2.0 + 7.0, theme.dim, &initials);
                }
            }
        }
        // The ring eases toward the selected tile.
        let target = self.tile_rect(self.selected).grow(4.0);
        let ring = match self.ring {
            Some(ring) => ring.lerp(target, 0.35),
            None => target,
        };
        self.ring = Some(ring);
        gfx.round_rect(ring, TILE_RADIUS + 4.0, 2.0, theme.text);
        gfx.clip(None);
    }

    fn draw_details(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let entry = &self.entries[self.selected];
        let (title, id) = (entry.info.title.clone(), entry.info.id.clone());
        let title = gfx.fit(Role::Heading, &title, W - 40.0);
        gfx.text_centered(Role::Heading, W / 2.0, 222.0, theme.text, &title);
        gfx.text_centered(Role::Small, W / 2.0, 238.0, theme.dim, &id);
    }

    fn draw_footer(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let baseline = 262.0;
        let mut x = 12.0;
        let mut hint = |gfx: &mut Gfx, symbol: Option<(Symbol, Color)>, key: &str, label: &str| {
            match symbol {
                Some((symbol, color)) => {
                    gfx.symbol(symbol, x, baseline - 9.5, 11.0, color);
                    x += 14.0;
                }
                None => {
                    let width = gfx.measure(Role::Label, key) + 10.0;
                    gfx.round_rect(Rect::new(x, baseline - 12.0, width, 16.0), 8.0, 0.0, theme.text.with_a(0x2a));
                    gfx.text(Role::Label, x + 5.0, baseline, theme.text, key);
                    x += width + 4.0;
                }
            }
            x = gfx.text(Role::Body, x, baseline, theme.text.alpha(0.85), label) + 13.0;
        };
        let stress = if self.entries.len() > self.real_count { "Remove tests" } else { "5,000 tests" };
        hint(gfx, Some((Symbol::Triangle, Color::rgb(0x40d6b4))), "", stress);
        hint(gfx, None, "SELECT", "Timing");
        hint(gfx, None, "L R", "Theme");
        hint(gfx, None, "START", "Quit");
        let position = format!("{} / {}", self.selected + 1, self.entries.len());
        gfx.text_right(Role::Body, W - 12.0, baseline, theme.dim, &position);
    }
}
