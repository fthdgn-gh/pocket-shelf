//! Categories (`src/categories.ts`): the built-in and smart ones, the tab
//! order, ids made from labels, and `categories.json`.

use serde_json::{json, Value};

/// A category whose label comes from the language's `categories` texts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CategoryName {
    Recent,
    Favorites,
    Search,
    Games,
    System,
    Homebrew,
    Psm,
    Psp,
    Psx,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Category {
    pub id: String,
    /// The label the user typed, or the English label of a category with a
    /// `name`.
    pub label: String,
    /// Set for every category the user did not create.
    pub name: Option<CategoryName>,
    /// Created by the user.
    pub custom: bool,
    /// Filled by the launcher; a title cannot be moved into it.
    pub smart: bool,
    /// A console's category of RetroArch games (`consoles.rs`).
    pub console: bool,
}

impl Category {
    fn named(id: &str, label: &str, name: CategoryName, smart: bool) -> Category {
        Category { id: id.into(), label: label.into(), name: Some(name), custom: false, smart, console: false }
    }

    pub fn custom(id: &str, label: &str) -> Category {
        Category { id: id.into(), label: label.into(), name: None, custom: true, smart: false, console: false }
    }
}

// The ids carry a prefix so they cannot match one made from a label the user
// typed before these categories existed.
pub const RECENT_ID: &str = "smart-recent";
pub const FAVORITES_ID: &str = "smart-favorites";
/// The tab of search results: first, and only while a search is open.
pub const SEARCH_ID: &str = "smart-search";
pub const SYSTEM_ID: &str = "system";
pub const PSM_ID: &str = "builtin-psm";
pub const PSP_ID: &str = "builtin-psp";
pub const PSX_ID: &str = "builtin-psx";

pub fn search_category() -> Category {
    Category::named(SEARCH_ID, "Search", CategoryName::Search, true)
}

/// "Last Played" and "Favorites": before the other tabs until reordered,
/// left out while empty.
pub fn smart_categories() -> Vec<Category> {
    vec![
        Category::named(RECENT_ID, "Last Played", CategoryName::Recent, true),
        Category::named(FAVORITES_ID, "Favorites", CategoryName::Favorites, true),
    ]
}

/// The default tab order: the system applications last.
pub fn builtin_categories() -> Vec<Category> {
    vec![
        Category::named("games", "Games", CategoryName::Games, false),
        Category::named("homebrew", "Homebrew", CategoryName::Homebrew, false),
        Category::named(PSM_ID, "PS Mobile", CategoryName::Psm, false),
        Category::named(PSP_ID, "PSP", CategoryName::Psp, false),
        Category::named(PSX_ID, "PSX", CategoryName::Psx, false),
        Category::named(SYSTEM_ID, "System", CategoryName::System, false),
    ]
}

/// A built-in category with no titles is left out of the tab bar unless
/// this is set. Categories the user created always show.
pub const SHOW_EMPTY_CATEGORIES: bool = false;

pub const CATEGORY_LABEL_MAX: usize = 16;

/// `^[a-z0-9-]{1,32}$`
pub fn valid_category_id(id: &str) -> bool {
    (1..=32).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Where a title lives, as the host reports it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Platform {
    Vita,
    Psm,
    Psp,
    Psx,
    Retro,
}

impl Platform {
    /// The platform a host entry names; Vita for an unknown one.
    pub fn from_key(key: &str) -> Platform {
        match key {
            "psm" => Platform::Psm,
            "psp" => Platform::Psp,
            "psx" => Platform::Psx,
            "retro" => Platform::Retro,
            _ => Platform::Vita,
        }
    }

    /// The platform's name in the views (`GENRES` in `catalog.ts`).
    pub fn genre(self) -> &'static str {
        match self {
            Platform::Vita => "PS Vita",
            Platform::Psm => "PS Mobile",
            Platform::Psp => "PSP",
            Platform::Psx => "PSX",
            Platform::Retro => "RetroArch",
        }
    }
}

/// Category of an installed title: PSP and PSX by platform, the rest by
/// title id (PCS retail, NPXS system, PSM ids, else homebrew).
pub fn category_of(title_id: &str, platform: Platform) -> String {
    match platform {
        Platform::Psp => PSP_ID,
        Platform::Psx => PSX_ID,
        _ if title_id.starts_with("PCS") => "games",
        _ if title_id.starts_with("NPXS") => SYSTEM_ID,
        _ if is_psm_id(title_id) => PSM_ID,
        _ => "homebrew",
    }
    .into()
}

/// A PlayStation Mobile title id (`NPNA00001`): `^NP[NOPQ]A\d{5}$`.
pub fn is_psm_id(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 9 && b.starts_with(b"NP") && b"NOPQ".contains(&b[2]) && b[3] == b'A' && b[4..].iter().all(u8::is_ascii_digit)
}

/// Categories in tab order. A category in the saved order keeps its place;
/// of the others, smart ones go first and the rest follow in the order
/// given, except System, which goes last. While System is the last saved
/// category, a new one goes before it; once the user moved System, new ones
/// go at the end.
pub fn order_categories(items: &[Category], saved: &[String]) -> Vec<Category> {
    let base: Vec<&Category> =
        items.iter().filter(|c| c.id != SYSTEM_ID).chain(items.iter().filter(|c| c.id == SYSTEM_ID)).collect();
    let system_at = saved.iter().position(|id| id == SYSTEM_ID);
    let system_last = system_at.is_some_and(|at| saved[at + 1..].iter().all(|id| !base.iter().any(|c| &c.id == id)));
    let position = |item: &Category| -> f64 {
        if let Some(index) = saved.iter().position(|id| *id == item.id) {
            return index as f64;
        }
        if item.smart {
            return -1.0;
        }
        if item.id == SYSTEM_ID {
            return f64::MAX;
        }
        match system_at {
            Some(at) if system_last => at as f64 - 0.5,
            _ => f64::MAX / 2.0,
        }
    };
    let mut ranked: Vec<(f64, usize, &Category)> = base.iter().enumerate().map(|(i, c)| (position(c), i, *c)).collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, _, c)| c.clone()).collect()
}

/// The id `delta` tabs from `current`, wrapping; the first tab counts as
/// current when `current` is not listed.
pub fn cycle_category<'a>(list: &'a [String], current: &'a str, delta: i32) -> &'a str {
    if list.is_empty() {
        return current;
    }
    let index = list.iter().position(|id| id == current).unwrap_or(0) as i32;
    let n = list.len() as i32;
    &list[((index + delta) % n + n) as usize % list.len()]
}

/// Lowercase, runs of anything but `a-z0-9` as one `-`, no `-` at the ends.
pub(crate) fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in text.to_lowercase().chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(ch);
        } else {
            dash = true;
        }
    }
    out
}

/// A category id from its label, unique among `taken`.
pub fn make_category_id(label: &str, taken: &dyn Fn(&str) -> bool) -> String {
    let mut slug = slug(label);
    slug.truncate(24);
    let slug = if slug.is_empty() { "category".to_string() } else { slug };
    let mut id = slug.clone();
    let mut n = 2;
    while taken(&id) {
        id = format!("{slug}-{n}");
        n += 1;
    }
    id
}

/// `text` trimmed and cut to `max` characters.
pub(crate) fn cut(text: &str, max: usize) -> String {
    text.trim().chars().take(max).collect()
}

/// `categories.json`: the user's categories, every id in tab order, and the
/// hidden ids.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CategoryConfig {
    pub custom: Vec<Category>,
    pub order: Vec<String>,
    pub hidden: Vec<String>,
}

pub const CATEGORIES_FILE: &str = "categories.json";

/// Valid ids, each once.
fn ids(value: Option<&Value>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in value.and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
        if valid_category_id(id) && !out.iter().any(|seen| seen == id) {
            out.push(id.into());
        }
    }
    out
}

impl CategoryConfig {
    /// The file's contents; anything unusable is left out, so a hand-edited
    /// file degrades to defaults.
    pub fn parse(bytes: &[u8]) -> CategoryConfig {
        let mut config = CategoryConfig::default();
        let Ok(object) = serde_json::from_slice::<Value>(bytes) else { return config };
        let mut seen: Vec<String> =
            std::iter::once(search_category()).chain(smart_categories()).chain(builtin_categories()).map(|c| c.id).collect();
        for value in object.get("custom").and_then(Value::as_array).into_iter().flatten() {
            let Some(id) = value.get("id").and_then(Value::as_str) else { continue };
            let label = value.get("label").and_then(Value::as_str).map(|l| cut(l, CATEGORY_LABEL_MAX)).unwrap_or_default();
            if !valid_category_id(id) || seen.iter().any(|s| s == id) || label.is_empty() {
                continue;
            }
            seen.push(id.into());
            config.custom.push(Category::custom(id, &label));
        }
        config.order = ids(object.get("order"));
        config.hidden = ids(object.get("hidden"));
        config
    }

    /// The file as the current app writes it: two-space JSON and a newline.
    pub fn render(&self) -> String {
        let custom: Vec<Value> = self.custom.iter().map(|c| json!({ "id": c.id, "label": c.label })).collect();
        let plain = json!({ "custom": custom, "order": self.order, "hidden": self.hidden });
        serde_json::to_string_pretty(&plain).expect("json") + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_of_reads_the_title_id_prefix() {
        assert_eq!(category_of("PCSA00069", Platform::Vita), "games");
        assert_eq!(category_of("NPXS10001", Platform::Vita), "system");
        assert_eq!(category_of("POCKTSHLF", Platform::Vita), "homebrew");
        assert_eq!(category_of("NPOA00013", Platform::Vita), PSM_ID);
        assert_eq!(category_of("NPNA00042", Platform::Vita), PSM_ID);
        assert_eq!(category_of("NPUZ00001", Platform::Psp), PSP_ID);
        assert_eq!(category_of("SCUS94163", Platform::Psx), PSX_ID);
        assert_eq!(category_of("SCUS94163", Platform::Vita), "homebrew");
    }

    #[test]
    fn default_tab_order_ends_with_system() {
        let ids: Vec<String> = builtin_categories().into_iter().map(|c| c.id).collect();
        assert_eq!(ids, ["games", "homebrew", PSM_ID, PSP_ID, PSX_ID, SYSTEM_ID]);
    }

    #[test]
    fn cycle_category_wraps_and_tolerates_unknown_ids() {
        let list: Vec<String> = ["games", "system", "homebrew"].map(String::from).into();
        assert_eq!(cycle_category(&list, "games", 1), "system");
        assert_eq!(cycle_category(&list, "homebrew", 1), "games");
        assert_eq!(cycle_category(&list, "games", -1), "homebrew");
        let two: Vec<String> = ["games", "system"].map(String::from).into();
        assert_eq!(cycle_category(&two, "gone", 1), "system");
        assert_eq!(cycle_category(&[], "games", 1), "games");
    }

    #[test]
    fn make_category_id_slugs_and_avoids_taken_ids() {
        assert_eq!(make_category_id("Retro Games!", &|_| false), "retro-games");
        assert_eq!(make_category_id("Retro Games", &|id| id == "retro-games"), "retro-games-2");
        assert_eq!(make_category_id("Retro Games", &|id| id == "retro-games" || id == "retro-games-2"), "retro-games-3");
        assert_eq!(make_category_id("!!!", &|_| false), "category");
    }

    #[test]
    fn parse_drops_unusable_entries() {
        let text = br#"{"custom":[{"id":"games","label":"Taken"},{"id":"Bad Id","label":"x"},{"id":"ok","label":"   "},
            {"id":"long","label":"  A label well over sixteen  "},{"id":"long","label":"Twice"}],
            "order":["games",3,"games","NOPE","system"],"hidden":"psm"}"#;
        let config = CategoryConfig::parse(text);
        assert_eq!(config.custom, vec![Category::custom("long", "A label well ove")]);
        assert_eq!(config.order, ["games", "system"]);
        assert!(config.hidden.is_empty());
        assert_eq!(CategoryConfig::parse(b"not json"), CategoryConfig::default());
        assert_eq!(CategoryConfig::parse(b"null"), CategoryConfig::default());
    }
}
