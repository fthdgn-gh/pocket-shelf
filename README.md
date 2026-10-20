# Pocket Shelf

A launcher for the PS Vita. It lists the titles installed on the console
(Vita games, homebrew, PlayStation Mobile, PSP and PS1 games, and system
applications) and the games in your RetroArch playlists, in tabs. Each title
gets an icon and a backdrop, and it starts with one button press.

Pocket Shelf is written in TypeScript with Solid and runs on
[PocketJS](https://github.com/pocket-stack/pocketjs). This repository is a fork
of PocketJS: the app lives in [`src/`](./src/), and the rest is the framework
and its Vita host.

- Title id `POCKTSHLF`, app id `dev.fthdgn.pocket-shelf`
- Version 0.1.0

<p align="center">
  <img src="./src/screenshots/home.png" width="720" alt="Pocket Shelf's Home view on the Games tab in the Dynamic theme: the selected title's icon large at the left in a white ring, the next titles smaller after it, the title's name, platform and category, and a Launch button, over the title's backdrop." />
</p>

| Carousel | Cascade | Grid |
| --- | --- | --- |
| ![The carousel view: the selected title's icon enlarged in the middle of a row, its name below.](./src/screenshots/carousel.png) | ![The cascade view: the selected title's icon large at the left, the next titles shrinking and darkening to the right, its name below.](./src/screenshots/cascade.png) | ![The grid view: two rows of five title icons with the selected one in a white ring, its name below.](./src/screenshots/grid.png) |

| List | RetroArch | Menu |
| --- | --- | --- |
| ![The list view: title rows with small icons, the selected row highlighted.](./src/screenshots/list.png) | ![The Home view on a RetroArch console tab, SNES: a game from a RetroArch playlist, its console in the detail line.](./src/screenshots/retroarch.png) | ![The SELECT menu's Appearance page as a floating card: theme, view, details, backdrop, icon box and status bar.](./src/screenshots/menu.png) |

The screens are rendered by `bun run shelf:preview` with sample titles; the
colored tiles with initials stand in for game icons.

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
- **Status bar** with the clock, Wi-Fi, Bluetooth and battery.
- **Five languages:** English, Turkish, German, French and Spanish.
- **Updates from GitHub** on the channel you pick (see [Updates](#updates)).

## Install

1. Download `pocket-shelf.vpk` from the newest tagged release, or build it
   yourself (see [Building](#building)).

   [![Newest release](https://img.shields.io/github/v/release/fthdgn-gh/pocket-shelf?include_prereleases&sort=semver&label=newest%20release)](https://github.com/fthdgn-gh/pocket-shelf/releases)

   There is no stable release yet. The current one is the beta
   [`shelf-v0.1.0-beta.3`](https://github.com/fthdgn-gh/pocket-shelf/releases/tag/shelf-v0.1.0-beta.3)
   ([download the VPK](https://github.com/fthdgn-gh/pocket-shelf/releases/download/shelf-v0.1.0-beta.3/pocket-shelf.vpk)).
   GitHub marks betas as pre-releases, so its "Latest" label does not point to
   them. Releases come in three channels:
   - **Stable:** tested on hardware.
   - **Beta** (marked pre-release): new features, tested less.
   - **Nightly** (`shelf-nightly`, when available): the current `main`, untested.
2. Copy it to the Vita and install it with VitaShell.
3. Open Pocket Shelf from the home screen. On the first start it scans the
   installed titles and shows its progress. Later starts read the saved list.

After you install or remove a title, run **SELECT ▸ Library ▸ Rescan titles**.

### Updates

**SELECT ▸ Updates** sets the channel (Stable, Beta, Nightly or Off) and checks
for a newer release. Pocket Shelf also checks once a day at start. To install,
it downloads the VPK, then starts a separate app, Pocket Shelf Updater, which
installs it, opens the new version and is removed again.

### Adrenaline (PSP and PS1 games)

Both TheOfficialFloW's Adrenaline and isage's fork (8.x) are supported, but
only TheOfficialFloW's has been tested on hardware. The first time you start a
PSP or PS1 game, Pocket Shelf offers to add its plugin to Adrenaline's plugin
list. Nothing changes until you confirm. Adrenaline's own files and settings
are never modified.

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

## Controls

| Button | Action |
| --- | --- |
| D-pad | Move the selection |
| L / R | Previous / next tab |
| ○ or ✕ (your choice in the menu) | Launch / back |
| △ | Edit the selected title |
| □ | Search |
| SELECT | Menu: appearance, library, confirm button, language, diagnostics |

## Data

Everything Pocket Shelf saves is in `ux0:/data/PocketShelf/`:

| File | Holds |
| --- | --- |
| `settings.json` | Settings and the last selection |
| `categories.json` | Your categories and their order |
| `titles/<title id>.json` | Your changes to one title |
| `recent.json` | The last 15 titles started |
| `art/`, `backdrops/` | Downloaded and chosen pictures |
| `titles.tsv` | The scanned title list. Delete it to force a new scan |

## Building

You need [Bun](https://bun.sh/), [Rust via rustup](https://rustup.rs/), and
VitaSDK with the pinned Rust toolchain described in
[`hosts/vita/README.md`](./hosts/vita/README.md).

```sh
bun install
bun run shelf:build:release   # VPK at dist/vita/pocket-shelf.vpk
bun run shelf:run             # debug build, opened in Vita3K
bun run shelf:check           # manifest check and TypeScript
bun run shelf:test            # unit tests
bun run shelf:preview         # render screens to PNG without a Vita
```

Install the release build on hardware. The debug executable is 105 MB, and
the system reads it from the memory card on every launch.

`shelf:preview` runs the PocketJS engine compiled to WebAssembly. When
`hosts/web/pocketjs.wasm` is missing, the first run builds it from
`engine/wasm` with `cargo build --release --target wasm32-unknown-unknown`.
That crate pins no toolchain, so it builds with your default one (stable),
which needs Rust's WebAssembly target. Add it once:

```sh
rustup target add wasm32-unknown-unknown
```

Without it the build stops with "missing target? run: rustup target add
wasm32-unknown-unknown". Delete `hosts/web/pocketjs.wasm` to rebuild it after
the engine changes, for example after merging PocketJS: an existing file is
used as it is. The Vita host builds with its own pinned nightly toolchain
instead (see [`hosts/vita/README.md`](./hosts/vita/README.md)).

The Adrenaline plugin is committed as `src/vita/psp/pocketshelf.prx`. To
rebuild it after changing `src/psp-boot/`, run `bun run shelf:plugin`, which
needs pspdev.

Notes on the engine limits, host behavior, and what has been verified where are
in [`src/CLAUDE.md`](./src/CLAUDE.md).

### Releases

Three GitHub Actions workflows build and publish the VPK:

| Workflow | Does |
| --- | --- |
| [`shelf-build.yml`](./.github/workflows/shelf-build.yml) | Checks, tests and builds the release VPK. The other two call it; "Run workflow" builds without publishing |
| [`shelf-release.yml`](./.github/workflows/shelf-release.yml) | On a `shelf-v*` tag, publishes a beta or stable release |
| [`shelf-nightly.yml`](./.github/workflows/shelf-nightly.yml) | Replaces the `shelf-nightly` pre-release with a build of `main`, once a day when `main` changed |

The tag's version must be the `version` in `pocket.json`:

```sh
git tag shelf-v0.1.0-beta.2   # beta: published as a pre-release
git tag shelf-v0.1.0          # stable: published as the latest release
git push origin <tag>
```

The nightly schedule is off until the repository variable `SHELF_NIGHTLY` is
set to `true` (Settings ▸ Secrets and variables ▸ Actions ▸ Variables).

## Credits

- [PocketJS](https://github.com/pocket-stack/pocketjs) by Yifeng "Evan" Wang
  and Pocket Nexus: the framework, renderer and Vita host this app is built on.
- [SteamGridDB](https://www.steamgriddb.com/) for the artwork lookups.
- RetroFlow and VitaShell,
  for showing how to launch system applications and PlayStation Mobile titles,
  and how to lay out the LiveArea.

## License

[MIT](./LICENSE), the same as PocketJS. Fonts: Figtree (SIL OFL 1.1) and
Inter (SIL OFL, in [`assets/fonts/`](./assets/fonts/)). See
[`src/fonts/NOTICE.md`](./src/fonts/NOTICE.md).

The updater app in [`src/updater/`](./src/updater/) (`POCKTUPDR`, which
installs Pocket Shelf's updates) is a separate program under the
[GPL-3.0](./src/updater/LICENSE): it uses VitaShell's package installer code.
The VPK carries its build in `updater/`.
