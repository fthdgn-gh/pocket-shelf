#!/bin/sh
# Release VPK at dist/vita/pocket-shelf-next.vpk.
#
# cargo-vita signs a safe SELF (-s), and the Vita keeps a safe app out of
# ux0:app (Vita3K does not). The SELF is signed again here without -s, and
# compressed (-c), as tools/vita.ts does for Pocket Shelf.
set -e
cd "$(dirname "$0")"
export VITASDK="${VITASDK:-$HOME/vitasdk}"
export PATH="$VITASDK/bin:$HOME/.cargo/bin:$PATH"
# This build's version, channel and commit for the update check (build.rs).
bun ../../src/tools/build-info.ts
rustup run nightly-2026-05-28 cargo vita build vpk --release
out=target/armv7-sony-vita-newlibeabihf/release
vita-make-fself -c "$out/pocket-shelf-next.velf" "$out/eboot.bin"
mkdir -p ../../dist/vita
# Adrenaline launches copy the boot plugin from the app folder
# (pspemu::copy_plugin reads app0:psp/pocketshelf.prx). The updater app
# (src/vita/updater, built by `bun run shelf:updater`) goes in as updater/;
# platform::update installs it from app0:updater.
set -- -a ../../src/vita/psp/pocketshelf.prx=psp/pocketshelf.prx
for file in $(cd ../../src/vita/updater && find . -type f | sed 's|^\./||' | sort); do
  set -- "$@" -a "../../src/vita/updater/$file=updater/$file"
done
vita-pack-vpk -s "$out/pocket-shelf-next.sfo" -b "$out/eboot.bin" "$@" \
  ../../dist/vita/pocket-shelf-next.vpk
ls -l "$out/eboot.bin" ../../dist/vita/pocket-shelf-next.vpk
