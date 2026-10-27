# Architecture

Pocket Shelf is a Rust app drawn with vita2d, with no JavaScript engine. It
replaced the PocketJS app in 0.2.0 (the port's phases are in `PLAN.md`) and
reads and writes that app's saved data unchanged.

| Folder | Holds |
|---|---|
| `core/` | State and screens, text, shapes, themes, frame timing, the `Host` trait, the library (`core/src/library/`). No platform code; `cargo test` runs on the desktop. |
| `desktop/` | A software rasterizer for the same draw commands, a fake `Host` with a sample library, a fake SteamGridDB, libretro, GitHub and updater, and scripted screenshots. |
| `vita/` | `Host` on the Vita (title scan, picture decoding, file writes, downloads and the log on worker threads), the vita2d backend, the frame loop, the platform modules (`vita/src/platform/`), and the VPK's other files (`vita/package/`). |
| `platform-tests/` | The Vita platform modules with no system calls (`accent`, `adrenaline_config`, `dds`, `iso`, `retroarch`, `seplugins`), included by path so their tests run on the desktop. |
| `tools/` | `make art` draws the LiveArea pictures (`tiny-skia`, `fontdue`, a 256-color quantizer); `make updater` lays out the updater's package and its `head.bin`. |

## Drawing

Screens are laid out in 480×272 logical units, the numbers Pocket Shelf 0.1's
layout used; `Gfx` records commands at 2× (960×544).
The core emits two primitives: a filled rectangle and a tinted part of a
texture. **A clip is applied on the CPU**: each rectangle and its texture
coordinates are cut to it, so a backend draws no stencil (vita2d's clip
cost two full-screen stencil draws per change, and the clipped top bar
flickered on hardware). Rounded rectangles, rings, soft shadows and the
button symbols are coverage masks computed once on the CPU and drawn tinted;
an icon's corners are rounded in its RGBA data when it arrives. Text is
Figtree through `fontdue`, one 1024×1024 glyph atlas, the roles of
Pocket Shelf 0.1 at **about two steps smaller** (Body 11, Small 9; any size
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
| `make test` | Core tests, the platform modules' tests (with `.samples/retroarch-playlists` when present), the tools' tests |
| `make shots` | Screenshots to `.pocket-build/validation/desktop/`; `SHOTS="words"` for those whose names hold them |
| `make build` | Release VPK to `dist/vita/pocket-shelf.vpk` (`vita/build.sh`); `GITHUB_REF_NAME=shelf-v<version>` sets the version the update check compares |
| `make art` | Redraw the LiveArea pictures and `template.xml` in `vita/package/sce_sys/` |
| `make updater` | Build the updater app into `vita/package/updater/` (VitaSDK) |
| `make plugin` | Build the Adrenaline plugin into `vita/package/psp/` (pspdev) |

**The SELF must not be "safe".** cargo-vita signs with `vita-make-fself -s`;
on hardware a safe app cannot open `ux0:app`, and the first hardware run
listed 0 titles. Vita3K does not enforce this. `build.sh` signs again with
`-c` only, and writes `param.sfo` itself (`ATTRIBUTE2=12`, `APP_VER` from
`vita/Cargo.toml`).

**The title id reaches the code through `vita/build.rs`**, read from
`Cargo.toml`. cargo-vita does not pass it to the compiler; without it the
app listed itself, refused to download updates and could not pass its own
package check (0.2.0-beta.1's first build).

Vita3K: install with `Vita3K dist/vita/pocket-shelf.vpk` (that start runs
the last app), then start it with `Vita3K -r POCKTSHLF`. Vita3K reports a
TRIANGLE press about 10 s into every run with nobody at the keys.

## Data

The library (`core/src/library/`) reads and writes Pocket Shelf 0.1's files
in their formats: `settings.json`, `categories.json`, `recent.json`,
`titles/<id>.json` and `titles.cache.json`, and lists `art/` to match box
art by title id or title. **Each writer reproduces, byte for byte, what
0.1 wrote** (`library/round_trip.rs`): the fixtures in
`core/tests/fixtures/` came from 0.1's own `saveSettings`,
`saveCategoryConfig`, `saveRecent` and `saveOverrides`, run on an in-memory
data folder before the TypeScript app was removed; they are maintained test
data now. Reading drops what 0.1 dropped (unknown values, bad ids, labels over 16 characters,
titles over 40), with the same cases tested.

**The Vita host writes `ux0:data/PocketShelf/` only for named files**:
those with a round-trip test (`settings.json`, `categories.json`,
`recent.json`, `titles.cache.json`, `titles/<id>.json`), `status-bar.json`,
`steamgriddb.txt`, `update.json` and `update/version.txt` (`writable` in
`vita/host.rs`), pictures in `art/` and `backdrops/`, and downloads to
`libretro/` and `update/`; it deletes only `titles/<id>.json` and pictures. **Writes run on a file thread, in
order** (`FileOp` in `vita/host.rs`): on hardware a `settings.json` write
in the frame took 24–30 ms, so every view change was a missed refresh.
`Host::flush` waits for them before a launch ends the process. The log and
a title list the app scanned itself are in `ux0:data/PocketShelfNext/`, the
folder the side-by-side build used.

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

`core/src/shelf/` draws the shelf from its state every frame:
the top bar with the sliding tab names and the status items (`chrome.rs`),
the five views at the three detail levels (`views.rs`), the art with its
icon box and initials (`art.rs`), backdrops with their wash, the tint wash,
the Dynamic theme's blend, the footer, and drawers (`panel.rs`). Sizes and
timings are 0.1's (`layout.rs`, `navigation.rs`); each moving value is a
`Tween`, eased out. Tiles are square and the selection ring has radius 3.

**Pictures** decode on two threads in the Vita host: icons and box art in
the order asked, and backdrops, where only the newest request waits
(`Requests` in `vita/host.rs`). Each frame the shelf asks for the pictures
near the selection and takes back (`Host::cancel`) the requests for titles
it scrolled past (`Art::want`). With one queue for both, a fast scroll
queued an icon for every title passed, and the backdrop of the title the
user stopped on waited behind them while the previous one stayed up. The
three most recent backdrops stay in memory after they leave the screen
(`BACKDROPS_KEPT`), so going back to a
title shows its picture at once. A backdrop decoded in 0.5–1.4 s on
hardware; its scaling did a 64-bit division per source pixel, a library
call on the Vita's 32-bit CPU, and now reads each column's texel from a
table; PNG checksums are skipped. The log splits each backdrop into read,
decode and scale times.

**Find artwork** (editor, `online/flow.rs`): SteamGridDB (the key typed or
in `steamgriddb.txt`) or libretro (the console's listing, kept in
`libretro/`), the games found, then each kind's candidates. **Candidates
download into memory** (`NetKind::Bytes`, at most 8 MB, the last four
kept) and decode from there (`Host::decode`); only the one used is written
to `art/` or `backdrops/` (`Host::save_picture`, a libretro box art fitted
into 128 px first). 0.1 wrote every candidate and deleted the unused ones
on close. **libretro's name lists are read and parsed on a host thread**
(`Host::listing`, `Event::Listing`): 9,301 PlayStation names took 690 ms in
the frame.

**Fetch artwork** (`online/fetch.rs`, `shelf/artwork.rs`) runs the same
lookups over a category, one request at a time, saving straight to the
card; busy replies are waited out five times, other failures halt the run,
and title changes are written every 20 pictures. **Clean up artwork**
deletes unused files or all of them. **Updates** (`updates.rs`,
`shelf/update.rs`, `vita/src/platform/update.rs`): the GitHub check, the
download, the unpack and package check on a thread, then the updater app
(`updater/`), which installs the release and starts Pocket Shelf again.

**Launching** (`launch_selected`): a title with no bubble first needs
Adrenaline's boot plugin on, and the drawer offers to add it or turn it on;
then `recent.json` and `settings.json` are written, the host accepts the id
(`installed::launch`), and once the frame with the "Launching" line is on
screen the main loop sends the request and the process ends
(`installed::finish_launch`). A launch the host rejects puts `recent.json`
back. The VPK carries `psp/pocketshelf.prx`, which an Adrenaline launch
copies to the memory stick.

**The SELECT menu** (`menu.rs`) holds every setting and saves each change. **The title editor**
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

**The status items** take the width of the ones shown: Wi-Fi only while
connected, and each of Wi-Fi, Bluetooth, battery and clock can be turned
off in Appearance ▸ Status bar (the Clock row's last value is Off), right of
R or left of L (`StatusLayout`, saved in `status-bar.json`: 0.1 rewrote `settings.json`
with only the keys it knew). The tab names have the rest of the bar.

**Languages**: `locales.rs` holds one `Messages` struct and a constant
per language (edited by hand; a missing text does not compile); `i18n.rs` has `fill`,
`upper` (the Turkish i) and the built-in categories' names by key. Every
screen reads its words from the chosen language.

On a first start (no `titles.tsv`), the scan page shows first: the mark,
the name, a bar and the phase with its count, fed by `Event::Scan`, until the titles arrive and 8 frames after.

Desktop shots: `02`–`06` Home (views and details are set through the menu), `10`–`13` Carousel, `20`–`22` Grid, `30`–`32`
List, `40`–`42` Cascade, `50`–`53` tabs, `60`–`63` themes, `70`–`72` the scan
page, `80`–`82` saved data, `90` test titles, `91` launching, `92`–`93` the
Adrenaline drawer, `100`–`105` the SELECT menu, `110`–`115` the keyboard and
search, `120`–`134` the editor, pickers and category manager, `140`–`150`
rescan and Diagnostics, `160`–`164` the status items' layout, `170`–`177`
Find artwork, `180`–`186` Fetch and Clean up artwork, `190`–`196` updates,
`64`–`66` Turkish and German.

## Timing

The log gets (the Diagnostics screen shows the same):

- the scan's time, the library's rebuilds, presses of SELECT, START, SQUARE
  and TRIANGLE, and the timing line at frames 600 and 1800 and at a launch;
- **each frame over 25 ms** (a missed display refresh), at most 200 per run,
  with where its time went: waiting for the GPU, host answers
  taken (and how many), update, draw (commands, new glyphs, new textures),
  texture uploads (count, KB), and handing the commands to the GPU.
