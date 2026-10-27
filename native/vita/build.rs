//! This build's identity for the update check: `src/build-info.json`,
//! written by `src/tools/build-info.ts` (`build.sh` runs it first). Without
//! the file the build is local, which checks no channel.

fn main() {
    let path = "../../src/build-info.json";
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
