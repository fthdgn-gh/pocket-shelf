# Pocket Shelf Updater

A small Vita app (title id `POCKTUPDR`) that installs a Pocket Shelf release.
Pocket Shelf cannot install itself: the system's package installer replaces
`ux0:app/POCKTSHLF`, which the running app is mounted from.

**License: GPL-3.0** ([LICENSE](./LICENSE)). This folder is a separate
program from Pocket Shelf, which stays under the MIT License. The two only
share files on disk and start each other.

## How an update runs

1. Pocket Shelf downloads the release VPK to
   `ux0:data/PocketShelf/update/pocket-shelf.vpk` and unpacks it into `update/pkg/`
   (`hosts/vita/src/update.rs`, `src/update-flow.ts`).
2. Pocket Shelf copies `app0:updater/` (this program's package, built into
   `src/vita/updater/`) to `update/helper/`, installs it with the package
   installer, starts it and exits.
3. The updater reads the release's version from `update/version.txt` (written
   by Pocket Shelf, shown on its screen), checks that `update/pkg/` is Pocket Shelf (`TITLE_ID` is
   `POCKTSHLF`), writes `sce_sys/package/head.bin`, installs the package,
   writes `update/result.txt` (`ok` or `error <step> <code>`) and
   starts Pocket Shelf.
4. Pocket Shelf shows the result, deletes `update/` and removes the updater
   app.

## Building

```sh
bun run shelf:updater   # make, then lay out src/vita/updater/
```

Needs VitaSDK with libvita2d. The output in `src/vita/updater/` is committed,
so a Pocket Shelf build does not need to build this program. Rebuild it after
changing anything here.

## Credits

- `makeHeadBin`, `fpkg_hmac`, the PAF loader, the install sequence and
  `head.bin` come from [VitaShell](https://github.com/TheOfficialFloW/VitaShell)
  (Copyright (C) 2015-2018, TheFloW, GPL-3.0), commit `81af709`.
- `sha1.c` and `sha1.h` are Brad Conte's SHA-1, as included in VitaShell.
