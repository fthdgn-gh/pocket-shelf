//! The shelf: its layout, navigation, header, footer, status items and the
//! five views (`src/layout.ts`, `navigation.ts`, `views/`, `components/`),
//! drawn from its state every frame.

pub mod art;
pub mod chrome;
pub mod layout;
pub mod navigation;
pub mod panel;
pub mod texts;
pub mod tween;
mod views;

use std::collections::HashMap;

use art::Art;
use chrome::{FooterInput, Header, HeaderInput};
use navigation::{icon_radius, move_selection, move_selection_by, Direction, HeldDirection, Step};
use tween::Tween;

use crate::app::buttons::*;
use crate::gfx::{Color, Format, Gfx, Rect, TexId};
use crate::host::{Host, Picture, PictureKey, PluginState, Status};
use crate::library::catalog::{Game, TINT_COLORS};
use crate::library::categories::{cycle_category, Category, RECENT_ID, SEARCH_ID};
use crate::library::recent::{push_recent, RECENT_FILE, RECENT_MAX};
use crate::library::settings::SETTINGS_FILE;
use crate::library::overrides::NO_BACKDROP;
use crate::library::settings::{ConfirmMode, DetailLevel, Settings, ViewMode};
use crate::library::Library;
use crate::theme::{dynamic_theme, theme_by_id, Theme, ThemeId};
use crate::{H, W};

/// Frames the selection rests before its backdrop is asked for (0.25 s).
const BACKDROP_DELAY: u32 = 15;
/// Backdrops kept after they leave the screen, newest first, so going
/// back to a title shows its picture at once (the current app keeps three
/// idle ones, `art-files.ts`). A backdrop is 512 KB.
const BACKDROPS_KEPT: usize = 3;
/// A backdrop fades in over this long once it is on the GPU.
const BACKDROP_FADE_US: u64 = 350_000;
/// Frames the selection rests before the Dynamic theme starts blending,
/// and frames between its steps.
const DYNAMIC_SETTLE_FRAMES: u32 = 10;
const DYNAMIC_STEP_FRAMES: u32 = 2;
/// Frames between status readings.
const STATUS_EVERY: u32 = 60;
/// The tint wash over the top of the screen (`TINT_AMBIENT`): its height and
/// its alpha at the top; it blends to the next title's tint in 300 ms.
const AMBIENT_H: f32 = 190.0;
const AMBIENT_ALPHA: u8 = 0x55;

/// A backdrop on screen.
struct Shown {
    key: PictureKey,
    tex: TexId,
    /// When it was first drawn complete, for the fade.
    ready_at: Option<u64>,
}

/// One tile's motion: where it is, eased toward where its slot puts it.
struct TileAnim {
    x: Tween,
    y: Tween,
    scale: Tween,
    opacity: Tween,
    shade: Tween,
}

/// What a tile's slot asks for this frame.
#[derive(Clone, Copy, Default)]
struct Slot {
    x: f32,
    y: f32,
    scale: f32,
    opacity: f32,
    shade: f32,
}

impl TileAnim {
    fn new(slot: Slot, ms: u32) -> TileAnim {
        TileAnim {
            x: Tween::new(slot.x, ms),
            y: Tween::new(slot.y, ms),
            scale: Tween::new(slot.scale, ms),
            opacity: Tween::new(slot.opacity, ms),
            shade: Tween::new(slot.shade, ms),
        }
    }

    fn step(&mut self, slot: Slot, now: u64) -> Slot {
        self.x.set(slot.x, now);
        self.y.set(slot.y, now);
        self.scale.set(slot.scale, now);
        self.opacity.set(slot.opacity, now);
        self.shade.set(slot.shade, now);
        Slot { x: self.x.at(now), y: self.y.at(now), scale: self.scale.at(now), opacity: self.opacity.at(now), shade: self.shade.at(now) }
    }
}

/// What a frame asks the platform to do after it is drawn.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Request {
    /// Start this title (phase 3's launch, `installed::launch`).
    Launch(String),
}

pub struct Shelf {
    pub settings: Settings,
    theme: Theme,
    pub library: Library,
    tabs: Vec<Category>,
    tab: String,
    /// The current tab's titles, as indexes into the library's games.
    list: Vec<usize>,
    selected: usize,

    held: HeldDirection,
    previous: u32,
    /// The view and tab the tiles were placed for; a change starts over.
    placed: Option<(ViewMode, DetailLevel, String)>,
    tiles: HashMap<usize, TileAnim>,
    /// The row's entrance, 0 to 1, when a view or a tab appears.
    intro: Tween,
    /// The carousel's strip x, the grid's and the list's scroll.
    strip: Tween,
    top_row: usize,

    header: Header,
    art: Art,
    backdrop: Option<Shown>,
    under: Option<Shown>,
    /// Backdrops off screen, newest first; trimmed to `BACKDROPS_KEPT`
    /// when drawing.
    kept: Vec<Shown>,
    backdrop_wait: u32,
    /// The backdrop the selection asks for.
    wanted: Option<PictureKey>,
    /// Washes over a backdrop: its colors and texture.
    wash: Option<((Color, Color), TexId)>,
    /// An alpha ramp (255 at the top to 0), tinted for the tint wash and the
    /// carousel's glow.
    ramp: Option<TexId>,
    ambient: (Color, Color, Tween),
    background: Option<((Color, Color), TexId)>,

    dynamic: Option<[f32; 3]>,
    dynamic_goal: Option<u32>,
    dynamic_rest: u32,

    status: Status,
    frame: u32,
    now: u64,
    launching: Option<String>,
    /// Test titles added (TRIANGLE, until the editor takes the button).
    stress: bool,
    /// The backdrop is to be cleared at the next draw.
    clear_pending: bool,
    /// The Adrenaline drawer: the plugin's state, and whether turning it on
    /// failed.
    adrenaline: Option<(PluginState, bool)>,
    drawer: Tween,
}

impl Shelf {
    pub fn new(settings: Settings, library: Library, status: Status) -> Shelf {
        let theme = theme_by_id(settings.theme);
        let mut shelf = Shelf {
            theme,
            settings,
            library,
            tabs: Vec::new(),
            tab: String::new(),
            list: Vec::new(),
            selected: 0,
            held: HeldDirection::default(),
            previous: 0,
            placed: None,
            tiles: HashMap::new(),
            intro: Tween::new(1.0, 240),
            strip: Tween::new(0.0, 220),
            top_row: 0,
            header: Header::default(),
            art: Art::default(),
            backdrop: None,
            under: None,
            kept: Vec::new(),
            backdrop_wait: 0,
            wanted: None,
            wash: None,
            ramp: None,
            ambient: (Color::rgba(0), Color::rgba(0), Tween::new(1.0, 300)),
            background: None,
            dynamic: None,
            dynamic_goal: None,
            dynamic_rest: 0,
            status,
            frame: 0,
            now: 0,
            launching: None,
            stress: false,
            clear_pending: false,
            adrenaline: None,
            drawer: panel::slide(),
        };
        shelf.tab = shelf.settings.category.clone().unwrap_or_default();
        shelf.refresh();
        shelf
    }

    pub fn games(&self) -> &[Game] {
        self.library.games()
    }

    fn game(&self, index: usize) -> Option<&Game> {
        self.list.get(index).map(|&i| &self.library.games()[i])
    }

    fn selected_game(&self) -> Option<&Game> {
        self.game(self.selected)
    }

    /// The tabs and the current tab's titles, after the library changed;
    /// the first tab when the current one is gone.
    fn refresh(&mut self) {
        self.tabs = self.library.tabs();
        if !self.tabs.iter().any(|c| c.id == self.tab) {
            self.tab = self.tabs.first().map(|c| c.id.clone()).unwrap_or_default();
            self.selected = 0;
        }
        self.list = self.library.titles_of(&self.tab);
        self.selected = self.selected.min(self.list.len().saturating_sub(1));
        self.placed = None;
    }

    /// New titles from the host. On the first list the saved selection is
    /// restored; later, the selected title keeps its place.
    pub fn set_titles(&mut self, games: Vec<Game>, first: bool) {
        let keep = if first { self.settings.title.clone() } else { self.selected_game().map(|g| g.id.to_string()) };
        self.library.set_catalog(games);
        self.refresh();
        if let Some(id) = keep {
            if let Some(index) = self.list.iter().position(|&i| *self.library.games()[i].id == id) {
                self.selected = index;
            }
        }
    }

    /// The host's answer for a picture: an icon or box art, or a backdrop.
    pub fn picture(&mut self, gfx: &mut Gfx, key: PictureKey, picture: Option<Picture>) {
        if matches!(key, PictureKey::Icon(_) | PictureKey::Art(_)) {
            self.art.answer(gfx, &key, picture);
            return;
        }
        // A backdrop the selection no longer asks for is dropped.
        if self.wanted.as_ref() != Some(&key) {
            return;
        }
        match picture {
            None => self.clear_backdrop(gfx),
            Some(_) if self.backdrop.as_ref().is_some_and(|b| b.key == key) => {}
            Some(picture) => {
                let tex = gfx.add_texture(picture.w, picture.h, Format::Rgba8, picture.rgba, true);
                self.show_backdrop(Shown { key, tex, ready_at: None });
            }
        }
    }

    /// Put a backdrop on screen: the current one goes under, the one
    /// underneath is kept for later.
    fn show_backdrop(&mut self, shown: Shown) {
        if let Some(old) = self.under.take() {
            self.kept.insert(0, old);
        }
        self.under = self.backdrop.take();
        self.backdrop = Some(shown);
    }

    /// No backdrop on screen; the ones that were are kept for later.
    fn clear_backdrop(&mut self, _gfx: &mut Gfx) {
        for shown in [self.backdrop.take(), self.under.take()].into_iter().flatten() {
            self.kept.insert(0, shown);
        }
    }
}

/// The view after `view` in the View row's order, and the same for details
/// and themes (until the SELECT menu, phase 4, sets them).
fn next<T: Copy + PartialEq>(all: &[T], current: T) -> T {
    let at = all.iter().position(|&v| v == current).unwrap_or(0);
    all[(at + 1) % all.len()]
}

impl Shelf {
    /// One frame of input and loading. `now` is the host's clock in µs.
    pub fn update(&mut self, host: &mut dyn Host, pad: u32, now: u64) -> Option<Request> {
        self.now = now;
        // The status arrives with the shelf; it is read again every second.
        if self.frame > 0 && self.frame % STATUS_EVERY == 0 && self.settings.status_bar {
            self.status = host.status();
        }
        self.frame += 1;
        let pressed = pad & !self.previous;
        self.previous = pad;
        let before = (self.tab.clone(), self.selected_game().map(|g| (g.id.clone(), g.backdrop.clone())));

        // The Adrenaline drawer takes the buttons while it is open.
        if self.adrenaline.is_some() {
            let (confirm, cancel) = if self.settings.confirm == ConfirmMode::Circle { (CIRCLE, CROSS) } else { (CROSS, CIRCLE) };
            self.held.update(None);
            if pressed & confirm != 0 {
                return self.adrenaline_confirm(host);
            }
            if pressed & cancel != 0 {
                self.adrenaline = None;
            }
            return None;
        }

        let direction = [(LEFT, Direction::Left), (RIGHT, Direction::Right), (UP, Direction::Up), (DOWN, Direction::Down)]
            .into_iter()
            .find(|(bit, _)| pad & bit != 0)
            .map(|(_, d)| d);
        let count = self.list.len();
        let target = match self.held.update(direction) {
            // A press wraps; a held direction stops at the ends.
            Some(Step::Press(d)) => move_selection(self.settings.view, d, self.selected, count, true),
            Some(Step::Repeat(d, steps)) => move_selection_by(self.settings.view, d, self.selected, count, steps),
            None => None,
        };
        if let Some(index) = target {
            self.select(index);
        }
        if pressed & L != 0 {
            self.change_category(-1);
        }
        if pressed & R != 0 {
            self.change_category(1);
        }
        // The other face button closes the search (phase 4).
        let confirm = if self.settings.confirm == ConfirmMode::Circle { CIRCLE } else { CROSS };
        let mut request = None;
        if pressed & confirm != 0 {
            request = self.launch_selected(host);
        }
        // Until the SELECT menu (phase 4): SELECT the view, START the
        // details, SQUARE the theme, TRIANGLE 5,000 test titles.
        if pressed & SELECT != 0 {
            self.settings.view = next(ViewMode::ALL, self.settings.view);
            self.placed = None;
            self.persist(host);
        }
        if pressed & START != 0 {
            self.settings.detail = next(DetailLevel::ALL, self.settings.detail);
            self.placed = None;
            self.persist(host);
        }
        if pressed & SQUARE != 0 {
            self.settings.theme = next(&ThemeId::ALL, self.settings.theme);
            self.dynamic = None;
            self.persist(host);
        }
        if pressed & TRIANGLE != 0 {
            self.toggle_stress(host);
        }

        let after = (self.tab.clone(), self.selected_game().map(|g| (g.id.clone(), g.backdrop.clone())));
        // The "Launching" line goes when the selection moves; the backdrop
        // is asked for once it rests, from the first frame on.
        if after != before {
            self.launching = None;
        }
        if after != before || self.frame == 1 {
            self.backdrop_wait = if self.settings.backdrop { BACKDROP_DELAY } else { 0 };
        }
        if self.backdrop_wait > 0 {
            self.backdrop_wait -= 1;
            if self.backdrop_wait == 0 {
                self.ask_backdrop(host);
            }
        }
        self.request_icons(host);
        self.blend_dynamic();
        request
    }

    fn select(&mut self, index: usize) {
        if index != self.selected {
            self.selected = index;
            self.launching = None;
        }
    }

    /// The previous or next tab, wrapping.
    fn change_category(&mut self, delta: i32) {
        let ids: Vec<String> = self.tabs.iter().map(|c| c.id.clone()).collect();
        let next = cycle_category(&ids, &self.tab, delta).to_string();
        if next != self.tab {
            self.tab = next;
            self.selected = 0;
            self.refresh();
        }
    }

    fn toggle_stress(&mut self, host: &mut dyn Host) {
        use crate::host::TitleInfo;
        use crate::library::catalog::load_catalog;
        self.stress = !self.stress;
        let started = host.time_us();
        let games: Vec<Game> = if self.stress {
            let tests: Vec<TitleInfo> = (1..=5000)
                .map(|n| TitleInfo { id: format!("TEST{n:05}"), title: format!("Test Title {n}"), platform: "vita".into(), adrenaline: false, console: None })
                .collect();
            self.library.base().iter().cloned().chain(load_catalog(&tests)).collect()
        } else {
            self.library.base().iter().filter(|g| !g.id.starts_with("TEST")).cloned().collect()
        };
        self.set_titles(games, false);
        host.log(&format!("library: {} titles in {} us", self.library.games().len(), host.time_us() - started));
    }

    /// Save the settings with the current selection (`persist` in
    /// `state.ts`). A search result is saved under its own category, since
    /// the search tab does not outlive the process.
    fn persist(&mut self, host: &mut dyn Host) {
        let game = self.selected_game().cloned();
        self.settings.category = Some(if self.tab == SEARCH_ID {
            game.as_ref().map(|g| g.category.to_string()).unwrap_or_default()
        } else {
            self.tab.clone()
        });
        self.settings.title = game.map(|g| g.id.to_string());
        if !host.write(SETTINGS_FILE, self.settings.render().as_bytes()) {
            host.log("Settings not saved");
        }
    }

    /// Start the selected title (`launchSelected` in `state.ts`). A title
    /// with no bubble first needs Adrenaline's boot plugin on. "Last
    /// Played" and the settings are written before the request: the
    /// platform ends this process once this frame is on screen, and a
    /// process still alive when the system takes the request gets the
    /// "close this application?" prompt.
    fn launch_selected(&mut self, host: &mut dyn Host) -> Option<Request> {
        let game = self.selected_game()?.clone();
        if game.adrenaline {
            let state = host.plugin_state();
            if state != PluginState::On {
                self.adrenaline = Some((state, false));
                self.drawer = panel::slide();
                self.drawer.set(1.0, self.now);
                return None;
            }
        }
        self.launching = Some(game.title.to_string());
        let before = self.library.recent.clone();
        let list = push_recent(&before, &game.id, RECENT_MAX);
        host.write(RECENT_FILE, crate::library::recent::render(&list).as_bytes());
        self.persist(host);
        host.log(&format!("Launch requested: {}", game.id));
        if !host.launch(&game.id) {
            host.log(&format!("Launch rejected: {}", game.id));
            host.write(RECENT_FILE, crate::library::recent::render(&before).as_bytes());
            return None;
        }
        self.library.recent = list;
        // In "Last Played" the started title moves to the front; follow it.
        if self.tab == RECENT_ID {
            self.list = self.library.titles_of(&self.tab);
            self.selected = 0;
        }
        Some(Request::Launch(game.id.to_string()))
    }

    /// The drawer's confirm: add or turn on the plugin, then start the title.
    fn adrenaline_confirm(&mut self, host: &mut dyn Host) -> Option<Request> {
        let (state, _) = self.adrenaline?;
        if state != PluginState::Missing && state != PluginState::Off {
            self.adrenaline = None;
            return None;
        }
        if !host.enable_plugin() {
            self.adrenaline = Some((state, true));
            return None;
        }
        self.adrenaline = None;
        self.launch_selected(host)
    }

    /// Icons for the titles near the selection, the selected one first.
    fn request_icons(&mut self, host: &mut dyn Host) {
        if self.list.is_empty() {
            return;
        }
        let radius = icon_radius(self.settings.view);
        let from = self.selected.saturating_sub(radius);
        let to = (self.selected + radius + 1).min(self.list.len());
        let games = self.library.games();
        let list = &self.list;
        let order = std::iter::once(self.selected).chain(from..to).map(|index| &games[list[index]]);
        self.art.want(host, order);
    }

    /// Ask for the selected title's backdrop, or clear it for a title set
    /// to none.
    fn ask_backdrop(&mut self, host: &mut dyn Host) {
        let Some(game) = self.selected_game() else { return };
        if game.backdrop.as_deref() == Some(NO_BACKDROP) {
            self.wanted = None;
            // Cleared at the next draw, which has the textures.
            self.backdrop_wait = 0;
            self.clear_pending = true;
            return;
        }
        let key = PictureKey::Backdrop { id: game.id.to_string(), file: game.backdrop.clone().unwrap_or_default() };
        self.wanted = Some(key.clone());
        if self.backdrop.as_ref().is_some_and(|b| b.key == key) {
            return;
        }
        // Kept from earlier (or still underneath): shown again at once.
        if self.under.as_ref().is_some_and(|b| b.key == key) {
            std::mem::swap(&mut self.under, &mut self.backdrop);
            if let Some(shown) = &mut self.backdrop {
                shown.ready_at = None;
            }
            return;
        }
        if let Some(at) = self.kept.iter().position(|b| b.key == key) {
            let mut shown = self.kept.remove(at);
            shown.ready_at = None;
            self.show_backdrop(shown);
            return;
        }
        host.request(key);
    }

    /// The Dynamic theme moves toward the selected title's color: two fifths
    /// of the way every 2 frames once the selection has rested 10.
    fn blend_dynamic(&mut self) {
        if self.settings.theme != ThemeId::Dynamic {
            self.theme = theme_by_id(self.settings.theme);
            return;
        }
        let Some(game) = self.selected_game() else { return };
        let target = self.art.color(game).unwrap_or(TINT_COLORS[game.tint as usize % TINT_COLORS.len()]);
        if Some(target) != self.dynamic_goal {
            self.dynamic_goal = Some(target);
            self.dynamic_rest = 0;
        }
        let goal = [(target >> 16) as f32, ((target >> 8) & 255) as f32, (target & 255) as f32];
        let Some(now) = self.dynamic else {
            self.dynamic = Some(goal);
            self.theme = dynamic_theme(target);
            return;
        };
        if now == goal {
            return;
        }
        self.dynamic_rest += 1;
        if self.dynamic_rest < DYNAMIC_SETTLE_FRAMES || (self.dynamic_rest - DYNAMIC_SETTLE_FRAMES) % DYNAMIC_STEP_FRAMES != 0 {
            return;
        }
        let mut next = now;
        for c in 0..3 {
            let gap = goal[c] - now[c];
            next[c] = if gap.abs() < 8.0 { goal[c] } else { now[c] + gap * 0.4 };
        }
        self.dynamic = Some(next);
        let rgb = ((next[0].round() as u32) << 16) | ((next[1].round() as u32) << 8) | next[2].round() as u32;
        self.theme = dynamic_theme(rgb);
    }

    /// Whether anything on screen is still moving (for a host that wants
    /// to know when frames are idle).
    pub fn moving(&self) -> bool {
        self.header.moving(self.now) || self.intro.moving(self.now) || self.strip.moving(self.now)
    }

    pub fn draw(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        if self.clear_pending {
            self.clear_pending = false;
            self.clear_backdrop(gfx);
        }
        // Free the backdrops kept beyond the newest few; they are not drawn.
        while self.kept.len() > BACKDROPS_KEPT {
            if let Some(old) = self.kept.pop() {
                gfx.textures.free(old.tex);
            }
        }
        let colors = (theme.bg_top, theme.bg_bottom);
        let background = match self.background {
            Some((c, tex)) if c == colors => tex,
            old => {
                if let Some((_, tex)) = old {
                    gfx.textures.free(tex);
                }
                let tex = gfx.gradient(theme.bg_top, theme.bg_bottom);
                self.background = Some((colors, tex));
                tex
            }
        };
        gfx.image_full(background, Rect::new(0.0, 0.0, W, H), Color::rgb(0xffffff));
        self.draw_backdrop(gfx);

        let input = HeaderInput {
            theme: &theme,
            settings: &self.settings,
            status: &self.status,
            tabs: &self.tabs,
            current: &self.tab,
            now: self.now,
        };
        self.header.draw(gfx, &input);
        self.draw_view(gfx);
        let footer = FooterInput {
            theme: &theme,
            settings: &self.settings,
            own_launch: self.settings.view == ViewMode::Home && self.settings.detail != DetailLevel::Basic,
            search_term: &self.library.search_term,
            launching: self.launching.as_deref(),
            position: self.selected,
            count: self.list.len(),
        };
        chrome::footer(gfx, &footer);
        if let Some((state, failed)) = self.adrenaline {
            self.draw_adrenaline(gfx, state, failed);
        }
    }

    /// Shown when a title with no bubble is started and Adrenaline would not
    /// load the boot plugin (`components/adrenaline.tsx`): add it, turn it
    /// on, or (no Adrenaline) only say so.
    fn draw_adrenaline(&mut self, gfx: &mut Gfx, state: PluginState, failed: bool) {
        let theme = self.theme;
        let title = self.selected_game().map(|g| g.title.to_string()).unwrap_or_default();
        let slide = self.drawer.at(self.now);
        let mut drawer = panel::Drawer::open(gfx, &theme, texts::ADR_TITLE, panel::Size::Wide, slide);
        let can_fix = matches!(state, PluginState::Missing | PluginState::Off);
        let action = if state == PluginState::Off { texts::ADR_TURN_ON } else { texts::ADR_ADD };
        if can_fix {
            drawer.row(gfx, action, None, true, true);
        }
        let note = if failed { texts::ADR_FAILED.to_string() } else { texts::adrenaline(state, &title) };
        drawer.note(gfx, &note, 1);
        let confirm = chrome::Button::confirm(self.settings.confirm);
        let cancel = chrome::Button::cancel(self.settings.confirm);
        if can_fix {
            drawer.hints(gfx, &[(confirm, action), (cancel, texts::BACK)]);
        } else {
            drawer.hints(gfx, &[(cancel, texts::BACK)]);
        }
    }

    /// An alpha ramp, 255 at the top to 0, tinted when drawn.
    fn ramp(&mut self, gfx: &mut Gfx) -> TexId {
        *self.ramp.get_or_insert_with(|| {
            let pixels: Vec<u8> = (0..64u32).flat_map(|y| std::iter::repeat_n((255 - y * 255 / 63) as u8, 8)).collect();
            gfx.add_texture(8, 64, Format::A8, pixels, true)
        })
    }

    /// The selected title's picture under a wash of the theme's background
    /// (`components/backdrop.tsx`), or with none, its tint over the top of
    /// the screen (not in the Dynamic theme, which takes the title's color).
    fn draw_backdrop(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let full = Rect::new(0.0, 0.0, W, H);
        let white = Color::rgb(0xffffff);
        let now = self.now;
        let mut shown_any = false;
        if self.settings.backdrop {
            if let Some(under) = &self.under {
                if gfx.textures.ready(under.tex) {
                    gfx.image(under.tex, tex_rect(gfx, under.tex), full, white);
                    shown_any = true;
                }
            }
            if let Some(current) = &mut self.backdrop {
                if gfx.textures.ready(current.tex) {
                    let since = *current.ready_at.get_or_insert(now);
                    let t = (now.saturating_sub(since) as f32 / BACKDROP_FADE_US as f32).min(1.0);
                    let opacity = 1.0 - (1.0 - t).powi(3);
                    gfx.image(current.tex, tex_rect(gfx, current.tex), full, white.alpha(opacity));
                    shown_any = true;
                }
            }
        }
        if shown_any {
            // Views that leave most of the screen open get a wash clearest at
            // the top; the grid and the list an even, darker one.
            let open = matches!(self.settings.view, ViewMode::Carousel | ViewMode::Cascade | ViewMode::Home);
            let colors = if open { (theme.bg_top.with_a(0x73), theme.bg_bottom.with_a(0xf2)) } else { (theme.bg_top.with_a(0xc7), theme.bg_bottom.with_a(0xf5)) };
            let wash = match self.wash {
                Some((c, tex)) if c == colors => tex,
                old => {
                    if let Some((_, tex)) = old {
                        gfx.textures.free(tex);
                    }
                    let tex = gfx.gradient_sized(colors.0, colors.1, 64);
                    self.wash = Some((colors, tex));
                    tex
                }
            };
            gfx.image_full(wash, full, white);
            return;
        }
        if theme.id == ThemeId::Dynamic {
            return;
        }
        let Some(game) = self.selected_game() else { return };
        let tint = Color::rgb(TINT_COLORS[game.tint as usize % TINT_COLORS.len()]);
        let (from, to, tween) = &mut self.ambient;
        if *to != tint {
            *from = if to.a == 0 { tint } else { from.lerp(*to, tween.at(now)) };
            *to = tint;
            tween.jump(0.0);
            tween.set(1.0, now);
        }
        let color = from.lerp(*to, tween.at(now)).with_a(AMBIENT_ALPHA);
        let ramp = self.ramp(gfx);
        gfx.image_full(ramp, Rect::new(0.0, 0.0, W, AMBIENT_H), color);
    }
}

/// A texture's whole area, as a source rectangle.
fn tex_rect(gfx: &Gfx, tex: TexId) -> Rect {
    let t = gfx.textures.get(tex);
    Rect::new(0.0, 0.0, t.w as f32, t.h as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{Event, TitleInfo};
    use crate::library::catalog::load_catalog;
    use crate::library::settings::UpdateChannel;
    use crate::library::testing::Files;

    /// A data folder in memory that accepts or refuses launches.
    struct TestHost {
        files: Files,
        accept: bool,
        plugin: PluginState,
        launched: Vec<String>,
        backdrops: Vec<PictureKey>,
    }

    impl Host for TestHost {
        fn time_us(&self) -> u64 {
            0
        }
        fn read(&mut self, path: &str) -> Option<Vec<u8>> {
            self.files.read(path)
        }
        fn write(&mut self, path: &str, data: &[u8]) -> bool {
            self.files.write(path, data)
        }
        fn remove(&mut self, path: &str) -> bool {
            self.files.remove(path)
        }
        fn list(&mut self, dir: &str) -> Vec<String> {
            self.files.list(dir)
        }
        fn status(&mut self) -> Status {
            Status::default()
        }
        fn scan(&mut self) {}
        fn request(&mut self, key: PictureKey) {
            if matches!(key, PictureKey::Backdrop { .. }) {
                self.backdrops.push(key);
            }
        }
        fn poll(&mut self, _: &mut Vec<Event>) {}
        fn log(&mut self, _: &str) {}
        fn launch(&mut self, id: &str) -> bool {
            self.launched.push(id.into());
            self.accept
        }
        fn plugin_state(&mut self) -> PluginState {
            self.plugin
        }
        fn enable_plugin(&mut self) -> bool {
            self.plugin = PluginState::On;
            true
        }
    }

    fn setup(accept: bool, plugin: PluginState) -> (Shelf, TestHost) {
        let mut host = TestHost { files: Files::default(), accept, plugin, launched: Vec::new(), backdrops: Vec::new() };
        host.files.0.insert(RECENT_FILE.into(), b"[\"VITASHELL\"]\n".to_vec());
        let library = Library::load(&mut host);
        let mut shelf = Shelf::new(Settings::defaults(UpdateChannel::Off), library, Status::default());
        let info = |id: &str, platform: &str, adrenaline: bool| TitleInfo {
            id: id.into(),
            title: format!("Title {id}"),
            platform: platform.into(),
            adrenaline,
            console: None,
        };
        let infos = [info("PCSA00069", "vita", false), info("VITASHELL", "vita", false), info("ULUS10041", "psp", true)];
        shelf.set_titles(load_catalog(&infos), true);
        (shelf, host)
    }

    fn press(shelf: &mut Shelf, host: &mut TestHost, button: u32) -> Option<Request> {
        let request = shelf.update(host, button, 0);
        shelf.update(host, 0, 0);
        request
    }

    #[test]
    fn a_launch_writes_last_played_and_the_settings_first() {
        let (mut shelf, mut host) = setup(true, PluginState::On);
        // "Last Played" is the first tab; Games holds PCSA00069.
        shelf.tab = "games".into();
        shelf.refresh();
        assert_eq!(press(&mut shelf, &mut host, CIRCLE), Some(Request::Launch("PCSA00069".into())));
        assert_eq!(host.files.0[RECENT_FILE], b"[\"PCSA00069\",\"VITASHELL\"]\n");
        let settings = String::from_utf8(host.files.0[SETTINGS_FILE].clone()).unwrap();
        assert!(settings.ends_with(r#""category":"games","title":"PCSA00069"}"#), "{settings}");
        assert_eq!(shelf.launching.as_deref(), Some("Title PCSA00069"));
    }

    #[test]
    fn a_rejected_launch_puts_last_played_back() {
        let (mut shelf, mut host) = setup(false, PluginState::On);
        assert_eq!(press(&mut shelf, &mut host, CIRCLE), None);
        assert_eq!(host.files.0[RECENT_FILE], b"[\"VITASHELL\"]\n");
        assert_eq!(shelf.library.recent, ["VITASHELL"]);
    }

    #[test]
    fn a_title_through_adrenaline_asks_for_the_plugin_first() {
        let (mut shelf, mut host) = setup(true, PluginState::Missing);
        shelf.tab = crate::library::categories::PSP_ID.into();
        shelf.refresh();
        assert_eq!(press(&mut shelf, &mut host, CIRCLE), None);
        assert!(shelf.adrenaline.is_some() && host.launched.is_empty());
        // Back closes it; confirm adds the plugin and launches.
        press(&mut shelf, &mut host, CROSS);
        assert!(shelf.adrenaline.is_none());
        press(&mut shelf, &mut host, CIRCLE);
        assert_eq!(press(&mut shelf, &mut host, CIRCLE), Some(Request::Launch("ULUS10041".into())));
        assert_eq!(host.plugin, PluginState::On);
    }

    /// Rest on the selection until its backdrop is asked for, and answer.
    fn rest(shelf: &mut Shelf, host: &mut TestHost, gfx: &mut Gfx) {
        for _ in 0..BACKDROP_DELAY + 1 {
            shelf.update(host, 0, 0);
        }
        if let Some(key) = host.backdrops.last().cloned().filter(|k| shelf.wanted.as_ref() == Some(k)) {
            if shelf.backdrop.as_ref().is_none_or(|b| b.key != key) {
                shelf.picture(gfx, key, Some(Picture { w: 4, h: 2, rgba: vec![0; 32], accent: None }));
            }
        }
        shelf.draw(gfx);
    }

    #[test]
    fn a_backdrop_seen_before_shows_again_without_decoding() {
        let (mut shelf, mut host) = setup(true, PluginState::On);
        let mut gfx = Gfx::new(2.0);
        // Last Played (VitaShell) first; R to Games (PCSA00069), L back.
        rest(&mut shelf, &mut host, &mut gfx);
        assert_eq!(host.backdrops.len(), 1);
        press(&mut shelf, &mut host, R);
        rest(&mut shelf, &mut host, &mut gfx);
        assert_eq!(host.backdrops.len(), 2);
        press(&mut shelf, &mut host, L);
        rest(&mut shelf, &mut host, &mut gfx);
        assert_eq!(host.backdrops.len(), 2, "no new request");
        assert_eq!(shelf.backdrop.as_ref().map(|b| b.key.clone()), Some(PictureKey::Backdrop { id: "VITASHELL".into(), file: String::new() }));
    }

    #[test]
    fn last_played_follows_the_started_title() {
        let (mut shelf, mut host) = setup(true, PluginState::On);
        shelf.tab = RECENT_ID.into();
        shelf.refresh();
        assert_eq!(shelf.list.len(), 1);
        shelf.set_titles(shelf.library.base().to_vec(), false);
        assert_eq!(press(&mut shelf, &mut host, CIRCLE), Some(Request::Launch("VITASHELL".into())));
        assert_eq!(shelf.selected, 0);
    }
}
