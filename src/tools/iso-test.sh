#!/bin/sh
# Run the disc-image tests in hosts/vita/src/iso.rs on this machine. The file
# needs miniz_oxide, so it is built inside a throwaway crate that includes it
# by path; cargo takes the crate from its cache when it can.
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
mkdir -p "$dir/src"
cat > "$dir/Cargo.toml" <<TOML
[package]
name = "iso-test"
version = "0.0.0"
edition = "2021"

[dependencies]
miniz_oxide = "0.8"
TOML
echo "#[path = \"$root/hosts/vita/src/iso.rs\"] pub mod iso;" > "$dir/src/lib.rs"
cargo test --manifest-path "$dir/Cargo.toml" --quiet
