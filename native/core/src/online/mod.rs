//! Artwork from the network (`src/scrapers.ts`): SteamGridDB (any title,
//! with the user's API key) and libretro's thumbnail server (RetroArch
//! games, PSX and PSP, no key). The title editor finds pictures for one
//! title; Fetch artwork goes through many.

pub mod fetch;
pub mod flow;
pub mod libretro;
pub mod steamgriddb;

use crate::library::catalog::Game;
use crate::library::settings::ScrapeSource;
use steamgriddb::{Asset, AssetKind};

/// `encodeURIComponent`: everything but `A-Z a-z 0-9 - _ . ! ~ * ' ( )` as
/// `%XX` of its UTF-8 bytes.
pub fn encode_uri_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scraper {
    SteamGridDb,
    Libretro,
}

impl Scraper {
    pub fn name(self) -> &'static str {
        match self {
            Scraper::SteamGridDb => "SteamGridDB",
            Scraper::Libretro => "libretro",
        }
    }
}

/// The scraper a title's pictures come from under the user's choice: Auto
/// gives libretro to RetroArch games and SteamGridDB to the rest. `None`
/// when the choice is libretro and the server has no folder for the title.
pub fn scraper_for(source: ScrapeSource, game: &Game) -> Option<Scraper> {
    match source {
        ScrapeSource::SteamGridDb => Some(Scraper::SteamGridDb),
        ScrapeSource::Libretro => libretro::libretro_folder(game.console.as_deref(), game.platform).map(|_| Scraper::Libretro),
        ScrapeSource::Auto => Some(if game.console.is_some() { Scraper::Libretro } else { Scraper::SteamGridDb }),
    }
}

/// One picture a title could use.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Candidate {
    pub url: String,
    /// File name in `art/` (an icon) or `backdrops/` (a backdrop).
    pub file: String,
    /// Saved fitted into a square icon by the host (a box art).
    pub fit: bool,
}

pub fn candidate_path(kind: AssetKind, candidate: &Candidate) -> String {
    format!("{}/{}", steamgriddb::folder(kind), candidate.file)
}

/// SteamGridDB's pictures of a kind as candidates.
pub fn sgdb_candidates(assets: &[Asset], owner_id: &str) -> Vec<Candidate> {
    assets.iter().map(|a| Candidate { url: a.url.clone(), file: steamgriddb::asset_file(a, owner_id), fit: false }).collect()
}

/// A libretro name's pictures as candidates: the box art, then the title
/// screen, as the icon; the screenshot, then the title screen, as the
/// backdrop. Files are `<title id>-lr-box.png` and so on.
pub fn libretro_candidates(folder: &str, name: &str, owner_id: &str, kind: AssetKind) -> Vec<Candidate> {
    use libretro::{thumbnail_url, Thumbnail};
    let make = |thumb, suffix: &str, fit| Candidate { url: thumbnail_url(folder, thumb, name), file: format!("{owner_id}-lr-{suffix}.png"), fit };
    match kind {
        AssetKind::Icon => vec![make(Thumbnail::Boxarts, "box", true), make(Thumbnail::Titles, "title", true)],
        AssetKind::Backdrop => vec![make(Thumbnail::Snaps, "snap", false), make(Thumbnail::Titles, "title", false)],
    }
}

/// Reading the user's sample playlists for the libretro test.
#[cfg(test)]
pub(crate) mod tests_support {
    /// The labels of a RetroArch playlist: RZIP (compressed) or plain JSON.
    pub fn read_playlist(path: &std::path::Path) -> Option<Vec<String>> {
        let bytes = std::fs::read(path).ok()?;
        let text = if bytes.starts_with(b"#RZIPv") {
            let size = u64::from_le_bytes(bytes.get(12..20)?.try_into().ok()?) as usize;
            let mut out = Vec::with_capacity(size);
            let mut at = 20;
            while out.len() < size && at + 4 <= bytes.len() {
                let length = u32::from_le_bytes(bytes[at..at + 4].try_into().ok()?) as usize;
                let part = miniz_oxide::inflate::decompress_to_vec_zlib(bytes.get(at + 4..at + 4 + length)?).ok()?;
                out.extend(part);
                at += 4 + length;
            }
            String::from_utf8(out).ok()?
        } else {
            String::from_utf8(bytes).ok()?
        };
        let json: serde_json::Value = serde_json::from_str(&text).ok()?;
        Some(json.get("items")?.as_array()?.iter().filter_map(|i| i.get("label")?.as_str().map(String::from)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::TitleInfo;
    use crate::library::catalog::load_catalog;

    fn game(platform: &str, console: Option<&str>) -> Game {
        load_catalog(&[TitleInfo { id: "ra0000001".into(), title: "T".into(), platform: platform.into(), adrenaline: false, console: console.map(String::from) }])
            .remove(0)
    }

    #[test]
    fn auto_gives_retroarch_games_libretro_and_the_rest_steamgriddb() {
        let retro = game("retro", Some("Nintendo - Game Boy"));
        let (psx, vita) = (game("psx", None), game("vita", None));
        assert_eq!(scraper_for(ScrapeSource::Auto, &retro), Some(Scraper::Libretro));
        assert_eq!(scraper_for(ScrapeSource::Auto, &psx), Some(Scraper::SteamGridDb));
        assert_eq!(scraper_for(ScrapeSource::Auto, &vita), Some(Scraper::SteamGridDb));
        assert_eq!(scraper_for(ScrapeSource::SteamGridDb, &retro), Some(Scraper::SteamGridDb));
        assert_eq!(scraper_for(ScrapeSource::Libretro, &psx), Some(Scraper::Libretro));
        assert_eq!(scraper_for(ScrapeSource::Libretro, &vita), None);
    }

    #[test]
    fn uri_components_are_escaped_as_javascript_does() {
        assert_eq!(encode_uri_component("Catrap (USA)"), "Catrap%20(USA)");
        assert_eq!(encode_uri_component("Pokémon/Ü?"), "Pok%C3%A9mon%2F%C3%9C%3F");
    }

    #[test]
    fn libretro_candidates_name_their_files() {
        let icons = libretro_candidates("Nintendo - Game Boy", "Tetris (World)", "ra0000001", AssetKind::Icon);
        assert_eq!(icons[0].file, "ra0000001-lr-box.png");
        assert!(icons[0].fit && icons[0].url.ends_with("/Named_Boxarts/Tetris%20(World).png"));
        let backdrops = libretro_candidates("Nintendo - Game Boy", "Tetris (World)", "ra0000001", AssetKind::Backdrop);
        assert_eq!((backdrops[0].file.as_str(), backdrops[0].fit), ("ra0000001-lr-snap.png", false));
    }
}
