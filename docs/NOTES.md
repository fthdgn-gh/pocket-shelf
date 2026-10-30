# Pocket Shelf: working notes

Notes for picking this project up again: what was learned the hard way about
the Vita, its firmware, Adrenaline and RetroArch, and what has been verified
where. Update them when something here stops being true. The crates and the
drawing model are in [`ARCHITECTURE.md`](ARCHITECTURE.md); the port from
PocketJS is in [`PLAN.md`](PLAN.md).

## What this is

Pocket Shelf (Vita title id `POCKTSHLF`) is a PS Vita launcher for installed
titles and RetroArch games, written in Rust (`core/`, `vita/`). Up to 0.1 it
was a TypeScript app on PocketJS in a fork of that framework; 0.2.0 replaced
it, kept its saved data, and the framework was removed from the repository.

- `origin` is `fthdgn-gh/pocket-shelf`.
- The data folder is `ux0:/data/PocketShelf/`, the log `log.txt` in it
  (256 KB at most, then `log.old.txt`). The side-by-side build used
  `ux0:/data/PocketShelfNext/`; nothing reads it any more.

## How we work

- Commit and push only when asked. Conventional Commits (`feat(shelf): ...`).
- After a change: `make test`, `make shots` and look at the renders, then
  `make build` for the user to install. The user installs
  `dist/vita/pocket-shelf.vpk` on a real Vita and sends the log; several
  features have only ever been confirmed there.
- State plainly what was verified on the desktop, in Vita3K, and on hardware.

## Seeing the app

**Desktop shots** (`make shots`, `SHOTS="words"` for some, `desktop/src/main.rs`): the core
draws into a software renderer; a fake host answers with sample titles,
generated icons and backdrops, a fake SteamGridDB, libretro server, GitHub
and updater. Shots are button scripts; renders go to
`.build/validation/desktop/` (ignored). The README's `screenshots/`
are copies of seven of them, captioned as renders with sample titles.

**Vita3K.** Its window cannot be seen or screenshotted from here, and it
cannot be driven. Evidence comes from the log and from files under
`~/Library/Application Support/Vita3K/Vita3K/fs/ux0/data/`.

- Launch in the background with output to a log file. `pkill` does not stop
  it; use `kill -9` on instances started here. Never kill one the user opened.
- It runs the real firmware `libssl`/`libhttp` modules, so TLS results there
  match hardware.
- `sceAppMgrLaunchAppByUri` is unimplemented: launching only works on
  hardware. It has no package installer either, so an update installs on
  hardware only.
- **Vita3K lacks some NEON instructions** the Vita has. `saturating_sub` over
  a `u64` array compiled to `vqsub.u64`; Vita3K logged "Unimplemented
  instruction" and computed wrong values. Check with
  `arm-vita-eabi-objdump -d <elf> | grep vqsub` when Vita3K shows numbers
  hardware does not.
- It restarts the app by itself about three seconds into the first run after
  `-r <id>` or an install, and reports a TRIANGLE press about 10 s into every
  run. Neither is a crash.

## Building

- **Install the release build on hardware.** The system reads the whole
  executable from the memory card at every launch; `vita/build.sh` signs it
  compressed (`vita-make-fself -c`).
- **The SELF must not be "safe"** (cargo-vita's `-s`): a safe app cannot open
  `ux0:app`, and the first hardware run listed 0 titles.
- **The title id must not start with `PC`.** `PCKTSHELF` was tried first: on
  the user's Vita the VitaCheat plugin's thread crashed in the app (prefetch
  abort, PC 0x0). Retail ids start with `PCS`; VitaCheat likely treats `PC`
  ids as retail games.
- **`vita-elf-create: ... segment 1 overlaps`**: the SCE data it appends to
  segment 0 does not fit before segment 1. Resize `LINK_PAD` in
  `vita/src/main.rs` by a few KiB (4 KiB to 8 KiB on 2026-10-28). Roughly one
  code change in twenty triggers it.
- **The title id is compiled in by `vita/build.rs`** from `Cargo.toml`.
  Without it the app listed itself, refused to download updates and failed
  its own package check.
- **LiveArea PNGs must be indexed 8-bit**; `make art` (`tools/src/art.rs`) quantizes them with
  capped Floyd-Steinberg dithering (uncapped, the error left reddish dots).
  The pictures hold no text: the system draws "Start" over the gate image,
  which hid a name drawn there. Name, author and version are LiveArea text in
  `template.xml` (frames 2, 3 and 4, VitaShell's layout), from `vita/Cargo.toml`.
  Confirmed on hardware.
- `APP_VER` is `MM.mm` of `vita/Cargo.toml`'s version ("0.2.0" is "00.02");
  the update check compares the full version `vita/build.rs` compiles in
  (from the tag on CI).

## Titles (`vita/src/platform/`)

- **System applications are in `vs0:app`**, not `ux0:app` (ids `NPXS` plus
  digits). The folder holds about 55 entries, most of them services; 16 are
  listed, in the built-in `system` category. **Launching one:**
  `psgm:play?titleid=NPXS…` does not work on hardware (`0x40000` opens the
  LiveArea; `0x20000` gives C2-12570-5). `SYSTEM_URIS` in `installed.rs`
  gives each its own URI (`settings_dlg:`, `wbapp0:`, `music:`, ...), sent
  once with `0xFFFFF`, from RetroFlow. Friends, Videos, Content Manager,
  Parental Controls and PS4 Link have no known URI and open their LiveArea.
  Confirmed on hardware.
- **The title list is kept in `titles.tsv`** (header version 6: id,
  platform, launch `uri`/`adrenaline`/`retroarch`, path, console, core,
  title). A scan reads one `param.sfo` per title (about ten seconds for a
  hundred titles on hardware), so it runs on the first start and on
  "Rescan titles" only, on a worker thread; later starts read the list. A
  title installed or removed afterwards is not noticed until a rescan.
- **Retail games' files are encrypted.** For an icon or backdrop the host
  tries `ux0:/app/<id>/sce_sys` (plain for homebrew), `ur0:appmeta/<id>/`
  (exists after the home screen opened that game's LiveArea), then
  `sceAppMgrGameDataMount`. All three confirmed on hardware. `param.sfo`
  reads without a mount.
- **The home screen's database** (`appdb.rs`): `ur0:shell/db/app.db`, table
  `tbl_appinfo_icon` (`titleId`, `title`, `iconPath`), read once per scan
  with the system's SQLite module. **The module needs
  `sceSqliteConfigMallocMethods` first**; without it `sqlite3_open_v2`
  returns 7 (SQLITE_NOMEM).
- **`app.db`'s `tbl_appinfo`** holds numbered keys per title. Read from the
  user's copy (2026-10-30, 223 titles): key **1281097414 is the last
  launch** (79 titles: VitaShell at the time the file was copied with it,
  Pocket Shelf 21 s after its install), **2722548093 the first install**
  and 414472871 the last install or update (all 223), as
  `2026-10-30 16:05:07.842` in **UTC** (15:57 for the user's 18:57).
  **The system keeps no play time and no launch count.** A RetroArch game
  counts as RetroArch's launch and a PSP game without a bubble as
  Adrenaline's. Whether a launch from Pocket Shelf (`psgm:play`) updates
  key 1281097414 is not yet confirmed on hardware.
- **PlayStation Mobile** (`psm.rs`): folders in `ux0:/psm` (`NPNA`, `NPOA`,
  `NPPA`, `NPQA` plus five digits), named from `app.db`; a folder with no row
  has no bubble and is left out. **Icons are DDS** (`ur0:appmeta/<id>/icon0.dds`,
  128x128 DXT1, unswizzled; `dds.rs`). Confirmed on hardware with four titles:
  names, icons, backdrops, launch.
- **PSP and PS1** (`pspemu.rs`, `iso.rs`): EBOOTs in `PSP/GAME/<id>/`
  (`CATEGORY` `ME` is PS1, called PSX everywhere), and `.iso`/`.cso` images in
  `ISO` and one level of subfolders (CSO v1 and v2 deflate; LZ4, ZSO, DAX
  and JSO are reported, not read). With a bubble in `app.db` a title starts
  with `psgm:play`; without one (the user's games) it starts through
  Adrenaline. The PSP side's memory stick location is read from
  `adrenaline.bin` (both Adrenalines' layouts, `adrenaline_config.rs`).
- **RetroArch** (`retroarch.rs`): every entry of RetroArch's playlists is a
  title (id `ra` plus seven base-36 digits of the FNV-1a hash of the
  lowercased content path). JSON and the old six-line format; **with
  "Compress Playlists" on (the user's setup) each file is RZIP** (zlib
  chunks; version 2, zstd, is reported). The console is the entry's
  `db_name`, else the playlist's name; each console gets a tab. The core is
  the entry's, else the playlist's default, else an installed core whose
  info file names the console. **Launch:**
  `psgm:play?titleid=RETROVITA&param=app0:/<core>&param2=<path>`; RetroArch's
  salamander stub cuts each parameter at the next `&`, so a path with `&` is
  left out. `finish_launch` closes a running RetroArch first, since a running
  app comes back as it was. The user's 5,811 entries: 102 left out for `&`.
  NES launches confirmed on hardware.
- **Pictures for RetroArch games** are RetroArch's own thumbnails
  (`Named_Boxarts`, else `Named_Titles`, fitted into 128 px; backdrop
  `Named_Snaps`), with `&*/:`"<>?\|` in the label as `_`.

## Launching

- **A launch ends this process.** The host accepts the id; once the frame
  with the "Launching" line is on screen, `installed::finish_launch` sends
  `psgm:play?titleid=<id>`, sends it again after 10 ms and exits (VitaShell's
  updater sequence). A process still alive when the system takes the request
  gets the "close this application?" prompt.
- **Nothing may run between the launch request and the exit.** Sending the
  request a frame before the exit gave the prompt on some launches; two file
  writes after it, on every launch. `recent.json` and `settings.json` go
  through the file thread, which `Host::flush` empties before the request.
  Confirmed on hardware over many titles: no prompt.
- **Staying open next to a game** ("system mode", `CATEGORY=gdc`,
  `libvita2d_sys`) was looked at and not done: it is a renderer port.

## Adrenaline (`psp-boot/`, `seplugins.rs`, `pspemu.rs`)

Pocket Shelf boots PSP and PS1 games through its own kernel plugin,
`pocketshelf.prx`, carried in the VPK as `psp/` and copied to the memory
stick at each launch when it differs.

- **Both Adrenalines must work**: TheOfficialFloW's (the user's, 6.61, with
  the `*KERNEL` line) and isage's fork (8.x, supported from its source
  only). Their plugin lists differ: TheOfficialFloW's reads the first 1024
  bytes of `vsh.txt`; isage's reads `EPIplugins.txt`, else `plugins.txt`.
  `plugin_state` reads both: `noAdrenaline`, `missing`, `off` or `on`.
- **Consent.** Before a launch through Adrenaline, a drawer offers "Add
  plugin" or "Turn on" when the state is not `on`; nothing changes until the
  user confirms. Adrenaline's own files and settings are never modified.
- **Launch.** Pocket Shelf writes `ux0:/pspemu/PocketShelf/boot.txt` (UTC
  tick; `psp`, `psx` or `iso`; the path; for an image the boot file and
  whether it has `BOOT.BIN`) and starts `PSPEMUCFW`. The plugin, at each XMB
  start, deletes the request, drops it when older than 30 s or outside
  `ms0:/PSP/GAME/` or `ms0:/ISO/`, and boots it (`0x141` game, `0x144` PS1;
  an image via `SetUmdFile` and the user's ISO driver setting, as each
  Adrenaline's vshctrl does).
- **When the plugin boots.** From a thread at `module_start`, the first
  launches worked and then one hung on a black screen until the Vita was
  restarted (it raced the XMB's modules). From the start-module handler at
  `vsh_module`, nothing started (both Adrenalines call it inside the module
  manager). Now: the handler notes `vsh_module` and the first module after
  it, and a thread boots 0.2 s after that one (falling back to 2 s after
  `vsh_module`). **Confirmed on hardware (2026-10-04):** ISO and CSO games
  boot; only the XMB's gray background shows for a moment. Each request
  leaves a line in `ux0:/pspemu/PocketShelf/boot.log`, shown at the end of
  Diagnostics.
- **Booting with no XMB at all** (RetroFlow's way, with patched Adrenaline
  modules) was looked at and not done: the user chose not to modify
  Adrenaline's files.
- The plugin is committed (`vita/package/psp/pocketshelf.prx`); rebuild with
  `make plugin` (pspdev in `~/pspdev`; its compiler needs Homebrew `gmp`,
  `mpfr`, `libmpc` and `zstd`).

## Network

- **The Vita's own SSL cannot reach Cloudflare-fronted sites** that serve
  only an ECDSA certificate (`0x80435061`). `vita/src/platform/http.rs` uses
  `rustls` with `rustls-rustcrypto` (an unaudited alpha) and Mozilla's roots,
  with verification on.
- `http::state` calls `sceKernelPowerTick` while a request runs, so the
  console does not suspend during a long Fetch artwork run.

## SteamGridDB and libretro

- **SteamGridDB**: `https://www.steamgriddb.com/api/v2`, `Authorization:
  Bearer <key>`, the key one line in `steamgriddb.txt`. **Never commit or
  print the user's key.** Static PNG only, since the decoders read PNG.
  Files are `<title id>-<picture id>.png`.
- **libretro's thumbnail server** (`online/libretro.rs`), the one
  RetroArch's "Download Thumbnails" uses, for RetroArch games and, by
  platform, PSX and PSP games. A console's name list (4 MB of HTML for NES)
  is saved once as names to `libretro/<console>.txt` and read on a host
  thread. **Matching** (`match_name`): the label, else a name with the same
  base in the label's region: 70% of the user's 5,811 games (510 by label,
  4,042 with the matching). The icon is the box art (else the title screen)
  fitted into 128 px by the host (25–35 KB instead of 450–700 KB); the
  backdrop the screenshot (else the title screen). Files
  `<title id>-lr-box.png`, `-lr-title.png`, `-lr-snap.png`.
- **The scraper is the user's choice** (`scraper` in `settings.json`): Auto
  (libretro for RetroArch games, SteamGridDB for the rest), SteamGridDB, or
  libretro.
- **Find artwork** (title editor) keeps candidates in memory (at most four
  downloads, 8 MB each) and writes only the one used. The used picture goes
  to the tile from the preview's texture: reading the file back while the
  file thread was still writing it cached "no picture" (the tile showed
  initials). Confirmed on hardware.
- **Fetch artwork** (SELECT ▸ Library): one request at a time over a
  category or all titles but the system apps (whose names match unrelated
  games); busy replies (429) waited out five times, other failures halt the
  run. Confirmed on hardware on 55 PSP games, libretro then SteamGridDB.
- **Clean up artwork**: Unused keeps files a title's changes name and icons
  matched by name; All deletes every file and takes the names out of the
  titles' changes. Confirmed on hardware.
- **Pickers** list the files named after the title (its name, the name it
  came with, or its id), or the whole folder; triangle switches. A file that
  does not decode is marked and not used.

## Updates

SELECT ▸ Updates: **Channel** (Stable, Beta, Nightly, Off) and **Check for
updates**.

- **Check:** `api.github.com/repos/fthdgn-gh/pocket-shelf/releases?per_page=10`
  (no key; 60 requests an hour per address). Only tags `shelf-v<version>`
  and `shelf-nightly` count; drafts are not listed, so a draft takes a
  release down. A beta comes before its release (`0.1.0-beta.3` < `0.1.0`).
  At start: the updater's report if it ran, else a check at most once a day
  (`update.json`: `checkedAt`, `later`).
- **Install:** the VPK downloads to `update/pocket-shelf.vpk`, is unpacked to
  `update/pkg/` on a thread (CRC-checked, its `param.sfo` must name this
  title id), and the **updater app** (`POCKTUPDR`, `updater/`, **GPL-3.0**
  for VitaShell's `makeHeadBin` and `head.bin`) is installed from
  `app0:updater`, started, and installs `update/pkg/` with
  `scePromoterUtilityPromotePkgWithRif`; it writes `update/result.txt` and
  starts Pocket Shelf, which shows the result, deletes `update/` and removes
  the updater.
- **The updater froze before starting Pocket Shelf** twice (2026-10-07 with
  0.1, 2026-10-27 with 0.2's first build): installed, but stuck on
  "Starting it..." with the PS button ignored, in the step that began with
  `vita2d_free_pgf` and `vita2d_fini`. The graphics are now left to the
  process exit and the launch waits 250 ms (VitaShell's updater draws
  nothing). **Confirmed on hardware (2026-10-27):** 0.2.0-beta.0 → beta.1
  downloaded, installed and restarted by itself. Each step goes to
  `ux0:data/PocketShelf/updater.log`.
- Rebuild the updater with `make updater` (VitaSDK); its build is
  committed in `vita/package/updater/`.

## Status bar (`vita/src/status.rs`, `shelf/chrome.rs`)

- The time comes from `sceRtcGetCurrentClockLocalTime`; "System" follows the
  system's 12/24-hour setting (`sceAppUtilSystemParamGetInt`).
- **Wi-Fi copies the Vita's own symbol** (a dot and three quarter rings, lit
  in 25% steps of the signal) and shows only while connected
  (`sceNetCtlInetGetState`). Vita3K stubs it as connected at 100%.
- **Bluetooth is only "switched on in Settings"** (`/CONFIG/BT/bt_enable`);
  user code has no call for connected devices. Not verified on hardware.
- The items take the width of those shown; each can be hidden, on the left
  or the right (`status-bar.json`).

## Library behavior

- **Category order**: smart tabs, built-ins (Games, Homebrew, PS Mobile,
  PSP, PSX), RetroArch consoles, the user's own, System last. A new category
  goes before System while System is last. Console ids (`retro-snes`, ...)
  are saved and must not change.
- **Smart categories**: "Last Played" (at most 15, `recent.json`) and
  "Favorites" show no tab while empty. A search result is saved under its
  own category, since the Search tab does not outlive the process.
- **Do not store a translated text in saved data.** Built-in categories
  carry a `name` key and get their label from the language.
- **A held direction speeds up** on the shelf: every 6 frames after 0.4 s,
  1 item, then 5 after 1.2 s, then 20 after 3 s; it stops at the end and
  starts over at one step on a tab change. A press wraps; a hold does not.
- **The tab names slide** so the current one is centered between L and R at
  every press; a name an end cuts shows in part, faint.

## Touch

- **The front panel reports 1920×1088**, twice the screen in each
  direction (`sceTouchPeek` after `sceTouchSetSamplingState` for the front
  port); the app halves it. The rear panel is not read (it is touched by
  accident while holding the console). Taps, drags and flicks are tested
  on the desktop only; the slop (16 px), the tap time (0.6 s) and the
  flick's slowing are not yet tuned on hardware.

## Drawing on the Vita

- **vita2d's clip flickered the top bar** on hardware: each clip change drew
  two full-screen stencil passes. Clips are applied on the CPU now.
- **Texture memory takes CPU writes at about 100 MB/s**; pictures upload
  through a queue at 256 KB a frame.
- **Writing a file in the frame cost 24–30 ms** on hardware; writes go
  through the file thread.
- Measured on hardware with 5,905 titles: library loaded in 55 ms, first
  frame at 0.8 s, rebuild 38.6 ms (a one-title change recomputes that title
  only), 16.7 ms average frame while idle.

## Languages

English, Turkish, German, French and Spanish, in `core/src/locales.rs`: a
language missing a text does not compile, and the tests check that every
language has the same `{placeholders}` as English.
The keyboard's accents page holds the same thirty letters in every language,
the current language's own first. `upper` handles the Turkish dotted and
dotless i.

## Open items

- Images on isage's Adrenaline, a moved memory stick and the ISO driver
  setting are not seen on hardware; game updates (`PBOOT.PBP`) for images and
  ZSO are not done.
- See `PLAN.md`, Follow-ups.
