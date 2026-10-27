# Pocket Shelf Next

Pocket Shelf without PocketJS: the UI in Rust, drawn with vita2d, no
JavaScript engine. It builds as its own bubble, **Pocket Shelf Next**
(`PSHLFNEXT`), until it matches the current app; the plan is in `PLAN.md`.

| Folder | Holds |
|---|---|
| `core/` | State and screens, text, shapes, themes, frame timing, the `Host` trait, the library (`core/src/library/`). No platform code; `cargo test` runs on the desktop. |
| `desktop/` | A software rasterizer for the same draw commands, a fake `Host` with the preview's sample library, and scripted screenshots. |
| `vita/` | `Host` on the Vita (title scan, picture decoding and the log on worker threads), the vita2d backend, the frame loop. |
| `platform-tests/` | The Vita platform modules with no system calls (`accent`, `adrenaline_config`, `dds`, `iso`, `retroarch`, `seplugins`), included by path so their tests run on the desktop. |
| `tools/` | `fixtures.test.ts`: the current app's save functions writing the round-trip fixtures. |

## Drawing

Screens are laid out in 480×272 logical units, the numbers the current UI's
`layout.ts` and class literals use; `Gfx` records commands at 2× (960×544).
The core emits two primitives: a filled rectangle and a tinted part of a
texture, plus clip changes. Rounded rectangles, rings, soft shadows and the
button symbols are coverage masks computed once on the CPU and drawn tinted;
an icon's corners are rounded in its RGBA data when it arrives. Text is
Figtree through `fontdue`, one 1024×1024 glyph atlas, the roles of
`src/text.ts` at **about two steps smaller** (Body 11, Small 9; any size
rasterizes, PocketJS stopped at 12) and an 8 px `Tiny`.

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
| `bun run next:test` | Core tests, then the platform modules' tests (with `.samples/retroarch-playlists` when present) |
| `bun run next:fixtures` | Write `core/tests/fixtures/` with the current app's own save functions |
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

The library (`core/src/library/`) reads and writes the current app's files
in their formats: `settings.json`, `categories.json`, `recent.json`,
`titles/<id>.json` and `titles.cache.json`, and lists `art/` to match box
art by title id or title. **Each writer reproduces, byte for byte, what the
current app writes** (`library/round_trip.rs`): the fixtures in
`core/tests/fixtures/` come from the app's own `saveSettings`,
`saveCategoryConfig`, `saveRecent` and `saveOverrides`, run by
`bun run next:fixtures` on an in-memory data folder. Reading drops what the
current app drops (unknown values, bad ids, labels over 16 characters,
titles over 40), with the same cases tested.

The Vita host still refuses `write` and `remove` in
`ux0:data/PocketShelf/`: the development screen changes nothing, and writing
is turned on with the first screen that does (phase 3: a launch writes
`recent.json` and `settings.json`). This build's log is
`ux0:data/PocketShelfNext/log.txt`.

**Titles reach the frame loop as `Game` values made on the host's thread**
(`load_catalog`, then `Event::Titles`), and a `Game`'s strings are
`Arc<str>`: the library copies every title at each change, and a copy is a
count. The library's maps hash with FxHash (`library/fxhash.rs`): the
default SipHash's 64-bit arithmetic is several instructions per step on the
Vita's 32-bit CPU. With 5,738 titles, the rebuild is 11.6 ms in Vita3K and
0.65 ms on the desktop, and 38.6 ms on hardware (111 ms before the art
lookups and the hasher changed). The log has a
`library: <n> titles in <us>` line for each rebuild.

## The development screen

Until the real views arrive (phase 3), one screen: the current tab's titles
in a grid, the selected title's backdrop behind it, the tab's name and place
at the top, and under the grid the title with its platform, category, id
and favorite mark. The saved theme and selection are used at start. D-pad
moves (a press wraps, a held direction repeats and stops at the ends), L
and R change the tab, SQUARE the theme (not saved), TRIANGLE adds or removes
5,000 test titles (Homebrew), SELECT resets the timing, START quits.

On a first start (no `titles.tsv`), the scan page shows instead: the mark,
the name, a bar and the phase with its count (`components/scan-page.tsx`),
fed by `Event::Scan`, until the titles arrive and 8 frames after.

Desktop shots: `20`–`23` tabs, `30`–`33` a first start's scan page, `40`–`42`
saved data (the user's own category, Favorites, Last Played).

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
