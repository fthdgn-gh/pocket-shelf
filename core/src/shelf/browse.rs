//! The title editor's Browse: a box art or a backdrop taken from any folder
//! of the memory card. It opens on the folder the last one was taken from
//! (`folders.json`), lists folders and PNG files a folder at a time (the
//! host reads the folder off the frame, [`Host::list_folder`]), shows the
//! highlighted picture, and copies the one chosen to `art/` or `backdrops/`
//! ([`Host::import_picture`]), so the title's changes name a file there as
//! they do for every other picture. The decoded preview goes to the tile
//! at once: the copy may still be on its way to the card.

use std::collections::HashMap;

use super::chrome::Button;
use super::navigation::{Direction, Step as Nav};
use super::panel::{self, Drawer, Step};
use super::Shelf;
use crate::gfx::{Format, Gfx, Rect, TexId};
use crate::host::{DecodeAs, FolderEntry, Host, Picture, PictureKey};
use crate::i18n::upper;
use crate::library::settings::FOLDERS_FILE;
use crate::shapes::Symbol;
use crate::text::Role;

/// Rows shown at once.
const ROWS: usize = 5;
/// Frames the highlight rests before its picture is loaded.
const PREVIEW_DELAY: u32 = 8;
/// Where Browse opens the first time.
pub const START: &str = "ux0:";

/// A folder's path with `name` in it: `ux0:` and `pictures` make
/// `ux0:pictures`, `ux0:pictures` and `a.png` make `ux0:pictures/a.png`.
pub fn join(dir: &str, name: &str) -> String {
    // On the list of devices the name is the whole path (`ux0:`).
    if dir.is_empty() || dir.ends_with(':') || dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

/// A saved folder as Browse opens it: a device's path has no slash before
/// it (0.5.2's development builds saved `/ux0:pictures` from the list of
/// devices).
pub fn clean(dir: &str) -> String {
    let dir = dir.trim();
    match dir.find(':') {
        Some(at) if dir[..at].starts_with('/') => dir.trim_start_matches('/').to_string(),
        _ => dir.to_string(),
    }
}

/// The folder holding `dir`; `""` (the list of devices) above a device.
pub fn parent(dir: &str) -> String {
    let dir = dir.trim_end_matches('/');
    if dir.ends_with(':') || dir.is_empty() {
        return String::new();
    }
    match dir.rfind('/') {
        Some(at) => dir[..at].to_string(),
        None => dir.split_inclusive(':').next().unwrap_or("").to_string(),
    }
}

pub fn is_png(name: &str) -> bool {
    name.to_lowercase().ends_with(".png")
}

/// A folder's entries as Browse lists them: folders, then PNG files, each
/// in name order regardless of case; hidden ones and other files left out.
pub fn sorted(entries: Vec<FolderEntry>) -> Vec<FolderEntry> {
    let mut kept: Vec<FolderEntry> = entries.into_iter().filter(|e| !e.name.starts_with('.') && (e.folder || is_png(&e.name))).collect();
    kept.sort_by(|a, b| b.folder.cmp(&a.folder).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    kept
}

/// The file an imported picture gets: the title id and the source path's
/// CRC, so taking the same file again writes the same one.
pub fn import_name(id: &str, from: &str) -> String {
    let id: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    format!("{id}-{:08x}.png", crate::online::gamedb::crc32(from.as_bytes()))
}

pub struct Browser {
    pub backdrop: bool,
    /// The folder shown; `""` lists the devices.
    pub dir: String,
    /// None while the host reads the folder.
    pub entries: Option<Vec<FolderEntry>>,
    pub row: usize,
    pub note: String,
    /// The entry to highlight once the folder is read (the folder just left).
    select: Option<String>,
    /// The picture asked for the highlighted file, and once decoded its
    /// texture, accent and shown part.
    pub preview: Option<(PictureKey, Option<(TexId, Option<u32>, [f32; 4])>)>,
    pub preview_wait: u32,
    /// Whether each file decoded, learned from its preview.
    pub decodes: HashMap<String, bool>,
    /// A confirm on a file not checked yet: used once it decodes.
    pub waiting: bool,
}

impl Browser {
    /// The rows: "Up one folder" first, except on the list of devices.
    fn up_row(&self) -> bool {
        !self.dir.is_empty()
    }

    pub(super) fn count(&self) -> usize {
        usize::from(self.up_row()) + self.entries.as_ref().map_or(0, Vec::len)
    }

    /// The entry on a row; None for "Up one folder".
    fn entry(&self, row: usize) -> Option<&FolderEntry> {
        let index = row.checked_sub(usize::from(self.up_row()))?;
        self.entries.as_ref()?.get(index)
    }

    fn path(&self, row: usize) -> Option<String> {
        self.entry(row).map(|e| join(&self.dir, &e.name))
    }

    pub(super) fn row(&self) -> usize {
        self.row
    }
}

impl Shelf {
    /// Open Browse on the folder the last box art or backdrop came from.
    pub(super) fn open_browser(&mut self, host: &mut dyn Host, backdrop: bool) {
        let folders = &self.settings.folders;
        let dir = if backdrop { &folders.backdrops } else { &folders.art };
        let dir = clean(dir);
        let dir = if dir.is_empty() { START.to_string() } else { dir };
        self.browser = Some(Browser { backdrop, dir: String::new(), entries: None, row: 0, note: String::new(), select: None, preview: None, preview_wait: 0, decodes: HashMap::new(), waiting: false });
        self.browse_to(host, dir, None, "");
    }

    /// Show a folder; `select` is the entry to highlight in it (the folder
    /// left when going up), `note` what to say once it is read.
    fn browse_to(&mut self, host: &mut dyn Host, dir: String, select: Option<String>, note: &str) {
        self.drop_preview();
        let Some(browser) = &mut self.browser else { return };
        browser.dir = dir.clone();
        browser.entries = None;
        browser.row = 0;
        browser.select = select;
        browser.note = note.to_string();
        browser.waiting = false;
        if !host.list_folder(&dir) {
            browser.entries = Some(Vec::new());
        }
    }

    /// The preview's texture goes (freed at the next draw).
    fn drop_preview(&mut self) {
        let Some(browser) = &mut self.browser else { return };
        if let Some((_, Some((tex, ..)))) = browser.preview.take() {
            self.pending_free.push(tex);
        }
    }

    /// Close Browse, back to the picker.
    pub(super) fn close_browser(&mut self) {
        self.drop_preview();
        self.browser = None;
    }

    /// The host's answer for a folder.
    pub fn folder_listed(&mut self, host: &mut dyn Host, dir: &str, entries: Option<Vec<FolderEntry>>) {
        let t = self.t();
        let Some(browser) = &mut self.browser else { return };
        if browser.dir != dir || browser.entries.is_some() {
            return;
        }
        let Some(entries) = entries else {
            // A folder that does not read (gone, or no access): its parent,
            // with a note; the list of devices says so itself.
            host.log(&format!("browse: {dir} does not read"));
            if dir.is_empty() {
                browser.entries = Some(Vec::new());
                browser.note = t.browse_failed.to_string();
            } else {
                self.browse_to(host, parent(dir), None, t.browse_failed);
            }
            return;
        };
        let select = browser.select.take();
        let up = usize::from(browser.up_row());
        let at = select.and_then(|name| entries.iter().position(|e| e.folder && e.name == name));
        browser.entries = Some(entries);
        // The folder just left, else the first entry.
        browser.row = at.map_or(if browser.count() > up { up } else { 0 }, |i| i + up);
        self.browser_moved(false);
    }

    pub(super) fn browser_input(&mut self, host: &mut dyn Host, step: Option<Nav>, confirm: bool, cancel: bool, triangle: bool) {
        let Some(browser) = &mut self.browser else { return };
        if cancel {
            self.close_browser();
            return;
        }
        if browser.entries.is_none() {
            return;
        }
        let count = browser.count().max(1);
        let before = browser.row;
        match step {
            Some(Nav::Press(Direction::Up)) | Some(Nav::Repeat(Direction::Up, _)) => browser.row = (browser.row + count - 1) % count,
            Some(Nav::Press(Direction::Down)) | Some(Nav::Repeat(Direction::Down, _)) => browser.row = (browser.row + 1) % count,
            _ => {}
        }
        if browser.row != before {
            self.browser_moved(true);
        }
        let Some(browser) = &self.browser else { return };
        let up = triangle || (confirm && browser.up_row() && browser.row == 0);
        if up {
            if browser.up_row() {
                let left = browser.dir.trim_end_matches('/').rsplit(['/', ':']).find(|s| !s.is_empty()).map(String::from);
                let dir = parent(&browser.dir);
                // A device goes back to the list of devices, highlighting it.
                let select = if dir.is_empty() { Some(browser.dir.trim_end_matches('/').to_string()) } else { left };
                self.browse_to(host, dir, select, "");
            }
            return;
        }
        if !confirm {
            return;
        }
        let Some(entry) = browser.entry(browser.row).cloned() else { return };
        let path = join(&browser.dir, &entry.name);
        if entry.folder {
            self.browse_to(host, path, None, "");
        } else {
            self.browser_use(host, path);
        }
    }

    /// A new row: its picture after the highlight rests. `clear` drops the
    /// note (a folder's note stays until the highlight moves).
    fn browser_moved(&mut self, clear: bool) {
        let cant_read = self.t().cant_read;
        self.drop_preview();
        let Some(browser) = &mut self.browser else { return };
        browser.waiting = false;
        browser.preview_wait = PREVIEW_DELAY;
        let known_bad = browser.path(browser.row).is_some_and(|p| browser.decodes.get(&p) == Some(&false));
        if known_bad {
            browser.note = cant_read.to_string();
        } else if clear {
            browser.note.clear();
        }
    }

    /// Highlight a row by touch, as the buttons would.
    pub(super) fn browser_focus(&mut self, row: usize) {
        let Some(browser) = &mut self.browser else { return };
        let row = row.min(browser.count().saturating_sub(1));
        if browser.row != row {
            browser.row = row;
            self.browser_moved(true);
        }
    }

    /// Ask for the highlighted file's picture once the highlight rested.
    pub(super) fn browser_frame(&mut self, host: &mut dyn Host) {
        if let Some(path) = self.pending_browse.take() {
            self.browser_use(host, path);
        }
        let Some(browser) = &mut self.browser else { return };
        if browser.preview_wait == 0 {
            return;
        }
        browser.preview_wait -= 1;
        if browser.preview_wait > 0 {
            return;
        }
        let Some(entry) = browser.entry(browser.row).filter(|e| !e.folder) else { return };
        let path = join(&browser.dir, &entry.name);
        let as_ = if browser.backdrop { DecodeAs::Backdrop } else { DecodeAs::FittedArt };
        let key = PictureKey::File { path, as_ };
        browser.preview = Some((key.clone(), None));
        host.request(key);
    }

    /// A picture answered for Browse's preview; true when it was.
    pub(super) fn browser_picture(&mut self, gfx: &mut Gfx, key: &PictureKey, picture: &Option<Picture>) -> bool {
        let cant_read = self.t().cant_read;
        let Some(browser) = &mut self.browser else { return false };
        if browser.preview.as_ref().map(|p| &p.0) != Some(key) {
            return false;
        }
        let PictureKey::File { path, .. } = key else { return false };
        if let Some(p) = picture {
            let tex = gfx.add_texture(p.w, p.h, Format::Rgba8, p.rgba.clone(), true);
            browser.preview = Some((key.clone(), Some((tex, p.accent, super::art::shown_part(p.content, p.w, p.h)))));
        } else {
            browser.note = cant_read.to_string();
        }
        browser.decodes.insert(path.clone(), picture.is_some());
        // A confirm that waited for this file is carried out next frame.
        if std::mem::take(&mut browser.waiting) && picture.is_some() {
            self.pending_browse = Some(path.clone());
        }
        true
    }

    /// Use a file: once it decoded, it is copied and set on the title.
    fn browser_use(&mut self, host: &mut dyn Host, path: String) {
        let cant_read = self.t().cant_read;
        let Some(id) = self.editor.as_ref().map(|e| e.id.clone()) else { return };
        let Some(browser) = &mut self.browser else { return };
        match browser.decodes.get(&path) {
            Some(false) => {
                browser.note = cant_read.to_string();
                return;
            }
            None => {
                // Checked first: its preview is asked for now, unless it was.
                browser.waiting = true;
                if browser.preview.is_none() {
                    browser.preview_wait = 1;
                }
                return;
            }
            Some(true) => {}
        }
        let backdrop = browser.backdrop;
        let dir = browser.dir.clone();
        let decoded = browser.preview.take().and_then(|(_, decoded)| decoded);
        let file = import_name(&id, &path);
        let folder = if backdrop { "backdrops" } else { "art" };
        if !host.import_picture(&path, &format!("{folder}/{file}"), !backdrop) {
            host.log(&format!("browse: {path} not copied"));
            if let Some((tex, ..)) = decoded {
                self.pending_free.push(tex);
            }
            return;
        }
        host.log(&format!("browse: {path} as {folder}/{file}"));
        if backdrop {
            let key = PictureKey::Backdrop { id: id.clone(), file: file.clone() };
            if let Some((tex, ..)) = decoded {
                self.wanted = Some(key.clone());
                self.show_backdrop(super::Shown { key, tex, ready_at: None });
            }
            self.library.patch(&id, |c| c.backdrop = Some(file));
            self.settings.folders.backdrops = dir;
        } else {
            // The preview goes to the tile cache: the copy may still be on
            // its way to the card, and a read before it lands would cache
            // "no picture".
            self.library.add_art(&file);
            self.art.forget_missing();
            if let Some((tex, accent, shown)) = decoded {
                self.art.expect(PictureKey::Art(file.clone()));
                self.pending_art.push((PictureKey::Art(file.clone()), tex, accent, shown));
            }
            self.library.patch(&id, |c| c.art = Some(file));
            self.settings.folders.art = dir;
        }
        host.write(FOLDERS_FILE, self.settings.folders.render().as_bytes());
        if !self.library.overrides.save(host) {
            host.log("Title changes not saved");
        }
        self.after_change_pub();
        self.browser = None;
        self.picker = None;
    }

    pub(super) fn draw_browser(&mut self, gfx: &mut Gfx) {
        let Some(browser) = &self.browser else { return };
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let title = if browser.backdrop { t.backdrop } else { t.box_art };
        let mut drawer = Drawer::open(gfx, &theme, &upper(title, self.settings.language), panel::Size::Wide, slide);
        if let Some((_, Some((tex, ..)))) = &browser.preview {
            if gfx.textures.ready(*tex) {
                let (frame, picture) = if browser.backdrop {
                    (Rect::new(16.0, 168.0, 360.0, 208.0), Rect::new(20.0, 172.0, 352.0, 200.0))
                } else {
                    (Rect::new(64.0, 140.0, 264.0, 264.0), Rect::new(68.0, 144.0, 256.0, 256.0))
                };
                gfx.fill(frame, theme.tile);
                gfx.image_full(*tex, picture, crate::gfx::Color::rgb(0xffffff));
            }
        }
        let count = browser.count();
        let first = browser.row.saturating_sub(ROWS / 2).min(count.saturating_sub(ROWS));
        drawer.index = first;
        drawer.selectable = true;
        for i in first..(first + ROWS).min(count) {
            let label = match browser.entry(i) {
                None => t.browse_up.to_string(),
                // A device is named as it is (`ux0:`), a folder with a `/`.
                Some(e) if e.folder && !e.name.ends_with(':') => format!("{}/", e.name),
                Some(e) => e.name.clone(),
            };
            let label = gfx.fit(Role::BodyBold, &label, 480.0);
            drawer.row(gfx, &label, None, None, i == browser.row, Step::None);
        }
        // The folder, then what there is to say about it.
        let place = if browser.dir.is_empty() { t.browse_devices.to_string() } else { browser.dir.clone() };
        let note = if browser.entries.is_none() {
            t.browse_reading.to_string()
        } else if !browser.note.is_empty() {
            browser.note.clone()
        } else if browser.entries.as_ref().is_some_and(Vec::is_empty) {
            t.browse_empty.to_string()
        } else {
            String::new()
        };
        let place = gfx.fit(Role::Small, &place, 540.0);
        let text = if note.is_empty() { place } else { format!("{place}\n{note}") };
        drawer.note(gfx, &text, 1);
        let confirm = Button::confirm(self.settings.confirm);
        let cancel = Button::cancel(self.settings.confirm);
        let on_file = browser.entry(browser.row).is_some_and(|e| !e.folder);
        let action = if on_file { t.r#use } else { t.browse_open };
        let mut hints = vec![(confirm, action)];
        if browser.up_row() {
            hints.push((Button::Face(Symbol::Triangle), t.browse_up));
        }
        hints.push((cancel, t.back));
        drawer.hints(gfx, &hints);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_join_and_go_up_to_the_devices() {
        assert_eq!(join("ux0:", "pictures"), "ux0:pictures");
        assert_eq!(join("ux0:pictures", "a.png"), "ux0:pictures/a.png");
        assert_eq!(join("ux0:/", "data"), "ux0:/data");
        // From the list of devices: the device itself, no slash before it.
        assert_eq!(join("", "ux0:"), "ux0:");
        assert_eq!(join(&join("", "ux0:"), "picture"), "ux0:picture");
        assert_eq!(clean("/ux0:picture/Box art"), "ux0:picture/Box art");
        assert_eq!(clean("ux0:picture"), "ux0:picture");
        assert_eq!(clean(""), "");
        assert_eq!(parent("ux0:pictures/covers"), "ux0:pictures");
        assert_eq!(parent("ux0:pictures"), "ux0:");
        assert_eq!(parent("ux0:/pictures"), "ux0:");
        assert_eq!(parent("ux0:"), "");
        assert_eq!(parent(""), "");
    }

    #[test]
    fn folders_come_first_then_png_files_and_names_are_stable() {
        let e = |name: &str, folder| FolderEntry { name: name.into(), folder };
        let listed = sorted(vec![e("b.PNG", false), e("notes.txt", false), e("Zelda", true), e(".hidden", true), e("a.png", false), e("art", true)]);
        let names: Vec<&str> = listed.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["art", "Zelda", "a.png", "b.PNG"]);
        assert_eq!(import_name("PCSA00069", "ux0:pictures/a.png"), import_name("PCSA00069", "ux0:pictures/a.png"));
        assert_ne!(import_name("PCSA00069", "ux0:pictures/a.png"), import_name("PCSA00069", "ux0:pictures/b.png"));
        assert!(import_name("ra0/x", "p").starts_with("ra0x-"));
    }

    use crate::host::{Event, PluginState, Status, TitleInfo};
    use crate::library::catalog::load_catalog;
    use crate::library::settings::{Folders, Settings, UpdateChannel};
    use crate::library::testing::Files;
    use crate::library::Library;

    /// A data folder that records folder listings, picture requests and
    /// imports.
    #[derive(Default)]
    struct BrowseHost {
        files: Files,
        listed: Vec<String>,
        requested: Vec<PictureKey>,
        imported: Vec<(String, String, bool)>,
    }

    impl Host for BrowseHost {
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
        fn list(&mut self, _: &str) -> Vec<String> {
            Vec::new()
        }
        fn status(&mut self) -> Status {
            Status::default()
        }
        fn scan(&mut self, _: bool) {}
        fn request(&mut self, key: PictureKey) {
            self.requested.push(key);
        }
        fn poll(&mut self, _: &mut Vec<Event>) {}
        fn log(&mut self, _: &str) {}
        fn launch(&mut self, _: &str, _: Option<crate::host::Emulator>) -> bool {
            true
        }
        fn plugin_state(&mut self) -> PluginState {
            PluginState::On
        }
        fn enable_plugin(&mut self) -> bool {
            true
        }
        fn list_folder(&mut self, dir: &str) -> bool {
            self.listed.push(dir.to_string());
            true
        }
        fn import_picture(&mut self, from: &str, to: &str, fit: bool) -> bool {
            self.imported.push((from.into(), to.into(), fit));
            true
        }
    }

    fn entry(name: &str, folder: bool) -> FolderEntry {
        FolderEntry { name: name.into(), folder }
    }

    fn frames(shelf: &mut Shelf, host: &mut BrowseHost, n: u32) {
        for _ in 0..n {
            shelf.browser_frame(host);
        }
    }

    #[test]
    fn a_picture_from_any_folder_is_copied_set_and_its_folder_kept() {
        let mut host = BrowseHost::default();
        let library = Library::load(&mut host);
        let mut shelf = Shelf::new(Settings::defaults(UpdateChannel::Off), library, Status::default());
        let infos = [TitleInfo { id: "PCSA00069".into(), title: "Gravity".into(), platform: "vita".into(), ..Default::default() }];
        shelf.set_titles(load_catalog(&infos), true);
        shelf.open_editor();
        let mut gfx = Gfx::new();

        // The first time it opens on the memory card's root.
        shelf.open_browser(&mut host, false);
        assert_eq!(host.listed, ["ux0:"]);
        shelf.folder_listed(&mut host, "ux0:", Some(vec![entry("data", true), entry("pictures", true)]));
        let down = Some(Nav::Press(Direction::Down));
        shelf.browser_input(&mut host, down, false, false, false);
        shelf.browser_input(&mut host, None, true, false, false);
        assert_eq!(host.listed.last().map(String::as_str), Some("ux0:pictures"));
        shelf.folder_listed(&mut host, "ux0:pictures", Some(vec![entry("covers", true), entry("gravity.png", false)]));
        // The first entry is highlighted, after the up row.
        assert_eq!(shelf.browser.as_ref().map(|b| b.row), Some(1));
        shelf.browser_input(&mut host, down, false, false, false);

        // Its picture once the highlight rests; a confirm before it waits.
        shelf.browser_input(&mut host, None, true, false, false);
        frames(&mut shelf, &mut host, PREVIEW_DELAY);
        let key = PictureKey::File { path: "ux0:pictures/gravity.png".into(), as_: DecodeAs::FittedArt };
        assert_eq!(host.requested.last(), Some(&key));
        assert!(host.imported.is_empty(), "not before it decodes");
        let picture = Picture::new(2, 2, vec![255; 16], Some(0));
        assert!(shelf.browser_picture(&mut gfx, &key, &Some(picture)));
        frames(&mut shelf, &mut host, 1);
        let file = import_name("PCSA00069", "ux0:pictures/gravity.png");
        assert_eq!(host.imported, [("ux0:pictures/gravity.png".into(), format!("art/{file}"), true)]);
        assert_eq!(shelf.library.game("PCSA00069").and_then(|g| g.art.clone()), Some(file));
        assert!(shelf.browser.is_none() && shelf.picker.is_none());
        let saved = Folders::parse(&host.read(FOLDERS_FILE).unwrap());
        assert_eq!(saved, Folders { art: "ux0:pictures".into(), backdrops: String::new() });

        // Next time it opens there; up goes back to the root with the
        // folder highlighted, and a folder that does not read goes up.
        shelf.open_browser(&mut host, false);
        assert_eq!(host.listed.last().map(String::as_str), Some("ux0:pictures"));
        shelf.folder_listed(&mut host, "ux0:pictures", Some(vec![entry("gravity.png", false)]));
        shelf.browser_input(&mut host, None, false, false, true);
        shelf.folder_listed(&mut host, "ux0:", Some(vec![entry("data", true), entry("pictures", true)]));
        assert_eq!(shelf.browser.as_ref().map(|b| b.row), Some(2), "pictures/, after the up row and data/");
        shelf.browser_input(&mut host, None, true, false, false);
        shelf.folder_listed(&mut host, "ux0:pictures", None);
        assert_eq!(host.listed.last().map(String::as_str), Some("ux0:"));
        shelf.browser_input(&mut host, None, false, true, false);
        assert!(shelf.browser.is_none());
    }

}
