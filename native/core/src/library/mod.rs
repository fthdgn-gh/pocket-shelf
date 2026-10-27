//! The library: the installed titles with the user's changes, the
//! categories and their tabs, "Last Played", favorites and search (the
//! library part of `src/state.ts`), and the saved files they come from.
//!
//! Every file is read and written in the current app's format; the tests in
//! `tests/round_trip.rs` check that against files its own code wrote.

pub mod catalog;
pub mod categories;
pub mod consoles;
pub mod fxhash;
pub mod overrides;
pub mod recent;
#[cfg(test)]
mod round_trip;
pub mod search;
pub mod settings;

use std::sync::Arc;

use catalog::{load_catalog, Game};
use categories::{
    builtin_categories, order_categories, search_category, smart_categories, Category, CategoryConfig, CATEGORIES_FILE,
    FAVORITES_ID, RECENT_ID, SEARCH_ID, SHOW_EMPTY_CATEGORIES,
};
use consoles::console_categories;
use fxhash::FxHashMap;
use overrides::{OverrideStore, TitleOverride, USE_ICON};

use crate::host::{Host, TitleInfo};

/// PNG files in a folder of the data folder (any case, not hidden), sorted
/// by lowercase name (`listPng` in `art-files.ts`).
pub fn list_png(host: &mut dyn Host, dir: &str) -> Vec<String> {
    let mut names: Vec<String> =
        host.list(dir).into_iter().filter(|name| !name.starts_with('.') && name.to_lowercase().ends_with(".png")).collect();
    // Each lowercase name made once, not once per comparison.
    names.sort_by_cached_key(|n| n.to_lowercase());
    names
}

/// PNG files in `art/` (any case, not hidden), by lowercase name without
/// `.png`.
fn art_index(host: &mut dyn Host) -> FxHashMap<String, String> {
    host.list("art")
        .into_iter()
        .filter(|name| !name.starts_with('.') && name.to_lowercase().ends_with(".png"))
        .map(|name| {
            let lower = name.to_lowercase();
            (lower[..lower.len() - 4].to_string(), name)
        })
        .collect()
}

/// `text` lowercased into `out`; ASCII without the Unicode tables.
fn lowercase_into(text: &str, out: &mut String) {
    out.clear();
    if text.is_ascii() {
        out.push_str(text);
        out.make_ascii_lowercase();
    } else {
        out.extend(text.chars().flat_map(char::to_lowercase));
    }
}

pub struct Library {
    /// The host's titles as listed, without the user's changes.
    base: Vec<Game>,
    pub overrides: OverrideStore,
    /// Art files by lowercase name without `.png`, for matching by title id
    /// or title.
    art: FxHashMap<String, String>,
    pub config: CategoryConfig,
    /// Title ids, the one started last first.
    pub recent: Vec<String>,
    /// The open search's term; empty when no search is open.
    pub search_term: String,

    /// Every category in tab order, hidden ones included.
    categories: Vec<Category>,
    /// The titles with the user's changes, in the host's order.
    games: Vec<Game>,
    /// Indexes into `games` per category id.
    by_category: FxHashMap<Arc<str>, Vec<usize>>,
    /// Index of each title by id.
    index: FxHashMap<Arc<str>, usize>,
}

impl Library {
    /// The saved changes, categories and "Last Played", with no titles until
    /// [`Library::set_titles`].
    pub fn load(host: &mut dyn Host) -> Library {
        let overrides = OverrideStore::load(host);
        let art = art_index(host);
        let config = host.read(CATEGORIES_FILE).map(|b| CategoryConfig::parse(&b)).unwrap_or_default();
        let recent = host.read(recent::RECENT_FILE).map(|b| recent::parse(&b)).unwrap_or_default();
        let mut library = Library {
            base: Vec::new(),
            overrides,
            art,
            config,
            recent,
            search_term: String::new(),
            categories: Vec::new(),
            games: Vec::new(),
            by_category: FxHashMap::default(),
            index: FxHashMap::default(),
        };
        library.rebuild();
        library
    }

    /// The host's titles ([`load_catalog`]), after a start or a scan.
    pub fn set_catalog(&mut self, games: Vec<Game>) {
        self.base = games;
        self.rebuild();
    }

    /// [`Library::set_catalog`] from the host's list.
    pub fn set_titles(&mut self, infos: &[TitleInfo]) {
        self.set_catalog(load_catalog(infos));
    }

    /// Recompute the categories and titles after a change.
    pub fn rebuild(&mut self) {
        // A console's category, unless the user made one with the same id.
        // Each console once, in the order of its first game.
        let mut seen = std::collections::HashSet::with_hasher(fxhash::FxBuild::default());
        let names = self.base.iter().filter_map(|g| g.console.as_deref()).filter(|name| seen.insert(*name));
        let consoles = console_categories(names);
        let retro = consoles.into_iter().filter(|c| !self.config.custom.iter().any(|own| own.id == c.id));
        let items: Vec<Category> =
            smart_categories().into_iter().chain(builtin_categories()).chain(retro).chain(self.config.custom.iter().cloned()).collect();
        self.categories = order_categories(&items, &self.config.order);

        let mut key = String::new();
        self.games = self.base.iter().map(|game| self.with_changes(game, &mut key)).collect();
        self.index.clear();
        self.by_category.clear();
        for (index, game) in self.games.iter().enumerate() {
            self.index.insert(game.id.clone(), index);
            self.by_category.entry(game.category.clone()).or_default().push(index);
        }
    }

    /// A host title with the user's changes and its art file. `key` is a
    /// buffer for the art lookups.
    fn with_changes(&self, game: &Game, key: &mut String) -> Game {
        // `<title id>.png`, then `<title>.png`, any case. With no art files,
        // nothing is looked up.
        let mut find_art = |names: [&str; 3]| {
            if self.art.is_empty() {
                return None;
            }
            names.iter().find_map(|name| {
                lowercase_into(name, key);
                self.art.get(key.as_str()).cloned()
            })
        };
        let change = self.overrides.titles.get(&*game.id);
        let title: Arc<str> = change.and_then(|c| c.title.as_deref()).map_or_else(|| game.title.clone(), Arc::from);
        let chosen = change.and_then(|c| c.art.clone());
        let force_icon = chosen.as_deref() == Some(USE_ICON);
        let art = if force_icon { None } else { chosen.clone().or_else(|| find_art([&game.id, &game.title, &title])) };
        let Some(change) = change else { return Game { art_auto: art.is_some(), art, ..game.clone() } };
        // A category that was deleted falls back to the title's own.
        let category = change
            .category
            .as_deref()
            .filter(|wanted| self.categories.iter().any(|c| c.id == *wanted))
            .map_or_else(|| game.category.clone(), Arc::from);
        Game {
            title,
            category,
            favorite: change.favorite,
            art_auto: !force_icon && chosen.is_none() && art.is_some(),
            art,
            art_icon: force_icon,
            backdrop: change.backdrop.clone(),
            ..game.clone()
        }
    }

    /// Change one title's changes and recompute that title alone: a full
    /// rebuild took 38 ms on hardware with 5,905 titles.
    pub fn patch(&mut self, id: &str, edit: impl FnOnce(&mut TitleOverride)) {
        let mut change = self.overrides.titles.get(id).cloned().unwrap_or_default();
        edit(&mut change);
        self.overrides.set(id, change);
        let Some(&index) = self.index.get(id) else { return };
        let mut key = String::new();
        let game = self.with_changes(&self.base[index], &mut key);
        let old = std::mem::replace(&mut self.games[index], game);
        if old.category != self.games[index].category {
            if let Some(list) = self.by_category.get_mut(&old.category) {
                list.retain(|&i| i != index);
            }
            // Kept in the host's order.
            let list = self.by_category.entry(self.games[index].category.clone()).or_default();
            let at = list.partition_point(|&i| i < index);
            list.insert(at, index);
        }
    }

    /// The title as the host listed it, before the user's changes.
    pub fn base_game(&self, id: &str) -> Option<&Game> {
        self.index.get(id).map(|&i| &self.base[i])
    }

    /// The art files known since the folder was last read, by name.
    pub fn art_names(&self) -> impl Iterator<Item = &String> {
        self.art.values()
    }

    /// The title with the user's changes.
    pub fn game(&self, id: &str) -> Option<&Game> {
        self.index.get(id).map(|&i| &self.games[i])
    }

    /// Read `art/` again (the art picker, the editor); titles are recomputed
    /// only when the files changed. Returns the PNG names, sorted.
    pub fn reload_art(&mut self, host: &mut dyn Host) -> Vec<String> {
        let art = art_index(host);
        let mut names: Vec<String> = art.values().cloned().collect();
        names.sort_by_cached_key(|n| n.to_lowercase());
        if art != self.art {
            self.art = art;
            self.rebuild();
        }
        names
    }
    /// The host's titles without the user's changes.
    pub fn base(&self) -> &[Game] {
        &self.base
    }

    pub fn games(&self) -> &[Game] {
        &self.games
    }

    /// Every category in tab order, hidden ones included (the category
    /// manager's list).
    pub fn all_categories(&self) -> &[Category] {
        &self.categories
    }

    /// The titles a category lists, as indexes into [`Library::games`]. A
    /// title that was uninstalled drops out of "Last Played".
    pub fn titles_of(&self, id: &str) -> Vec<usize> {
        match id {
            SEARCH_ID => search::search(&self.games, &self.search_term, |g| (&g.title, &g.id)),
            RECENT_ID => self.recent.iter().filter_map(|rid| self.games.iter().position(|g| *g.id == **rid)).collect(),
            FAVORITES_ID => (0..self.games.len()).filter(|&i| self.games[i].favorite).collect(),
            _ => self.by_category.get(id).cloned().unwrap_or_default(),
        }
    }

    fn count(&self, id: &str) -> usize {
        match id {
            SEARCH_ID | RECENT_ID | FAVORITES_ID => self.titles_of(id).len(),
            _ => self.by_category.get(id).map_or(0, Vec::len),
        }
    }

    /// The tab bar: categories not hidden that list a title (the user's own
    /// always), with the search tab in front while a search is open.
    pub fn tabs(&self) -> Vec<Category> {
        let mut tabs: Vec<Category> = Vec::new();
        if !self.search_term.is_empty() {
            tabs.push(search_category());
        }
        tabs.extend(
            self.categories
                .iter()
                .filter(|c| !self.config.hidden.contains(&c.id) && (c.custom || SHOW_EMPTY_CATEGORIES || self.count(&c.id) > 0))
                .cloned(),
        );
        tabs
    }

    /// The category config to save: every category's id in tab order.
    pub fn category_config(&self) -> CategoryConfig {
        CategoryConfig {
            custom: self.config.custom.clone(),
            order: self.categories.iter().map(|c| c.id.clone()).collect(),
            hidden: self.config.hidden.clone(),
        }
    }

    /// Replace a title's changes and recompute.
    pub fn set_override(&mut self, id: &str, change: TitleOverride) {
        self.overrides.set(id, change);
        self.rebuild();
    }
}

/// A data folder in memory, for tests.
#[cfg(test)]
pub(crate) mod testing {
    use std::collections::HashMap;

    use crate::host::{Event, Host, PictureKey, Status};

    #[derive(Default)]
    pub struct Files(pub HashMap<String, Vec<u8>>);

    impl Host for Files {
        fn time_us(&self) -> u64 {
            0
        }
        fn read(&mut self, path: &str) -> Option<Vec<u8>> {
            self.0.get(path).cloned()
        }
        fn write(&mut self, path: &str, data: &[u8]) -> bool {
            self.0.insert(path.into(), data.into());
            true
        }
        fn remove(&mut self, path: &str) -> bool {
            self.0.remove(path);
            true
        }
        fn list(&mut self, dir: &str) -> Vec<String> {
            let mut names: Vec<String> =
                self.0.keys().filter_map(|k| k.strip_prefix(&format!("{dir}/"))).map(String::from).collect();
            names.sort();
            names
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
    }
}

#[cfg(test)]
mod tests {
    use super::testing::Files;
    use super::*;

    fn info(id: &str, title: &str, platform: &str, console: Option<&str>) -> TitleInfo {
        TitleInfo { id: id.into(), title: title.into(), platform: platform.into(), adrenaline: false, console: console.map(String::from) }
    }

    fn library(files: &[(&str, &str)]) -> Library {
        let mut host = Files::default();
        for (path, text) in files {
            host.0.insert(path.to_string(), text.as_bytes().to_vec());
        }
        let mut library = Library::load(&mut host);
        library.set_titles(&[
            info("PCSA00069", "Gravity Daze", "vita", None),
            info("PCSB00245", "Neon Drift", "vita", None),
            info("VITASHELL", "VitaShell", "vita", None),
            info("NPXS10015", "Settings", "vita", None),
            info("ra0000001", "Tetris", "retro", Some("Nintendo - Game Boy")),
        ]);
        library
    }

    fn tab_ids(library: &Library) -> Vec<String> {
        library.tabs().into_iter().map(|c| c.id).collect()
    }

    #[test]
    fn tabs_leave_out_empty_builtins_and_keep_system_last() {
        let library = library(&[]);
        assert_eq!(tab_ids(&library), ["games", "homebrew", "retro-gb", "system"]);
    }

    #[test]
    fn overrides_move_rename_and_favorite_titles() {
        let library = library(&[
            ("categories.json", r#"{"custom":[{"id":"mine","label":"Mine"}],"order":[],"hidden":["homebrew"]}"#),
            ("titles/PCSA00069.json", r#"{"category":"mine","title":"Renamed","favorite":true}"#),
            ("titles/PCSB00245.json", r#"{"category":"deleted-one"}"#),
            ("recent.json", r#"["VITASHELL","GONE00000","PCSA00069"]"#),
        ]);
        assert_eq!(tab_ids(&library), ["smart-recent", "smart-favorites", "games", "retro-gb", "mine", "system"]);
        let titles = |id: &str| library.titles_of(id).into_iter().map(|i| library.games()[i].title.to_string()).collect::<Vec<_>>();
        assert_eq!(titles("mine"), ["Renamed"]);
        assert_eq!(titles("games"), ["Neon Drift"]);
        assert_eq!(titles(RECENT_ID), ["VitaShell", "Renamed"]);
        assert_eq!(titles(FAVORITES_ID), ["Renamed"]);
    }

    #[test]
    fn art_files_match_by_id_or_title_unless_the_icon_was_chosen() {
        let library = library(&[
            ("art/pcsa00069.PNG", ""),
            ("art/Neon Drift.png", ""),
            ("art/VitaShell.png", ""),
            ("titles/VITASHELL.json", r#"{"art":"icon"}"#),
        ]);
        let game = |id: &str| library.games().iter().find(|g| &*g.id == id).unwrap().clone();
        assert_eq!(game("PCSA00069").art.as_deref(), Some("pcsa00069.PNG"));
        assert!(game("PCSA00069").art_auto);
        assert_eq!(game("PCSB00245").art.as_deref(), Some("Neon Drift.png"));
        assert_eq!(game("VITASHELL").art, None);
        assert!(game("VITASHELL").art_icon);
    }

    #[test]
    fn a_patch_gives_what_a_full_rebuild_gives() {
        let mut library = library(&[("categories.json", r#"{"custom":[{"id":"mine","label":"Mine"}],"order":[],"hidden":[]}"#)]);
        library.patch("PCSB00245", |c| c.category = Some("mine".into()));
        library.patch("VITASHELL", |c| {
            c.title = Some("Shell".into());
            c.favorite = true;
        });
        library.patch("PCSA00069", |c| c.category = Some("homebrew".into()));
        library.patch("PCSA00069", |c| c.category = None);
        let patched: Vec<Game> = library.games().to_vec();
        let tabs: Vec<(String, Vec<usize>)> = library.all_categories().iter().map(|c| (c.id.clone(), library.titles_of(&c.id))).collect();
        library.rebuild();
        assert_eq!(library.games(), &patched[..]);
        for (id, titles) in tabs {
            assert_eq!(library.titles_of(&id), titles, "{id}");
        }
        assert_eq!(library.titles_of("mine").len(), 1);
    }

    #[test]
    fn search_tab_comes_first_while_a_term_is_set() {
        let mut library = library(&[]);
        library.search_term = "tet".into();
        assert_eq!(tab_ids(&library)[0], SEARCH_ID);
        assert_eq!(library.titles_of(SEARCH_ID).len(), 1);
    }
}

#[cfg(test)]
mod timing {
    use super::testing::Files;
    use super::*;

    /// `cargo test --release -- --ignored --nocapture rebuild_time`
    #[test]
    #[ignore]
    fn rebuild_time() {
        let consoles = ["Nintendo - Nintendo Entertainment System", "Nintendo - Super Nintendo Entertainment System", "Sega - Mega Drive - Genesis", "MAME 2003-Plus", "Neo Geo"];
        let mut infos: Vec<TitleInfo> = (0..70)
            .map(|i| TitleInfo { id: format!("PCSA{i:05}"), title: format!("Game {i}"), platform: "vita".into(), adrenaline: false, console: None })
            .collect();
        infos.extend((0..5709).map(|i| TitleInfo {
            id: format!("ra{i:07}"),
            title: format!("Some Game Title {i} (USA)"),
            platform: "retro".into(),
            adrenaline: false,
            console: Some(consoles[i % consoles.len()].into()),
        }));
        let mut host = Files::default();
        // As the user's data: 85 art files, 87 changed titles.
        for i in 0..85 {
            host.0.insert(format!("art/ra{i:07}-lr-box.png"), Vec::new());
        }
        for i in 0..87 {
            host.0.insert(format!("titles/ra{:07}.json", i * 7), br#"{"art":"x.png","favorite":true}"#.to_vec());
        }
        let mut library = Library::load(&mut host);
        let started = std::time::Instant::now();
        let games = load_catalog(&infos);
        let catalog = started.elapsed();
        let started = std::time::Instant::now();
        library.set_catalog(games);
        let set = started.elapsed();
        let started = std::time::Instant::now();
        library.rebuild();
        let rebuild = started.elapsed();
        let started = std::time::Instant::now();
        let tabs = library.tabs();
        let tab_time = started.elapsed();
        println!("load_catalog {catalog:?}, set_catalog {set:?}, rebuild {rebuild:?}, tabs {tab_time:?} ({} tabs)", tabs.len());
    }
}
