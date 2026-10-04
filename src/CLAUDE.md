# Pocket Shelf: working notes

Notes for picking this project up again. They record what was learned the hard
way and is not obvious from the code. Update them when something here stops
being true.

## What this is

Pocket Shelf (`dev.fthdgn.pocket-shelf`, Vita title id `PBCF7609D`) is a PS Vita
launcher for installed titles. It lives in this `src/` folder of a fork of the
PocketJS framework and builds from the root `pocket.json`.

- `origin` is `fthdgn-gh/pocket-shelf`. `upstream` is `pocket-stack/pocketjs`.
- `apps/*` are upstream demos. `apps/hero` is imported by six device demos and
  `apps/launcher` is upstream's own "Pocket Launcher". Do not repurpose either.
- The Vita host support is in `hosts/vita/src/` behind four cargo features:
  `installed-apps`, `data-fs`, `http`, `status`.

## How we work

- Work on `main`. Commit and push only when asked. Commit messages: one short
  Conventional Commits line (`feat(shelf): ...`), no long body.
- After a change, build and open Vita3K so the user can look. The user also
  installs `dist/vita/pocket-shelf.vpk` on a real Vita; several features have
  only ever been confirmed there.
- **Build release (`bun run shelf:build:release`) for the VPK the user
  installs.** The system reads the whole executable from the memory card at
  every launch. The debug executable is 105 MB; the release one is 2.4 MB
  (`tools/vita.ts` compresses it with `vita-make-fself -c`). The user
  measured about seven seconds to start with the debug build.
- State plainly what was verified in the preview, in Vita3K, and on hardware.

## Commands

| Command | Does |
|---|---|
| `bun run shelf:build` | Debug VPK to `dist/vita/pocket-shelf.vpk` |
| `bun run shelf:build:release` | Release VPK (2.4 MB; the debug one is 25 MB and unpacks to 105 MB) |
| `bun run shelf:run` | Build, then open in Vita3K |
| `bun run shelf:check` | Manifest check for the Vita target, then `tsc` |
| `bun run shelf:test` | Unit tests under `src/tests/` |
| `bun run shelf:preview [words]` | Render the app headless to PNG (see below) |
| `bun run shelf:icons` / `shelf:art` | Regenerate `src/icons/*.svg` / LiveArea PNGs |

The Rust tests in `hosts/vita/src/accent.rs` run natively:
`rustc --edition 2021 --test hosts/vita/src/accent.rs -o /tmp/t && /tmp/t`.

## Seeing the app

**Preview (`src/tools/preview.ts`).** Renders the real bundle in the wasm
simulator and writes frames to `.pocket-build/validation/shelf-preview/`
(ignored by git). It fakes the Vita host: a title table (including a
125-title category), icons, backdrops, the network and SteamGridDB, and it
answers "still decoding" at first like the real host. Shots are button
scripts; adding a menu or editor row shifts the `DOWN` counts in existing
shots. Its launch op does not end the app, so a shot can start several titles. It
needs the `wasm32-unknown-unknown` Rust target.

**Vita3K.** Its window cannot be seen or screenshotted from here, and it
cannot be driven. Evidence comes from its log and from files under
`~/Library/Application Support/Vita3K/Vita3K/fs/ux0/data/PocketShelf/`.

- Launch in the background with output to a log file. `pkill` does not stop
  it; use `kill -9` on instances started here. Never kill one the user opened.
- It runs the real firmware `libssl`/`libhttp` modules, so TLS results there
  match hardware.
- `sceAppMgrLaunchAppByUri` is unimplemented: launching a title only works on
  hardware. If the stub reports success, pressing launch there ends the app.
- `Unhandled EXC_BAD_ACCESS` in its log is not fatal by itself. It appears when
  a freed texture's memory is reused for a new upload. A real crash is followed
  by `Game closed`.
- To test something that needs buttons, add a temporary self-test in `app.tsx`
  that writes its results to a file in the data folder, then remove it.

## Engine constraints that shaped the UI

- **Class strings are compiled at build time.** A class literal must be a full
  literal; no interpolation. One unknown token turns the whole literal into
  plain text with no error. Colors therefore go through `style` props
  (`bgColor`, `textColor`, `gradFrom`, ...), which is why themes are palettes.
- **Textures are rectangles.** `rounded-*` does not clip an image, so tiles are
  square. Textures cannot be tinted, so the button icons carry fixed colors.
- **Texture sides are powers of two, at most 512.** An icon narrower than its
  texture sits at the left and the UI clips it.
- **The SVG baker** (`framework/compiler/bake-svg.ts`) reads filled `rect`,
  `circle` and `path` only: no strokes, text, arcs or group transforms. Letters
  in icons are outlines generated from Space Grotesk Bold.
- **Fractional scale misaligns frames.** A selected tile scaled to 110% put its
  frame on fractions of a pixel. The grid's selected tile is drawn at 100% and
  the others at 90%. The frame is four bars, so nothing shows through a
  see-through icon.
- **Monospace** is baked at 12, 14 and 16 px in one weight only.
- **Do not make tiles focusable.** The framework fires `onPress` of the focused
  node on CIRCLE regardless of the confirm setting, which caused a double
  launch. The launcher owns selection and reads buttons itself (`input.ts`).
  Touch is not handled.
- **One `onButtonPress` handler per button when a press changes the panel.**
  Handlers run in the order they were bound, and each checks its `active`
  option when its turn comes. A press that opens or closes a panel then also
  reaches a later handler bound to the new panel: square opened the search
  keyboard and deleted a character with the same press. Square and START
  dispatch on `state.modal()` in one handler each (`input.ts`).
- **A D-pad press wraps around the list; a held direction does not.**
  `moveSelection` takes `wrap`, and `input.ts` passes it for the first press
  only, so holding a direction stops at the end. In the grid, up and down
  keep the column between the first and last rows.
- **Mount only what is on screen.** Each view windows its list
  (`views/window.ts`) and places items by index. With every title mounted, a
  125-title category was 500 to 850 nodes; windowed it is about 100.

## Host facts (`hosts/vita/src/`)

- **Prefer host extras over new spec ops.** Extras are `ui.__name` functions
  outside the numbered contract (`__appArt`, `__appBackdrop`,
  `__appBackdropFree`, `__artAccent`, `__appRescan`, `__startupMarks`,
  `__processMs`, `__netGet`, `__netSave`, `__netState`,
  `__netText`, `__netClose`, `__status`). The one spec op added, `appIcon`, collided with
  upstream at 52 and is now **57**.
- **Pictures decode on worker threads** (`jobs.rs`). Icons, custom art and
  backdrops answer `-2` until ready and the app asks again every two frames.
  Only the texture upload runs on the main thread.
- **Never free a texture that is still drawn.** It crashed the app. The host
  frees a backdrop only when the app says so; `art-files.ts` counts users per
  handle, keeps the three most recent idle ones, and frees others a few frames
  after their last use. Icon and art textures are never freed.
- **System applications are in `vs0:app`**, not `ux0:app` (ids `NPXS` plus
  digits, `CATEGORY` `gda`). `installed.rs` scans both. Their `param.sfo`,
  `icon0.png` and `pic0.png` are plain files. The folder holds about 55
  entries, most of them services and dialogs; `SYSTEM_APPS` in `catalog.ts`
  names the 16 that are listed, and they fill the built-in `system` category.
  Listing, titles and icons are confirmed in Vita3K, whose firmware has the
  same folder.
- **Launching a system application.** `psgm:play?titleid=NPXS…` does not
  work for them on hardware: flags `0x40000` (vita-launcher's value) open the
  LiveArea, where Start has to be pressed; `0x20000` starts the application,
  which then shows error C2-12570-5. `SYSTEM_URIS` in `installed.rs` gives
  each one its own URI (`settings_dlg:`, `wbapp0:`, `music:`, ...), sent once
  with `0xFFFFF`, taken from RetroFlow. Friends, Videos, Content Manager,
  Parental Controls and PS4 Link have no known URI and open their LiveArea
  (`0x40000`). Confirmed on hardware: the listed ones start, with no error.
- **The title list is kept in `titles.tsv` in the data folder.** A scan reads
  one `param.sfo` per title, which the user measured at about ten seconds for
  a hundred titles on hardware. `installed.rs` scans on the first start,
  writes the list, and reads it on later starts. A title installed or removed
  afterwards is not noticed until "Rescan titles" in the SELECT menu
  (`__appRescan`), which also reports how long the scan took. The scan blocks
  the main thread. In Vita3K a second start opens `titles.tsv` and neither
  app folder; the time saved on hardware is not measured yet.
- **Retail games' files are encrypted.** For an icon or backdrop the host
  tries, in order: `ux0:/app/<id>/sce_sys` (plain for homebrew),
  `ur0:appmeta/<id>/` (exists only after the home screen opened that game's
  LiveArea), then `sceAppMgrGameDataMount` on `ux0:app/<id>`. All three are
  confirmed on hardware. `param.sfo` is readable without a mount.
- **A launch ends this process.** `installed::launch` checks the id and
  keeps it; `main.rs` presents that frame, then `installed::finish_launch`
  sends `psgm:play?titleid=<id>`, sends it again after 10 ms and calls
  `sceKernelExitProcess(0)` (the sequence in VitaShell's updater). A process
  still alive when the system takes the request gets the "close this
  application?" prompt, since one game-category title runs at a time.
- **Nothing may run between the launch request and the exit.** Sending the
  first request from `appLaunch`, a frame before the exit, gave the prompt on
  some launches on hardware; with two file writes after it, on every launch.
  The app writes `settings.json` and `recent.json` before `appLaunch`, and
  the host sends both requests in `finish_launch`. A request the system
  refuses there leaves the app running with the "Launching" line, which goes
  away when the selection moves.
  Confirmed on hardware over many titles: no prompt.
- **Staying open next to a game ("system mode") was looked at and not done.**
  ElevenMPV-A does it with `CATEGORY=gdc` in `param.sfo` (ours is the default
  `gd`) and `libvita2d_sys`: `sceGxmInitializeInternal`, drawing into the
  shell's `sceSharedFb`, textures on the `USER_NC` heap, no CDRAM. The host
  uses stock `vita2d` and the game memory budget, so this is a renderer port.
- **The Vita's own SSL cannot reach Cloudflare-fronted sites** that serve only
  an ECDSA certificate; the handshake fails (`0x80435061`). `http.rs` uses
  `rustls` with `rustls-rustcrypto` (an unaudited alpha) and Mozilla's roots,
  with verification on.
- **`segment 1 overlaps` from `vita-elf-create`:** resize `LINK_PAD` in
  `lib.rs` by a few KiB. Roughly one code change in twenty triggers it.
- **LiveArea PNGs must be indexed 8-bit.** `tools/vita-package.ts` enforces it;
  `src/tools/art.ts` quantizes them.
- The title id comes from the app id in `pocket.json`. Changing the id makes a
  new bubble and a new data folder.

## SteamGridDB

- Base `https://www.steamgriddb.com/api/v2`, header `Authorization: Bearer
  <key>`. The user's key is one line in `steamgriddb.txt` in the data folder.
  Never commit or print it.
- `/search/autocomplete/{term}`, `/icons/game/{id}`, `/heroes/game/{id}`.
  The app asks for static PNG only, since the decoders read PNG. There are no
  thumbnails: a candidate is downloaded in full to preview it.
- Downloads land in `art/` and `backdrops/` as `<title id>-<id>.png`: the
  title id, then the picture's SteamGridDB id (`PCSA00069-48213.png`). Candidates not chosen are deleted when the
  drawer closes.
- **The art and backdrop pickers** list the files named after the edited
  title (`fileBelongsTo`: its name, the name it came with, or its id), or the
  whole folder; triangle switches. A title with no files of its own starts on
  the whole folder. The highlighted row's picture shows left of the drawer
  (`Drawer`'s `aside`) once the highlight has rested 8 frames. An icon
  preview stays in memory like every art texture (256 px at most); a backdrop
  preview is released when the highlight moves.
- **A picked file is used only once the host has decoded it.** The pickers
  list `.png` names only, but the host still answers -1 for a file that is
  not a PNG, is damaged, or is over its size limits (8 MB, 1920x1080). The
  preview's result is remembered per file (`decodes` in `state.ts`); such a
  row shows the `cantRead` note and confirm keeps the drawer open. Confirm on
  a row not yet checked loads it at once and applies it if it decodes. In the
  preview, a file named `broken...` stands for one that does not decode.

- **Fetch artwork (SELECT menu)** runs the same lookups for many titles
  (`scrape.ts`): all titles or one category, one request at a time, the first
  game's first icon and first hero. "Missing only" asks for the pictures a
  title has no file for (`missingArt`); "All" replaces both for every
  title, the user's own choices included. "All" leaves out system applications,
  whose names match unrelated games. A busy reply (429) is waited out five
  times; any other failure halts the run, since it would repeat for every
  title. A run costs up to five requests per title and a hero PNG is a few
  megabytes. `http::state` calls `sceKernelPowerTick` while a request runs so
  the console does not suspend during a long run. Checked against the
  preview's stand-in server only.

- **Clean up artwork (SELECT menu)** deletes files from `art/` and
  `backdrops/`: "Unused" keeps a file a title's changes name and an icon
  matched to a title by name; "All" deletes every file, the user's own
  included, and takes the file names out of the titles' changes
  (`withoutFiles`). "Delete" is pressed twice. A title that is no longer
  installed still counts as using its files.

## Data folder (`ux0:/data/PocketShelf/`)

`titles.tsv` (the host's list of installed titles; deleting it forces a scan),
`titles.cache.json` (every title's changes in one file, rebuilt from `titles/`),
`bundle.qjsc` (the app's compiled code, rebuilt when the app changes),
`settings.json` (settings, the language among them, and the last selection),
`categories.json`, `recent.json` (title ids, the one started
last first, at most 15), `titles/<title id>.json` (per-title overrides:
category, title, art, backdrop, favorite), `art/`, `backdrops/`,
`steamgriddb.txt`.

## Smart categories

"Last Played" (`smart-recent`) and "Favorites" (`smart-favorites`) are filled
by the launcher (`SMART_CATEGORIES` in `categories.ts`, `titlesOf` in
`state.ts`). A title keeps its own category; the editor's Category row skips
the smart ones. They show no tab while empty, sit before the other tabs until
the user reorders them, and can be hidden or moved in the category manager.
`launchSelected` writes `recent.json` before the launch request and restores it if the host rejects the launch.
"Reset to defaults" keeps the favorite mark.

## Languages

English, Turkish, German, French and Spanish. `locales/en.ts` defines the
texts and their type; the other files in `locales/` follow it, and `i18n.ts`
lists them. `state.t()` gives the texts of the chosen language; the choice is
the second to last row of the SELECT menu (two presses up from the first row)
and is saved in `settings.json`. English is the default; the system language is not
read.

- **A character draws only if the font was baked with it.** `fonts.json`
  bakes Basic Latin, Latin-1 Supplement and Latin Extended-A (322 glyphs per
  size). A language outside those ranges needs its range added there.
- **Text does not wrap by itself.** `Note` breaks its text into lines with
  `wrapText`; `Prompt` keeps each hint whole and moves hints that do not fit
  to the next line. A translated label can therefore be longer than the
  English one, at the cost of a line. The category manager shows 4 rows and
  the menu uses the wide drawer to leave room for that.
- **Do not store a translated text in saved data.** Built-in categories carry
  a `name` key and get their label from the language; notes are translated
  when they are set. SteamGridDB errors are `SgdbProblem` keys, not text.
- **The keyboard has three pages**, stepped with R or the page key: letters,
  symbols, accents. The accents page holds the same thirty letters in every
  language (`ACCENTS` in `keyboard.ts`), with the current language's own
  first (`ACCENTS_FIRST`). `searchKey` matches them by their plain spelling
  (`ı` as `i`, `ß` as `ss`). A new letter there also has to be in the baked
  font ranges.
- `upper` in `i18n.ts` handles the Turkish dotted and dotless i for the drawer
  headings.
- `bun run shelf:preview 60-` renders twelve screens per translated language.

## Status bar

A strip above the category header (`components/status-bar.tsx`): the time on
the left; Wi-Fi, Bluetooth and the battery on the right. Three SELECT menu
rows after "Icon box": "Status bar" (on/off), "Clock" (System, 24-hour,
12-hour) and "Battery percent" (on/off), all saved in `settings.json`. The
app reads `__status` (`hosts/vita/src/status.rs`) on the first frame and then
every 60 frames.

- **`Date` in QuickJS is UTC on the Vita.** Checked in Vita3K: the host's
  `sceRtcGetCurrentClockLocalTime` gave the Mac's local 10:25 while `Date`
  gave 07:25 GMT. The time therefore comes from the host. "System" uses the
  system's 12- or 24-hour setting (`sceAppUtilSystemParamGetInt`).
- **Wi-Fi copies the Vita's own symbol** (the info bar icons in Sony's online
  manual, `manuals.playstation.net/document/imgpsvita/basic_screens_13.png`):
  a dot in the lower left and three quarter rings, lit from the inside out in
  four levels (`wifiLevel`: 25% per ring). It shows only while connected;
  Wi-Fi switched off and "on but not connected" both hide it. The value is `sceNetCtlInetGetState` plus the signal percent. Vita3K stubs
  it as connected at 100%.
- **Icons cannot be tinted**, so the Wi-Fi and Bluetooth symbols exist in two
  inks: light for the dark themes and `-day` files for Daylight
  (`isLightTheme` judges the theme by its background).
- **Sizes:** the symbols are 12 px, drawn in the top left of a 16 px texture
  and clipped; the battery body is 24x12. The number in it is 12 px bold, the
  smallest baked size, so the body cannot get lower without a new font size.
- **The battery is drawn with views**, after the iOS one: a rounded body
  filled from the left, the number over it, a nub on the right. The charge is
  the text color, the accent while charging, red at 15% or less. Its value is
  `scePowerGetBatteryLifePercent`.
- **Bluetooth is only "switched on in Settings"**, from the registry key
  `/CONFIG/BT/bt_enable`. User-mode code has no call for connected devices.
  The icon shows while it is on. The key read works in Vita3K (it answers 0);
  on hardware it is not verified.
- **The list shows five rows under the bar.** The screen is 480x272 and six
  list rows filled it to the pixel; `listRows` in `layout.ts` and `pageSize`
  take the setting. The carousel and grid fit with the bar.
- The preview fakes `__status` as `21 47 1 72 0 80 1`; a shot can give its
  own line (`status`). Shots 80 to 87 cover the bar.

## Search

Square opens the keyboard (`openSearch` in `state.ts`); the field shows how
many titles the text finds. Done puts the results in a "Search" tab
(`smart-search`) in front of the others, in the current view. The tab exists
only while a term is set: the back button on it, or an empty term, closes it
and returns to the title selected before. On that tab the footer shows the
term before the position, in place of the "Menu" hint. Matching and order are in
`search.ts`. `persist` saves a selected result under its own category, since
the tab does not exist after a restart.

## Startup time

Measured by the user on hardware with the release build, 111 titles, in
milliseconds since the process started: `main` 340, `graphics` 582, `pak`
2449, `quickjs` 2550, `eval` 4053, first frame 4088. The stopwatch time from
the bubble is about seven seconds, so about three are the system's own launch.

- **Fonts were 1.9 s of it (`pak`).** Each baked size is an atlas; eleven
  atlases of 322 glyphs came to 36 MB as four-byte textures.
  `register_font_atlas` in `hosts/vita/src/graphics.rs` now uploads them as
  one byte per texel (`U8_R111`, alpha under white). The result is not yet
  measured: `pak` fell from 2449 to 1066 and the first frame from 4088 to
  2695.
- **`eval` is 1.5 s:** about 0.55 s reading the app's files (0.43 s of it the
  per-title files in `titles/`), the rest QuickJS parsing the bundle and
  mounting the first screen. Two changes, not yet measured on hardware:
  `overrides.ts` reads every title's changes from `titles.cache.json` (the
  per-title files stay the source; "Rescan titles" reads them again and
  rebuilds the cache), and the shelf scripts set `POCKET_MINIFY=1`, which
  halves the bundle (376 KB to 182 KB) through a switch in `tools/build.ts`.
- **A second reading** (second launch, release build): `quickjs` 1070, app
  code begins 1708, `eval` 2699. So the engine parses the bundle for about
  0.64 s, and the app's code runs for about 0.96 s: 0.17 s reading files
  (the changes cache cut that from 0.55 s), 0.24 s building the header, view
  and footer, and about 0.5 s not yet attributed. Minifying did not show a
  gain in that reading.
- **Bytecode file.** `Runtime::eval` in `hosts/vita/src/lib.rs` compiles the
  bundle on the first start, writes the bytecode to `bundle.qjsc` in the data
  folder (719 KB) with the source's hash and its own, and reads it on later
  starts so the parser is skipped. A file that does not match is compiled
  over. Measured on hardware: the gap between `quickjs` and the app's code
  beginning fell from about 640 ms to 158 ms.
- **Where it stands (2026-10-03, second launch, release build):** `quickjs`
  1152, app code begins 1310, `eval` 1832, first frame 1847. The first frame
  was at 4088 before this work. Inside the app's code: `mount` 516, of which
  `tree` 284 and the framework's `setup` 20. The stopwatch time from the
  bubble has not been taken again.
- **Diagnostics screen.** SELECT menu, "Diagnostics" (the last row): the startup timing of
  the current launch as a list. "Host" lines say when a phase finished
  (`startup_mark` in `lib.rs`, read through `__startupMarks`); "App" lines
  say how long a step took (`timed` in `diagnostics.ts`). Wrap a new startup
  step in `timed` to see it there.
- A full title scan is 2.8 to 3.5 s, which the title list file avoids.
## Open items

- **Next planned work:** list PSP and PS1 Classics that install on the Vita,
  and PSP and PS1 games that run under Adrenaline. Adrenaline's game bubbles
  (`PSPEMU` + digits) are hidden from the list for this reason; Adrenaline
  itself (`PSPEMUCFW`) is kept. The filter is in `catalog.ts`, not the host.
