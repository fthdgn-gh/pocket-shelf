//! What the user changed about each title (`src/overrides.ts`).
//!
//! One file per title, `titles/<id>.json`, so a title's changes can be
//! copied, deleted or edited by hand; every title's changes also in one
//! file, `titles.cache.json`, read at start (one file per title cost 0.43 s
//! with a hundred titles). The per-title files stay the source: the cache is
//! rewritten with them, and [`OverrideStore::reload`] rebuilds it from them.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::categories::{cut, valid_category_id};
use crate::host::Host;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TitleOverride {
    pub category: Option<String>,
    pub title: Option<String>,
    /// File name in `art/`, or [`USE_ICON`].
    pub art: Option<String>,
    /// File name in `backdrops/`, or [`NO_BACKDROP`].
    pub backdrop: Option<String>,
    /// In the "Favorites" category.
    pub favorite: bool,
}

/// Overrides by title id, in the order the current app keeps them (the
/// cache's key order).
pub type Overrides = IndexMap<String, TitleOverride, super::fxhash::FxBuild>;

pub const TITLE_MAX: usize = 40;
/// `art` value: show the title's own icon, even if a file matches its name.
pub const USE_ICON: &str = "icon";
/// `backdrop` value: draw no picture behind this title.
pub const NO_BACKDROP: &str = "none";

pub const TITLES_DIR: &str = "titles";
pub const CACHE_FILE: &str = "titles.cache.json";
const CACHE_VERSION: u64 = 1;

/// `^[A-Za-z0-9_-]{1,32}$`
pub fn valid_title_id(id: &str) -> bool {
    (1..=32).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// A `.png` name (any case) of at most 96 UTF-16 units.
fn png_name(name: &str) -> bool {
    name.len() >= 4 && name[name.len() - 4..].eq_ignore_ascii_case(".png") && name.encode_utf16().count() <= 96
}

impl TitleOverride {
    pub fn is_empty(&self) -> bool {
        *self == TitleOverride::default()
    }

    /// Keeps only valid fields, so a hand-edited file cannot break the
    /// launcher.
    pub fn sanitize(value: &Value) -> TitleOverride {
        let text = |key: &str| value.get(key).and_then(Value::as_str);
        let mut clean = TitleOverride::default();
        clean.category = text("category").filter(|c| valid_category_id(c)).map(String::from);
        clean.title = text("title").filter(|t| !t.trim().is_empty()).map(|t| cut(t, TITLE_MAX));
        clean.art = text("art").filter(|a| *a == USE_ICON || png_name(a)).map(String::from);
        clean.backdrop = text("backdrop").filter(|b| *b == NO_BACKDROP || png_name(b)).map(String::from);
        clean.favorite = value.get("favorite") == Some(&Value::Bool(true));
        clean
    }

    /// As JSON, fields in the order `sanitize` builds them.
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        for (key, field) in [("category", &self.category), ("title", &self.title), ("art", &self.art), ("backdrop", &self.backdrop)] {
            if let Some(value) = field {
                map.insert(key.into(), Value::String(value.clone()));
            }
        }
        if self.favorite {
            map.insert("favorite".into(), Value::Bool(true));
        }
        Value::Object(map)
    }

    /// A title's file: two-space JSON and a newline.
    pub fn render(&self) -> String {
        serde_json::to_string_pretty(&self.to_json()).expect("json") + "\n"
    }
}

fn file_of(id: &str) -> String {
    format!("{TITLES_DIR}/{id}.json")
}

/// The art or backdrop files the titles' changes name.
pub fn files_in_use(overrides: &Overrides, backdrops: bool) -> HashSet<String> {
    overrides
        .values()
        .filter_map(|change| if backdrops { change.backdrop.as_ref() } else { change.art.as_ref() })
        .filter(|file| *file != USE_ICON && *file != NO_BACKDROP)
        .cloned()
        .collect()
}

/// `overrides` without the files they name; "icon" and "none" stay.
pub fn without_files(overrides: &Overrides) -> Overrides {
    let mut next = Overrides::default();
    for (id, change) in overrides {
        let mut kept = change.clone();
        if kept.art.as_deref().is_some_and(|a| a != USE_ICON) {
            kept.art = None;
        }
        if kept.backdrop.as_deref().is_some_and(|b| b != NO_BACKDROP) {
            kept.backdrop = None;
        }
        if !kept.is_empty() {
            next.insert(id.clone(), kept);
        }
    }
    next
}

/// The cache file's text.
pub fn render_cache(overrides: &Overrides) -> String {
    let titles: Map<String, Value> = overrides.iter().map(|(id, change)| (id.clone(), change.to_json())).collect();
    let mut root = Map::new();
    root.insert("version".into(), Value::from(CACHE_VERSION));
    root.insert("titles".into(), Value::Object(titles));
    serde_json::to_string(&Value::Object(root)).expect("json") + "\n"
}

/// The overrides, and the text last read or written per title, so unchanged
/// titles are not rewritten.
#[derive(Default)]
pub struct OverrideStore {
    pub titles: Overrides,
    on_disk: HashMap<String, String>,
}

impl OverrideStore {
    /// From the cache, or from the per-title files when there is no usable
    /// cache.
    pub fn load(host: &mut dyn Host) -> OverrideStore {
        let mut store = OverrideStore::default();
        if !store.read_cache(host) {
            store.reload(host);
        }
        store
    }

    fn read_cache(&mut self, host: &mut dyn Host) -> bool {
        let Some(raw) = host.read(CACHE_FILE).and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { return false };
        if raw.get("version").and_then(Value::as_f64) != Some(CACHE_VERSION as f64) {
            return false;
        }
        let Some(titles) = raw.get("titles").and_then(Value::as_object) else { return false };
        for (id, value) in titles {
            if !valid_title_id(id) {
                continue;
            }
            let clean = TitleOverride::sanitize(value);
            if clean.is_empty() {
                continue;
            }
            self.on_disk.insert(id.clone(), clean.render());
            self.titles.insert(id.clone(), clean);
        }
        true
    }

    /// Read the per-title files again and rebuild the cache from them. An
    /// unreadable or malformed file is skipped and left as it is.
    pub fn reload(&mut self, host: &mut dyn Host) {
        self.titles.clear();
        self.on_disk.clear();
        for name in host.list(TITLES_DIR) {
            let Some(id) = name.strip_suffix(".json") else { continue };
            if !valid_title_id(id) {
                continue;
            }
            let Some(value) = host.read(&file_of(id)).and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { continue };
            let clean = TitleOverride::sanitize(&value);
            if clean.is_empty() {
                continue;
            }
            self.on_disk.insert(id.into(), clean.render());
            self.titles.insert(id.into(), clean);
        }
        host.write(CACHE_FILE, render_cache(&self.titles).as_bytes());
    }

    /// Replace one title's changes; a title with none is dropped. A title
    /// already listed keeps its place in the cache; a new one goes last.
    pub fn set(&mut self, id: &str, change: TitleOverride) {
        if change.is_empty() {
            self.titles.shift_remove(id);
        } else {
            self.titles.insert(id.into(), change);
        }
    }

    /// Write the titles whose changes differ from disk, delete the files of
    /// titles that were reset, and rewrite the cache when anything changed.
    /// Stops at the first file the host cannot write. Returns whether
    /// everything was saved.
    pub fn save(&mut self, host: &mut dyn Host) -> bool {
        let mut changed = false;
        for (id, change) in &self.titles {
            if !valid_title_id(id) {
                continue;
            }
            let text = change.render();
            if self.on_disk.get(id) == Some(&text) {
                continue;
            }
            if !host.write(&file_of(id), text.as_bytes()) {
                host.log(&format!("Title changes not saved: {id}"));
                return false;
            }
            self.on_disk.insert(id.clone(), text);
            changed = true;
        }
        let gone: Vec<String> = self.on_disk.keys().filter(|id| !self.titles.contains_key(*id)).cloned().collect();
        for id in gone {
            if !host.remove(&file_of(&id)) {
                host.log(&format!("Title changes not saved: {id} not deleted"));
                return false;
            }
            self.on_disk.remove(&id);
            changed = true;
        }
        if changed && !host.write(CACHE_FILE, render_cache(&self.titles).as_bytes()) {
            host.log("Title changes not saved: cache");
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Overrides {
        let mut o = Overrides::default();
        o.insert(
            "PCSA00069".into(),
            TitleOverride { art: Some("PCSA00069-1.png".into()), backdrop: Some("PCSA00069-2.png".into()), favorite: true, ..Default::default() },
        );
        o.insert(
            "PCSB00245".into(),
            TitleOverride { art: Some("icon".into()), backdrop: Some("none".into()), title: Some("Neon".into()), ..Default::default() },
        );
        o.insert("PCSE00317".into(), TitleOverride { backdrop: Some("PCSE00317-3.png".into()), ..Default::default() });
        o
    }

    #[test]
    fn files_in_use_lists_named_files_only() {
        let o = sample();
        assert_eq!(files_in_use(&o, false), HashSet::from(["PCSA00069-1.png".to_string()]));
        assert_eq!(files_in_use(&o, true), HashSet::from(["PCSA00069-2.png".to_string(), "PCSE00317-3.png".to_string()]));
    }

    #[test]
    fn without_files_keeps_every_other_change() {
        let next = without_files(&sample());
        assert_eq!(next.len(), 2);
        assert_eq!(next["PCSA00069"], TitleOverride { favorite: true, ..Default::default() });
        assert_eq!(next["PCSB00245"], sample()["PCSB00245"]);
    }

    #[test]
    fn sanitize_keeps_valid_fields() {
        let value: Value = serde_json::from_str(
            r#"{"category":"Bad Id","title":"   A title that is much longer than forty characters in all   ",
                "art":"x.PNG","backdrop":"x.jpg","favorite":"yes"}"#,
        )
        .unwrap();
        let clean = TitleOverride::sanitize(&value);
        assert_eq!(clean.category, None);
        assert_eq!(clean.title.as_deref(), Some("A title that is much longer than forty c"));
        assert_eq!(clean.art.as_deref(), Some("x.PNG"));
        assert_eq!(clean.backdrop, None);
        assert!(!clean.favorite);
        assert!(TitleOverride::sanitize(&serde_json::json!({"title": "  "})).is_empty());
    }
}
