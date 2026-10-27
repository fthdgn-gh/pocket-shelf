#!/bin/sh
# Release VPK at dist/vita/pocket-shelf.vpk (the name releases attach and
# the update check looks for).
#
# cargo-vita signs a safe SELF (-s), and the Vita keeps a safe app out of
# ux0:app (Vita3K does not). The SELF is signed again here without -s, and
# compressed (-c): the system reads the whole SELF from the memory card at
# every launch.
set -e
cd "$(dirname "$0")"
export VITASDK="${VITASDK:-$HOME/vitasdk}"
export PATH="$VITASDK/bin:$HOME/.cargo/bin:$PATH"
# This build's version, channel and commit for the update check (build.rs):
# the version from a shelf-v<version> tag (GITHUB_REF_NAME), else package.json's.
bun ../tools/build-info.ts
rustup run nightly-2026-05-28 cargo vita build vpk --release
out=target/armv7-sony-vita-newlibeabihf/release
vita-make-fself -c "$out/pocket-shelf-next.velf" "$out/eboot.bin"

# param.sfo as tools/vita.ts writes it for the PocketJS build: ATTRIBUTE2=12,
# and APP_VER, the version the system shows, as MM.mm of package.json's
# major and minor version ("0.2.0" is "00.02").
title_id=$(sed -n 's/^title_id = "\(.*\)"/\1/p' Cargo.toml)
title_name=$(sed -n 's/^title_name = "\(.*\)"/\1/p' Cargo.toml)
version=$(sed -n 's/.*"version": "\([0-9]*\)\.\([0-9]*\)\..*/\1 \2/p' ../package.json)
app_ver=$(printf '%02d.%02d' $version)
vita-mksfoex -d ATTRIBUTE2=12 -s TITLE_ID="$title_id" -s APP_VER="$app_ver" "$title_name" "$out/param.sfo"

mkdir -p ../dist/vita
# The bubble icon, splash and LiveArea (vita/package/sce_sys, drawn
# by `bun run art`). Adrenaline launches copy the boot plugin from the app
# folder (pspemu::copy_plugin reads app0:psp/pocketshelf.prx). The updater
# app (vita/package/updater, built by `bun run updater`) goes in as
# updater/; platform::update installs it from app0:updater.
set --
for dir in sce_sys psp updater; do
  for file in $(cd package/$dir && find . -type f ! -name '.*' | sed 's|^\./||' | sort); do
    set -- "$@" -a "package/$dir/$file=$dir/$file"
  done
done
vita-pack-vpk -s "$out/param.sfo" -b "$out/eboot.bin" "$@" ../dist/vita/pocket-shelf.vpk
echo "$title_id $title_name APP_VER $app_ver"
ls -l "$out/eboot.bin" ../dist/vita/pocket-shelf.vpk
