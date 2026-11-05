//! SELECT ▸ Library ▸ Game database, one console at a time: libretro's
//! database for it is downloaded when its table is missing or a week old,
//! and the host identifies its titles ([`Host::identify`]); then the
//! console's Wikidata table is downloaded when the release's `index.tsv`
//! (read once at the start) lists a newer build than the card's, and the
//! host matches the titles to its games ([`Host::match_wikidata`]). Vita
//! titles, which libretro does not list, have the Wikidata step only, last.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use super::chrome::Button;
use super::panel::{self, Drawer, Step};
use super::Shelf;
use crate::gfx::Gfx;
use crate::host::{Host, NetId, NetKind, NetState};
use crate::i18n::{fill, upper};
use crate::online::{gamedb, wikidata};
use crate::text::Role;

/// The sources of the game details: libretro's databases' license (CC BY-SA
/// 4.0) asks for their source to be named; Wikidata's data is CC0.
const CREDIT: &str = "libretro-database, CC BY-SA 4.0 · Wikidata, CC0";

/// The Game database drawer asks for tables at most this old.
pub const UPDATE_AGE_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// The console name shown for Vita titles' table.
const VITA: &str = "Sony - PlayStation Vita";

/// One step of a run.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Task {
    /// Identify a libretro database's titles.
    Identify(String),
    /// Match a Wikidata table's titles.
    Match(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Wait {
    Idle,
    Identify { database: String, downloaded: bool },
    Fetch(NetId, String),
    /// The release's `index.tsv`.
    Index(NetId),
    Match { file: String, downloaded: bool },
    /// `labels.tsv`, then the table `file`.
    Labels(NetId, String),
    Table(NetId, String),
}

/// The drawer's run.
pub struct DatabaseRun {
    pub running: bool,
    pub status: String,
    /// The console being worked on.
    pub current: String,
    /// The steps left: each with the console it shows and whether it is the
    /// console's last.
    queue: VecDeque<(Task, String, bool)>,
    /// Each table's build time on the release; empty when `index.tsv` did
    /// not download (the tables on the card are used).
    index: HashMap<String, u64>,
    /// The labels were downloaded this run.
    labels: bool,
    /// Whether the step running is its console's last.
    last: bool,
    pub total: usize,
    pub done: usize,
    /// Titles identified this run.
    pub named: usize,
    /// The console being identified: games checked of its total, and found.
    pub checked: (u32, u32),
    pub found: u32,
    /// The library's titles identified and those a database covers, and the
    /// consoles, worked out when the drawer opens and after each console:
    /// counting them walked every title, every frame.
    known: (usize, usize),
    consoles: usize,
    wait: Wait,
}

/// A run's steps, by the order of each console's first title: each
/// libretro database, then its Wikidata table when there is one; Vita
/// titles' table last. Several databases sharing a table (PSP and its PSN
/// titles) match it after each, the last with every title identified.
fn tasks(shelf: &Shelf) -> Vec<(Task, String, bool)> {
    use crate::library::categories::Platform;
    let mut databases: Vec<String> = Vec::new();
    let mut vita = false;
    for game in shelf.library.base() {
        if let Some(database) = gamedb::database_of(game) {
            if !databases.contains(&database) {
                databases.push(database);
            }
        } else if game.platform == Platform::Vita && wikidata::vita_game(&game.id) {
            vita = true;
        }
    }
    let mut tasks = Vec::new();
    for database in databases {
        let file = wikidata::file_of_database(&database);
        tasks.push((Task::Identify(database.clone()), database.clone(), file.is_none()));
        if let Some(file) = file {
            tasks.push((Task::Match(file), database, true));
        }
    }
    if vita {
        tasks.push((Task::Match("vita".into()), VITA.into(), true));
    }
    tasks
}

impl Shelf {
    pub(super) fn open_database(&mut self) {
        let consoles = tasks(self).iter().filter(|(_, _, last)| *last).count();
        let known = self.database_counts();
        self.database = Some(DatabaseRun {
            running: false,
            status: String::new(),
            current: String::new(),
            queue: VecDeque::new(),
            index: HashMap::new(),
            labels: false,
            last: false,
            total: 0,
            done: 0,
            named: 0,
            checked: (0, 0),
            found: 0,
            known,
            consoles,
            wait: Wait::Idle,
        });
        self.drawer = panel::slide();
        self.drawer.set(1.0, self.now);
    }

    /// Confirm starts; back closes, or stops a run (what was found stays).
    pub(super) fn database_input(&mut self, host: &mut dyn Host, confirm: bool, cancel: bool) {
        let t = self.t();
        // This runs every frame the drawer is open: walking the titles here
        // took 15 ms of each frame on the Vita (6,011 titles).
        let starting = confirm && self.database.as_ref().is_some_and(|r| !r.running);
        let tasks = if starting { tasks(self) } else { Vec::new() };
        let Some(run) = &mut self.database else { return };
        if run.running {
            if cancel {
                match std::mem::replace(&mut run.wait, Wait::Idle) {
                    Wait::Fetch(id, _) | Wait::Index(id) | Wait::Labels(id, _) | Wait::Table(id, _) => host.net_close(id),
                    Wait::Identify { .. } => host.identify_stop(),
                    // Matching takes a moment; its answer still reaches the
                    // library.
                    Wait::Match { .. } | Wait::Idle => {}
                }
                run.running = false;
                run.queue.clear();
                run.current.clear();
                run.status = t.scrape_stopped.to_string();
            }
            return;
        }
        if confirm {
            if tasks.is_empty() {
                run.status = t.no_database_titles.to_string();
                return;
            }
            run.running = true;
            run.total = tasks.iter().filter(|(_, _, last)| *last).count();
            run.done = 0;
            run.named = 0;
            run.labels = false;
            run.index.clear();
            run.status = t.getting_details.to_string();
            let wiki = tasks.iter().any(|(task, _, _)| matches!(task, Task::Match(_)));
            run.queue = tasks.into();
            // The release's build times, before the first table.
            match wiki.then(|| host.net_start(&wikidata::index_url(), NetKind::Text { authorization: String::new() })).flatten() {
                Some(id) => run.wait = Wait::Index(id),
                None => self.next_task(host),
            }
        } else if cancel {
            self.database = None;
        }
    }

    /// A step finished: count its console when it was the last, and go on.
    fn task_done(&mut self, host: &mut dyn Host) {
        let Some(run) = &mut self.database else { return };
        run.wait = Wait::Idle;
        if run.last {
            run.done += 1;
        }
        self.next_task(host);
    }

    /// The next step, or the end of the run.
    fn next_task(&mut self, host: &mut dyn Host) {
        let t = self.t();
        let newer = host.unix_ms().saturating_sub(UPDATE_AGE_MS);
        let Some(run) = &mut self.database else { return };
        let Some((task, shown, last)) = run.queue.pop_front() else {
            run.running = false;
            run.wait = Wait::Idle;
            run.current.clear();
            run.status = fill(t.database_done, &[("count", &run.named.to_string())]);
            host.log(&format!("game database: {} titles identified over {} consoles", run.named, run.total));
            return;
        };
        run.current = shown;
        run.last = last;
        run.status = t.identifying_start.to_string();
        run.checked = (0, 0);
        run.found = 0;
        let started = match task {
            Task::Identify(database) => host.identify(&database, newer).then_some(Wait::Identify { database, downloaded: false }),
            Task::Match(file) => {
                let newer = run.index.get(&file).copied().unwrap_or(0);
                host.match_wikidata(&file, newer).then_some(Wait::Match { file, downloaded: false })
            }
        };
        match started {
            Some(wait) => run.wait = wait,
            None => self.task_done(host),
        }
    }

    /// The host's answer for a database the run waits on.
    pub(super) fn database_identified(&mut self, host: &mut dyn Host, database: &str, found: Option<usize>) {
        let t = self.t();
        // The library took the console's results already (`identified`).
        let known = self.database_counts();
        let Some(run) = &mut self.database else { return };
        run.known = known;
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
        self.task_done(host);
    }

    /// Titles matched to Wikidata's games: the library takes them, and a run
    /// waiting on the table goes on (downloading it first when the card's is
    /// missing or older than the release's). `named` counts the titles
    /// libretro's database did not identify.
    pub fn wikidata_matched(&mut self, host: &mut dyn Host, file: &str, found: Option<Vec<(String, Arc<wikidata::Found>)>>) {
        let mut named = None;
        if let Some(found) = &found {
            let started = host.time_us();
            named = Some(found.iter().filter(|(id, _)| self.library.base_game(id).is_some_and(|g| g.details.is_none())).count());
            let keep = self.selected_game().map(|g| g.id.to_string());
            self.library.set_wiki(found);
            self.relist(keep);
            host.log(&format!("wikidata {file}: {} titles matched, library in {} us", found.len(), host.time_us() - started));
        }
        let t = self.t();
        let known = self.database_counts();
        let Some(run) = &mut self.database else { return };
        run.known = known;
        let Wait::Match { file: waiting, downloaded } = run.wait.clone() else { return };
        if waiting != file {
            return;
        }
        run.wait = Wait::Idle;
        if named.is_none() && !downloaded {
            run.status = t.getting_details.to_string();
            let started = if run.labels {
                host.net_start(&wikidata::table_url(file), NetKind::Save(wikidata::table_path(file))).map(|id| Wait::Table(id, file.to_string()))
            } else {
                host.net_start(&wikidata::labels_url(), NetKind::Save(wikidata::LABELS_PATH.to_string())).map(|id| Wait::Labels(id, file.to_string()))
            };
            match started {
                Some(wait) => run.wait = wait,
                None => {
                    run.running = false;
                    run.queue.clear();
                    run.status = t.no_network.to_string();
                }
            }
            return;
        }
        // A table shared by several databases counts each title once: at
        // its last match.
        if !run.queue.iter().any(|(task, _, _)| *task == Task::Match(file.to_string())) {
            run.named += named.unwrap_or(0);
        }
        self.task_done(host);
    }

    /// The host's progress identifying a database's titles.
    pub(super) fn database_progress(&mut self, database: &str, done: u32, total: u32, found: u32) {
        let Some(run) = &mut self.database else { return };
        if matches!(&run.wait, Wait::Identify { database: waiting, .. } if waiting == database) {
            run.checked = (done, total);
            run.found = found;
        }
    }

    /// A download in flight: its progress, then the next step. A failed
    /// database download leaves an older table usable; with none, the
    /// database is skipped. So is a Wikidata table.
    pub(super) fn database_frame(&mut self, host: &mut dyn Host) {
        let t = self.t();
        let Some(run) = &mut self.database else { return };
        let (id, details) = match &run.wait {
            Wait::Fetch(id, _) => (*id, false),
            Wait::Index(id) | Wait::Labels(id, _) | Wait::Table(id, _) => (*id, true),
            _ => return,
        };
        let state = host.net_state(id);
        if let NetState::Busy { received, total } = state {
            if total > 0 {
                let text = if details { t.getting_details_percent } else { t.getting_database_percent };
                run.status = fill(text, &[("percent", &(received * 100 / total).to_string())]);
            }
            return;
        }
        let text = matches!(state, NetState::Done { status: 200, .. }).then(|| host.net_text(id));
        host.net_close(id);
        let wait = std::mem::replace(&mut run.wait, Wait::Idle);
        if let NetState::Error(error) = &state {
            host.log(&format!("game database {wait:?}: {error}"));
        }
        match wait {
            Wait::Fetch(_, database) => {
                run.status = t.identifying_start.to_string();
                if host.identify(&database, 0) {
                    run.wait = Wait::Identify { database, downloaded: true };
                } else {
                    self.task_done(host);
                }
            }
            Wait::Index(_) => {
                run.index = text.map(|text| wikidata::parse_index(&text)).unwrap_or_default();
                host.log(&format!("wikidata: index of {} tables", run.index.len()));
                self.next_task(host);
            }
            Wait::Labels(_, file) => {
                // Tried once a run: the tables match without new labels.
                run.labels = true;
                match host.net_start(&wikidata::table_url(&file), NetKind::Save(wikidata::table_path(&file))) {
                    Some(id) => run.wait = Wait::Table(id, file),
                    None => self.task_done(host),
                }
            }
            Wait::Table(_, file) => {
                run.status = t.identifying_start.to_string();
                if host.match_wikidata(&file, 0) {
                    run.wait = Wait::Match { file, downloaded: true };
                } else {
                    self.task_done(host);
                }
            }
            _ => {}
        }
    }

    /// Titles with a database or a Wikidata table, and those identified by
    /// either.
    fn database_counts(&self) -> (usize, usize) {
        use crate::library::categories::Platform;
        let base = self.library.base();
        let with = base.iter().filter(|g| (g.platform == Platform::Vita && wikidata::vita_game(&g.id)) || gamedb::database_of(g).is_some());
        let total = with.clone().count();
        let known = with.filter(|g| g.details.is_some() || g.wiki.is_some()).count();
        (known, total)
    }

    pub(super) fn draw_database(&mut self, gfx: &mut Gfx) {
        let Some(run) = &self.database else { return };
        let (running, status, current, done, total) = (run.running, run.status.clone(), run.current.clone(), run.done, run.total);
        let ((known, titles), consoles, checked, found) = (run.known, run.consoles, run.checked, run.found);
        let t = self.t();
        let theme = self.theme;
        let slide = self.drawer.at(self.now);
        let mut drawer = Drawer::open(gfx, &theme, &upper(t.game_database, self.settings.language), panel::Size::Wide, slide);
        let of = |count: usize, total: usize| fill(t.identified_count, &[("count", &count.to_string()), ("total", &total.to_string())]);
        let identified = of(known, titles);
        if running {
            let name = gfx.fit(Role::BodyBold, &current, 420.0);
            drawer.row(gfx, t.database_current, Some(&name), None, true, Step::None);
            // The console's games as the host reads them, then the library's
            // count once the console is done.
            let games = if checked.1 > 0 { of(checked.0 as usize, checked.1 as usize) } else { "-".to_string() };
            drawer.row(gfx, t.database_games, Some(&games), None, false, Step::None);
            drawer.row(gfx, t.database_found, Some(&found.to_string()), None, false, Step::None);
            drawer.row(gfx, t.database_consoles, Some(&of(done, total)), None, false, Step::None);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{Event, PictureKey, PluginState, Status, TitleInfo};
    use crate::library::catalog::load_catalog;
    use crate::library::settings::{Settings, UpdateChannel};
    use crate::library::Library;

    /// Records what the run asks for; every download finishes at the first
    /// look.
    #[derive(Default)]
    struct RunHost {
        calls: Vec<String>,
        next: i32,
    }

    impl Host for RunHost {
        fn time_us(&self) -> u64 {
            0
        }
        fn read(&mut self, _: &str) -> Option<Vec<u8>> {
            None
        }
        fn write(&mut self, _: &str, _: &[u8]) -> bool {
            true
        }
        fn remove(&mut self, _: &str) -> bool {
            true
        }
        fn list(&mut self, _: &str) -> Vec<String> {
            Vec::new()
        }
        fn status(&mut self) -> Status {
            Status::default()
        }
        fn scan(&mut self, _: bool) {}
        fn request(&mut self, _: PictureKey) {}
        fn poll(&mut self, _: &mut Vec<Event>) {}
        fn log(&mut self, _: &str) {}
        fn launch(&mut self, _: &str, _: Option<crate::host::Emulator>) -> bool {
            false
        }
        fn plugin_state(&mut self) -> PluginState {
            PluginState::On
        }
        fn enable_plugin(&mut self) -> bool {
            true
        }
        fn net_start(&mut self, url: &str, kind: NetKind) -> Option<NetId> {
            let file = url.rsplit('/').next().unwrap_or("");
            self.calls.push(match kind {
                NetKind::Save(path) => format!("get {file} to {path}"),
                _ => format!("get {file}"),
            });
            self.next += 1;
            Some(NetId(self.next))
        }
        fn net_state(&mut self, _: NetId) -> NetState {
            NetState::Done { status: 200, received: 10 }
        }
        fn net_text(&mut self, _: NetId) -> String {
            "psp\t2\t50\nvita\t1\t60\n".into()
        }
        fn identify(&mut self, database: &str, _: u64) -> bool {
            self.calls.push(format!("identify {database}"));
            true
        }
        fn match_wikidata(&mut self, file: &str, newer_than_ms: u64) -> bool {
            self.calls.push(format!("match {file} {newer_than_ms}"));
            true
        }
    }

    fn found(item: &str) -> Arc<wikidata::Found> {
        Arc::new(wikidata::Found { game: wikidata::Game { item: item.into(), name: item.into(), year: 2005, ..Default::default() }, genres: Vec::new() })
    }

    #[test]
    fn a_run_identifies_then_matches_each_console_and_the_vita_last() {
        let mut host = RunHost::default();
        let library = Library::load(&mut host);
        let mut shelf = Shelf::new(Settings::defaults(UpdateChannel::Off), library, Status::default());
        let info = |id: &str, platform: &str| TitleInfo { id: id.into(), title: format!("Title {id}"), platform: platform.into(), ..Default::default() };
        let infos = [info("PCSA00069", "vita"), info("VITASHELL", "vita"), info("ULUS10041", "psp"), info("NPUZ00001", "psp")];
        shelf.set_titles(load_catalog(&infos), true);
        shelf.open_database();
        assert_eq!(shelf.database.as_ref().map(|r| (r.consoles, r.known)), Some((3, (0, 3))), "PSP, its PSN titles and the Vita; not VitaShell");

        shelf.database_input(&mut host, true, false);
        assert_eq!(host.calls, ["get index.tsv"]);
        shelf.database_frame(&mut host);
        assert_eq!(host.calls[1..], ["identify Sony - PlayStation Portable"]);
        shelf.identified(&mut host, gamedb::PSP, Some(Vec::new()));
        assert_eq!(host.calls[2..], ["match psp 50"], "the release's build time");
        // No table on the card: the labels, then the table, then again.
        shelf.wikidata_matched(&mut host, "psp", None);
        shelf.database_frame(&mut host);
        shelf.database_frame(&mut host);
        assert_eq!(host.calls[3..], ["get labels.tsv to wikidata/labels.tsv", "get psp.tsv to wikidata/psp.tsv", "match psp 0"]);
        shelf.wikidata_matched(&mut host, "psp", Some(vec![("ULUS10041".into(), found("Q1"))]));
        assert_eq!(shelf.database.as_ref().map(|r| (r.done, r.named)), Some((1, 0)), "counted at the table's last match");
        assert_eq!(host.calls[6..], ["identify Sony - PlayStation Portable (PSN)"]);
        shelf.identified(&mut host, gamedb::PSP_PSN, Some(Vec::new()));
        shelf.wikidata_matched(&mut host, "psp", Some(vec![("ULUS10041".into(), found("Q1")), ("NPUZ00001".into(), found("Q2"))]));
        // The labels came this run: only the Vita's table is downloaded.
        shelf.wikidata_matched(&mut host, "vita", None);
        shelf.database_frame(&mut host);
        assert_eq!(host.calls[7..], ["match psp 50", "match vita 60", "get vita.tsv to wikidata/vita.tsv", "match vita 0"]);
        shelf.wikidata_matched(&mut host, "vita", Some(vec![("PCSA00069".into(), found("Q3"))]));
        let run = shelf.database.as_ref().unwrap();
        assert_eq!((run.running, run.done, run.total, run.named, run.known), (false, 3, 3, 3, (3, 3)));
        assert_eq!(shelf.library.base_game("PCSA00069").and_then(|g| g.wiki.as_ref()).map(|w| w.game.item.as_str()), Some("Q3"));
        assert_eq!(shelf.library.base_game("PCSA00069").map(|g| g.year()), Some(2005));
    }
}
