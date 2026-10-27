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
texture. **A clip is applied on the CPU**: each rectangle and its texture
coordinates are cut to it, so a backend draws no stencil (vita2d's clip
cost two full-screen stencil draws per change, and the clipped top bar
flickered on hardware). Rounded rectangles, rings, soft shadows and the
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
| `bun run next:locales` | Write `core/src/locales.rs` from `src/locales/*.ts` |
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

**The Vita host writes `ux0:data/PocketShelf/` only for the files with a
round-trip test**: `settings.json`, `categories.json`, `recent.json`,
`titles.cache.json` and `titles/<id>.json` (`writable` in `vita/host.rs`);
it deletes only `titles/<id>.json`. **Writes run on a file thread, in
order** (`FileOp` in `vita/host.rs`): on hardware a `settings.json` write
in the frame took 24–30 ms, so every view change was a missed refresh.
`Host::flush` waits for them before a launch ends the process. The folder
is the current app's, so a
setting changed in this build is the current app's setting too. This
build's log is `ux0:data/PocketShelfNext/log.txt`.

**Titles reach the frame loop as `Game` values made on the host's thread**
(`load_catalog`, then `Event::Titles`), and a `Game`'s strings are
`Arc<str>`: the library copies every title at each change, and a copy is a
count. The library's maps hash with FxHash (`library/fxhash.rs`): the
default SipHash's 64-bit arithmetic is several instructions per step on the
Vita's 32-bit CPU. With 5,738 titles, the rebuild is 11.6 ms in Vita3K and
0.65 ms on the desktop, and 38.6 ms on hardware (111 ms before the art
lookups and the hasher changed). The log has a
`library: <n> titles in <us>` line for each rebuild.

## The shelf

`core/src/shelf/` draws the current app's shelf from its state every frame:
the top bar with the sliding tab names and the status items (`chrome.rs`),
the five views at the three detail levels (`views.rs`), the art with its
icon box and initials (`art.rs`), backdrops with their wash, the tint wash,
the Dynamic theme's blend, the footer, and drawers (`panel.rs`). Sizes and
timings are the current app's (`layout.rs`, `navigation.rs`); the motion
that `animate` tweens there is a `Tween` per value here, eased out over the
same time. Tiles are square and the selection ring has radius 3, as in the
current app.

**Pictures** decode on two threads in the Vita host: icons and box art in
the order asked, and backdrops, where only the newest request waits
(`Requests` in `vita/host.rs`). Each frame the shelf asks for the pictures
near the selection and takes back (`Host::cancel`) the requests for titles
it scrolled past (`Art::want`). With one queue for both, a fast scroll
queued an icon for every title passed, and the backdrop of the title the
user stopped on waited behind them while the previous one stayed up. The
three most recent backdrops stay in memory after they leave the screen
(`BACKDROPS_KEPT`, as the current app's `art-files.ts`), so going back to a
title shows its picture at once. A backdrop decoded in 0.5–1.4 s on
hardware; its scaling did a 64-bit division per source pixel, a library
call on the Vita's 32-bit CPU, and now reads each column's texel from a
table; PNG checksums are skipped. The log splits each backdrop into read,
decode and scale times.

**Launching** (`launch_selected`): a title with no bubble first needs
Adrenaline's boot plugin on, and the drawer offers to add it or turn it on;
then `recent.json` and `settings.json` are written, the host accepts the id
(`installed::launch`), and once the frame with the "Launching" line is on
screen the main loop sends the request and the process ends
(`installed::finish_launch`). A launch the host rejects puts `recent.json`
back. The VPK carries `psp/pocketshelf.prx`, which an Adrenaline launch
copies to the memory stick.

**The SELECT menu** (`menu.rs`, `components/menu.tsx`) holds every setting
on the current app's pages and saves each change; Fetch artwork, Clean up
artwork and Check for updates arrive in phase 5. **The title editor**
(TRIANGLE, `editor.rs`): favorite, category, title (keyboard), box art and
backdrop pickers with a preview and the decode check, the scraper choice,
reset (the favorite stays). **The category manager** (Library ▸
Categories): new, rename, delete (pressed twice; titles go back to their
own category), hide and show, reorder with L and R. **Rescan titles** scans
again with its progress in the drawer and reads the per-title files again;
**Diagnostics** lists the frame timing since it opened and this start's
marks (`tools.rs`). A change to one title recomputes that title alone
(`Library::patch`). **Search** (`keyboard.rs`):
SQUARE opens the keyboard (three pages, the accents page led by the
language's own letters), Done shows the results in a Search tab in front,
the other face button on that tab closes it and returns to the title
selected before.

**Languages**: `locales.rs` is generated from the current app's
`src/locales/*.ts` (`bun run next:locales`, `tools/locales.ts`), one
`Messages` struct and a constant per language; `i18n.rs` has `fill`,
`upper` (the Turkish i) and the built-in categories' names by key. Every
screen reads its words from the chosen language.

On a first start (no `titles.tsv`), the scan page shows first: the mark,
the name, a bar and the phase with its count (`components/scan-page.tsx`),
fed by `Event::Scan`, until the titles arrive and 8 frames after.

Desktop shots: `02`–`06` Home (views and details are set through the menu), `10`–`13` Carousel, `20`–`22` Grid, `30`–`32`
List, `40`–`42` Cascade, `50`–`53` tabs, `60`–`63` themes, `70`–`72` the scan
page, `80`–`82` saved data, `90` test titles, `91` launching, `92`–`93` the
Adrenaline drawer, `100`–`105` the SELECT menu, `110`–`115` the keyboard and
search, `120`–`134` the editor, pickers and category manager, `140`–`150`
rescan and Diagnostics, `64`–`66` Turkish and German.

## Timing

The log gets (the Diagnostics screen, phase 4, will show the same):

- the scan's time, the library's rebuilds, presses of SELECT, START, SQUARE
  and TRIANGLE, and the timing line at frames 600 and 1800 and at a launch;
- **each frame over 25 ms** (a missed display refresh), at most 200 per run,
  with where its time went: waiting for the GPU, host answers
  taken (and how many), update, draw (commands, new glyphs, new textures),
  texture uploads (count, KB), and handing the commands to the GPU.
