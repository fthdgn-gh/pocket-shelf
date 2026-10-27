//! Fetch artwork (`components/scrape.tsx`) and Clean up artwork
//! (`components/cleanup.tsx`), both opened from the SELECT menu.

use std::collections::HashSet;

use super::chrome::Button;
use super::navigation::{Direction, Step as Nav};
use super::panel::{self, Drawer, Step};
use super::{cycle_by, Shelf};
use crate::gfx::Gfx;
use crate::host::Host;
use crate::i18n::{category_label, fill, upper};
use crate::library::categories::SYSTEM_ID;
use crate::library::list_png;
use crate::library::overrides::{files_in_use, without_files};
use crate::library::settings::ScrapeSource;
use crate::online::fetch::{self, Action, Fetch, Job, MODE_ROW, SETUP_ROWS, SOURCE_ROW, START_ROW};
use crate::online::steamgriddb::AssetKind;
use crate::text::Role;

/// Pictures applied between writes of the title changes during a run.
const SAVE_EVERY: u32 = 20;

/// The Fetch artwork drawer: the run and the choices it starts from.
pub struct FetchPanel {
    pub flow: Fetch,
    /// Category ids and labels to choose from; "" is every title but the
    /// system applications.
    scopes: Vec<(String, String)>,
    /// Titles the setup's choices would fetch for, counted when a choice
    /// changes (a pass over every title).
    wanted: usize,
}

/// The Clean up artwork drawer.
pub struct Cleanup {
    row: usize,
    every: bool,
    armed: bool,
    note: String,
    /// The folders as read on open, less what was deleted since.
    art: Vec<String>,
    backdrops: Vec<String>,
    /// The files the chosen mode deletes.
    pub(super) targets: (Vec<String>, Vec<String>),
}

fn source_name(t: &crate::locales::Messages, source: ScrapeSource) -> &'static str {
    match source {
        ScrapeSource::Auto => t.source_auto,
        ScrapeSource::SteamGridDb => "SteamGridDB",
        ScrapeSource::Libretro => "libretro",
    }
}

impl Shelf {
    // --- Fetch artwork -----------------------------------------------------

    pub(super) fn open_fetch(&mut self) {
        let t = self.t();
        // "All" leaves out the system applications: a search for "Settings"
        // or "Music" finds an unrelated game. Their own category can still
        // be chosen.
        let mut scopes = vec![(String::new(), t.all_titles.to_string())];
        for c in self.library.all_categories() {
            if !c.smart && !self.library.titles_of(&c.id).is_empty() {
                scopes.push((c.id.clone(), category_label(t, c)));
            }
        }
        // Start on the current category when it is one of the choices.
        let scope = scopes.iter().position(|(id, _)| *id == self.tab).unwrap_or(0);
        self.fetch = Some(FetchPanel { flow: Fetch::new(scope), scopes, wanted: 0 });
        self.count_wanted();
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    /// The scope's titles as games' indexes.
    fn scope_titles(&self, id: &str) -> Vec<usize> {
        if id.is_empty() {
            let games = self.library.games();
            (0..games.len()).filter(|&i| self.library.base_game(&games[i].id).is_some_and(|b| &*b.category != SYSTEM_ID)).collect()
        } else {
            self.library.titles_of(id)
        }
    }

    fn fetch_jobs(&self) -> Vec<Job> {
        let Some(panel) = &self.fetch else { return Vec::new() };
        let id = &panel.scopes[panel.flow.scope].0;
        let games = self.library.games();
        self.scope_titles(id)
            .into_iter()
            .filter_map(|i| {
                let game = &games[i];
                let label = self.library.base_game(&game.id).map(|b| b.title.to_string()).unwrap_or_default();
                Job::for_game(game, &label, self.settings.scraper, panel.flow.replace)
            })
            .collect()
    }

    fn count_wanted(&mut self) {
        let Some(panel) = &self.fetch else { return };
        let (id, replace) = (panel.scopes[panel.flow.scope].0.clone(), panel.flow.replace);
        let games = self.library.games();
        let count = self
            .scope_titles(&id)
            .into_iter()
            .filter(|&i| !fetch::kinds_for(&games[i], replace).is_empty() && crate::online::scraper_for(self.settings.scraper, &games[i]).is_some())
            .count();
        if let Some(panel) = &mut self.fetch {
            panel.wanted = count;
        }
    }

    fn start_fetch(&mut self, host: &mut dyn Host) {
        let jobs = self.fetch_jobs();
        let art: HashSet<String> = self.library.art_names().cloned().collect();
        let t = self.t();
        let Some(panel) = &mut self.fetch else { return };
        let actions = panel.flow.run(host, t, &mut self.listings, jobs, art);
        self.fetch_actions(host, actions);
    }

    pub(super) fn fetch_input(&mut self, host: &mut dyn Host, step: Option<Nav>, confirm: bool, cancel: bool) {
        let t = self.t();
        let Some(panel) = &mut self.fetch else { return };
        let flow = &mut panel.flow;
        match flow.step {
            fetch::Step::Setup => {
                let (mut dx, mut changed) = (0, false);
                match step {
                    Some(Nav::Press(Direction::Up)) | Some(Nav::Repeat(Direction::Up, _)) => flow.row = (flow.row + SETUP_ROWS - 1) % SETUP_ROWS,
                    Some(Nav::Press(Direction::Down)) | Some(Nav::Repeat(Direction::Down, _)) => flow.row = (flow.row + 1) % SETUP_ROWS,
                    Some(Nav::Press(Direction::Left)) => dx = -1,
                    Some(Nav::Press(Direction::Right)) => dx = 1,
                    _ => {}
                }
                if confirm {
                    if flow.row == START_ROW {
                        self.start_fetch(host);
                        return;
                    }
                    dx = 1;
                }
                if dx != 0 {
                    match flow.row {
                        0 => {
                            let count = panel.scopes.len() as i32;
                            flow.scope = (flow.scope as i32 + dx).rem_euclid(count) as usize;
                            changed = true;
                        }
                        SOURCE_ROW => {
                            self.settings.scraper = cycle_by(ScrapeSource::ALL, self.settings.scraper, dx);
                            changed = true;
                        }
                        MODE_ROW => {
                            flow.replace = !flow.replace;
                            changed = true;
                        }
                        _ => {}
                    }
                }
                if changed {
                    if let Some(panel) = &mut self.fetch {
                        panel.flow.status.clear();
                    }
                    self.count_wanted();
                    self.persist(host);
                } else if cancel {
                    self.fetch = None;
                }
            }
            fetch::Step::Key => {
                if confirm {
                    self.open_keyboard(super::keyboard::Target::ApiKey, "");
                } else if cancel {
                    flow.step = fetch::Step::Setup;
                }
            }
            fetch::Step::Running => {
                if cancel {
                    flow.stop(host, t);
                    self.save_fetched(host);
                }
            }
            fetch::Step::Done => {
                if confirm || cancel {
                    self.fetch = None;
                }
            }
        }
    }

    /// The keyboard's API key for Fetch artwork; false when the drawer is
    /// not open.
    pub(super) fn commit_fetch_key(&mut self, host: &mut dyn Host, text: &str) -> bool {
        let t = self.t();
        let Some(panel) = &mut self.fetch else { return false };
        if panel.flow.submit_key(host, t, text) {
            self.start_fetch(host);
        }
        true
    }

    pub(super) fn fetch_frame(&mut self, host: &mut dyn Host) {
        let t = self.t();
        let Some(panel) = &mut self.fetch else { return };
        let was_running = panel.flow.step == fetch::Step::Running;
        let actions = panel.flow.frame(host, t, &mut self.listings);
        let ended = was_running && panel.flow.step != fetch::Step::Running;
        self.fetch_actions(host, actions);
        if ended {
            self.save_fetched(host);
        }
    }

    /// A listing the host read, for a run waiting on it.
    pub(super) fn fetch_listing(&mut self, host: &mut dyn Host, folder: &str) {
        let t = self.t();
        let Some(panel) = &mut self.fetch else { return };
        let actions = panel.flow.listing_ready(host, t, &mut self.listings, folder);
        let ended = panel.flow.step != fetch::Step::Running;
        self.fetch_actions(host, actions);
        if ended {
            self.save_fetched(host);
        }
    }

    fn fetch_actions(&mut self, host: &mut dyn Host, actions: Vec<Action>) {
        if actions.is_empty() {
            return;
        }
        let applied = actions.iter().filter(|a| matches!(a, Action::Apply { .. })).count() as u32;
        for action in actions {
            match action {
                Action::AskKey => self.open_keyboard(super::keyboard::Target::ApiKey, ""),
                Action::Apply { id, kind, file } => match kind {
                    AssetKind::Icon => {
                        self.library.add_art(&file);
                        self.art.forget_missing();
                        self.library.patch(&id, |c| c.art = Some(file));
                    }
                    AssetKind::Backdrop => self.library.patch(&id, |c| c.backdrop = Some(file)),
                },
            }
        }
        // Every save renders each changed title's file and the cache, so a
        // run over thousands of titles writes every 20 pictures and at its
        // end.
        self.fetch_unsaved += applied;
        if self.fetch_unsaved >= SAVE_EVERY {
            self.save_fetched(host);
        }
        self.after_change_pub();
    }

    fn save_fetched(&mut self, host: &mut dyn Host) {
        if std::mem::take(&mut self.fetch_unsaved) > 0 && !self.library.overrides.save(host) {
            host.log("Title changes not saved");
        }
    }

    pub(super) fn draw_fetch(&mut self, gfx: &mut Gfx) {
        let Some(panel) = &self.fetch else { return };
        let flow = &panel.flow;
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.fetch_art, self.settings.language), panel::Size::Wide, slide);
        let confirm = Button::confirm(self.settings.confirm);
        let cancel = Button::cancel(self.settings.confirm);
        let scope = panel.scopes.get(flow.scope).map_or("", |s| s.1.as_str());
        match flow.step {
            fetch::Step::Setup => {
                let scope = gfx.fit(Role::BodyBold, scope, 150.0);
                drawer.row(gfx, t.scrape_scope, Some(&scope), None, flow.row == 0, Step::Both);
                drawer.row(gfx, t.scrape_source, Some(source_name(t, self.settings.scraper)), None, flow.row == SOURCE_ROW, Step::Both);
                drawer.row(gfx, t.scrape_mode, Some(if flow.replace { t.everything } else { t.only_missing }), None, flow.row == MODE_ROW, Step::Both);
                let count = fill(if flow.replace { t.scrape_count_all } else { t.scrape_count }, &[("count", &panel.wanted.to_string())]);
                drawer.row(gfx, t.start, Some(&count), None, flow.row == START_ROW, Step::Right);
                // What the choices do: per source, missing pictures or all.
                let help = match (self.settings.scraper, flow.replace) {
                    (ScrapeSource::Libretro, true) => t.scrape_help_libretro_all,
                    (ScrapeSource::Libretro, false) => t.scrape_help_libretro,
                    (ScrapeSource::Auto, true) => t.scrape_help_auto_all,
                    (ScrapeSource::Auto, false) => t.scrape_help_auto,
                    (ScrapeSource::SteamGridDb, true) => t.scrape_help_all,
                    (ScrapeSource::SteamGridDb, false) => t.scrape_help,
                };
                drawer.note(gfx, if flow.status.is_empty() { help } else { &flow.status }, 1);
                drawer.hints(gfx, &[(confirm, if flow.row == START_ROW { t.start } else { t.change }), (cancel, t.back)]);
            }
            fetch::Step::Key => {
                drawer.row(gfx, t.enter_key, None, None, true, Step::Right);
                drawer.space(4.0);
                drawer.text(gfx, t.key_help);
                drawer.text(gfx, "ux0:data/PocketShelf/steamgriddb.txt");
                if !flow.status.is_empty() {
                    drawer.note(gfx, &flow.status, 1);
                }
                drawer.hints(gfx, &[(confirm, t.r#type), (cancel, t.back)]);
            }
            fetch::Step::Running | fetch::Step::Done => {
                // The title being fetched for; once the run ends, the titles
                // it covered.
                if flow.current.is_empty() {
                    let scope = gfx.fit(Role::BodyBold, scope, 170.0);
                    drawer.row(gfx, t.scrape_scope, Some(&scope), None, true, Step::None);
                } else {
                    let title = gfx.fit(Role::BodyBold, &flow.current, 190.0);
                    drawer.row(gfx, t.title, Some(&title), None, true, Step::None);
                }
                drawer.row(gfx, t.progress, Some(&format!("{} / {}", flow.done, flow.total)), None, false, Step::None);
                drawer.row(gfx, t.icons, Some(&flow.found[0].to_string()), None, false, Step::None);
                drawer.row(gfx, t.backdrops, Some(&flow.found[1].to_string()), None, false, Step::None);
                drawer.row(gfx, t.not_found, Some(&flow.missing.to_string()), None, false, Step::None);
                if !flow.status.is_empty() {
                    drawer.note(gfx, &flow.status, 1);
                }
                let running = flow.step == fetch::Step::Running;
                drawer.hints(gfx, &[(cancel, if running { t.stop } else { t.close })]);
            }
        }
    }

    // --- Clean up artwork --------------------------------------------------

    pub(super) fn open_cleanup(&mut self, host: &mut dyn Host) {
        let art = self.library.reload_art(host);
        let backdrops = list_png(host, "backdrops");
        self.cleanup = Some(Cleanup { row: 0, every: false, armed: false, note: String::new(), art, backdrops, targets: (Vec::new(), Vec::new()) });
        self.clean_targets();
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    /// The files the chosen mode deletes. In use: a file a title's changes
    /// name, or an icon matched to a title by its name.
    fn clean_targets(&mut self) {
        let Some(clean) = &self.cleanup else { return };
        let targets = if clean.every {
            (clean.art.clone(), clean.backdrops.clone())
        } else {
            let mut art = files_in_use(&self.library.overrides.titles, false);
            art.extend(self.library.games().iter().filter_map(|g| g.art.clone()));
            let backdrops = files_in_use(&self.library.overrides.titles, true);
            (
                clean.art.iter().filter(|f| !art.contains(*f)).cloned().collect(),
                clean.backdrops.iter().filter(|f| !backdrops.contains(*f)).cloned().collect(),
            )
        };
        if let Some(clean) = &mut self.cleanup {
            clean.targets = targets;
        }
    }

    pub(super) fn cleanup_input(&mut self, host: &mut dyn Host, step: Option<Nav>, confirm: bool, cancel: bool) {
        let t = self.t();
        let Some(clean) = &mut self.cleanup else { return };
        let mut toggle = false;
        match step {
            Some(Nav::Press(Direction::Up | Direction::Down)) | Some(Nav::Repeat(Direction::Up | Direction::Down, _)) => {
                clean.row = 1 - clean.row;
                clean.armed = false;
                clean.note.clear();
            }
            Some(Nav::Press(Direction::Left | Direction::Right)) if clean.row == 0 => toggle = true,
            _ => {}
        }
        if cancel {
            self.cleanup = None;
            return;
        }
        if confirm && clean.row == 0 {
            toggle = true;
        }
        if toggle {
            clean.every = !clean.every;
            clean.armed = false;
            clean.note.clear();
            self.clean_targets();
            return;
        }
        if !confirm {
            return;
        }
        // On Delete, press twice: the first press asks, the second deletes.
        let count = clean.targets.0.len() + clean.targets.1.len();
        if count == 0 {
            clean.note = t.clean_nothing.to_string();
        } else if !clean.armed {
            clean.armed = true;
        } else {
            let deleted = self.delete_clean_targets(host);
            if let Some(clean) = &mut self.cleanup {
                clean.armed = false;
                clean.note = fill(t.clean_done, &[("count", &deleted.to_string())]);
            }
        }
    }

    fn delete_clean_targets(&mut self, host: &mut dyn Host) -> usize {
        let Some(clean) = &mut self.cleanup else { return 0 };
        let (art, backdrops) = std::mem::take(&mut clean.targets);
        let every = clean.every;
        let mut deleted = 0;
        let mut gone = HashSet::new();
        for (folder, files) in [("art", &art), ("backdrops", &backdrops)] {
            for file in files {
                // A file the host refuses stays, and is counted again.
                if host.remove(&format!("{folder}/{file}")) {
                    deleted += 1;
                    gone.insert(format!("{folder}/{file}"));
                }
            }
        }
        clean.art.retain(|f| !gone.contains(&format!("art/{f}")));
        clean.backdrops.retain(|f| !gone.contains(&format!("backdrops/{f}")));
        let left = clean.art.clone();
        // With every file gone, the titles that named one go back to their
        // own pictures.
        if every {
            let titles = without_files(&self.library.overrides.titles);
            self.library.set_overrides(titles);
            if !self.library.overrides.save(host) {
                host.log("Title changes not saved");
            }
        }
        self.library.set_art_files(left);
        self.after_change_pub();
        self.clean_targets();
        deleted
    }

    pub(super) fn draw_cleanup(&mut self, gfx: &mut Gfx) {
        let Some(clean) = &self.cleanup else { return };
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.clean_art, self.settings.language), panel::Size::Wide, slide);
        let count = clean.targets.0.len() + clean.targets.1.len();
        drawer.row(gfx, t.clean_mode, Some(if clean.every { t.everything } else { t.unused }), None, clean.row == 0, Step::Both);
        let files = fill(t.clean_count, &[("count", &count.to_string())]);
        drawer.row(gfx, t.remove, Some(&files), None, clean.row == 1, Step::Right);
        // The last press's result, the question before a delete, or what the
        // mode does.
        let note = if !clean.note.is_empty() {
            clean.note.clone()
        } else if clean.armed {
            fill(t.clean_confirm, &[("count", &count.to_string())])
        } else if clean.every {
            t.clean_help_all.to_string()
        } else {
            t.clean_help_unused.to_string()
        };
        drawer.note(gfx, &note, 1);
        let confirm = Button::confirm(self.settings.confirm);
        drawer.hints(gfx, &[(confirm, if clean.row == 0 { t.change } else { t.remove }), (Button::cancel(self.settings.confirm), t.back)]);
    }
}
