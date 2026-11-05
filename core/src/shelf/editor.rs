//! The title editor, the art and backdrop picker and the
//! category manager.

use std::collections::HashMap;

use super::chrome::{self, Button};
use super::panel::{self, Drawer, Step};
use super::{keyboard, Shelf};
use crate::gfx::{Format, Gfx, Rect, TexId};
use crate::host::{Host, Picture, PictureKey};
use crate::i18n::{fill, upper};
use crate::library::categories::{make_category_id, valid_category_id, CATEGORIES_FILE, SEARCH_ID};
use crate::library::list_png;
use crate::library::overrides::{NO_BACKDROP, USE_ICON};
use crate::library::search::search_key;
use crate::library::settings::ScrapeSource;
use crate::shapes::Symbol;
use crate::text::Role;

/// The editor's rows, in order (`editor_rows`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Row {
    Favorite,
    Category,
    /// The app that starts the game; only for a game both RetroArch and
    /// Emu4Vita++ open.
    OpenWith,
    Title,
    BoxArt,
    Backdrop,
    FindArt,
    Reset,
}

/// The rows for a title: Open with only when both apps can start it.
pub(super) fn editor_rows(game: &crate::library::catalog::Game) -> Vec<Row> {
    use Row::*;
    let mut rows = vec![Favorite, Category, OpenWith, Title, BoxArt, Backdrop, FindArt, Reset];
    if !game.opens.both() {
        rows.retain(|row| *row != OpenWith);
    }
    rows
}

/// Editor rows shown at once; with Open with the list scrolls.
const EDITOR_VISIBLE: usize = 7;
/// Room for a value beside its label.
const VALUE_W: f32 = 320.0;
/// Picker rows shown at once.
const PICKER_ROWS: usize = 5;
/// The picker's fixed rows: Default, Game icon (or None), Browse; the
/// folder's files follow.
pub(super) const PICKER_FIXED: usize = 3;
const BROWSE_ROW: usize = 2;
/// Frames the picker's highlight rests before its picture is loaded.
const PREVIEW_DELAY: u32 = 8;
/// Category manager rows shown at once.
const CATEGORY_ROWS: usize = 4;

pub struct Editor {
    pub id: String,
    pub row: usize,
    pub note: String,
}

/// The art or backdrop picker: the fixed rows ("Default", then "Game icon"
/// or "None", then "Browse...", `browse.rs`), then the PNG files of the
/// folder.
pub struct Picker {
    pub backdrop: bool,
    pub folder: Vec<String>,
    /// All the folder's files, or only those named after the title.
    pub every: bool,
    /// The files listed now: worked out when the picker opens and when
    /// triangle switches, not every frame (thousands of files in `art/`
    /// took 20 ms a frame on hardware).
    pub listed: Vec<String>,
    pub row: usize,
    pub note: String,
    /// Whether the host decoded a file, learned from its preview.
    pub decodes: HashMap<String, bool>,
    /// A confirm on a file not checked yet: used once it decodes.
    pub waiting: bool,
    pub preview_wait: u32,
    /// The picture asked for the highlighted row, and its texture once
    /// decoded (a backdrop's own; an icon's is the art cache's).
    pub preview: Option<(PictureKey, Option<TexId>)>,
}

pub struct Categories {
    /// Row 0 is "New category"; then every category in tab order.
    pub row: usize,
    /// The category a first delete press armed.
    pub armed: Option<String>,
    pub note: String,
}

/// A name as part of a file name: lowercase letters without marks, digits,
/// `-` between words.
fn file_slug(text: &str) -> String {
    let key = search_key(text);
    let mut out = String::new();
    let mut dash = false;
    for c in key.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c);
        } else {
            dash = true;
        }
    }
    out
}

/// Whether `file` is named after one of `names`: the name, or the name and
/// `-` and more (`fileBelongsTo`).
#[cfg(test)]
pub fn file_belongs_to(file: &str, names: &[&str]) -> bool {
    let slugs: Vec<String> = names.iter().map(|n| file_slug(n)).collect();
    belongs(file, &slugs)
}

/// `file_belongs_to` with the names' slugs worked out once.
fn belongs(file: &str, slugs: &[String]) -> bool {
    let stem = if file.to_lowercase().ends_with(".png") { &file[..file.len() - 4] } else { file };
    let base = file_slug(stem);
    slugs.iter().any(|slug| !slug.is_empty() && (base == *slug || (base.len() > slug.len() && base.starts_with(slug.as_str()) && base.as_bytes()[slug.len()] == b'-')))
}

impl Shelf {
    /// Save the per-title changes that differ from disk, and the cache.
    fn save_overrides(&mut self, host: &mut dyn Host) {
        if !self.library.overrides.save(host) {
            host.log("Title changes not saved");
        }
    }

    /// Save `categories.json`: the user's categories, every id in tab
    /// order, the hidden ones.
    fn save_categories(&mut self, host: &mut dyn Host) {
        let text = self.library.category_config().render();
        host.write(CATEGORIES_FILE, text.as_bytes());
    }

    pub(super) fn after_change_pub(&mut self) {
        self.after_change();
    }

    /// After a title or a category changed: the tabs and the current tab's
    /// titles again, the selection kept in range.
    fn after_change(&mut self) {
        let tab = self.tab.clone();
        self.refresh();
        if self.tab == tab {
            self.placed = Some((self.settings.view, self.settings.detail, tab));
        }
    }

    // --- Editor ------------------------------------------------------------

    /// TRIANGLE: edit the selected title.
    pub(super) fn open_editor(&mut self) {
        let Some(game) = self.selected_game() else { return };
        // `art/` is listed when a picker opens, not here: listing thousands
        // of files took 65 ms on hardware.
        let id = game.id.to_string();
        self.editor = Some(Editor { id, row: 0, note: String::new() });
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    fn editor_game(&self) -> Option<crate::library::catalog::Game> {
        self.editor.as_ref().and_then(|e| self.library.game(&e.id)).cloned()
    }

    /// The edited title's rows; none when no title is edited.
    pub(super) fn editor_row_list(&self) -> Vec<Row> {
        self.editor_game().map(|game| editor_rows(&game)).unwrap_or_default()
    }

    pub(super) fn editor_input(&mut self, host: &mut dyn Host, step: Option<super::navigation::Step>, confirm: bool, cancel: bool) {
        use super::navigation::{Direction, Step as Nav};
        let rows = self.editor_row_list();
        let Some(editor) = &mut self.editor else { return };
        let count = rows.len().max(1);
        editor.row = editor.row.min(count - 1);
        let at = editor.row;
        let row = rows.get(at).copied().unwrap_or(Row::Reset);
        match step {
            Some(Nav::Press(Direction::Up)) | Some(Nav::Repeat(Direction::Up, _)) => editor.row = (at + count - 1) % count,
            Some(Nav::Press(Direction::Down)) | Some(Nav::Repeat(Direction::Down, _)) => editor.row = (at + 1) % count,
            Some(Nav::Press(Direction::Left)) | Some(Nav::Press(Direction::Right)) => {
                let delta = if matches!(step, Some(Nav::Press(Direction::Left))) { -1 } else { 1 };
                match row {
                    Row::Favorite => self.toggle_favorite(host),
                    Row::Category => self.change_game_category(host, delta),
                    Row::OpenWith => self.change_open_with(host, delta),
                    Row::FindArt => self.change_scraper(host, delta),
                    _ => {}
                }
            }
            _ => {}
        }
        if confirm {
            match row {
                Row::Favorite => self.toggle_favorite(host),
                Row::Category => self.change_game_category(host, 1),
                Row::OpenWith => self.change_open_with(host, 1),
                Row::Title => {
                    let title = self.editor_game().map(|g| g.title.to_string()).unwrap_or_default();
                    self.open_keyboard(keyboard::Target::Title, &title);
                }
                Row::BoxArt => self.open_picker(host, false),
                Row::Backdrop => self.open_picker(host, true),
                Row::FindArt => self.start_online(host),
                Row::Reset => self.reset_game(host),
            }
        } else if cancel {
            self.editor = None;
            self.after_change();
        }
    }

    /// The edited title's app: the menu's choice, RetroArch, Emu4Vita++.
    /// Saved in `emulators.json`.
    fn change_open_with(&mut self, host: &mut dyn Host, delta: i32) {
        use crate::library::settings::{Emulator, EMULATORS_FILE};
        let Some(game) = self.editor_game() else { return };
        let choices = [None, Some(Emulator::RetroArch), Some(Emulator::Emu4Vita)];
        let titles = &mut self.settings.emulators.titles;
        let now = titles.get(&*game.id).copied();
        match super::cycle_by(&choices, now, delta) {
            Some(app) => titles.insert(game.id.to_string(), app),
            None => titles.remove(&*game.id),
        };
        host.write(EMULATORS_FILE, self.settings.emulators.render().as_bytes());
    }

    /// Add the edited title to "Favorites" or take it out.
    fn toggle_favorite(&mut self, host: &mut dyn Host) {
        let Some(game) = self.editor_game() else { return };
        self.library.patch(&game.id, |c| c.favorite = !game.favorite);
        self.save_overrides(host);
        self.after_change();
    }

    /// Move the edited title to the next or previous category (not a smart
    /// one, nor a hidden one); the tab stays.
    fn change_game_category(&mut self, host: &mut dyn Host, delta: i32) {
        let Some(game) = self.editor_game() else { return };
        let hidden = &self.library.config.hidden;
        let choices: Vec<String> = self
            .library
            .all_categories()
            .iter()
            .filter(|c| !c.smart && (!hidden.contains(&c.id) || *c.id == *game.category))
            .map(|c| c.id.clone())
            .collect();
        let next = crate::library::categories::cycle_category(&choices, &game.category, delta).to_string();
        let own = self.library.base_game(&game.id).map(|g| g.category.to_string());
        self.library.patch(&game.id, |c| c.category = (Some(&next) != own.as_ref()).then_some(next.clone()));
        self.save_overrides(host);
        self.after_change();
    }

    /// Step the scraper choice, a setting (Fetch artwork's source too).
    fn change_scraper(&mut self, host: &mut dyn Host, delta: i32) {
        self.settings.scraper = super::cycle_by(ScrapeSource::ALL, self.settings.scraper, delta);
        self.persist(host);
    }

    /// Back to the title's defaults, its app the menu's choice again; the
    /// favorite mark stays.
    fn reset_game(&mut self, host: &mut dyn Host) {
        let Some(game) = self.editor_game() else { return };
        let favorite = game.favorite;
        self.library.patch(&game.id, |c| *c = crate::library::overrides::TitleOverride { favorite, ..Default::default() });
        self.save_overrides(host);
        if self.settings.emulators.titles.remove(&*game.id).is_some() {
            host.write(crate::library::settings::EMULATORS_FILE, self.settings.emulators.render().as_bytes());
        }
        self.after_change();
        let done = self.t().reset_done;
        if let Some(editor) = &mut self.editor {
            editor.note = done.to_string();
        }
    }

    /// The keyboard's Done for the title: typing the original name back (or
    /// nothing) removes the change.
    pub(super) fn commit_title(&mut self, host: &mut dyn Host, text: &str) {
        let Some(game) = self.editor_game() else { return };
        let own = self.library.base_game(&game.id).map(|g| g.title.to_string()).unwrap_or_default();
        let title = (!text.is_empty() && text != own).then(|| text.to_string());
        self.library.patch(&game.id, |c| c.title = title);
        self.save_overrides(host);
        self.after_change();
    }

    pub(super) fn draw_editor(&mut self, gfx: &mut Gfx) {
        let Some(editor) = &self.editor else { return };
        let (row, note) = (editor.row, editor.note.clone());
        let Some(game) = self.editor_game() else { return };
        let t = self.t();
        let theme = self.theme;
        // A file Browse set shows by its name, not its whole path.
        let named = |file: &str| file.rsplit(['/', ':']).next().unwrap_or(file).to_string();
        let art = if game.art_icon {
            t.game_icon.to_string()
        } else {
            match &game.art {
                None => t.standard.to_string(),
                Some(file) if game.art_auto => fill(t.auto_art, &[("file", file)]),
                Some(file) => named(file),
            }
        };
        let backdrop = match game.backdrop.as_deref() {
            Some(NO_BACKDROP) => t.none.to_string(),
            Some(file) => named(file),
            None => t.standard.to_string(),
        };
        let category = self
            .library
            .all_categories()
            .iter()
            .find(|c| *c.id == *game.category)
            .map_or(game.category.to_string(), |c| crate::i18n::category_label(t, c));
        let scraper = match self.settings.scraper {
            ScrapeSource::Auto => t.source_auto,
            ScrapeSource::SteamGridDb => "SteamGridDB",
            ScrapeSource::Libretro => "libretro",
        };
        let fit = |gfx: &mut Gfx, text: &str| gfx.fit(Role::BodyBold, text, VALUE_W);
        let favorite = match (game.favorite, game.emu_favorite) {
            (true, _) => t.on.to_string(),
            (false, true) => fill(t.favorite_in, &[("app", crate::library::settings::Emulator::Emu4Vita.name())]),
            (false, false) => t.off.to_string(),
        };
        let open_with = match self.settings.emulators.titles.get(&*game.id) {
            Some(app) => app.name().to_string(),
            None => {
                let app = self.settings.emulators.pick("", game.opens).unwrap_or(self.settings.emulators.default);
                fill(t.default_app, &[("app", app.name())])
            }
        };
        let rows: Vec<(&str, String, Step)> = editor_rows(&game)
            .into_iter()
            .map(|r| match r {
                Row::Favorite => (t.favorite, favorite.clone(), Step::Both),
                Row::Category => (t.category, category.clone(), Step::Both),
                Row::OpenWith => (t.open_with, open_with.clone(), Step::Both),
                Row::Title => (t.title, fit(gfx, &game.title), Step::Right),
                Row::BoxArt => (t.box_art, fit(gfx, &art), Step::Right),
                Row::Backdrop => (t.backdrop, fit(gfx, &backdrop), Step::Right),
                Row::FindArt => (t.find_art, scraper.to_string(), Step::Both),
                Row::Reset => (t.reset, String::new(), Step::None),
            })
            .collect();
        let slide = self.drawer.at(self.now);
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.editor_title, self.settings.language), panel::Size::Wide, slide);
        // Every row does something, Reset included.
        drawer.selectable = true;
        // The highlight in the middle where it can be, as the menu scrolls.
        let first = row.saturating_sub(EDITOR_VISIBLE / 2).min(rows.len().saturating_sub(EDITOR_VISIBLE));
        drawer.index = first;
        for (i, (label, value, step)) in rows.iter().enumerate().skip(first).take(EDITOR_VISIBLE) {
            let value = (!value.is_empty()).then_some(value.as_str());
            drawer.row(gfx, label, value, None, i == row, *step);
        }
        if note.is_empty() {
            let confirm = Button::confirm(self.settings.confirm);
            let cancel = Button::cancel(self.settings.confirm);
            drawer.hints(gfx, &[(confirm, t.change), (cancel, t.back)]);
        } else {
            drawer.note(gfx, &note, 0);
        }
    }

    // --- Art and backdrop picker ------------------------------------------

    /// The file the edited title uses now, when the user chose one.
    pub(super) fn chosen_file(&self, backdrop: bool) -> Option<String> {
        let game = self.editor_game()?;
        if backdrop {
            game.backdrop.filter(|b| b != NO_BACKDROP)
        } else if game.art_auto {
            None
        } else {
            game.art
        }
    }

    /// The picker's listed files: all of the folder, or those named after
    /// the title (its name, the name it came with, its id), the one in use
    /// listed whatever its name.
    fn compute_listed(&self) -> Vec<String> {
        let Some(picker) = &self.picker else { return Vec::new() };
        if picker.every {
            return picker.folder.clone();
        }
        let Some(game) = self.editor_game() else { return Vec::new() };
        let own = self.library.base_game(&game.id).map(|g| g.title.to_string()).unwrap_or_default();
        let slugs: Vec<String> = [&*game.title, own.as_str(), &*game.id].iter().map(|n| file_slug(n)).collect();
        let chosen = self.chosen_file(picker.backdrop);
        picker.folder.iter().filter(|f| Some(*f) == chosen.as_ref() || belongs(f, &slugs)).cloned().collect()
    }

    /// Work the listed files out again (on open and on triangle).
    fn refresh_listed(&mut self) {
        let listed = self.compute_listed();
        if let Some(picker) = &mut self.picker {
            picker.listed = listed;
        }
    }

    /// The file on a picker row (rows 0 and 1 are fixed).
    fn picker_file(&self, row: usize) -> Option<String> {
        self.picker.as_ref().and_then(|p| p.listed.get(row.wrapping_sub(PICKER_FIXED)).cloned())
    }

    pub(super) fn picker_count(&self) -> usize {
        self.picker.as_ref().map_or(0, |p| p.listed.len())
    }

    fn open_picker(&mut self, host: &mut dyn Host, backdrop: bool) {
        let Some(game) = self.editor_game() else { return };
        let folder = if backdrop { list_png(host, "backdrops") } else { self.library.reload_art(host) };
        self.art.forget_missing();
        self.picker = Some(Picker {
            backdrop,
            folder,
            every: false,
            listed: Vec::new(),
            row: 0,
            note: String::new(),
            decodes: HashMap::new(),
            waiting: false,
            preview_wait: PREVIEW_DELAY,
            preview: None,
        });
        // A title with no files of its own starts on the whole folder.
        self.refresh_listed();
        let every = self.picker_count() == 0;
        let chosen = self.chosen_file(backdrop);
        if let Some(picker) = &mut self.picker {
            picker.every = every;
        }
        if every {
            self.refresh_listed();
        }
        let files = &self.picker.as_ref().map(|p| p.listed.clone()).unwrap_or_default();
        let fixed = if backdrop { game.backdrop.as_deref() == Some(NO_BACKDROP) } else { game.art_icon };
        let chosen_path = chosen.as_deref().is_some_and(crate::library::overrides::is_path);
        let current = chosen.and_then(|c| files.iter().position(|f| *f == c));
        if let Some(picker) = &mut self.picker {
            picker.row = if fixed {
                1
            } else if chosen_path {
                // A file Browse set: Browse opens on it.
                BROWSE_ROW
            } else {
                current.map_or(0, |i| i + PICKER_FIXED)
            };
        }
    }

    pub(super) fn picker_input(&mut self, host: &mut dyn Host, step: Option<super::navigation::Step>, confirm: bool, cancel: bool, triangle: bool) {
        use super::navigation::{Direction, Step as Nav};
        let count = self.picker_count() + PICKER_FIXED;
        let Some(picker) = &mut self.picker else { return };
        let before = picker.row;
        match step {
            Some(Nav::Press(Direction::Up)) | Some(Nav::Repeat(Direction::Up, _)) => picker.row = (picker.row + count - 1) % count,
            Some(Nav::Press(Direction::Down)) | Some(Nav::Repeat(Direction::Down, _)) => picker.row = (picker.row + 1) % count,
            _ => {}
        }
        if triangle {
            // Switch between this title's files and the whole folder; the
            // highlight stays on its file.
            let file = self.picker_file(before);
            if let Some(picker) = &mut self.picker {
                picker.every = !picker.every;
            }
            self.refresh_listed();
            if let Some(picker) = &mut self.picker {
                let files = &picker.listed;
                picker.row = match file.and_then(|f| files.iter().position(|x| *x == f)) {
                    Some(i) => i + PICKER_FIXED,
                    None => picker.row.min(PICKER_FIXED - 1),
                };
            }
        }
        if self.picker.as_ref().is_some_and(|p| p.row != before) || triangle {
            self.picker_moved();
        }
        if confirm {
            self.picker_confirm(host);
        } else if cancel {
            self.picker = None;
        }
    }

    /// A new row in the picker: its picture after the highlight rests; a
    /// file known not to decode says so at once.
    fn picker_moved(&mut self) {
        let cant_read = self.t().cant_read;
        let row = self.picker.as_ref().map_or(0, |p| p.row);
        let file = self.picker_file(row);
        let Some(picker) = &mut self.picker else { return };
        picker.preview = None;
        picker.waiting = false;
        picker.preview_wait = PREVIEW_DELAY;
        let file = file.as_ref();
        picker.note = if file.is_some_and(|f| picker.decodes.get(f) == Some(&false)) { cant_read.to_string() } else { String::new() };
    }

    /// Highlight a picker row by touch, as the buttons would.
    pub(super) fn picker_focus(&mut self, row: usize) {
        let count = self.picker_count() + PICKER_FIXED;
        let Some(picker) = &mut self.picker else { return };
        let row = row.min(count - 1);
        if picker.row != row {
            picker.row = row;
            self.picker_moved();
        }
    }

    /// Ask for the highlighted row's picture once the highlight has rested.
    pub(super) fn picker_frame(&mut self, host: &mut dyn Host) {
        let Some(picker) = &mut self.picker else { return };
        if picker.preview_wait == 0 {
            return;
        }
        picker.preview_wait -= 1;
        if picker.preview_wait > 0 {
            return;
        }
        let (backdrop, row) = (picker.backdrop, picker.row);
        // Browse has no picture of its own.
        if row == BROWSE_ROW {
            return;
        }
        let Some(game) = self.editor_game() else { return };
        let file = self.picker_file(row);
        let key = if backdrop {
            // Row 0 is the title's own picture, row 1 none.
            if row == 1 {
                return;
            }
            PictureKey::Backdrop { id: game.id.to_string(), file: file.unwrap_or_default() }
        } else {
            // Row 0 is a file matched by name or the title's own icon, row 1
            // that icon.
            let auto = if row == 0 && game.art_auto { game.art.clone() } else { None };
            match file.or(auto) {
                Some(name) => PictureKey::Art(name),
                None => PictureKey::Icon(game.id.to_string()),
            }
        };
        if let Some(picker) = &mut self.picker {
            picker.preview = Some((key.clone(), None));
        }
        host.request(key);
    }

    /// A picture answered for the picker's preview; true when it was.
    pub(super) fn picker_picture(&mut self, gfx: &mut Gfx, key: &PictureKey, picture: &Option<Picture>) -> bool {
        let cant_read = self.t().cant_read;
        let file = self.picker.as_ref().and_then(|p| self.picker_file(p.row));
        let Some(picker) = &mut self.picker else { return false };
        if picker.preview.as_ref().map(|p| &p.0) != Some(key) {
            return false;
        }
        if let (PictureKey::Backdrop { .. }, Some(picture)) = (key, picture) {
            let tex = gfx.add_texture(picture.w, picture.h, Format::Rgba8, picture.rgba.clone(), true);
            picker.preview = Some((key.clone(), Some(tex)));
        }
        if let Some(file) = file {
            let ok = picture.is_some();
            picker.decodes.insert(file, ok);
            if !ok {
                picker.note = cant_read.to_string();
            }
            let waiting = std::mem::take(&mut picker.waiting);
            if waiting && ok {
                self.pending_picker_confirm = true;
            }
        }
        matches!(key, PictureKey::Backdrop { .. })
    }

    /// Use the highlighted row. A file is used only once the host decoded
    /// it: one it cannot read keeps the drawer open with a note, one not
    /// checked yet is checked first.
    pub(super) fn picker_confirm(&mut self, host: &mut dyn Host) {
        let cant_read = self.t().cant_read;
        let Some(game) = self.editor_game() else { return };
        let file = self.picker.as_ref().and_then(|p| self.picker_file(p.row));
        let Some(picker) = &mut self.picker else { return };
        let row = picker.row;
        if let Some(file) = &file {
            match picker.decodes.get(file) {
                Some(false) => {
                    picker.note = cant_read.to_string();
                    return;
                }
                None => {
                    picker.waiting = true;
                    picker.preview_wait = 1;
                    return;
                }
                Some(true) => {}
            }
        }
        let backdrop = picker.backdrop;
        if row == BROWSE_ROW {
            self.open_browser(host, backdrop);
            return;
        }
        let value = match row {
            0 => None,
            1 => Some(if backdrop { NO_BACKDROP } else { USE_ICON }.to_string()),
            _ => file,
        };
        if backdrop {
            self.library.patch(&game.id, |c| c.backdrop = value);
            // The selected title's backdrop is asked for again.
            self.backdrop_wait = 1;
        } else {
            self.library.patch(&game.id, |c| c.art = value);
        }
        self.save_overrides(host);
        self.after_change();
        self.picker = None;
    }

    pub(super) fn draw_picker(&mut self, gfx: &mut Gfx) {
        let Some(picker) = &self.picker else { return };
        let count = picker.listed.len();
        let (backdrop, row, every, note) = (picker.backdrop, picker.row, picker.every, picker.note.clone());
        let preview = picker.preview.clone();
        let t = self.t();
        let theme = self.theme;
        // Only the rows shown are read from the list.
        let total = count + PICKER_FIXED;
        let entry = |i: usize| -> String {
            match i {
                0 => t.standard.into(),
                1 => (if backdrop { t.none } else { t.game_icon }).into(),
                BROWSE_ROW => t.browse.into(),
                _ => self.picker.as_ref().map(|p| p.listed[i - PICKER_FIXED].clone()).unwrap_or_default(),
            }
        };
        let slide = self.drawer.at(self.now);
        let title = if backdrop { t.backdrop } else { t.box_art };
        let mut drawer = Drawer::open(gfx, &theme, &upper(title, self.settings.language), panel::Size::Wide, slide);
        // The highlighted row's picture, in the room left of the drawer.
        if let Some((key, tex)) = preview {
            let tex = tex.or_else(|| self.art.texture(&key));
            if let Some(tex) = tex.filter(|&t| gfx.textures.ready(t)) {
                let (frame, picture) = if backdrop {
                    (Rect::new(16.0, 168.0, 360.0, 208.0), Rect::new(20.0, 172.0, 352.0, 200.0))
                } else {
                    (Rect::new(64.0, 140.0, 264.0, 264.0), Rect::new(68.0, 144.0, 256.0, 256.0))
                };
                gfx.fill(frame, theme.tile);
                gfx.image_full(tex, picture, crate::gfx::Color::rgb(0xffffff));
            }
        }
        let first = row.saturating_sub(PICKER_ROWS / 2).min(total.saturating_sub(PICKER_ROWS));
        drawer.index = first;
        drawer.selectable = true;
        for i in first..(first + PICKER_ROWS).min(total) {
            let label = gfx.fit(Role::BodyBold, &entry(i), 480.0);
            drawer.row(gfx, &label, None, None, i == row, Step::None);
        }
        let confirm = Button::confirm(self.settings.confirm);
        let cancel = Button::cancel(self.settings.confirm);
        let hints = [(confirm, t.r#use), (Button::Face(Symbol::Triangle), if every { t.this_title } else { t.all_files }), (cancel, t.back)];
        if !note.is_empty() {
            drawer.note(gfx, &note, 1);
        } else if count == 0 {
            let folder = if backdrop { "ux0:data/PocketShelf/backdrops" } else { "ux0:data/PocketShelf/art" };
            drawer.note(gfx, &format!("{} {folder}", t.no_png), 1);
        }
        drawer.hints(gfx, &hints);
    }

    // --- Category manager ---------------------------------------------------

    pub(super) fn open_categories(&mut self) {
        self.categories = Some(Categories { row: 0, armed: None, note: String::new() });
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    /// The category on the highlighted row (row 0 is "New category").
    fn category_at(&self, row: usize) -> Option<crate::library::categories::Category> {
        row.checked_sub(1).and_then(|i| self.library.all_categories().get(i)).cloned()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn categories_input(&mut self, host: &mut dyn Host, step: Option<super::navigation::Step>, confirm: bool, cancel: bool, triangle: bool, square: bool, start: bool, shoulder: i32) {
        use super::navigation::{Direction, Step as Nav};
        let count = self.library.all_categories().len() + 1;
        let Some(manager) = &mut self.categories else { return };
        let dy = match step {
            Some(Nav::Press(Direction::Up)) | Some(Nav::Repeat(Direction::Up, _)) => -1,
            Some(Nav::Press(Direction::Down)) | Some(Nav::Repeat(Direction::Down, _)) => 1,
            _ => 0,
        };
        if dy != 0 {
            manager.row = (manager.row as i32 + dy).rem_euclid(count as i32) as usize;
            manager.armed = None;
            manager.note.clear();
        }
        let row = manager.row;
        let item = self.category_at(row);
        let t = self.t();
        if confirm {
            match &item {
                None => self.open_keyboard(keyboard::Target::NewCategory, ""),
                Some(c) if c.custom => self.open_keyboard(keyboard::Target::Category(c.id.clone()), &c.label.clone()),
                Some(_) => self.set_category_note(t.builtin_keep_names),
            }
        } else if triangle {
            self.open_keyboard(keyboard::Target::NewCategory, "");
        } else if square {
            self.delete_category_press(host);
        } else if start {
            self.toggle_hidden(host);
        } else if shoulder != 0 {
            self.reorder_category(host, shoulder);
        } else if cancel {
            self.categories = None;
            self.after_change();
        }
    }

    fn set_category_note(&mut self, note: &str) {
        if let Some(manager) = &mut self.categories {
            manager.note = note.to_string();
        }
    }

    /// Press twice on the same row: the first press asks, the second deletes.
    /// Titles placed in it go back to their own category.
    fn delete_category_press(&mut self, host: &mut dyn Host) {
        let Some(manager) = &self.categories else { return };
        let Some(item) = self.category_at(manager.row) else { return };
        let t = self.t();
        if !item.custom {
            self.set_category_note(t.builtin_no_delete);
            return;
        }
        if manager.armed.as_deref() != Some(&item.id) {
            if let Some(manager) = &mut self.categories {
                manager.armed = Some(item.id.clone());
                manager.note.clear();
            }
            return;
        }
        let id = item.id.clone();
        self.library.config.custom.retain(|c| c.id != id);
        self.library.config.hidden.retain(|h| *h != id);
        let placed: Vec<String> = self.library.overrides.titles.iter().filter(|(_, c)| c.category.as_deref() == Some(&id)).map(|(t, _)| t.clone()).collect();
        for title in placed {
            self.library.overrides.set(&title, crate::library::overrides::TitleOverride { category: None, ..self.library.overrides.titles[&title].clone() });
        }
        self.library.rebuild();
        self.save_overrides(host);
        self.save_categories(host);
        let count = self.library.all_categories().len();
        if let Some(manager) = &mut self.categories {
            manager.armed = None;
            manager.note = t.deleted.to_string();
            manager.row = manager.row.min(count);
        }
        self.after_change();
    }

    /// Show or hide the highlighted category in the tab bar; at least one
    /// category other than the smart ones stays shown.
    fn toggle_hidden(&mut self, host: &mut dyn Host) {
        let Some(manager) = &self.categories else { return };
        let Some(item) = self.category_at(manager.row) else { return };
        let t = self.t();
        let name = crate::i18n::category_label(t, &item);
        if let Some(manager) = &mut self.categories {
            manager.armed = None;
        }
        let hidden = &mut self.library.config.hidden;
        if hidden.contains(&item.id) {
            hidden.retain(|h| *h != item.id);
            self.set_category_note(&fill(t.now_shown, &[("name", &name)]));
        } else {
            let shown = self.library.all_categories().iter().filter(|c| !c.smart && !self.library.config.hidden.contains(&c.id)).count();
            if !item.smart && shown <= 1 {
                self.set_category_note(t.one_visible);
                return;
            }
            self.library.config.hidden.push(item.id.clone());
            self.set_category_note(&fill(t.now_hidden, &[("name", &name)]));
        }
        self.save_categories(host);
        self.after_change();
    }

    /// Move the highlighted category earlier or later in the tab order.
    fn reorder_category(&mut self, host: &mut dyn Host, delta: i32) {
        let Some(manager) = &self.categories else { return };
        let Some(index) = manager.row.checked_sub(1) else { return };
        let mut order: Vec<String> = self.library.all_categories().iter().map(|c| c.id.clone()).collect();
        let target = index as i32 + delta;
        if target < 0 || target as usize >= order.len() {
            return;
        }
        order.swap(index, target as usize);
        self.library.config.order = order;
        self.library.rebuild();
        self.save_categories(host);
        if let Some(manager) = &mut self.categories {
            manager.row = target as usize + 1;
            manager.armed = None;
            manager.note.clear();
        }
        self.after_change();
    }

    /// The keyboard's Done for a new category or a new name. A name another
    /// category has (in any case) is refused with a note.
    pub(super) fn commit_category(&mut self, host: &mut dyn Host, target: &keyboard::Target, text: &str) {
        if text.is_empty() {
            return;
        }
        let t = self.t();
        let taken = self.library.all_categories().iter().any(|c| {
            crate::i18n::category_label(t, c).to_lowercase() == text.to_lowercase() && !matches!(target, keyboard::Target::Category(id) if *id == c.id)
        });
        if taken {
            self.set_category_note(t.name_taken);
            return;
        }
        match target {
            keyboard::Target::NewCategory => {
                let ids: Vec<String> = std::iter::once(SEARCH_ID.to_string()).chain(self.library.all_categories().iter().map(|c| c.id.clone())).collect();
                let id = make_category_id(text, &|candidate| ids.iter().any(|i| i == candidate));
                debug_assert!(valid_category_id(&id));
                self.library.config.custom.push(crate::library::categories::Category::custom(&id, text));
                self.library.rebuild();
                let row = self.library.all_categories().iter().position(|c| c.id == id).map_or(0, |i| i + 1);
                if let Some(manager) = &mut self.categories {
                    manager.row = row;
                    manager.note.clear();
                }
            }
            keyboard::Target::Category(id) => {
                if let Some(c) = self.library.config.custom.iter_mut().find(|c| c.id == *id) {
                    c.label = text.to_string();
                }
                self.library.rebuild();
                self.set_category_note("");
            }
            _ => return,
        }
        self.save_categories(host);
        self.after_change();
    }

    pub(super) fn draw_categories(&mut self, gfx: &mut Gfx) {
        let Some(manager) = &self.categories else { return };
        let (row, armed, note) = (manager.row, manager.armed.clone(), manager.note.clone());
        let t = self.t();
        let theme = self.theme;
        let categories = self.library.all_categories().to_vec();
        let hidden = self.library.config.hidden.clone();
        let slide = self.drawer.at(self.now);
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.categories_row, self.settings.language), panel::Size::Wide, slide);
        let count = categories.len() + 1;
        let first = row.saturating_sub(CATEGORY_ROWS / 2).min(count.saturating_sub(CATEGORY_ROWS));
        drawer.index = first;
        drawer.selectable = true;
        for i in first..(first + CATEGORY_ROWS).min(count) {
            if i == 0 {
                drawer.row(gfx, t.new_category, None, Some(Button::Plus), i == row, Step::None);
            } else {
                let c = &categories[i - 1];
                let label = crate::i18n::category_label(t, c);
                let value = hidden.contains(&c.id).then_some(t.hidden);
                drawer.row(gfx, &label, value, None, i == row, Step::None);
            }
        }
        // Two lines of hints at the bottom, and above them the armed delete's
        // prompt or a note.
        let toggle = match self.category_at(row) {
            Some(c) if hidden.contains(&c.id) => t.show,
            _ => t.hide,
        };
        let confirm = Button::confirm(self.settings.confirm);
        let cancel = Button::cancel(self.settings.confirm);
        let first_line = [(confirm, t.rename), (Button::Face(Symbol::Triangle), t.create), (Button::Face(Symbol::Square), t.remove)];
        let second_line = [(Button::Start, toggle), (Button::ShoulderLR, t.reorder), (cancel, t.back)];
        let bottom = drawer.bottom;
        chrome::hints(gfx, &theme, &second_line, drawer.x, bottom - 16.0);
        chrome::hints(gfx, &theme, &first_line, drawer.x, bottom - 56.0);
        drawer.bottom = bottom - 80.0;
        if let Some(id) = armed {
            let name = categories.iter().find(|c| c.id == id).map(|c| crate::i18n::category_label(t, c)).unwrap_or_default();
            let text = fill(t.delete_again, &[("name", &name)]);
            chrome::text_with_button(gfx, &theme, &text, Button::Face(Symbol::Square), drawer.x, drawer.bottom - 16.0);
        } else if !note.is_empty() {
            drawer.note(gfx, &note, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_belong_to_a_title_by_name_or_id() {
        assert!(file_belongs_to("PCSA00069-48213.png", &["Gravity Daze", "PCSA00069"]));
        assert!(file_belongs_to("gravity-daze.PNG", &["Gravity Daze"]));
        assert!(file_belongs_to("Pokémon Stadium 2.png", &["Pokemon: Stadium 2"]));
        // A name and more after a dash belongs too, as in Pocket Shelf 0.1.
        assert!(file_belongs_to("Gravity Daze 2.png", &["Gravity Daze"]));
        assert!(!file_belongs_to("other.png", &["Gravity Daze", ""]));
    }
}
