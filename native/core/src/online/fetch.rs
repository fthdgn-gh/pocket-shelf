//! Fetch artwork for many titles (`src/scrape.ts`, SELECT menu -> Fetch
//! artwork). Each title goes to the scraper its source gives it
//! ([`scraper_for`]). On SteamGridDB it is searched by its name, and the
//! first game found gives its first icon and first backdrop. On libretro's
//! server its name is matched in its console's box art list
//! ([`match_name`]): the box art (else the title screen) becomes the icon,
//! the screenshot (else the title screen) the backdrop. One request runs at a
//! time, and files go straight to the card.

use std::collections::{HashSet, VecDeque};

use super::flow::{Listings, Online};
use super::libretro::{self, match_name};
use super::steamgriddb::{self as sgdb, AssetKind, Problem};
use super::{candidate_path, libretro_candidates, scraper_for, Candidate, Scraper};
use crate::host::{Host, NetId, NetKind, NetState};
use crate::i18n::fill;
use crate::library::catalog::Game;
use crate::library::list_png;
use crate::library::settings::ScrapeSource;
use crate::locales::Messages;

/// Frames to wait before asking again after SteamGridDB said it is busy
/// (5 s at 60 per second).
pub const BUSY_WAIT: u32 = 300;
/// Busy replies in a row waited out before the run halts.
pub const BUSY_RETRIES: u32 = 5;

/// Rows of the setup page: the titles, the source, missing pictures or all
/// of them, then Start.
pub const SETUP_ROWS: usize = 4;
pub const SOURCE_ROW: usize = 1;
pub const MODE_ROW: usize = 2;
pub const START_ROW: usize = 3;

/// Which pictures a title has no file of its own for. An icon the user
/// chose, one matched by file name, and the title's own icon chosen over a
/// file are kept; so are a chosen backdrop and the choice of none.
pub fn missing_art(game: &Game) -> Vec<AssetKind> {
    let mut kinds = Vec::new();
    if game.art.is_none() && !game.art_icon {
        kinds.push(AssetKind::Icon);
    }
    if game.backdrop.is_none() {
        kinds.push(AssetKind::Backdrop);
    }
    kinds
}

/// The pictures to fetch for a title: the missing ones, or both when
/// replacing.
pub fn kinds_for(game: &Game, replace: bool) -> Vec<AssetKind> {
    if replace {
        vec![AssetKind::Icon, AssetKind::Backdrop]
    } else {
        missing_art(game)
    }
}

/// A title the run fetches for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Job {
    pub id: String,
    pub title: String,
    /// The name it came with (a RetroArch game's playlist label), which
    /// libretro matches first.
    pub label: String,
    pub scraper: Scraper,
    /// Its console's folder on libretro's server.
    pub folder: Option<String>,
    pub kinds: Vec<AssetKind>,
}

impl Job {
    /// The job for a title under a source, or `None` when it needs nothing
    /// or the source has no scraper for it.
    pub fn for_game(game: &Game, label: &str, source: ScrapeSource, replace: bool) -> Option<Job> {
        let kinds = kinds_for(game, replace);
        if kinds.is_empty() {
            return None;
        }
        let scraper = scraper_for(source, game)?;
        let folder = (scraper == Scraper::Libretro).then(|| libretro::libretro_folder(game.console.as_deref(), game.platform)).flatten();
        Some(Job { id: game.id.to_string(), title: game.title.to_string(), label: label.to_string(), scraper, folder, kinds })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Setup,
    /// No usable SteamGridDB key, and some title goes there.
    Key,
    Running,
    Done,
}

/// What the shelf has to do for the run.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Action {
    AskKey,
    /// Use a file now on the card for a title.
    Apply { id: String, kind: AssetKind, file: String },
}

/// The request in flight, or what the run waits for.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Wait {
    Idle,
    /// The host reading a console's listing; `fetched` once it came from
    /// the server this run.
    Listing { folder: String, fetched: bool },
    ListingFetch(NetId, String),
    Search(NetId),
    Assets(NetId, u64, AssetKind),
    /// A SteamGridDB file being saved, for the game found.
    SgdbFile(NetId, u64, AssetKind, String),
    /// A libretro candidate being saved; the others of its kind follow on a
    /// 404.
    LibretroFile(NetId, AssetKind, Candidate, Vec<Candidate>),
}

/// Asked again after a busy reply.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Retry {
    Search,
    Assets(u64),
}

pub struct Fetch {
    pub step: Step,
    pub row: usize,
    pub scope: usize,
    pub replace: bool,
    pub status: String,
    /// The title being fetched for.
    pub current: String,
    pub total: usize,
    pub done: usize,
    /// Icons and backdrops found.
    pub found: [usize; 2],
    pub missing: usize,
    queue: VecDeque<Job>,
    job: Option<Job>,
    /// The current job's pictures still to fetch: kinds (SteamGridDB) or
    /// each kind's candidates (libretro).
    kinds: VecDeque<AssetKind>,
    candidates: VecDeque<(AssetKind, Vec<Candidate>)>,
    wait: Wait,
    wait_frames: u32,
    busy_replies: u32,
    retry: Option<Retry>,
    /// Consoles libretro's server has no folder for, found this run.
    unknown: HashSet<String>,
    /// Files in `art/` and `backdrops/`; these are not downloaded again.
    on_disk: [HashSet<String>; 2],
    key: String,
}

fn slot(kind: AssetKind) -> usize {
    match kind {
        AssetKind::Icon => 0,
        AssetKind::Backdrop => 1,
    }
}

impl Fetch {
    pub fn new(scope: usize) -> Fetch {
        Fetch {
            step: Step::Setup,
            row: 0,
            scope,
            replace: false,
            status: String::new(),
            current: String::new(),
            total: 0,
            done: 0,
            found: [0, 0],
            missing: 0,
            queue: VecDeque::new(),
            job: None,
            kinds: VecDeque::new(),
            candidates: VecDeque::new(),
            wait: Wait::Idle,
            wait_frames: 0,
            busy_replies: 0,
            retry: None,
            unknown: HashSet::new(),
            on_disk: [HashSet::new(), HashSet::new()],
            key: String::new(),
        }
    }

    /// Start a run over `jobs`; `art` is the art folder as the library last
    /// read it. Returns [`Action::AskKey`] when a title goes to SteamGridDB
    /// and there is no key.
    pub fn run(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings, jobs: Vec<Job>, art: HashSet<String>) -> Vec<Action> {
        if jobs.is_empty() {
            self.status = t.scrape_nothing.to_string();
            return Vec::new();
        }
        self.key = Online::key(host);
        if self.key.is_empty() && jobs.iter().any(|j| j.scraper == Scraper::SteamGridDb) {
            self.step = Step::Key;
            self.status.clear();
            return Vec::new();
        }
        self.step = Step::Running;
        self.total = jobs.len();
        self.done = 0;
        self.found = [0, 0];
        self.missing = 0;
        self.status.clear();
        self.queue = jobs.into();
        self.busy_replies = 0;
        self.unknown.clear();
        self.on_disk = [art, list_png(host, "backdrops").into_iter().collect()];
        self.next(host, t, listings)
    }

    /// The keyboard's API key: saved, then the run starts over the same
    /// titles.
    pub fn submit_key(&mut self, host: &mut dyn Host, t: &Messages, text: &str) -> bool {
        match sgdb::render_key(text) {
            Some(line) if host.write(sgdb::KEY_FILE, line.as_bytes()) => true,
            _ => {
                self.status = t.bad_key.to_string();
                false
            }
        }
    }

    /// Stop a run; what was fetched stays.
    pub fn stop(&mut self, host: &mut dyn Host, t: &Messages) {
        if self.step == Step::Running {
            self.halt(host, t.scrape_stopped.to_string());
        }
    }

    fn halt(&mut self, host: &mut dyn Host, message: String) {
        self.close(host);
        self.queue.clear();
        self.job = None;
        self.retry = None;
        self.wait_frames = 0;
        self.step = Step::Done;
        self.current.clear();
        self.status = message;
    }

    fn close(&mut self, host: &mut dyn Host) {
        match std::mem::replace(&mut self.wait, Wait::Idle) {
            Wait::ListingFetch(id, _) | Wait::Search(id) | Wait::Assets(id, ..) | Wait::SgdbFile(id, ..) | Wait::LibretroFile(id, ..) => host.net_close(id),
            Wait::Idle | Wait::Listing { .. } => {}
        }
    }

    fn next(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings) -> Vec<Action> {
        let Some(job) = self.queue.pop_front() else {
            self.halt(host, t.scrape_done.to_string());
            return Vec::new();
        };
        self.current = job.title.clone();
        let scraper = job.scraper;
        self.job = Some(job);
        match scraper {
            Scraper::Libretro => self.libretro(host, t, listings),
            Scraper::SteamGridDb => {
                self.search(host, t);
                Vec::new()
            }
        }
    }

    /// The title is done: the next one.
    fn finish(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings) -> Vec<Action> {
        self.done += 1;
        self.next(host, t, listings)
    }

    fn not_found(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings) -> Vec<Action> {
        self.missing += 1;
        self.finish(host, t, listings)
    }

    fn start(&mut self, host: &mut dyn Host, t: &Messages, url: &str, kind: NetKind) -> Option<NetId> {
        let id = host.net_start(url, kind);
        if id.is_none() {
            self.halt(host, t.no_network.to_string());
        }
        id
    }

    // --- libretro ----------------------------------------------------------

    fn libretro(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings) -> Vec<Action> {
        let folder = self.job.as_ref().and_then(|j| j.folder.clone()).unwrap_or_default();
        if folder.is_empty() || self.unknown.contains(&folder) {
            return self.not_found(host, t, listings);
        }
        if listings.get(&folder).is_some() {
            return self.matched(host, t, listings, &folder);
        }
        self.status = t.getting_list.to_string();
        listings.load(host, &folder);
        self.wait = Wait::Listing { folder, fetched: false };
        Vec::new()
    }

    /// A listing the host read; the run goes on when it was waiting for it.
    pub fn listing_ready(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings, folder: &str) -> Vec<Action> {
        let Wait::Listing { folder: waiting, fetched } = self.wait.clone() else { return Vec::new() };
        if waiting != folder {
            return Vec::new();
        }
        self.wait = Wait::Idle;
        if listings.get(folder).is_some() {
            return self.matched(host, t, listings, folder);
        }
        if fetched {
            // A reply with no names: the server has no such console.
            self.unknown.insert(folder.to_string());
            return self.not_found(host, t, listings);
        }
        if let Some(id) = self.start(host, t, &libretro::listing_url(folder), NetKind::Save(libretro::listing_path(folder))) {
            self.wait = Wait::ListingFetch(id, folder.to_string());
        }
        Vec::new()
    }

    fn matched(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings, folder: &str) -> Vec<Action> {
        let Some(job) = self.job.clone() else { return Vec::new() };
        let Some(listing) = listings.get(folder) else { return Vec::new() };
        let label = if job.label.is_empty() { &job.title } else { &job.label };
        let Some(name) = match_name(label, listing).or_else(|| match_name(&job.title, listing)) else {
            return self.not_found(host, t, listings);
        };
        self.candidates = job.kinds.iter().map(|&kind| (kind, libretro_candidates(folder, &name, &job.id, kind))).collect();
        self.next_candidate(host, t, listings)
    }

    /// The first candidate of each kind the server has, the next one on a
    /// 404, then the next title.
    fn next_candidate(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings) -> Vec<Action> {
        let mut actions = Vec::new();
        loop {
            let Some((kind, mut candidates)) = self.candidates.pop_front() else {
                actions.extend(self.finish(host, t, listings));
                return actions;
            };
            if candidates.is_empty() {
                continue;
            }
            let candidate = candidates.remove(0);
            if self.on_disk[slot(kind)].contains(&candidate.file) {
                actions.push(self.apply(kind, candidate.file));
                continue;
            }
            self.status = t.downloading.to_string();
            let path = candidate_path(kind, &candidate);
            let net = if candidate.fit { NetKind::SaveIcon(path) } else { NetKind::Save(path) };
            if let Some(id) = self.start(host, t, &candidate.url.clone(), net) {
                self.wait = Wait::LibretroFile(id, kind, candidate, candidates);
            }
            return actions;
        }
    }

    // --- SteamGridDB -------------------------------------------------------

    fn search(&mut self, host: &mut dyn Host, t: &Messages) {
        let Some(job) = &self.job else { return };
        let url = sgdb::search_url(&job.title);
        self.status = t.searching.to_string();
        let net = NetKind::Text { authorization: sgdb::authorization(&self.key) };
        if let Some(id) = self.start(host, t, &url, net) {
            self.wait = Wait::Search(id);
        }
    }

    fn next_kind(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings, game: u64) -> Vec<Action> {
        let Some(&kind) = self.kinds.front() else { return self.finish(host, t, listings) };
        self.status = t.loading_art.to_string();
        let net = NetKind::Text { authorization: sgdb::authorization(&self.key) };
        if let Some(id) = self.start(host, t, &sgdb::assets_url(kind, game), net) {
            self.wait = Wait::Assets(id, game, kind);
        }
        Vec::new()
    }

    /// A failed reply: wait and ask again when the server is busy; halt for
    /// anything else, which would repeat for every title that follows.
    fn problem(&mut self, host: &mut dyn Host, t: &Messages, problem: Problem, retry: Retry) {
        if problem != Problem::ServerBusy || self.busy_replies >= BUSY_RETRIES {
            self.halt(host, Online::problem_text(t, problem));
            return;
        }
        self.busy_replies += 1;
        self.retry = Some(retry);
        self.wait_frames = BUSY_WAIT;
        self.status = t.scrape_waiting.to_string();
    }

    fn apply(&mut self, kind: AssetKind, file: String) -> Action {
        self.found[slot(kind)] += 1;
        let id = self.job.as_ref().map(|j| j.id.clone()).unwrap_or_default();
        Action::Apply { id, kind, file }
    }

    // --- Each frame --------------------------------------------------------

    pub fn frame(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings) -> Vec<Action> {
        if self.step != Step::Running {
            return Vec::new();
        }
        if self.wait_frames > 0 {
            self.wait_frames -= 1;
            if self.wait_frames == 0 {
                match self.retry.take() {
                    Some(Retry::Search) => self.search(host, t),
                    Some(Retry::Assets(game)) => return self.next_kind(host, t, listings, game),
                    None => {}
                }
            }
            return Vec::new();
        }
        let id = match &self.wait {
            Wait::ListingFetch(id, _) | Wait::Search(id) | Wait::Assets(id, ..) | Wait::SgdbFile(id, ..) | Wait::LibretroFile(id, ..) => *id,
            Wait::Idle | Wait::Listing { .. } => return Vec::new(),
        };
        let state = host.net_state(id);
        if let NetState::Busy { received, total } = state {
            if matches!(self.wait, Wait::SgdbFile(..) | Wait::LibretroFile(..)) && total > 0 {
                self.status = fill(t.downloading_percent, &[("percent", &(received * 100 / total).to_string())]);
            }
            return Vec::new();
        }
        let text = if matches!(self.wait, Wait::Search(_) | Wait::Assets(..)) && matches!(state, NetState::Done { .. }) { host.net_text(id) } else { String::new() };
        host.net_close(id);
        let wait = std::mem::replace(&mut self.wait, Wait::Idle);
        let status = match state {
            NetState::Done { status, .. } => status,
            _ => 0,
        };
        let error = match &state {
            NetState::Error(error) => Some(error.clone()),
            _ => None,
        };
        match wait {
            Wait::ListingFetch(_, folder) => match error {
                // A network failure would repeat for every title.
                Some(error) => self.halt(host, fill(t.unreachable_libretro, &[("error", &error)])),
                None if status == 200 => {
                    listings.load(host, &folder);
                    self.wait = Wait::Listing { folder, fetched: true };
                }
                // No such console on the server: its titles are skipped.
                None => {
                    self.unknown.insert(folder);
                    return self.not_found(host, t, listings);
                }
            },
            Wait::LibretroFile(_, kind, candidate, others) => match error {
                Some(error) => self.halt(host, fill(t.download_failed, &[("error", &error)])),
                None if status != 200 => {
                    self.candidates.push_front((kind, others));
                    return self.next_candidate(host, t, listings);
                }
                None => {
                    self.on_disk[slot(kind)].insert(candidate.file.clone());
                    let mut actions = vec![self.apply(kind, candidate.file)];
                    actions.extend(self.next_candidate(host, t, listings));
                    return actions;
                }
            },
            Wait::Search(_) => {
                if let Some(error) = error {
                    self.halt(host, Online::unreachable(t, &error));
                    return Vec::new();
                }
                match sgdb::parse_games(status, &text) {
                    Err(problem) => self.problem(host, t, problem, Retry::Search),
                    Ok(games) => {
                        self.busy_replies = 0;
                        let Some(game) = games.first() else { return self.not_found(host, t, listings) };
                        self.kinds = self.job.as_ref().map(|j| j.kinds.iter().copied().collect()).unwrap_or_default();
                        return self.next_kind(host, t, listings, game.id);
                    }
                }
            }
            Wait::Assets(_, game, kind) => {
                if let Some(error) = error {
                    self.halt(host, Online::unreachable(t, &error));
                    return Vec::new();
                }
                match sgdb::parse_assets(status, &text) {
                    Err(problem) => self.problem(host, t, problem, Retry::Assets(game)),
                    Ok(assets) => {
                        self.busy_replies = 0;
                        let Some(asset) = assets.first() else {
                            self.kinds.pop_front();
                            return self.next_kind(host, t, listings, game);
                        };
                        let owner = self.job.as_ref().map(|j| j.id.clone()).unwrap_or_default();
                        let file = sgdb::asset_file(asset, &owner);
                        if self.on_disk[slot(kind)].contains(&file) {
                            self.kinds.pop_front();
                            let mut actions = vec![self.apply(kind, file)];
                            actions.extend(self.next_kind(host, t, listings, game));
                            return actions;
                        }
                        self.status = t.downloading.to_string();
                        if let Some(id) = self.start(host, t, &asset.url, NetKind::Save(sgdb::asset_path(kind, asset, &owner))) {
                            self.wait = Wait::SgdbFile(id, game, kind, file);
                        }
                    }
                }
            }
            Wait::SgdbFile(_, game, kind, file) => {
                if let Some(error) = error {
                    self.halt(host, fill(t.download_failed, &[("error", &error)]));
                    return Vec::new();
                }
                self.kinds.pop_front();
                // A file the server no longer has is skipped; the title keeps
                // what it had.
                let mut actions = Vec::new();
                if status == 200 {
                    self.on_disk[slot(kind)].insert(file.clone());
                    actions.push(self.apply(kind, file));
                }
                actions.extend(self.next_kind(host, t, listings, game));
                return actions;
            }
            Wait::Idle | Wait::Listing { .. } => {}
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::host::{Event, PictureKey, Status, TitleInfo};
    use crate::library::catalog::load_catalog;
    use crate::library::testing::Files;

    fn game(id: &str, title: &str, platform: &str, console: Option<&str>) -> Game {
        load_catalog(&[TitleInfo { id: id.into(), title: title.into(), platform: platform.into(), adrenaline: false, console: console.map(String::from) }]).remove(0)
    }

    #[test]
    fn a_title_with_no_files_of_its_own_lacks_both_pictures() {
        let g = game("PCSB00245", "Neon", "vita", None);
        assert_eq!(missing_art(&g), [AssetKind::Icon, AssetKind::Backdrop]);
        // An icon the user chose, one matched by name, and the title's own icon are kept.
        assert_eq!(missing_art(&Game { art: Some("neon.png".into()), ..g.clone() }), [AssetKind::Backdrop]);
        assert_eq!(missing_art(&Game { art: Some("PCSB00245.png".into()), art_auto: true, ..g.clone() }), [AssetKind::Backdrop]);
        assert_eq!(missing_art(&Game { art_icon: true, ..g.clone() }), [AssetKind::Backdrop]);
        // A chosen backdrop, or the choice of none, is kept.
        assert_eq!(missing_art(&Game { backdrop: Some("neon.png".into()), ..g.clone() }), [AssetKind::Icon]);
        assert_eq!(missing_art(&Game { backdrop: Some("none".into()), ..g.clone() }), [AssetKind::Icon]);
        assert!(missing_art(&Game { art: Some("neon.png".into()), backdrop: Some("neon.png".into()), ..g }).is_empty());
        assert_eq!(kinds_for(&game("A", "A", "vita", None), true).len(), 2);
    }

    /// A server answering each request at once, by URL.
    #[derive(Default)]
    struct Server {
        files: Files,
        replies: HashMap<String, (u16, String)>,
        open: HashMap<i32, String>,
        next: i32,
        asked: Vec<String>,
        saved: Vec<String>,
        listings: Vec<String>,
    }

    impl Host for Server {
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
        fn request(&mut self, _: PictureKey) {}
        fn poll(&mut self, _: &mut Vec<Event>) {}
        fn log(&mut self, _: &str) {}
        fn launch(&mut self, _: &str) -> bool {
            true
        }
        fn plugin_state(&mut self) -> crate::host::PluginState {
            crate::host::PluginState::On
        }
        fn enable_plugin(&mut self) -> bool {
            true
        }
        fn net_start(&mut self, url: &str, kind: NetKind) -> Option<NetId> {
            self.next += 1;
            self.asked.push(url.into());
            if let NetKind::Save(path) | NetKind::SaveIcon(path) = kind {
                if self.replies.get(url).is_none_or(|r| r.0 == 200) {
                    self.saved.push(path);
                }
            }
            self.open.insert(self.next, url.into());
            Some(NetId(self.next))
        }
        fn net_state(&mut self, id: NetId) -> NetState {
            let status = self.replies.get(&self.open[&id.0]).map_or(200, |r| r.0);
            NetState::Done { status, received: 1 }
        }
        fn net_text(&mut self, id: NetId) -> String {
            self.replies.get(&self.open[&id.0]).map(|r| r.1.clone()).unwrap_or_default()
        }
        fn listing(&mut self, folder: &str) {
            self.listings.push(folder.into());
        }
    }

    const KEY: &str = "0123456789abcdef0123456789abcdef";

    fn server() -> Server {
        let mut server = Server::default();
        server.files.0.insert(sgdb::KEY_FILE.into(), format!("{KEY}\n").into_bytes());
        server.replies.insert(sgdb::search_url("Gravity Daze"), (200, r#"{"success":true,"data":[{"id":7,"name":"Gravity Daze"}]}"#.into()));
        server.replies.insert(sgdb::search_url("Unknown Game"), (200, r#"{"success":true,"data":[]}"#.into()));
        server.replies.insert(sgdb::assets_url(AssetKind::Icon, 7), (200, r#"{"success":true,"data":[{"id":11,"url":"https://x/i.png"}]}"#.into()));
        server.replies.insert(sgdb::assets_url(AssetKind::Backdrop, 7), (200, r#"{"success":true,"data":[{"id":12,"url":"https://x/b.png"}]}"#.into()));
        server
    }

    fn jobs(games: &[Game], source: ScrapeSource) -> Vec<Job> {
        games.iter().filter_map(|g| Job::for_game(g, &g.title, source, false)).collect()
    }

    fn run_to_end(fetch: &mut Fetch, host: &mut Server, t: &Messages, listings: &mut Listings, mut actions: Vec<Action>) -> Vec<Action> {
        for _ in 0..2000 {
            if fetch.step != Step::Running {
                break;
            }
            actions.extend(fetch.frame(host, t, listings));
        }
        actions
    }

    fn t() -> &'static Messages {
        crate::i18n::messages(crate::library::settings::Language::En)
    }

    #[test]
    fn steamgriddb_gives_the_first_icon_and_backdrop_of_the_first_game() {
        let (mut host, mut listings, mut fetch) = (server(), Listings::default(), Fetch::new(0));
        let games = [game("PCSA00069", "Gravity Daze", "vita", None), game("PCSB00001", "Unknown Game", "vita", None)];
        let actions = fetch.run(&mut host, t(), &mut listings, jobs(&games, ScrapeSource::Auto), HashSet::new());
        let actions = run_to_end(&mut fetch, &mut host, t(), &mut listings, actions);
        assert_eq!(
            actions,
            [
                Action::Apply { id: "PCSA00069".into(), kind: AssetKind::Icon, file: "PCSA00069-11.png".into() },
                Action::Apply { id: "PCSA00069".into(), kind: AssetKind::Backdrop, file: "PCSA00069-12.png".into() },
            ]
        );
        assert_eq!(host.saved, ["art/PCSA00069-11.png", "backdrops/PCSA00069-12.png"]);
        assert_eq!((fetch.step, fetch.done, fetch.total, fetch.found, fetch.missing), (Step::Done, 2, 2, [1, 1], 1));
        assert_eq!(fetch.status, t().scrape_done);
    }

    #[test]
    fn a_file_already_on_the_card_is_used_without_a_download() {
        let (mut host, mut listings, mut fetch) = (server(), Listings::default(), Fetch::new(0));
        let games = [game("PCSA00069", "Gravity Daze", "vita", None)];
        let art = HashSet::from(["PCSA00069-11.png".to_string()]);
        let actions = fetch.run(&mut host, t(), &mut listings, jobs(&games, ScrapeSource::Auto), art);
        run_to_end(&mut fetch, &mut host, t(), &mut listings, actions);
        assert_eq!(host.saved, ["backdrops/PCSA00069-12.png"]);
        assert_eq!(fetch.found, [1, 1]);
    }

    #[test]
    fn a_busy_server_is_waited_out_then_the_run_halts() {
        let (mut host, mut listings, mut fetch) = (server(), Listings::default(), Fetch::new(0));
        host.replies.insert(sgdb::search_url("Gravity Daze"), (429, String::new()));
        let games = [game("PCSA00069", "Gravity Daze", "vita", None)];
        fetch.run(&mut host, t(), &mut listings, jobs(&games, ScrapeSource::Auto), HashSet::new());
        fetch.frame(&mut host, t(), &mut listings);
        assert_eq!((fetch.step, fetch.status.as_str()), (Step::Running, t().scrape_waiting));
        for _ in 0..BUSY_WAIT {
            fetch.frame(&mut host, t(), &mut listings);
        }
        assert_eq!(host.asked.len(), 2, "asked again after the wait");
        run_to_end(&mut fetch, &mut host, t(), &mut listings, Vec::new());
        assert_eq!(host.asked.len(), 1 + BUSY_RETRIES as usize);
        assert_eq!((fetch.step, fetch.status.as_str()), (Step::Done, t().server_busy));
    }

    #[test]
    fn a_rejected_key_halts_and_no_key_asks_for_one() {
        let (mut host, mut listings, mut fetch) = (server(), Listings::default(), Fetch::new(0));
        host.replies.insert(sgdb::search_url("Gravity Daze"), (401, String::new()));
        let games = [game("PCSA00069", "Gravity Daze", "vita", None), game("PCSA00070", "Gravity Daze", "vita", None)];
        fetch.run(&mut host, t(), &mut listings, jobs(&games, ScrapeSource::Auto), HashSet::new());
        run_to_end(&mut fetch, &mut host, t(), &mut listings, Vec::new());
        assert_eq!((fetch.step, fetch.status.as_str(), fetch.done), (Step::Done, t().key_rejected, 0));

        host.files.0.remove(sgdb::KEY_FILE);
        let mut fetch = Fetch::new(0);
        fetch.run(&mut host, t(), &mut listings, jobs(&games, ScrapeSource::Auto), HashSet::new());
        assert_eq!(fetch.step, Step::Key);
        assert!(!fetch.submit_key(&mut host, t(), "not a key"));
        assert!(fetch.submit_key(&mut host, t(), KEY));
    }

    #[test]
    fn libretro_matches_the_label_and_skips_a_console_the_server_lacks() {
        let (mut host, mut listings, mut fetch) = (server(), Listings::default(), Fetch::new(0));
        let gb = "Nintendo - Game Boy";
        let tetris = game("ra0000001", "Tetris (World)", "retro", Some(gb));
        let odd = game("ra0000002", "Odd Game", "retro", Some("Odd - Console"));
        let odd_too = game("ra0000003", "Odd Game 2", "retro", Some("Odd - Console"));
        let names = jobs(&[tetris.clone(), odd, odd_too], ScrapeSource::Auto);
        let folder = names[0].folder.clone().unwrap();
        let odd_folder = names[1].folder.clone().unwrap();
        // The box art is missing on the server: the title screen is next.
        let boxart = libretro_candidates(&folder, "Tetris (World)", "ra0000001", AssetKind::Icon);
        host.replies.insert(boxart[0].url.clone(), (404, String::new()));
        host.replies.insert(libretro::listing_url(&odd_folder), (404, String::new()));

        let mut actions = fetch.run(&mut host, t(), &mut listings, names, HashSet::new());
        // The host is asked for the listing; it is on the card.
        assert_eq!(host.listings, [folder.clone()]);
        listings.answer(&folder, libretro::Listing::parse("Tetris (World)\nTetris (Japan)\n"));
        actions.extend(fetch.listing_ready(&mut host, t(), &mut listings, &folder));
        for _ in 0..10 {
            actions.extend(fetch.frame(&mut host, t(), &mut listings));
        }
        // The odd console: no file, then a 404 from the server; its second
        // title is skipped without asking again.
        listings.answer(&odd_folder, None);
        actions.extend(fetch.listing_ready(&mut host, t(), &mut listings, &odd_folder));
        actions = run_to_end(&mut fetch, &mut host, t(), &mut listings, actions);
        assert_eq!(
            actions,
            [
                Action::Apply { id: "ra0000001".into(), kind: AssetKind::Icon, file: "ra0000001-lr-title.png".into() },
                Action::Apply { id: "ra0000001".into(), kind: AssetKind::Backdrop, file: "ra0000001-lr-snap.png".into() },
            ]
        );
        assert_eq!(host.asked.iter().filter(|u| **u == libretro::listing_url(&odd_folder)).count(), 1);
        assert_eq!((fetch.step, fetch.done, fetch.missing), (Step::Done, 3, 2));
    }
}
