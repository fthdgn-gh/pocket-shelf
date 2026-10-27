//! The installed titles as the launcher lists them (`src/catalog.ts`).

use std::collections::HashMap;
use std::sync::Arc;

use super::categories::{category_of, Platform};
use super::consoles::console_category;
use crate::host::TitleInfo;

/// How many tints titles cycle through.
pub const TINT_COUNT: usize = 6;
/// The tints as `0xRRGGBB`, for a title whose icon gives no color.
pub const TINT_COLORS: [u32; TINT_COUNT] = [0xf59e0b, 0x10b981, 0x3b82f6, 0x8b5cf6, 0xec4899, 0x06b6d4];

/// One title as the launcher shows it. The strings are shared (`Arc`): the
/// library copies every title at each change, and a copy of a shared string
/// is a count, not an allocation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Game {
    pub title: Arc<str>,
    /// Title id (`PCSA00069`); `ra` and seven digits for a RetroArch game.
    pub id: Arc<str>,
    pub platform: Platform,
    pub category: Arc<str>,
    /// Index into [`TINT_COLORS`].
    pub tint: u8,
    /// A PSP or PS1 title with no bubble, started through Adrenaline.
    pub adrenaline: bool,
    /// A RetroArch game's console database name (`Nintendo - Game Boy`).
    pub console: Option<Arc<str>>,
    pub favorite: bool,
    /// Box art: a file name in `art/`.
    pub art: Option<String>,
    /// `art` was found by name, not chosen by the user.
    pub art_auto: bool,
    /// The user chose the title's own icon over a matching art file.
    pub art_icon: bool,
    /// The user's backdrop: a file in `backdrops/`, or "none".
    pub backdrop: Option<String>,
}

impl Game {
    /// The id the "Detailed" views show: "RetroArch" for a RetroArch game,
    /// whose id is a hash.
    pub fn shown_id(&self) -> &str {
        if self.console.is_some() {
            Platform::Retro.genre()
        } else {
            &self.id
        }
    }
}

fn is_mark(ch: char) -> bool {
    matches!(ch, '®' | '™' | '©' | '℠')
}

/// The title without trademark marks (the font has none), runs of spaces as
/// one, trimmed. A mark between two word characters with no space
/// ("PlayStation®Store") becomes a space.
pub fn clean_title(title: &str) -> String {
    // Most titles have nothing to clean: no marks, single spaces, no space
    // at either end.
    let plain = !title.starts_with(' ')
        && !title.ends_with(' ')
        && !title.contains("  ")
        && !title.chars().any(|ch| is_mark(ch) || (ch.is_whitespace() && ch != ' '));
    if plain {
        return title.to_string();
    }
    let word = |ch: Option<&char>| ch.is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_');
    let chars: Vec<char> = title.chars().collect();
    let mut out = String::with_capacity(title.len());
    for (i, &ch) in chars.iter().enumerate() {
        if is_mark(ch) {
            if i > 0 && word(chars.get(i - 1)) && word(chars.get(i + 1)) {
                out.push(' ');
            }
        } else {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

// Adrenaline is installed as PSPEMUCFW; the bubbles it makes for single
// games use the same prefix with a number (PSPEMU001).
const ADRENALINE_ID: &str = "PSPEMUCFW";
const ADRENALINE_BUBBLE_PREFIX: &str = "PSPEMU";

/// The system applications with a bubble on the home screen.
const SYSTEM_APPS: [&str; 16] = [
    "NPXS10000", // near
    "NPXS10001", // Party
    "NPXS10002", // PlayStation Store
    "NPXS10003", // Internet Browser
    "NPXS10004", // Photos
    "NPXS10006", // Friends
    "NPXS10008", // Trophy Collection
    "NPXS10009", // Music
    "NPXS10010", // Videos
    "NPXS10014", // Messages
    "NPXS10015", // Settings
    "NPXS10026", // Content Manager
    "NPXS10072", // Email
    "NPXS10091", // Calendar
    "NPXS10094", // Parental Controls
    "NPXS10098", // PS4 Link
];

/// Whether an installed title belongs in the launcher: not Adrenaline's game
/// bubbles (Adrenaline itself stays), and of the system applications only
/// those in `SYSTEM_APPS`.
pub fn is_listed(id: &str) -> bool {
    if id.starts_with("NPXS") {
        return SYSTEM_APPS.contains(&id);
    }
    id == ADRENALINE_ID || !id.starts_with(ADRENALINE_BUBBLE_PREFIX)
}

/// The host's titles less those `is_listed` leaves out, as titles with no
/// changes.
pub fn load_catalog(infos: &[TitleInfo]) -> Vec<Game> {
    // Thousands of RetroArch games share a few consoles: each console's
    // category is worked out once.
    // Categories and console names are shared by many titles: one string
    // each.
    let mut consoles: HashMap<&str, (Arc<str>, Arc<str>)> = HashMap::new();
    let mut categories: HashMap<String, Arc<str>> = HashMap::new();
    infos
        .iter()
        .filter(|info| is_listed(&info.id))
        .enumerate()
        .map(|(index, info)| {
            let platform = Platform::from_key(&info.platform);
            let (category, console) = match info.console.as_deref().filter(|_| platform == Platform::Retro) {
                Some(database) => {
                    let (category, name) =
                        consoles.entry(database).or_insert_with(|| (console_category(database).id.into(), database.into()));
                    (category.clone(), Some(name.clone()))
                }
                None => {
                    let id = category_of(&info.id, platform);
                    (categories.entry(id).or_insert_with_key(|id| id.as_str().into()).clone(), None)
                }
            };
            Game {
                title: clean_title(&info.title).into(),
                id: info.id.as_str().into(),
                platform,
                category,
                tint: (index % TINT_COUNT) as u8,
                adrenaline: info.adrenaline,
                console,
                favorite: false,
                art: None,
                art_auto: false,
                art_icon: false,
                backdrop: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_title_drops_marks_and_collapses_spaces() {
        assert_eq!(clean_title("Uncharted™:  Golden Abyss®"), "Uncharted: Golden Abyss");
        assert_eq!(clean_title("  Plain  "), "Plain");
        assert_eq!(clean_title("PlayStation®Store"), "PlayStation Store");
    }

    #[test]
    fn adrenaline_bubbles_are_not_listed() {
        assert!(!is_listed("PSPEMU001"));
        assert!(!is_listed("PSPEMU123"));
        assert!(is_listed("PSPEMUCFW"));
        assert!(is_listed("PCSA00069"));
        assert!(is_listed("VITASHELL"));
    }

    #[test]
    fn system_apps_with_a_bubble_are_listed() {
        assert!(is_listed("NPXS10015"));
        assert!(is_listed("NPXS10003"));
        assert!(!is_listed("NPXS10016"));
        assert!(!is_listed("NPXS10079"));
    }

    #[test]
    fn catalog_sorts_titles_into_categories() {
        let info = |id: &str, platform: &str, console: Option<&str>| TitleInfo {
            id: id.into(),
            title: format!("{id}™"),
            platform: platform.into(),
            adrenaline: false,
            console: console.map(String::from),
        };
        let games = load_catalog(&[
            info("PCSA00069", "vita", None),
            info("PSPEMU001", "vita", None),
            info("ra0000001", "retro", Some("Nintendo - Game Boy")),
            info("ra0000002", "retro", Some("Sony - PlayStation")),
            info("NPUZ00001", "psp", Some("ignored")),
        ]);
        let got: Vec<(&str, &str, u8)> = games.iter().map(|g| (&*g.id, &*g.category, g.tint)).collect();
        assert_eq!(got, [("PCSA00069", "games", 0), ("ra0000001", "retro-gb", 1), ("ra0000002", "builtin-psx", 2), ("NPUZ00001", "builtin-psp", 3)]);
        assert_eq!(&*games[0].title, "PCSA00069");
        assert_eq!(games[1].shown_id(), "RetroArch");
        assert_eq!(games[3].console, None);
    }
}
