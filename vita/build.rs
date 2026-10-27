//! This build's identity for the update check: `build-info.json` at the
//! repository root, written by `tools/build-info.ts` (`build.sh` runs it first). Without
//! the file the build is local, which checks no channel. Also this build's
//! title id, from `Cargo.toml`'s `[package.metadata.vita]` (cargo-vita does
//! not pass it to the compiler; PocketJS's tools/vita.ts did, as
//! `VITA_DEFAULT_TITLE_ID`).

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap_or_default();
    let title_id = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("title_id = \"").and_then(|rest| rest.strip_suffix('"')))
        .unwrap_or("");
    assert!(!title_id.is_empty(), "no title_id in Cargo.toml");
    println!("cargo:rustc-env=SHELF_TITLE_ID={title_id}");

    let path = "../build-info.json";
    println!("cargo:rerun-if-changed={path}");
    let text = std::fs::read_to_string(path).unwrap_or_default();
    // Three flat string fields; no JSON parser needed at build time.
    let field = |name: &str| -> String {
        let key = format!("\"{name}\"");
        text.split_once(&key)
            .and_then(|(_, rest)| rest.split_once(':'))
            .and_then(|(_, rest)| rest.split('"').nth(1))
            .unwrap_or("")
            .to_string()
    };
    let (version, channel, commit) = (field("version"), field("channel"), field("commit"));
    println!("cargo:rustc-env=SHELF_VERSION={}", if version.is_empty() { "0.0.0" } else { &version });
    println!("cargo:rustc-env=SHELF_CHANNEL={}", if channel.is_empty() { "local" } else { &channel });
    println!("cargo:rustc-env=SHELF_COMMIT={commit}");
}
