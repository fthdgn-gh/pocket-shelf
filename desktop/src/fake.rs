//! A stand-in for the Vita host, as the PocketJS preview was for
//! current app: sample titles, generated icons and backdrops, an in-memory
//! data folder, a fixed status line. Answers arrive one poll after they were
//! asked for, as a worker thread's would.

use std::collections::HashMap;

use shelf_core::host::{DecodeAs, Event, Host, NetId, NetKind, NetState, Picture, PictureKey, PluginState, ScanPhase, Status, TitleInfo};
use shelf_core::library::catalog::load_catalog;
use shelf_core::text::{render_mask, Weight};
use shelf_core::theme::ACCENT_HAS_ALPHA;

/// The preview's sample library, in its order.
fn sample_titles() -> Vec<TitleInfo> {
    // id, title, platform, console (a RetroArch game's database name).
    let fixed: &[(&str, &str, &str, &str)] = &[
        ("PCSA00069", "Gravity Daze", "vita", ""),
        ("PCSB00245", "Neon Drift", "vita", ""),
        ("PCSE00317", "Castle Siege Tactics", "vita", ""),
        ("PCSF00042", "Star Harbor", "vita", ""),
        ("PCSG00551", "Pixel Quest Chronicles", "vita", ""),
        ("PCSA00126", "Sky Runner", "vita", ""),
        ("PCSB00890", "Deep Blue", "vita", ""),
        ("PCSE00444", "Iron Fist Arena", "vita", ""),
        ("PCSH00213", "Moon Garden", "vita", ""),
        ("PCSA00777", "Turbo Kart Rally", "vita", ""),
        ("PCSB00101", "Shadow Temple", "vita", ""),
        ("PCSE00912", "Orbit Breaker", "vita", ""),
        ("PCSG00003", "Paper Kingdom", "vita", ""),
        ("NPXS10001", "Media Player", "vita", ""),
        ("NPXS10003", "Web Browser", "vita", ""),
        ("NPXS10004", "Photo Viewer", "vita", ""),
        ("NPXS10009", "Music Box", "vita", ""),
        ("VITASHELL", "VitaShell", "vita", ""),
        ("RETROVITA", "RetroArch", "vita", ""),
        ("SAVEMGR00", "Save Manager", "vita", ""),
        ("PLUGLOAD1", "Plugin Loader", "vita", ""),
        ("TINYSYNTH", "Tiny Synth", "vita", ""),
        ("PSPEMUCFW", "Adrenaline", "vita", ""),
        ("PSPEMU001", "Crisis Core", "vita", ""),
        ("PSPEMU002", "Castlevania SOTN", "vita", ""),
        ("NPOA00013", "Tiny Racer", "psm", ""),
        ("NPNA00042", "Puzzle Garden", "psm", ""),
        ("NPPA00007", "Sky Kite", "psm", ""),
        ("NPUZ00001", "Echo Chamber", "psp", ""),
        ("ULUS10041", "Velocity Run", "psp", ""),
        ("SCUS94163", "Crystal Quest", "psx", ""),
        ("NPUJ00662", "Cave Raiders", "psx", ""),
        ("ra0snes01", "Star Knight Saga (USA)", "retro", "Nintendo - Super Nintendo Entertainment System"),
        ("ra0snes02", "Pixel Quest", "retro", "Nintendo - Super Nintendo Entertainment System"),
        ("ra0md0001", "Turbo Hedgehog (World)", "retro", "Sega - Mega Drive - Genesis"),
        ("ra0arc001", "Slug Brigade", "retro", "FBNeo - Arcade Games"),
        ("ra0arc002", "Maze Muncher", "retro", "MAME 2003-Plus"),
        ("ra0psx001", "Monkey Escape", "retro", "Sony - PlayStation"),
    ];
    let mut titles: Vec<TitleInfo> = fixed
        .iter()
        .map(|&(id, title, platform, console)| TitleInfo {
            id: id.into(),
            title: title.into(),
            platform: platform.into(),
            // PSP and PS1 games started through Adrenaline.
            adrenaline: matches!(id, "ULUS10041" | "SCUS94163"),
            console: (!console.is_empty()).then(|| console.into()),
        })
        .collect();
    for index in 0..120 {
        titles.push(TitleInfo {
            id: format!("HBREW{index:04}"),
            title: format!("Homebrew Sample {}", index + 1),
            platform: "vita".into(),
            adrenaline: false,
            console: None,
        });
    }
    titles
}

/// Icon gradients: start and end.
const ICON_COLORS: [(u32, u32); 7] = [
    (0xf59e0b, 0xb91c1c),
    (0x22d3ee, 0x1d4ed8),
    (0x34d399, 0x0f766e),
    (0xa78bfa, 0x5b21b6),
    (0xfb7185, 0x9d174d),
    (0xfde047, 0xc2410c),
    (0x93c5fd, 0x312e81),
];

fn rgb(hex: u32) -> [f32; 3] {
    [(hex >> 16) as f32, ((hex >> 8) & 255) as f32, (hex & 255) as f32]
}

/// A tiny RGBA painter for the stand-in pictures.
struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Canvas { w, h, px: vec![[0.0; 4]; w * h] }
    }

    /// Blend `color` at coverage `a` (0-1) into pixel (x, y).
    fn blend(&mut self, x: usize, y: usize, color: [f32; 3], a: f32) {
        let p = &mut self.px[y * self.w + x];
        let out_a = a + p[3] * (1.0 - a);
        if out_a <= 0.0 {
            return;
        }
        for c in 0..3 {
            p[c] = (color[c] * a + p[c] * p[3] * (1.0 - a)) / out_a;
        }
        p[3] = out_a;
    }

    fn each(&mut self, f: impl Fn(f32, f32) -> Option<([f32; 3], f32)>) {
        for y in 0..self.h {
            for x in 0..self.w {
                if let Some((color, a)) = f(x as f32 + 0.5, y as f32 + 0.5) {
                    self.blend(x, y, color, a.clamp(0.0, 1.0));
                }
            }
        }
    }

    /// Text centered on (cx, cy), in `color`.
    fn text(&mut self, text: &str, px: f32, cx: f32, cy: f32, color: [f32; 3]) {
        let (w, h, mask) = render_mask(Weight::Bold, px, text);
        let (ox, oy) = ((cx - w as f32 / 2.0).round() as i32, (cy - h as f32 / 2.0).round() as i32);
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let (px, py) = (ox + x, oy + y);
                if px >= 0 && py >= 0 && (px as usize) < self.w && (py as usize) < self.h {
                    let a = mask[(y as u32 * w + x as u32) as usize] as f32 / 255.0;
                    if a > 0.0 {
                        self.blend(px as usize, py as usize, color, a);
                    }
                }
            }
        }
    }

    fn rgba(&self) -> Vec<u8> {
        self.px.iter().flat_map(|p| [p[0] as u8, p[1] as u8, p[2] as u8, (p[3] * 255.0).round() as u8]).collect()
    }
}

fn initials(title: &str) -> String {
    title.split(' ').filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase()
}

/// A 128×128 stand-in for a title's icon0.png:
/// a gradient with shapes and initials, or every third one a ring and
/// initials on a clear background.
fn icon(index: usize, title: &str) -> Picture {
    let (from, to) = ICON_COLORS[index % ICON_COLORS.len()];
    let (from, to) = (rgb(from), rgb(to));
    let mut canvas = Canvas::new(128, 128);
    if index % 3 == 1 {
        canvas.each(|x, y| {
            let d = ((x - 64.0).powi(2) + (y - 64.0).powi(2)).sqrt();
            Some((from, 6.5 - (d - 50.0).abs()))
        });
        canvas.text(&initials(title), 46.0, 64.0, 67.0, from);
    } else {
        canvas.each(|x, y| {
            let t = ((x + y) / 256.0).clamp(0.0, 1.0);
            Some(([0, 1, 2].map(|c| from[c] + (to[c] - from[c]) * t), 1.0))
        });
        let (cx, cy) = (24.0 + ((index * 37) % 80) as f32, 30.0 + ((index * 53) % 60) as f32);
        canvas.each(|x, y| {
            let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            Some(([255.0; 3], (46.5 - d).clamp(0.0, 1.0) * 0.22))
        });
        let edge = 60.0 + ((index * 17) % 40) as f32;
        canvas.each(|x, y| {
            // Below the line from (0, 128) to (128, edge).
            let line = 128.0 + (edge - 128.0) * x / 128.0;
            Some(([0.0; 3], (y - line + 0.5).clamp(0.0, 1.0) * 0.18))
        });
        canvas.text(&initials(title), 54.0, 64.0, 68.0, [255.0; 3]);
    }
    // The accent: the first gradient color, flagged for the
    // see-through ones.
    let accent = ICON_COLORS[index % ICON_COLORS.len()].0 | if index % 3 == 1 { ACCENT_HAS_ALPHA } else { 0 };
    Picture::new(128, 128, canvas.rgba(), Some(accent))
}

/// A square sample icon squeezed to 144:80 (128×71) in the middle of the
/// square, clear above and below, flagged see-through.
fn psp_icon(square: Picture) -> Picture {
    const SIDE: usize = 128;
    let high = SIDE * 80 / 144;
    let top = (SIDE - high) / 2;
    let mut rgba = vec![0u8; SIDE * SIDE * 4];
    for y in 0..high {
        let from = y * SIDE / high;
        let row = (top + y) * SIDE * 4;
        rgba[row..row + SIDE * 4].copy_from_slice(&square.rgba[from * SIDE * 4..(from + 1) * SIDE * 4]);
    }
    let accent = square.accent.map(|a| a | ACCENT_HAS_ALPHA);
    Picture::new(SIDE as u32, SIDE as u32, rgba, accent)
}

/// A 512×256 stand-in for a title's full-screen picture: sky, sun and three hills in its colors.
fn backdrop(index: usize) -> Picture {
    let (from, to) = ICON_COLORS[index % ICON_COLORS.len()];
    let (from, to) = (rgb(from), rgb(to));
    let mut canvas = Canvas::new(512, 256);
    canvas.each(|_, y| Some(([0, 1, 2].map(|c| to[c] + (from[c] - to[c]) * y / 256.0), 1.0)));
    let (sx, sy) = (330.0 + ((index * 41) % 120) as f32, 70.0 + ((index * 23) % 40) as f32);
    canvas.each(|x, y| Some(([255.0; 3], (34.5 - ((x - sx).powi(2) + (y - sy).powi(2)).sqrt()).clamp(0.0, 1.0) * 0.55)));
    for (layer, shade) in [0.25f32, 0.45, 0.7].into_iter().enumerate() {
        let layer = layer as f32;
        let wave = |x: f32| 150.0 + layer * 30.0 + ((x + index as f32 * 53.0) / (38.0 + layer * 21.0)).sin() * (26.0 - layer * 6.0);
        canvas.each(|x, y| {
            // The canvas path joins points 16 px apart with straight lines.
            let x0 = (x / 16.0).floor() * 16.0;
            let t = (x - x0) / 16.0;
            let edge = wave(x0) + (wave(x0 + 16.0) - wave(x0)) * t;
            Some((rgb(0x04060f), (y - edge + 0.5).clamp(0.0, 1.0) * shade))
        });
    }
    Picture::new(512, 256, canvas.rgba(), None)
}

pub struct FakeHost {
    titles: Vec<TitleInfo>,
    files: HashMap<String, Vec<u8>>,
    pub status: Status,
    /// Answers waiting for the next poll, and those for the poll after.
    next: Vec<Event>,
    later: Vec<Event>,
    frame: u64,
    /// A first start: no saved list, so `scan` runs a scan with progress.
    pub first_start: bool,
    /// The frame a scan started on.
    scanning: Option<u64>,
    /// Adrenaline's boot plugin, as `pspemu::plugin_state` would read it.
    pub plugin: PluginState,
    /// Network requests open, by id.
    requests: HashMap<i32, FakeRequest>,
    next_request: i32,
    /// The launches accepted, by title id.
    pub launched: Vec<String>,
    pub log: Vec<String>,
    /// Whether this build may install an update (the real title id).
    pub installable: bool,
    /// The updater's report at start ("", "ok" or "error <reason>").
    pub update_report: String,
    /// The update step running and the frame it started on.
    update_step: Option<(&'static str, u64)>,
}

/// GitHub's releases list: a beta newer than the fake build (0.1.0, beta)
/// and the build's own release.
fn github_reply() -> String {
    let release = |tag: &str, date: &str| {
        format!(
            r#"{{"tag_name":"{tag}","name":"Pocket Shelf {tag}","draft":false,"target_commitish":"main","published_at":"{date}T18:00:00Z","assets":[{{"name":"pocket-shelf.vpk","browser_download_url":"https://github.com/fthdgn-gh/pocket-shelf/releases/download/{tag}/pocket-shelf.vpk","size":2516582}}]}}"#
        )
    };
    format!("[{},{}]", release("shelf-v0.2.0-beta.1", "2026-10-27"), release("shelf-v0.1.0", "2026-10-05"))
}

/// The names the fake libretro server lists for every console: the sample
/// RetroArch games' labels as No-Intro writes them, and look-alikes.
const LIBRETRO_NAMES: [&str; 10] = [
    "Star Knight Saga (USA)",
    "Star Knight Saga (Japan)",
    "Pixel Quest (USA)",
    "Pixel Quest (Europe)",
    "Pixel Quest 2 (USA)",
    "Turbo Hedgehog (World)",
    "Slug Brigade",
    "Maze Muncher",
    "Monkey Escape (USA)",
    "Crystal Quest (USA)",
];

/// The one API key the fake SteamGridDB accepts (`PREVIEW_KEY`).
pub const PREVIEW_KEY: &str = "0123456789abcdef0123456789abcdef";

/// A fake request: polls so far, its reply's status, text and size.
struct FakeRequest {
    polls: u32,
    status: u16,
    text: String,
    size: u64,
}

/// SteamGridDB's reply to a GET.
fn steamgriddb_reply(url: &str, authorization: &str) -> (u16, String) {
    let reply = |data: String| format!("{{\"success\":true,\"data\":{data}}}");
    if authorization != format!("Bearer {PREVIEW_KEY}") {
        return (401, "{\"success\":false,\"errors\":[\"Authentication Required\"]}".into());
    }
    if let Some(term) = url.split("/search/autocomplete/").nth(1) {
        let term = percent_decode(term);
        // A game id per term, so each title gets pictures of its own.
        let id = 100 + (term.chars().map(|c| c as u32).sum::<u32>() % 800) * 10;
        let games = [
            format!("{{\"id\":{id},\"name\":{},\"release_date\":1339459200}}", json_string(&term)),
            format!("{{\"id\":{},\"name\":{},\"release_date\":1484870400}}", id + 1, json_string(&format!("{term} 2"))),
            format!("{{\"id\":{},\"name\":{},\"release_date\":1454371200}}", id + 2, json_string(&format!("{term} Remastered"))),
            format!("{{\"id\":{},\"name\":{},\"release_date\":0}}", id + 3, json_string(&format!("{term}: The Animation"))),
        ];
        return (200, reply(format!("[{}]", games.join(","))));
    }
    for (part, kind, count, offset) in [("/icons/game/", "icon", 14, 0), ("/heroes/game/", "hero", 6, 50)] {
        if let Some(rest) = url.split(part).nth(1) {
            let game: u32 = rest.split(|c: char| !c.is_ascii_digit()).next().and_then(|n| n.parse().ok()).unwrap_or(0);
            let items: Vec<String> = (0..count)
                .map(|i| format!("{{\"id\":{},\"url\":\"https://cdn2.steamgriddb.com/{kind}/{i}.png\"}}", game * 100 + offset + i))
                .collect();
            return (200, reply(format!("[{}]", items.join(","))));
        }
    }
    (200, reply("[]".into()))
}

fn json_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `%XX` escapes back to their bytes.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Frames a fake scan takes, and its phases' shares of the steps.
const SCAN_FRAMES: u64 = 40;
const SCAN_PHASES: [(ScanPhase, u32); 5] =
    [(ScanPhase::Apps, 26), (ScanPhase::System, 55), (ScanPhase::Psm, 1), (ScanPhase::Psp, 4), (ScanPhase::Retro, 6)];

impl FakeHost {
    pub fn new() -> Self {
        // Pictures in the data folder, as the preview has them: box art named
        // after a title, one file that does not decode, a backdrop.
        let files = ["art/PCSA00069-48213.png", "art/Neon Drift.png", "art/broken-art.png", "backdrops/PCSA00069-777.png"]
            .into_iter()
            .map(|path| (path.to_string(), Vec::new()))
            .collect();
        FakeHost {
            titles: sample_titles(),
            files,
            // The preview's `21 47 1 72 0 80 1`.
            status: Status { hour: 21, minute: 47, clock24: Some(true), battery: Some(72), charging: false, wifi: Some(80), bluetooth: Some(true) },
            next: Vec::new(),
            later: Vec::new(),
            frame: 0,
            first_start: false,
            scanning: None,
            plugin: PluginState::Missing,
            requests: HashMap::new(),
            next_request: 1,
            launched: Vec::new(),
            log: Vec::new(),
            installable: false,
            update_report: String::new(),
            update_step: None,
        }
    }

    /// A library like the user's: 3,000 RetroArch games with varied names
    /// and no box art, so their tiles show initials.
    pub fn with_retro_library(mut self) -> Self {
        let words = [
            "Alpha", "Battle", "Castle", "Dragon", "Echo", "Frontier", "Galaxy", "Hyper", "Iron", "Jungle", "Knight", "Legend", "Mega",
            "Ninja", "Omega", "Pixel", "Quest", "Rocket", "Super", "Turbo", "Ultra", "Venture", "Wonder", "Xeno", "Yoshi", "Zero",
            "1942", "3D", "Ötzi", "Éclair",
        ];
        let consoles = ["Nintendo - Nintendo Entertainment System", "Nintendo - Super Nintendo Entertainment System", "Sega - Mega Drive - Genesis"];
        for n in 0..3000 {
            let (a, b) = (words[n % words.len()], words[(n / words.len() + n) % words.len()]);
            self.titles.push(TitleInfo {
                id: format!("rq{n:07}"),
                title: format!("{a} {b} {n}"),
                platform: "retro".into(),
                adrenaline: false,
                console: Some(consoles[n % consoles.len()].into()),
            });
        }
        self
    }

    /// The SteamGridDB key the fake server accepts, saved.
    pub fn with_key(mut self) -> Self {
        self.files.insert("steamgriddb.txt".into(), format!("{PREVIEW_KEY}\n").into_bytes());
        self
    }

    /// Settings saved with only a language, so the shelf shows it.
    pub fn with_language(mut self, key: &str) -> Self {
        self.files.insert("settings.json".into(), format!("{{\"language\":\"{key}\"}}").into_bytes());
        self
    }

    /// The system's dates for the sample titles, as `app.db` gives them: an
    /// install date for each title with a bubble (RetroArch games have
    /// none), a last launch for every third one.
    fn dates(&self) -> Vec<(String, shelf_core::library::played::SystemDates)> {
        const DAY: u64 = 86_400_000;
        // 2026-01-01, UTC.
        const START: u64 = 1_767_225_600_000;
        self.titles
            .iter()
            .enumerate()
            .filter(|(_, t)| t.platform != "retro")
            .map(|(i, t)| {
                let i = i as u64;
                let installed = Some(START + (i * 37 % 200) * DAY);
                let launched = (i % 3 == 0).then(|| START + 250 * DAY + (i * 53 % 50) * DAY);
                (t.id.clone(), shelf_core::library::played::SystemDates { launched, installed })
            })
            .collect()
    }

    /// The list view and a sort order for the Games tab, saved.
    pub fn with_sorted_games(mut self, order: &str) -> Self {
        self.files.insert("settings.json".into(), br#"{"view":"list","detail":"detailed"}"#.to_vec());
        self.files.insert("sort.json".into(), format!("{{\"games\":\"{order}\"}}").into_bytes());
        self
    }

    /// A saved text size (`text-size.json`).
    pub fn with_text_size(mut self, key: &str) -> Self {
        self.files.insert("text-size.json".into(), format!("{{\"size\":\"{key}\"}}").into_bytes());
        self
    }

    /// Saved data as a user of Pocket Shelf 0.1 has it: settings with a
    /// selection, a category of their own, changed titles, "Last Played".
    pub fn with_saved_data(mut self) -> Self {
        let files: &[(&str, &str)] = &[
            (
                "settings.json",
                r#"{"language":"en","theme":"aurora","view":"home","detail":"normal","confirm":"circle","backdrop":true,"iconBox":true,"statusBar":true,"clock":"system","batteryPercent":true,"updates":"off","scraper":"auto","category":"favorites-mine","title":"PCSF00042"}"#,
            ),
            ("categories.json", "{\n  \"custom\": [\n    {\n      \"id\": \"favorites-mine\",\n      \"label\": \"Couch Co-op\"\n    }\n  ],\n  \"order\": [],\n  \"hidden\": [\"homebrew\"]\n}\n"),
            ("titles/PCSF00042.json", "{\n  \"category\": \"favorites-mine\",\n  \"title\": \"Star Harbor: Director's Cut\",\n  \"favorite\": true\n}\n"),
            ("titles/PCSE00444.json", "{\n  \"category\": \"favorites-mine\"\n}\n"),
            ("titles/PCSB00245.json", "{\n  \"favorite\": true\n}\n"),
            ("recent.json", "[\"PCSE00444\",\"VITASHELL\",\"ra0snes01\"]\n"),
        ];
        for (path, text) in files {
            self.files.insert(path.to_string(), text.as_bytes().to_vec());
        }
        self
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.titles.iter().position(|t| t.id == id)
    }
}

impl Host for FakeHost {
    fn time_us(&self) -> u64 {
        self.frame * 16_667
    }

    fn read(&mut self, path: &str) -> Option<Vec<u8>> {
        self.files.get(path).cloned()
    }

    fn write(&mut self, path: &str, data: &[u8]) -> bool {
        self.files.insert(path.into(), data.to_vec());
        true
    }

    fn remove(&mut self, path: &str) -> bool {
        self.files.remove(path);
        true
    }

    fn list(&mut self, dir: &str) -> Vec<String> {
        let prefix = format!("{}/", dir.trim_end_matches('/'));
        let mut names: Vec<String> = self.files.keys().filter_map(|k| k.strip_prefix(&prefix)).filter(|n| !n.contains('/')).map(String::from).collect();
        names.sort();
        names
    }

    fn status(&mut self) -> Status {
        self.status
    }

    fn scan(&mut self, force: bool) {
        if self.first_start || force {
            self.scanning = Some(self.frame);
        } else {
            self.later.push(Event::Titles(load_catalog(&self.titles), 120));
            self.later.push(Event::Dates(self.dates()));
        }
    }

    fn request(&mut self, key: PictureKey) {
        let picture = match &key {
            // The retro library's games have no box art.
            PictureKey::Icon(id) if id.starts_with("rq") => None,
            // A PSP game's icon is 144×80, fitted whole into the square with
            // clear bars, as `pspemu::fit_png` does on the Vita.
            PictureKey::Icon(id) => self.index(id).map(|i| {
                let picture = icon(i, &self.titles[i].title);
                if self.titles[i].platform == "psp" { psp_icon(picture) } else { picture }
            }),
            // A box art file decodes as a sample icon, unless it stands for a
            // damaged file.
            PictureKey::Art(name) if name.starts_with("broken") => None,
            PictureKey::Art(name) => Some(icon(name.len() % 7 + 3, name)),
            PictureKey::Backdrop { id, .. } => self.index(id).map(backdrop),
            // Pictures in memory come through `decode`.
            PictureKey::Memory { .. } => None,
        };
        self.later.push(Event::Picture(key, picture));
    }

    /// A scan report as the Vita host joins it (PSM, PSP and PS1,
    /// RetroArch), shaped like a real one.
    fn scan_report(&mut self) -> String {
        "PSM folders=2,PSM source=app.db,PSP stick=ux0:/pspemu,PSP folders=3,PSP titles=2,PSP skipped=1,\
         skip=NPUZ00001 noEboot,ISO files=2,ISO titles=2,RetroArch playlists=9,RetroArch titles=5709,RetroArch skipped=102"
            .into()
    }

    fn boot_log(&mut self) -> String {
        "age=4s path=ms0:/PSP/GAME/SCUS94163/EBOOT.PBP boot psx\ntrigger=after sceVshCommonGui\ndriver=3 (default) EBOOT.BIN\n\
         age=3s path=ms0:/ISO/Racing/Ridge Racer.cso boot iso"
            .into()
    }

    fn build(&self) -> shelf_core::updates::BuildInfo {
        shelf_core::updates::BuildInfo { version: "0.1.0".into(), channel: "beta".into(), commit: String::new() }
    }

    fn unix_ms(&self) -> u64 {
        1_800_000_000_000
    }

    fn can_install_update(&self) -> bool {
        self.installable
    }

    fn update_unpack(&mut self) -> bool {
        self.update_step = Some(("unpack", self.frame));
        true
    }

    fn update_install(&mut self) -> bool {
        self.update_step = Some(("install", self.frame));
        true
    }

    fn update_launch(&mut self) -> bool {
        self.log.push("updater started".into());
        true
    }

    /// Unpacking 40 files over 40 frames, installing over 20.
    fn update_state(&mut self) -> String {
        match self.update_step {
            Some(("unpack", at)) if self.frame - at < 40 => format!("busy {} 40", self.frame - at),
            Some(("unpack", _)) => "done 00.02".into(),
            Some(("install", at)) if self.frame - at < 20 => "installing".into(),
            Some(("install", _)) => "installed".into(),
            _ => "idle".into(),
        }
    }

    fn update_result(&mut self) -> String {
        std::mem::take(&mut self.update_report)
    }

    fn listing(&mut self, folder: &str) {
        use shelf_core::online::libretro::{listing_path, Listing};
        let listing = self.files.get(&listing_path(folder)).and_then(|b| Listing::parse(&String::from_utf8_lossy(b)));
        self.later.push(Event::Listing(folder.to_string(), listing));
    }

    fn cancel(&mut self, key: &PictureKey) {
        self.later.retain(|event| !matches!(event, Event::Picture(k, _) if k == key));
    }

    fn poll(&mut self, events: &mut Vec<Event>) {
        self.frame += 1;
        if let Some(start) = self.scanning {
            let total: u32 = SCAN_PHASES.iter().map(|p| p.1).sum();
            let elapsed = self.frame - start;
            if elapsed >= SCAN_FRAMES {
                self.scanning = None;
                self.later.push(Event::Titles(load_catalog(&self.titles), 2800));
                self.later.push(Event::Dates(self.dates()));
            } else {
                let done = (elapsed as u32 * total) / SCAN_FRAMES as u32;
                let mut before = 0;
                let phase = SCAN_PHASES.iter().find(|p| { before += p.1; done < before }).map_or(ScanPhase::Retro, |p| p.0);
                events.push(Event::Scan { done, total, phase });
            }
        }
        events.append(&mut self.next);
        std::mem::swap(&mut self.next, &mut self.later);
    }

    fn log(&mut self, line: &str) {
        self.log.push(line.into());
    }

    fn launch(&mut self, id: &str) -> bool {
        let known = self.index(id).is_some();
        if known {
            self.launched.push(id.into());
        }
        known
    }

    fn plugin_state(&mut self) -> PluginState {
        self.plugin
    }

    fn enable_plugin(&mut self) -> bool {
        self.plugin = PluginState::On;
        true
    }

    /// A request finishes after six polls with a canned reply; a download
    /// lands in the data folder so the pickers list it.
    fn net_start(&mut self, url: &str, kind: NetKind) -> Option<NetId> {
        let request = match kind {
            NetKind::Text { .. } if url.starts_with("https://api.github.com/") => {
                let text = github_reply();
                FakeRequest { polls: 0, status: 200, size: text.len() as u64, text }
            }
            NetKind::Save(path) if path.starts_with("update/") => {
                self.files.insert(path, b"PK".to_vec());
                FakeRequest { polls: 0, status: 200, text: String::new(), size: 2516582 }
            }
            NetKind::Text { authorization } => {
                let (status, text) = steamgriddb_reply(url, &authorization);
                FakeRequest { polls: 0, status, size: text.len() as u64, text }
            }
            NetKind::Bytes => {
                // libretro's title screens are missing on the fake server.
                let status = if url.contains("/Named_Titles/") { 404 } else { 200 };
                FakeRequest { polls: 0, status, text: if status == 200 { "PNG".into() } else { String::new() }, size: 480 * 1024 }
            }
            NetKind::Save(path) | NetKind::SaveIcon(path) if path.starts_with("libretro/") => {
                self.files.insert(path, format!("{}\n", LIBRETRO_NAMES.join("\n")).into_bytes());
                FakeRequest { polls: 0, status: 200, text: String::new(), size: 900 * 1024 }
            }
            NetKind::Save(path) | NetKind::SaveIcon(path) => {
                // libretro's title screens are missing on the fake server.
                let status = if url.contains("/Named_Titles/") { 404 } else { 200 };
                if status == 200 {
                    self.files.insert(path, b"PNG".to_vec());
                }
                FakeRequest { polls: 0, status, text: String::new(), size: 480 * 1024 }
            }
        };
        let id = self.next_request;
        self.next_request += 1;
        self.requests.insert(id, request);
        Some(NetId(id))
    }

    fn net_state(&mut self, id: NetId) -> NetState {
        let Some(request) = self.requests.get_mut(&id.0) else { return NetState::Error("unknown request".into()) };
        request.polls += 1;
        if request.polls < 6 {
            return NetState::Busy { received: request.size * request.polls as u64 / 6, total: request.size };
        }
        NetState::Done { status: request.status, received: request.size }
    }

    fn net_text(&mut self, id: NetId) -> String {
        self.requests.get(&id.0).map(|r| r.text.clone()).unwrap_or_default()
    }

    fn net_close(&mut self, id: NetId) {
        self.requests.remove(&id.0);
    }

    fn net_bytes(&mut self, id: NetId) -> Vec<u8> {
        self.requests.get(&id.0).map(|r| r.text.as_bytes().to_vec()).unwrap_or_default()
    }

    /// A picture in memory decodes as a sample icon or backdrop.
    fn decode(&mut self, key: PictureKey, _png: Vec<u8>) {
        let picture = match &key {
            PictureKey::Memory { id, as_: DecodeAs::Backdrop } => Some(backdrop(*id as usize % 5)),
            PictureKey::Memory { id, .. } => Some(icon(*id as usize % 7 + 2, "Candidate")),
            _ => None,
        };
        self.later.push(Event::Picture(key, picture));
    }

    fn save_picture(&mut self, path: &str, png: Vec<u8>, _fit: bool) -> bool {
        self.files.insert(path.into(), png);
        true
    }
}
