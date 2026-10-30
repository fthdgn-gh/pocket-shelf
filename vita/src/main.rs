//! Native Pocket Shelf on the Vita: no JavaScript engine, the UI in Rust,
//! drawn with vita2d at 960×544.

mod backend;
mod host;
mod platform;
mod status;

use shelf_core::app::{buttons, App};
use shelf_core::gfx::Gfx;
use shelf_core::host::Host;
use shelf_core::stats::Trace;
use vita2d_sys::*;
use vitasdk_sys::*;

use host::{now_us, VitaHost};

#[no_mangle]
#[used]
pub static sceUserMainThreadStackSize: u32 = 1024 * 1024;

/// Padding at the end of the read-only segment. `vita-elf-create` fails with "segment 1
/// overlaps" when the SCE data it appends to segment 0 does not fit before
/// segment 1; resizing this by a few KiB moves segment 1 (as `LINK_PAD` in
/// the PocketJS host did).
#[used]
static LINK_PAD: [u8; 8192] = [0x5a; 8192];

/// A frame longer than this is logged with where its time went: it missed
/// a display refresh (a missed one makes a frame 33 ms). The "over 20 ms"
/// count stays at 20 ms, as in Pocket Shelf 0.1's Diagnostics; Vita3K's own
/// timing puts many frames at 20-21 ms.
const SLOW_US: u32 = 25_000;
/// At most this many slow frames are logged per run (each SELECT starts over).
const SLOW_LOG_MAX: u32 = 200;

fn read_pad() -> u32 {
    let mut raw: SceCtrlData = unsafe { core::mem::zeroed() };
    unsafe { sceCtrlPeekBufferPositive(0, &mut raw, 1) };
    let map = [
        (SCE_CTRL_UP, buttons::UP),
        (SCE_CTRL_DOWN, buttons::DOWN),
        (SCE_CTRL_LEFT, buttons::LEFT),
        (SCE_CTRL_RIGHT, buttons::RIGHT),
        (SCE_CTRL_CROSS, buttons::CROSS),
        (SCE_CTRL_CIRCLE, buttons::CIRCLE),
        (SCE_CTRL_SQUARE, buttons::SQUARE),
        (SCE_CTRL_TRIANGLE, buttons::TRIANGLE),
        (SCE_CTRL_START, buttons::START),
        (SCE_CTRL_SELECT, buttons::SELECT),
        (SCE_CTRL_LTRIGGER, buttons::L),
        (SCE_CTRL_RTRIGGER, buttons::R),
    ];
    map.iter().filter(|(vita, _)| raw.buttons & *vita != 0).fold(0, |pad, (_, bit)| pad | bit)
}

/// The first finger on the front touch screen, in screen pixels: the panel
/// reports 1920×1088, twice the screen's 960×544.
fn read_touch() -> Option<(f32, f32)> {
    let mut data: SceTouchData = unsafe { core::mem::zeroed() };
    let read = unsafe { sceTouchPeek(SCE_TOUCH_PORT_FRONT, &mut data, 1) };
    if read < 1 || data.reportNum == 0 {
        return None;
    }
    let report = data.report[0];
    Some((report.x as f32 / 2.0, report.y as f32 / 2.0))
}

fn main() {
    let mut host = VitaHost::new();
    host.log("start");
    unsafe {
        if vita2d_init_advanced(2 * 1024 * 1024) < 0 {
            host.log("vita2d_init_advanced failed");
            return;
        }
        vita2d_set_vblank_wait(1);
        vita2d_set_clear_color(0xff00_0000);
        sceCtrlSetSamplingMode(SCE_CTRL_MODE_DIGITAL);
        sceTouchSetSamplingState(SCE_TOUCH_PORT_FRONT, SCE_TOUCH_SAMPLING_STATE_START);
    }

    let mut app = App::new();
    let mut gfx = Gfx::new();
    let mut backend = backend::Backend::default();
    let mut last_start = now_us();
    let mut frame = 0u64;
    let mut previous_pad = 0u32;
    let mut last = (Trace::default(), 0u32);
    let mut slow_logged = 0;

    loop {
        let start = now_us();
        let frame_us = (start - last_start) as u32;
        last_start = start;
        // From the first frame on: the time after it counts in the timing
        // line too (a 133 ms gap there went unexplained on hardware).
        if frame >= 1 && frame_us > SLOW_US && slow_logged < SLOW_LOG_MAX {
            host.log(&last.0.line(frame - 1, frame_us, last.1));
            slow_logged += 1;
        }

        let mut trace = Trace::default();
        // The GPU finishes the previous frame before textures change.
        unsafe { vita2d_wait_rendering_done() };
        let mut at = now_us();
        let wait = (at - start) as u32;
        let work_start = at;
        let mut lap = || {
            let now = now_us();
            let spent = (now - at) as u32;
            at = now;
            spent
        };
        trace.wait = wait;

        trace.event_count = app.events(&mut host, &mut gfx) as u32;
        trace.events = lap();

        let pad = read_pad();
        let pressed = pad & !previous_pad;
        previous_pad = pad;
        // Until the SELECT menu: SELECT the view, START the details, SQUARE
        // the theme, TRIANGLE the test titles.
        for (bit, name) in [(buttons::TRIANGLE, "TRIANGLE"), (buttons::SELECT, "SELECT"), (buttons::START, "START"), (buttons::SQUARE, "SQUARE")] {
            if pressed & bit != 0 {
                host.log(&format!("pressed {name} at frame {frame}"));
            }
        }
        app.update(&mut host, pad, read_touch());
        if app.quit {
            host.log(&format!("quit: frames {}  {}", app.stats.frames, app.stats.line()));
            break;
        }
        trace.update = lap();

        app.draw(&mut gfx);
        trace.commands = gfx.cmds.len() as u32;
        trace.glyphs = gfx.work.glyphs;
        trace.textures_added = gfx.work.textures_added;
        if gfx.work.atlas_reset {
            host.log(&format!("glyph atlas started over ({} times)", gfx.atlas_resets()));
        }
        trace.draw = lap();

        let uploads = unsafe { backend.sync(&mut gfx.textures) };
        trace.uploads = uploads.count;
        trace.upload_kb = (uploads.bytes / 1024) as u32;
        trace.sync = lap();

        unsafe {
            vita2d_start_drawing();
            vita2d_clear_screen();
            backend.run(&gfx.cmds);
            vita2d_end_drawing();
            // The system keyboard, while open, over the frame.
            if platform::ime::active() {
                vita2d_common_dialog_update();
            }
        }
        trace.submit = lap();
        let cpu_us = (now_us() - work_start) as u32;
        unsafe { vita2d_swap_buffers() };

        // A launch the host accepted: this frame (with its "Launching" line)
        // is on screen, so the request goes out and the process ends
        // (`platform::installed::finish_launch`). A request the system refuses returns
        // here and the app goes on.
        if let Some(request) = app.request.take() {
            let id = match request {
                shelf_core::shelf::Request::Launch(id) => id,
                shelf_core::shelf::Request::Updater => String::from("the updater"),
            };
            // The launch frame's own breakdown: the process ends before the
            // next frame would log it as slow.
            let so_far = (now_us() - start) as u32;
            host.log(&trace.line(frame, so_far, cpu_us));
            host.log(&format!("launch {id}: frames {}  {}", app.stats.frames, app.stats.line()));
            // "Last Played" and the settings are on the card, and the log
            // thread has written the line, before the process ends.
            host.flush();
            std::thread::sleep(std::time::Duration::from_millis(30));
            unsafe {
                vita2d_wait_rendering_done();
                platform::installed::finish_launch();
            }
            host.log(&format!("launch {id} refused by the system"));
        }

        frame += 1;
        if frame == 1 {
            host.log("first frame");
        } else {
            app.stats.push(cpu_us, frame_us);
        }
        last = (trace, cpu_us);
        if frame == 600 || frame == 1800 {
            host.log(&format!(
                "frame {frame}: {} titles, textures {} KB, {}",
                app.len(),
                gfx.textures.live_bytes() / 1024,
                app.stats.line()
            ));
        }
    }

    unsafe {
        vita2d_wait_rendering_done();
        vita2d_fini();
    }
    // Give the log thread time to write the last lines.
    drop(host);
    std::thread::sleep(std::time::Duration::from_millis(100));
    unsafe { sceKernelExitProcess(0) };
}
