# Pocket Shelf: working notes

Notes for picking this project up again: what was learned the hard way about
the Vita, its firmware, Adrenaline and RetroArch, and what has been verified
where. Update them when something here stops being true. The crates and the
drawing model are in [`ARCHITECTURE.md`](ARCHITECTURE.md); the port from
PocketJS is in [`PLAN.md`](PLAN.md).

## What this is

Pocket Shelf (Vita title id `POCKTSHLF`) is a PS Vita launcher for installed
titles and RetroArch and Emu4Vita++ games, written in Rust (`core/`, `vita/`). Up to 0.1 it
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

**Vita3K** (`/Applications/Vita3K.app`, binary
`Contents/MacOS/Vita3K`) comes after the desktop shots and before a console
test: the user drives its window, and its log is read here directly at
`~/Library/Application Support/Vita3K/Vita3K/fs/ux0/data/PocketShelf/log.txt`
(no copy to Downloads). Its `ux0:` is
`~/Library/Application Support/Vita3K/Vita3K/fs/ux0/`. Its window cannot be
seen or screenshotted from here, and it cannot be driven. What it cannot
show goes to the console: launching titles, package installs (updates),
taiHEN plugins (Console start), and NEON results (below).

- Launch in the background with output to a log file. `pkill` does not stop
  it; use `kill -9` on instances started here. Never kill one the user opened.
- It runs the real firmware `libssl`/`libhttp` modules, so TLS results there
  match hardware.
- **It does not empty a file opened for writing** (2026-11-05): a shorter
  `settings.json` kept the old file's tail (`..."title":"NPNA00001"}}a"}`),
  did not parse at the next start, and every setting went back to its
  default; `titles.cache.json` likewise. The file thread now writes
  `<file>.part` and renames it over the file, and saved JSON is read up to
  its first value (`library::read_json`). The console empties the file.
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
- **The title list is kept in `titles.tsv`** (header version 7: id,
  platform, launch `uri`/`adrenaline`/`retroarch`, path, console, RetroArch
  core, Emu4Vita++ core, title; version 7 added the Emu4Vita++ column, so
  the first start after the update scans again). A scan reads one `param.sfo` per title (about ten seconds for a
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
- **Emu4Vita++** (`emu4vita.rs`, noword/Emu4VitaPlus, title id
  `EMU4VPLUS`): one app with every core as `app0:eboot_<core>.self`. **Its
  core picker reads a launch URI** since June 2026 (release 0.81):
  `psgm:play?titleid=EMU4VPLUS&core=<core>&rom=<path>` (`arch/source/main.cpp`)
  runs the core with `--rom <path>`; each value ends at `&` and is then
  percent-decoded with `+` as a space, so the path goes with `%`, `&` and
  `+` escaped, and a path with `&` works. The core-only VPKs
  (`StandAlones.7z`) read no URI. **It writes no playlists**: it reads
  RetroArch's on `ux0:` (the folder `retroarch.cfg` names), its own
  `ux0:/data/EMU4VITAPLUS/playlists` and RetroArch's on `uma0:` for names
  only (`ra_lpl.cpp`). The scan reads the same three folders, RetroArch's
  first so a file keeps the id it had, and gives each entry an Emu4Vita++
  core: the RetroArch core of the same name, else its preferred core for the
  console (`DATABASES`, copied from its core picker, `arch/source/app.cpp`).
  A file listed twice under two spellings (`ux0:roms`, `ux0:/Roms`) is one
  game (`path_key`). **Favourites** are per core in
  `ux0:/data/EMU4VITAPLUS/<short name>/favourite.ini` (SimpleIni: `path`
  folder, `name` file, `rom_name`); they are read at every start, after the
  list (`with_favourites`), not saved in `titles.tsv`, and never written.
  `finish_launch` closes a running Emu4Vita++ first. **Not yet confirmed on
  hardware**: the launch URI, the favourites and the box art Emu4Vita++
  downloads (`cache/thumbnails/<console>/<rom_name>.png`).
- **Which app opens a game** both can open: SELECT ▸ Games ▸ Open games
  with, and the editor's Open with per game, saved in `emulators.json`
  (`EmulatorChoice`; RetroArch when absent). A game only one app opens uses
  that app whatever the choice.
- **Which core** (the editor's Core row, `cores.json`, `CoreChoice`): per
  title and per app. `Host::cores` reads the lists on a thread: RetroArch's
  installed cores that run the game's console, the playlist's core first: by
  `info/<core>.info`'s `database`, by another game of the console in the
  playlists using it, or by Emu4Vita++'s table naming it (Vita3K's
  RetroArch had info files for 2 of its 8 cores); Emu4Vita++'s installed cores for the
  console from its table (`emu4vita::DATABASES`). At launch the chosen core
  is checked with one file stat (`<core>_libretro.self` in RetroArch's app
  folder, `eboot_<core>.self` in Emu4Vita++'s) and the default used when it
  is gone. **Recommended** (`library/recommended.rs`), offered after Default
  when it is another installed core, is saved as `recommended` and resolved
  at launch from a table researched on 2026-11-05 for RetroArch 1.22.2 and
  Emu4Vita++ 0.81: GameTechWiki's "Emulators on PS Vita" frame rates, the
  cores' Vita builds (gpSP and PCSX ReARMed dynarecs) and Emu4Vita++'s
  starred cores (`arch/source/app.cpp`, `ICON_STAR`); unsourced picks and
  arcade (ROM set decides) are left out. **Not yet run on hardware**;
  Vita3K cannot launch titles.
- **Pictures for RetroArch games** are RetroArch's own thumbnails
  (`Named_Boxarts`, else `Named_Titles`, fitted into 128 px; backdrop
  `Named_Snaps`), with `&*/:`"<>?\|` in the label as `_`; then, for an
  Emu4Vita++ favourite, the box art Emu4Vita++ downloaded, and for the
  backdrop `<folder>/.previews/<file stem>.png`, Emu4Vita++'s preview.

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

## Console start (`shell-boot/`, `autostart.rs`)

- **No system setting starts an app at console start**; it takes a taiHEN
  user plugin in the home screen. taiHEN loads `*main` plugins into SceShell
  and reads `ux0:tai/config.txt`, else `ur0:tai/config.txt` (taiHEN's
  `taihen.h`). **Holding L (or L1) while the console starts skips every
  plugin** (`taihen.c`, `SCE_CTRL_LTRIGGER | SCE_CTRL_L1`): the way back
  from a plugin that keeps the home screen from starting.
- **Prior art** (2026-11-05): Rinnegatamante's AutoBoot (no license, so none
  of its code is used) waits for NPXS10079, which the system starts with the
  home screen, then calls `sceAppMgrLaunchAppByUri(0xFFFFF,
  "psgm:play?titleid=…")` with the title in `ux0:data/AutoBoot/boot.cfg`.
  VitaXMB (GPL-3.0) found **a launch 5 s into the boot crashed the system
  four times while AutoBoot's 10.6 s did not**; VitaOS (GPL-3.0) launches
  twice, as a first launch can stop at the title's LiveArea page.
- **Pocket Shelf's plugin** (`shell-boot/main.c`, MIT, 7 KB): open at start
  waits for NPXS10079 and 12 s after it loaded, opens `POCKTSHLF` when it
  is installed and no other application runs, and again 3 s later when it
  is not running. Return after a game: Pocket Shelf ends when it starts a
  title, so before the request it writes `return.txt`; the plugin polls
  every 0.5 s, waits until an application other than Pocket Shelf and the
  system's (`NPXS…`) runs, then until none does for two polls, deletes the
  file and opens Pocket Shelf. It deletes the file without opening anything
  when Pocket Shelf runs again first, or when nothing started within 60 s.
  A system app started from Pocket Shelf (Settings) does not count, so
  closing it does not bring Pocket Shelf back. Its log of each console start
  is `boot-plugin.log`; Console start copies its last 40 lines into the
  app's log.
- **SELECT ▸ System ▸ Console start** changes the tai config only after a second
  press, adding one line first under `*main` (or a `*main` section) in the
  file's own line endings; the change is checked to touch no other line,
  the old file is kept as `config.txt.pocketshelf.bak`, the new one goes
  through `config.txt.part` and is read back. With AutoBoot in `*main`, Open
  at console start writes AutoBoot's `boot.cfg` instead, and Pocket Shelf's
  plugin does not open the app at start (`boot=0`). An installed plugin
  older than the app's is replaced when the drawer opens.
- **On the user's Vita** (2026-11-05, build c3546cd0): the drawer installed
  the plugin (so the app may write `ur0:tai` and the tai config), Pocket
  Shelf opened at console start, and it opened again when a game it started
  closed; closing Pocket Shelf itself left the home screen, as designed.
  What `sceAppMgrGetRunningAppIdListForShell` lists is in
  `boot-plugin.log` (not yet read).

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
- **libretro's game databases** (`online/gamedb.rs`): libretro-database's
  `rdb/<database>.rdb` on GitHub (raw.githubusercontent.com), one per
  console, named as RetroArch names the console; 148 files, 151 MB in all,
  Game Boy 1.0 MB, NES 6.3 MB, PlayStation 7.7 MB (2026-11-04). The file is
  libretrodb: `RARCHDB\0`, a big-endian u64 metadata offset, then one
  MessagePack map per record up to a nil. **Records of one game are
  merged by CRC** (several dat files are merged into each database: NES
  has 29,646 records, 4,328 sharing a CRC). The download thread turns the
  `.rdb` into `gamedb/<database>.tsv` (crc, serial, arcade set, name,
  year, genre, developer, publisher, players), and the `.rdb` is never
  written: NES 6.1 MB becomes 1.5 MB in 25 ms on the Mac. Fetch artwork
  downloads a console's table when it is missing or over 90 days old.
  **NES records hold the CRC of the file with its 16-byte header**; a
  GoodTools dump with another header does not match and goes by label.
- **Identifying titles** (`platform/gamedb.rs`, a host thread): the label
  when it is a database name (RetroArch's scanner wrote it), a PSP or PS1
  title by its title id as a serial (`ULUS10041` is `ULUS-10041`; PSN
  titles, `NP…`, are in `Sony - PlayStation Portable (PSN)`; no file is
  read), an arcade set by its zip's name, then the CRC: a zip's directory holds each
  file's CRC (two reads of the file's end), a loose file up to 64 MB is
  read whole, with a second CRC after 512 bytes for a SNES copier header;
  disc images are not read. **The user's playlists carry no CRC** (every
  `crc32` is `00000000|crc`, a manual scan): 1,212 zips, 3,489 `.nes` and
  947 `.bin` files. CRCs are kept in `gamedb/crcs.tsv` by path, so a file
  is read on the first run only. **What each title was found to be is
  kept in `gamedb/known.tsv`** (database, the table's build time, the
  details; a miss is looked up again in a newer table only) and read on
  the scan thread with the title list. At each start, after the dates, the
  titles new since the last run are identified for each table on the card
  (no network). SELECT ▸ Games ▸ Game database downloads every table
  over a week old and identifies every title, a database at a time, each
  followed by its Wikidata table.
  Fetch artwork matches the database name before the label; Find artwork
  searches it. **Coverage** of regular releases (2026-11-04): year 58% on
  NES, 82–89% on SNES, Mega Drive, Game Boy and PS1; genre, developer and
  players 13% on NES, 29% on Mega Drive, 47% on SNES, 83% on GBA, 86% on
  PS1; region over 80% everywhere. **Not yet confirmed on hardware**: the
  first run's file reads, how many of the user's games are identified,
  and the frame that takes a console's results (2.1 ms on the Mac for
  5,905 titles, about 100 ms on the Vita by the usual ratio).
- **What identified titles show**: Games ▸ Game names (`game-info.json`):
  Own (the playlist label or the title's name), Database, or Short (the
  database name without brackets, a trailing ", The" moved to the front);
  a title the user renamed keeps the user's name. With database names,
  name order is worked out in the rebuild (0.3 ms more on the Mac for
  5,905 titles); the names are made once per title (`Game::with_details`).
  Games ▸ Game details: the panel of year, genre, developer,
  publisher, players and region with their symbols (`shapes::Icon`), a
  column right of the title in Home and the Cascade, one row under it in
  the Carousel and the Grid; with the panel off, the line under the title
  carries the year and genre. SELECT ▸ Sort has Newest and Oldest release;
  titles with no year go last.
- **Wikidata's game details** (`online/wikidata.rs`, `tools/src/wikidata.rs`):
  CC0 data, so Pocket Shelf publishes it: `make wikidata` (weekly in
  `gamedb-data.yml`) builds one table per console, named as the app's tab
  for the console (`nes.tsv`, `arcade.tsv`, `psp.tsv`; Vita titles
  `vita.tsv`), with each game's item, English (else multilingual) name,
  aliases, earliest release year, genre and mode items, developers,
  publishers, series and Wikipedia article titles (en, tr, de, fr, es, ja);
  `labels.tsv` names the genres and modes in the app's five languages.
  **The tables are release files, not committed**: the `gamedb-data`
  release, files replaced in place (`gh release upload --clobber`). The
  update check reads only `shelf-v` tags, so the release is never offered
  as an update. **It is a pre-release**: the app's releases all are, so as a
  full release it became the repository's Latest (`--latest=false` did not
  prevent it). **No checksums**: a game is found by name
  (`wikidata::name_key`, the article moved and brackets dropped as
  `short_name` does, compared as `base_name` compares), among its names and
  aliases. Each console is a set of small queries (names, dates, aliases,
  genres, modes, developers, publishers, series, articles): a query joining
  them timed out (the query service cuts a reply off at 60 s), as did one
  grouping names with dates for the NES. One second between queries;
  busy and cut-off replies are waited out. **A full build** (2026-11-04,
  from the desktop): 62 tables, 31,805 games, 4.4 MB, 576 queries in about
  70 minutes; the larger consoles (PlayStation, DOS, Commodore 64) got
  120 s waits, so the workflow allows 4 hours. **Matches**
  (`make wikidata`): the sample playlists' labels 90% on GBA, 89% SNES,
  82% Game Boy, 80% Game Gear, 75% Game Boy Color, 68% Mega-CD, 66% Mega
  Drive, 54% Neo Geo, 47% NES. The NES misses are mostly the GoodNES set's
  hacks, public-domain programs, translations, unlicensed carts and bad
  dumps; the Mega Drive ones retitled releases (`Escape From Mars Starring
  Taz` is *Taz in Escape from Mars*) and European and Japanese games
  Wikidata lacks. Of Wikidata's own games, libretro's regular names reach
  1,040 of 1,211 on the NES, 1,108 of 1,427 on the SNES, 1,859 of 2,239 on
  the PlayStation.
- **Wikidata's details in the app** (`shelf/database.rs`,
  `vita/src/platform/wikidata.rs`): the Game database run reads the
  release's `index.tsv` once, then goes console by console: libretro's
  database identifies the titles, then the console's Wikidata table is
  matched; Vita titles (ids `PCSA`–`PCSH`, `VCAS`/`VLJM` forms; no
  homebrew, no system apps) have the Wikidata step only, last. A table is
  downloaded to `wikidata/<file>.tsv` when the card's was built before the
  release's, with `wikidata/labels.tsv` once a run; a download that does
  not read as a table is not written. A title is looked up by the database
  name libretro gave it, then its own name, and of several games with the
  name the one released in libretro's year. **What each title was found to
  be is kept in `wikidata/found.tsv`** (id, table, the table's build time,
  the game's line without its aliases; a miss is looked up again in a newer
  table only), sent after the title list (5,942 titles read in 592 ms on
  the Vita, which the list waited for when they came with it), and at each
  start the new titles are matched in the tables on the card (no network).
  The details panel takes each field from libretro first, Wikidata second:
  year, the first genre in the app's language, developers, publishers, and
  players from the modes (Multiplayer for a multiplayer or co-op mode, 1
  player for single-player only); region is libretro's only. On the Mac
  the PlayStation table (2,239 games) parses in 7 ms and matches in 4 ms,
  about 350 and 200 ms on the Vita's worker thread by the usual ratio.
  **On the user's Vita** (2026-11-05, build 7eee69a5): the downloads
  through the release's redirect work; a run over 13 consoles took 76 s
  and matched 1,745 of 3,489 NES titles, 726 of 947 Mega Drive, 703 of
  777 SNES, 137 of 150 GBA, 124 of 150 Game Boy, 42 of 53 PSP and 43 of
  53 Vita games. A table reads in 100–350 ms on the worker thread (the
  PlayStation's 2,239 games in 1.3 s), a match takes 1.6–2.9 s with the
  reads of `known.tsv` and `found.tsv`, and the frame that takes a
  console's results spends about 90 ms in the library's rebuild, as
  libretro's results do. **At start** (0.5.1-beta.1, 2026-11-05): the
  title list in 781 ms as before; `found.tsv` read in 817 ms on the scan
  thread after it (5,942 titles, 3,733 found), and the frame that takes
  them spends 422 ms in events, 307 ms of it in `set_wiki` and the
  library's rebuild. **The update from 0.5.0-beta.1 worked** on hardware.
- **The scraper is the user's choice** (`scraper` in `settings.json`): Auto
  (libretro for RetroArch games, SteamGridDB for the rest), SteamGridDB, or
  libretro.
- **Find artwork** (title editor) keeps candidates in memory (at most four
  downloads, 8 MB each) and writes only the one used. The used picture goes
  to the tile from the preview's texture: reading the file back while the
  file thread was still writing it cached "no picture" (the tile showed
  initials). Confirmed on hardware. **The tile's own request in the same
  update went to the host anyway** until 2026-11-05 (`Art::expect` now
  marks the box art as on its way): the desktop's render showed the file
  read back, not the preview.
- **Browse** (title editor ▸ Box art or Backdrop ▸ Browse..., `browse.rs`):
  the host lists a folder on a thread of its own (`Host::list_folder`,
  folders and `.png` files, at most 10,000 entries; `""` lists the devices
  that read, `ux0:` first) and decodes a file by its full path as it would
  be saved (`PictureKey::File`: 128 px fitted box art, or a backdrop). **The
  file used is named by its full path** in the title's changes
  (`overrides::is_path`: a device, `:`, `.png`; up to 255 characters where
  a name in `art/` stays limited to 96), read where it is by `art.rs` and
  `backdrop.rs`; nothing is copied (the first version copied it, which the
  user did not want). Clean up artwork leaves paths alone. The preview goes
  to the tile as Find artwork's does. The folder is kept per kind in
  `folders.json`; Browse opens on the title's own file when it has one. **In
  Vita3K** (2026-11-05, the copying version): folders listed in 2–9 ms,
  `ux0:` read as it is (not `ux0:/`), a box art and a backdrop set and both
  folders kept; it showed a device opened from the list of devices as
  `/ux0:`, fixed. **Not yet run on hardware**: the device roots and listing
  times on the card.
- **Folder categories** (SELECT ▸ Games ▸ Categories ▸ New folder
  category, 2026-11-06): Browse in folder mode picks the folder, kept as
  `ux0:/…` (`roms::canonical`) in `rom-folders.json`, a file of its own so
  `categories.json` stays as 0.5 writes it. `Host::rom_folders` hands the
  folders to the host before the first scan; adding or deleting one reads
  them again on the scan thread (`installed::rescan_folders`: the saved list
  without its folder games, the folders walked, saved), not a full scan.
  `library/roms.rs` walks and names consoles (extension, zip central
  directory, disc headers, folder names; tested with files in memory);
  `vita/src/platform/folders.rs` gives each game RetroArch's recommended core
  when installed, else one whose info file names the console, else one
  Emu4Vita++ runs it with, and Emu4Vita++'s preferred core. Folder games are
  `folder` lines in `titles.tsv` with a playlist game's columns; a release
  before them meets an unknown launch and scans over the list. Their ids are
  `retroarch::game_id` of the path, the same a playlist entry for the file
  gets; a file a playlist lists is left to the playlist, before it is
  opened (`walk`'s `skip`: a folder holding a playlist's 5,800 files would
  otherwise open each). A folder tab lists only the games its reading found
  (the user's choice, 2026-11-06: the console tabs already hold the playlist
  games); such a game takes the folder tab as its own category, adds no
  console tab, and shows in an outer folder category's tab too
  (`Game::path`, a `path_key`). Limits: 8 folders deep, 60,000 entries,
  20,000 games per reading; a zip64 or `.7z`/`.chd` goes by its folders.
  **Not yet run on hardware**: walking times on the card, and launches of
  folder games.
- **Rescan this tab** (SELECT, above Language, 2026-11-06): `Host::rescan_some`
  with the scan phases that fill the tab (`Shelf::tab_sources`);
  `installed::rescan` keeps the saved list's other titles (`Parts::holds`:
  apps, PSM, PSP/PS1, playlists, folders) and runs `scan_parts` for the
  rest. A playlist rescan reads the folders after it, since folder games
  leave out what playlists list. Tabs of titles from anywhere run a full
  scan. Its progress shows as the row's value.
- **Fetch artwork** (SELECT ▸ Games ▸ Artwork): one request at a time over a
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

SELECT ▸ System: **Updates**, the channel (Stable, Beta, Nightly, Off), and
**Check for updates**.

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

## The system keyboard

- **`sceImeDialog`** (SceCommonDialog stub): the parameters are set up as
  the SDK's header function `sceImeDialogParamInit` does (zeroed, the SDK
  version, and the magic number plus the common part's address), and they
  and the UTF-16 buffers stay at one address until the dialog is finished.
  The dialog is drawn by `vita2d_common_dialog_update` between
  `vita2d_end_drawing` and the swap. **Not yet tried on hardware.**

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
