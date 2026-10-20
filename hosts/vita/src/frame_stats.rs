//! Where each frame's time goes, for the Diagnostics screen
//! (`ui.__frameStats`). The main loop times four parts of every frame:
//!
//! - `js`: the guest's frame call, its `onFrame` handlers, Solid's updates
//!   and QuickJS's garbage collection when an allocation triggers one;
//! - `layout`: the core's tick (layout and animations);
//! - `draw`: building the draw list and issuing it to vita2d;
//! - `present`: handing the frame to the GPU and waiting for the display.
//!
//! A frame that keeps up takes 16.7 ms in all, most of it in `present`.
//! Counted from the previous `take`, so each opening of the screen shows the
//! time since the one before.

use std::string::String;
use std::sync::Mutex;

const PARTS: usize = 4;
const LABELS: [&str; PARTS] = ["JS", "Layout", "Draw", "Present"];

struct Stats {
    /// The frame `take` was called in: it also counts the heap for the
    /// screen, which is not frame work, so it is left out.
    skip: bool,
    frames: u32,
    sum: [u64; PARTS],
    max: [u64; PARTS],
}

static STATS: Mutex<Stats> = Mutex::new(Stats { skip: false, frames: 0, sum: [0; PARTS], max: [0; PARTS] });

/// The host's idle garbage collections (`Runtime::collect_when_idle`): how
/// many, and the longest in microseconds, measured with the live-size count
/// that follows each.
static GC: Mutex<(u32, u64)> = Mutex::new((0, 0));

/// Record one idle collection that took `us` microseconds.
pub fn record_gc(us: u64) {
    if let Ok(mut gc) = GC.lock() {
        gc.0 += 1;
        gc.1 = gc.1.max(us);
    }
}

/// Microseconds since the process started.
pub fn now_us() -> u64 {
    unsafe { vitasdk_sys::sceKernelGetProcessTimeWide() }
}

/// Record one frame: the time stamps at its start and after each part.
pub fn record(start: u64, marks: [u64; PARTS]) {
    // A frame longer than this is a clock that jumped, not a frame.
    const BOGUS_US: u64 = 10_000_000;
    if marks.iter().any(|mark| mark.saturating_sub(start) > BOGUS_US || *mark < start) {
        return;
    }
    let Ok(mut stats) = STATS.lock() else {
        return;
    };
    if stats.skip {
        stats.skip = false;
        return;
    }
    stats.frames += 1;
    let mut previous = start;
    for (part, mark) in marks.into_iter().enumerate() {
        let spent = mark.saturating_sub(previous);
        stats.sum[part] += spent;
        stats.max[part] = stats.max[part].max(spent);
        previous = mark;
    }
}

/// `Label=value` pairs separated by commas: each part's average and longest
/// time in milliseconds since the last call, then counting starts over.
pub fn take() -> String {
    let Ok(mut stats) = STATS.lock() else {
        return String::new();
    };
    let frames = stats.frames.max(1) as u64;
    let ms = |us: u64| format!("{}.{}", us / 1000, us % 1000 / 100);
    let parts: Vec<String> = (0..PARTS)
        .map(|part| format!("{} avg/max={}/{} ms", LABELS[part], ms(stats.sum[part] / frames), ms(stats.max[part])))
        .collect();
    *stats = Stats { skip: true, frames: 0, sum: [0; PARTS], max: [0; PARTS] };
    let mut text = parts.join(",");
    if let Ok(mut gc) = GC.lock() {
        text.push_str(&format!(",Idle GC count/max={}/{} ms", gc.0, ms(gc.1)));
        *gc = (0, 0);
    }
    text
}
