//! The shelf: its layout, navigation, header, footer, status items and the
//! five views (`src/layout.ts`, `navigation.ts`, `views/`, `components/`),
//! drawn from its state every frame.

pub mod art;
pub mod chrome;
mod editor;
pub mod keyboard;
pub mod layout;
pub mod menu;
pub mod navigation;
pub mod panel;
mod tools;
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
#[cfg(test)]
use crate::library::categories::CATEGORIES_FILE;
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
    /// The backdrop is to be cleared at the next draw.
    clear_pending: bool,
    /// The Adrenaline drawer: the plugin's state, and whether turning it on
    /// failed.
    adrenaline: Option<(PluginState, bool)>,
    drawer: Tween,
    menu: menu::Menu,
    keyboard: Option<keyboard::Keyboard>,
    /// The library search's count for the typed text, worked out when the
    /// text changes (searching 5,900 titles every frame would not fit one).
    found: Option<(String, usize)>,
    /// Where the selection was when the search opened; closing it goes back.
    before_search: Option<(String, Option<String>)>,
    editor: Option<editor::Editor>,
    picker: Option<editor::Picker>,
    categories: Option<editor::Categories>,
    /// A picker confirm that waited for its file to decode, now due.
    pending_picker_confirm: bool,
    rescan: Option<tools::Rescan>,
    /// Diagnostics' highlighted row while open, and its lines (the app's
    /// frame timing and startup marks, filled while it is open).
    diagnostics: Option<usize>,
    pub diagnostics_lines: Vec<(String, String)>,
    /// Diagnostics opened: the app resets its frame timing.
    pub reset_stats: bool,
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
            clear_pending: false,
            adrenaline: None,
            drawer: panel::slide(),
            menu: menu::Menu::default(),
            keyboard: None,
            found: None,
            before_search: None,
            editor: None,
            picker: None,
            categories: None,
            pending_picker_confirm: false,
            rescan: None,
            diagnostics: None,
            diagnostics_lines: Vec::new(),
            reset_stats: false,
        };
        shelf.tab = shelf.settings.category.clone().unwrap_or_default();
        shelf.refresh();
        shelf
    }

    pub fn games(&self) -> &[Game] {
        self.library.games()
    }

    /// The texts of the chosen language.
    pub fn t(&self) -> &'static crate::locales::Messages {
        crate::i18n::messages(self.settings.language)
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
        let t = self.t();
        self.tabs = self.library.tabs().into_iter().map(|c| Category { label: crate::i18n::category_label(t, &c), ..c }).collect();
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
        // The picker's preview: a backdrop is its own; an icon or box art
        // also goes to the art cache below.
        if self.picker_picture(gfx, &key, &picture) {
            return;
        }
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

/// The value `delta` steps from `current` in `all`, wrapping (`cycle` in
/// `state.ts`).
pub(super) fn cycle_by<T: Copy + PartialEq>(all: &[T], current: T, delta: i32) -> T {
    let at = all.iter().position(|&v| v == current).unwrap_or(0) as i32;
    all[(at + delta).rem_euclid(all.len() as i32) as usize]
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
        let (confirm, cancel) = if self.settings.confirm == ConfirmMode::Circle { (CIRCLE, CROSS) } else { (CROSS, CIRCLE) };
        let direction = [(LEFT, Direction::Left), (RIGHT, Direction::Right), (UP, Direction::Up), (DOWN, Direction::Down)]
            .into_iter()
            .find(|(bit, _)| pad & bit != 0)
            .map(|(_, d)| d);
        let step = self.held.update(direction);
        let mut request = None;

        if let Some(keyboard) = &mut self.keyboard {
            // The keyboard takes every button but SELECT; a held direction
            // repeats over the keys.
            match step {
                Some(Step::Press(d)) | Some(Step::Repeat(d, _)) => match d {
                    Direction::Left => keyboard.step(-1, 0),
                    Direction::Right => keyboard.step(1, 0),
                    Direction::Up => keyboard.step(0, -1),
                    Direction::Down => keyboard.step(0, 1),
                },
                None => {}
            }
            let mut done = false;
            if pressed & confirm != 0 {
                done = keyboard.press();
            }
            if pressed & SQUARE != 0 {
                keyboard.backspace();
            }
            if pressed & L != 0 {
                keyboard.toggle_shift();
            }
            if pressed & R != 0 {
                keyboard.next_page();
            }
            if pressed & START != 0 {
                done = true;
            }
            if done {
                self.commit_keyboard(host);
            } else if pressed & cancel != 0 {
                self.keyboard = None;
            }
        } else if self.rescan.is_some() {
            self.rescan_input(host, pressed & confirm != 0, pressed & cancel != 0);
        } else if self.diagnostics.is_some() {
            self.diagnostics_input(step, pressed & cancel != 0);
        } else if self.picker.is_some() {
            self.picker_input(host, step, pressed & confirm != 0, pressed & cancel != 0, pressed & TRIANGLE != 0);
        } else if self.editor.is_some() {
            self.editor_input(host, step, pressed & confirm != 0, pressed & cancel != 0);
        } else if self.categories.is_some() {
            let shoulder = if pressed & L != 0 { -1 } else if pressed & R != 0 { 1 } else { 0 };
            self.categories_input(
                host,
                step,
                pressed & confirm != 0,
                pressed & cancel != 0,
                pressed & TRIANGLE != 0,
                pressed & SQUARE != 0,
                pressed & START != 0,
                shoulder,
            );
        } else if self.adrenaline.is_some() {
            // The Adrenaline drawer takes the buttons while it is open.
            if pressed & confirm != 0 {
                request = self.adrenaline_confirm(host);
            } else if pressed & cancel != 0 {
                self.adrenaline = None;
            }
        } else if pressed & SELECT != 0 {
            // SELECT opens the menu, and closes it from any page.
            if self.menu.open {
                self.menu.open = false;
            } else {
                self.menu.open();
                self.drawer = panel::slide();
                self.drawer.set(1.0, now);
            }
        } else if self.menu.open {
            match step {
                Some(Step::Press(Direction::Up)) | Some(Step::Repeat(Direction::Up, _)) => self.menu.step_row(-1),
                Some(Step::Press(Direction::Down)) | Some(Step::Repeat(Direction::Down, _)) => self.menu.step_row(1),
                // A held direction scrolls rows; it does not step a value.
                Some(Step::Press(Direction::Left)) => self.menu_change(host, -1),
                Some(Step::Press(Direction::Right)) => self.menu_change(host, 1),
                _ => {}
            }
            if pressed & confirm != 0 {
                self.menu_change(host, 1);
            } else if pressed & cancel != 0 {
                self.menu.back();
            }
        } else {
            let count = self.list.len();
            let target = match step {
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
            if pressed & confirm != 0 {
                request = self.launch_selected(host);
            }
            if pressed & cancel != 0 {
                self.close_search();
            }
            if pressed & SQUARE != 0 {
                self.open_search();
            }
            if pressed & TRIANGLE != 0 {
                self.open_editor(host);
            }
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
        if std::mem::take(&mut self.pending_picker_confirm) {
            self.picker_confirm(host);
        }
        self.picker_frame(host);
        self.request_icons(host);
        self.blend_dynamic();
        request
    }

    /// Save the status items' layout (`status-bar.json`).
    fn save_layout(&mut self, host: &mut dyn Host) {
        let text = self.settings.layout.render();
        host.write(crate::library::settings::STATUS_BAR_FILE, text.as_bytes());
    }

    /// Open the keyboard for `target`, starting from `text`.
    fn open_keyboard(&mut self, target: keyboard::Target, text: &str) {
        self.keyboard = Some(keyboard::Keyboard::new(target, text, self.settings.language));
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    /// The keyboard's Done: the text goes to what it was typed for.
    fn commit_keyboard(&mut self, host: &mut dyn Host) {
        let Some(keyboard) = self.keyboard.take() else { return };
        let text = keyboard.text.trim().to_string();
        match &keyboard.target {
            keyboard::Target::Library => self.apply_search(&text),
            keyboard::Target::Title => self.commit_title(host, &text),
            keyboard::Target::NewCategory | keyboard::Target::Category(_) => self.commit_category(host, &keyboard.target, &text),
            // SteamGridDB's search and key arrive in phase 5.
            keyboard::Target::Search | keyboard::Target::ApiKey => {}
        }
    }

    /// Square: type a search term; on the search tab, start from it.
    fn open_search(&mut self) {
        let term = if self.tab == SEARCH_ID { self.library.search_term.clone() } else { String::new() };
        self.open_keyboard(keyboard::Target::Library, &term);
    }

    /// Show the titles `term` finds in the search tab, in front of the
    /// others; an empty term closes the search.
    fn apply_search(&mut self, term: &str) {
        if term.is_empty() {
            self.clear_search();
            return;
        }
        if self.tab != SEARCH_ID {
            self.before_search = Some((self.tab.clone(), self.selected_game().map(|g| g.id.to_string())));
        }
        self.library.search_term = term.to_string();
        self.tab = SEARCH_ID.into();
        self.selected = 0;
        self.refresh();
        self.launching = None;
    }

    /// Close the search and go back to the title selected before it.
    fn clear_search(&mut self) {
        if self.library.search_term.is_empty() {
            return;
        }
        let back = self.before_search.take();
        let searching = self.tab == SEARCH_ID;
        self.library.search_term.clear();
        let mut title = None;
        if searching {
            // The tab it came from (the first tab when that one is gone).
            let (tab, id) = back.unwrap_or_default();
            self.tab = tab;
            self.selected = 0;
            title = id;
        }
        self.refresh();
        if let Some(id) = title {
            if let Some(index) = self.list.iter().position(|&i| *self.library.games()[i].id == id) {
                self.selected = index;
            }
        }
        self.launching = None;
    }

    /// The other face button on the shelf: leave the search tab.
    fn close_search(&mut self) {
        if self.tab == SEARCH_ID {
            self.clear_search();
        }
    }

    /// How many titles the keyboard's text finds, worked out once per text.
    fn found_count(&mut self, text: &str) -> usize {
        if let Some((cached, count)) = &self.found {
            if cached == text {
                return *count;
            }
        }
        let count = crate::library::search::search(self.library.games(), text, |g| (&g.title, &g.id)).len();
        self.found = Some((text.to_string(), count));
        count
    }

    /// Step the highlighted menu row's value, or open the group or panel it
    /// names (`menuChange` in `state.ts`). A changed setting is saved.
    fn menu_change(&mut self, host: &mut dyn Host, delta: i32) {
        use menu::Item;
        let step = |all: &[_], current| cycle_by(all, current, delta);
        let s = &mut self.settings;
        match self.menu.item() {
            Item::Open(page) => {
                if delta > 0 {
                    self.menu.show(page, None);
                }
                return;
            }
            Item::Theme => {
                s.theme = step(&ThemeId::ALL, s.theme);
                self.dynamic = None;
            }
            Item::View => {
                s.view = cycle_by(ViewMode::ALL, s.view, delta);
                self.placed = None;
            }
            Item::Details => {
                s.detail = cycle_by(DetailLevel::ALL, s.detail, delta);
                self.placed = None;
            }
            Item::Backdrop => {
                s.backdrop = !s.backdrop;
                // On: the selected title's backdrop is asked for; off: gone.
                self.backdrop_wait = if s.backdrop { BACKDROP_DELAY } else { 0 };
                if !s.backdrop {
                    self.wanted = None;
                    self.clear_pending = true;
                }
            }
            Item::IconBox => s.icon_box = !s.icon_box,
            Item::StatusBar => s.status_bar = !s.status_bar,
            // The layout is Pocket Shelf Next's own file.
            Item::StatusPosition | Item::ShowWifi | Item::ShowBluetooth | Item::ShowBattery => {
                use crate::library::settings::StatusPosition;
                let l = &mut s.layout;
                match self.menu.item() {
                    Item::StatusPosition => {
                        l.position = if l.position == StatusPosition::Right { StatusPosition::Left } else { StatusPosition::Right }
                    }
                    Item::ShowWifi => l.wifi = !l.wifi,
                    Item::ShowBluetooth => l.bluetooth = !l.bluetooth,
                    _ => l.battery = !l.battery,
                }
                self.save_layout(host);
                return;
            }
            Item::Clock => {
                // System, 24-hour, 12-hour, then Off (the layout's clock).
                use crate::library::settings::ClockFormat;
                let steps = [Some(ClockFormat::System), Some(ClockFormat::H24), Some(ClockFormat::H12), None];
                let now = if s.layout.clock { Some(s.clock) } else { None };
                match cycle_by(&steps, now, delta) {
                    Some(format) => {
                        s.clock = format;
                        s.layout.clock = true;
                    }
                    None => s.layout.clock = false,
                }
                self.save_layout(host);
            }
            Item::BatteryPercent => s.battery_percent = !s.battery_percent,
            Item::UpdateChannel => s.updates = cycle_by(crate::library::settings::UpdateChannel::ALL, s.updates, delta),
            Item::Confirm => {
                s.confirm = if s.confirm == ConfirmMode::Circle { ConfirmMode::Cross } else { ConfirmMode::Circle };
            }
            Item::Language => {
                s.language = cycle_by(crate::library::settings::Language::ALL, s.language, delta);
                // Built-in tab names come in the new language.
                let tab = self.tab.clone();
                self.refresh();
                self.tab = tab;
            }
            Item::Categories => {
                self.open_categories();
                return;
            }
            Item::Rescan => {
                self.open_rescan();
                return;
            }
            Item::Diagnostics => {
                self.open_diagnostics();
                return;
            }
            // Fetch artwork, Clean up artwork and updates arrive in phase 5.
            Item::FetchArt | Item::CleanArt | Item::CheckUpdates => return,
        }
        if self.settings.status_bar {
            self.status = host.status();
        }
        self.persist(host);
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
            // A direction held across the change starts slow again.
            self.held.restart();
        }
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
        let Some(game) = self.selected_game() else {
            // No title selected (a search that found nothing): no backdrop.
            self.wanted = None;
            self.clear_pending = true;
            return;
        };
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
            t: self.t(),
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
        // Panels in the current app's order, the later on top.
        if self.menu.open {
            self.draw_menu(gfx);
        }
        if self.categories.is_some() {
            self.draw_categories(gfx);
        }
        if self.editor.is_some() {
            self.draw_editor(gfx);
        }
        if self.rescan.is_some() {
            self.draw_rescan(gfx);
        }
        if self.diagnostics.is_some() {
            self.draw_diagnostics(gfx);
        }
        if self.picker.is_some() {
            self.draw_picker(gfx);
        }
        if self.keyboard.is_some() {
            self.draw_keyboard(gfx);
        }
    }

    /// The on-screen keyboard (`components/keyboard.tsx`): a card rising
    /// from the bottom edge with what is typed, the keys and the hints.
    fn draw_keyboard(&mut self, gfx: &mut Gfx) {
        use crate::gfx::Rect;
        use crate::text::Role;
        use keyboard::{Action, Page, Target};
        let theme = self.theme;
        let t = self.t();
        let Some(kb) = &self.keyboard else { return };
        let (target, text, page, shift, row, col) = (kb.target.clone(), kb.text.clone(), kb.page, kb.shift, kb.row, kb.col);
        let rows = kb.rows();
        // The page key and the R hint name the page the next press leads to.
        let next_page = match page {
            Page::Letters => t.symbols,
            Page::Symbols => "Áçñ",
            Page::Accents => t.key_letters,
        };
        let k = &t.keyboard_for;
        let heading = match target {
            Target::Title => k.title,
            Target::NewCategory => k.new_category,
            Target::Category(_) => k.category,
            Target::Library => k.library,
            Target::Search => k.search,
            Target::ApiKey => k.api_key,
        };
        let note = if target == Target::Library && !text.trim().is_empty() {
            let count = self.found_count(&text).to_string();
            crate::i18n::fill(t.found, &[("count", &count)])
        } else {
            String::new()
        };
        let confirm = chrome::Button::confirm(self.settings.confirm);
        let cancel = chrome::Button::cancel(self.settings.confirm);
        let hints = [
            (confirm, t.key),
            (chrome::Button::Face(crate::shapes::Symbol::Square), t.key_delete),
            (chrome::Button::ShoulderL, t.shift),
            (chrome::Button::ShoulderR, next_page),
            (chrome::Button::Start, t.done),
            (cancel, t.cancel),
        ];

        // The card's height from its content: the heading, the field, the
        // key rows and the hints, 4 apart, 10 above and 8 below.
        let label_h = gfx.line_height(Role::Label);
        let hints_h = chrome::hints_height(gfx, &hints, 456.0);
        let height = 10.0 + label_h + 4.0 + 26.0 + 4.0 + 2.0 + 4.0 + 5.0 * 24.0 + 4.0 * 4.0 + 4.0 + 2.0 + 4.0 + hints_h + 8.0;
        let slide = self.drawer.at(self.now);
        gfx.fill(Rect::new(0.0, 0.0, crate::W, crate::H), theme.scrim.alpha(slide.min(1.0)));
        let card = Rect::new(8.0, crate::H - 8.0 - height + (1.0 - slide) * 80.0, crate::W - 16.0, height);
        gfx.shadow(card, 12.0, 8.0, crate::gfx::BLACK.with_a(80));
        gfx.round_rect(card, 12.0, 0.0, theme.panel);
        let left = (crate::W - 416.0) / 2.0;
        let mut y = card.y + 10.0;
        gfx.text_top(Role::Label, left + 2.0, y, theme.dim, &crate::i18n::upper(heading, self.settings.language));
        y += label_h + 4.0;
        // The field: text longer than it shows its end, where the caret is;
        // a note at its right end takes its width from the text.
        let field = Rect::new(left, y, 416.0, 26.0);
        gfx.round_rect(field, 6.0, 0.0, theme.tile);
        let note_w = if note.is_empty() { 0.0 } else { gfx.measure(Role::Small, &note) + 12.0 };
        let shown = gfx.fit_tail(Role::BodyBold, &format!("{text}_"), 396.0 - note_w);
        let mid = |gfx: &mut Gfx, role: Role| (field.y + 13.0 - gfx.line_height(role) / 2.0 + gfx.ascent(role)).round();
        let base = mid(gfx, Role::BodyBold);
        gfx.text(Role::BodyBold, field.x + 10.0, base, theme.text, &shown);
        if !note.is_empty() {
            let base = mid(gfx, Role::Small);
            gfx.text_right(Role::Small, field.right() - 10.0, base, theme.dim, &note);
        }
        y += 26.0 + 4.0 + 2.0 + 4.0;
        for (r, keys) in rows.iter().enumerate() {
            let width: f32 = keys.iter().map(|k| if k.wide() { 80.0 } else { 38.0 }).sum::<f32>() + 4.0 * (keys.len() - 1) as f32;
            let mut x = ((crate::W - width) / 2.0).round();
            for (c, key) in keys.iter().enumerate() {
                let w = if key.wide() { 80.0 } else { 38.0 };
                let active = r == row && c == col;
                gfx.round_rect(Rect::new(x, y, w, 24.0), 6.0, 0.0, if active { theme.accent } else { theme.tile });
                let label: String = match key.action {
                    Action::Char => (if shift { key.upper } else { key.lower }).to_string(),
                    Action::Symbols => next_page.to_string(),
                    Action::Shift => t.shift.to_string(),
                    Action::Space => t.key_space.to_string(),
                    Action::Delete => t.key_delete.to_string(),
                    Action::Done => t.done.to_string(),
                };
                let role = if active { Role::SmallBold } else { Role::Small };
                let base = (y + 12.0 - gfx.line_height(role) / 2.0 + gfx.ascent(role)).round();
                gfx.text_centered(role, x + w / 2.0, base, if active { theme.panel } else { theme.text }, &label);
                x += w + 4.0;
            }
            y += 24.0 + 4.0;
        }
        y += 2.0 + 4.0;
        chrome::hints_centered(gfx, &theme, &hints, crate::W / 2.0, y, 456.0);
    }

    /// A menu row's label, value or symbol, and arrows
    /// (`components/menu.tsx`).
    fn menu_row(&self, item: menu::Item) -> (&'static str, Option<String>, Option<chrome::Button>, panel::Step) {
        use crate::library::settings::{ClockFormat, ScrapeSource, UpdateChannel as Channel};
        use menu::{Item, Page};
        use panel::Step::{Both, Right};
        let (t, s) = (self.t(), &self.settings);
        let on_off = |on: bool| Some(if on { t.on } else { t.off }.to_string());
        let page = |p: Page| match p {
            Page::Main => t.menu,
            Page::Appearance => t.appearance,
            Page::Status => t.status_bar,
            Page::Library => t.library,
            Page::Updates => t.updates,
        };
        match item {
            Item::Open(p) => (page(p), None, None, Right),
            Item::Theme => {
                let th = &t.themes;
                let name = match s.theme {
                    ThemeId::Midnight => th.midnight,
                    ThemeId::Aurora => th.aurora,
                    ThemeId::Sakura => th.sakura,
                    ThemeId::Ember => th.ember,
                    ThemeId::Daylight => th.daylight,
                    ThemeId::Dynamic => th.dynamic,
                };
                (t.theme, Some(name.into()), None, Both)
            }
            Item::View => {
                let v = &t.views;
                let name = match s.view {
                    ViewMode::Home => v.home,
                    ViewMode::Carousel => v.carousel,
                    ViewMode::Grid => v.grid,
                    ViewMode::List => v.list,
                    ViewMode::Cascade => v.cascade,
                };
                (t.view, Some(name.into()), None, Both)
            }
            Item::Details => {
                let d = &t.detail_levels;
                let name = match s.detail {
                    DetailLevel::Basic => d.basic,
                    DetailLevel::Normal => d.normal,
                    DetailLevel::Detailed => d.detailed,
                };
                (t.details, Some(name.into()), None, Both)
            }
            Item::Backdrop => (t.backdrop, on_off(s.backdrop), None, Both),
            Item::IconBox => (t.icon_box, on_off(s.icon_box), None, Both),
            Item::StatusBar => (t.show_status_bar, on_off(s.status_bar), None, Both),
            Item::StatusPosition => {
                let left = s.layout.position == crate::library::settings::StatusPosition::Left;
                (t.status_position, Some(if left { t.left } else { t.right }.into()), None, Both)
            }
            Item::ShowWifi => (t.wifi, on_off(s.layout.wifi), None, Both),
            Item::ShowBluetooth => (t.bluetooth, on_off(s.layout.bluetooth), None, Both),
            Item::ShowBattery => (t.battery, on_off(s.layout.battery), None, Both),
            Item::Clock => {
                let c = &t.clock_formats;
                let name = match s.clock {
                    _ if !s.layout.clock => t.off,
                    ClockFormat::System => c.system,
                    ClockFormat::H24 => c.h24,
                    ClockFormat::H12 => c.h12,
                };
                (t.clock, Some(name.into()), None, Both)
            }
            Item::BatteryPercent => (t.battery_percent, on_off(s.battery_percent), None, Both),
            Item::Categories => (t.categories_row, Some(t.manage.into()), None, Right),
            Item::FetchArt => {
                let source = match s.scraper {
                    ScrapeSource::Auto => t.source_auto,
                    ScrapeSource::SteamGridDb => "SteamGridDB",
                    ScrapeSource::Libretro => "libretro",
                };
                (t.fetch_art, Some(source.into()), None, Right)
            }
            Item::CleanArt => (t.clean_art, None, None, Right),
            Item::Rescan => (t.rescan, None, None, Right),
            Item::UpdateChannel => {
                let u = &t.update_channels;
                let name = match s.updates {
                    Channel::Stable => u.stable,
                    Channel::Beta => u.beta,
                    Channel::Nightly => u.nightly,
                    Channel::Off => u.off,
                };
                (t.update_channel, Some(name.into()), None, Both)
            }
            // This build's version, as `buildLabel` shows a local build's.
            Item::CheckUpdates => (t.check_updates, Some(format!("{} (next)", env!("CARGO_PKG_VERSION"))), None, Right),
            Item::Confirm => (t.confirm, None, Some(chrome::Button::confirm(s.confirm)), Both),
            Item::Language => {
                // Each language under its own name, whatever the current one.
                let name = crate::i18n::LANGUAGES.iter().find(|(l, _)| *l == s.language).map_or("", |(_, n)| n);
                (t.language, Some(name.into()), None, Both)
            }
            Item::Diagnostics => (t.diagnostics, None, None, Right),
        }
    }

    /// The SELECT menu: the main page, or the group opened from it.
    fn draw_menu(&mut self, gfx: &mut Gfx) {
        let theme = self.theme;
        let t = self.t();
        let title = self.menu_row(menu::Item::Open(self.menu.page)).0;
        let heading = crate::i18n::upper(title, self.settings.language);
        let slide = self.drawer.at(self.now);
        let mut drawer = panel::Drawer::open(gfx, &theme, &heading, panel::Size::Wide, slide);
        let items = menu::items(self.menu.page);
        let first = self.menu.first_visible();
        for (index, item) in items.iter().enumerate().skip(first).take(menu::VISIBLE) {
            let (label, value, icon, step) = self.menu_row(*item);
            drawer.row(gfx, label, value.as_deref(), icon, index == self.menu.row, step);
        }
        let cancel = chrome::Button::cancel(self.settings.confirm);
        let back = if self.menu.page == menu::Page::Main { t.close } else { t.back };
        drawer.hints(gfx, &[(chrome::Button::DpadVertical, t.navigate), (chrome::Button::DpadHorizontal, t.change), (cancel, back)]);
    }

    /// Shown when a title with no bubble is started and Adrenaline would not
    /// load the boot plugin (`components/adrenaline.tsx`): add it, turn it
    /// on, or (no Adrenaline) only say so.
    fn draw_adrenaline(&mut self, gfx: &mut Gfx, state: PluginState, failed: bool) {
        let theme = self.theme;
        let title = self.selected_game().map(|g| g.title.to_string()).unwrap_or_default();
        let slide = self.drawer.at(self.now);
        let t = self.t();
        let heading = crate::i18n::upper(t.adr_title, self.settings.language);
        let mut drawer = panel::Drawer::open(gfx, &theme, &heading, panel::Size::Wide, slide);
        let can_fix = matches!(state, PluginState::Missing | PluginState::Off);
        let action = if state == PluginState::Off { t.adr_turn_on } else { t.adr_add };
        if can_fix {
            drawer.row(gfx, action, None, None, true, panel::Step::Right);
        }
        let note = if failed {
            t.adr_failed.to_string()
        } else {
            let text = match state {
                PluginState::Off => t.adr_off,
                PluginState::Missing => t.adr_missing,
                PluginState::NoAdrenaline | PluginState::On => t.adr_none,
            };
            crate::i18n::fill(text, &[("title", &title)])
        };
        drawer.note(gfx, &note, 1);
        let confirm = chrome::Button::confirm(self.settings.confirm);
        let cancel = chrome::Button::cancel(self.settings.confirm);
        if can_fix {
            drawer.hints(gfx, &[(confirm, action), (cancel, t.back)]);
        } else {
            drawer.hints(gfx, &[(cancel, t.back)]);
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
        fn scan(&mut self, _: bool) {}
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
    fn the_menu_steps_a_setting_and_saves_it() {
        let (mut shelf, mut host) = setup(true, PluginState::On);
        // SELECT, Appearance, down to View, right: Carousel.
        for button in [SELECT, RIGHT, DOWN, RIGHT] {
            press(&mut shelf, &mut host, button);
        }
        assert!(shelf.menu.open);
        assert_eq!(shelf.settings.view, ViewMode::Carousel);
        let saved = String::from_utf8(host.files.0[SETTINGS_FILE].clone()).unwrap();
        assert!(saved.contains(r#""view":"carousel""#), "{saved}");
        // The shelf takes no buttons while the menu is open: confirm steps
        // the row, it does not launch.
        press(&mut shelf, &mut host, CIRCLE);
        assert_eq!(shelf.settings.view, ViewMode::Grid);
        assert!(host.launched.is_empty());
        // Back returns to the main page; SELECT closes from any page.
        press(&mut shelf, &mut host, CROSS);
        assert_eq!(shelf.menu.page, menu::Page::Main);
        press(&mut shelf, &mut host, RIGHT);
        press(&mut shelf, &mut host, SELECT);
        assert!(!shelf.menu.open);
    }

    #[test]
    fn a_search_shows_its_tab_and_closing_returns_to_the_title() {
        let (mut shelf, mut host) = setup(true, PluginState::On);
        // On Games, the second title is not there; Games holds PCSA00069.
        press(&mut shelf, &mut host, R);
        let before = shelf.selected_game().map(|g| g.id.to_string());
        press(&mut shelf, &mut host, SQUARE);
        assert!(shelf.keyboard.is_some());
        shelf.keyboard.as_mut().unwrap().text = "vita".into();
        press(&mut shelf, &mut host, START);
        assert_eq!(shelf.tab, SEARCH_ID);
        assert_eq!(shelf.tabs[0].id, SEARCH_ID);
        assert_eq!(shelf.selected_game().map(|g| g.id.to_string()).as_deref(), Some("VITASHELL"));
        // Back on the search tab closes it and returns.
        press(&mut shelf, &mut host, CROSS);
        assert_ne!(shelf.tab, SEARCH_ID);
        assert!(shelf.tabs.iter().all(|c| c.id != SEARCH_ID));
        assert_eq!(shelf.selected_game().map(|g| g.id.to_string()), before);
    }

    fn type_text(shelf: &mut Shelf, host: &mut TestHost, text: &str) {
        shelf.keyboard.as_mut().expect("keyboard open").text = text.into();
        press(shelf, host, START);
    }

    #[test]
    fn the_editor_saves_a_favorite_a_name_and_resets_to_the_defaults() {
        let (mut shelf, mut host) = setup(true, PluginState::On);
        press(&mut shelf, &mut host, R); // Games: PCSA00069
        press(&mut shelf, &mut host, TRIANGLE);
        press(&mut shelf, &mut host, CIRCLE); // Favorite on
        assert!(host.files.0.contains_key("titles/PCSA00069.json"));
        // Title row: type a name.
        press(&mut shelf, &mut host, DOWN);
        press(&mut shelf, &mut host, DOWN);
        press(&mut shelf, &mut host, CIRCLE);
        type_text(&mut shelf, &mut host, "Daze");
        assert_eq!(&*shelf.library.game("PCSA00069").unwrap().title, "Daze");
        let saved = String::from_utf8(host.files.0["titles/PCSA00069.json"].clone()).unwrap();
        assert_eq!(saved, "{\n  \"title\": \"Daze\",\n  \"favorite\": true\n}\n");
        // Reset keeps the favorite mark only.
        for _ in 0..4 {
            press(&mut shelf, &mut host, DOWN);
        }
        press(&mut shelf, &mut host, CIRCLE);
        let game = shelf.library.game("PCSA00069").unwrap();
        assert!(game.favorite && &*game.title == "Title PCSA00069");
        // Favorite off as well: no changes left, the file is deleted.
        press(&mut shelf, &mut host, DOWN);
        press(&mut shelf, &mut host, CIRCLE);
        assert!(!host.files.0.contains_key("titles/PCSA00069.json"));
    }

    #[test]
    fn the_category_manager_creates_renames_hides_and_deletes() {
        let (mut shelf, mut host) = setup(true, PluginState::On);
        shelf.open_categories();
        // New category through the keyboard.
        press(&mut shelf, &mut host, TRIANGLE);
        type_text(&mut shelf, &mut host, "Retro Picks");
        assert!(shelf.library.config.custom.iter().any(|c| c.id == "retro-picks" && c.label == "Retro Picks"));
        let saved = String::from_utf8(host.files.0[CATEGORIES_FILE].clone()).unwrap();
        assert!(saved.contains("\"retro-picks\""), "{saved}");
        // A name another category has is refused.
        press(&mut shelf, &mut host, CIRCLE);
        type_text(&mut shelf, &mut host, "games");
        assert_eq!(shelf.categories.as_ref().unwrap().note, shelf.t().name_taken);
        // Move a title into it, then delete it: the title goes back.
        shelf.library.patch("PCSA00069", |c| c.category = Some("retro-picks".into()));
        press(&mut shelf, &mut host, SQUARE);
        assert!(shelf.categories.as_ref().unwrap().armed.is_some());
        press(&mut shelf, &mut host, SQUARE);
        assert!(shelf.library.config.custom.is_empty());
        assert_eq!(&*shelf.library.game("PCSA00069").unwrap().category, "games");
        // Hide a built-in one, and show it again.
        let row = 1 + shelf.library.all_categories().iter().position(|c| c.id == "homebrew").unwrap();
        shelf.categories.as_mut().unwrap().row = row;
        press(&mut shelf, &mut host, START);
        assert!(shelf.library.config.hidden.contains(&"homebrew".to_string()));
        assert!(shelf.tabs.iter().all(|c| c.id != "homebrew"));
        press(&mut shelf, &mut host, START);
        assert!(shelf.library.config.hidden.is_empty());
    }

    #[test]
    fn the_status_layout_is_saved_in_its_own_file() {
        use crate::library::settings::{StatusLayout, StatusPosition, STATUS_BAR_FILE};
        let (mut shelf, mut host) = setup(true, PluginState::On);
        // Appearance > Status bar > Position: left; then the clock off (left
        // from System).
        for button in [SELECT, RIGHT, UP, RIGHT, DOWN, RIGHT] {
            press(&mut shelf, &mut host, button);
        }
        for _ in 0..5 {
            press(&mut shelf, &mut host, DOWN);
        }
        press(&mut shelf, &mut host, LEFT);
        let saved = StatusLayout::parse(&host.files.0[STATUS_BAR_FILE]);
        assert_eq!(saved, StatusLayout { position: StatusPosition::Left, clock: false, ..Default::default() });
        // settings.json does not carry the layout.
        let settings = host.files.0.get(SETTINGS_FILE).map(|b| String::from_utf8(b.clone()).unwrap()).unwrap_or_default();
        assert!(!settings.contains("position"));
        // Right again from Off: the clock is back with its format.
        press(&mut shelf, &mut host, RIGHT);
        assert!(StatusLayout::parse(&host.files.0[STATUS_BAR_FILE]).clock);
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
