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

use std::collections::HashMap;

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
        let mut head = lines.next()?.split_whitespace();
        if head.next()? != TABLE_MARK || head.next()?.parse::<u32>().ok()? != TABLE_VERSION {
            return None;
        }
        let built_ms = head.next()?.parse().ok()?;
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
        let key = name_key(name);
        let found = self.by_name.get(&key)?;
        let own = found.iter().find(|&&at| name_key(&self.games[at].name) == key);
        own.or(found.first()).map(|&at| &self.games[at])
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
}
