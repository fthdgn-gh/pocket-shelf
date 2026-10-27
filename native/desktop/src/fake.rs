//! A stand-in for the Vita host, as `src/tools/preview.ts` is for the
//! current app: sample titles, generated icons and backdrops, an in-memory
//! data folder, a fixed status line. Answers arrive one poll after they were
//! asked for, as a worker thread's would.

use std::collections::HashMap;

use shelf_core::host::{Event, Host, Picture, PictureKey, ScanPhase, Status, TitleInfo};
use shelf_core::library::catalog::load_catalog;
use shelf_core::text::{render_mask, Weight};
use shelf_core::theme::ACCENT_HAS_ALPHA;

/// The preview's sample library (`TITLES` in preview.ts), in its order.
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
            // preview.ts's NO_BUBBLE: PSP and PS1 games started through Adrenaline.
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

/// `ICON_COLORS` in preview.ts: gradient start and end.
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

/// A 128×128 stand-in for a title's icon0.png (`iconPixels` in preview.ts):
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
    // preview.ts's iconAccent: the first gradient color, flagged for the
    // see-through ones.
    let accent = ICON_COLORS[index % ICON_COLORS.len()].0 | if index % 3 == 1 { ACCENT_HAS_ALPHA } else { 0 };
    Picture { w: 128, h: 128, rgba: canvas.rgba(), accent: Some(accent) }
}

/// A 512×256 stand-in for a title's full-screen picture (`backdropPixels`
/// in preview.ts): sky, sun and three hills in its colors.
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
    Picture { w: 512, h: 256, rgba: canvas.rgba(), accent: None }
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
    pub log: Vec<String>,
}

/// Frames a fake scan takes, and its phases' shares of the steps.
const SCAN_FRAMES: u64 = 40;
const SCAN_PHASES: [(ScanPhase, u32); 5] =
    [(ScanPhase::Apps, 26), (ScanPhase::System, 55), (ScanPhase::Psm, 1), (ScanPhase::Psp, 4), (ScanPhase::Retro, 6)];

impl FakeHost {
    pub fn new() -> Self {
        FakeHost {
            titles: sample_titles(),
            files: HashMap::new(),
            // The preview's `21 47 1 72 0 80 1`.
            status: Status { hour: 21, minute: 47, clock24: Some(true), battery: Some(72), charging: false, wifi: Some(80), bluetooth: Some(true) },
            next: Vec::new(),
            later: Vec::new(),
            frame: 0,
            first_start: false,
            scanning: None,
            log: Vec::new(),
        }
    }

    /// Saved data as a user of the current app has it: settings with a
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

    fn scan(&mut self) {
        if self.first_start {
            self.scanning = Some(self.frame);
        } else {
            self.later.push(Event::Titles(load_catalog(&self.titles), 120));
        }
    }

    fn request(&mut self, key: PictureKey) {
        let picture = match &key {
            PictureKey::Icon(id) => self.index(id).map(|i| icon(i, &self.titles[i].title)),
            PictureKey::Backdrop(id) => self.index(id).map(backdrop),
        };
        self.later.push(Event::Picture(key, picture));
    }

    fn poll(&mut self, events: &mut Vec<Event>) {
        self.frame += 1;
        if let Some(start) = self.scanning {
            let total: u32 = SCAN_PHASES.iter().map(|p| p.1).sum();
            let elapsed = self.frame - start;
            if elapsed >= SCAN_FRAMES {
                self.scanning = None;
                self.later.push(Event::Titles(load_catalog(&self.titles), 2800));
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
}
