//! libretro's thumbnail server (https://thumbnails.libretro.com), the one RetroArch's "Download
//! Thumbnails" uses. One folder per console, named after its database, each
//! with `Named_Boxarts`, `Named_Snaps` and `Named_Titles`; a picture is
//! `<name>.png`, the name in No-Intro's style with &*/:`"<>?\| written as _.
//!
//! A playlist's label is that name when RetroArch's scanner made it from a
//! No-Intro set. Older GoodTools sets give "Battleship (U)" or "Frogger (UJ)
//! [!]"; for those the console's box art names are fetched once (the host
//! keeps only the names, in `libretro/<console>.txt`) and a name is matched
//! by its words and region. On the user's 5,811 games: 510 by the exact name,
//! 4,042 (70%) with the matching.

use std::collections::{HashMap, HashSet};

use super::encode_uri_component;
use crate::library::categories::Platform;

pub const SERVER: &str = "https://thumbnails.libretro.com";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Thumbnail {
    Boxarts,
    Snaps,
    Titles,
}

impl Thumbnail {
    fn folder(self) -> &'static str {
        match self {
            Thumbnail::Boxarts => "Named_Boxarts",
            Thumbnail::Snaps => "Named_Snaps",
            Thumbnail::Titles => "Named_Titles",
        }
    }
}

/// The server's folder for a console's database name: every MAME version
/// shares "MAME"; the user's "Neo Geo" playlist is "SNK - Neo Geo".
pub fn server_folder(console: &str) -> String {
    let name = console.trim();
    if name.starts_with("MAME") {
        return "MAME".into();
    }
    match name {
        "Neo Geo" => "SNK - Neo Geo".into(),
        _ => name.into(),
    }
}

/// The server's folder for a title: a RetroArch game's console, or the
/// PlayStation and PSP folders for PSX and PSP games; Vita titles have none
/// (its folder holds a handful of pictures).
pub fn libretro_folder(console: Option<&str>, platform: Platform) -> Option<String> {
    if let Some(console) = console {
        return Some(server_folder(console));
    }
    match platform {
        Platform::Psx => Some("Sony - PlayStation".into()),
        Platform::Psp => Some("Sony - PlayStation Portable".into()),
        _ => None,
    }
}

/// A label as a picture's file name, without `.png`.
pub fn scrubbed(label: &str) -> String {
    label.chars().map(|c| if "&*/:`\"<>?\\|".contains(c) { '_' } else { c }).collect()
}

pub fn thumbnail_url(folder: &str, kind: Thumbnail, name: &str) -> String {
    format!("{SERVER}/{}/{}/{}.png", encode_uri_component(folder), kind.folder(), encode_uri_component(name))
}

/// A console's box art listing page.
pub fn listing_url(folder: &str) -> String {
    format!("{SERVER}/{}/Named_Boxarts/", encode_uri_component(folder))
}

/// Where a console's names are kept under the data folder.
pub fn listing_path(folder: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for c in folder.chars() {
        if c.is_ascii_alphanumeric() {
            if dash && !slug.is_empty() {
                slug.push('-');
            }
            dash = false;
            slug.push(c);
        } else {
            dash = true;
        }
    }
    slug.truncate(56);
    let slug = if slug.is_empty() { "console".to_string() } else { slug };
    format!("libretro/{slug}.txt")
}

/// A console's names, each with its base for matching.
pub struct Listing {
    names: HashSet<String>,
    by_base: HashMap<String, Vec<String>>,
    all: Vec<(String, String)>,
}

impl Listing {
    pub fn new(names: impl IntoIterator<Item = String>) -> Listing {
        let mut listing = Listing { names: HashSet::new(), by_base: HashMap::new(), all: Vec::new() };
        for name in names {
            let base = base_name(&name);
            listing.by_base.entry(base.clone()).or_default().push(name.clone());
            listing.all.push((name.clone(), base));
            listing.names.insert(name);
        }
        listing
    }

    pub fn len(&self) -> usize {
        self.all.len()
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    /// The names of a listing file (one per line); `None` when empty.
    pub fn parse(text: &str) -> Option<Listing> {
        let names: Vec<String> = text.split('\n').filter(|n| !n.is_empty()).map(String::from).collect();
        (!names.is_empty()).then(|| Listing::new(names))
    }
}

/// Accents folded to plain letters.
fn fold(c: char) -> &'static str {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
        'ç' => "c",
        'è' | 'é' | 'ê' | 'ë' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'ı' => "i",
        'ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => "o",
        'ù' | 'ú' | 'û' | 'ü' => "u",
        'ý' | 'ÿ' => "y",
        'ß' => "ss",
        'ş' => "s",
        'ğ' => "g",
        _ => "",
    }
}

/// What two names are compared by: the name before its first bracket, with
/// ", The" moved to the front, accents folded, "&", "_" and "and" dropped,
/// letters and digits only.
pub fn base_name(name: &str) -> String {
    let cut = name.find(['(', '[']).map_or(name, |i| &name[..i]).trim();
    // ", The" / ", A" / ", An" at the end moves to the front.
    let mut text = cut.to_string();
    for article in ["The", "A", "An"] {
        let suffix = format!(", {article}");
        if text.len() > suffix.len() && text[text.len() - suffix.len()..].eq_ignore_ascii_case(&suffix) {
            let head = text[..text.len() - suffix.len()].to_string();
            let found = text[text.len() - article.len()..].to_string();
            text = format!("{found} {head}");
            break;
        }
    }
    let lower = text.to_lowercase();
    let mut folded = String::new();
    for c in lower.chars() {
        let plain = fold(c);
        if plain.is_empty() {
            folded.push(c);
        } else {
            folded.push_str(plain);
        }
    }
    let spaced: String = folded.chars().map(|c| if c == '&' || c == '_' { ' ' } else { c }).collect();
    // "and" as a word is dropped.
    let words: Vec<&str> = spaced.split(|c: char| !c.is_alphanumeric()).collect();
    let mut out = String::new();
    for word in words {
        if word == "and" {
            continue;
        }
        out.extend(word.chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }
    out
}

/// GoodTools' region codes, as No-Intro writes the region.
fn good_tools(code: &str) -> Option<&'static str> {
    Some(match code {
        "U" | "UE" | "JU" | "UJ" | "JUE" => "USA",
        "E" => "Europe",
        "J" => "Japan",
        "W" => "World",
        "A" => "Australia",
        "B" => "Brazil",
        "K" => "Korea",
        "F" => "France",
        "G" => "Germany",
        _ => return None,
    })
}

/// Regions preferred when a name says none, best first.
const PREFERRED: [&str; 4] = ["USA", "World", "Europe", "Japan"];

/// Releases that are not the game's box: matched only by a label that is
/// one too.
fn special(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["(beta", "(proto", "(sample", "(demo", "(hack", "(pirate", "(unl", "(aftermarket"].iter().any(|tag| lower.contains(tag))
}

/// The regions a name gives, in either style.
pub fn regions_of(name: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    let mut rest = name;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')') else { break };
        let tag = &rest[open + 1..open + close];
        if let Some(code) = good_tools(tag) {
            found.push(code);
        }
        for part in tag.split(',') {
            if let Some(region) = PREFERRED.iter().find(|r| **r == part.trim()) {
                found.push(region);
            }
        }
        rest = &rest[open + close + 1..];
    }
    found
}

/// Of names sharing a base, the one that best fits `label`'s region and kind.
fn best_of(label: &str, names: &[String]) -> Option<String> {
    let label_special = special(label);
    let pool: Vec<&String> = names.iter().filter(|n| label_special || !special(n)).collect();
    let choices: Vec<&String> = if pool.is_empty() { names.iter().collect() } else { pool };
    let wanted: Vec<&str> = regions_of(label).into_iter().chain(PREFERRED).collect();
    let rank = |name: &str| regions_of(name).iter().filter_map(|r| wanted.iter().position(|w| w == r)).min().unwrap_or(wanted.len());
    let mut best: Option<&String> = None;
    for name in choices {
        let better = match best {
            None => true,
            Some(b) => rank(name) < rank(b) || (rank(name) == rank(b) && name.chars().count() < b.chars().count()),
        };
        if better {
            best = Some(name);
        }
    }
    best.cloned()
}

/// The listing's name for a label: the label itself when the server has it,
/// else the best match.
pub fn match_name(label: &str, listing: &Listing) -> Option<String> {
    let exact = scrubbed(label);
    if listing.names.contains(&exact) {
        return Some(exact);
    }
    listing.by_base.get(&base_name(label)).and_then(|same| best_of(label, same))
}

/// Names for a typed term, best first, at most `max`: the names matching it
/// as `match_name` would, then those holding every word of it, shorter first.
pub fn search_names(term: &str, listing: &Listing, max: usize) -> Vec<String> {
    let base = base_name(term);
    if base.is_empty() {
        return Vec::new();
    }
    let mut results = Vec::new();
    let same = listing.by_base.get(&base).cloned().unwrap_or_default();
    let first = best_of(term, &same);
    if let Some(first) = &first {
        results.push(first.clone());
    }
    results.extend(same.iter().filter(|n| Some(*n) != first.as_ref()).cloned());
    let lower = term.to_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit())).filter(|w| w.len() > 1 && *w != "the" && *w != "and").collect();
    let mut more: Vec<&String> = listing
        .all
        .iter()
        .filter(|(_, b)| *b != base)
        .filter(|(_, b)| if words.is_empty() { b.contains(&base) } else { words.iter().all(|w| b.contains(w)) })
        .map(|(n, _)| n)
        .collect();
    more.sort_by(|a, b| a.chars().count().cmp(&b.chars().count()).then_with(|| a.cmp(b)));
    results.extend(more.into_iter().cloned());
    results.truncate(max);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing() -> Listing {
        Listing::new(
            [
                "Battleship (USA)",
                "Battleship (Europe)",
                "Frogger (USA)",
                "Frogger (Japan)",
                "Frogger (USA) (Beta)",
                "Adventures of Batman _ Robin, The (USA)",
                "Secret of Monkey Island, The (USA) (RE)",
                "Pokemon - Ruby Version (USA, Europe) (Rev 2)",
                "Stunt Race FX (USA)",
                "Stunt Race FX (Europe)",
                "Super Mario World (USA)",
                "Super Mario World 2 - Yoshi's Island (USA)",
                "Mario Paint (USA)",
            ]
            .map(String::from),
        )
    }

    #[test]
    fn bases_ignore_regions_tags_articles_accents_and_punctuation() {
        assert_eq!(base_name("Battleship (U)"), base_name("Battleship (USA)"));
        assert_eq!(base_name("Frogger (UJ) [!]"), "frogger");
        assert_eq!(base_name("Adventures of Batman & Robin, The (E)"), base_name("Adventures of Batman _ Robin, The (USA)"));
        assert_eq!(base_name("André Agassi Tennis (U)"), "andreagassitennis");
    }

    #[test]
    fn regions_come_from_either_style() {
        assert_eq!(regions_of("Battleship (U)"), ["USA"]);
        assert_eq!(regions_of("Pele's World Tournament Soccer (JUE) [!]"), ["USA"]);
        assert_eq!(regions_of("Ninja Gaiden (USA, Europe, Brazil) (En)"), ["USA", "Europe"]);
    }

    #[test]
    fn labels_become_file_names_as_retroarch_scrubs_them() {
        assert_eq!(scrubbed("Q*bert: The Arcade?"), "Q_bert_ The Arcade_");
        assert_eq!(
            thumbnail_url("Nintendo - Game Boy", Thumbnail::Snaps, "Catrap (USA)"),
            "https://thumbnails.libretro.com/Nintendo%20-%20Game%20Boy/Named_Snaps/Catrap%20(USA).png"
        );
        assert_eq!(server_folder("MAME 2003-Plus"), "MAME");
        assert_eq!(server_folder("Neo Geo"), "SNK - Neo Geo");
        assert_eq!(listing_path("Nintendo - Super Nintendo Entertainment System"), "libretro/Nintendo-Super-Nintendo-Entertainment-System.txt");
    }

    #[test]
    fn psx_and_psp_have_the_playstation_folders() {
        assert_eq!(libretro_folder(None, Platform::Psx).as_deref(), Some("Sony - PlayStation"));
        assert_eq!(libretro_folder(None, Platform::Psp).as_deref(), Some("Sony - PlayStation Portable"));
        assert_eq!(libretro_folder(None, Platform::Vita), None);
        assert_eq!(libretro_folder(Some("MAME 2003-Plus"), Platform::Retro).as_deref(), Some("MAME"));
    }

    #[test]
    fn match_name_finds_the_label_or_its_best_match() {
        let l = listing();
        assert_eq!(match_name("Stunt Race FX (Europe)", &l).as_deref(), Some("Stunt Race FX (Europe)"));
        assert_eq!(match_name("Battleship (U)", &l).as_deref(), Some("Battleship (USA)"));
        assert_eq!(match_name("Battleship (E)", &l).as_deref(), Some("Battleship (Europe)"));
        assert_eq!(match_name("Frogger (J) [!]", &l).as_deref(), Some("Frogger (Japan)"));
        assert_eq!(match_name("Stunt Race FX", &l).as_deref(), Some("Stunt Race FX (USA)"));
        assert_eq!(match_name("Adventures of Batman & Robin, The", &l).as_deref(), Some("Adventures of Batman _ Robin, The (USA)"));
        assert_eq!(match_name("Frogger", &l).as_deref(), Some("Frogger (USA)"));
        assert_eq!(match_name("Boogerman", &l), None);
    }

    #[test]
    fn search_names_puts_the_match_first() {
        let l = listing();
        assert_eq!(search_names("mario world", &l, 30), ["Super Mario World (USA)", "Super Mario World 2 - Yoshi's Island (USA)"]);
        assert_eq!(search_names("Super Mario World", &l, 30)[0], "Super Mario World (USA)");
        assert!(search_names("", &l, 30).is_empty());
    }

    /// With the user's playlists and the server's listings in `.samples`
    /// (not in git): most of the library finds a box art.
    #[test]
    fn the_sample_library_finds_most_of_its_box_art() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.samples");
        let (playlists, listings) = (root.join("retroarch-playlists"), root.join("libretro-listings"));
        let (Ok(files), true) = (std::fs::read_dir(&playlists), listings.exists()) else { return };
        let (mut games, mut found) = (0, 0);
        for file in files.flatten() {
            let name = file.file_name().to_string_lossy().to_string();
            let Some(console) = name.strip_suffix(".lpl") else { continue };
            let folder = server_folder(console);
            let page = listings.join(format!("{}.html", folder.replace([' ', '/'], "_")));
            let Ok(html) = std::fs::read_to_string(&page) else { continue };
            let names: Vec<String> = html
                .split("href=\"")
                .skip(1)
                .filter_map(|part| part.split('"').next())
                .filter_map(|href| href.strip_suffix(".png"))
                .map(percent_decode)
                .collect();
            let listing = Listing::new(names);
            let Some(text) = super::super::tests_support::read_playlist(&file.path()) else { continue };
            for label in text {
                games += 1;
                if match_name(&label, &listing).is_some() {
                    found += 1;
                }
            }
        }
        if games == 0 {
            return;
        }
        eprintln!("libretro sample: {found} of {games} games find a box art");
        assert!(games > 5000, "{games}");
        assert!(found as f32 / games as f32 > 0.68, "{found} of {games}");
    }

    fn percent_decode(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(b) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                    out.push(b);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }
}

#[cfg(test)]
mod timing {
    /// `LIBRETRO_LISTING=<names file> cargo test --release -p pocket-core listing_timing -- --nocapture`
    #[test]
    fn listing_timing() {
        let Ok(path) = std::env::var("LIBRETRO_LISTING") else { return };
        let text = std::fs::read_to_string(path).unwrap();
        let start = std::time::Instant::now();
        let listing = super::Listing::parse(&text).unwrap();
        let parsed = start.elapsed();
        let start = std::time::Instant::now();
        let names = super::search_names("Crash Bandicoot", &listing, 30);
        println!("parse {:?}, search {:?}, {} names, first {:?}", parsed, start.elapsed(), listing.all.len(), names.first());
    }
}
