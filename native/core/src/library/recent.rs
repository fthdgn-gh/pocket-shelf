//! "Last Played": `recent.json`, title ids, the one started last first
//! (`src/recent.ts`).

use serde_json::Value;

pub const RECENT_MAX: usize = 15;
pub const RECENT_FILE: &str = "recent.json";

/// `list` with `id` moved to the front, cut to `max` ids.
pub fn push_recent(list: &[String], id: &str, max: usize) -> Vec<String> {
    std::iter::once(id.to_string()).chain(list.iter().filter(|other| *other != id).cloned()).take(max).collect()
}

/// `^[A-Za-z0-9]{9}$`
fn valid(id: &str) -> bool {
    id.len() == 9 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// Valid ids from the file, each once, at most [`RECENT_MAX`].
pub fn parse(bytes: &[u8]) -> Vec<String> {
    let Ok(Value::Array(items)) = serde_json::from_slice::<Value>(bytes) else { return Vec::new() };
    let mut ids: Vec<String> = Vec::new();
    for id in items.iter().filter_map(Value::as_str).filter(|id| valid(id)) {
        if !ids.iter().any(|seen| seen == id) {
            ids.push(id.into());
        }
    }
    ids.truncate(RECENT_MAX);
    ids
}

pub fn render(list: &[String]) -> String {
    serde_json::to_string(list).expect("json") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn push_puts_the_started_title_first_once() {
        assert_eq!(push_recent(&[], "PCSA00001", RECENT_MAX), ids(&["PCSA00001"]));
        assert_eq!(push_recent(&ids(&["PCSA00001", "PCSA00002"]), "PCSA00003", RECENT_MAX), ids(&["PCSA00003", "PCSA00001", "PCSA00002"]));
        assert_eq!(
            push_recent(&ids(&["PCSA00001", "PCSA00002", "PCSA00003"]), "PCSA00003", RECENT_MAX),
            ids(&["PCSA00003", "PCSA00001", "PCSA00002"])
        );
    }

    #[test]
    fn push_drops_the_oldest_past_the_limit() {
        let full: Vec<String> = (0..RECENT_MAX).map(|i| format!("PCSA{i:05}")).collect();
        let next = push_recent(&full, "PCSB00000", RECENT_MAX);
        assert_eq!(next.len(), RECENT_MAX);
        assert_eq!(next[0], "PCSB00000");
        assert!(!next.contains(&full[RECENT_MAX - 1]));
    }

    #[test]
    fn parse_keeps_valid_ids_once() {
        assert_eq!(parse(br#"["PCSA00001",1,"short","PCSA00001","ra0k3x9zq"]"#), ids(&["PCSA00001", "ra0k3x9zq"]));
        assert!(parse(b"{}").is_empty());
    }
}
