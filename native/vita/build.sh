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
rustup run nightly-2026-05-28 cargo vita build vpk --release
out=target/armv7-sony-vita-newlibeabihf/release
vita-make-fself -c "$out/pocket-shelf-next.velf" "$out/eboot.bin"
mkdir -p ../../dist/vita
# Adrenaline launches copy the boot plugin from the app folder
# (pspemu::copy_plugin reads app0:psp/pocketshelf.prx).
vita-pack-vpk -s "$out/pocket-shelf-next.sfo" -b "$out/eboot.bin" \
  -a ../../src/vita/psp/pocketshelf.prx=psp/pocketshelf.prx \
  ../../dist/vita/pocket-shelf-next.vpk
ls -l "$out/eboot.bin" ../../dist/vita/pocket-shelf-next.vpk
