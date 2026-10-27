//! Finding an icon and a backdrop for one title (title editor ▸ Find
//! artwork, `src/online.ts`), on SteamGridDB or libretro's server. The shelf
//! steps it once per frame; it polls its own requests and returns what the
//! shelf has to do (open the keyboard, ask for a preview, apply a file).

use std::collections::{HashMap, HashSet};

use super::libretro::{self, Listing};
use super::steamgriddb::{self as sgdb, AssetKind, Problem};
use super::{candidate_path, libretro_candidates, sgdb_candidates, Candidate, Scraper};
use crate::host::{Host, NetId, NetKind, NetState, PictureKey};
use crate::i18n::fill;
use crate::library::list_png;
use crate::locales::Messages;

/// Rows shown at once in the games list, the search row included.
pub const ONLINE_ROWS: usize = 6;
pub const KINDS: [AssetKind; 2] = [AssetKind::Icon, AssetKind::Backdrop];

fn slot(kind: AssetKind) -> usize {
    match kind {
        AssetKind::Icon => 0,
        AssetKind::Backdrop => 1,
    }
}

/// Consoles' box art names: read from `libretro/<console>.txt`, or fetched
/// once (`createListings`). A kept listing is used as it is.
#[derive(Default)]
pub struct Listings {
    loaded: HashMap<String, Listing>,
}

impl Listings {
    /// A console's listing when on disk or loaded.
    pub fn get(&mut self, host: &mut dyn Host, folder: &str) -> Option<&Listing> {
        if !self.loaded.contains_key(folder) {
            let listing = host.read(&libretro::listing_path(folder)).and_then(|b| Listing::parse(&String::from_utf8_lossy(&b)))?;
            self.loaded.insert(folder.to_string(), listing);
        }
        self.loaded.get(folder)
    }

    /// Start fetching a console's listing to its file.
    pub fn fetch(host: &mut dyn Host, folder: &str) -> Option<NetId> {
        host.net_start(&libretro::listing_url(folder), NetKind::Save(libretro::listing_path(folder)))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    /// No usable SteamGridDB API key yet.
    Key,
    /// The search term and the games (or libretro names) it found.
    Games,
    /// The chosen one's icons and backdrops, one candidate at a time.
    Assets,
}

/// A game found: SteamGridDB's id and name, or a libretro name.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Found {
    pub id: u64,
    pub name: String,
    pub year: Option<i32>,
}

/// What a request in flight is for.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Purpose {
    Search,
    Listing(String),
    Assets(AssetKind),
    Download(AssetKind, String),
}

/// What the shelf has to do for the flow.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Action {
    /// Open the keyboard for the search term (`false`) or the API key.
    AskText { key: bool, text: String },
    /// Decode a candidate for its preview.
    Preview(PictureKey),
    /// Use a downloaded file for the edited title.
    Apply(AssetKind, String),
}

pub struct Online {
    pub scraper: Scraper,
    owner: String,
    folder: Option<String>,
    pub step: Step,
    pub row: usize,
    pub status: String,
    pub busy: bool,
    pub term: String,
    pub games: Vec<Found>,
    pub chosen: Option<Found>,
    pub assets: [Vec<Candidate>; 2],
    pub index: [usize; 2],
    /// The picture asked for each kind's candidate, and whether it decoded.
    pub preview: [Option<PictureKey>; 2],
    pub shown: [bool; 2],
    pending: Vec<(NetId, Purpose)>,
    asset_lists: [Option<Result<Vec<sgdb::Asset>, String>>; 2],
    on_disk: [HashSet<String>; 2],
    downloaded: [HashSet<String>; 2],
}

impl Online {
    /// Open for a title: its id, its folder on libretro's server, and the
    /// term the search starts from (its playlist label for libretro).
    pub fn start(host: &mut dyn Host, t: &Messages, scraper: Scraper, owner: &str, folder: Option<String>, term: &str, listings: &mut Listings) -> (Online, Vec<Action>) {
        let mut online = Online {
            scraper,
            owner: owner.to_string(),
            folder,
            step: Step::Games,
            row: 0,
            status: String::new(),
            busy: false,
            term: term.to_string(),
            games: Vec::new(),
            chosen: None,
            assets: [Vec::new(), Vec::new()],
            index: [0, 0],
            preview: [None, None],
            shown: [false, false],
            pending: Vec::new(),
            asset_lists: [None, None],
            on_disk: [list_png(host, "art").into_iter().collect(), list_png(host, "backdrops").into_iter().collect()],
            downloaded: [HashSet::new(), HashSet::new()],
        };
        let actions = online.search(host, t, listings);
        (online, actions)
    }

    fn cancel_all(&mut self, host: &mut dyn Host) {
        for (id, _) in self.pending.drain(..) {
            host.net_close(id);
        }
        self.busy = false;
    }

    fn unreachable(t: &Messages, error: &str) -> String {
        fill(t.unreachable, &[("error", error)])
    }

    fn problem_text(t: &Messages, problem: Problem) -> String {
        match problem {
            Problem::KeyRejected => t.key_rejected.to_string(),
            Problem::ServerBusy => t.server_busy.to_string(),
            Problem::ServerStatus(status) => fill(t.server_status, &[("status", &status.to_string())]),
            Problem::BadReply => t.bad_reply.to_string(),
        }
    }

    fn key(host: &mut dyn Host) -> String {
        host.read(sgdb::KEY_FILE).map(|b| sgdb::parse_key(&String::from_utf8_lossy(&b))).unwrap_or_default()
    }

    fn start_request(&mut self, host: &mut dyn Host, t: &Messages, url: &str, kind: NetKind, purpose: Purpose) -> bool {
        match host.net_start(url, kind) {
            Some(id) => {
                self.pending.push((id, purpose));
                true
            }
            None => {
                self.busy = false;
                self.status = t.no_network.to_string();
                false
            }
        }
    }

    /// Search the term: libretro's listing for the title's console, or
    /// SteamGridDB (with the key; none yet asks for it).
    fn search(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings) -> Vec<Action> {
        self.cancel_all(host);
        self.step = Step::Games;
        self.games.clear();
        self.row = 0;
        if self.scraper == Scraper::Libretro {
            let Some(folder) = self.folder.clone() else {
                self.status = t.not_retro_arch.to_string();
                return Vec::new();
            };
            if listings.get(host, &folder).is_some() {
                self.show_names(host, t, listings, &folder);
                return Vec::new();
            }
            self.busy = true;
            self.status = t.getting_list.to_string();
            if let Some(id) = Listings::fetch(host, &folder) {
                self.pending.push((id, Purpose::Listing(folder)));
            } else {
                self.busy = false;
                self.status = t.no_network.to_string();
            }
            return Vec::new();
        }
        let key = Online::key(host);
        if key.is_empty() {
            self.step = Step::Key;
            self.status.clear();
            return Vec::new();
        }
        self.busy = true;
        self.status = t.searching.to_string();
        let url = sgdb::search_url(&self.term);
        self.start_request(host, t, &url, NetKind::Text { authorization: sgdb::authorization(&key) }, Purpose::Search);
        Vec::new()
    }

    fn show_names(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings, folder: &str) {
        let names = listings.get(host, folder).map(|l| libretro::search_names(&self.term, l, 30)).unwrap_or_default();
        self.games = names.into_iter().enumerate().map(|(i, name)| Found { id: i as u64, name, year: None }).collect();
        self.row = if self.games.is_empty() { 0 } else { 1 };
        self.status = if self.games.is_empty() { t.no_games.to_string() } else { String::new() };
    }

    /// Put the chosen one's candidates up and show the first icon.
    fn show_candidates(&mut self, host: &mut dyn Host, t: &Messages, found: [Vec<Candidate>; 2], problem: String) -> Vec<Action> {
        self.busy = false;
        self.assets = found;
        self.index = [0, 0];
        self.preview = [None, None];
        self.shown = [false, false];
        self.step = Step::Assets;
        self.row = 0;
        self.status = problem.clone();
        if problem.is_empty() {
            self.show(host, t, AssetKind::Icon)
        } else {
            Vec::new()
        }
    }

    fn load_assets(&mut self, host: &mut dyn Host, t: &Messages, game: Found) -> Vec<Action> {
        self.cancel_all(host);
        self.chosen = Some(game.clone());
        if self.scraper == Scraper::Libretro {
            let Some(folder) = self.folder.clone() else { return Vec::new() };
            let found = [
                libretro_candidates(&folder, &game.name, &self.owner, AssetKind::Icon),
                libretro_candidates(&folder, &game.name, &self.owner, AssetKind::Backdrop),
            ];
            return self.show_candidates(host, t, found, String::new());
        }
        let key = Online::key(host);
        self.busy = true;
        self.status = t.loading_art.to_string();
        self.asset_lists = [None, None];
        for kind in KINDS {
            let url = sgdb::assets_url(kind, game.id);
            if !self.start_request(host, t, &url, NetKind::Text { authorization: sgdb::authorization(&key) }, Purpose::Assets(kind)) {
                break;
            }
        }
        Vec::new()
    }

    /// Put the current candidate of `kind` up, downloading it first when
    /// needed.
    fn show(&mut self, host: &mut dyn Host, t: &Messages, kind: AssetKind) -> Vec<Action> {
        self.cancel_all(host);
        let s = slot(kind);
        self.preview[s] = None;
        self.shown[s] = false;
        let Some(candidate) = self.assets[s].get(self.index[s]).cloned() else {
            self.status = (if kind == AssetKind::Icon { t.no_icons } else { t.no_backdrops }).to_string();
            return Vec::new();
        };
        if self.on_disk[s].contains(&candidate.file) {
            return vec![self.display(t, kind, &candidate.file)];
        }
        self.busy = true;
        self.status = t.downloading.to_string();
        let path = candidate_path(kind, &candidate);
        let net = if candidate.fit { NetKind::SaveIcon(path) } else { NetKind::Save(path) };
        self.start_request(host, t, &candidate.url, net, Purpose::Download(kind, candidate.file));
        Vec::new()
    }

    /// Ask for a downloaded candidate's picture.
    fn display(&mut self, t: &Messages, kind: AssetKind, file: &str) -> Action {
        let key = match kind {
            AssetKind::Icon => PictureKey::Art(file.to_string()),
            AssetKind::Backdrop => PictureKey::Backdrop { id: self.owner.clone(), file: file.to_string() },
        };
        self.preview[slot(kind)] = Some(key.clone());
        self.status = t.loading.to_string();
        Action::Preview(key)
    }

    /// A preview's answer: decoded or not.
    pub fn previewed(&mut self, t: &Messages, key: &PictureKey, ok: bool) -> bool {
        let Some(s) = self.preview.iter().position(|p| p.as_ref() == Some(key)) else { return false };
        self.shown[s] = ok;
        self.status = if ok { String::new() } else { t.unreadable.to_string() };
        true
    }

    /// Poll the requests in flight; once per frame.
    pub fn frame(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings) -> Vec<Action> {
        let mut actions = Vec::new();
        let pending = std::mem::take(&mut self.pending);
        let mut finished = Vec::new();
        for (id, purpose) in pending {
            match host.net_state(id) {
                NetState::Busy { received, total } => {
                    if let Purpose::Download(..) = purpose {
                        self.status = if total > 0 {
                            fill(t.downloading_percent, &[("percent", &(received * 100 / total).to_string())])
                        } else {
                            fill(t.downloading_size, &[("size", &(received / 1024).to_string())])
                        };
                    }
                    self.pending.push((id, purpose));
                }
                state => {
                    let text = if matches!(state, NetState::Done { .. }) { host.net_text(id) } else { String::new() };
                    host.net_close(id);
                    finished.push((purpose, state, text));
                }
            }
        }
        for (purpose, state, text) in finished {
            actions.extend(self.finished(host, t, listings, purpose, state, text));
        }
        actions
    }

    fn finished(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings, purpose: Purpose, state: NetState, text: String) -> Vec<Action> {
        let failed = |state: &NetState| match state {
            NetState::Error(error) => Some(error.clone()),
            _ => None,
        };
        let status = match state {
            NetState::Done { status, .. } => status,
            _ => 0,
        };
        match purpose {
            Purpose::Search => {
                self.busy = false;
                if let Some(error) = failed(&state) {
                    self.status = Online::unreachable(t, &error);
                    return Vec::new();
                }
                match sgdb::parse_games(status, &text) {
                    Ok(found) => {
                        self.games = found.into_iter().map(|g| Found { id: g.id, name: g.name, year: g.year }).collect();
                        self.row = if self.games.is_empty() { 0 } else { 1 };
                        self.status = if self.games.is_empty() { t.no_games.to_string() } else { String::new() };
                    }
                    Err(problem) => {
                        self.status = Online::problem_text(t, problem);
                        if problem == Problem::KeyRejected {
                            self.step = Step::Key;
                        }
                    }
                }
                Vec::new()
            }
            Purpose::Listing(folder) => {
                self.busy = false;
                let error = failed(&state).unwrap_or_else(|| format!("HTTP {status}"));
                if status == 200 && listings.get(host, &folder).is_some() {
                    self.show_names(host, t, listings, &folder);
                } else {
                    self.status = fill(t.unreachable_libretro, &[("error", &error)]);
                }
                Vec::new()
            }
            Purpose::Assets(kind) => {
                let list = match failed(&state) {
                    Some(error) => Err(Online::unreachable(t, &error)),
                    None => sgdb::parse_assets(status, &text).map_err(|p| Online::problem_text(t, p)),
                };
                self.asset_lists[slot(kind)] = Some(list);
                let [Some(icons), Some(backdrops)] = &self.asset_lists else { return Vec::new() };
                let problem = [icons, backdrops].iter().find_map(|l| l.as_ref().err().cloned()).unwrap_or_default();
                let owner = self.owner.clone();
                let found = [
                    sgdb_candidates(icons.as_deref().unwrap_or(&[]), &owner),
                    sgdb_candidates(backdrops.as_deref().unwrap_or(&[]), &owner),
                ];
                self.show_candidates(host, t, found, problem)
            }
            Purpose::Download(kind, file) => {
                self.busy = false;
                // libretro's server has no picture of this kind under this name.
                if status == 404 {
                    self.status = t.not_on_server.to_string();
                    return Vec::new();
                }
                if let Some(error) = failed(&state).or_else(|| (status != 200).then(|| fill(t.status_code, &[("status", &status.to_string())]))) {
                    self.status = fill(t.download_failed, &[("error", &error)]);
                    return Vec::new();
                }
                self.on_disk[slot(kind)].insert(file.clone());
                self.downloaded[slot(kind)].insert(file.clone());
                vec![self.display(t, kind, &file)]
            }
        }
    }

    fn kind_of_row(&self) -> AssetKind {
        KINDS[self.row.min(1)]
    }

    pub fn current(&self, kind: AssetKind) -> Option<&Candidate> {
        self.assets[slot(kind)].get(self.index[slot(kind)])
    }

    /// The D-pad: rows, or a kind's previous and next candidate.
    pub fn step_by(&mut self, host: &mut dyn Host, t: &Messages, dx: i32, dy: i32) -> Vec<Action> {
        match self.step {
            Step::Games if dy != 0 => {
                let count = self.games.len() as i32 + 1;
                self.row = (self.row as i32 + dy).rem_euclid(count) as usize;
            }
            Step::Assets if dy != 0 => {
                self.row = (self.row as i32 + dy).rem_euclid(2) as usize;
                let kind = self.kind_of_row();
                if self.preview[slot(kind)].is_none() {
                    return self.show(host, t, kind);
                }
            }
            Step::Assets if dx != 0 => {
                let kind = self.kind_of_row();
                let count = self.assets[slot(kind)].len() as i32;
                if count < 2 {
                    return Vec::new();
                }
                let s = slot(kind);
                self.index[s] = (self.index[s] as i32 + dx).rem_euclid(count) as usize;
                return self.show(host, t, kind);
            }
            _ => {}
        }
        Vec::new()
    }

    pub fn confirm(&mut self, host: &mut dyn Host, t: &Messages) -> Vec<Action> {
        match self.step {
            Step::Key => vec![Action::AskText { key: true, text: String::new() }],
            Step::Games if self.row == 0 => vec![Action::AskText { key: false, text: self.term.clone() }],
            Step::Games => match self.games.get(self.row - 1).cloned() {
                Some(game) if !self.busy => self.load_assets(host, t, game),
                _ => Vec::new(),
            },
            Step::Assets => {
                let kind = self.kind_of_row();
                let Some(candidate) = self.current(kind).cloned() else { return Vec::new() };
                if !self.shown[slot(kind)] {
                    return Vec::new();
                }
                self.status = (if kind == AssetKind::Icon { t.icon_applied } else { t.backdrop_applied }).to_string();
                vec![Action::Apply(kind, candidate.file)]
            }
        }
    }

    /// Back one step; true when the flow closes.
    pub fn back(&mut self, host: &mut dyn Host) -> bool {
        if self.step == Step::Assets {
            self.cancel_all(host);
            self.preview = [None, None];
            self.shown = [false, false];
            self.step = Step::Games;
            let chosen = self.chosen.as_ref().map(|c| c.id);
            self.row = self.games.iter().position(|g| Some(g.id) == chosen).map_or(1, |i| i + 1);
            self.status.clear();
            return false;
        }
        true
    }

    /// Close: requests cancelled, and candidates looked at and not chosen
    /// deleted (`in_use` says which files a title uses).
    pub fn close(&mut self, host: &mut dyn Host, in_use: impl Fn(AssetKind, &str) -> bool) {
        self.cancel_all(host);
        for kind in KINDS {
            for file in self.downloaded[slot(kind)].drain() {
                if !in_use(kind, &file) {
                    host.remove(&format!("{}/{file}", sgdb::folder(kind)));
                }
            }
        }
    }

    pub fn submit_term(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings, text: &str) -> Vec<Action> {
        if text.is_empty() {
            return Vec::new();
        }
        self.term = text.to_string();
        self.search(host, t, listings)
    }

    pub fn submit_key(&mut self, host: &mut dyn Host, t: &Messages, listings: &mut Listings, text: &str) -> Vec<Action> {
        match sgdb::render_key(text) {
            Some(line) if host.write(sgdb::KEY_FILE, line.as_bytes()) => self.search(host, t, listings),
            _ => {
                self.status = t.bad_key.to_string();
                Vec::new()
            }
        }
    }
}
