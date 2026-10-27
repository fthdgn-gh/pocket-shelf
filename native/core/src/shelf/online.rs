//! Find artwork in the title editor (`components/online.tsx`): the online
//! flow (`crate::online::flow`) on the shelf's buttons, keyboard, pictures
//! and drawer.

use super::chrome::Button;
use super::keyboard::Target;
use super::navigation::{Direction, Step as Nav};
use super::panel::{self, Drawer, Step};
use super::Shelf;
use crate::gfx::{Color, Format, Gfx, Rect};
use crate::host::{Host, Picture, PictureKey};
use crate::i18n::upper;
use crate::online::flow::{Action, Online, Step as Flow, KINDS, ONLINE_ROWS};
use crate::online::steamgriddb::AssetKind;
use crate::online::{libretro, scraper_for, Scraper};
use crate::text::Role;

impl Shelf {
    /// The editor's "Find artwork": the edited title on its scraper.
    pub(super) fn start_online(&mut self, host: &mut dyn Host) {
        let Some(editor) = &self.editor else { return };
        let Some(game) = self.library.game(&editor.id).cloned() else { return };
        // libretro for a title its server has no folder for says so.
        let scraper = scraper_for(self.settings.scraper, &game).unwrap_or(Scraper::Libretro);
        let folder = libretro::libretro_folder(game.console.as_deref(), game.platform);
        // libretro searches the playlist label, SteamGridDB the title.
        let label = self.library.base_game(&game.id).map(|g| g.title.to_string()).unwrap_or_default();
        let term = if scraper == Scraper::Libretro && !label.is_empty() { label } else { game.title.to_string() };
        let t = self.t();
        let (online, actions) = Online::start(host, t, scraper, &game.id, folder, &term, &mut self.listings);
        self.online = Some(online);
        self.online_tex = None;
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
        self.online_actions(host, actions);
    }

    /// Carry out what the flow asks.
    fn online_actions(&mut self, host: &mut dyn Host, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::AskText { key, text } => self.open_keyboard(if key { Target::ApiKey } else { Target::Search }, &text),
                Action::Preview(key) => host.request(key),
                Action::Apply(kind, file) => {
                    let Some(id) = self.editor.as_ref().map(|e| e.id.clone()) else { continue };
                    match kind {
                        AssetKind::Icon => {
                            // The new file has to be in the art index first.
                            self.library.reload_art(host);
                            self.library.patch(&id, |c| c.art = Some(file));
                        }
                        AssetKind::Backdrop => {
                            self.library.patch(&id, |c| c.backdrop = Some(file));
                            self.backdrop_wait = 1;
                        }
                    }
                    if !self.library.overrides.save(host) {
                        host.log("Title changes not saved");
                    }
                    self.after_change_pub();
                }
            }
        }
    }

    pub(super) fn online_frame(&mut self, host: &mut dyn Host) {
        let Some(online) = &mut self.online else { return };
        let t = crate::i18n::messages(self.settings.language);
        let actions = online.frame(host, t, &mut self.listings);
        self.online_actions(host, actions);
    }

    pub(super) fn online_input(&mut self, host: &mut dyn Host, step: Option<Nav>, confirm: bool, cancel: bool) {
        let t = self.t();
        let Some(online) = &mut self.online else { return };
        let (dx, dy) = match step {
            Some(Nav::Press(Direction::Up)) | Some(Nav::Repeat(Direction::Up, _)) => (0, -1),
            Some(Nav::Press(Direction::Down)) | Some(Nav::Repeat(Direction::Down, _)) => (0, 1),
            // A held direction does not step through candidates.
            Some(Nav::Press(Direction::Left)) => (-1, 0),
            Some(Nav::Press(Direction::Right)) => (1, 0),
            _ => (0, 0),
        };
        let mut actions = if dx != 0 || dy != 0 { online.step_by(host, t, dx, dy) } else { Vec::new() };
        if confirm {
            actions.extend(online.confirm(host, t));
        } else if cancel && online.back(host) {
            let overrides = &self.library.overrides.titles;
            online.close(host, |kind, file| {
                overrides.values().any(|c| match kind {
                    AssetKind::Icon => c.art.as_deref() == Some(file),
                    AssetKind::Backdrop => c.backdrop.as_deref() == Some(file),
                })
            });
            self.online = None;
            if let Some(tex) = self.online_tex.take() {
                self.pending_free.push(tex);
            }
        }
        self.online_actions(host, actions);
    }

    /// The keyboard's Done for SteamGridDB's search term or API key.
    pub(super) fn commit_online(&mut self, host: &mut dyn Host, key: bool, text: &str) {
        let t = self.t();
        let Some(online) = &mut self.online else { return };
        let actions = if key { online.submit_key(host, t, &mut self.listings, text) } else { online.submit_term(host, t, &mut self.listings, text) };
        self.online_actions(host, actions);
    }

    /// A picture answered for a preview; true when it was one (a backdrop
    /// preview keeps its own texture; an icon's goes to the art cache too).
    pub(super) fn online_picture(&mut self, gfx: &mut Gfx, key: &PictureKey, picture: &Option<Picture>) -> bool {
        let t = self.t();
        let Some(online) = &mut self.online else { return false };
        if !online.previewed(t, key, picture.is_some()) {
            return false;
        }
        if let (PictureKey::Backdrop { .. }, Some(p)) = (key, picture) {
            if let Some(old) = self.online_tex.take() {
                gfx.textures.free(old);
            }
            self.online_tex = Some(gfx.add_texture(p.w, p.h, Format::Rgba8, p.rgba.clone(), true));
        }
        matches!(key, PictureKey::Backdrop { .. })
    }

    pub(super) fn draw_online(&mut self, gfx: &mut Gfx) {
        let Some(online) = &self.online else { return };
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let mut drawer = Drawer::open(gfx, &theme, &upper(online.scraper.name(), self.settings.language), panel::Size::Wide, slide);
        let confirm = Button::confirm(self.settings.confirm);
        let cancel = Button::cancel(self.settings.confirm);
        match online.step {
            Flow::Key => {
                drawer.row(gfx, t.enter_key, None, None, true, Step::Right);
                drawer.space(4.0);
                drawer.text(gfx, t.key_help);
                drawer.text(gfx, "ux0:data/PocketShelf/steamgriddb.txt");
                if !online.status.is_empty() {
                    drawer.note(gfx, &online.status, 1);
                }
                drawer.hints(gfx, &[(confirm, t.r#type), (cancel, t.back)]);
            }
            Flow::Games => {
                let count = online.games.len() + 1;
                let first = online.row.saturating_sub(ONLINE_ROWS / 2).min(count.saturating_sub(ONLINE_ROWS));
                for i in first..(first + ONLINE_ROWS).min(count) {
                    if i == 0 {
                        let term = gfx.fit(Role::BodyBold, &online.term, 170.0);
                        drawer.row(gfx, t.search, Some(&term), None, online.row == 0, Step::Right);
                    } else {
                        let game = &online.games[i - 1];
                        let label = gfx.fit(Role::BodyBold, &game.name, if game.year.is_some() { 200.0 } else { 240.0 });
                        let year = game.year.map(|y| y.to_string());
                        drawer.row(gfx, &label, year.as_deref(), None, online.row == i, Step::None);
                    }
                }
                drawer.note(gfx, &online.status, 1);
                drawer.hints(gfx, &[(confirm, if online.row == 0 { t.edit } else { t.choose }), (cancel, t.back)]);
            }
            Flow::Assets => {
                for (i, kind) in KINDS.iter().enumerate() {
                    let count = online.assets[i].len();
                    let position = if count == 0 { t.none.to_string() } else { format!("{} / {count}", online.index[i] + 1) };
                    let label = if *kind == AssetKind::Icon { t.icon } else { t.backdrop };
                    drawer.row(gfx, label, Some(&position), None, online.row == i, Step::Both);
                }
                // The candidate at the size it is judged at: an icon as a
                // tile, a backdrop in the screen's shape.
                let top = drawer.space(128.0);
                let kind = KINDS[online.row.min(1)];
                let slot = online.row.min(1);
                if online.shown[slot] {
                    let tex = match (kind, &online.preview[slot]) {
                        (AssetKind::Backdrop, _) => self.online_tex,
                        (AssetKind::Icon, Some(key)) => self.art.texture(key),
                        _ => None,
                    };
                    if let Some(tex) = tex.filter(|&t| gfx.textures.ready(t)) {
                        let (w, h) = if kind == AssetKind::Icon { (96.0, 96.0) } else { (212.0, 120.0) };
                        let rect = Rect::new(drawer.x + (drawer.w - w) / 2.0, top + (128.0 - h) / 2.0, w, h);
                        gfx.image_full(tex, rect, Color::rgb(0xffffff));
                    }
                }
                let note = if online.status.is_empty() {
                    gfx.fit(Role::BodyBold, online.chosen.as_ref().map_or("", |c| c.name.as_str()), 250.0)
                } else {
                    online.status.clone()
                };
                drawer.note(gfx, &note, 1);
                drawer.hints(gfx, &[(confirm, t.r#use), (Button::DpadHorizontal, t.next), (cancel, t.back)]);
            }
        }
    }
}
