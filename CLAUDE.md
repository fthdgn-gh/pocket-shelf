# Repository Instructions

Pocket Shelf is a PS Vita launcher written in Rust (`POCKTSHLF`, released as 0.2.0-beta.1). Read [`docs/NOTES.md`](docs/NOTES.md) first: what the hardware, the firmware, Adrenaline and RetroArch do, what has been verified where, and how to test. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) describes the crates and the drawing model; [`docs/PLAN.md`](docs/PLAN.md) records the port from PocketJS and lists the open follow-ups.

## Working here

- `make test` (core, platform module and tools tests), `make shots` (desktop renders to `.build/validation/desktop/`, `SHOTS="words"` for some), `make build` (VPK at `dist/vita/pocket-shelf.vpk`); `make art`, `make updater` and `make plugin` regenerate committed files in `vita/package/`. The version is `version` in `vita/Cargo.toml`. Texts live in `core/src/locales.rs`.
- **Verify on the desktop before handing over a build**: `make test`, then `make shots` and look at the renders of the screens you changed (a fake host stands in for the Vita, `desktop/src/fake.rs`). Add a shot for a new screen or state.
- **The user tests on a real Vita** and sends the app's log, `ux0:data/PocketShelf/log.txt`, which they copy to `~/Downloads/log.txt`. Check its date against the build's before reading it: an old log has misled a diagnosis before. The log lists each frame over 25 ms with where its time went (update, draw, uploads), the library rebuilds, backdrop decode times and each launch step.
- Commit on local `main`; no branch or pull request per change. Run `make test` (and `make build` for Vita changes) before committing. Push only when the user asks or a GitHub Actions build is needed; a push runs the CI build.
- Name commits using the Conventional Commits format — `type(scope): summary`, e.g. `feat(shelf): …`, `fix: …`, `docs: …`, `refactor: …`.
- Ask before anything that cannot be taken back on GitHub: a force-push, a history rewrite, a release tag. History was rewritten on 2026-10-28 to Pocket Shelf's own commits; the old history is in `../pocket-shelf-history-2026-10-28.bundle`.

## Rules learned the hard way

- **The Vita's CPU runs this code about 40–50 times slower than a Mac.** Work that takes 12 ms on the desktop took 690 ms in a frame on hardware (parsing libretro's 9,301 PlayStation names). Anything that walks the whole library (5,905 titles on the user's Vita), parses a file or reads a folder of thousands of files does not belong in a frame: put it on a host thread and answer with an `Event`, or work it out once and keep it (one title's change recomputes that title alone, `Library::patch`).
- **No file I/O in the frame.** A `settings.json` write took 24–30 ms on hardware; writes go through the host's file thread (`FileOp`, `Host::write`), and `Host::flush` empties it before a launch ends the process.
- **Do not read back a file the file thread may still be writing.** A box art used from Find artwork vanished from its tile: the tile read the file before the write landed and cached "no picture". Hand over what is already in memory (the decoded preview texture) instead.
- **Nothing may run between a launch request and the process exit**, or the system shows "close this application?". Save everything first; `installed::finish_launch` sends the request and exits.
- **vita2d's stencil clip flickered on hardware**; clips are applied on the CPU (`Gfx::clip`). Do not add backend clipping.
- **Saved data stays compatible** across releases: read and write each file in the format the previous release used (`core/src/library/round_trip.rs` and its fixtures check it). New settings go in a file of their own rather than new keys in a shared one (`status-bar.json`). A breaking change waits for 1.0.0 and is called out.
- **A host value that is never read is a missing feature.** The port reported Wi-Fi as `None` from the start and the symbol never showed; Diagnostics lost its scan report while the Vita functions that make it still existed. The Vita crate's "function is never used" warnings point at such gaps; check them after porting or moving code.
- **cargo-vita does not pass the title id to the compiler.** `vita/build.rs` reads it from `Cargo.toml` (`SHELF_TITLE_ID`); without it 0.2.0-beta.1's first build listed itself, refused to download updates and could not pass its own package check.
- **`vita-elf-create: ... segment 1 overlaps`**: resize `LINK_PAD` in `vita/src/main.rs` by a few KiB. About one change in twenty triggers it.
- **Every text is in five languages.** A new text goes in `Messages`, each language constant and `Messages::texts` (the tests check all three); keep the keyboard's and the texts' characters inside Figtree (`fonts/`).

## Releases and updates

- Release tags are `shelf-v<version>` and must match `version` in `vita/Cargo.toml`; installed copies' update check reads only that form (`shelf-v0.2.0-beta.1`, not `pocket-shelf-v…`). The release VPK keeps the name `pocket-shelf.vpk` and the title id `POCKTSHLF`; the updater app installs and starts `POCKTSHLF` only.
- **Test the update path before a release**: build with a lower version, `GITHUB_REF_NAME=shelf-v<lower version> make build`, install it on the Vita and let it update to the published release. Download, unpack, the updater's install and its restart all run on hardware only (Vita3K has no package installer). The updater logs its steps to `ux0:data/PocketShelf/updater.log`.
- **A draft release is invisible to the update check**, so turning a release into a draft takes it down. To publish a tag again, delete the release and the tag first: the release workflow cannot create a release that already exists, even as a draft. A version number reused this way is not offered to consoles that installed the first build.
- **Each build's VPK is attested** (`actions/attest-build-provenance` in `shelf-build.yml`): `gh attestation verify pocket-shelf.vpk -R fthdgn-gh/pocket-shelf` checks a download. A workflow that calls `shelf-build.yml` has to grant `id-token: write` and `attestations: write`.
- **CI caches** (`.github/workflows/shelf-build.yml`): the build runs on pushes to `main` because tags and other branches can restore only `main`'s cache. The pinned nightly is cached with its files' times (installed fresh, its standard library looked newer than every cached build and cargo rebuilt everything), and the rust-cache key leaves out the runner's own Rust (its hash differs between runner images).

## Repository rules

- Keep per-run validation screenshots, videos, raw logs, traces, benchmark dumps, and build/install receipts in ignored `.build/validation/<task>/<run>/` output or an artifact store. Device validation does not require committing these files to Git.
- Put reproducible commands, results, build identities, and acceptance limits in the commit message; do not add a new source directory for each validation run.
- Commit an image or recording when a test consumes it as a maintained fixture, or when it is an intentional product/documentation asset with an identified consumer (the README's `screenshots/`). Temporary debugging output and historical screenshots are not test fixtures merely because they are called evidence.
- Before staging, inspect the file list and remove unintended validation artifacts. Preserve needed originals outside Git, and remove stale documentation links when cleaning up generated records. Do not infer a requirement to commit artifacts from a previous session's actions or a memory summary; apply the user's current instructions and these repository rules.
- Never commit or print the user's SteamGridDB API key (`steamgriddb.txt`).
- Documentation prose (`docs/`, `README.md`) states the mechanism directly and bolds concrete engineering facts, never slogans. No meta-framing, no imported architecture jargon ("vertical slice", "first-class citizen"), no personification or dramatic one-liners, no empty intensifiers ("simply", "elegant", "magic"). No adverbs modifying a verb or adjective ("simply", "just", "actually", "typically", "carefully", "silently", "properly") — delete the adverb, or replace it with the fact it was standing in for; prepositional phrases that carry a mechanism ("once per frame", "at the down edge") are facts, not adverbs, and stay.
