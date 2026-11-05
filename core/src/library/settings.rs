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
        let Ok(raw) = crate::library::read_json(bytes) else { return layout };
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

keyed!(
    /// The size of the text, in five steps around the roles' own sizes.
    TextSize { Smaller = "smaller", Small = "small", Normal = "normal", Large = "large", Larger = "larger" }
);

/// The text size, a setting of its own since 0.2 for the reason
/// `status-bar.json` is: `{"size": "normal"}`.
pub const TEXT_SIZE_FILE: &str = "text-size.json";

impl TextSize {
    /// The factor on each role's size (`Gfx::text_scale`).
    pub fn scale(self) -> f32 {
        match self {
            TextSize::Smaller => 0.85,
            TextSize::Small => 0.92,
            TextSize::Normal => 1.0,
            TextSize::Large => 1.12,
            TextSize::Larger => 1.25,
        }
    }

    /// The saved size; Normal when the file is missing or unusable.
    pub fn parse(bytes: &[u8]) -> TextSize {
        crate::library::read_json(bytes)
            .ok()
            .and_then(|raw| raw.get("size").and_then(Value::as_str).and_then(TextSize::from_key))
            .unwrap_or(TextSize::Normal)
    }

    /// Two-space JSON and a newline, as the app's other files.
    pub fn render(self) -> String {
        let mut map = Map::new();
        map.insert("size".into(), self.key().into());
        serde_json::to_string_pretty(&Value::Object(map)).expect("json") + "\n"
    }
}

keyed!(
    /// The name a title identified in a game database shows: its own (the
    /// playlist label or the title's name), the database's, or the
    /// database's without the brackets.
    GameNames { Playlist = "playlist", Database = "database", Short = "short" }
);

/// What the game database adds to the shelf, a setting of its own for the
/// reason `status-bar.json` is: `{"names": "playlist", "details": true}`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameInfo {
    pub names: GameNames,
    /// The details panel next to the selected title.
    pub details: bool,
}

pub const GAME_INFO_FILE: &str = "game-info.json";

impl Default for GameInfo {
    fn default() -> Self {
        GameInfo { names: GameNames::Playlist, details: true }
    }
}

impl GameInfo {
    pub fn parse(bytes: &[u8]) -> GameInfo {
        let mut info = GameInfo::default();
        let Ok(raw) = crate::library::read_json(bytes) else { return info };
        if let Some(names) = raw.get("names").and_then(Value::as_str).and_then(GameNames::from_key) {
            info.names = names;
        }
        if let Some(details) = raw.get("details").and_then(Value::as_bool) {
            info.details = details;
        }
        info
    }

    /// Two-space JSON and a newline, as the app's other files.
    pub fn render(self) -> String {
        let mut map = Map::new();
        map.insert("names".into(), self.names.key().into());
        map.insert("details".into(), self.details.into());
        serde_json::to_string_pretty(&Value::Object(map)).expect("json") + "\n"
    }
}

keyed!(
    /// The keyboard text is typed with: the Vita's own, or Pocket Shelf's.
    KeyboardKind { System = "system", Own = "pocket-shelf" }
);

/// The keyboard choice, a setting of its own since 0.2 for the reason
/// `status-bar.json` is: `{"keyboard": "system"}`.
pub const KEYBOARD_FILE: &str = "keyboard.json";

impl KeyboardKind {
    /// The saved choice; Pocket Shelf's keyboard when the file is missing
    /// or unusable.
    pub fn parse(bytes: &[u8]) -> KeyboardKind {
        crate::library::read_json(bytes)
            .ok()
            .and_then(|raw| raw.get("keyboard").and_then(Value::as_str).and_then(KeyboardKind::from_key))
            .unwrap_or(KeyboardKind::Own)
    }

    /// Two-space JSON and a newline, as the app's other files.
    pub fn render(self) -> String {
        let mut map = Map::new();
        map.insert("keyboard".into(), self.key().into());
        serde_json::to_string_pretty(&Value::Object(map)).expect("json") + "\n"
    }
}

keyed!(
    /// The app that starts a game both RetroArch and Emu4Vita++ can open.
    Emulator { RetroArch = "retroarch", Emu4Vita = "emu4vita" }
);

/// Which app starts a game, a setting of its own (`emulators.json`): the
/// choice for every game, and the titles the user chose otherwise for, by
/// id: `{"default": "retroarch", "titles": {"ra1x2y3z4": "emu4vita"}}`.
pub const EMULATORS_FILE: &str = "emulators.json";

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EmulatorChoice {
    pub default: Emulator,
    pub titles: std::collections::BTreeMap<String, Emulator>,
}

impl Default for EmulatorChoice {
    /// RetroArch, which started every game before Emu4Vita++ was read.
    fn default() -> Self {
        EmulatorChoice { default: Emulator::RetroArch, titles: Default::default() }
    }
}

impl Emulator {
    /// The app's name, the same in every language.
    pub fn name(self) -> &'static str {
        match self {
            Emulator::RetroArch => "RetroArch",
            Emulator::Emu4Vita => "Emu4Vita++",
        }
    }
}

impl EmulatorChoice {
    /// The saved choice; the default for a missing or unusable file or value.
    pub fn parse(bytes: &[u8]) -> EmulatorChoice {
        let mut choice = EmulatorChoice::default();
        let Ok(raw) = crate::library::read_json(bytes) else { return choice };
        if let Some(app) = raw.get("default").and_then(Value::as_str).and_then(Emulator::from_key) {
            choice.default = app;
        }
        if let Some(titles) = raw.get("titles").and_then(Value::as_object) {
            for (id, app) in titles {
                if let Some(app) = app.as_str().and_then(Emulator::from_key) {
                    choice.titles.insert(id.clone(), app);
                }
            }
        }
        choice
    }

    /// Two-space JSON and a newline, as the app's other files; titles by id.
    pub fn render(&self) -> String {
        let mut map = Map::new();
        map.insert("default".into(), self.default.key().into());
        let titles: Map<String, Value> = self.titles.iter().map(|(id, app)| (id.clone(), app.key().into())).collect();
        map.insert("titles".into(), Value::Object(titles));
        serde_json::to_string_pretty(&Value::Object(map)).expect("json") + "\n"
    }

    /// The app that starts `id`, which `opens` says can: the title's own
    /// choice, else the default, else the one app that can. None for a
    /// title neither app opens (an installed title).
    pub fn pick(&self, id: &str, opens: crate::library::catalog::Emulators) -> Option<Emulator> {
        let can = |app: Emulator| opens.has(app).then_some(app);
        self.titles.get(id).copied().and_then(can).or_else(|| can(self.default)).or_else(|| Emulator::ALL.iter().copied().find_map(can))
    }
}

/// The folders the title editor's Browse last took a box art and a
/// backdrop from (`folders.json`), where it opens next time.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Folders {
    pub art: String,
    pub backdrops: String,
}

pub const FOLDERS_FILE: &str = "folders.json";

impl Folders {
    pub fn parse(bytes: &[u8]) -> Folders {
        let Ok(raw) = crate::library::read_json(bytes) else { return Folders::default() };
        let get = |key: &str| raw.get(key).and_then(Value::as_str).unwrap_or("").to_string();
        Folders { art: get("art"), backdrops: get("backdrops") }
    }

    /// Two-space JSON and a newline, as the app's other files.
    pub fn render(&self) -> String {
        let mut map = Map::new();
        map.insert("art".into(), self.art.clone().into());
        map.insert("backdrops".into(), self.backdrops.clone().into());
        serde_json::to_string_pretty(&Value::Object(map)).expect("json") + "\n"
    }
}

impl Settings {
    /// The settings as saved: `settings.json`, then the files of their own
    /// that later settings went in.
    pub fn load(host: &mut dyn crate::host::Host, channel: UpdateChannel) -> Settings {
        let mut settings = Settings::defaults(channel);
        if let Some(bytes) = host.read(SETTINGS_FILE) {
            settings = Settings::parse(&bytes, settings);
        }
        if let Some(bytes) = host.read(STATUS_BAR_FILE) {
            settings.layout = StatusLayout::parse(&bytes);
        }
        if let Some(bytes) = host.read(KEYBOARD_FILE) {
            settings.keyboard = KeyboardKind::parse(&bytes);
        }
        if let Some(bytes) = host.read(EMULATORS_FILE) {
            settings.emulators = EmulatorChoice::parse(&bytes);
        }
        if let Some(bytes) = host.read(GAME_INFO_FILE) {
            settings.game_info = GameInfo::parse(&bytes);
        }
        if let Some(bytes) = host.read(TEXT_SIZE_FILE) {
            settings.text_size = TextSize::parse(&bytes);
        }
        if let Some(bytes) = host.read(crate::autostart::AUTOSTART_FILE) {
            settings.autostart = crate::autostart::AutoStart::parse(&bytes);
        }
        if let Some(bytes) = host.read(FOLDERS_FILE) {
            settings.folders = Folders::parse(&bytes);
        }
        settings
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
    /// Saved in its own file, `text-size.json` (not rendered here).
    pub text_size: TextSize,
    /// Saved in its own file, `keyboard.json` (not rendered here).
    pub keyboard: KeyboardKind,
    /// Saved in its own file, `emulators.json` (not rendered here).
    pub emulators: EmulatorChoice,
    /// Saved in its own file, `game-info.json` (not rendered here).
    pub game_info: GameInfo,
    /// Saved in its own file, `autostart.txt`, which the console start
    /// plugin reads (not rendered here).
    pub autostart: crate::autostart::AutoStart,
    /// Saved in its own file, `folders.json` (not rendered here).
    pub folders: Folders,
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
            text_size: TextSize::Normal,
            keyboard: KeyboardKind::Own,
            emulators: EmulatorChoice::default(),
            game_info: GameInfo::default(),
            autostart: crate::autostart::AutoStart::default(),
            folders: Folders::default(),
        }
    }

    /// `defaults` with each usable value of a saved file over it.
    pub fn parse(bytes: &[u8], defaults: Settings) -> Settings {
        let mut s = defaults;
        let Ok(raw) = crate::library::read_json(bytes) else { return s };
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
    fn the_keyboard_choice_round_trips_and_defaults_to_pocket_shelfs() {
        assert_eq!(KeyboardKind::Own.render(), "{\n  \"keyboard\": \"pocket-shelf\"\n}\n");
        assert_eq!(KeyboardKind::parse(KeyboardKind::Own.render().as_bytes()), KeyboardKind::Own);
        assert_eq!(KeyboardKind::parse(KeyboardKind::System.render().as_bytes()), KeyboardKind::System);
        assert_eq!(KeyboardKind::parse(b"nope"), KeyboardKind::Own);
    }

    #[test]
    fn emulator_choices_round_trip_and_pick_an_app_that_can_open() {
        use crate::library::catalog::Emulators;
        let mut choice = EmulatorChoice::default();
        assert_eq!(choice.render(), "{\n  \"default\": \"retroarch\",\n  \"titles\": {}\n}\n");
        choice.default = Emulator::Emu4Vita;
        choice.titles.insert("ra0000002".into(), Emulator::RetroArch);
        assert_eq!(EmulatorChoice::parse(choice.render().as_bytes()), choice);
        let parsed = EmulatorChoice::parse(br#"{"default": "nope", "titles": {"a": "emu4vita", "b": 3}}"#);
        assert_eq!(parsed.default, Emulator::RetroArch);
        assert_eq!(parsed.titles.len(), 1);
        assert_eq!(EmulatorChoice::parse(b"nope"), EmulatorChoice::default());

        let both = Emulators { retroarch: true, emu4vita: true };
        let retroarch = Emulators { retroarch: true, emu4vita: false };
        let emu4vita = Emulators { retroarch: false, emu4vita: true };
        assert_eq!(choice.pick("ra0000001", both), Some(Emulator::Emu4Vita));
        assert_eq!(choice.pick("ra0000002", both), Some(Emulator::RetroArch));
        // A choice the game cannot follow falls to the app that can.
        assert_eq!(choice.pick("ra0000001", retroarch), Some(Emulator::RetroArch));
        assert_eq!(choice.pick("ra0000002", emu4vita), Some(Emulator::Emu4Vita));
        assert_eq!(choice.pick("PCSA00069", Emulators::default()), None);
    }

    #[test]
    fn the_text_size_round_trips_and_defaults() {
        assert_eq!(TextSize::Large.render(), "{\n  \"size\": \"large\"\n}\n");
        for &size in TextSize::ALL {
            assert_eq!(TextSize::parse(size.render().as_bytes()), size);
        }
        assert_eq!(TextSize::parse(br#"{"size":"huge"}"#), TextSize::Normal);
        assert_eq!(TextSize::parse(b"nope"), TextSize::Normal);
    }

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
