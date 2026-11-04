//! SELECT ▸ Library ▸ Game database: each database the library's titles
//! belong to (RetroArch consoles, PSP, PS1) is downloaded when its table is
//! missing or a week old, then the host identifies its titles
//! ([`Host::identify`]), one database at a time.

use std::collections::VecDeque;

use super::chrome::Button;
use super::panel::{self, Drawer, Step};
use super::Shelf;
use crate::gfx::Gfx;
use crate::host::{Host, NetId, NetKind, NetState};
use crate::i18n::{fill, upper};
use crate::online::gamedb;
use crate::text::Role;

/// The source of the game databases, as their license (CC BY-SA 4.0) asks.
const CREDIT: &str = "libretro-database, CC BY-SA 4.0";

/// The Game database drawer asks for tables at most this old.
pub const UPDATE_AGE_MS: u64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Clone, PartialEq, Eq, Debug)]
enum Wait {
    Idle,
    Identify { database: String, downloaded: bool },
    Fetch(NetId, String),
}

/// The drawer's run.
pub struct DatabaseRun {
    pub running: bool,
    pub status: String,
    /// The database being worked on.
    pub current: String,
    queue: VecDeque<String>,
    pub total: usize,
    pub done: usize,
    /// Titles identified this run.
    pub named: usize,
    wait: Wait,
}

/// The databases the library's titles belong to, in the order of their
/// first title.
fn databases(shelf: &Shelf) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for game in shelf.library.base() {
        if let Some(database) = gamedb::database_of(game) {
            if !found.contains(&database) {
                found.push(database);
            }
        }
    }
    found
}

impl Shelf {
    pub(super) fn open_database(&mut self) {
        self.database = Some(DatabaseRun {
            running: false,
            status: String::new(),
            current: String::new(),
            queue: VecDeque::new(),
            total: 0,
            done: 0,
            named: 0,
            wait: Wait::Idle,
        });
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    /// Confirm starts; back closes, or stops a run (what was found stays).
    pub(super) fn database_input(&mut self, host: &mut dyn Host, confirm: bool, cancel: bool) {
        let t = self.t();
        let list = databases(self);
        let Some(run) = &mut self.database else { return };
        if run.running {
            if cancel {
                match std::mem::replace(&mut run.wait, Wait::Idle) {
                    Wait::Fetch(id, _) => host.net_close(id),
                    Wait::Identify { .. } => host.identify_stop(),
                    Wait::Idle => {}
                }
                run.running = false;
                run.queue.clear();
                run.current.clear();
                run.status = t.scrape_stopped.to_string();
            }
            return;
        }
        if confirm {
            if list.is_empty() {
                run.status = t.no_database_titles.to_string();
                return;
            }
            run.running = true;
            run.total = list.len();
            run.done = 0;
            run.named = 0;
            run.status.clear();
            run.queue = list.into();
            self.next_database(host);
        } else if cancel {
            self.database = None;
        }
    }

    /// The next database, or the end of the run.
    fn next_database(&mut self, host: &mut dyn Host) {
        let t = self.t();
        let newer = host.unix_ms().saturating_sub(UPDATE_AGE_MS);
        let Some(run) = &mut self.database else { return };
        let Some(database) = run.queue.pop_front() else {
            run.running = false;
            run.current.clear();
            run.status = fill(t.database_done, &[("count", &run.named.to_string())]);
            host.log(&format!("game database: {} titles identified over {} databases", run.named, run.total));
            return;
        };
        run.current = database.clone();
        run.status = t.identifying_start.to_string();
        if host.identify(&database, newer) {
            run.wait = Wait::Identify { database, downloaded: false };
        } else {
            run.done += 1;
            self.next_database(host);
        }
    }

    /// The host's answer for a database the run waits on.
    pub(super) fn database_identified(&mut self, host: &mut dyn Host, database: &str, found: Option<usize>) {
        let t = self.t();
        let Some(run) = &mut self.database else { return };
        let Wait::Identify { database: waiting, downloaded } = run.wait.clone() else { return };
        if waiting != database {
            return;
        }
        run.wait = Wait::Idle;
        if found.is_none() && !downloaded {
            run.status = t.getting_database.to_string();
            match host.net_start(&gamedb::rdb_url(database), NetKind::Save(gamedb::table_path(database))) {
                Some(id) => run.wait = Wait::Fetch(id, database.to_string()),
                None => {
                    run.running = false;
                    run.queue.clear();
                    run.status = t.no_network.to_string();
                }
            }
            return;
        }
        run.named += found.unwrap_or(0);
        run.done += 1;
        self.next_database(host);
    }

    /// The host's progress identifying a database's titles.
    pub(super) fn database_progress(&mut self, database: &str, done: u32, total: u32) {
        let t = self.t();
        let Some(run) = &mut self.database else { return };
        if matches!(&run.wait, Wait::Identify { database: waiting, .. } if waiting == database) {
            run.status = fill(t.identifying, &[("done", &done.to_string()), ("total", &total.to_string())]);
        }
    }

    /// A download in flight: its progress, then the host is asked again
    /// with whatever table it has (a failed download leaves an older one
    /// usable; with none, the database is skipped).
    pub(super) fn database_frame(&mut self, host: &mut dyn Host) {
        let t = self.t();
        let Some(run) = &mut self.database else { return };
        let Wait::Fetch(id, database) = run.wait.clone() else { return };
        match host.net_state(id) {
            NetState::Busy { received, total } => {
                if total > 0 {
                    run.status = fill(t.getting_database_percent, &[("percent", &(received * 100 / total).to_string())]);
                }
            }
            state => {
                host.net_close(id);
                if let NetState::Error(error) = &state {
                    host.log(&format!("game database {database}: {error}"));
                }
                run.status = t.identifying_start.to_string();
                if host.identify(&database, 0) {
                    run.wait = Wait::Identify { database, downloaded: true };
                } else {
                    run.wait = Wait::Idle;
                    run.done += 1;
                    self.next_database(host);
                }
            }
        }
    }

    /// Titles with a database, and those identified.
    fn database_counts(&self) -> (usize, usize) {
        let base = self.library.base();
        let with = base.iter().filter(|g| gamedb::database_of(g).is_some());
        let total = with.clone().count();
        let known = with.filter(|g| g.details.is_some()).count();
        (known, total)
    }

    pub(super) fn draw_database(&mut self, gfx: &mut Gfx) {
        let Some(run) = &self.database else { return };
        let (running, status, current, done, total) = (run.running, run.status.clone(), run.current.clone(), run.done, run.total);
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let (known, titles) = self.database_counts();
        let consoles = databases(self).len();
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.game_database, self.settings.language), panel::Size::Wide, slide);
        let identified = fill(t.identified_count, &[("count", &known.to_string()), ("total", &titles.to_string())]);
        if running {
            let name = gfx.fit(Role::BodyBold, &current, 420.0);
            drawer.row(gfx, t.database_current, Some(&name), None, true, Step::None);
            drawer.row(gfx, t.progress, Some(&format!("{done} / {total}")), None, false, Step::None);
            drawer.row(gfx, t.database_identified, Some(&identified), None, false, Step::None);
            drawer.note(gfx, &status, 1);
            drawer.hints(gfx, &[(Button::cancel(self.settings.confirm), t.stop)]);
        } else {
            drawer.row(gfx, t.database_consoles, Some(&consoles.to_string()), None, true, Step::Right);
            drawer.row(gfx, t.database_identified, Some(&identified), None, false, Step::None);
            let text = if status.is_empty() { t.database_help.to_string() } else { status };
            drawer.note(gfx, &text, 2);
            // The databases' license asks for their source to be named.
            drawer.note(gfx, CREDIT, 1);
            drawer.hints(gfx, &[(Button::confirm(self.settings.confirm), t.update_database), (Button::cancel(self.settings.confirm), t.back)]);
        }
    }
}
