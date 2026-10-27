# Port plan: Pocket Shelf without PocketJS

The spike (`README.md`) ran on hardware on 2026-10-27: 173 titles, first frame
at 0.76 s, 2.4–2.9 ms CPU per frame, one frame over 20 ms in 20 s of
scrolling 5,173 titles. The current app's best reading: 8.7 ms JS per frame,
longest frame 237 ms, first frame at 1.85 s. This plan moves every feature to
the native build, then removes PocketJS from the repository.

## What carries over

| Part | Lines | Fate |
|---|---|---|
| Platform modules in `hosts/vita/src/`: `installed`, `pspemu`, `iso`, `psm`, `appdb`, `retroarch`, `dds`, `accent`, `seplugins`, `adrenaline_config`, `http`, `net`, `update`, `status`, `jobs`, `art`, `backdrop`, `datafs` | ~5,000 | Moved as they are. 9 of them name no engine type; the rest lose a few lines of JS glue (texture handles, `Ui`). |
| Engine glue: `ffi`, `lib` runtime, `graphics`, `pak`, `switch`, `package_file`, `dev*`, `audio`, `vid`, `bench` | ~5,000 | Dropped. |
| App logic in `src/*.ts`: `state`, `catalog`, `categories`, `overrides`, `settings`, `recent`, `search`, `consoles`, `scrape`, `online`, `libretro`, `steamgriddb`, `updates`, `update-flow`, `themes`, `i18n` and `locales/` | ~5,600 | Rewritten in Rust, same behavior and file formats. |
| Views and components (`src/views`, `src/components`) | ~2,500 | Rewritten on the spike's drawing core. |
| Tests in `src/tests/` | ~1,200 | Rewritten as Rust tests, same cases. Rust tests already in `accent`, `dds`, `seplugins`, `retroarch`, `iso` keep running. |
| Preview (`src/tools/preview.ts`) | 968 | Replaced by the desktop renderer with a fake host and the same shot scripts. |
| `src/psp-boot/`, `src/updater/`, `src/vita/` (LiveArea, plugin, updater VPK), `src/tools/art.ts`, `icons.ts` | | Kept; the VPK packs the same files. |

## Rules for the whole port

- **Saved data stays compatible** (released v0.x): `settings.json`,
  `categories.json`, `recent.json`, `titles/<id>.json`,
  `titles.cache.json`, `titles.tsv` (version 6), `art/`, `backdrops/`,
  `steamgriddb.txt`, `update.json` are read and written in their current
  formats. Each format gets a round-trip test against files written by the
  current app.
- **Side by side until parity.** The port builds as its own bubble (title id
  to be chosen, not starting with `PC`) and reads `ux0:data/PocketShelf/`.
  It writes there only once the writer for that file has its round-trip
  test. The released app stays `POCKTSHLF` until cutover.
- **Every screen has desktop shots.** The fake host answers what
  `preview.ts` answers today (title table, icons, backdrops, network,
  SteamGridDB, libretro), so screens are checked without the console.
- **Timing stays visible.** Diagnostics keeps frame timing; slow frames are
  logged with what ran in them (upload, decode result, glyph, scan
  message), so perf work happens as it comes up.

## Architecture

- `core/`: state, logic and screens. No platform calls; a `Host` trait for
  files, title scan, pictures, network, launch, status and time. Tests run on
  the desktop with `cargo test`.
- `vita/`: `Host` on the moved platform modules, the vita2d backend, the frame
  loop. Worker threads (`jobs`) for scans, decoding and network; results
  reach the frame loop through channels.
- `desktop/`: the software backend, the fake `Host`, the shot runner.
- **Screens are drawn from state every frame** (no retained tree): each view is
  a function of the state and the time. Tweens are values eased per frame.
  Only what is on screen is drawn, as `views/window.ts` does now.
- **Textures** have a budget: icons and art are kept, backdrops are freed
  once no longer drawn, after `vita2d_wait_rendering_done`, recycling
  same-size allocations (Vita3K faults on live texture frees).

## Phases

Each phase ends with desktop shots, a Vita3K start, and a hardware build for
the user.

1. **Foundations.** Workspace layout, `Host` trait, fake host with the
   preview's sample data, shot runner with button scripts, text wrapping,
   themes as palettes, texture budget and backdrop freeing, slow-frame log.
   *Done 2026-10-27 (desktop shots, Vita3K); hardware run pending.* Left for
   later phases: an icon budget (icons are never freed, as in the current
   app, which a RetroArch library with art would outgrow), and dropping picture
   requests the user has scrolled past. First hardware run: every slow
   frame was a backdrop upload (2 MB, 21 ms) or the whole glyph atlas; both
   are now spread or cut (README "Drawing").
2. **Library.** *In progress. Done (2026-10-27): the platform modules are
   in `vita/src/platform/`, changed only where they made engine textures;
   icons (with their accent) and backdrops (512×256, as before) decode on
   the picture thread; the saved `titles.tsv` is read (5,777 titles in 33 ms
   in Vita3K); a scan saves to `ux0:data/PocketShelfNext/` for now. Next: the
   core's catalog, categories, overrides, settings and recent, the scan
   page, round-trip tests, and the platform modules' own tests on the
   desktop.* Move the platform modules; scan to `titles.tsv` on a worker
   thread with the scan page; read `titles.tsv`, overrides (via
   `titles.cache.json`), categories, settings and recent; catalog, category
   order, smart categories. Port `logic.test.ts`, `consoles.test.ts`.
3. **Shelf.** Header with sliding tabs, footer, status bar, the five views
   (Home, Carousel, Grid, List, Cascade), details levels, backdrops, accent
   and the Dynamic theme, held-direction acceleration. **Launching**: Vita,
   PSM, system apps, RetroArch, and Adrenaline with the plugin consent
   drawer; the exit sequence from `finish_launch`. After this phase the port
   can replace the current app for daily use.
4. **Panels.** SELECT menu and its groups, all settings, languages (five
   locales), keyboard (three pages) and search, title editor, category
   manager, rescan, Diagnostics with startup and frame timing.
5. **Online.** Network and TLS (`http`, `net`), SteamGridDB and libretro
   scrapers, art and backdrop pickers, Fetch artwork, Clean up artwork,
   updates with the updater app. Port `steamgriddb`, `libretro`, `updates`,
   `accent`, `status` tests.
6. **Cutover.** Parity check against the current app's preview shots and a
   copy of the user's data folder; release the port as `POCKTSHLF` on the
   beta channel, so installed copies update to it through the existing
   updater; then delete PocketJS (`framework/`, `engine/`, `apps/`, other
   `hosts/`, `tools/`, `site/`, ...), move `native/` to the root, rewrite
   the README and `src/CLAUDE.md`, ask GitHub Support to detach the fork,
   and remove the `upstream` remote.

## Decisions

- Side by side as **Pocket Shelf Next**, `PSHLFNEXT` (2026-10-27).
- **Everything carries over**; dropping or adding features waits until the
  port is complete (2026-10-27).
