//! `shelf-tools wikidata [out dir] [table...]`: the game details tables
//! the Vita downloads (`shelf_core::online::wikidata`), built from
//! Wikidata's query service, one table per console plus `labels.tsv`
//! (genre and mode names) and `index.tsv` (each table's game count).
//!
//! Each console is a handful of small queries (games, aliases, genres,
//! developers, publishers, modes, series, Wikipedia articles) rather than
//! one: a query joining them all timed out for the larger platforms. A
//! second between queries, and a busy reply waited out, as the query
//! service asks of bots.
//!
//! `shelf-tools wikidata-match [dir]`: how many of the sample playlists'
//! labels (`.samples/retroarch-playlists`) and of libretro's database names
//! (`.samples/gamedb`) a built table finds.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use shelf_core::online::wikidata::{self, Game, Index, LANGUAGES, WIKIS};

const ENDPOINT: &str = "https://query.wikidata.org/sparql";
const USER_AGENT: &str = "PocketShelfGameData/1.0 (https://github.com/fthdgn-gh/pocket-shelf)";
const ENTITY: &str = "http://www.wikidata.org/entity/";
const TRIES: u32 = 6;

type Row = HashMap<String, String>;

struct Client {
    agent: ureq::Agent,
    queries: u32,
}

impl Client {
    fn new() -> Client {
        Client { agent: ureq::AgentBuilder::new().timeout(Duration::from_secs(120)).build(), queries: 0 }
    }

    /// The rows of a SELECT query, each variable's value with Wikidata's
    /// entity prefix taken off.
    fn select(&mut self, query: &str) -> Result<Vec<Row>, String> {
        for attempt in 1..=TRIES {
            if self.queries > 0 {
                std::thread::sleep(Duration::from_secs(1));
            }
            self.queries += 1;
            let reply = self
                .agent
                .get(ENDPOINT)
                .query("query", query)
                .query("format", "json")
                .set("Accept", "application/sparql-results+json")
                .set("User-Agent", USER_AGENT)
                .call();
            let wait = match reply {
                // A query over the service's time limit is cut off mid-reply.
                Ok(response) => match response.into_json::<Value>() {
                    Ok(json) => return Ok(rows(&json)),
                    Err(error) => {
                        eprintln!("  reply cut off: {error}");
                        10 * attempt as u64
                    }
                },
                Err(ureq::Error::Status(status, response)) if matches!(status, 429 | 500 | 502 | 503 | 504) => {
                    response.header("Retry-After").and_then(|s| s.parse().ok()).unwrap_or(10 * attempt as u64)
                }
                Err(ureq::Error::Status(status, response)) => {
                    let body = response.into_string().unwrap_or_default();
                    return Err(format!("status {status}: {}", body.lines().take(3).collect::<Vec<_>>().join(" ")));
                }
                Err(error) => {
                    eprintln!("  {error}");
                    10 * attempt as u64
                }
            };
            eprintln!("  busy, waiting {wait} s (try {attempt} of {TRIES})");
            std::thread::sleep(Duration::from_secs(wait));
        }
        Err("the query service stayed busy".into())
    }
}

fn rows(json: &Value) -> Vec<Row> {
    let Some(bindings) = json["results"]["bindings"].as_array() else { return Vec::new() };
    bindings
        .iter()
        .filter_map(|b| b.as_object())
        .map(|b| b.iter().filter_map(|(k, v)| Some((k.clone(), v["value"].as_str()?.trim_start_matches(ENTITY).to_string()))).collect())
        .collect()
}

/// The `VALUES` and pattern every query of a platform starts from.
fn games_of(items: &[&str]) -> String {
    let values: Vec<String> = items.iter().map(|i| format!("wd:{i}")).collect();
    format!("VALUES ?p {{ {} }} ?g wdt:P31 wd:Q7889; wdt:P400 ?p.", values.join(" "))
}

/// An item's English name, else its multilingual one.
const NAMED: &str = r#"OPTIONAL { ?v rdfs:label ?en FILTER(LANG(?en) = "en") } OPTIONAL { ?v rdfs:label ?mul FILTER(LANG(?mul) = "mul") }"#;

fn name_of(row: &Row) -> Option<String> {
    row.get("en").or_else(|| row.get("mul")).filter(|n| !n.trim().is_empty()).cloned()
}

/// A platform's games.
fn platform(client: &mut Client, items: &[&str]) -> Result<Vec<Game>, String> {
    let base = games_of(items);
    let mut games: BTreeMap<u64, Game> = BTreeMap::new();
    let number = |item: &str| item.trim_start_matches('Q').parse::<u64>().unwrap_or(u64::MAX);

    // Names: the English label, else the multilingual one.
    let named = format!(r#"SELECT ?g ?l (LANG(?l) AS ?lang) WHERE {{ {base} ?g rdfs:label ?l. FILTER(LANG(?l) IN ("en", "mul")) }}"#);
    for row in client.select(&named)? {
        let (Some(item), Some(name), Some(lang)) = (row.get("g"), row.get("l"), row.get("lang")) else { continue };
        let game = games.entry(number(item)).or_insert_with(|| Game { item: item.clone(), name: name.clone(), ..Default::default() });
        if lang == "en" {
            game.name = name.clone();
        }
    }
    // The earliest release year.
    let dates = format!("SELECT ?g ?d WHERE {{ {base} ?g wdt:P577 ?d. }}");
    for row in client.select(&dates)? {
        let Some(game) = row.get("g").and_then(|g| games.get_mut(&number(g))) else { continue };
        let year = row.get("d").and_then(|d| d.trim_start_matches('+').get(..4)?.parse::<u16>().ok()).filter(|y| (1950..=2100).contains(y));
        if let Some(year) = year {
            if game.year == 0 || year < game.year {
                game.year = year;
            }
        }
    }

    let aliases = format!(r#"SELECT ?g ?a WHERE {{ {base} ?g skos:altLabel ?a. FILTER(LANG(?a) IN ("en", "mul")) }}"#);
    for row in client.select(&aliases)? {
        if let (Some(game), Some(alias)) = (row.get("g").and_then(|g| games.get_mut(&number(g))), row.get("a")) {
            if *alias != game.name && !game.aliases.contains(alias) {
                game.aliases.push(alias.clone());
            }
        }
    }

    // Genres and modes by item (named in labels.tsv); companies and series
    // by name.
    for (property, field) in [("P136", 0), ("P404", 1), ("P178", 2), ("P123", 3), ("P179", 4)] {
        let by_item = field < 2;
        let query = if by_item {
            format!("SELECT ?g ?v WHERE {{ {base} ?g wdt:{property} ?v. }}")
        } else {
            format!("SELECT ?g ?v ?en ?mul WHERE {{ {base} ?g wdt:{property} ?v. {NAMED} }}")
        };
        for row in client.select(&query)? {
            let Some(game) = row.get("g").and_then(|g| games.get_mut(&number(g))) else { continue };
            let value = if by_item { row.get("v").filter(|v| v.starts_with('Q')).cloned() } else { name_of(&row) };
            let Some(value) = value else { continue };
            let list = match field {
                0 => &mut game.genres,
                1 => &mut game.modes,
                2 => &mut game.developers,
                3 => &mut game.publishers,
                _ => {
                    if game.series.is_empty() {
                        game.series = value;
                    }
                    continue;
                }
            };
            if !list.contains(&value) {
                list.push(value);
            }
        }
    }

    let sites: Vec<String> = WIKIS.iter().map(|l| format!("<https://{l}.wikipedia.org/>")).collect();
    let articles = format!(
        "SELECT ?g ?w ?t WHERE {{ {base} ?s schema:about ?g; schema:isPartOf ?w; schema:name ?t. VALUES ?w {{ {} }} }}",
        sites.join(" ")
    );
    for row in client.select(&articles)? {
        let (Some(game), Some(site), Some(title)) = (row.get("g").and_then(|g| games.get_mut(&number(g))), row.get("w"), row.get("t")) else { continue };
        let lang = site.trim_start_matches("https://").split('.').next().unwrap_or("").to_string();
        if !game.wikis.iter().any(|(l, _)| *l == lang) {
            game.wikis.push((lang, title.clone()));
        }
    }

    // Stable order, so a table changes only when the data does.
    let mut games: Vec<Game> = games.into_values().collect();
    for game in &mut games {
        game.aliases.sort();
        game.genres.sort();
        game.modes.sort();
        game.developers.sort();
        game.publishers.sort();
        game.wikis.sort_by_key(|(lang, _)| WIKIS.iter().position(|w| w == lang));
    }
    Ok(games)
}

/// The names of genre and mode items in the app's languages, the English
/// one (else the multilingual one) where a language has none.
fn labels(client: &mut Client, items: &BTreeSet<String>) -> Result<Vec<(String, [String; 5])>, String> {
    let mut names: BTreeMap<String, HashMap<String, String>> = BTreeMap::new();
    let langs: Vec<String> = LANGUAGES.iter().chain(&["mul"]).map(|l| format!("\"{l}\"")).collect();
    let list: Vec<&String> = items.iter().collect();
    for chunk in list.chunks(150) {
        let values: Vec<String> = chunk.iter().map(|i| format!("wd:{i}")).collect();
        let query = format!(
            "SELECT ?v ?l (LANG(?l) AS ?lang) WHERE {{ VALUES ?v {{ {} }} ?v rdfs:label ?l. FILTER(LANG(?l) IN ({})) }}",
            values.join(" "),
            langs.join(", ")
        );
        for row in client.select(&query)? {
            if let (Some(item), Some(label), Some(lang)) = (row.get("v"), row.get("l"), row.get("lang")) {
                names.entry(item.clone()).or_default().insert(lang.clone(), label.clone());
            }
        }
    }
    Ok(names
        .into_iter()
        .filter_map(|(item, by_lang)| {
            let fallback = by_lang.get("en").or_else(|| by_lang.get("mul"))?.clone();
            let row: [String; 5] = std::array::from_fn(|i| by_lang.get(LANGUAGES[i]).cloned().unwrap_or_else(|| fallback.clone()));
            Some((item, row))
        })
        .collect())
}

pub fn build(root: &Path, args: &[String]) -> Result<(), String> {
    let out = args.first().map(|a| root.join(a)).unwrap_or_else(|| root.join(".build/wikidata"));
    let only: Vec<&String> = args.iter().skip(1).collect();
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let built = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    let mut client = Client::new();
    let mut used = BTreeSet::new();
    let mut index = String::new();
    for (file, items) in wikidata::tables() {
        if !only.is_empty() && !only.iter().any(|o| **o == file) {
            continue;
        }
        eprintln!("{file} ({})", items.join(", "));
        let games = platform(&mut client, items)?;
        for game in &games {
            used.extend(game.genres.iter().cloned());
            used.extend(game.modes.iter().cloned());
        }
        let text = wikidata::render(&file, built, &games);
        std::fs::write(out.join(format!("{file}.tsv")), &text).map_err(|e| format!("{file}: {e}"))?;
        let count = |f: fn(&Game) -> bool| games.iter().filter(|g| f(g)).count();
        eprintln!(
            "  {} games, {} KB; genre {}, developer {}, year {}, article {}",
            games.len(),
            text.len() / 1024,
            count(|g| !g.genres.is_empty()),
            count(|g| !g.developers.is_empty()),
            count(|g| g.year > 0),
            count(|g| !g.wikis.is_empty())
        );
        index.push_str(&format!("{file}\t{}\t{built}\n", games.len()));
    }
    eprintln!("labels ({} items)", used.len());
    let labels = labels(&mut client, &used)?;
    std::fs::write(out.join(wikidata::LABELS_FILE), wikidata::render_labels(&labels)).map_err(|e| format!("labels: {e}"))?;
    std::fs::write(out.join("index.tsv"), index).map_err(|e| format!("index: {e}"))?;
    eprintln!("{} queries; tables in {}", client.queries, out.display());
    Ok(())
}

// --- Match rates --------------------------------------------------------------

/// A playlist's `(label, db_name)` items: plain JSON or RZIP.
fn playlist_items(path: &Path) -> Option<Vec<(String, String)>> {
    let bytes = std::fs::read(path).ok()?;
    let text = if bytes.starts_with(b"#RZIPv") {
        let size = u64::from_le_bytes(bytes.get(12..20)?.try_into().ok()?) as usize;
        let mut out = Vec::with_capacity(size);
        let mut at = 20;
        while out.len() < size && at + 4 <= bytes.len() {
            let length = u32::from_le_bytes(bytes[at..at + 4].try_into().ok()?) as usize;
            out.extend(miniz_oxide::inflate::decompress_to_vec_zlib(bytes.get(at + 4..at + 4 + length)?).ok()?);
            at += 4 + length;
        }
        String::from_utf8(out).ok()?
    } else {
        String::from_utf8(bytes).ok()?
    };
    let json: Value = serde_json::from_str(&text).ok()?;
    Some(
        json["items"]
            .as_array()?
            .iter()
            .map(|i| (i["label"].as_str().unwrap_or("").to_string(), i["db_name"].as_str().unwrap_or("").trim_end_matches(".lpl").to_string()))
            .collect(),
    )
}

/// Releases that are not a game's own (their names rarely are Wikidata's).
fn regular(name: &str) -> bool {
    !["(Beta", "(Proto", "(Unl", "(Pirate", "(Hack", "(Demo", "(Sample", "(Aftermarket", "(Program"].iter().any(|t| name.contains(t))
}

/// A built table, read once.
fn index_of<'a>(indexes: &'a mut HashMap<String, Option<Index>>, dir: &Path, file: &str) -> Option<&'a Index> {
    if !indexes.contains_key(file) {
        let index = std::fs::read_to_string(dir.join(format!("{file}.tsv"))).ok().and_then(|t| Index::parse(&t));
        indexes.insert(file.to_string(), index);
    }
    indexes.get(file).and_then(Option::as_ref)
}

pub fn check(root: &Path, args: &[String]) -> Result<(), String> {
    let dir = args.first().map(|a| root.join(a)).unwrap_or_else(|| root.join(".build/wikidata"));
    let mut indexes: HashMap<String, Option<Index>> = HashMap::new();
    println!("Sample playlists' labels:");
    let playlists = root.join(".samples/retroarch-playlists");
    let mut entries: Vec<_> = std::fs::read_dir(&playlists).map_err(|e| format!("{}: {e}", playlists.display()))?.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let Some(items) = playlist_items(&path) else { continue };
        let database = items.iter().map(|(_, db)| db.clone()).find(|db| !db.is_empty()).unwrap_or(stem.clone());
        let file = wikidata::file_for(&database);
        let Some(index) = index_of(&mut indexes, &dir, &file) else {
            println!("  {stem}: no table {file}.tsv");
            continue;
        };
        let found = items.iter().filter(|(label, _)| index.find(label).is_some()).count();
        println!("  {stem}: {found} of {} ({}%) in {file}.tsv ({} games)", items.len(), 100 * found / items.len().max(1), index.games.len());
    }
    println!("libretro's database names (regular releases):");
    let samples = root.join(".samples/gamedb");
    let mut rdbs: Vec<_> = std::fs::read_dir(&samples).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    rdbs.sort();
    for path in rdbs {
        let database = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let file = wikidata::file_for(&database);
        let Some(index) = index_of(&mut indexes, &dir, &file) else { continue };
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let Ok(rows) = shelf_core::online::gamedb::read_rdb(&bytes) else { continue };
        let names: BTreeSet<&str> = rows.iter().map(|r| r.name.as_str()).filter(|n| regular(n)).collect();
        let found = names.iter().filter(|n| index.find(n).is_some()).count();
        // Distinct games: names that are one game in several regions.
        let games: BTreeSet<&str> = names.iter().filter_map(|n| index.find(n)).map(|g| g.item.as_str()).collect();
        println!("  {database}: {found} of {} names ({}%), {} distinct games of {}", names.len(), 100 * found / names.len().max(1), games.len(), index.games.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_rows_lose_the_entity_prefix() {
        let json: Value = serde_json::from_str(
            r#"{"results":{"bindings":[{"g":{"type":"uri","value":"http://www.wikidata.org/entity/Q14625"},"l":{"type":"literal","xml:lang":"en","value":"Zelda"}},{"g":{"type":"uri","value":"http://www.wikidata.org/entity/Q1"}}]}}"#,
        )
        .unwrap();
        let rows = rows(&json);
        assert_eq!((rows[0]["g"].as_str(), rows[0]["l"].as_str()), ("Q14625", "Zelda"));
        assert_eq!(rows[1].get("l"), None);
        assert!(games_of(&["Q172742", "Q135321"]).starts_with("VALUES ?p { wd:Q172742 wd:Q135321 }"));
    }

    #[test]
    fn regular_releases_leave_out_prototypes_and_hacks() {
        assert!(regular("Super Mario Bros. (World)"));
        assert!(!regular("Super Mario Bros. (World) (Proto)"));
        assert!(!regular("Some Game (Unl)"));
    }
}
