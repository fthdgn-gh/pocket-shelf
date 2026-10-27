//! The app's own updates: which build this is, the
//! releases on GitHub, which one a channel offers, and when a check at start
//! is due.

use serde_json::Value;

use crate::host::Host;
use crate::library::settings::UpdateChannel;

/// This build, compiled in by `vita/build.rs`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuildInfo {
    pub version: String,
    /// "stable", "beta", "nightly" or "local".
    pub channel: String,
    pub commit: String,
}

impl BuildInfo {
    pub fn local() -> BuildInfo {
        BuildInfo { version: "0.0.0".into(), channel: "local".into(), commit: String::new() }
    }

    /// The channel a fresh install checks: the one it was released on; a
    /// local build checks none.
    pub fn default_channel(&self) -> UpdateChannel {
        UpdateChannel::from_key(&self.channel).unwrap_or(UpdateChannel::Off)
    }
}

const REPOSITORY: &str = "fthdgn-gh/pocket-shelf";
/// The newest releases, newest first. Ten reach past any run of betas.
pub fn releases_url() -> String {
    format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=10")
}
pub const NIGHTLY_TAG: &str = "shelf-nightly";
const VPK_NAME: &str = "pocket-shelf.vpk";
/// Where the download goes, under the data folder; the Vita host's `http`
/// accepts only this name for a package.
pub const UPDATE_FILE: &str = "update/pocket-shelf.vpk";
/// The release's version for the updater's screen (`updater/main.c`).
pub const UPDATE_VERSION_FILE: &str = "update/version.txt";

/// A release that can be installed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Release {
    pub tag: String,
    /// "0.1.0-beta.1"; a nightly has none.
    pub version: Option<String>,
    /// The commit a nightly was built from.
    pub commit: Option<String>,
    pub title: String,
    /// "2026-10-05", the day it was published.
    pub date: String,
    pub url: String,
    pub size: u64,
}

/// `^shelf-v(\d+\.\d+\.\d+(?:-beta\.\d+)?)$`
fn tag_version(tag: &str) -> Option<&str> {
    let version = tag.strip_prefix("shelf-v")?;
    let (core, beta) = match version.split_once("-beta.") {
        Some((core, beta)) => (core, Some(beta)),
        None => (version, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let parts: Vec<&str> = core.split('.').collect();
    (parts.len() == 3 && parts.iter().all(|p| digits(p)) && beta.is_none_or(digits)).then_some(version)
}

/// The installable releases of a reply to [`releases_url`]: published, with
/// the VPK attached, and tagged `shelf-v<version>` or `shelf-nightly`.
/// `None` for a reply that is not the list GitHub sends.
pub fn parse_releases(text: &str) -> Option<Vec<Release>> {
    let raw: Value = serde_json::from_str(text).ok()?;
    let items = raw.as_array()?;
    let mut releases = Vec::new();
    for item in items {
        let Some(tag) = item.get("tag_name").and_then(Value::as_str) else { continue };
        if item.get("draft") == Some(&Value::Bool(true)) {
            continue;
        }
        let assets = item.get("assets").and_then(Value::as_array);
        let Some(asset) = assets.and_then(|a| a.iter().find(|e| e.get("name").and_then(Value::as_str) == Some(VPK_NAME))) else { continue };
        let (Some(url), Some(size)) = (asset.get("browser_download_url").and_then(Value::as_str), asset.get("size").and_then(Value::as_f64)) else { continue };
        if !url.starts_with("https://") {
            continue;
        }
        let version = tag_version(tag).map(String::from);
        let commit = item.get("target_commitish").and_then(Value::as_str).unwrap_or("");
        let nightly = tag == NIGHTLY_TAG && commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if version.is_none() && !nightly {
            continue;
        }
        let name = item.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).unwrap_or(tag);
        let date = item.get("published_at").and_then(Value::as_str).map(|d| d.chars().take(10).collect()).unwrap_or_default();
        releases.push(Release {
            tag: tag.into(),
            commit: version.is_none().then(|| commit.to_string()),
            version,
            title: name.into(),
            date,
            url: url.into(),
            size: size as u64,
        });
    }
    Some(releases)
}

/// The order of two versions; a beta comes before its release.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |version: &str| -> Vec<u64> {
        let (core, beta) = match version.split_once("-beta.") {
            Some((core, beta)) => (core, Some(beta)),
            None => (version, None),
        };
        let mut parts: Vec<u64> = core.split('.').map(|p| p.parse().unwrap_or(0)).collect();
        parts.push(beta.map_or(u64::MAX, |b| b.parse().unwrap_or(0)));
        parts
    };
    let (left, right) = (parse(a), parse(b));
    for i in 0..left.len().max(right.len()) {
        let order = left.get(i).unwrap_or(&0).cmp(right.get(i).unwrap_or(&0));
        if order.is_ne() {
            return order;
        }
    }
    std::cmp::Ordering::Equal
}

/// The release the channel offers over `build`, or `None` when there is
/// none or it is what is installed. Stable takes the newest release without
/// a beta number, beta the newest of all tagged releases; either only when
/// it is newer than the installed version. Nightly takes `shelf-nightly`
/// when it was built from another commit.
pub fn pick_update<'a>(releases: &'a [Release], channel: UpdateChannel, build: &BuildInfo) -> Option<&'a Release> {
    match channel {
        UpdateChannel::Off => None,
        UpdateChannel::Nightly => releases.iter().find(|r| r.tag == NIGHTLY_TAG).filter(|r| r.commit.as_deref() != Some(build.commit.as_str())),
        UpdateChannel::Stable | UpdateChannel::Beta => releases
            .iter()
            .filter(|r| r.version.as_deref().is_some_and(|v| channel == UpdateChannel::Beta || !v.contains("-beta.")))
            .max_by(|a, b| compare_versions(a.version.as_deref().unwrap_or(""), b.version.as_deref().unwrap_or("")))
            .filter(|r| compare_versions(r.version.as_deref().unwrap_or(""), &build.version).is_gt()),
    }
}

/// A name for a release in the drawer: "0.1.0-beta.1", or the nightly's day
/// and commit.
pub fn release_label(release: &Release) -> String {
    match &release.version {
        Some(version) => version.clone(),
        None => format!("{} ({})", release.date, release.commit.as_deref().unwrap_or("").chars().take(7).collect::<String>()),
    }
}

/// The installed build, in the same form.
pub fn build_label(build: &BuildInfo) -> String {
    if build.channel == "nightly" || build.channel == "local" {
        let short: String = build.commit.chars().take(7).collect();
        format!("{} ({})", build.version, if short.is_empty() { build.channel.clone() } else { short })
    } else {
        build.version.clone()
    }
}

/// The key `later` stores: the tag, and for a nightly its commit, since its
/// tag stays the same.
pub fn release_key(release: &Release) -> String {
    format!("{}@{}", release.tag, release.commit.as_deref().unwrap_or(""))
}

// --- When to check -----------------------------------------------------------

/// A check at start waits this long after the last one.
pub const CHECK_INTERVAL_MS: u64 = 24 * 60 * 60 * 1000;

pub const RECORD_FILE: &str = "update.json";

/// What `update.json` keeps between starts.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct UpdateRecord {
    /// When the last check finished, in ms since 1970.
    pub checked_at: u64,
    /// A release the user put off: a check at start does not offer it again.
    pub later: Option<String>,
}

impl UpdateRecord {
    pub fn load(host: &mut dyn Host) -> UpdateRecord {
        let Some(raw) = host.read(RECORD_FILE).and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { return UpdateRecord::default() };
        UpdateRecord {
            checked_at: raw.get("checkedAt").and_then(Value::as_f64).map_or(0, |n| n as u64),
            later: raw.get("later").and_then(Value::as_str).map(String::from),
        }
    }

    /// `JSON.stringify` of the record, as Pocket Shelf 0.1 wrote it.
    pub fn render(&self) -> String {
        let mut map = serde_json::Map::new();
        map.insert("checkedAt".into(), Value::from(self.checked_at));
        if let Some(later) = &self.later {
            map.insert("later".into(), Value::from(later.clone()));
        }
        Value::Object(map).to_string()
    }

    pub fn save(&self, host: &mut dyn Host) {
        if !host.write(RECORD_FILE, self.render().as_bytes()) {
            host.log("Update record not saved");
        }
    }
}

/// Whether a check at start is due: a channel is chosen and the last check
/// is a day old (or the clock was set back).
pub fn check_due(channel: UpdateChannel, record: &UpdateRecord, now: u64) -> bool {
    channel != UpdateChannel::Off && (now.saturating_sub(record.checked_at) >= CHECK_INTERVAL_MS || now < record.checked_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NIGHTLY_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const OTHER_COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn release(tag: &str, extra: &str) -> String {
        let base = format!(
            r#""tag_name":"{tag}","name":"Pocket Shelf {tag}","draft":false,"target_commitish":"main","published_at":"2026-10-05T19:35:55Z","assets":[{{"name":"pocket-shelf.vpk","browser_download_url":"https://github.com/x/releases/download/{tag}/pocket-shelf.vpk","size":2450654}}]"#
        );
        // Later keys win in serde_json, as in a spread.
        if extra.is_empty() {
            format!("{{{base}}}")
        } else {
            format!("{{{base},{extra}}}")
        }
    }

    fn reply() -> String {
        let items = [
            release("shelf-nightly", &format!(r#""target_commitish":"{NIGHTLY_COMMIT}""#)),
            release("shelf-v0.2.0-beta.1", ""),
            release("shelf-v0.1.0", ""),
            release("shelf-v0.1.0-beta.2", ""),
            release("shelf-v0.1.0-beta.1", ""),
            release("shelf-v0.3.0", r#""draft":true"#),
            release("shelf-v0.2.5", r#""assets":[]"#),
            release("v0.14.0", ""),
        ];
        format!("[{}]", items.join(","))
    }

    fn build(version: &str, channel: &str, commit: &str) -> BuildInfo {
        BuildInfo { version: version.into(), channel: channel.into(), commit: commit.into() }
    }

    #[test]
    fn published_shelf_releases_with_the_vpk_are_kept() {
        let releases = parse_releases(&reply()).unwrap();
        let tags: Vec<&str> = releases.iter().map(|r| r.tag.as_str()).collect();
        assert_eq!(tags, ["shelf-nightly", "shelf-v0.2.0-beta.1", "shelf-v0.1.0", "shelf-v0.1.0-beta.2", "shelf-v0.1.0-beta.1"]);
        let (nightly, beta) = (&releases[0], &releases[1]);
        assert_eq!((nightly.version.as_deref(), nightly.commit.as_deref()), (None, Some(NIGHTLY_COMMIT)));
        assert_eq!((beta.version.as_deref(), beta.date.as_str(), beta.size), (Some("0.2.0-beta.1"), "2026-10-05", 2450654));
        // A nightly without a commit id is left out.
        assert_eq!(parse_releases(&format!("[{}]", release("shelf-nightly", ""))), Some(Vec::new()));
        // A reply that is not the list.
        assert_eq!(parse_releases(r#"{"message":"API rate limit exceeded"}"#), None);
        assert_eq!(parse_releases("<html>"), None);
    }

    #[test]
    fn betas_come_before_their_release() {
        let mut versions = ["0.1.0", "0.2.0-beta.1", "0.1.0-beta.10", "0.1.0-beta.2", "0.10.0"];
        versions.sort_by(|a, b| compare_versions(a, b));
        assert_eq!(versions, ["0.1.0-beta.2", "0.1.0-beta.10", "0.1.0", "0.2.0-beta.1", "0.10.0"]);
        assert!(compare_versions("0.1.0-beta.1", "0.1.0-beta.1").is_eq());
    }

    #[test]
    fn each_channel_offers_its_release() {
        let releases = parse_releases(&reply()).unwrap();
        let tag = |c, b: BuildInfo| pick_update(&releases, c, &b).map(|r| r.tag.clone());
        use UpdateChannel::*;
        assert_eq!(tag(Stable, build("0.1.0-beta.1", "beta", OTHER_COMMIT)).as_deref(), Some("shelf-v0.1.0"));
        assert_eq!(tag(Stable, build("0.1.0", "stable", OTHER_COMMIT)), None);
        assert_eq!(tag(Beta, build("0.1.0", "stable", OTHER_COMMIT)).as_deref(), Some("shelf-v0.2.0-beta.1"));
        assert_eq!(tag(Beta, build("0.2.0", "stable", OTHER_COMMIT)), None);
        assert_eq!(tag(Nightly, build("0.1.0", "nightly", OTHER_COMMIT)).as_deref(), Some("shelf-nightly"));
        assert_eq!(tag(Nightly, build("0.1.0", "nightly", NIGHTLY_COMMIT)), None);
        assert_eq!(tag(Off, build("0.0.1", "stable", OTHER_COMMIT)), None);
    }

    #[test]
    fn a_check_at_start_waits_a_day() {
        let now = 1_800_000_000_000;
        let at = |checked_at| UpdateRecord { checked_at, later: None };
        assert!(!check_due(UpdateChannel::Beta, &at(now - CHECK_INTERVAL_MS + 1), now));
        assert!(check_due(UpdateChannel::Beta, &at(now - CHECK_INTERVAL_MS), now));
        // A clock set back checks again.
        assert!(check_due(UpdateChannel::Beta, &at(now + 1000), now));
        assert!(!check_due(UpdateChannel::Off, &at(0), now));
    }

    #[test]
    fn labels_and_the_record() {
        let releases = parse_releases(&reply()).unwrap();
        assert_eq!(release_label(&releases[0]), "2026-10-05 (aaaaaaa)");
        assert_eq!(release_key(&releases[1]), "shelf-v0.2.0-beta.1@");
        assert_eq!(build_label(&build("0.1.0", "local", "")), "0.1.0 (local)");
        assert_eq!(build_label(&build("0.1.0", "nightly", OTHER_COMMIT)), "0.1.0 (bbbbbbb)");
        let record = UpdateRecord { checked_at: 1_800_000_000_000, later: Some("shelf-nightly@abc".into()) };
        assert_eq!(record.render(), r#"{"checkedAt":1800000000000,"later":"shelf-nightly@abc"}"#);
    }
}
