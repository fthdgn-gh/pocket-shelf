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

fn saved() -> fake::FakeHost {
    fake::FakeHost::new().with_saved_data()
}

const fn shot(name: &'static str, script: &'static [(u32, u32)]) -> Shot {
    Shot { name, script, setup: plain }
}

const SHOTS: &[Shot] = &[
    shot("01-reading", &[(0, 1)]),
    shot("02-icons-arriving", &[(0, 3)]),
    shot("03-start", &[(0, SETTLE)]),
    shot("04-right-3", &[(0, 4), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, SETTLE)]),
    shot("05-down-2", &[(0, 4), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, SETTLE)]),
    shot("06-scrolling", &[(0, 4), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, 2)]),
    shot("07-wrap-to-last", &[(0, 4), (LEFT, TAP), (0, SETTLE)]),
    shot("08-held-down", &[(0, 4), (R, TAP), (0, TAP), (DOWN, 120), (0, SETTLE)]),
    shot("09-test-titles", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (R, TAP), (0, TAP), (UP, TAP), (0, SETTLE)]),
    shot("10-aurora", &[(0, 4), (SQUARE, TAP), (0, SETTLE)]),
    shot("11-sakura", &[(0, 4), (SQUARE, TAP), (0, TAP), (SQUARE, TAP), (0, SETTLE)]),
    shot("12-ember", &[(0, 4), (SQUARE, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (SQUARE, TAP), (0, SETTLE)]),
    shot("13-daylight", &[(0, 4), (SQUARE, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (SQUARE, TAP), (0, SETTLE)]),
    // Tabs: R from Games to Homebrew, the consoles, System; L wraps to the last.
    shot("20-tab-homebrew", &[(0, 4), (R, TAP), (0, SETTLE)]),
    shot("21-tab-psp", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, SETTLE)]),
    shot("22-tab-snes", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, SETTLE)]),
    shot("23-tab-system", &[(0, 4), (L, TAP), (0, SETTLE)]),
    // A first start: the scan page, then the titles.
    Shot { name: "30-scan-start", script: &[(0, 3)], setup: first_start },
    Shot { name: "31-scan-middle", script: &[(0, 24)], setup: first_start },
    Shot { name: "32-scan-done", script: &[(0, 44)], setup: first_start },
    Shot { name: "33-scan-titles", script: &[(0, 44 + 8 + SETTLE)], setup: first_start },
    // Saved data: Aurora, the user's own category with the saved title
    // selected, renamed and a favorite; R past System wraps to Last Played,
    // then Favorites.
    Shot { name: "40-saved-selection", script: &[(0, SETTLE)], setup: saved },
    Shot { name: "41-saved-favorites", script: &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, SETTLE)], setup: saved },
    Shot { name: "42-saved-last-played", script: &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, SETTLE)], setup: saved },
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
        println!("{} ({} commands, {} KB of textures)", path.display(), gfx.cmds.len(), gfx.textures.live_bytes() / 1024);
    }
}
