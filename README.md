# Pocket Shelf

A launcher for the PS Vita. It lists the titles installed on the console
(Vita games, homebrew, PlayStation Mobile, PSP and PS1 games, and system
applications) and the games in your RetroArch playlists, in tabs. Each title
gets an icon and a backdrop, and it starts with one button press.

Pocket Shelf is a native Rust app: it draws with vita2d, rasterizes its own
text with `fontdue`, and runs the title scan, picture decoding, file writes and
downloads on worker threads. Up to 0.1 it was a TypeScript app on
[PocketJS](https://github.com/pocket-stack/pocketjs); 0.2 replaced it and kept
its saved data.

- Title id `POCKTSHLF`
- Version 0.2.0

<p align="center">
  <img src="./screenshots/home.png" width="720" alt="Pocket Shelf's Home view on the Games tab in the Dynamic theme: the selected title's icon large at the left in a white ring, the next titles smaller after it, the title's name, platform and category, and a Launch button, over the title's backdrop." />
</p>

| Carousel | Cascade | Grid |
| --- | --- | --- |
| ![The carousel view: the selected title's icon enlarged in the middle of a row, its name below.](./screenshots/carousel.png) | ![The cascade view: the selected title's icon large at the left, the next titles shrinking and darkening to the right, its name below.](./screenshots/cascade.png) | ![The grid view: two rows of five title icons with the selected one in a white ring, its name below.](./screenshots/grid.png) |

| List | RetroArch | Menu |
| --- | --- | --- |
| ![The list view: title rows with small icons, the selected row highlighted.](./screenshots/list.png) | ![The Home view on a RetroArch console tab, SNES: a game from a RetroArch playlist, its console in the detail line.](./screenshots/retroarch.png) | ![The SELECT menu's Appearance page as a floating card: theme, view, details, backdrop, icon box and status bar.](./screenshots/menu.png) |

The screens are rendered on the desktop by `make shots` with sample titles;
the colored tiles with initials stand in for game icons.

## Features

- **Tabs per kind of title:** Games, Homebrew, PS Mobile, PSP, PSX, one tab
  per RetroArch console, and System last. You can add your own categories,
  and rename, reorder or hide any of them. "Last Played" and "Favorites" fill
  themselves.
- **Five views:** Home (the default), after the PS5 home screen: the selected
  title large at the left, its name and a Launch button below; a carousel; a
  five-column grid; a list; and a cascade after the Xbox 360 dashboard. Each
  has three detail levels: Basic, Normal and Detailed.
- **RetroArch games** from RetroArch's own playlists, one tab per console.
  Each game starts straight into the core its playlist names, without
  RetroArch's menu (see [RetroArch](#retroarch)).
- **PSP and PS1 games through Adrenaline.** This covers EBOOTs in
  `PSP/GAME` and ISO and CSO images in `ISO`. Pocket Shelf boots the game with
  its own small VSH plugin, so you don't have to pick it in the XMB. Before it
  installs or turns on the plugin, Pocket Shelf asks you.
- **Artwork from [SteamGridDB](https://www.steamgriddb.com/) or libretro's
  thumbnail server.** You can pick an icon and a backdrop for one title, or
  fetch artwork for a whole category at once. Choose the source: Auto
  (libretro for RetroArch games, SteamGridDB for the rest), SteamGridDB, or
  libretro (RetroArch, PS1 and PSP games, no key). SteamGridDB needs your own
  API key.
- **Long lists:** holding a direction speeds up, from one title at a time to
  twenty, so a tab of thousands of games can be crossed in seconds.
- **Search** across every tab with the on-screen keyboard.
- **Per-title changes:** name, category, icon, backdrop and favorite.
- **Six themes:** Midnight, Aurora, Sakura, Ember, Daylight, and Dynamic, which
  takes its colors from the selected title.
- **Five text sizes**, from Smaller to Larger (SELECT ▸ Appearance ▸ Text size).
- **Sort each tab** (SELECT ▸ Sort): by name, by when it was last played,
  by how often Pocket Shelf started it, or by when it was installed. The
  system records the last launch and the install date; it keeps no play
  count, so "Most played" counts the launches made from Pocket Shelf.
- **Status bar** with the clock, Wi-Fi, Bluetooth and battery, each of which
  can be hidden, on the left or the right of the top bar.
- **Diagnostics** (SELECT menu): frame timing, startup timing, what the last
  title scan found and skipped, and the last Adrenaline launches (see
  [Troubleshooting](#troubleshooting)).
- **Five languages:** English, Turkish, German, French and Spanish.
- **Updates from GitHub** on the channel you pick (see [Updates](#updates)).

## Install

1. Get Pocket Shelf from one of these:
   - **VitaDB:** [Pocket Shelf on VitaDB](https://www.rinnegatamante.eu/vitadb/#/info/1704).
     The VitaDB Downloader app on the Vita downloads and installs it; go on to
     step 3. VitaDB lists a new release after its maintainer adds it, so it
     can be a version behind GitHub.
   - **GitHub:** download `pocket-shelf.vpk` from the newest tagged release,
     or build it yourself (see [Building](#building)).

   [![Newest release](https://img.shields.io/github/v/release/fthdgn-gh/pocket-shelf?include_prereleases&sort=semver&label=newest%20release)](https://github.com/fthdgn-gh/pocket-shelf/releases)

   There is no stable release yet. The current one is the beta
   [`shelf-v0.2.0-beta.3`](https://github.com/fthdgn-gh/pocket-shelf/releases/tag/shelf-v0.2.0-beta.3)
   ([download the VPK](https://github.com/fthdgn-gh/pocket-shelf/releases/download/shelf-v0.2.0-beta.3/pocket-shelf.vpk)).
   GitHub marks betas as pre-releases, so its "Latest" label does not point to
   them. Releases come in three channels:
   - **Stable:** tested on hardware.
   - **Beta** (marked pre-release): new features, tested less.
   - **Nightly** (`shelf-nightly`, when available): the current `main`, untested.
   Each release's VPK carries a signed build provenance from GitHub Actions:
   `gh attestation verify pocket-shelf.vpk -R fthdgn-gh/pocket-shelf` checks
   that the file was built by this repository's workflow, and from which
   commit.
2. Copy the VPK from GitHub to the Vita and install it with VitaShell.
3. Open Pocket Shelf from the home screen. On the first start it scans the
   installed titles and shows its progress. Later starts read the saved list.

After you install or remove a title, run **SELECT ▸ Library ▸ Rescan titles**.

### Updates

**SELECT ▸ Updates** sets the channel (Stable, Beta, Nightly or Off) and checks
for a newer release. Pocket Shelf also checks once a day at start. To install,
it downloads the VPK, then starts a separate app, Pocket Shelf Updater, which
installs it, opens the new version and is removed again.

0.2 is offered to 0.1 betas on the Beta channel, and keeps their settings,
categories, title changes and artwork. That update runs 0.1's updater, which
can stop on its last screen ("Starting it..."): the update is installed by
then, so restart the Vita and open Pocket Shelf. From 0.2 on, the updater
starts Pocket Shelf by itself.

### Adrenaline (PSP and PS1 games)

Both TheOfficialFloW's Adrenaline and isage's fork (8.x) are supported, but
only TheOfficialFloW's has been tested on hardware. The first time you start a
PSP or PS1 game, Pocket Shelf offers to add its plugin to Adrenaline's plugin
list. Nothing changes until you confirm. Adrenaline's own files and settings
are never modified.

A launch writes a request to `ux0:/pspemu/PocketShelf/boot.txt` (under the
memory stick location Adrenaline uses) and starts Adrenaline; the plugin
boots the game and adds a line to `boot.log` next to it.

### RetroArch

Install RetroArch for the Vita (`RETROVITA`) and scan your games into
playlists in RetroArch (Import Content). Pocket Shelf reads those playlists,
compressed or not, from `ux0:/data/retroarch/playlists`, or the folder
`retroarch.cfg` names. Each game starts with the core its playlist entry names,
else the playlist's default core, else an installed core for its console.

- After you add games to a playlist, run **Rescan titles**.
- Games whose file path holds `&` are left out: RetroArch cuts its launch
  parameter there. Rename the file to bring the game in.
- RetroArch's PlayStation and PSP games join the PSX and PSP tabs.

### SteamGridDB API key

Create a key in your SteamGridDB profile's preferences. You can type it in the
app, or put it as one line in `ux0:/data/PocketShelf/steamgriddb.txt`.

### Your own box art and backdrops

Pocket Shelf reads PNG files from two folders in `ux0:/data/PocketShelf/`.
Create them if they are not there yet (Find artwork creates them when it
saves its first picture), and copy files in with VitaShell (USB or FTP):

| Folder | Holds | Shown as |
| --- | --- | --- |
| `art/` | Box art, in place of a title's icon | Whole, in its own shape, inside the tile's square; the selection ring follows its edges |
| `backdrops/` | The picture behind the selected title | Cropped to the screen's shape (960×544), centered |

**Box art is matched by file name.** A file named after the title id
(`PCSA00069.png`) or the title (`Gravity Daze.png`) is used without any
further step; letter case does not matter. With SELECT ▸ Appearance ▸
Details set to Detailed, the shelf shows each title's id. Pocket Shelf reads
the `art/` folder at start, so restart it after copying, or open the
title's Box art picker, which reads the folder again.

**A backdrop is chosen per title.** Select the title, press △, open
**Backdrop** and pick the file. Box art works the same way under **Box
art**, for a file whose name does not match.

The pickers list the files named after the title: its name, the name it came
with, or its id, alone or followed by a dash and anything
(`gravity-daze-2.png`, `PCSA00069-night.png`). △ in a picker switches to
every file in the folder. **Default** goes back to the box art matched by
name (else the title's icon) or to the title's own backdrop; **None** shows
no backdrop, and **Game icon** shows the title's icon in place of matched
box art.

Files Pocket Shelf reads:

- PNG, at most 8 MB, at most 2,097,152 pixels (1920×1080 fits).
- Box art at least 16×16, in any shape; it is scaled to at most 256 pixels
  on its long side.
- Backdrops in the screen's shape look best: 960×544, or 1920×1088. A wider
  picture loses its sides and a taller one its top and bottom.
- File names of at most 96 characters, not starting with a dot.

A file that does not decode is marked in the picker and not used.
**SELECT ▸ Library ▸ Clean up artwork with Remove set to Unused deletes the
files no title uses**: a backdrop you copied but have not chosen for a title
counts as unused, and so does box art whose name matches no title.

## Controls

| Button | Action |
| --- | --- |
| D-pad | Move the selection |
| L / R | Previous / next tab |
| ○ or ✕ (your choice in the menu) | Launch / back |
| △ | Edit the selected title |
| □ | Search |
| SELECT | Menu: sort, appearance, library, updates, confirm button, language, diagnostics |

**Touch screen.** Everything the buttons do can also be done on the front
touch screen:

- Tap a title to select it; tap the selected title to start it.
- Drag along the row (Home, Carousel, Cascade) or up and down the Grid and
  the List to move through the titles; a flick keeps going and slows down.
- Tap a tab name, or L or R, to change tabs.
- Tap a button hint at the bottom of the screen (△ Edit, □ Search,
  SELECT Menu, and the hints in each menu) to do what it says.
- In a menu, tap a row to open or change it, or its ◀ ▶ arrows to step its
  value; drag to move through a long list; tap outside the menu to go back.
  In a box art or backdrop list, the first tap shows the picture and a
  second one uses it.
- Tap keys on the on-screen keyboard.

## Data

Everything Pocket Shelf saves is in `ux0:/data/PocketShelf/`:

| File | Holds |
| --- | --- |
| `settings.json` | Settings and the last selection |
| `status-bar.json` | Which status items show, and on which side |
| `text-size.json` | The text size |
| `sort.json` | Each tab's sort order |
| `plays.json` | How often and when Pocket Shelf started each title |
| `categories.json` | Your categories and their order |
| `titles/<title id>.json` | Your changes to one title |
| `titles.cache.json` | Every title's changes in one file, read at start |
| `recent.json` | The last 15 titles started |
| `art/`, `backdrops/` | Box art and backdrops, downloaded or your own (see [Your own box art and backdrops](#your-own-box-art-and-backdrops)) |
| `libretro/` | libretro's name lists, one per console, fetched once |
| `steamgriddb.txt` | Your SteamGridDB API key |
| `update.json`, `update/` | The last update check; a downloaded update |
| `titles.tsv` | The scanned title list. Delete it to force a new scan |

The log is `log.txt` in the same folder. At 256 KB it becomes `log.old.txt`
(replacing the previous one) and a new log starts. The updater app writes its
steps to `updater.log`.

## Troubleshooting

- **A title is missing:** run **SELECT ▸ Library ▸ Rescan titles**, then open
  **SELECT ▸ Diagnostics**. After the timing it lists what the scan found and
  why it skipped any PSP folder, disc image or RetroArch game (for example
  `skip=<id> noEboot`). These lines appear only after a scan in the same run.
- **A PSP or PS1 game does not start:** the end of Diagnostics shows the last
  Adrenaline launches (`Boot iso`, `Boot stale`, ...), the boot trigger and the
  ISO driver.
- **Something is slow or wrong:** `ux0:/data/PocketShelf/log.txt` records each
  frame over 25 ms with where its time went, each launch, backdrop decode
  times and the scan. Attach it to an issue.

## Building

You need [Rust via rustup](https://rustup.rs/), `make`,
[VitaSDK](https://vitasdk.org/) with libvita2d, and
[cargo-vita](https://github.com/vita-rust/cargo-vita) 0.2.2. The Vita crate
pins its nightly toolchain in [`vita/rust-toolchain.toml`](./vita/rust-toolchain.toml).

```sh
make test      # core, platform module and tools tests, on the desktop
make shots     # render every screen to .build/validation/desktop/
make build     # VPK at dist/vita/pocket-shelf.vpk
```

| Folder | Holds |
| --- | --- |
| [`core/`](./core/) | Everything but the platform: the library and saved files, the shelf's screens and panels, text, shapes, themes, artwork lookups, updates, the `Host` trait |
| [`vita/`](./vita/) | The Vita host, the vita2d backend and the frame loop; `vita/package/` holds the VPK's other files |
| [`desktop/`](./desktop/) | A software renderer and a fake host with sample titles, for `make shots` |
| [`platform-tests/`](./platform-tests/) | The Vita modules with no system calls, built for the desktop so their tests run there |
| [`updater/`](./updater/), [`psp-boot/`](./psp-boot/) | The updater app and the Adrenaline plugin (C) |
| [`tools/`](./tools/) | `make art` (the LiveArea pictures) and `make updater` (the updater's package) |
| [`docs/`](./docs/) | [Architecture](./docs/ARCHITECTURE.md), working [notes](./docs/NOTES.md), the port's [plan](./docs/PLAN.md) |

The texts in five languages are in [`core/src/locales.rs`](./core/src/locales.rs).
The version is `version` in [`vita/Cargo.toml`](./vita/Cargo.toml). The updater
app (`make updater`, needs VitaSDK) and the Adrenaline plugin (`make plugin`,
needs pspdev) are committed in `vita/package/`, so a build needs neither
toolchain.

### Releases

Three GitHub Actions workflows build and publish the VPK:

| Workflow | Does |
| --- | --- |
| [`shelf-build.yml`](./.github/workflows/shelf-build.yml) | Tests and builds the release VPK, on every push to `main` and when the other two call it; "Run workflow" builds without publishing |
| [`shelf-release.yml`](./.github/workflows/shelf-release.yml) | On a `shelf-v*` tag, publishes a beta or stable release |
| [`shelf-nightly.yml`](./.github/workflows/shelf-nightly.yml) | Replaces the `shelf-nightly` pre-release with a build of `main`, once a day when `main` changed |

The tag's version must be the `version` in `vita/Cargo.toml`:

```sh
git tag shelf-v0.2.0-beta.3   # beta: published as a pre-release
git tag shelf-v0.2.0          # stable: published as the latest release
git push origin <tag>
```

The nightly schedule is off until the repository variable `SHELF_NIGHTLY` is
set to `true` (Settings ▸ Secrets and variables ▸ Actions ▸ Variables).

Before tagging, test the update path on a Vita: build the current code with a
lower version, `GITHUB_REF_NAME=shelf-v<lower version> make build`, install it,
and let it update to the newest published release. Download, install and the
restart run on hardware only.

## Contributors

- [Fatih Doğan](https://github.com/fthdgn-gh): author and maintainer.
- [Claude](https://github.com/claude) ([Claude Code](https://claude.com/claude-code)),
  Anthropic's AI model: co-author of the code and documentation.

## Credits

- [PocketJS](https://github.com/pocket-stack/pocketjs) by Yifeng "Evan" Wang
  and Pocket Nexus: Pocket Shelf 0.1 was a PocketJS app, and its Vita platform
  modules began in the PocketJS Vita host.
- [SteamGridDB](https://www.steamgriddb.com/) for the artwork lookups.
- RetroFlow and VitaShell,
  for showing how to launch system applications and PlayStation Mobile titles,
  and how to lay out the LiveArea.

## License

[MIT](./LICENSE). Font: Figtree (SIL OFL 1.1), see
[`fonts/NOTICE.md`](./fonts/NOTICE.md). The licenses of the libraries built
into the app (Rust crates, the Rust standard library, vita2d, Figtree) are in
[`vita/package/THIRD_PARTY_NOTICES.txt`](./vita/package/THIRD_PARTY_NOTICES.txt),
which the VPK carries; `make notices` rewrites it.

The updater app in [`updater/`](./updater/) (`POCKTUPDR`, which installs
Pocket Shelf's updates) is a separate program under the
[GPL-3.0](./updater/LICENSE): it uses VitaShell's package installer code.
The VPK carries its build in `updater/`.
