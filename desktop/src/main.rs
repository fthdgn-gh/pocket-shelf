//! Renders native Pocket Shelf on the desktop and writes screenshots.
//!
//! `cargo run --release -- [words…]` runs every shot whose name contains
//! one of the words (all shots without words), each from a fresh start with
//! the fake host, and writes `<name>.png` to
//! `.build/validation/desktop/`.

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

/// A script step that puts a finger on the front touch screen at `(x, y)`
/// pixels instead of pressing buttons: a flag above the buttons' bits, then
/// x/2 and y/2 in nine bits each (even pixels).
const TOUCH: u32 = 1 << 12;
const fn touch(x: u32, y: u32) -> u32 {
    TOUCH | ((x / 2) << 13) | ((y / 2) << 22)
}

/// The finger a script step puts down, and the buttons it holds.
fn decode(step: u32) -> (u32, Option<(f32, f32)>) {
    if step & TOUCH == 0 {
        return (step, None);
    }
    let x = ((step >> 13) & 511) * 2;
    let y = ((step >> 22) & 511) * 2;
    (0, Some((x as f32, y as f32)))
}
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

fn keyed() -> fake::FakeHost {
    fake::FakeHost::new().with_key()
}

fn installable() -> fake::FakeHost {
    let mut host = fake::FakeHost::new();
    host.installable = true;
    host
}

fn updated() -> fake::FakeHost {
    let mut host = fake::FakeHost::new();
    host.update_report = "ok".into();
    host
}

fn larger() -> fake::FakeHost {
    fake::FakeHost::new().with_text_size("larger")
}

fn smaller() -> fake::FakeHost {
    fake::FakeHost::new().with_text_size("smaller")
}

fn larger_turkish() -> fake::FakeHost {
    fake::FakeHost::new().with_language("tr").with_text_size("larger")
}

fn larger_first_start() -> fake::FakeHost {
    let mut host = fake::FakeHost::new().with_text_size("larger");
    host.first_start = true;
    host
}

fn sorted_recent() -> fake::FakeHost {
    fake::FakeHost::new().with_sorted_games("played-recent")
}

fn sorted_installed() -> fake::FakeHost {
    fake::FakeHost::new().with_sorted_games("installed-newest")
}

fn larger_german() -> fake::FakeHost {
    fake::FakeHost::new().with_language("de").with_text_size("larger")
}

fn tall_art() -> fake::FakeHost {
    fake::FakeHost::new().with_tall_art()
}

fn system_keyboard() -> fake::FakeHost {
    fake::FakeHost::new().with_system_keyboard("st")
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
    shot("05-home-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("06-home-basic", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("10-carousel", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("11-carousel-right-2", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("12-carousel-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("13-carousel-basic", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("20-grid", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("21-grid-down-2", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (0, SETTLE)]),
    shot("22-grid-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("30-list", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("31-list-held-down", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (DOWN, 60), (0, SETTLE)]),
    shot("32-list-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("40-cascade", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("41-cascade-right-2", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("42-cascade-detailed", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    // Tabs: R from Games to Homebrew, PS Mobile, PSP, PSX, the consoles;
    // L wraps to System.
    shot("50-tab-homebrew", &[(0, 4), (R, TAP), (0, SETTLE)]),
    shot("51-tab-snes", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, SETTLE)]),
    shot("52-tab-system", &[(0, 4), (L, TAP), (0, SETTLE)]),
    shot("53-tab-sliding", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, 3)]),
    // Themes by Appearance > Theme: Aurora 1, Daylight 4, Dynamic 5.
    shot("60-aurora", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("61-daylight", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("62-dynamic", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("63-dynamic-carousel", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    // The SELECT menu: main page, Appearance, Status bar, a value stepped
    // with the menu open, the language changed to Turkish.
    shot("100-menu", &[(0, 4), (SELECT, TAP), (0, SETTLE)]),
    shot("101-menu-appearance", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("102-menu-status", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("103-menu-view-changed", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("104-menu-turkish", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("105-menu-confirm-cross", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    // Search: square opens the keyboard; typing "st" (row 2 col 1, row 1
    // col 4) shows the count; Done shows the results tab; back closes it.
    shot("110-keyboard", &[(0, 4), (SQUARE, TAP), (0, TAP), (0, SETTLE)]),
    shot("111-keyboard-typed", &[(0, 4), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("112-search-results", &[(0, 4), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (START, TAP), (0, TAP), (0, SETTLE)]),
    shot("113-search-closed", &[(0, 4), (RIGHT, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (START, TAP), (0, TAP), (CROSS, TAP), (0, TAP), (0, SETTLE)]),
    shot("114-search-nothing", &[(0, 4), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (START, TAP), (0, TAP), (0, SETTLE)]),
    shot("115-keyboard-accents-turkish", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SQUARE, TAP), (0, SETTLE), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, 40), (0, SETTLE)]),
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
    shot("130-categories", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("131-categories-hidden", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (START, TAP), (0, TAP), (0, SETTLE)]),
    shot("132-categories-new", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (TRIANGLE, TAP), (0, TAP), (0, SETTLE)]),
    Shot { name: "133-categories-delete-armed", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (UP, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (0, SETTLE)], setup: saved },
    shot("134-categories-builtin-note", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (SQUARE, TAP), (0, TAP), (0, SETTLE)]),
    // Rescan titles (Library, fourth row) before, during and after its
    // scan; Diagnostics (last row of the main page).
    shot("140-rescan", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("141-rescan-running", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, 20)]),
    shot("142-rescan-done", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, 50)]),
    shot("150-diagnostics", &[(0, SETTLE), (SELECT, TAP), (0, TAP), (UP, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    // Further down: the build, then the scan report and the boot log.
    shot("151-diagnostics-report", &[(0, SETTLE), (SELECT, TAP), (0, TAP), (UP, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (0, SETTLE)]),
    shot("152-diagnostics-boot", &[(0, SETTLE), (SELECT, TAP), (0, TAP), (UP, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (0, SETTLE)]),
    // The status items (SELECT > Appearance > Status bar): rows Show,
    // Position, Wi-Fi, Bluetooth, Battery, Battery percent, Clock.
    shot("160-status-menu", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    shot("161-status-left", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("162-status-no-wifi", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("163-status-clock-only", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    shot("164-status-left-no-clock", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (UP, TAP), (0, TAP), (UP, TAP), (0, TAP), (LEFT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)]),
    // Find artwork (editor, sixth row): SteamGridDB without a key, then
    // with the fake server's key: games, candidates, one applied; libretro
    // for a RetroArch game (SNES tab).
    shot("170-find-art-key", &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    Shot { name: "171-find-art-games", script: &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)], setup: keyed },
    Shot { name: "172-find-art-icon", script: &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, TAP), (0, SETTLE)], setup: keyed },
    Shot { name: "173-find-art-backdrop", script: &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (DOWN, TAP), (0, TAP), (0, SETTLE)], setup: keyed },
    Shot { name: "174-find-art-applied", script: &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (RIGHT, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, TAP), (CROSS, TAP), (0, TAP), (CROSS, TAP), (0, TAP), (CROSS, TAP), (0, TAP), (0, SETTLE)], setup: keyed },
    shot("175-find-art-libretro", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("176-find-art-libretro-icon", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("177-find-art-libretro-title-404", &[(0, 4), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    // Fetch artwork (SELECT > Library, second row): the setup on the
    // current category, a run against the fake server, its end; Start with
    // no key asks for one. Clean up artwork (third row): unused files, then
    // the question before a delete.
    Shot { name: "180-fetch-art-setup", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)], setup: keyed },
    Shot { name: "181-fetch-art-running", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, 30)], setup: keyed },
    Shot { name: "182-fetch-art-done", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, 3000)], setup: keyed },
    shot("183-fetch-art-key", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("185-clean-up", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("186-clean-up-confirm", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    // Updates (SELECT > Updates > Check for updates) against a fake GitHub
    // offering 0.2.0-beta.1 over the fake build's 0.1.0 (beta): offered;
    // Download refused under the side-by-side title id; with the real one,
    // download, unpack, install and the updater's start; the report after
    // the restart.
    shot("190-update-available", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("191-update-not-supported", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    Shot { name: "192-update-downloading", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, 3)], setup: installable },
    Shot { name: "193-update-unpacking", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, 30)], setup: installable },
    Shot { name: "194-update-ready", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, 80)], setup: installable },
    Shot { name: "195-update-restarting", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE), (CIRCLE, TAP), (0, 80), (CIRCLE, TAP), (0, 40)], setup: installable },
    Shot { name: "196-update-done", script: &[(0, SETTLE)], setup: updated },
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
            (START, TAP), (0, TAP), (RIGHT, 40), (0, TAP), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, 40), (0, TAP),
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
    // Text size: SELECT > Appearance > Text size (fourth row); the rest
    // start from a saved size.
    Shot { name: "200-text-size-menu", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)], setup: plain },
    Shot { name: "201-larger-home", script: &[(0, SETTLE)], setup: larger },
    Shot { name: "202-larger-home-detailed", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "203-larger-carousel-detailed", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "204-larger-grid", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "205-larger-list-detailed", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "206-larger-cascade-detailed", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "207-larger-menu", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "208-larger-status-menu", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "209-larger-keyboard", script: &[(0, 4), (SQUARE, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "210-larger-editor", script: &[(0, 4), (TRIANGLE, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "211-larger-backdrop-picker", script: &[(0, 4), (TRIANGLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "212-larger-categories", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "213-larger-rescan", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "214-larger-diagnostics", script: &[(0, SETTLE), (SELECT, TAP), (0, TAP), (UP, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "215-larger-update", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "216-larger-clean-up", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)], setup: larger },
    Shot { name: "217-larger-turkish-menu", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)], setup: larger_turkish },
    Shot { name: "218-larger-turkish-home", script: &[(0, SETTLE)], setup: larger_turkish },
    Shot { name: "219-larger-first-start", script: &[(0, 20)], setup: larger_first_start },
    Shot { name: "220-smaller-home-detailed", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)], setup: smaller },
    Shot { name: "221-smaller-list-detailed", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)], setup: smaller },
    Shot { name: "222-smaller-menu", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)], setup: smaller },
    // Sort: the first row of the SELECT menu, the current tab's order.
    shot("230-sort-menu", &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)]),
    Shot { name: "231-sorted-recently-played", script: &[(0, SETTLE)], setup: sorted_recent },
    Shot { name: "232-sorted-newest-installed", script: &[(0, SETTLE)], setup: sorted_installed },
    Shot { name: "233-larger-german-sort", script: &[(0, 4), (SELECT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (0, SETTLE)], setup: larger_german },
    // A PSP game's 144:80 icon: the ring, the shadow and the shade around
    // the picture, not the square it is fitted into (PSP tab, second title).
    shot("240-psp-carousel", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (RIGHT, TAP), (0, SETTLE)]),
    shot("241-psp-grid", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (RIGHT, TAP), (0, SETTLE)]),
    shot("242-psp-cascade", &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (R, TAP), (0, TAP), (RIGHT, TAP), (0, SETTLE)]),
    // A portrait box art for Gravity Daze, fitted whole: the ring around it.
    Shot { name: "243-tall-art-home", script: &[(0, SETTLE)], setup: tall_art },
    Shot { name: "244-tall-art-carousel", script: &[(0, 4), (SELECT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (SELECT, TAP), (0, TAP), (0, SETTLE)], setup: tall_art },
    // Touch: a tap on the next Home tile selects it, a second one on it
    // asks to launch; a tab name; the footer's Menu hint, then a row; a
    // drag and flick along Home; typing on the keyboard (each drawer
    // tapped once it has slid in); a row's arrow in
    // the editor; a tap outside a drawer closes it.
    shot("250-touch-tile", &[(0, 4), (touch(320, 140), TAP), (0, SETTLE)]),
    shot("251-touch-tile-launch", &[(0, 4), (touch(320, 140), TAP), (0, TAP), (touch(140, 140), TAP), (0, SETTLE)]),
    shot("252-touch-tab", &[(0, 4), (touch(496, 26), TAP), (0, SETTLE)]),
    shot("253-touch-menu-row", &[(0, 4), (touch(300, 518), TAP), (0, SETTLE), (touch(600, 152), TAP), (0, SETTLE)]),
    shot("254-touch-drag", &[(0, 4), (touch(700, 150), 1), (touch(660, 150), 1), (touch(620, 150), 1), (touch(580, 150), 1), (touch(540, 150), 1), (touch(500, 150), 1), (touch(460, 150), 1), (touch(420, 150), 1), (0, SETTLE)]),
    shot("255-touch-keyboard", &[(0, 4), (touch(180, 518), TAP), (0, SETTLE), (touch(102, 268), TAP), (0, TAP), (touch(186, 268), TAP), (0, SETTLE)]),
    shot("256-touch-editor-arrow", &[(0, 4), (touch(80, 518), TAP), (0, TAP), (touch(896, 96), TAP), (0, SETTLE)]),
    shot("257-touch-outside-closes", &[(0, 4), (touch(300, 518), TAP), (0, TAP), (touch(120, 300), TAP), (0, SETTLE)]),
    // The system keyboard (the stand-in answers "st" after 20 frames):
    // square opens it, its text searches the library.
    Shot { name: "260-system-keyboard-search", script: &[(0, 4), (SQUARE, TAP), (0, 40)], setup: system_keyboard },
    // The Keyboard row, switched to Pocket Shelf's.
    shot("261-keyboard-setting", &[(0, 4), (SELECT, TAP), (0, TAP), (UP, TAP), (0, TAP), (UP, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, SETTLE)]),
    // Pocket Shelf's keyboard, after the Vita's: "st", the cursor moved
    // back two (L) and "a" typed there; Shift pressed twice locks capitals;
    // the 123 page and its #+= page; a letter's accents held open with the
    // confirm button, and with a finger, then one tapped.
    shot("262-keyboard-cursor", &[(0, 4), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (L, TAP), (0, TAP), (L, TAP), (0, TAP), (LEFT, TAP), (0, TAP), (LEFT, TAP), (0, TAP), (LEFT, TAP), (0, TAP), (LEFT, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, SETTLE)]),
    shot("263-keyboard-shift-lock", &[(0, 4), (SQUARE, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (DOWN, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("264-keyboard-digits", &[(0, 4), (SQUARE, TAP), (0, TAP), (UP, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("265-keyboard-symbols", &[(0, 4), (SQUARE, TAP), (0, TAP), (UP, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (UP, TAP), (0, TAP), (CIRCLE, TAP), (0, TAP), (0, SETTLE)]),
    shot("266-keyboard-accents-held", &[(0, 4), (SQUARE, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (RIGHT, TAP), (0, TAP), (CIRCLE, 40), (0, SETTLE)]),
    shot("267-touch-keyboard-accent", &[(0, 4), (SQUARE, TAP), (0, SETTLE), (touch(270, 268), 40), (0, TAP), (touch(228, 332), TAP), (0, SETTLE)]),
];

fn main() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.build/validation/desktop");
    std::fs::create_dir_all(&out).expect("create output folder");

    for shot in SHOTS.iter().filter(|shot| words.is_empty() || words.iter().any(|w| shot.name.contains(w.as_str()))) {
        let mut host = (shot.setup)();
        let mut gfx = Gfx::new();
        let mut app = App::new();
        for &(step, frames) in shot.script {
            let (pad, finger) = decode(step);
            for _ in 0..frames {
                let started = Instant::now();
                app.events(&mut host, &mut gfx);
                app.update(&mut host, pad, finger);
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
