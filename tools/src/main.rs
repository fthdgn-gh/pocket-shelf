//! Pocket Shelf's build tools, run from the repository root:
//!
//!   cargo run --release --manifest-path tools/Cargo.toml -- art
//!   cargo run --release --manifest-path tools/Cargo.toml -- updater
//!   cargo run --release --manifest-path tools/Cargo.toml -- notices
//!
//! (`make art`, `make updater`, `make notices`.) `art` draws the bubble
//! icon, the splash and the LiveArea into `vita/package/sce_sys/`; `updater`
//! lays out the updater app built by `updater/Makefile` in
//! `vita/package/updater/`; `notices` writes the third-party licenses the VPK
//! carries, `vita/package/THIRD_PARTY_NOTICES.txt`.

mod art;
mod manifest;
mod notices;
mod png;
mod updater;

use std::path::PathBuf;
use std::process::ExitCode;

/// The repository root: this crate's parent folder.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn main() -> ExitCode {
    let command = std::env::args().nth(1).unwrap_or_default();
    let result = match command.as_str() {
        "art" => art::run(&root()),
        "updater" => updater::run(&root()),
        "notices" => notices::run(&root()),
        _ => Err("usage: shelf-tools art | updater | notices".to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
