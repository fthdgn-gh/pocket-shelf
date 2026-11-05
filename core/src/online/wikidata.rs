//! Game details from Wikidata (CC0), built into one table per console by
//! `shelf-tools wikidata` and published as files of the `gamedb-data`
//! release, which the Vita downloads like libretro's databases.
//!
//! Wikidata has no ROM checksums: a game identified by libretro's database
//! (or its label) is found here by name ([`Index::find`]), compared as
//! [`base_name`] compares names, among the game's English and multilingual
//! labels and aliases.
//!
//! A table, `<file>.tsv`: a first line `#pocket-shelf-wikidata <version>
//! <built ms> <file>`, then one game per line, fields separated by tabs:
//! item id, name, aliases, year, genre ids, developers, publishers, game
//! mode ids, series, Wikipedia articles (`lang:title`); lists separated by
//! `|`. Genre and mode names, in the app's languages, are in `labels.tsv`:
//! item id, then en, tr, de, fr, es.
//!
//! **On the card** (`wikidata/`): the tables the library's titles need,
//! `labels.tsv`, and `found.tsv` ([`Matches`]), what each title was found
//! to be. The Game database run reads the release's `index.tsv` (each
//! table's build time) and downloads a table only when the release's is
//! newer; the host matches titles on a worker thread ([`match_titles`]).

use std::collections::HashMap;
use std::sync::Arc;

use super::gamedb::short_name;
use super::libretro::base_name;
use crate::library::consoles::console_category;

/// The first line's mark and the format's version.
const TABLE_MARK: &str = "#pocket-shelf-wikidata";
pub const TABLE_VERSION: u32 = 1;

/// The labels file, and the languages it holds, in its columns' order.
pub const LABELS_FILE: &str = "labels.tsv";
pub const LANGUAGES: [&str; 5] = ["en", "tr", "de", "fr", "es"];

/// The Wikipedias whose article titles a table keeps, for descriptions in
/// the app's languages (Japanese for games with no English article).
pub const WIKIS: [&str; 6] = ["en", "tr", "de", "fr", "es", "ja"];

/// Where the tables are published.
pub const RELEASE_URL: &str = "https://github.com/fthdgn-gh/pocket-shelf/releases/download/gamedb-data";

/// The consoles, by the database names RetroArch gives them, and the
/// Wikidata platforms ("platform", P400) their games are on. Several
/// databases share a table (every MAME version, PSP and its PSN titles);
/// a platform can have several items (Neo Geo and its arcade board).
pub const PLATFORMS: &[(&[&str], &[&str])] = &[
    (&["Nintendo - Nintendo Entertainment System"], &["Q172742"]),
    (&["Nintendo - Family Computer Disk System"], &["Q135321"]),
    (&["Nintendo - Super Nintendo Entertainment System"], &["Q183259"]),
    (&["Nintendo - Nintendo 64"], &["Q184839"]),
    (&["Nintendo - Game Boy"], &["Q186437"]),
    (&["Nintendo - Game Boy Color"], &["Q203992"]),
    (&["Nintendo - Game Boy Advance"], &["Q188642"]),
    (&["Nintendo - Nintendo DS"], &["Q170323"]),
    (&["Nintendo - Nintendo DSi"], &["Q637178"]),
    (&["Nintendo - Virtual Boy"], &["Q164651"]),
    (&["Handheld Electronic Game"], &["Q215034"]),
    (&["Sega - SG-1000"], &["Q1136956"]),
    (&["Sega - Master System - Mark III"], &["Q209868"]),
    (&["Sega - Game Gear"], &["Q751719"]),
    (&["Sega - Mega Drive - Genesis"], &["Q10676"]),
    (&["Sega - Mega-CD - Sega CD"], &["Q1047516"]),
    (&["Sega - 32X"], &["Q1063978"]),
    (&["Sega - Saturn"], &["Q200912"]),
    (&["Sega - Dreamcast"], &["Q184198"]),
    (&["Sony - PlayStation"], &["Q10677"]),
    (&["Sony - PlayStation Portable", "Sony - PlayStation Portable (PSN)"], &["Q170325"]),
    (&["Sony - PlayStation Vita"], &["Q188808"]),
    (&["NEC - PC Engine - TurboGrafx 16"], &["Q1057377"]),
    (&["NEC - PC Engine CD - TurboGrafx-CD"], &["Q10854461"]),
    (&["NEC - PC-FX"], &["Q1136902"]),
    (&["NEC - PC-98"], &["Q183505"]),
    (&["NEC - PC-8001 - PC-8801", "NEC - PC-88"], &["Q1338888"]),
    (&["SNK - Neo Geo", "Neo Geo"], &["Q1054350", "Q3338058"]),
    (&["SNK - Neo Geo CD"], &["Q2703883"]),
    (&["SNK - Neo Geo Pocket Color"], &["Q1977455"]),
    (&["Bandai - WonderSwan"], &["Q1065792"]),
    (&["Bandai - WonderSwan Color"], &["Q1048035"]),
    (&["Atari - 2600"], &["Q206261"]),
    (&["Atari - 5200"], &["Q743222"]),
    (&["Atari - 7800"], &["Q753600"]),
    (&["Atari - 8-bit Family"], &["Q249075"]),
    (&["Atari - Lynx"], &["Q753657"]),
    (&["Atari - Jaguar"], &["Q650601"]),
    (&["Atari - ST"], &["Q627302"]),
    (&["Coleco - ColecoVision"], &["Q1046862"]),
    (&["Mattel - Intellivision"], &["Q1061441"]),
    (&["Magnavox - Odyssey2"], &["Q576932"]),
    (&["The 3DO Company - 3DO"], &["Q229429"]),
    (&["Philips - CD-i"], &["Q1023103"]),
    (&["Microsoft - MSX", "Microsoft - MSX2"], &["Q853547", "Q11232203"]),
    (&["Commodore - 64"], &["Q99775"]),
    (&["Commodore - Amiga"], &["Q100047"]),
    (&["Commodore - CD32"], &["Q695161"]),
    (&["Commodore - CDTV"], &["Q955368"]),
    (&["Commodore - VIC-20"], &["Q918232"]),
    (&["Commodore - Plus-4"], &["Q868568"]),
    (&["Commodore - PET"], &["Q946661"]),
    (&["Sinclair - ZX Spectrum", "Sinclair - ZX Spectrum +3"], &["Q23882"]),
    (&["Amstrad - CPC"], &["Q478829"]),
    (&["Apple - II"], &["Q3017175", "Q201652"]),
    (&["DOS"], &["Q170434", "Q47604"]),
    (&["Sharp - X68000"], &["Q1758277"]),
    (&["Sharp - X1"], &["Q2710884"]),
    (&["Fujitsu - FM Towns"], &["Q531896"]),
    (&["Thomson - MOTO"], &["Q2396081", "Q55315688"]),
    (&["Mobile - J2ME"], &["Q193828"]),
    (&["FBNeo - Arcade Games", "MAME"], &["Q192851"]),
];

/// The table a database's games are looked up in, named as the app's tab
/// for the console is (`nes`, `arcade`, `psp`); Vita titles are `vita`.
pub fn file_for(database: &str) -> String {
    let database = database.trim();
    if database == "Sony - PlayStation Vita" {
        return "vita".into();
    }
    // A database sharing a platform's table goes by the platform's first.
    let first = PLATFORMS.iter().find(|(names, _)| names.contains(&database)).map_or(database, |(names, _)| names[0]);
    let id = console_category(first).id;
    id.strip_prefix("retro-").or_else(|| id.strip_prefix("builtin-")).unwrap_or(&id).to_string()
}

/// The table's file name and Wikidata items of each platform, once each.
pub fn tables() -> Vec<(String, &'static [&'static str])> {
    PLATFORMS.iter().map(|(databases, items)| (file_for(databases[0]), *items)).collect()
}

/// Where a console's table is published.
pub fn table_url(file: &str) -> String {
    format!("{RELEASE_URL}/{file}.tsv")
}

/// The release's list of tables: `file`, games, build time, one per line.
pub fn index_url() -> String {
    format!("{RELEASE_URL}/index.tsv")
}

/// Where the labels are published.
pub fn labels_url() -> String {
    format!("{RELEASE_URL}/{LABELS_FILE}")
}

/// Where a table is kept in the data folder.
pub fn table_path(file: &str) -> String {
    format!("wikidata/{file}.tsv")
}

/// Where the labels are kept in the data folder.
pub const LABELS_PATH: &str = "wikidata/labels.tsv";

/// What each title was found to be ([`Matches`]).
pub const FOUND_PATH: &str = "wikidata/found.tsv";

/// The table a listed title is looked up in: its game database's, when
/// that console has a table; `vita` for Vita titles. None for the rest
/// (PSM titles, a console Wikidata's tables leave out).
pub fn file_of(platform: &str, id: &str, console: Option<&str>) -> Option<String> {
    if platform == "vita" {
        return vita_game(id).then(|| "vita".into());
    }
    file_of_database(&super::gamedb::database_for(platform, id, console)?)
}

/// Whether a Vita title id is a game's: `PCSA`–`PCSH` and the Asian
/// `VCAS`, `VLJM` forms, then five digits. The system's apps (`NPXS`) and
/// homebrew (`VITASHELL`, `RTCW00001`) are left out, so a homebrew named
/// like a game is not taken for it.
pub fn vita_game(id: &str) -> bool {
    let b = id.as_bytes();
    let prefix = (b.len() == 9 && &b[..3] == b"PCS" && (b'A'..=b'H').contains(&b[3]))
        || (b.len() == 9 && b[0] == b'V' && matches!(b[1], b'C' | b'L') && matches!(b[2], b'A' | b'J' | b'K') && matches!(b[3], b'S' | b'M'));
    prefix && b[4..].iter().all(u8::is_ascii_digit)
}

/// The table of a libretro database's titles, when there is one.
pub fn file_of_database(database: &str) -> Option<String> {
    let file = file_for(database);
    published().iter().any(|f| *f == file).then_some(file)
}

/// [`file_of`] for a listed title.
pub fn file_of_game(game: &crate::library::catalog::Game) -> Option<String> {
    use crate::library::categories::Platform;
    let platform = match game.platform {
        Platform::Vita => "vita",
        Platform::Retro => "retro",
        Platform::Psp => "psp",
        Platform::Psx => "psx",
        Platform::Psm => return None,
    };
    file_of(platform, &game.id, game.console.as_deref())
}

/// The tables' file names, worked out once.
fn published() -> &'static [String] {
    static FILES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    FILES.get_or_init(|| tables().into_iter().map(|(f, _)| f).collect())
}

/// Whether a download for the file named `file` (`labels.tsv`, `nes.tsv`)
/// reads as one, so an error page never replaces a good file: a table by
/// its first line, the labels by theirs (an item and five names).
pub fn download_reads(file: &str, text: &str) -> bool {
    let first = text.split('\n').next().unwrap_or("");
    if file == LABELS_FILE {
        first.starts_with('Q') && first.split('\t').count() == 6
    } else {
        table_built(first).is_some()
    }
}

/// The release's `index.tsv`: each table's build time by file.
pub fn parse_index(text: &str) -> HashMap<String, u64> {
    text.lines()
        .filter_map(|line| {
            let mut f = line.split('\t');
            let file = f.next().filter(|f| !f.is_empty())?;
            let built = f.nth(1)?.trim().parse().ok()?;
            Some((file.to_string(), built))
        })
        .collect()
}

/// A table's build time from its first line; None for anything else (an
/// error page, another version).
pub fn table_built(first_line: &str) -> Option<u64> {
    let mut head = first_line.split_whitespace();
    if head.next()? != TABLE_MARK || head.next()?.parse::<u32>().ok()? != TABLE_VERSION {
        return None;
    }
    head.next()?.parse().ok()
}

/// What two names are compared by: the name without its brackets and with
/// a trailing article moved to the front ([`short_name`]), as [`base_name`]
/// compares it, so `Legend of Zelda, The - A Link to the Past (USA)` is
/// `The Legend of Zelda: A Link to the Past`.
pub fn name_key(name: &str) -> String {
    base_name(&short_name(name))
}

/// One game of a table.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Game {
    pub item: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub year: u16,
    /// Wikidata items, named in `labels.tsv`.
    pub genres: Vec<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    /// Wikidata items (single-player, multiplayer, co-op...).
    pub modes: Vec<String>,
    pub series: String,
    /// `(language, article title)`.
    pub wikis: Vec<(String, String)>,
}

/// A field on one line with no tabs or list separators.
fn clean(text: &str) -> String {
    text.chars().map(|c| if c.is_control() || c == '|' { ' ' } else { c }).collect::<String>().trim().to_string()
}

fn list(items: &[String]) -> String {
    items.iter().map(|i| clean(i)).filter(|i| !i.is_empty()).collect::<Vec<_>>().join("|")
}

fn split(field: &str) -> Vec<String> {
    field.split('|').filter(|s| !s.is_empty()).map(String::from).collect()
}

impl Game {
    fn write(&self, out: &mut String) {
        let wikis: Vec<String> = self.wikis.iter().map(|(lang, title)| format!("{lang}:{}", clean(title))).collect();
        let year = if self.year == 0 { String::new() } else { self.year.to_string() };
        let fields = [
            clean(&self.item),
            clean(&self.name),
            list(&self.aliases),
            year,
            list(&self.genres),
            list(&self.developers),
            list(&self.publishers),
            list(&self.modes),
            clean(&self.series),
            wikis.join("|"),
        ];
        out.push_str(&fields.join("\t"));
        out.push('\n');
    }

    fn read(line: &str) -> Option<Game> {
        let mut f = line.split('\t');
        let item = f.next()?.to_string();
        let name = f.next()?.to_string();
        if item.is_empty() || name.is_empty() {
            return None;
        }
        let mut next = || f.next().unwrap_or("");
        let aliases = split(next());
        let year = next().parse().unwrap_or(0);
        let genres = split(next());
        let developers = split(next());
        let publishers = split(next());
        let modes = split(next());
        let series = next().to_string();
        let wikis = split(next()).into_iter().filter_map(|w| w.split_once(':').map(|(l, t)| (l.to_string(), t.to_string()))).collect();
        Some(Game { item, name, aliases, year, genres, developers, publishers, modes, series, wikis })
    }
}

/// A console's table as written, games in item order.
pub fn render(file: &str, built_ms: u64, games: &[Game]) -> String {
    let mut out = format!("{TABLE_MARK} {TABLE_VERSION} {built_ms} {file}\n");
    for game in games {
        game.write(&mut out);
    }
    out
}

/// A console's games, by name for matching.
pub struct Index {
    pub built_ms: u64,
    pub games: Vec<Game>,
    by_name: HashMap<String, Vec<usize>>,
}

impl Index {
    pub fn parse(text: &str) -> Option<Index> {
        let mut lines = text.split('\n');
        let built_ms = table_built(lines.next()?)?;
        let mut index = Index { built_ms, games: Vec::new(), by_name: HashMap::new() };
        for line in lines {
            let Some(game) = Game::read(line) else { continue };
            let at = index.games.len();
            for name in std::iter::once(&game.name).chain(&game.aliases) {
                let key = name_key(name);
                if key.is_empty() {
                    continue;
                }
                let list = index.by_name.entry(key).or_default();
                if !list.contains(&at) {
                    list.push(at);
                }
            }
            index.games.push(game);
        }
        Some(index)
    }

    /// The game a name (a database name such as `Legend of Zelda, The - A
    /// Link to the Past (USA)`, or a playlist label) is; of several with
    /// that name, the one whose own name is it (not an alias), else the
    /// first.
    pub fn find(&self, name: &str) -> Option<&Game> {
        self.find_near(name, 0)
    }

    /// [`find`](Index::find), and of several games with the name, the one
    /// released in `year` first (0 when unknown): *Tetris* is a different
    /// game on the NES in 1989 than in 1988.
    pub fn find_near(&self, name: &str, year: u16) -> Option<&Game> {
        let key = name_key(name);
        let found = self.by_name.get(&key)?;
        let same_year = found.iter().find(|&&at| year > 0 && self.games[at].year == year);
        let own = found.iter().find(|&&at| name_key(&self.games[at].name) == key);
        same_year.or(own).or(found.first()).map(|&at| &self.games[at])
    }
}

/// Genre and mode names by item, in [`LANGUAGES`]' order.
pub fn render_labels(labels: &[(String, [String; 5])]) -> String {
    let mut out = String::new();
    for (item, names) in labels {
        let names: Vec<String> = names.iter().map(|n| clean(n)).collect();
        out.push_str(&format!("{}\t{}\n", clean(item), names.join("\t")));
    }
    out
}

pub fn parse_labels(text: &str) -> HashMap<String, [String; 5]> {
    let mut labels = HashMap::new();
    for line in text.split('\n') {
        let mut f = line.split('\t');
        let Some(item) = f.next().filter(|i| !i.is_empty()) else { continue };
        let names: [String; 5] = std::array::from_fn(|_| f.next().unwrap_or("").to_string());
        labels.insert(item.to_string(), names);
    }
    labels
}

// --- What each title was found to be ----------------------------------------------

/// Genre and mode names in [`LANGUAGES`]' order, shared by the games that
/// use them.
pub type Names = Arc<[String; 5]>;

/// `labels.tsv` with each name list shared.
pub fn shared_labels(text: &str) -> HashMap<String, Names> {
    parse_labels(text).into_iter().map(|(item, names)| (item, Arc::new(names))).collect()
}

/// Wikidata's items for single-player, multiplayer and co-op modes.
const SINGLE_PLAYER: &str = "Q208850";
const MULTIPLAYER: [&str; 2] = ["Q6895044", "Q1758804"];

/// A title's game in a table, with its genres' names.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Found {
    pub game: Game,
    /// The genres `labels.tsv` names, in the game's order.
    pub genres: Vec<Names>,
}

impl Found {
    pub fn new(game: Game, labels: &HashMap<String, Names>) -> Found {
        let genres = game.genres.iter().filter_map(|item| labels.get(item).cloned()).collect();
        Found { game, genres }
    }

    /// The first genre in the language at `language` of [`LANGUAGES`]
    /// (English when it has no name there), its first letter capitalized:
    /// Wikidata writes `platform game`.
    pub fn genre(&self, language: usize) -> Option<String> {
        let names = self.genres.first()?;
        let name = names.get(language).filter(|n| !n.is_empty()).unwrap_or(&names[0]);
        let mut chars = name.chars();
        let first = chars.next()?;
        Some(first.to_uppercase().chain(chars).collect())
    }

    /// Whether the game has a multiplayer or co-op mode (`Some(true)`), only
    /// a single-player one (`Some(false)`), or no modes listed.
    pub fn multiplayer(&self) -> Option<bool> {
        let modes = &self.game.modes;
        if modes.iter().any(|m| MULTIPLAYER.contains(&m.as_str())) {
            Some(true)
        } else if modes.iter().any(|m| m == SINGLE_PLAYER) {
            Some(false)
        } else {
            None
        }
    }
}

/// A title to look up: its id, the names to try in order (the database
/// name libretro gave it, then its own), and the year libretro gave.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Title {
    pub id: String,
    pub names: Vec<String>,
    pub year: u16,
}

/// Each title's game in the table, `(title id, game)` for those found.
pub fn match_titles(index: &Index, titles: &[Title]) -> Vec<(String, Game)> {
    titles
        .iter()
        .filter_map(|title| {
            let game = title.names.iter().filter(|n| !n.trim().is_empty()).find_map(|name| index.find_near(name, title.year))?;
            Some((title.id.clone(), game.clone()))
        })
        .collect()
}

/// `wikidata/found.tsv`: for each title looked up, its table, the table's
/// build time and the game's line of the table (empty when not found), one
/// title per line: `id`, `file`, `built`, then the game's fields. A title
/// not found is looked up again in a newer table only.
#[derive(Default)]
pub struct Matches {
    entries: HashMap<String, (String, u64, Option<Game>)>,
}

impl Matches {
    pub fn parse(text: &str) -> Matches {
        let mut matches = Matches::default();
        for line in text.split('\n') {
            let mut fields = line.splitn(4, '\t');
            let (Some(id), Some(file), Some(built), Some(rest)) = (fields.next(), fields.next(), fields.next(), fields.next()) else { continue };
            if id.is_empty() {
                continue;
            }
            let game = Game::read(rest);
            matches.entries.insert(id.to_string(), (file.to_string(), built.parse().unwrap_or(0), game));
        }
        matches
    }

    /// One title per line, in id order.
    pub fn render(&self) -> String {
        let mut ids: Vec<&String> = self.entries.keys().collect();
        ids.sort();
        let mut out = String::new();
        for id in ids {
            let (file, built, game) = &self.entries[id];
            out.push_str(&format!("{}\t{}\t{built}\t", clean(id), clean(file)));
            match game {
                Some(game) => game.write(&mut out),
                None => out.push('\n'),
            }
        }
        out
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<&Game> {
        self.entries.get(id)?.2.as_ref()
    }

    /// Whether a title has not been looked up in `file`'s table built at
    /// `built_ms` or later.
    pub fn wants(&self, id: &str, file: &str, built_ms: u64) -> bool {
        match self.entries.get(id) {
            None => true,
            Some((f, built, game)) => f != file || (game.is_none() && *built < built_ms),
        }
    }

    /// A run over `titles`: those `found` with their games, the rest as
    /// looked up and not found. The aliases are left out: only matching
    /// reads them, and the file is read at each start.
    pub fn record(&mut self, file: &str, built_ms: u64, titles: &[Title], found: &[(String, Game)]) {
        let found: HashMap<&str, &Game> = found.iter().map(|(id, g)| (id.as_str(), g)).collect();
        for title in titles {
            let game = found.get(title.id.as_str()).map(|g| Game { aliases: Vec::new(), ..(*g).clone() });
            self.entries.insert(title.id.clone(), (file.to_string(), built_ms, game));
        }
    }

    /// Each title's game from this file, named with `labels` (the host does
    /// this on its thread before
    /// [`load_catalog`](crate::library::catalog::load_catalog)).
    pub fn apply(&self, infos: &mut [crate::host::TitleInfo], labels: &HashMap<String, Names>) {
        for info in infos {
            info.wiki = self.get(&info.id).map(|game| Arc::new(Found::new(game.clone(), labels)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zelda() -> Game {
        Game {
            item: "Q14625".into(),
            name: "The Legend of Zelda: A Link to the Past".into(),
            aliases: vec!["Zelda no Densetsu: Kamigami no Triforce".into(), "A Link to the Past".into()],
            year: 1991,
            genres: vec!["Q1342372".into()],
            developers: vec!["Nintendo EAD".into()],
            publishers: vec!["Nintendo".into()],
            modes: vec!["Q208850".into()],
            series: "The Legend of Zelda".into(),
            wikis: vec![("en".into(), "The Legend of Zelda: A Link to the Past".into()), ("ja".into(), "ゼルダの伝説 神々のトライフォース".into())],
        }
    }

    #[test]
    fn a_table_round_trips_and_finds_games_by_database_name() {
        let other = Game { item: "Q1".into(), name: "A Link to the Past".into(), ..Default::default() };
        let text = render("snes", 1_700_000_000_000, &[zelda(), other]);
        assert!(text.starts_with("#pocket-shelf-wikidata 1 1700000000000 snes\nQ14625\tThe Legend of Zelda: A Link to the Past\t"));
        let index = Index::parse(&text).unwrap();
        assert_eq!(index.games[0], zelda());
        assert_eq!(index.find("Legend of Zelda, The - A Link to the Past (USA)").map(|g| g.item.as_str()), Some("Q14625"));
        assert_eq!(index.find("Zelda no Densetsu - Kamigami no Triforce (Japan)").map(|g| g.item.as_str()), Some("Q14625"));
        // Its own name wins over another game's alias.
        assert_eq!(index.find("A Link to the Past (Europe)").map(|g| g.item.as_str()), Some("Q1"));
        assert!(index.find("Frogger").is_none());
        assert!(Index::parse("#pocket-shelf-wikidata 2 0 snes\n").is_none());
    }

    #[test]
    fn labels_round_trip() {
        let labels = vec![("Q1342372".to_string(), ["action-adventure game".into(), "aksiyon-macera oyunu".into(), "Action-Adventure".into(), String::new(), "videojuego de acción-aventura".into()])];
        let parsed = parse_labels(&render_labels(&labels));
        assert_eq!(parsed["Q1342372"][2], "Action-Adventure");
        assert_eq!(parsed["Q1342372"][3], "");
    }

    #[test]
    fn tables_are_named_as_the_consoles_tabs() {
        assert_eq!(file_for("Nintendo - Super Nintendo Entertainment System"), "snes");
        assert_eq!(file_for("MAME 2003-Plus"), "arcade");
        assert_eq!(file_for("FBNeo - Arcade Games"), "arcade");
        assert_eq!(file_for("Sony - PlayStation"), "psx");
        assert_eq!(file_for("Sony - PlayStation Portable (PSN)"), "psp");
        assert_eq!(file_for("Sony - PlayStation Vita"), "vita");
        assert_eq!(file_for("Neo Geo"), "neogeo");
        assert_eq!(file_for("Atari - 8-bit Family"), "atari-8-bit-family");
        let files: Vec<String> = tables().into_iter().map(|(f, _)| f).collect();
        let mut unique = files.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), files.len(), "one table per platform: {files:?}");
        for (databases, _) in PLATFORMS {
            assert!(databases.iter().all(|d| file_for(d) == file_for(databases[0])), "{databases:?} share a table");
        }
        assert_eq!(table_url("nes"), format!("{RELEASE_URL}/nes.tsv"));
    }

    #[test]
    fn titles_match_by_database_name_then_label_and_round_trip_in_found_tsv() {
        let tetris = |item: &str, year| Game { item: item.into(), name: "Tetris".into(), year, ..Default::default() };
        let text = render("nes", 1_700_000_000_000, &[zelda(), tetris("Q1", 1988), tetris("Q2", 1989)]);
        let index = Index::parse(&text).unwrap();
        let titles = [
            Title { id: "a".into(), names: vec!["Legend of Zelda, The - A Link to the Past (USA)".into(), "zelda3".into()], year: 0 },
            Title { id: "b".into(), names: vec![String::new(), "Tetris (USA)".into()], year: 1989 },
            Title { id: "c".into(), names: vec!["Frogger (USA)".into()], year: 0 },
        ];
        let found = match_titles(&index, &titles);
        assert_eq!(found.iter().map(|(id, g)| (id.as_str(), g.item.as_str())).collect::<Vec<_>>(), [("a", "Q14625"), ("b", "Q2")], "the 1989 Tetris");

        let mut matches = Matches::default();
        matches.record("nes", 5, &titles, &found);
        let again = Matches::parse(&matches.render());
        assert_eq!(again.len(), 3);
        assert_eq!(again.get("a"), Some(&Game { aliases: Vec::new(), ..zelda() }));
        assert_eq!(again.get("c"), None);
        // A miss is looked up again in a newer table only; another table
        // always.
        assert!(!again.wants("c", "nes", 5) && again.wants("c", "nes", 6) && again.wants("a", "snes", 0) && again.wants("d", "nes", 0));
        assert!(!again.wants("a", "nes", 6), "a match is kept");
        assert_eq!(table_built("#pocket-shelf-wikidata 1 42 nes"), Some(42));
        assert_eq!(table_built("<!DOCTYPE html>"), None);
        assert!(download_reads("labels.tsv", "Q1004\tcomic\tçizgi roman\tComic\tbande dessinée\thistorieta\nQ2\t"));
        assert!(download_reads("nes.tsv", &text));
        assert!(!download_reads("labels.tsv", &text) && !download_reads("nes.tsv", "Q1004\tcomic\t\t\t\t\n"));
        assert!(!download_reads("nes.tsv", "<!DOCTYPE html>"));
        assert_eq!(parse_index("nes\t1211\t42\nvita\t928\t43\n\n").get("vita"), Some(&43));
    }

    #[test]
    fn found_games_name_their_genre_and_modes_in_the_apps_language() {
        let labels = shared_labels("Q1342372\taction-adventure game\taksiyon-macera oyunu\tAction-Adventure\t\tvideojuego de acción-aventura\n");
        let found = Found::new(zelda(), &labels);
        assert_eq!(found.genre(0).as_deref(), Some("Action-adventure game"));
        assert_eq!(found.genre(2).as_deref(), Some("Action-Adventure"));
        assert_eq!(found.genre(3).as_deref(), Some("Action-adventure game"), "English when French has no name");
        assert_eq!(found.multiplayer(), Some(false));
        let co_op = Found::new(Game { modes: vec!["Q208850".into(), "Q1758804".into()], ..zelda() }, &labels);
        assert_eq!(co_op.multiplayer(), Some(true));
        assert_eq!(Found::new(Game { genres: vec!["Q404".into()], modes: Vec::new(), ..zelda() }, &labels).genre(0), None);
    }

    #[test]
    fn titles_go_to_their_consoles_table() {
        assert_eq!(file_of("retro", "x", Some("Nintendo - Super Nintendo Entertainment System")).as_deref(), Some("snes"));
        assert_eq!(file_of("retro", "x", Some("MAME 2003-Plus")).as_deref(), Some("arcade"));
        assert_eq!(file_of("psp", "NPUZ00001", None).as_deref(), Some("psp"));
        assert_eq!(file_of("psx", "SCUS94163", None).as_deref(), Some("psx"));
        assert_eq!(file_of("vita", "PCSA00069", None).as_deref(), Some("vita"));
        assert_eq!(file_of("vita", "NPXS10001", None), None, "a system app");
        assert_eq!(file_of("vita", "VITASHELL", None), None, "homebrew");
        assert_eq!(file_of("vita", "RTCW00001", None), None, "homebrew");
        assert_eq!(file_of("vita", "VLJS00055", None).as_deref(), Some("vita"));
        assert_eq!(file_of("psm", "NPOA00013", None), None);
        assert_eq!(file_of("retro", "x", Some("Nintendo - Pokemon Mini")), None, "no table");
    }
}
