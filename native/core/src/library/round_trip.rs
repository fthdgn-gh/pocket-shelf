//! Each saved file, read and written again, matches byte for byte what the
//! current app's own save functions wrote (`tests/fixtures/`, made by
//! `bun run next:fixtures` from `native/tools/fixtures.test.ts`).

use std::path::PathBuf;

use super::categories::CategoryConfig;
use super::overrides::{render_cache, OverrideStore, TitleOverride, CACHE_FILE};
use super::recent;
use super::settings::{Settings, UpdateChannel};
use super::testing::Files;
use crate::host::Host;
use crate::theme::ThemeId;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

const TITLES: [&str; 3] = ["PCSA00069", "PCSB00245", "ra0k3x9zq"];

#[test]
fn settings() {
    for name in ["settings/full.json", "settings/no-title.json"] {
        let text = fixture(name);
        let settings = Settings::parse(text.as_bytes(), Settings::defaults(UpdateChannel::Stable));
        assert_eq!(settings.render(), text, "{name}");
    }
    let full = Settings::parse(fixture("settings/full.json").as_bytes(), Settings::defaults(UpdateChannel::Stable));
    assert_eq!(full.theme, ThemeId::Dynamic);
    assert_eq!(full.title.as_deref(), Some("ra0k3x9zq"));
}

#[test]
fn categories() {
    for name in ["categories/full.json", "categories/empty.json"] {
        let text = fixture(name);
        assert_eq!(CategoryConfig::parse(text.as_bytes()).render(), text, "{name}");
    }
    let config = CategoryConfig::parse(fixture("categories/full.json").as_bytes());
    assert_eq!(config.custom[1].label, "Çocuk \"Oyunları\"");
}

#[test]
fn recent() {
    for name in ["recent/three.json", "recent/empty.json"] {
        let text = fixture(name);
        assert_eq!(recent::render(&recent::parse(text.as_bytes())), text, "{name}");
    }
}

fn folder(with_cache: bool) -> Files {
    let mut files = Files::default();
    for id in TITLES {
        let path = format!("titles/{id}.json");
        files.0.insert(path.clone(), fixture(&path).into_bytes());
    }
    if with_cache {
        files.0.insert(CACHE_FILE.into(), fixture(CACHE_FILE).into_bytes());
    }
    files
}

#[test]
fn title_files_and_cache() {
    // Read from the cache, and from the per-title files (which rebuilds the
    // cache): the same changes, written back as they were.
    for with_cache in [true, false] {
        let mut files = folder(with_cache);
        let store = OverrideStore::load(&mut files);
        assert_eq!(render_cache(&store.titles), fixture(CACHE_FILE), "cache, with_cache={with_cache}");
        assert_eq!(files.read(CACHE_FILE).unwrap(), fixture(CACHE_FILE).into_bytes());
        for id in TITLES {
            let path = format!("titles/{id}.json");
            assert_eq!(store.titles[id].render(), fixture(&path), "{path}");
        }
    }
}

#[test]
fn saving_writes_only_what_changed() {
    let mut files = folder(true);
    let mut store = OverrideStore::load(&mut files);
    let before = files.0.clone();
    assert!(store.save(&mut files));
    assert_eq!(files.0, before, "nothing changed, nothing written");

    store.set("PCSB00245", TitleOverride { favorite: true, ..Default::default() });
    store.set("ra0k3x9zq", TitleOverride::default());
    assert!(store.save(&mut files));
    assert_eq!(files.0["titles/PCSB00245.json"], b"{\n  \"favorite\": true\n}\n");
    assert!(!files.0.contains_key("titles/ra0k3x9zq.json"));
    assert_eq!(files.0["titles/PCSA00069.json"], before["titles/PCSA00069.json"]);
    let cache = String::from_utf8(files.0[CACHE_FILE].clone()).unwrap();
    assert_eq!(
        cache,
        "{\"version\":1,\"titles\":{\"PCSA00069\":{\"category\":\"emulators\",\"title\":\"Gravity \\\\ Daze\",\"art\":\"PCSA00069-48213.png\",\"favorite\":true},\"PCSB00245\":{\"favorite\":true}}}\n"
    );
}
