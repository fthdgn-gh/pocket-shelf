//! Pocket Shelf's build tools, run from the repository root:
//!
//!   cargo run --release --manifest-path tools/Cargo.toml -- art
//!   cargo run --release --manifest-path tools/Cargo.toml -- updater
//!   cargo run --release --manifest-path tools/Cargo.toml -- notices
//!   cargo run --release --manifest-path tools/Cargo.toml -- wikidata [out dir] [table...]
//!   cargo run --release --manifest-path tools/Cargo.toml -- wikidata-match [dir]
//!
//! (`make art`, `make updater`, `make notices`.) `art` draws the bubble
//! icon, the splash and the LiveArea into `vita/package/sce_sys/`; `updater`
//! lays out the updater app built by `updater/Makefile` in
//! `vita/package/updater/`; `notices` writes the third-party licenses the VPK
//! carries, `vita/package/THIRD_PARTY_NOTICES.txt`; `wikidata` builds the
//! game details tables the `gamedb-data` release publishes (`make
//! wikidata`), and `wikidata-match` reports how many sample games they find.

mod art;
mod manifest;
mod notices;
mod png;
mod updater;
mod wikidata;

use std::path::PathBuf;
use std::process::ExitCode;

/// The repository root: this crate's parent folder.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn main() -> ExitCode {
    let command = std::env::args().nth(1).unwrap_or_default();
    let args: Vec<String> = std::env::args().skip(2).collect();
    let result = match command.as_str() {
        "art" => art::run(&root()),
        "updater" => updater::run(&root()),
        "notices" => notices::run(&root()),
        "wikidata" => wikidata::build(&root(), &args),
        "wikidata-match" => wikidata::check(&root(), &args),
        _ => Err("usage: shelf-tools art | updater | notices | wikidata [out dir] [table...] | wikidata-match [dir]".to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
