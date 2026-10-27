# Pocket Shelf Next

Pocket Shelf without PocketJS: the UI in Rust, drawn with vita2d, no
JavaScript engine. It builds as its own bubble, **Pocket Shelf Next**
(`PSHLFNEXT`), until it matches the current app; the plan is in `PLAN.md`.

| Folder | Holds |
|---|---|
| `core/` | State and screens, text, shapes, themes, frame timing, the `Host` trait. No platform code; `cargo test` runs on the desktop. |
| `desktop/` | A software rasterizer for the same draw commands, a fake `Host` with the preview's sample library, and scripted screenshots. |
| `vita/` | `Host` on the Vita (title scan, picture decoding and the log on worker threads), the vita2d backend, the frame loop. |

## Drawing

Screens are laid out in 480×272 logical units, the numbers the current UI's
`layout.ts` and class literals use; `Gfx` records commands at 2× (960×544).
The core emits two primitives: a filled rectangle and a tinted part of a
texture, plus clip changes. Rounded rectangles, rings, soft shadows and the
button symbols are coverage masks computed once on the CPU and drawn tinted;
an icon's corners are rounded in its RGBA data when it arrives. Text is
Figtree through `fontdue`, one 1024×1024 glyph atlas, the eight roles of
`src/text.ts`.

Textures: the app frees a texture it no longer draws (`Textures::free`); the
Vita backend keeps released allocations for reuse up to 16 MB and drops a
texture's CPU copy once uploaded. **Texture memory takes CPU writes at about
100 MB/s on hardware** (a 960×544 backdrop, 2 MB, took 21 ms in one frame),
so pictures upload through a queue at 256 KB a frame and are drawn once
complete (`Textures::ready`); the glyph atlas uploads only the rows a new
glyph changed.

## Commands

| Command | Does |
|---|---|
| `bun run next:test` | Core tests |
| `bun run next:shots [words…]` | Screenshots to `.pocket-build/validation/native/desktop/` |
| `bun run next:build` | Release VPK to `dist/vita/pocket-shelf-next.vpk` (`vita/build.sh`) |

**The SELF must not be "safe".** cargo-vita signs with `vita-make-fself -s`;
on hardware a safe app cannot open `ux0:app`, and the first hardware run
listed 0 titles. Vita3K does not enforce this. `build.sh` signs again with
`-c` only.

Vita3K: install with `Vita3K dist/vita/pocket-shelf-next.vpk` (that start
runs the last app), then start it with `Vita3K -r PSHLFNEXT`. Vita3K reports
a TRIANGLE press about 10 s into every run with nobody at the keys.

## Data

The current app's folder, `ux0:data/PocketShelf/`, is read only: `write`
refuses until each file's writer has a round-trip test. This build's log is
`ux0:data/PocketShelfNext/log.txt`.

## The development screen

Until the real views arrive (phase 3), one screen: the titles in a grid,
the selected title's backdrop behind it. D-pad moves (a press wraps, a held
direction repeats and stops at the ends), TRIANGLE adds or removes 5,000 test
titles, L and R change the theme, SELECT resets the timing, START quits.

## Timing

Top right: CPU time per frame and the time from one frame's start to the
next (averages over 120 frames, then maximums), and frames over 20 ms, since
the start or the last SELECT. The log gets:

- the scan's time, presses of TRIANGLE, SELECT and START, and the timing line
  at frames 600 and 1800, before a SELECT reset and at START;
- **each frame over 25 ms** (a missed display refresh), at most 200 per run
  or per SELECT, with where its time went: waiting for the GPU, host answers
  taken (and how many), update, draw (commands, new glyphs, new textures),
  texture uploads (count, KB), and handing the commands to the GPU.
