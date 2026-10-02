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
- The Vita host support is in `hosts/vita/src/` behind three cargo features:
  `installed-apps`, `data-fs`, `http`.

## How we work

- Work on `main`. Commit and push only when asked. Commit messages: one short
  Conventional Commits line (`feat(shelf): ...`), no long body.
- After a change, build and open Vita3K so the user can look. The user also
  installs `dist/vita/pocket-shelf.vpk` on a real Vita; several features have
  only ever been confirmed there.
- State plainly what was verified in the preview, in Vita3K, and on hardware.

## Commands

| Command | Does |
|---|---|
| `bun run shelf:build` | Debug VPK to `dist/vita/pocket-shelf.vpk` |
| `bun run shelf:build:release` | Release VPK (about 2.6 MB against 25 MB debug) |
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
- **Mount only what is on screen.** Each view windows its list
  (`views/window.ts`) and places items by index. With every title mounted, a
  125-title category was 500 to 850 nodes; windowed it is about 100.

## Host facts (`hosts/vita/src/`)

- **Prefer host extras over new spec ops.** Extras are `ui.__name` functions
  outside the numbered contract (`__appArt`, `__appBackdrop`,
  `__appBackdropFree`, `__artAccent`, `__netGet`, `__netSave`, `__netState`,
  `__netText`, `__netClose`). The one spec op added, `appIcon`, collided with
  upstream at 52 and is now **57**.
- **Pictures decode on worker threads** (`jobs.rs`). Icons, custom art and
  backdrops answer `-2` until ready and the app asks again every two frames.
  Only the texture upload runs on the main thread.
- **Never free a texture that is still drawn.** It crashed the app. The host
  frees a backdrop only when the app says so; `art-files.ts` counts users per
  handle, keeps the three most recent idle ones, and frees others a few frames
  after their last use. Icon and art textures are never freed.
- **Retail games' files are encrypted.** For an icon or backdrop the host
  tries, in order: `ux0:/app/<id>/sce_sys` (plain for homebrew),
  `ur0:appmeta/<id>/` (exists only after the home screen opened that game's
  LiveArea), then `sceAppMgrGameDataMount` on `ux0:app/<id>`. All three are
  confirmed on hardware. `param.sfo` is readable without a mount.
- **A launch ends this process.** `installed::launch` sends
  `psgm:play?titleid=<id>`; `main.rs` presents that frame, then
  `installed::finish_launch` sends the request again after 10 ms and calls
  `sceKernelExitProcess(0)` (the sequence in VitaShell's updater). A process
  that stays alive gets the system's "close this application?" prompt, since
  one game-category title runs at a time. Confirmed on hardware: no prompt,
  and the "Launching" line shows before the switch.
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
- Downloads land in `art/` and `backdrops/` as `sgdb-<id>.png`. Candidates not
  chosen are deleted when the drawer closes.

## Data folder (`ux0:/data/PocketShelf/`)

`settings.json`, `categories.json`, `recent.json` (title ids, the one started
last first, at most 15), `titles/<title id>.json` (per-title overrides:
category, title, art, backdrop, favorite), `art/`, `backdrops/`,
`steamgriddb.txt`.

## Smart categories

"Last Played" (`smart-recent`) and "Favorites" (`smart-favorites`) are filled
by the launcher (`SMART_CATEGORIES` in `categories.ts`, `titlesOf` in
`state.ts`). A title keeps its own category; the editor's Category row skips
the smart ones. They show no tab while empty, sit before the other tabs until
the user reorders them, and can be hidden or moved in the category manager.
`launchSelected` writes `recent.json` after the host accepts the launch.
"Reset to defaults" keeps the favorite mark.

## Search

Square opens the keyboard (`openSearch` in `state.ts`); the field shows how
many titles the text finds. Done puts the results in a "Search" tab
(`smart-search`) in front of the others, in the current view. The tab exists
only while a term is set: the back button on it, or an empty term, closes it
and returns to the title selected before. On that tab the footer shows the
term before the position, in place of the "Menu" hint. Matching and order are in
`search.ts`. `persist` saves a selected result under its own category, since
the tab does not exist after a restart.

## Open items

- **Next planned work:** list PSP and PS1 Classics that install on the Vita,
  and PSP and PS1 games that run under Adrenaline. Adrenaline's game bubbles
  (`PSPEMU` + digits) are hidden from the list for this reason; Adrenaline
  itself (`PSPEMUCFW`) is kept. The filter is in `catalog.ts`, not the host.
- A clock in the header was skipped: the Vita's local time zone handling was
  not verified.
- The art and backdrop pickers accept a file the host cannot read; the title
  then keeps its own icon or shows no backdrop, with no message.
