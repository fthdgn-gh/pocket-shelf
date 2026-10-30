//! When titles were played and installed: Pocket Shelf's own launches
//! (`plays.json`, since 0.2) and the system's dates from the home screen's
//! database (`app.db`), read by the host at each start.

use serde_json::{Map, Value};

pub const PLAYS_FILE: &str = "plays.json";

/// One title's launches through Pocket Shelf.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Play {
    pub count: u32,
    /// Milliseconds since 1970, UTC.
    pub last: u64,
}

/// What the system's database records for a title, in milliseconds since
/// 1970, UTC.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SystemDates {
    /// Last started by the system (`tbl_appinfo` key 1281097414). A
    /// RetroArch game counts as RetroArch, a PSP game without a bubble as
    /// Adrenaline.
    pub launched: Option<u64>,
    /// First installed (key 2722548093).
    pub installed: Option<u64>,
}

/// `tbl_appinfo`'s key for the last time the system started a title.
pub const KEY_LAUNCHED: u32 = 1281097414;
/// `tbl_appinfo`'s key for the time a title was first installed.
pub const KEY_INSTALLED: u32 = 2722548093;

/// `^[A-Za-z0-9]{9}$`, the ids `recent.json` keeps (RetroArch's `ra…` too).
fn valid(id: &str) -> bool {
    id.len() == 9 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// The launches in `plays.json`: `{"<id>": {"count": 3, "last": <ms>}}`.
/// Entries that do not parse are left out.
pub fn parse(bytes: &[u8]) -> Vec<(String, Play)> {
    let Ok(Value::Object(map)) = serde_json::from_slice::<Value>(bytes) else { return Vec::new() };
    map.iter()
        .filter(|(id, _)| valid(id))
        .filter_map(|(id, v)| {
            let count = v.get("count")?.as_u64()?.min(u32::MAX as u64) as u32;
            let last = v.get("last")?.as_u64()?;
            Some((id.clone(), Play { count, last }))
        })
        .collect()
}

/// Two-space JSON and a newline, ids in order.
pub fn render(plays: &[(String, Play)]) -> String {
    let mut sorted: Vec<&(String, Play)> = plays.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut map = Map::new();
    for (id, play) in sorted {
        let mut entry = Map::new();
        entry.insert("count".into(), play.count.into());
        entry.insert("last".into(), play.last.into());
        map.insert(id.clone(), Value::Object(entry));
    }
    serde_json::to_string_pretty(&Value::Object(map)).expect("json") + "\n"
}

/// `plays` with one more launch of `id` at `now`.
pub fn record(plays: &[(String, Play)], id: &str, now: u64) -> Vec<(String, Play)> {
    let mut next = plays.to_vec();
    match next.iter_mut().find(|(other, _)| other == id) {
        Some((_, play)) => {
            play.count = play.count.saturating_add(1);
            play.last = play.last.max(now);
        }
        None => next.push((id.to_string(), Play { count: 1, last: now })),
    }
    next
}

/// A time as the system's database writes it, `2026-10-30 16:05:07.842`
/// (UTC), in milliseconds since 1970.
pub fn parse_db_time(text: &str) -> Option<u64> {
    let text = text.trim();
    let (date, time) = text.split_once(' ')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>().ok());
    let (year, month, day) = (d.next()??, d.next()??, d.next()??);
    let (clock, millis) = time.split_once('.').unwrap_or((time, "0"));
    let mut t = clock.split(':').map(|p| p.parse::<i64>().ok());
    let (hour, minute, second) = (t.next()??, t.next()??, t.next()??);
    let millis: i64 = format!("{millis:0<3}").get(..3)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let ms = ((days * 24 + hour) * 60 + minute) * 60_000 + second * 1000 + millis;
    u64::try_from(ms).ok()
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_times_are_read_as_utc_milliseconds() {
        assert_eq!(parse_db_time("1970-01-01 00:00:00.000"), Some(0));
        // `date -u -d '2026-10-30 16:05:07' +%s` is 1793376307.
        assert_eq!(parse_db_time("2026-10-30 16:05:07.842"), Some(1_793_376_307_842));
        assert_eq!(parse_db_time("2024-02-29 12:00:00"), Some(1_709_208_000_000));
        assert_eq!(parse_db_time("2026-13-01 00:00:00.000"), None);
        assert_eq!(parse_db_time("gd"), None);
        assert_eq!(parse_db_time(""), None);
    }

    #[test]
    fn launches_round_trip_and_count_up() {
        let plays = record(&[], "PCSA00069", 5);
        let plays = record(&plays, "PCSA00069", 9);
        let plays = record(&plays, "ra00000001", 7);
        let plays = record(&plays, "ra0000001", 7);
        assert_eq!(render(&plays), "{\n  \"PCSA00069\": {\n    \"count\": 2,\n    \"last\": 9\n  },\n  \"ra00000001\": {\n    \"count\": 1,\n    \"last\": 7\n  },\n  \"ra0000001\": {\n    \"count\": 1,\n    \"last\": 7\n  }\n}\n");
        // The ten-letter id is not one Pocket Shelf lists: dropped on read.
        let read = parse(render(&plays).as_bytes());
        assert_eq!(read.len(), 2);
        assert_eq!(read.iter().find(|(id, _)| id == "PCSA00069").map(|p| p.1), Some(Play { count: 2, last: 9 }));
        assert!(parse(b"nope").is_empty());
        assert!(parse(br#"{"PCSA00069":{"count":"x"}}"#).is_empty());
    }
}
