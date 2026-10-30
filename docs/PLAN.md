# Port plan: Pocket Shelf without PocketJS

**Done (2026-10-28).** The port was released as `shelf-v0.2.0-beta.1` and
PocketJS was removed from the repository. This file is the record of the
port: paths below such as `src/`, `hosts/vita/` and `native/` are where
things were at the time. Open follow-ups are at the end.

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
2. **Library.** *Done 2026-10-27 (desktop shots, Vita3K); hardware run
   pending. The platform modules are in `vita/src/platform/`, changed only
   where they made engine textures; icons (with their accent) and backdrops
   (512×256, as before) decode on the picture thread; the saved
   `titles.tsv` is read (5,777 titles in 33 ms in Vita3K); a scan saves to
   `ux0:data/PocketShelfNext/` for now. `core/src/library/`: catalog,
   categories, consoles, overrides, settings, recent and search, with the
   cases of `logic.test.ts` and `consoles.test.ts` that cover them (47 core
   tests); round-trip tests against fixtures written by the current app's
   own save functions; the scan page; the platform modules' 35 tests on the
   desktop (`platform-tests/`). In Vita3K the library loads Vita3K's copy
   of the user's data (87 changed titles) in 17–19 ms, and the frame that
   takes 5,738 titles is 19–27 ms (it was 103 ms before the catalog moved to
   the scan thread and titles shared their strings).
   **First hardware run (2026-10-27, 5,905 titles):** library loaded in
   46 ms; the saved list read and turned into titles on the scan thread in
   409 ms (53 ms in Vita3K); the frame that takes them 125 ms (events
   111 ms), about 4× Vita3K; TRIANGLE 302 ms adding 5,000, 155 ms removing.
   No other frame over 25 ms: the spread uploads hold (the run before them,
   in the same log, lost 20–35 ms to each 2 MB backdrop). Then: art
   matching by lowercase name without `.png`, ASCII lowercased without the
   Unicode tables, FxHash for the library's maps (SipHash is 64-bit
   arithmetic on a 32-bit CPU), and no copy of the list before a rebuild:
   desktop rebuild 2.7 to 0.7 ms with 85 art files, Vita3K 11.6 ms. The log
   now has `library: <n> titles in <us>`. **Second hardware run:** rebuild
   38.6 ms (was 111), the frame taking the titles 52.7 ms (was 125),
   TRIANGLE to 10,905 titles 201 ms (was 302); still no other frame over
   25 ms. The rebuild runs on every change to a title, so phase 4's editor
   and favorite toggle need an update of the one title, not a rebuild. The
   timing line's 132.8 ms longest frame was the gap after the first frame,
   which the slow-frame log skipped; it logs that frame now.
   Writing to `ux0:data/PocketShelf/` stays off until phase 3.
   Search's keyboard and the category manager's screens are phase 4.* Move the platform modules; scan to `titles.tsv` on a worker
   thread with the scan page; read `titles.tsv`, overrides (via
   `titles.cache.json`), categories, settings and recent; catalog, category
   order, smart categories. Port `logic.test.ts`, `consoles.test.ts`.
3. **Shelf.** *Desktop shots and Vita3K done 2026-10-27; hardware run
   pending. `core/src/shelf/`: layout and navigation with the cases of
   `logic.test.ts` that cover them (71 core tests), the top bar, status
   items, footer, the five views at three detail levels, art with icon box
   and initials, backdrops and the tint wash, the Dynamic theme, held
   directions with acceleration, launching with the Adrenaline drawer, and
   writes for the files with round-trip tests. Until the SELECT menu,
   SELECT, START and SQUARE step the view, details and theme. Hardware
   (2026-10-27): a Vita game launches. Each view change cost 24–30 ms in
   the frame, writing `settings.json`, and the shelf's first frame 25 ms
   reading the status for the first time; writes now run on a file thread
   (flushed before a launch) and the first status reading is in the
   startup frame. Confirmed on hardware (2026-10-27): Vita, PSP (through
   Adrenaline) and NES (RetroArch) launches; no slow frame in use after
   the file thread, CPU-side clipping (the clipped top bar flickered with
   vita2d's stencil clip) and the picture queues (backdrops waited behind
   every icon asked for while scrolling). Backdrops take 0.3–1.1 s on
   hardware: for the 1920×620 pictures of PSP and PS1 titles, read 50–320
   ms, PNG decode 400–580 ms, scale 200–280 ms (wall time, with the icon
   thread running). Follow-ups: decode the neighbors' backdrops ahead, or a
   small scaled cache in this build's own folder; an icon budget (texture
   memory reached 16 MB in a 30 s run).* Header with sliding tabs, footer, status bar, the five views
   (Home, Carousel, Grid, List, Cascade), details levels, backdrops, accent
   and the Dynamic theme, held-direction acceleration. **Launching**: Vita,
   PSM, system apps, RetroArch, and Adrenaline with the plugin consent
   drawer; the exit sequence from `finish_launch`. After this phase the port
   can replace the current app for daily use.
4. **Panels.** *Desktop shots and Vita3K done 2026-10-27; hardware run
   pending. The SELECT menu with every setting, the five languages
   (generated from `src/locales/`), the keyboard and search, the title
   editor with the art and backdrop pickers, the category manager, rescan
   and Diagnostics; one title's change recomputes that title alone (the
   follow-up from phase 3); 94 core tests. Fetch artwork, Clean up artwork
   and updates are phase 5.* SELECT menu and its groups, all settings, languages (five
   locales), keyboard (three pages) and search, title editor, category
   manager, rescan, Diagnostics with startup and frame timing.
5. **Online.** *In progress 2026-10-27. Done: HTTPS in the port (rustls),
   SteamGridDB and libretro logic with tests, Find artwork in the editor
   (candidates kept in memory, only the one used written; on hardware),
   Fetch artwork (one request at a time, busy replies waited out, runs
   against the desktop fake server) and Clean up artwork. libretro name
   lists are read and parsed on a host thread (9,301 PlayStation names took
   690 ms in the frame). Updates: the check, the drawer, download, unpack,
   the updater app (packed as `updater/`) and its report after the restart,
   against a desktop fake GitHub and updater. The updater app installs and
   starts `POCKTSHLF` only, so under `PSHLFNEXT` the check runs but
   Download says this build cannot download updates, and there is no check
   at start; the install is seen on hardware after the cutover.* Network and TLS (`http`, `net`), SteamGridDB and libretro
   scrapers, art and backdrop pickers, Fetch artwork, Clean up artwork,
   updates with the updater app. Port `steamgriddb`, `libretro`, `updates`,
   `accent`, `status` tests.
6. **Cutover.** *Done 2026-10-28.* The port was checked on hardware over
   the user's data folder (settings, categories, artwork and the LiveArea
   kept) and released as `POCKTSHLF` 0.2.0-beta.1 on the beta channel. The
   first build of that tag compiled an empty title id (cargo-vita does not
   pass it; `vita/build.rs` now does) and its updater froze before
   restarting Pocket Shelf, as 0.1's did once; it was taken down, both were
   fixed, and the tag was released again from the fixed `main`. An update
   from a local 0.2.0-beta.0 build to it ran on hardware: download, unpack,
   install, and the restart. Then PocketJS was deleted (`framework/`,
   `engine/`, `apps/`, `hosts/`, `microts/`, `pocket3d/`, `site/`, `tools/`,
   the TypeScript app in `src/`, and the PocketJS workflows), `native/`
   moved to the root, `src/`'s remaining files moved to `locales/`,
   `fonts/`, `updater/`, `psp-boot/`, `tools/` and `vita/package/`, and
   `package.json` took over the version from `pocket.json`. The README,
   the repository instructions and the working notes (`docs/NOTES.md`,
   was `src/CLAUDE.md`) were rewritten. Then Bun and TypeScript went too:
   the build identity moved into `vita/build.rs` and the version into
   `vita/Cargo.toml`, `core/src/locales.rs` became the edited file, the art
   and updater scripts were ported to a Rust crate in `tools/` (the
   updater's `head.bin` byte for byte; the pictures within dithering noise
   of the committed ones, which were kept), and a `Makefile` replaced
   `package.json`. Left to the user: asking GitHub Support to detach the
   fork.

## Decisions

- Side by side as **Pocket Shelf Next**, `PSHLFNEXT` (2026-10-27), until
  the cutover: the port is `POCKTSHLF`, "Pocket Shelf", version 0.2.0, first
  released as `shelf-v0.2.0-beta.1` (the user, 2026-10-27). The tag keeps
  the `shelf-v` form, the only one installed copies' update check reads.
  `shelf-build.yml` builds the native port; `build.sh` writes param.sfo as
  the PocketJS build did (`ATTRIBUTE2=12`, `APP_VER` `00.02`) and packs
  `src/vita/sce_sys`, `psp/` and `updater/`.
- **Everything carries over**; dropping or adding features waits until the
  port is complete (2026-10-27).
- **The status bar's layout** comes now, ahead of the rest of the new
  features (the user, 2026-10-27): the status items take the width of the
  ones shown, each of Wi-Fi, Bluetooth, battery and clock can be hidden,
  and the items sit right or left. Saved in `status-bar.json`, not
  `settings.json`, which the current app rewrites with only its own keys.
- **A text size setting** comes after the port (the user, 2026-10-27): the
  roles' sizes are one table (`Role::style`), so a setting can scale it.
- **Smaller text** (2026-10-27): each role is about two steps under its
  `text.ts` size (Display 18, Heading 15, Title 13, Body 11, Small and
  Label 9), plus an 8 px bold `Tiny` for numbers inside symbols. PocketJS
  baked 12 px as its smallest size; `fontdue` rasterizes any size. Layouts
  ported in phases 3 and 4 use these sizes, not the `text-*` classes.
- **Screen pixels as the layout unit** (the user, 2026-10-30): the layouts
  came over in PocketJS's 480×272 units, drawn at 2×. Every length was
  doubled (the sizes above are now Display 36 px to Tiny 16 px) and
  `Gfx`'s scale removed; the desktop renders matched the previous ones byte
  for byte before roundings moved to whole pixels.

- **Sort orders per tab** (the user, 2026-10-30): name A–Z and Z–A, last
  played (the system's `app.db` and Pocket Shelf's launches), launch count
  (Pocket Shelf's own: the system keeps none), install date (`app.db`).
  Play time from TrackPlug or RetroArch's runtime logs waits until
  RetroArch titles need it.

- **Touch everywhere** (the user, 2026-10-30): the shelf, every menu and
  drawer, and the keyboard; a tap selects a tile and a second tap on it
  launches; the front panel only.

- **Two keyboards** (the user, 2026-10-30): the Vita's own (`sceImeDialog`)
  or Pocket Shelf's, chosen in SELECT ▸ Keyboard. Pocket Shelf's was
  rebuilt after the system's layout and buttons, as the user confirmed
  (□ delete, △ space, L and R the cursor, START done, held ○ accents).

## Follow-ups

- **Icon texture budget**: *done 2026-10-30, hardware run pending.* Icon
  and box art textures are freed past 6 MB, those used longest ago first
  (`Art::trim`); texture memory had reached 16 MB in a 30 s run.
- **Backdrop decode time**: *done 2026-10-30, hardware run pending.* A
  1920×620 backdrop takes about 0.5 s to decode; the neighbors' backdrops
  are decoded ahead once the selected one is answered. A scaled cache on the
  memory card would also cover a jump of more than one step; not done.
- **A text size setting** (the user, 2026-10-27): *done 2026-10-30,
  hardware run pending.* Five steps (the user, 2026-10-30), Smaller to
  Larger, in `text-size.json`.
