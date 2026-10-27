//! `settings.json`: read value by value, so a stale or
//! hand-edited file degrades to defaults; written in full, in one line.

use serde_json::{Map, Value};

use crate::theme::{ThemeId, DEFAULT_THEME};

/// An enum saved as one of a fixed set of strings.
macro_rules! keyed {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $key:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn key(self) -> &'static str {
                match self { $($name::$variant => $key),+ }
            }

            pub fn from_key(key: &str) -> Option<$name> {
                Self::ALL.iter().copied().find(|v| v.key() == key)
            }
        }
    };
}

keyed!(Language { En = "en", Tr = "tr", De = "de", Fr = "fr", Es = "es" });
keyed!(
    /// The shelf's views, in the View row's order.
    ViewMode { Home = "home", Carousel = "carousel", Grid = "grid", List = "list", Cascade = "cascade" }
);
keyed!(
    /// How much text each title shows.
    DetailLevel { Basic = "basic", Normal = "normal", Detailed = "detailed" }
);
keyed!(ConfirmMode { Circle = "circle", Cross = "cross" });
keyed!(
    /// The system's 12- or 24-hour setting, or a fixed one.
    ClockFormat { System = "system", H24 = "24", H12 = "12" }
);
keyed!(UpdateChannel { Stable = "stable", Beta = "beta", Nightly = "nightly", Off = "off" });
keyed!(
    /// Where artwork is found: libretro for RetroArch games and SteamGridDB
    /// for the rest, or one of them for every title.
    ScrapeSource { Auto = "auto", SteamGridDb = "steamgriddb", Libretro = "libretro" }
);

pub const SETTINGS_FILE: &str = "settings.json";

keyed!(
    /// Where the status items sit in the top bar.
    StatusPosition { Right = "right", Left = "left" }
);

/// Which status items show and where (a setting of its own since 0.2, in
/// `status-bar.json`: Pocket Shelf 0.1 rewrote `settings.json` with only
/// the keys it knows, which would drop these).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StatusLayout {
    pub position: StatusPosition,
    pub wifi: bool,
    pub bluetooth: bool,
    pub battery: bool,
    pub clock: bool,
}

pub const STATUS_BAR_FILE: &str = "status-bar.json";

impl Default for StatusLayout {
    fn default() -> Self {
        StatusLayout { position: StatusPosition::Right, wifi: true, bluetooth: true, battery: true, clock: true }
    }
}

impl StatusLayout {
    /// The saved layout; a missing or unusable value keeps its default.
    pub fn parse(bytes: &[u8]) -> StatusLayout {
        let mut layout = StatusLayout::default();
        let Ok(raw) = serde_json::from_slice::<Value>(bytes) else { return layout };
        if let Some(p) = raw.get("position").and_then(Value::as_str).and_then(StatusPosition::from_key) {
            layout.position = p;
        }
        let flag = |key: &str, default: bool| raw.get(key).and_then(Value::as_bool).unwrap_or(default);
        layout.wifi = flag("wifi", layout.wifi);
        layout.bluetooth = flag("bluetooth", layout.bluetooth);
        layout.battery = flag("battery", layout.battery);
        layout.clock = flag("clock", layout.clock);
        layout
    }

    /// Two-space JSON and a newline, as the app's other files.
    pub fn render(&self) -> String {
        let mut map = Map::new();
        map.insert("position".into(), self.position.key().into());
        map.insert("wifi".into(), self.wifi.into());
        map.insert("bluetooth".into(), self.bluetooth.into());
        map.insert("battery".into(), self.battery.into());
        map.insert("clock".into(), self.clock.into());
        serde_json::to_string_pretty(&Value::Object(map)).expect("json") + "\n"
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Settings {
    pub language: Language,
    pub theme: ThemeId,
    pub view: ViewMode,
    pub detail: DetailLevel,
    pub confirm: ConfirmMode,
    /// The selected title's picture behind the screen.
    pub backdrop: bool,
    /// A box behind an icon with transparent parts.
    pub icon_box: bool,
    pub status_bar: bool,
    pub clock: ClockFormat,
    pub battery_percent: bool,
    pub updates: UpdateChannel,
    pub scraper: ScrapeSource,
    /// Category and title selected when the settings were saved.
    pub category: Option<String>,
    pub title: Option<String>,
    /// The status items' layout, saved in its own file (not rendered here).
    pub layout: StatusLayout,
}

impl Settings {
    /// A fresh install's settings. `updates` is the build's own channel (Off
    /// for a local build).
    pub fn defaults(updates: UpdateChannel) -> Settings {
        Settings {
            language: Language::En,
            theme: DEFAULT_THEME,
            view: ViewMode::Home,
            detail: DetailLevel::Normal,
            confirm: ConfirmMode::Circle,
            backdrop: true,
            icon_box: true,
            status_bar: true,
            clock: ClockFormat::System,
            battery_percent: true,
            updates,
            scraper: ScrapeSource::Auto,
            category: None,
            title: None,
            layout: StatusLayout::default(),
        }
    }

    /// `defaults` with each usable value of a saved file over it.
    pub fn parse(bytes: &[u8], defaults: Settings) -> Settings {
        let mut s = defaults;
        let Ok(raw) = serde_json::from_slice::<Value>(bytes) else { return s };
        let text = |key: &str| raw.get(key).and_then(Value::as_str);
        let flag = |key: &str| raw.get(key).and_then(Value::as_bool);
        if let Some(v) = text("language").and_then(Language::from_key) {
            s.language = v;
        }
        if let Some(v) = text("theme").and_then(ThemeId::from_key) {
            s.theme = v;
        }
        if let Some(v) = text("view").and_then(ViewMode::from_key) {
            s.view = v;
        }
        if let Some(v) = text("detail").and_then(DetailLevel::from_key) {
            s.detail = v;
        }
        if let Some(v) = text("confirm").and_then(ConfirmMode::from_key) {
            s.confirm = v;
        }
        s.backdrop = flag("backdrop").unwrap_or(s.backdrop);
        s.icon_box = flag("iconBox").unwrap_or(s.icon_box);
        s.status_bar = flag("statusBar").unwrap_or(s.status_bar);
        if let Some(v) = text("clock").and_then(ClockFormat::from_key) {
            s.clock = v;
        }
        s.battery_percent = flag("batteryPercent").unwrap_or(s.battery_percent);
        if let Some(v) = text("updates").and_then(UpdateChannel::from_key) {
            s.updates = v;
        }
        if let Some(v) = text("scraper").and_then(ScrapeSource::from_key) {
            s.scraper = v;
        }
        if let Some(v) = text("category") {
            s.category = Some(v.into());
        }
        if let Some(v) = text("title") {
            s.title = Some(v.into());
        }
        s
    }

    /// The file as Pocket Shelf 0.1 wrote it: one line, no newline; a key
    /// with no value is left out.
    pub fn render(&self) -> String {
        let mut map = Map::new();
        let mut put = |key: &str, value: Value| {
            map.insert(key.into(), value);
        };
        put("language", self.language.key().into());
        put("theme", self.theme.key().into());
        put("view", self.view.key().into());
        put("detail", self.detail.key().into());
        put("confirm", self.confirm.key().into());
        put("backdrop", self.backdrop.into());
        put("iconBox", self.icon_box.into());
        put("statusBar", self.status_bar.into());
        put("clock", self.clock.key().into());
        put("batteryPercent", self.battery_percent.into());
        put("updates", self.updates.key().into());
        put("scraper", self.scraper.key().into());
        if let Some(category) = &self.category {
            put("category", category.as_str().into());
        }
        if let Some(title) = &self.title {
            put("title", title.as_str().into());
        }
        serde_json::to_string(&Value::Object(map)).expect("json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_layout_round_trips_and_defaults() {
        let layout = StatusLayout { position: StatusPosition::Left, wifi: false, bluetooth: true, battery: false, clock: true };
        assert_eq!(StatusLayout::parse(layout.render().as_bytes()), layout);
        assert_eq!(layout.render(), "{\n  \"position\": \"left\",\n  \"wifi\": false,\n  \"bluetooth\": true,\n  \"battery\": false,\n  \"clock\": true\n}\n");
        assert_eq!(StatusLayout::parse(br#"{"position":"top","clock":false}"#), StatusLayout { clock: false, ..Default::default() });
        assert_eq!(StatusLayout::parse(b"nope"), StatusLayout::default());
    }

    #[test]
    fn unusable_values_keep_the_defaults() {
        let defaults = Settings::defaults(UpdateChannel::Off);
        let text = br#"{"language":"xx","theme":"aurora","view":7,"backdrop":"no","clock":"24","font":"mono","title":null}"#;
        let s = Settings::parse(text, defaults.clone());
        assert_eq!(s.theme, ThemeId::Aurora);
        assert_eq!(s.clock, ClockFormat::H24);
        assert_eq!(Settings { theme: defaults.theme, clock: defaults.clock, ..s }, defaults);
        assert_eq!(Settings::parse(b"{broken", defaults.clone()), defaults);
    }
}
