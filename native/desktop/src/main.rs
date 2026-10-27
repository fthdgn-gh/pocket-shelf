//! Renders native Pocket Shelf on the desktop and writes screenshots.
//!
//! `cargo run --release -- [words…]` runs every shot whose name contains
//! one of the words (all shots without words), each from a fresh start with
//! the fake host, and writes `<name>.png` to
//! `.pocket-build/validation/native/desktop/`.

mod fake;
mod raster;

use std::path::PathBuf;
use std::time::Instant;

use shelf_core::app::{buttons::*, App};
use shelf_core::gfx::Gfx;

/// One screenshot: button masks held for a number of frames, in order,
/// from a fresh start of the host `setup` gives.
struct Shot {
    name: &'static str,
    script: &'static [(u32, u32)],
    setup: fn() -> fake::FakeHost,
}

const TAP: u32 = 2;
const SETTLE: u32 = 40;

fn plain() -> fake::FakeHost {
    fake::FakeHost::new()
}

fn first_start() -> fake::FakeHost {
    let mut host = fake::FakeHost::new();
    host.first_start = true;
    host
}

fn retro() -> fake::FakeHost {
    fake::FakeHost::new().with_retro_library()
}

fn turkish() -> fake::FakeHost {
    fake::FakeHost::new().with_language("tr")
}

fn german_first_start() -> fake::FakeHost {
    let mut host = fake::FakeHost::new().with_language("de");
    host.first_start = true;
    host
}

fn saved() -> fake::FakeHost {
    fake::FakeHost::new().with_saved_data()
}

const fn shot(name: &'static str, script: &'static [(u32, u32)]) -> Shot {
    Shot { name, script, setup: plain }
}

// Views through SELECT > Appearance > View, steps from Home: Carousel 1,
// Grid 2, List 3, Cascade 4. Details through Appearance > Details, steps
// from Normal: Detailed 1, Basic 2.
const SHOTS: &[Shot] = &[
    shot("01-reading", &[(0, 1)]),
    shot("02-home", &[(0, SETTLE)]),
    shot("03-home-right-3", &[(0, 4), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, SETTLE)]),
    shot("04-home-moving", &[(0, 4), (RIGHT, TAP), (0, 4)]),
    shot("05-home-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("06-home-basic", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("10-carousel", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("11-carousel-right-2", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("12-carousel-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("13-carousel-basic", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("20-grid", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("21-grid-down-2", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (0, SETTLE)]),
    shot("22-grid-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("30-list", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("31-list-held-down", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (DOWN, 60), (0, SETTLE)]),
    shot("32-list-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("40-cascade", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("41-cascade-right-2", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("42-cascade-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    // Tabs: R from Games to Homebrew, PS Mobile, PSP, PSX, the consoles;
    // L wraps to System.
    shot("50-tab-homebrew", &[(0, 4), (R, TAP), (0, SETTLE)]),
    shot("51-tab-snes", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, SETTLE)]),
    shot("52-tab-system", &[(0, 4), (L, TAP), (0, SETTLE)]),
    shot("53-tab-sliding", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, 3)]),
    // Themes by Appearance > Theme: Aurora 1, Daylight 4, Dynamic 5.
    shot("60-aurora", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("61-daylight", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("62-dynamic", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("63-dynamic-carousel", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    // The SELECT menu: main page, Appearance, Status bar, a value stepped
    // with the menu open, the language changed to Turkish.
    shot("100-menu", &[(0, 4), (SELECT, TAP), (0, SETTLE)]),
    shot("101-menu-appearance", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("102-menu-status", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("103-menu-view-changed", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("104-menu-turkish", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("105-menu-confirm-cross", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    // Search: square opens the keyboard; typing "st" (row 2 col 1, row 1
    // col 4) shows the count; Done shows the results tab; back closes it.
    shot("110-keyboard", &[(0, 4), (SQUARE, TAP), (0, TAP), (0, SETTLE)]),
    shot("111-keyboard-typed", &[(0, 4), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("112-search-results", &[(0, 4), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (START, TAP), (0, TAP), (0, SETTLE)]),
    shot("113-search-closed", &[(0, 4), (RIGHT, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (START, TAP), (0, TAP), (CROSS, TAP), (0, TAP), (0, SETTLE)]),
    shot("114-search-nothing", &[(0, 4), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (START, TAP), (0, TAP), (0, SETTLE)]),
    shot("115-keyboard-accents-turkish", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (0, SETTLE)]),
    // The title editor (TRIANGLE): favorite, category, box art and backdrop
    // pickers with previews, the title keyboard; the category manager.
    shot("120-editor", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("121-editor-favorite", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("122-editor-category", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("123-art-picker", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("124-art-picker-all-broken", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, SETTLE)]),
    shot("125-art-picked", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CROSS, TAP), (0, TAP), (0, SETTLE)]),
    shot("126-backdrop-picker", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (0, SETTLE)]),
    shot("127-title-keyboard", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("130-categories", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("131-categories-hidden", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (START, TAP), (0, TAP), (0, SETTLE)]),
    shot("132-categories-new", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (TRIANGLE, TAP), (0, TAP), (0, SETTLE)]),
    Shot { name: "133-categories-delete-armed", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (UP, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (0, SETTLE)], setup: saved },
    shot("134-categories-builtin-note", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (0, SETTLE)]),
    // Rescan titles (Library, fourth row) before, during and after its
    // scan; Diagnostics (last row of the main page).
    shot("140-rescan", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("141-rescan-running", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, 20)]),
    shot("142-rescan-done", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, 50)]),
    shot("150-diagnostics", &[(0, SETTLE), (SELECT, TAP), (0, TAP), (UP, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    // Languages: the shelf in Turkish (the dotted capital İ in a drawer
    // heading), the scan page in German.
    Shot { name: "64-turkish", script: &[(0, SETTLE)], setup: turkish },
    Shot { name: "65-turkish-adrenaline", script: &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, SETTLE)], setup: turkish },
    Shot { name: "66-german-scan", script: &[(0, 24)], setup: german_first_start },
    // A first start: the scan page, then the titles.
    Shot { name: "70-scan-start", script: &[(0, 3)], setup: first_start },
    Shot { name: "71-scan-middle", script: &[(0, 24)], setup: first_start },
    Shot { name: "72-scan-titles", script: &[(0, 44 + 8 + SETTLE)], setup: first_start },
    // Saved data: Aurora, the user's own category with the saved title
    // selected, renamed and a favorite; R past System wraps to Last Played,
    // then Favorites.
    Shot { name: "80-saved-selection", script: &[(0, SETTLE)], setup: saved },
    Shot { name: "81-saved-favorites", script: &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, SETTLE)], setup: saved },
    Shot { name: "82-saved-last-played", script: &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, SETTLE)], setup: saved },
    shot("91-launching", &[(0, 4), (CIRCLE, TAP), (0, SETTLE)]),
    // Every view and detail level over icon-less RetroArch games (the NES
    // tab: L twice from Games wraps past System to the last console), with
    // moves: the glyph atlas count is printed.
    Shot {
        name: "94-retro-views",
        script: &[
            (0, 4), (L, TAP), (0, TAP), (L, TAP), (0, TAP),
            (START, TAP), (0, TAP), (RIGHT, 40), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, 40), (0, TAP),
            (SELECT, TAP), (0, TAP), (DOWN, 40), (0, TAP), (SELECT, TAP), (0, TAP), (DOWN, 40), (0, TAP),
            (SELECT, TAP), (0, TAP), (RIGHT, 40), (0, TAP), (START, TAP), (0, TAP), (SELECT, TAP), (0, TAP),
            (RIGHT, 40), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (DOWN, 40), (0, TAP),
            (START, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, 40), (0, SETTLE),
        ],
        setup: retro,
    },
    // Velocity Run (PSP tab, second) starts through Adrenaline: the drawer
    // offers the plugin; confirm adds it and launches.
    shot("92-adrenaline-drawer", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, SETTLE)]),
    shot("93-adrenaline-launch", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, SETTLE), (CIRCLE, TAP), (0, SETTLE)]),
];

fn main() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.pocket-build/validation/native/desktop");
    std::fs::create_dir_all(&out).expect("create output folder");

    for shot in SHOTS.iter().filter(|shot| words.is_empty() || words.iter().any(|w| shot.name.contains(w.as_str()))) {
        let mut host = (shot.setup)();
        let mut gfx = Gfx::new(2.0);
        let mut app = App::new();
        for &(pad, frames) in shot.script {
            for _ in 0..frames {
                let started = Instant::now();
                app.events(&mut host, &mut gfx);
                app.update(&mut host, pad);
                app.draw(&mut gfx);
                // What a backend does once it has let go of freed textures.
                gfx.textures.recycle_freed();
                gfx.textures.mark_all_uploaded();
                // Desktop CPU time of the shared code; the frame time is the
                // Vita's 60 Hz so the line reads as it would there.
                app.stats.push(started.elapsed().as_micros() as u32, 16_667);
            }
        }
        let mut canvas = raster::Canvas::new(960, 544);
        canvas.run(&gfx.cmds, &gfx.textures);
        let path = out.join(format!("{}.png", shot.name));
        canvas.save_png(&path).expect("write png");
        println!(
            "{} ({} commands, {} KB of textures, glyph atlas started over {} times)",
            path.display(),
            gfx.cmds.len(),
            gfx.textures.live_bytes() / 1024,
            gfx.atlas_resets()
        );
    }
}
