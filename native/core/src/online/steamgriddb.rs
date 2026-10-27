//! SteamGridDB (`src/steamgriddb.ts`, https://www.steamgriddb.com/api/v2):
//! search a game by name, then list its icons and heroes. Every call needs
//! the user's own API key, sent as `Authorization: Bearer <key>`. The
//! launcher decodes PNG only, so each list asks for static PNG files in sizes
//! its decoders accept.

use serde_json::Value;

use super::encode_uri_component;

const API: &str = "https://www.steamgriddb.com/api/v2";

/// Candidates kept per list; the API returns them best score first.
pub const ASSETS_MAX: usize = 30;
pub const GAMES_MAX: usize = 12;

/// `steamgriddb.txt` in the data folder, one line: the API key.
pub const KEY_FILE: &str = "steamgriddb.txt";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AssetKind {
    Icon,
    Backdrop,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Game {
    pub id: u64,
    pub name: String,
    /// Release year, when the database has one.
    pub year: Option<i32>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Asset {
    pub id: u64,
    pub url: String,
}

/// `^[A-Za-z0-9]{16,64}$`
fn valid_key(key: &str) -> bool {
    (16..=64).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// The saved key from the file's text, or "" when it does not hold one.
pub fn parse_key(text: &str) -> String {
    let key = text.trim();
    if valid_key(key) {
        key.to_string()
    } else {
        String::new()
    }
}

/// A typed key as the file's text; `None` when it does not look like a key.
pub fn render_key(text: &str) -> Option<String> {
    let key = text.trim();
    valid_key(key).then(|| format!("{key}\n"))
}

pub fn authorization(key: &str) -> String {
    format!("Bearer {key}")
}

pub fn search_url(term: &str) -> String {
    format!("{API}/search/autocomplete/{}", encode_uri_component(term.trim()))
}

pub fn assets_url(kind: AssetKind, game_id: u64) -> String {
    match kind {
        AssetKind::Icon => format!("{API}/icons/game/{game_id}?types=static&mimes=image/png&dimensions=128,256,512"),
        AssetKind::Backdrop => format!("{API}/heroes/game/{game_id}?types=static&mimes=image/png&dimensions=1920x620,1600x650"),
    }
}

/// The folder a kind is saved in.
pub fn folder(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Icon => "art",
        AssetKind::Backdrop => "backdrops",
    }
}

/// File name of a candidate: the title's id, then the picture's id on
/// SteamGridDB (`PCSA00069-48213.png`). The title id does not change when
/// the user renames the title, and lets the picker list one title's files.
pub fn asset_file(asset: &Asset, owner_id: &str) -> String {
    format!("{owner_id}-{}.png", asset.id)
}

pub fn asset_path(kind: AssetKind, asset: &Asset, owner_id: &str) -> String {
    format!("{}/{}", folder(kind), asset_file(asset, owner_id))
}

/// Why a reply has no data; each a key of a message in the language files.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Problem {
    KeyRejected,
    ServerBusy,
    ServerStatus(u16),
    BadReply,
}

/// The `data` array of a reply, or why there is none.
fn data_of(status: u16, text: &str) -> Result<Vec<Value>, Problem> {
    match status {
        401 => return Err(Problem::KeyRejected),
        404 => return Ok(Vec::new()),
        429 => return Err(Problem::ServerBusy),
        200 => {}
        _ => return Err(Problem::ServerStatus(status)),
    }
    let reply: Value = serde_json::from_str(text).map_err(|_| Problem::BadReply)?;
    if reply.get("success") != Some(&Value::Bool(true)) {
        return Err(Problem::BadReply);
    }
    match reply.get("data") {
        Some(Value::Array(items)) => Ok(items.clone()),
        _ => Err(Problem::BadReply),
    }
}

/// The year of a Unix time in seconds (UTC).
fn year_of(seconds: i64) -> i32 {
    // Days to a civil date (Howard Hinnant's algorithm), year only.
    let z = seconds.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + if month <= 2 { 1 } else { 0 }) as i32
}

pub fn parse_games(status: u16, text: &str) -> Result<Vec<Game>, Problem> {
    let mut games = Vec::new();
    for item in data_of(status, text)? {
        let (Some(id), Some(name)) = (item.get("id").and_then(Value::as_f64), item.get("name").and_then(Value::as_str)) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let seconds = item.get("release_date").and_then(Value::as_f64).unwrap_or(0.0);
        games.push(Game { id: id as u64, name: name.to_string(), year: (seconds > 0.0).then(|| year_of(seconds as i64)) });
        if games.len() == GAMES_MAX {
            break;
        }
    }
    Ok(games)
}

pub fn parse_assets(status: u16, text: &str) -> Result<Vec<Asset>, Problem> {
    let mut assets = Vec::new();
    for item in data_of(status, text)? {
        let (Some(id), Some(url)) = (item.get("id").and_then(Value::as_f64), item.get("url").and_then(Value::as_str)) else {
            continue;
        };
        // The host downloads over https and keeps PNG only.
        if !url.starts_with("https://") || !url.to_lowercase().ends_with(".png") {
            continue;
        }
        assets.push(Asset { id: id as u64, url: url.to_string() });
        if assets.len() == ASSETS_MAX {
            break;
        }
    }
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(data: &str) -> String {
        format!(r#"{{"success":true,"data":{data}}}"#)
    }

    #[test]
    fn the_search_term_is_trimmed_and_escaped() {
        assert_eq!(search_url("  Gravity Rush  "), "https://www.steamgriddb.com/api/v2/search/autocomplete/Gravity%20Rush");
        assert_eq!(
            search_url("Persona 4: Golden / Vita?"),
            "https://www.steamgriddb.com/api/v2/search/autocomplete/Persona%204%3A%20Golden%20%2F%20Vita%3F"
        );
    }

    #[test]
    fn asset_lists_ask_for_static_png() {
        assert_eq!(assets_url(AssetKind::Icon, 42), "https://www.steamgriddb.com/api/v2/icons/game/42?types=static&mimes=image/png&dimensions=128,256,512");
        assert_eq!(
            assets_url(AssetKind::Backdrop, 42),
            "https://www.steamgriddb.com/api/v2/heroes/game/42?types=static&mimes=image/png&dimensions=1920x620,1600x650"
        );
        assert_eq!(authorization("abc123"), "Bearer abc123");
    }

    #[test]
    fn a_candidate_is_saved_under_its_id() {
        let asset = Asset { id: 9001, url: "https://cdn2.steamgriddb.com/hero/x.png".into() };
        assert_eq!(asset_file(&asset, "PCSA00069"), "PCSA00069-9001.png");
        assert_eq!(asset_path(AssetKind::Icon, &asset, "PCSA00069"), "art/PCSA00069-9001.png");
        assert_eq!(asset_path(AssetKind::Backdrop, &asset, "PCSA00069"), "backdrops/PCSA00069-9001.png");
    }

    #[test]
    fn games_read_id_name_and_year() {
        let text = reply(r#"[{"id":1,"name":"Gravity Rush","release_date":1339459200,"types":["steam"]},{"id":2,"name":"No Date"},{"id":3,"name":"Zero Date","release_date":0}]"#);
        assert_eq!(
            parse_games(200, &text).unwrap(),
            [
                Game { id: 1, name: "Gravity Rush".into(), year: Some(2012) },
                Game { id: 2, name: "No Date".into(), year: None },
                Game { id: 3, name: "Zero Date".into(), year: None },
            ]
        );
        assert!(parse_games(200, &reply(r#"[{"name":"No id"},{"id":5},{"id":6,"name":""},null]"#)).unwrap().is_empty());
        let many: Vec<String> = (0..GAMES_MAX + 8).map(|i| format!(r#"{{"id":{},"name":"Game {i}"}}"#, i + 1)).collect();
        assert_eq!(parse_games(200, &reply(&format!("[{}]", many.join(",")))).unwrap().len(), GAMES_MAX);
        assert!(parse_games(200, &reply("[]")).unwrap().is_empty());
        assert!(parse_games(404, "").unwrap().is_empty());
    }

    #[test]
    fn assets_keep_https_png_and_cap_the_list() {
        let text = reply(
            r#"[{"id":10,"url":"https://cdn2.steamgriddb.com/icon/a.png","thumb":"https://x/a.jpg"},{"id":11,"url":"https://cdn2.steamgriddb.com/icon/b.PNG"},{"id":12,"url":"https://cdn2.steamgriddb.com/icon/c.ico"},{"id":13,"url":"https://cdn2.steamgriddb.com/hero/d.jpg"},{"id":14,"url":"http://cdn2.steamgriddb.com/icon/e.png"},{"url":"https://cdn2.steamgriddb.com/icon/f.png"}]"#,
        );
        assert_eq!(
            parse_assets(200, &text).unwrap(),
            [
                Asset { id: 10, url: "https://cdn2.steamgriddb.com/icon/a.png".into() },
                Asset { id: 11, url: "https://cdn2.steamgriddb.com/icon/b.PNG".into() },
            ]
        );
        let many: Vec<String> = (0..ASSETS_MAX + 5).map(|i| format!(r#"{{"id":{},"url":"https://cdn2.steamgriddb.com/icon/{i}.png"}}"#, i + 1)).collect();
        assert_eq!(parse_assets(200, &reply(&format!("[{}]", many.join(",")))).unwrap().len(), ASSETS_MAX);
    }

    #[test]
    fn failed_replies_are_named() {
        assert_eq!(parse_games(401, r#"{"success":false,"errors":["Authentication Required"]}"#), Err(Problem::KeyRejected));
        assert_eq!(parse_assets(429, ""), Err(Problem::ServerBusy));
        assert_eq!(parse_assets(503, ""), Err(Problem::ServerStatus(503)));
        for text in ["", "<html>", r#"{"success":false}"#, r#"{"success":true,"data":{}}"#] {
            assert_eq!(parse_games(200, text), Err(Problem::BadReply), "{text}");
        }
    }

    #[test]
    fn keys_are_checked_and_saved_on_one_line() {
        assert_eq!(parse_key(" 0123456789abcdef0123456789abcdef \n"), "0123456789abcdef0123456789abcdef");
        assert_eq!(parse_key("short"), "");
        assert_eq!(render_key("abcdefghijklmnop").as_deref(), Some("abcdefghijklmnop\n"));
        assert_eq!(render_key("not a key!"), None);
    }
}
