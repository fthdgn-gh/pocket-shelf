#!/bin/sh
# Run the RetroArch playlist tests in hosts/vita/src/retroarch.rs on this
# machine. The file needs serde_json and miniz_oxide, so it is built inside a
# throwaway crate that includes it by path; cargo takes the crates from its
# cache when it can. Playlists in .samples/retroarch-playlists (not in git), when
# there, are read too.
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
mkdir -p "$dir/src"
cat > "$dir/Cargo.toml" <<TOML
[package]
name = "retroarch-test"
version = "0.0.0"
edition = "2021"

[dependencies]
serde_json = "1"
miniz_oxide = "0.8"
TOML
echo "#[path = \"$root/hosts/vita/src/retroarch.rs\"] pub mod retroarch;" > "$dir/src/lib.rs"
if [ -d "$root/.samples/retroarch-playlists" ]; then
  export RETROARCH_SAMPLES="$root/.samples/retroarch-playlists"
fi
cargo test --manifest-path "$dir/Cargo.toml" --quiet
