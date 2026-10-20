# Pocket Shelf: working notes

Notes for picking this project up again. They record what was learned the hard
way and is not obvious from the code. Update them when something here stops
being true.

## What this is

Pocket Shelf (`dev.fthdgn.pocket-shelf`, Vita title id `POCKTSHLF`) is a PS Vita
launcher for installed titles. It lives in this `src/` folder of a fork of the
PocketJS framework and builds from the root `pocket.json`.

- `origin` is `fthdgn-gh/pocket-shelf`. `upstream` is `pocket-stack/pocketjs`.
- `apps/*` are upstream demos. `apps/hero` is imported by six device demos and
  `apps/launcher` is upstream's own "Pocket Launcher". Do not repurpose either.
- The Vita host support is in `hosts/vita/src/` behind five cargo features:
  `installed-apps`, `data-fs`, `http`, `status`, `self-update`.

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
| `bun run shelf:plugin` | Build the Adrenaline boot plugin into `src/vita/psp/pocketshelf.prx` |

The Rust tests in `hosts/vita/src/accent.rs`, `dds.rs` and `seplugins.rs` run
natively: `rustc --edition 2021 --test hosts/vita/src/accent.rs -o /tmp/t && /tmp/t`.
`retroarch.rs` needs `serde_json`: `sh src/tools/retroarch-test.sh`.

## Seeing the app

**Preview (`src/tools/preview.ts`).** Renders the real bundle in the wasm
simulator and writes frames to `.pocket-build/validation/shelf-preview/`
(ignored by git). It fakes the Vita host: a title table (including a
125-title category), icons, backdrops, the network and SteamGridDB, and it
answers "still decoding" at first like the real host. Shots are button
scripts. `menuTo("clock")` opens the SELECT menu on a named row, read from
`MENU` in `menu-items.ts`, so menu changes do not shift them; adding an editor
row shifts the `DOWN` counts in existing shots. A script that leaves a dialog
opened from a group presses back once per page to reach the shelf (dialog,
group, menu for Library). Its launch op does not end the app, so a shot can start several titles. It
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
- **Vita3K lacks some NEON instructions** the Vita has. `saturating_sub` over
  a `u64` array compiled to `vqsub.u64`; Vita3K logged "Unimplemented
  instruction" and computed wrong values (the bogus frame times in
  `frame_stats` before it was rewritten). Check with
  `arm-vita-eabi-objdump -d <elf> | grep vqsub` when Vita3K shows numbers
  hardware does not.
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
  in icons are outlines generated from Figtree Bold (`bun run shelf:icons`).
- **Fractional scale misaligns frames.** A selected tile scaled to 110% put its
  frame on fractions of a pixel. Every view draws its selected tile at 100%
  and scales the others. The selection is a ring (`SelectedFrame`: a 2 px
  border, radius 3, 3 px clear of the art) with nothing behind the art, so
  a see-through icon shows the screen. A larger radius drew stepped corners.
- **A shadow needs a fill.** The core draws `shadow-lg` only for a node with
  a background, so `TileShadow` fills `#00000001`.
- **`rounded-full` needs a fixed width and height** in the same class literal;
  the Launch pill uses `rounded-[13]` on a 26 px height instead.
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
  only, so holding a direction stops at the end.
- **A held direction speeds up on the shelf** (`HOLD_STAGES`, `holdSteps`,
  `moveSelectionBy` in `navigation.ts`): repeats every 6 frames after 0.4 s,
  moving 1 item, then 5 once held 1.2 s, then 20 once held 3 s; it still
  stops at the end. Panels keep the plain repeat. Added 2026-10-20 for
  RetroArch tabs of thousands of games (one at a time, 3,489 NES games took
  six minutes); on trial, the user decides whether it stays. A letter jump
  while holding, and an A-Z strip, were offered and not taken: START was
  kept free. In the grid, up and down
  keep the column between the first and last rows.
- **Mount only what is on screen.** Each view windows its list
  (`views/window.ts`) and places items by index. With every title mounted, a
  125-title category was 500 to 850 nodes; windowed it is about 100.

## Host facts (`hosts/vita/src/`)

- **Prefer host extras over new spec ops.** Extras are `ui.__name` functions
  outside the numbered contract (`__appArt`, `__appBackdrop`,
  `__appBackdropFree`, `__artAccent`, `__titlesReady`, `__scanStart`,
  `__scanState`, `__startupMarks`, `__processMs`, `__scanReport`,
  `__adrPlugin`, `__adrPluginEnable`, `__bootLog`, `__netGet`, `__netSave`,
  `__netState`, `__netText`, `__netClose`, `__status`). The one spec op added, `appIcon`, collided with
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
  afterwards is not noticed until "Rescan titles" in the SELECT menu, which
  also reports how long the scan took. In Vita3K a second start opens
  `titles.tsv` and neither app folder; the time saved on hardware is not
  measured yet.
- **The scan runs on a worker thread** (`installed::scan_start`, thread
  `pocket-scan`, 256 KiB stack) and saves the list there; `scan_state`
  reports `running <done> <total> <phase>` and, once finished, moves the
  result into the list in memory on the main thread and says `done <titles>
  <ms>`. The total is counted first from the folder listings (no file reads):
  one step per entry in ux0:/app and vs0:/app, one for the database and PSM
  together, one per title-id folder in `PSP/GAME` and per file under `ISO`
  (`pspemu::item_count`). Phases: `apps`, `system`, `psm`, `psp`, `images`.
  The app asks `__titlesReady` first (reads the list file, no scan). With no
  list (first start, or the file deleted), `app.tsx`'s `Root` shows
  `components/scan-page.tsx` instead of the shelf: the name, a progress bar
  and the phase with its count, in the saved language, theme and font; it
  polls once per frame and mounts the shelf when the scan is done. "Rescan
  titles" uses the same scan; its drawer shows the bar while it runs. In
  Vita3K a first start scanned on the thread and wrote `titles.tsv`; the page
  itself is seen in the preview only (shots `72b`, `72c`, `71b`,
  `60-<lang>-first-start`). Not yet seen on hardware.
- **Splash** (`sce_sys/pic0.png`, 960x544, indexed): the system shows it
  while the app loads. Drawn by `src/tools/art.ts` in the Midnight theme's
  colors, with the mark and the name where the scan page puts them
  (`SPLASH_MARK_TOP`, `SPLASH_TITLE_Y`, measured from preview shot `72b`),
  so a first start reads as one screen.
- **The mark** (2026-10-20): the Home view in small, a selected tile in the
  white ring and two smaller tiles after it (`MARK` in `art.ts`, `Mark` in
  `components/scan-page.tsx`; keep them in step). It is the bubble icon (the
  home screen crops it to a circle, so the row runs off the edge), the
  splash, the gate image, and faint on the LiveArea background. It replaced
  the old Carousel scene with its cyan frame and lit shelf.
- **The LiveArea PNGs are dithered** (Floyd-Steinberg, carried error capped
  at 12 per channel, `indexedPng` in `art.ts`). Reduced to 256 colors
  without it, the gradients and glows showed bands and blotches; uncapped,
  the error left reddish dots in the splash's dark half.
- **The home screen's database** (`appdb.rs`): `ur0:shell/db/app.db`,
  table `tbl_appinfo_icon` (`titleId`, `title`, `iconPath`), one row per
  bubble. Read once per scan with the system's SQLite module and shared by
  the PSM and PSP/PS1 scans; the icon decoder also asks it for `iconPath`.
- **PSP and PS1 Classics** (`pspemu.rs`): folders in
  `ux0:/pspemu/PSP/GAME/<id>/` with an `EBOOT.PBP` whose `PARAM.SFO` reads.
  The PBP header gives the offsets of its sections (`PARAM.SFO`,
  `ICON0.PNG`, ..., `PIC1.PNG`); `CATEGORY` `ME` is a PS1 game (platform
  `psx`), anything else PSP. The host sends a `platform` (`vita`, `psm`,
  `psp`, `psx`) with each title; `categoryOf` puts PSP and PSX titles in
  `builtin-psp` and `builtin-psx`. The app calls PS1 games PSX everywhere:
  tab, id, platform, `titles.tsv`, the plugin's request (`psx`) and log.
  Confirmed on hardware after the rename (2026-10-04): launches work as
  before.
  - **With a bubble** (a row of the same id in `app.db`, an official package
    for the Vita's own emulator): named by the bubble, started with
    `psgm:play?titleid=<id>`, no Adrenaline. Not yet seen on hardware.
  - **Without a bubble** (games installed for Adrenaline; the user's PSP and
    PS1 games are all this kind): named from the PBP's `TITLE`, icon from its
    `ICON0.PNG` fitted whole into a 128 px square with clear bars, backdrop
    from its `PIC1.PNG`, started through Adrenaline (below). `titles.tsv`
    (header version 4) stores `uri` or `adrenaline` per title.
  - **Disc images** (`iso.rs`): `.iso` and `.cso` in `ux0:/pspemu/ISO` and
    one level of subfolders. Read: ISO 9660 to `PSP_GAME/PARAM.SFO`,
    `ICON0.PNG`, `PIC1.PNG` and `SYSDIR/EBOOT.OLD`; CSO v1 and v2 with raw
    deflate blocks (`miniz_oxide`, already a dependency of `png`). Not read:
    CSO v2 LZ4 blocks, ZSO, DAX, JSO (reported as `format <ext>`). The id is
    the image's `DISC_ID`; an image whose id is already listed is left out
    (`duplicate <id>`). `titles.tsv` (header version 5) keeps the image's
    path under the memory stick (`ISO/Racing/Game.cso`). Tests:
    `src/tools/iso-test.sh` (a throwaway crate that includes `iso.rs`). The
    user's PSP games are all ISO and CSO; the folders in `PSP/GAME` are PS1
    games and update or DLC folders (no `EBOOT.PBP`, `skip=<id> noEboot`).
    Updates (`PSP/GAME/<DISC_ID>/PBOOT.PBP`) are not booted with an image.
  - **Both Adrenalines must work.** The app is meant for other people too:
    TheOfficialFloW's original and isage's fork (8.x). The user runs
    TheOfficialFloW's; isage's is supported from its source only.
  - **The user's setup:** TheOfficialFloW's original Adrenaline (6.61
    firmware on the PSP side), with the `*KERNEL` line for
    `adrenaline_kernel.skprx`, so Adrenaline starts on the first launch. Between games they hold the PS button, go
    back to the LiveArea, close Adrenaline, and open Pocket Shelf, so every
    launch starts Adrenaline from scratch.
  - Not listed: folders whose name is not a nine-character id (most
    homebrew).
  - **Memory stick location** (`adrenaline_config.rs`, native tests): read
    once per process from `ux0:data/PSPEMUCFW/adrenaline.bin`, else
    `ux0:app/PSPEMUCFW/adrenaline.bin` (isage's reads them in that order;
    TheOfficialFloW's has only the second). Second magic `0x334F4E33`: `int`
    fields, location at byte 24 (TheOfficialFloW's, isage's before 8);
    `0x8451860B`: byte fields, location at byte 11 (isage's). Locations 0 to 4
    are `ux0:`, `ur0:`, `imc0:`, `xmc0:`, `uma0:` + `/pspemu`; 5 (isage's
    only) is the root of `uma0:`. Default `ux0:/pspemu`. Every path (scan,
    plugin, lists, request, log) is under it; Diagnostics shows `PSP stick`.
    In Vita3K a settings file naming `ur0:` found the images copied there.
    isage's `ef0:` (PSP Go internal storage) location is not read.
  - Diagnostics shows `PSP folders`, `titles`, `psx`, `psp`, `noBubble`,
    `firstNoBubble`, and `firstOther` (a bubble that is none of the titles).
    Folders left out: `otherNames`/`firstOtherName` (not a title id),
    `skipped` and up to six `skip=<id> <reason>` (`noEboot`, `short`,
    `notPbp <first bytes>`, `sfoSpan <start>-<end>`, `sfoRead`, `notSfo`,
    `listedElsewhere`), then up to ten `file=<name> <size>` of the first one.
    Images: `ISO files`, `ISO titles`, `ISO skipped` and up to six
    `ISO skip=<path> <reason>` (`open`, `notCso`, `csoLz4`, `notIso9660`,
    `noParamSfo`, `noDiscId`, `duplicate <id>`, `format <ext>`, ...).
- **Starting a title through Adrenaline (`src/psp-boot/`).** Pocket Shelf's
  own VSH plugin, `pocketshelf.prx`, shipped in the VPK as `psp/`.
  - **Plugin lists** (`seplugins.rs`, parsed as each Adrenaline does):
    TheOfficialFloW's reads the first 1024 bytes of `vsh.txt` (`<path> 1` is
    on, any other ending off); isage's reads `EPIplugins.txt` when it exists,
    else `plugins.txt` (`<runlevel>, <path>, on|1|true|enabled`, runlevel
    with `vsh`/`xmb` or `all`/`always`, `#` `;` `//` comments, the last line
    for a module decides). Which Adrenaline is installed is not known, so
    `pspemu::plugin_state` reads both: `noAdrenaline`, `missing`, `off` (a
    list has it off) or `on` (both on). `__adrPlugin` reports it.
  - **Consent.** The table marks such titles `adrenaline: true`. Before a
    launch the app reads the state; when it is not `on`, a drawer
    (`components/adrenaline.tsx`) explains and offers "Add plugin" or
    "Turn on" (or only says Adrenaline is missing). Confirm calls
    `__adrPluginEnable` (copies the plugin, adds missing lines, changes a
    line that turns it off; a `vsh.txt` line goes first when it would fall
    past 1024 bytes; other lines and line ends are kept), then launches. The
    host refuses a launch while the state is not `on`. The plugin file is
    copied again at each launch when it differs from the VPK's.
  - **Launch.** Writes `ux0:/pspemu/PocketShelf/boot.txt` (UTC tick in µs;
    `psp`, `psx` or `iso`; `ms0:/PSP/GAME/<id>/EBOOT.PBP` or
    `ms0:/ISO/...`; for `iso` a fourth line, `EBOOT.OLD` when the image has
    one, else `EBOOT.BIN`) and sends `psgm:play?titleid=PSPEMUCFW`. The
    plugin runs at each XMB start: no file, nothing happens; a file is
    deleted first, then dropped when older than 30 s or its path is not one
    folder under `ms0:/PSP/GAME/` (or a `.iso`/`.cso` under `ms0:/ISO/`), else
    booted with `sctrlKernelLoadExecVSHWithApitype`: `0x141` key `game`, PS1
    `0x144` key `pops` (the call isage's own autorun makes); an image the way
    both Adrenalines' vshctrl boots one from the XMB's virtual
    `/MMMMMISO<n>` entries: `SetUmdFile(path)`, boot config 3 (NP9660, the
    same number in both; Inferno is 1 in TheOfficialFloW's and 4 in isage's,
    where 4 is recovery in the other), `disc0:/PSP_GAME/SYSDIR/<boot file>`
    with key `umdemu`, apitype `0x120` in TheOfficialFloW's, and in isage's
    `sctrlSESetDiscType(0x10)` then `0x123`.
  - **ISO driver and BOOT.BIN** follow the user's PSP-side Adrenaline
    settings, read with `sctrlSEGetConfig` (`0x16C3B7EE`, both). TheOfficialFloW's
    (magic `0x31483943`/`0x334F4E33`): `umdmode` `int` at byte 20, Inferno 0,
    March33 1, NP9660 2, booted as config 1, 2, 3; `executebootbin` at byte
    80. isage's `SEConfigEPI` (`0x192EFC3C`/`0x17BEB6AA`): `umd_mode` byte 11,
    Inferno 0, March33 1, ME 2, NP9660 3, booted as 4, 2, 5, 3;
    `execute_boot_bin` byte 26. Unreadable settings: NP9660 (3 in both).
    `EBOOT.OLD` wins over `BOOT.BIN`, as in both vshctrls. The request's
    fifth line says whether the image has a `BOOT.BIN`. The plugin logs
    `driver=<n> [(default)] <file>`; Diagnostics shows it as `Boot driver`.
  - **isage's XMB does more for an ISO** than the plugin: game updates and
    DLC (`PSP/GAME/<DISC_ID>/PBOOT.PBP`, apitype `0x124`), `opnssmp_ver` for
    images with an `OPNSSMP.BIN`, and holding R for `BOOT.BIN`. Not done.
  - **When the plugin boots.** First version: a thread started from
    `module_start` booted at once. On hardware the first one or two launches
    worked, then one stuck on a black screen after Adrenaline opened, and no
    title started again until the Vita was restarted. The plugin is loaded
    while the XMB's modules are still starting; booting from a thread then
    races them.
    Second version: booting from the start-module handler at `vsh_module`.
    On hardware no title started at all. Both Adrenalines call that handler
    from `PrologueModulePatched`, inside the module manager, where a boot
    (which unloads every module) cannot proceed. isage's own autorun boots
    from its `sceKernelStartModule` hook, which a plugin cannot reach.
    Now: with a request present, `module_start` registers a start-module
    handler (`sctrlHENSetStartModuleHandler`, `0x1C90BECB`) that only notes
    `vsh_module` and passes every module on (it stays registered, so a later
    plugin's handler is not dropped), and starts a thread that waits for that
    note (at most 20 s, else it logs `vsh_module not seen; request left`),
    waits 2 s more (`XMB_SETTLE_US`) and boots, with the XMB up as when the
    user picks a game. Without a request nothing is registered.
    **Confirmed on hardware (2026-10-04, TheOfficialFloW's Adrenaline):**
    the user reported ISO and CSO games launching from Pocket Shelf "working
    great" with this version.
    That version showed the XMB for a moment before the game. Now
    (confirmed on hardware 2026-10-04: games boot, the XMB's menu no longer
    shows, only its gray background with the white line for a moment): the
    handler also notes the first module that starts
    after `vsh_module` (the XMB's own code is running and loading its menu,
    not drawn yet) and the thread boots `AFTER_XMB_US` (0.2 s) after it,
    falling back to `XMB_SETTLE_US` (2 s) after `vsh_module` when none comes.
    The log says which: `trigger=after <module>` or `trigger=fallback`,
    shown as `Boot trigger`. If this hangs as the first version did, set the
    thread back to the 2 s wait. Adrenaline's own splash is skipped by its
    "Skip Adrenaline Boot Logo" setting (and "Skip Sony logo" in recovery);
    Pocket Shelf does not change Adrenaline's settings.
  - **Booting straight into a game (no XMB at all) was looked at and not
    done.** RetroFlow does it with Adrenaline Bubble Manager's parts: it
    replaces Adrenaline's own modules in `ux0:app/PSPEMUCFW/sce_module` with
    patched copies (plus `adrbubblebooter.suprx`, `bootconv.suprx`), checks
    them against CRC tables of known Adrenaline builds, asks for a reboot,
    and boots through its own bubble (`RETROLNCR`, `data/boot.bin` with the
    path at `0x40`). The user chose not to modify Adrenaline's files or ship
    patched ones to save the splash and the moment of XMB background.
  - **The plugin is a kernel module** (`0x1000`). It imports two
    `SystemCtrlForKernel` functions by name-hash NID, which both Adrenalines
    export: `sctrlHENFindFunction` (`0x159AF5CC`) and
    `sctrlKernelLoadExecVSHWithApitype` (`0x2D10FB28`). `SetUmdFile`
    (`0xB64186D0`), `sctrlSESetBootConfFileIndex` (`0x5CB025F0`),
    `sctrlSESetDiscType` (`0x31C6160D`, isage's only, which tells them apart)
    and `sceRtcGetCurrentTick` (module `sceRTC_Service`, library `sceRtc`,
    `0x3F7AD767`) are looked up at run time; without the clock the age is not
    checked (`age=?` in the log). Other imports: `IoFileMgrForKernel`,
    `ThreadManForKernel`, `SysclibForKernel`. Links `libgcc` for 64-bit
    division. Pocket Shelf deletes the request when the
    system refuses the launch and when it starts. Each request leaves a line
    in `ux0:/pspemu/PocketShelf/boot.log` (age, path, result); its last six
    lines end the Diagnostics screen (`__bootLog`), as `Boot <outcome>` with
    the age and the file name (an EBOOT by its folder). Preview shot `92b`.
  - **Gaps.** Adrenaline failing before its XMB, then opened from its bubble
    within 30 s, boots the game. TheOfficialFloW's first start after a
    reboot (without the `*KERNEL` line) does not reach the XMB. Adrenaline's
    "XMB plugins" setting is not read.
  - **Verified.** Images: in Vita3K a `hdiutil` ISO and a CSO made from one
    (in a subfolder) are listed by `DISC_ID` with icon and picture, a ZSO is
    reported, a CSO launch writes an `iso` request. Vita3K (no Adrenaline): plugin copied, request written and
    deleted at the next start; `off` read, turned on, `on` read, with the
    other `vsh.txt` line and CRLF kept and `plugins.txt` created. Preview
    shots 95 to 99 and `60-<lang>-adrenaline` show the drawer. On hardware,
    with TheOfficialFloW's Adrenaline: disc image games boot from Pocket
    Shelf (so the request's age passed the 30 s check). isage's Adrenaline,
    a moved memory stick, and the driver setting on hardware are not seen.
  - **Toolchain:** pspdev in `~/pspdev` (release v20261001, macOS arm64). Its
    compiler needs Homebrew `gmp`, `mpfr`, `libmpc` and `zstd`. The built
    `src/vita/psp/pocketshelf.prx` is committed so a VPK build needs no PSP
    toolchain; run `shelf:plugin` after changing `src/psp-boot/`.
- **PlayStation Mobile titles** (`psm.rs`) are folders in `ux0:/psm` named
  `NPNA`, `NPOA`, `NPPA` or `NPQA` and five digits. They have no
  `param.sfo`. The name comes from the home screen's database
  `ur0:shell/db/app.db` (`tbl_appinfo_icon`: `titleId`, `title`), read with
  the system's SQLite module (`SceSqlite`); a folder with no row there has no
  bubble and is left out. **The module needs `sceSqliteConfigMallocMethods`
  first;** without it `sqlite3_open_v2` returns 7 (SQLITE_NOMEM), seen in
  Vita3K. When the database cannot be read, every folder is listed, named
  from `ur0:appmeta/<id>/param.sfo` or by id. Launch is
  `psgm:play?titleid=<id>`, as for a game. **Icons are DDS:** the home
  screen keeps a PSM icon as `ur0:appmeta/<id>/icon0.dds` (the database's
  `iconPath` names it), 8320 bytes on hardware: a 128-byte header and a
  128x128 DXT1 image. `dds.rs` decodes DXT1, DXT3, DXT5 and 32-bit DDS; its
  tests run natively (`rustc --edition 2021 --test hosts/vita/src/dds.rs`).
  Found through the Diagnostics lines: after "Rescan titles" they list the
  first title's `ur0:appmeta` files ("PSM file"). The blocks are in plain
  order (not swizzled). **Confirmed on hardware (four PSM titles):** listed
  in the PS Mobile tab under their bubble names, icons, backdrops
  (`pic0.png` in `ur0:appmeta`), and launch. This follows RetroFlow. The scan's findings (`__scanReport`) show as
  "PSM" lines on the Diagnostics screen after a scan, not after a start that
  read the list file. In Vita3K, with a hand-made `app.db` and two folders:
  one listed under its database name, one left out, icon found. The list
  file's header is version 2 since this; a version 1 file is scanned over.
- **RetroArch games** (`retroarch.rs`, tests: `sh src/tools/retroarch-test.sh`,
  a throwaway crate with `serde_json`). The scan's last phase (`retro`, one
  step per playlist file) lists every entry of RetroArch's playlists as a
  title: platform `retro`, id `ra` plus seven lowercase base-36 digits of the
  FNV-1a hash of the content path (lowercase, so it cannot be a system id;
  the same file keeps its id and the user's changes to it).
  - **Where:** RetroArch is `RETROVITA`, installed when
    `ux0:/app/RETROVITA/eboot.bin` exists. Data in `ux0:/data/retroarch`;
    `retroarch.cfg`'s `playlist_directory` and `thumbnails_directory` are
    read (`default` or unset: `playlists/`, `thumbnails/`; `app0:` and `:`
    mean RetroArch's app folder). History and favorites (`content_*.lpl`)
    are not read.
  - **Format:** JSON playlists (`default_core_path`, `items[]` with `path`,
    `label`, `core_path`, `db_name`), and the six-line format from before
    RetroArch 1.7.6 (path, label, core path, core name, CRC, database).
    **With "Compress Playlists" on (the user's setup) each file is RZIP**
    (`decode_rzip`, from RetroArch's `rzip_stream.c`): `#RZIPv`, version,
    `#`, chunk size (u32 LE), text size (u64 LE), then chunks of a u32 LE
    length and one whole zlib stream each, inflated with `miniz_oxide`.
    Version 2 (zstd, newer RetroArch) is reported as `rzipZstd`, not read.
    The first build read plain text only, and the user's Vita reported all
    nine playlists unreadable. A playlist that cannot be read shows as
    `RetroArch unreadable=<n>,RetroArch firstUnreadable=<file> <reason>`.
    The user's playlists (not in git) are in `.samples/retroarch-playlists/`;
    `retroarch-test.sh` reads them when the folder is there.
  - **Console** is the entry's `db_name` less `.lpl`, else the playlist's
    file name (RetroArch does the same for thumbnails). `consoles.ts` gives
    each console a tab after the built-ins (`retro-snes`, label "SNES"); every
    MAME and FBNeo database shares `retro-arcade`; `Sony - PlayStation` and
    `Sony - PlayStation Portable` go to the PSX and PSP tabs; an unknown
    console gets `retro-<slug>` and the part of its name after the maker.
    These ids are saved (tab order, a title moved into one) and must not
    change. A custom category with the same id keeps it.
  - **Core:** the entry's `core_path`, else the playlist's
    `default_core_path` (an entry's `DETECT` means none), else the first
    installed core by file name whose `info/<core>.info` `database` list names
    the console. Only cores in RetroArch's app folder that exist count:
    `app0:/x_libretro.self` or the same file by its `ux0:` path.
  - **Launch:** `psgm:play?titleid=RETROVITA&param=app0:/<core>&param2=<path>`,
    sent twice like a game's. RetroArch's `eboot.bin` is its "salamander"
    stub (`dist-scripts/dist-cores.sh`), which reads both parameters
    (`frontend_psp_exec` in `frontend/drivers/platform_psp.c`), cuts each at
    the next `&`, and runs the core with the content. Without `param` it opens
    the last core and no game. So an entry is left out when its path holds `&`
    (`pathChars`), when no core is found (`noCore`), when the URI passes 1000
    bytes (`long`; the system's limit is not known), or when an earlier
    playlist listed the same file (`duplicate`). `finish_launch` first closes
    a running RetroArch (`sceAppMgrGetIdByName`, `sceAppMgrDestroyAppByName`,
    up to 1 s), since a running app comes back as it was and does not read
    the new parameters. One game-category app runs at a time, so RetroArch is
    likely closed already when Pocket Shelf runs. **Not yet seen on
    hardware.** In Vita3K (2026-10-20, a stand-in install: empty `eboot.bin`
    and core files, two playlists, one box art): the first start scanned and
    listed three games with the playlist's default core and the info-file
    core, left out a path with `&`, and read the box art; the second start
    read `titles.tsv` and no playlist. Launching cannot be tried there.
  - **Pictures:** the icon is the game's box art from RetroArch's
    thumbnails (`<thumbnails>/<console>/Named_Boxarts/<label>.png`, else
    `Named_Titles`), fitted whole into 128 px like a PSP icon
    (`pspemu::fit_png`, at most 4 megapixels); the backdrop is
    `Named_Snaps`, else `Named_Titles`. RetroArch's file name is the label
    with `&*/:`"<>?\|` replaced by `_`; the label up to its first ` (` is
    tried too. A MAME game's thumbnails are in `MAME`.
  - Diagnostics lines: `RetroArch playlists`, `titles`, `skipped`, up to six
    `RetroArch skip=<title> <reason>`, `unreadable`, or
    `RetroArch=notInstalled`. The "Detailed" views show "RetroArch" in place
    of the id. `titles.tsv` is header version 6 since this (columns: id,
    platform, launch `uri`/`adrenaline`/`retroarch`, path, console, core,
    title); a version 5 file is scanned over once.
  - **Scale (the user's playlists, Vita3K, 2026-10-20):** 5,811 entries in
    nine playlists, 102 left out for `&`, 5,709 listed. The scan took about
    0.2 s; `titles.tsv` is 958 KB. A start from the list: `catalog` 123 ms,
    building the titles 80 ms, `state` 300 ms, first frame at 0.9 s, then 60
    frames a second. Hardware not measured. Most entries are `DETECT` and
    take the playlist's default core; a game the user started once carries
    its own core. `Neo Geo.lpl` is a playlist the user named; its console
    "Neo Geo" is mapped to `retro-neogeo`.
  - **Stutter with thousands of titles (user, hardware, 2026-10-20):**
    scrolling a big tab and switching tabs stuttered; none of the user's
    games has box art. Changed, not yet measured on hardware: the host
    finds a title by id through an index (`installed::find`, rebuilt with
    the list) instead of walking every title for each icon and backdrop
    request, which the app repeats every few frames; a RetroArch game whose
    console has no thumbnail folder gets no picture at once (each folder is
    checked once, `folder_exists`), where it used to start a job that
    checked four missing files; the app groups titles by category once
    (`byCategory`), looks base titles up in a map (`catalogById`; the Fetch
    artwork list was quadratic), and does not copy the icon and box tables
    for a title with no icon.
  - **Frame timing** is the top of the Diagnostics screen: frames, frames
    over 20 ms, and the longest frame, since the screen was last opened
    (`frameTick` in `state.frame`), then the host's split of the same frames
    (`frame_stats.rs`: JS, Layout, Draw, Present, average and longest) and
    the QuickJS heap (`JS heap`, `JS objects`). Open it, close it, do the
    thing that stutters, open it again.
  - **First hardware reading** (user, after the changes above, scrolling
    NES and switching tabs): 2448 frames, 1672 over 20 ms, longest 729 ms;
    JS 28.2/717.2 ms, Layout 0.7/9.7, Draw 3.0/13.7, Present 3.4/12.2; heap
    16053 KB, 83965 objects. JavaScript is the cost; the GPU is not.
  - **The Dynamic theme was most of it.** Profiled in Vita3K with a
    temporary per-phase timer and a stack trace on every Solid signal
    write (both removed): the blend wrote `dynamicColor` on most frames,
    and each write rebuilds `theme()`, which every node with a theme color
    reads. Per-frame JS while scrolling: Dynamic 3.8 ms, Midnight 1.0 ms.
    Now the blend waits until the selection has rested 10 frames and steps
    every 2 frames, two fifths of the way: Dynamic 1.0 ms. Tab switches:
    1.9 ms before, 1.2 ms after.
  - **Long frames (hundreds of ms) are likely QuickJS's garbage
    collection**, which walks every object; it runs when the heap passes
    1.5 times its size after the last collection (`js_trigger_gc`). A title
    with no changes and no art file is no longer copied in `allGames`.
  - **Second hardware reading** (after the Dynamic change): JS 4.2/632.3
    ms, longest frame 645 ms. The average fits the frame; the pauses stay.
  - **Garbage collection is paced by the host** (`Runtime::collect_when_idle`
    in `lib.rs`, called after each present): a collection runs once the
    buttons have rested 60 frames after use, or after 30 s of use without
    such a rest, and QuickJS's own trigger is moved 24 MB past the live size
    it measures (`JS_SetGCThreshold`), so it only fires for a heap growing
    faster than expected. The first trigger is 48 MB, set when the runtime
    is made, so startup no longer collects. The C heap is 128 MB (newlib's
    `_get_vita_heap_size`, read in Vita3K). Diagnostics shows
    `Idle GC count/max`. In Vita3K a faked press ran one idle collection:
    11 ms, 7.3 MB and 20,860 objects live after it.
  - **Third hardware reading** (idle collection): JS 8.7/226.1 ms, longest
    frame 237 ms, idle GC 2/234.5 ms. The longest frame is the idle
    collection. The JS maximum was the frame that opened Diagnostics: the
    heap count walks every object. `frame_stats` now leaves the frame that
    calls `take` out, as `takeFrameEntries` already did on the app side.
  - **Vita3K restarts the app by itself** about three seconds into the first
    run after `-r <id>` or an install: "Game closed" is followed straight
    away by `Idle -> Launching`, with no error before it. That is not a
    crash.
  - **Next:** custom categories that point at a ROM folder, with a core
    chosen per category, launched the same way.
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
- **LiveArea text, not text in pictures.** The system draws its "Start" label
  over the bottom of the gate image (`startup.png`), which hid the name drawn
  there. The pictures hold no text; `src/tools/art.ts` also writes
  `src/vita/sce_sys/livearea/contents/template.xml` (style `psmobile`), the
  title and version from `pocket.json` and the author from `src/about.json`,
  laid out as VitaShell's LiveArea: the title in `frame2`
  (size 50, bold), "by <author>" in `frame3` (size 22; left out without an
  author), "v<major.minor>" in `frame4` (size 18), all white with a shadow.
  Run `bun run shelf:art` after changing the title, author or version.
  Confirmed on hardware (2026-10-04) with the name and version: they show,
  clear of the gate.
- **The author is in `src/about.json`**, not `pocket.json`: the manifest
  schema (framework) has no such field and rejects unknown keys. Adding one
  to the framework was tried and undone; app-only values live in `src/`.
- **The app's version is `pocket.json`'s `version`** (0.1.0 since
  2026-10-04; the 0.13.0 before it was the framework's release number).
  `tools/vita.ts` writes it to `param.sfo` as `APP_VER` (`MM.mm`, "00.01"),
  which the system shows as the app's version.
- **The title id is `POCKTSHLF`**, set by the `--title=POCKTSHLF` backend
  argument in the `shelf:build` script (upstream's flag since PocketJS
  0.15.0; `tools/vita.ts` checks it: a letter, then eight uppercase letters
  or digits). Until the 0.15.0 merge it was our own `POCKET_VITA_TITLE_ID`
  variable. Without it the id is derived from the app id in `pocket.json`; that
  was `PBCF7609D` until 2026-10-05. Changing the id installs a new bubble and
  leaves the old one; the data folder (`ux0:/data/PocketShelf/`) is fixed in
  `datafs.rs` and stays the same.
- **The title id must not start with `PC`.** `PCKTSHELF` was tried first: on
  the user's Vita the VitaCheat plugin's thread crashed in the app (prefetch
  abort, PC 0x0; the app's own thread was fine), and the app opened with
  VitaCheat turned off. `POCKTSHLF` runs with VitaCheat on (hardware,
  2026-10-05). Retail ids start with `PCS`; VitaCheat likely treats `PC` ids
  as retail games. Read a `psp2dmp` with xyzz's `vita-parse-core` (Python 2;
  under Python 3 it needs `pyelftools==0.29` and a bytes fix in `util.py`)
  against `hosts/vita/target/armv7-sony-vita-newlibeabihf/release/pocketjs-vita.elf`.

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

- **Without an API key**, both SteamGridDB drawers (the editor's and Fetch
  artwork's `key` step) first explain the key and where its file goes, and
  open the keyboard on confirm. Fetch artwork used to open the keyboard at
  once, and the user could not tell what it was for.
- **The keyboard names what it types** in a heading above the field
  (`keyboardFor` in the locales, by `KeyboardTarget` kind: title, new
  category, category name, library search, SteamGridDB search, API key).
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

## Updates

SELECT ▸ Updates: **Channel** (Stable, Beta, Nightly, Off; setting `updates`)
and **Check for updates**. The install works on hardware.

- **Build identity:** `src/tools/build-info.ts` writes `src/build-info.json`
  (ignored by git) before every shelf build, check, test and preview: the
  version from a `shelf-v<version>` tag (CI's `GITHUB_REF_NAME`), else
  `pocket.json`'s; the channel (`stable`/`beta` from the tag, `nightly` for
  other CI builds, `local` here); the commit. A fresh install checks its own
  channel; a local build defaults to Off.
- **Check:** `src/updates.ts` reads `api.github.com/repos/fthdgn-gh/pocket-shelf/releases?per_page=10`
  (no key; 60 requests an hour per address). Stable and beta compare versions
  (`0.1.0-beta.1` < `0.1.0`); nightly compares the `shelf-nightly` release's
  `target_commitish` (the workflow sets it to the commit) with the build's.
  A build ahead of the channel is not offered the older release.
- **At start:** the updater's report if it ran, else a check at most once a
  day (`update.json`: `checkedAt`, `later`) on a background request; what it
  finds opens the drawer once no other drawer is open. "Later" stores the
  release so the check at start does not offer it again.
- **Download:** `http.rs` accepts one more destination, `update/pocket-shelf.vpk`
  (64 MB limit, ZIP signature). GitHub answers the asset URL with one redirect.
- **Unpack:** `update.rs` unpacks it into `update/pkg/` on a thread (stored and
  deflate entries, CRC-32 checked, no `..` paths) and requires its `param.sfo`
  to name this title id. Checked in Vita3K (2026-10-05) with VitaShell's VPK:
  the download matched a direct download and the unpack ended with
  `title id VITASHELL`, as it should.
- **Install:** the installer (`scePromoterUtilityPromotePkgWithRif`) replaces
  `ux0:app/POCKTSHLF`, which the running app is mounted from, so a separate
  app installs it: **Pocket Shelf Updater** (`POCKTUPDR`, `src/updater/`,
  **GPL-3.0** because it uses VitaShell's `makeHeadBin`, `fpkg_hmac` and
  `head.bin`; the rest of the app stays MIT). Its build is committed in
  `src/vita/updater/` (VPK path `updater/`, with its own `head.bin` made by
  `src/updater/make-package.ts`, checked byte for byte against VitaShell's C);
  rebuild with `bun run shelf:updater`. Pocket Shelf copies it to
  `update/helper/`, installs it, starts it (`installed::launch_unlisted`) and
  exits; the updater installs `update/pkg/` (its screen shows the version
  Pocket Shelf wrote to `update/version.txt`; `APP_VER` holds only `MM.mm`),
  writes `update/result.txt` (`ok` or `error ...`) and
  starts Pocket Shelf, which shows the result, deletes `update/` and removes
  the updater (`scePromoterUtilityDeletePkg`).
- **Confirmed on hardware (2026-10-06):** a release build of `main` made as
  `GITHUB_REF_NAME=shelf-v0.1.0-beta.0 bun run shelf:build:release` was
  offered `shelf-v0.1.0-beta.1` at start, downloaded, installed it through
  the updater and restarted. Vita3K has no promoter, so the install runs on
  hardware only. The first try showed nothing at start; it did after
  `update.json` (the last check's time) was deleted.
- **The updater stopped responding on hardware (2026-10-07)** while
  installing beta.1 to beta.2: the install completed (after a restart of the
  Vita, Pocket Shelf was the new version, showed the result and deleted
  `update/`), but the updater froze on its last screen and did not start
  Pocket Shelf. So it stopped after `report("ok")`: in the two seconds of
  drawing, `vita2d_fini`, or the launch requests and exit. The same code
  restarted Pocket Shelf in the 2026-10-06 test. Not yet known which step.

## Keeping up with PocketJS

`upstream` is `pocket-stack/pocketjs`. Merge `upstream/main` on a branch
(`sync/pocketjs-<version>`), then check: `shelf:check`, `shelf:test`,
`retroarch-test.sh`, a release build (its `param.sfo` must still hold
`POCKTSHLF`, "Pocket Shelf" and `APP_VER`), every preview shot against the
renders from before the merge, and a start in Vita3K.

- **Conflicts to expect:** `pocket.json` (keep ours: the root manifest is
  Pocket Shelf, upstream's is the Hero demo), `README.md` (ours is Pocket
  Shelf's), `tools/vita.ts` (our `-c` SELF compression and `APP_VER`).
- **0.15.0 (merged 2026-10-20, 47 commits):** four conflicts, those above
  and a test written two ways. All 189 preview shots matched pixel for
  pixel except the Diagnostics timings. `tests/launcher-sim.test.ts` fails
  11 tests on this machine both merged and on a clean upstream checkout.

## Data folder (`ux0:/data/PocketShelf/`)

`titles.tsv` (the host's list of installed titles; deleting it forces a scan),
`titles.cache.json` (every title's changes in one file, rebuilt from `titles/`),
`bundle.qjsc` (the app's compiled code, rebuilt when the app changes),
`settings.json` (settings, the language among them, and the last selection),
`categories.json`, `recent.json` (title ids, the one started
last first, at most 15), `titles/<title id>.json` (per-title overrides:
category, title, art, backdrop, favorite), `art/`, `backdrops/`,
`steamgriddb.txt`, `update.json` (last update check, a release put off),
`update/` (the downloaded VPK, `pkg/` unpacked from it, `helper/` while the
updater is installed, `result.txt` from the updater).

## Cascade view

The fourth view (`views/cascade.tsx`), after the Xbox 360 dashboard of 2008
(the "NXE"): the selected tile at the left at full size (`CASCADE` in
`layout.ts`: 144, 128, 112 px), the next five each 80% of the one before,
from the third on a quarter behind the tile in front, and darkened by a
theme-colored shade (not opacity, which would show the tile behind through
it). `cascadeSlot` gives each offset's place. **Transitions follow class
changes only, not `style` props**, so the tiles are tweened with `animate`
(240 ms) when the offset changes; a tile mounts in its place. One tile before
the selection stays mounted and slides off the left. Preview shots `c1` to
`c6`. Confirmed on hardware (2026-10-07, beta.2), screenshot
`src/screenshots/cascade.png`. **The shade covers the whole square**, so on an
icon with clear bars (PSP icons fitted into the square) it shows as a dark box
above and below the picture.

## Category order

The default tab order (`orderCategories` in `categories.ts`): the smart
tabs, the built-ins (`BUILTIN_CATEGORIES`: Games, Homebrew, PS Mobile, PSP,
PSX), the RetroArch consoles, the user's own categories, and System last. A
category not yet in the saved order goes before System while System is the
last saved one, so System stays last when a console or a category is added;
once the user moves System elsewhere, new ones go at the end. Until
2026-10-20 the consoles came after System. Preview shots reach a tab with
`tabTo(id)` and `tabToConsole(n)`, read from that order.

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
the second to last row of the SELECT menu's main page (two presses up from the first row)
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

## SELECT menu

`MENU` in `menu-items.ts` lists the pages. The main page: Appearance ▸
(theme, font, view, details, backdrop, icon box, Status bar ▸), Library ▸
(categories, fetch artwork, clean up artwork, rescan titles), Confirm,
Language, Diagnostics. Right or confirm opens a group; back returns to the
page holding it, on its row (`parentPage`), and closes the menu from the main
page. SELECT closes it from any page; it opens again on the main page. `menuChange` in `state.ts` acts on the row's name,
and `components/menu.tsx` draws each row by name.

## Look (2026-10-20)

- **Top bar** (`components/header.tsx`, 30 high): L, the category names, R,
  then the status items in `STATUS_W` (128; 112 let Wi-Fi run into the last
  name). Names are measured in bold so a name keeps its width when it
  becomes current; the current one is bold over a 14 px accent bar; no
  counts. The names slide so the current one is centered between L and R
  at every press. A first version slid only when the current name left the
  view; on hardware the user saw the names "jumping back and forth". A name
  an end of the bar cuts shows in part, faint (`CUT_OPACITY`, 0.5, times the
  part in view), tweened with `animate` as the names slide, so the next
  categories can be glimpsed. The engine has no blur; a gradient over the
  ends in the theme's color was tried and showed as blocks, since the bar
  sits over the title's backdrop and the theme's wash, not over a flat color.
- **Five views.** Home is the default since 2026-10-21 (`DEFAULT_VIEW` in
  `navigation.ts`; a view saved in `settings.json` is kept). Carousel
  (`views/carousel.tsx`) is the shelf as before and was the default until
  then. Home is also first in `VIEW_MODES`, the View row's order: Home,
  Carousel, Grid, List, Cascade. Preview shots pick a view with
  `viewTo(mode)`, counted from the default, and shots with no view show Home. **Home** (`views/home.tsx`, id `home`), after the PS5 home
  screen, came after it: the selected tile at `HOME_LEFT` at full size with
  ring and shadow, the next ones at `small` after it, aligned at the top,
  at 80% opacity (`homeSlot`, `HOME` in `layout.ts`), tweened with
  `animate` as in the cascade. Under it the title in the `display` role (24
  px bold), a dim line (platform, category, and the id at "Detailed"), and
  a Launch pill in the text color. The footer leaves out its Launch hint in
  Home, except at "Basic", which shows tiles only.
- **Drawers and the keyboard are floating cards**: 8 px from the edges,
  radius 12, `shadow-lg`, the heading dim, no accent edge. The active row is
  the text color at 11% (`1c`), as is the list's selected row.
- The list keeps six rows with the status bar on, since the status items no
  longer take a row of their own.
- **Font: Figtree** (`src/fonts`, OFL), baked into the sans slots, and used
  by `shelf:icons` and `shelf:art`. The Font setting (Space Grotesk or
  Hack) was removed: the engine bakes one full family; its monospace slots
  hold one weight at 12, 14 and 16 px, so a second font there had no bold
  or large sizes. Compared in the preview: Inter, Figtree, Manrope, Plus
  Jakarta Sans. Manrope (static instances cut from the variable font) and
  Plus Jakarta Sans lost their word spaces in the baked atlas ("NeonDrift");
  not looked into.

## Status bar

The status items at the right of the top bar (`StatusItems` in
`components/status-bar.tsx`): Wi-Fi, Bluetooth, the battery and the time. The SELECT menu's
"Status bar" group, inside Appearance, holds "Show" (on/off), "Clock" (System, 24-hour,
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
- **The "Detailed" grid's tiles are 64 px** instead of 72 (`GRID` in
  `layout.ts`): under the old separate status strip its id line pushed the
  footer 13 px off screen. With the status items in the top bar there is
  room again; the size was left as it is.
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
- **Diagnostics screen.** SELECT menu, "Diagnostics" (the last row of the main page): the startup timing of
  the current launch as a list. "Host" lines say when a phase finished
  (`startup_mark` in `lib.rs`, read through `__startupMarks`); "App" lines
  say how long a step took (`timed` in `diagnostics.ts`). Wrap a new startup
  step in `timed` to see it there.
- A full title scan is 2.8 to 3.5 s, which the title list file avoids.
## Open items

- **Next:** see an image boot on hardware; then possibly the user's own ISO
  driver setting, game updates (`PBOOT.PBP`) for images, ZSO. Adrenaline's
  game bubbles (`PSPEMU` + digits) are hidden from the list; Adrenaline
  itself (`PSPEMUCFW`) is kept. The filter is in `catalog.ts`, not the host.
