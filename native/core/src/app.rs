//! The development screen: the titles of one tab in a grid, moved with the
//! D-pad, L and R between tabs, the selected title's backdrop behind it; on
//! a first start, the scan page. It exercises the foundations and the
//! library (saved settings, categories, changes per title, "Last Played")
//! until the real views replace it in phase 3.

use std::collections::{HashMap, HashSet};

use crate::gfx::{Color, Format, Gfx, Rect, TexId, BLACK};
use crate::host::{Event, Host, PictureKey, ScanPhase, Status, TitleInfo};
use crate::library::catalog::{load_catalog, Game};
use crate::library::categories::{cycle_category, Category};
use crate::library::settings::{Settings, UpdateChannel, SETTINGS_FILE};
use crate::library::Library;
use crate::shapes::{round_corners, Symbol};
use crate::stats::FrameStats;
use crate::text::Role;
use crate::theme::{theme_by_id, Theme, ThemeId};
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
/// Synthetic titles added by TRIANGLE (homebrew ids), to measure a large
/// library.
const STRESS_TITLES: usize = 5000;
/// A held direction repeats after this many frames, then every `REPEAT`.
const HOLD_DELAY: u32 = 24;
const REPEAT: u32 = 6;
/// Frames the selection rests before its backdrop is asked for.
const BACKDROP_REST: u32 = 8;
/// Frames between status readings.
const STATUS_EVERY: u32 = 60;
/// Frames the full scan bar stays before the titles replace the page.
const SCAN_DONE_FRAMES: u32 = 8;
const SCAN_BAR_W: f32 = 240.0;

struct Icon {
    tex: TexId,
    /// No see-through pixels, so it gets a shadow.
    opaque: bool,
}

struct Backdrop {
    id: String,
    tex: TexId,
    /// 0 to 1 while it fades in.
    shown: f32,
}

/// The scan page's state: steps done of the total, the phase, and frames
/// since the titles arrived.
struct Scan {
    done: u32,
    total: u32,
    phase: Option<ScanPhase>,
    finished: Option<u32>,
}

pub struct App {
    settings: Settings,
    theme: Theme,
    library: Option<Library>,
    stress: bool,
    tabs: Vec<Category>,
    tab: String,
    /// The current tab's titles, as indexes into the library's games.
    view: Vec<usize>,
    icons: HashMap<String, Icon>,
    scanned: Option<u32>,
    scan: Option<Scan>,
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
    /// The mark's three tile gradients, made on first use.
    mark: Option<[TexId; 3]>,
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

fn initials(title: &str) -> String {
    title.split_whitespace().filter_map(|word| word.chars().find(|c| c.is_alphanumeric())).take(2).flat_map(char::to_uppercase).collect()
}

impl App {
    pub fn new() -> Self {
        let settings = Settings::defaults(UpdateChannel::Off);
        App {
            theme: theme_by_id(settings.theme),
            settings,
            library: None,
            stress: false,
            tabs: Vec::new(),
            tab: String::new(),
            view: Vec::new(),
            icons: HashMap::new(),
            scanned: None,
            scan: None,
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
            mark: None,
            status: Status::default(),
            frame: 0,
            events: Vec::new(),
            stats: FrameStats::default(),
            quit: false,
        }
    }

    /// Titles in the library.
    pub fn len(&self) -> usize {
        self.library.as_ref().map_or(0, |l| l.games().len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Take the host's answers. Returns how many there were.
    pub fn events(&mut self, host: &mut dyn Host, gfx: &mut Gfx) -> usize {
        let mut events = std::mem::take(&mut self.events);
        host.poll(&mut events);
        let count = events.len();
        for event in events.drain(..) {
            match event {
                Event::Scan { done, total, phase } => {
                    self.scan = Some(Scan { done, total, phase: Some(phase), finished: None });
                }
                Event::Titles(titles, ms) => {
                    host.log(&format!("titles {} in {ms} ms", titles.len()));
                    if let Some(scan) = &mut self.scan {
                        scan.done = scan.total.max(1);
                        scan.total = scan.done;
                        scan.finished = Some(0);
                    }
                    self.stress = false;
                    let started = host.time_us();
                    self.set_titles(Some(titles));
                    host.log(&format!("library: {} titles in {} us", self.len(), host.time_us() - started));
                    self.scanned = Some(ms);
                }
                Event::Picture(PictureKey::Icon(id), picture) => {
                    let Some(picture) = picture else { continue };
                    let mut rgba = picture.rgba;
                    let opaque = rgba.chunks_exact(4).all(|p| p[3] >= 250);
                    round_corners(&mut rgba, picture.w, picture.h, TILE_RADIUS * picture.w as f32 / TILE);
                    let tex = gfx.add_texture(picture.w, picture.h, Format::Rgba8, rgba, true);
                    self.icons.insert(id, Icon { tex, opaque });
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

    /// Give the library the host's titles, or (with `None`) add or remove
    /// the test titles. With the host's titles, the saved selection is
    /// restored.
    fn set_titles(&mut self, titles: Option<Vec<Game>>) {
        let first = titles.is_some();
        let selected = self.selected_id().map(String::from);
        let Some(library) = &mut self.library else { return };
        let games = match titles {
            Some(games) => games,
            None if self.stress => {
                let tests: Vec<TitleInfo> = (1..=STRESS_TITLES)
                    .map(|n| TitleInfo { id: format!("TEST{n:05}"), title: format!("Test Title {n}"), platform: "vita".into(), adrenaline: false, console: None })
                    .collect();
                library.base().iter().cloned().chain(load_catalog(&tests)).collect()
            }
            None => library.base().iter().filter(|g| !g.id.starts_with("TEST")).cloned().collect(),
        };
        library.set_catalog(games);
        if first {
            self.tab = self.settings.category.clone().unwrap_or_default();
        }
        self.refresh_view();
        let wanted = if first { self.settings.title.clone() } else { selected };
        self.selected = wanted.and_then(|id| self.position_of(&id)).unwrap_or(0);
        self.jump_to_selection();
    }

    /// The tabs and the current tab's titles, after the library changed; the
    /// first tab when the current one is gone.
    fn refresh_view(&mut self) {
        let Some(library) = &self.library else { return };
        self.tabs = library.tabs();
        if !self.tabs.iter().any(|c| c.id == self.tab) {
            self.tab = self.tabs.first().map(|c| c.id.clone()).unwrap_or_default();
        }
        self.view = library.titles_of(&self.tab);
        self.selected = self.selected.min(self.view.len().saturating_sub(1));
    }

    fn position_of(&self, id: &str) -> Option<usize> {
        let games = self.library.as_ref()?.games();
        self.view.iter().position(|&i| &*games[i].id == id)
    }

    fn jump_to_selection(&mut self) {
        let row = self.selected / COLS;
        self.top_row = row.saturating_sub(1);
        self.scroll = self.top_row as f32;
        self.ring = None;
    }

    fn game(&self, index: usize) -> Option<&Game> {
        let library = self.library.as_ref()?;
        self.view.get(index).map(|&i| &library.games()[i])
    }

    fn selected_id(&self) -> Option<&str> {
        self.game(self.selected).map(|g| &*g.id)
    }

    fn rows(&self) -> usize {
        self.view.len().div_ceil(COLS)
    }

    fn move_by(&mut self, step: isize, wrap: bool) {
        let count = self.view.len() as isize;
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

    fn switch_tab(&mut self, delta: i32) {
        let ids: Vec<String> = self.tabs.iter().map(|c| c.id.clone()).collect();
        let next = cycle_category(&ids, &self.tab, delta).to_string();
        if next != self.tab {
            self.tab = next;
            self.selected = 0;
            self.refresh_view();
            self.jump_to_selection();
        }
    }

    pub fn update(&mut self, host: &mut dyn Host, pad: u32) {
        if self.frame == 0 {
            if let Some(bytes) = host.read(SETTINGS_FILE) {
                self.settings = Settings::parse(&bytes, self.settings.clone());
                self.theme = theme_by_id(self.settings.theme);
            }
            let started = host.time_us();
            self.library = Some(Library::load(host));
            host.log(&format!("library loaded in {} ms", (host.time_us() - started) / 1000));
            host.scan();
        }
        if self.frame % STATUS_EVERY == 0 {
            self.status = host.status();
        }
        self.frame += 1;
        if let Some(scan) = &mut self.scan {
            if let Some(frames) = &mut scan.finished {
                *frames += 1;
                if *frames >= SCAN_DONE_FRAMES {
                    self.scan = None;
                }
            }
            // The page takes no buttons.
            self.previous = pad;
            return;
        }

        let pressed = pad & !self.previous;
        let directions = pad & (UP | DOWN | LEFT | RIGHT);
        self.held = if directions != 0 && directions == self.previous & (UP | DOWN | LEFT | RIGHT) { self.held + 1 } else { 0 };
        let repeat = self.held >= HOLD_DELAY && (self.held - HOLD_DELAY) % REPEAT == 0;
        let fire = |bit: u32| pressed & bit != 0 || (repeat && pad & bit != 0);
        let wrap = pressed & directions != 0;
        let before = (self.tab.clone(), self.selected);
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
        if pressed & L != 0 {
            self.switch_tab(-1);
        }
        if pressed & R != 0 {
            self.switch_tab(1);
        }
        if pressed & TRIANGLE != 0 {
            self.stress = !self.stress;
            let started = host.time_us();
            self.set_titles(None);
            host.log(&format!("library: {} titles in {} us", self.len(), host.time_us() - started));
        }
        if pressed & SQUARE != 0 {
            let at = ThemeId::ALL.iter().position(|&id| id == self.theme.id).unwrap_or(0);
            self.theme = theme_by_id(ThemeId::ALL[(at + 1) % ThemeId::ALL.len()]);
        }
        if pressed & SELECT != 0 {
            self.stats.reset();
        }
        if pressed & START != 0 {
            self.quit = true;
        }
        self.previous = pad;
        self.rested = if (self.tab.as_str(), self.selected) == (before.0.as_str(), before.1) { self.rested + 1 } else { 0 };

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
        if self.view.is_empty() {
            return;
        }
        let first = self.top_row.saturating_sub(ICON_MARGIN_ROWS);
        let last = (self.top_row + 2 + ICON_MARGIN_ROWS).min(self.rows());
        // The selected tile first, then the rest in reading order.
        let order: Vec<usize> = std::iter::once(self.selected).chain(first * COLS..(last * COLS).min(self.view.len())).collect();
        for index in order {
            let Some(id) = self.game(index).map(|g| g.id.to_string()) else { continue };
            let key = PictureKey::Icon(id);
            if !self.requested.contains(&key) {
                host.request(key.clone());
                self.requested.insert(key);
            }
        }
        if self.rested == BACKDROP_REST {
            let Some(id) = self.selected_id().map(String::from) else { return };
            let key = PictureKey::Backdrop(id.clone());
            if self.backdrop.as_ref().is_none_or(|b| b.id != id) && !self.requested.contains(&key) {
                host.request(key.clone());
                self.requested.insert(key);
            }
        }
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
        if self.scan.is_some() {
            self.draw_scan(gfx);
            return;
        }
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
        if self.view.is_empty() {
            let line = if self.scanned.is_some() { "No titles found" } else { "Reading titles…" };
            gfx.text_centered(Role::Body, W / 2.0, H / 2.0, theme.dim, line);
            return;
        }
        self.draw_grid(gfx);
        self.draw_details(gfx);
        self.draw_footer(gfx);
    }

    /// The first start's page (`components/scan-page.tsx`): the mark, the
    /// name, what is happening, a bar and the phase with its count.
    fn draw_scan(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let Some(scan) = &self.scan else { return };
        let (done, total, phase) = (scan.done, scan.total, scan.phase);
        // A column of: mark 48, a 2 unit gap, heading, body, bar 6, small
        // line, 10 apart, centered.
        let lines = [gfx.line_height(Role::Heading), gfx.line_height(Role::Body), gfx.line_height(Role::Small)];
        let height = 48.0 + 2.0 + lines.iter().sum::<f32>() + 6.0 + 10.0 * 5.0;
        let mut y = ((H - height) / 2.0).round();
        self.draw_mark(gfx, (W - 110.0) / 2.0, y);
        y += 48.0 + 10.0 + 2.0 + 10.0;
        gfx.text_top_centered(Role::Heading, W / 2.0, y, theme.text, "Pocket Shelf");
        y += lines[0] + 10.0;
        gfx.text_top_centered(Role::Body, W / 2.0, y, theme.dim, "Finding your games and apps");
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
        if let (Some(phase), None) = (phase, self.scan.as_ref().and_then(|s| s.finished)) {
            let step = format!("{}: {done} of {total}", phase.label());
            gfx.text_top_centered(Role::Small, W / 2.0, y, theme.faint, &step);
        }
    }

    /// The app's mark: the Home view in small, a selected tile in its ring
    /// and two smaller tiles after it (`MARK` in `src/tools/art.ts`).
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

    fn draw_header(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let end = gfx.text(Role::Title, 12.0, 20.0, theme.text, "Pocket Shelf");
        let end = gfx.text(Role::Small, end + 5.0, 20.0, theme.accent, "Next");
        // The tab: its name, its place among the tabs.
        if let Some(index) = self.tabs.iter().position(|c| c.id == self.tab) {
            let label = gfx.fit(Role::Title, &self.tabs[index].label, 110.0);
            let end = gfx.text(Role::Title, end + 14.0, 20.0, theme.text, &label);
            gfx.text(Role::Small, end + 5.0, 20.0, theme.faint, &format!("{}/{}", index + 1, self.tabs.len()));
        }
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
        let end = (last_row * COLS).min(self.view.len());
        for index in first_row * COLS..end {
            // Whole screen pixels, so a 128 px icon in a 64-unit tile is 1:1.
            let rect = self.tile_rect(index).scale(gfx.scale).round().scale(1.0 / gfx.scale);
            let Some(game) = self.game(index) else { continue };
            let icon = self.icons.get(&*game.id).filter(|icon| gfx.textures.ready(icon.tex)).map(|icon| (icon.tex, icon.opaque));
            match icon {
                Some((tex, opaque)) => {
                    if opaque {
                        gfx.shadow(rect, TILE_RADIUS, 6.0, BLACK.with_a(110));
                    } else {
                        gfx.round_rect(rect, TILE_RADIUS, 0.0, theme.tile);
                    }
                    gfx.image_full(tex, rect, Color::rgb(0xffffff));
                }
                None => {
                    let initials = initials(&game.title);
                    gfx.round_rect(rect, TILE_RADIUS, 0.0, theme.tile);
                    gfx.text_centered(Role::Heading, rect.x + TILE / 2.0, rect.y + TILE / 2.0 + 6.0, theme.dim, &initials);
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
        let Some(library) = &self.library else { return };
        let Some(game) = self.game(self.selected) else { return };
        let category = library.all_categories().iter().find(|c| *c.id == *game.category).map_or(&*game.category, |c| c.label.as_str());
        // Platform, category, the id (not for a RetroArch game, where it
        // would repeat "RetroArch"), and the favorite mark in words: Figtree
        // has no star.
        let mut parts = vec![game.platform.genre(), category];
        if game.shown_id() != game.platform.genre() {
            parts.push(game.shown_id());
        }
        if game.favorite {
            parts.push("Favorite");
        }
        parts.dedup();
        let line = parts.join(" · ");
        let title = game.title.clone();
        let title = gfx.fit(Role::Heading, &title, W - 40.0);
        gfx.text_centered(Role::Heading, W / 2.0, 222.0, theme.text, &title);
        gfx.text_centered(Role::Small, W / 2.0, 238.0, theme.dim, &line);
    }

    fn draw_footer(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let baseline = 262.0;
        let mut x = 12.0;
        let mut hint = |gfx: &mut Gfx, symbol: Option<(Symbol, Color)>, key: &str, label: &str| {
            match symbol {
                Some((symbol, color)) => {
                    gfx.symbol(symbol, x, baseline - 9.0, 10.0, color);
                    x += 13.0;
                }
                None => {
                    let width = gfx.measure(Role::Label, key) + 10.0;
                    gfx.round_rect(Rect::new(x, baseline - 11.0, width, 14.0), 7.0, 0.0, theme.text.with_a(0x2a));
                    gfx.text(Role::Label, x + 5.0, baseline - 0.5, theme.text, key);
                    x += width + 4.0;
                }
            }
            x = gfx.text(Role::Body, x, baseline, theme.text.alpha(0.85), label) + 12.0;
        };
        hint(gfx, None, "L R", "Tabs");
        hint(gfx, Some((Symbol::Square, Color::rgb(0xf38bc8))), "", "Theme");
        hint(gfx, Some((Symbol::Triangle, Color::rgb(0x40d6b4))), "", if self.stress { "Remove tests" } else { "5,000 tests" });
        hint(gfx, None, "SELECT", "Timing");
        hint(gfx, None, "START", "Quit");
        let position = format!("{} / {}", self.selected + 1, self.view.len());
        gfx.text_right(Role::Body, W - 12.0, baseline, theme.dim, &position);
    }
}
