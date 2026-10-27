//! This build's identity, compiled in for the update check
//! (`core/src/updates.rs`):
//!
//!   version  from a `shelf-v<version>` tag (CI sets `GITHUB_REF_NAME`),
//!            else this crate's `version`
//!   channel  "stable" or "beta" from the tag, "nightly" for any other CI
//!            build, "local" on a developer's machine
//!   commit   `GITHUB_SHA`, else `git rev-parse HEAD`; "" when unknown
//!
//! Also this build's title id, from `Cargo.toml`'s `[package.metadata.vita]`
//! (cargo-vita does not pass it to the compiler; PocketJS's tools/vita.ts
//! did, as `VITA_DEFAULT_TITLE_ID`).

/// The version in `shelf-v0.2.0` or `shelf-v0.2.0-beta.1`.
fn tag_version(tag: &str) -> Option<&str> {
    let version = tag.strip_prefix("shelf-v")?;
    let (core, beta) = match version.split_once("-beta.") {
        Some((core, beta)) => (core, Some(beta)),
        None => (version, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let parts: Vec<&str> = core.split('.').collect();
    (parts.len() == 3 && parts.iter().all(|p| digits(p)) && beta.map_or(true, digits)).then_some(version)
}

fn is_commit(text: &str) -> bool {
    text.len() == 40 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap_or_default();
    let title_id = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("title_id = \"").and_then(|rest| rest.strip_suffix('"')))
        .unwrap_or("");
    assert!(!title_id.is_empty(), "no title_id in Cargo.toml");
    println!("cargo:rustc-env=SHELF_TITLE_ID={title_id}");

    for name in ["GITHUB_REF_NAME", "GITHUB_ACTIONS", "GITHUB_SHA"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    // A new commit changes the commit compiled in: the branch HEAD names.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    if let Some(branch) = std::fs::read_to_string("../.git/HEAD").ok().and_then(|h| h.trim().strip_prefix("ref: ").map(String::from)) {
        println!("cargo:rerun-if-changed=../.git/{branch}");
    }

    let tag = std::env::var("GITHUB_REF_NAME").unwrap_or_default();
    let tagged = tag_version(&tag);
    let version = tagged.map(String::from).unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap());
    let channel = match tagged {
        Some(v) if v.contains("-beta.") => "beta",
        Some(_) => "stable",
        None if std::env::var_os("GITHUB_ACTIONS").is_some() => "nightly",
        None => "local",
    };
    let commit = std::env::var("GITHUB_SHA")
        .ok()
        .filter(|c| is_commit(c))
        .or_else(|| {
            let out = std::process::Command::new("git").args(["rev-parse", "HEAD"]).output().ok()?;
            Some(String::from_utf8(out.stdout).ok()?.trim().to_string()).filter(|c| is_commit(c))
        })
        .unwrap_or_default();
    println!("cargo:rustc-env=SHELF_VERSION={version}");
    println!("cargo:rustc-env=SHELF_CHANNEL={channel}");
    println!("cargo:rustc-env=SHELF_COMMIT={commit}");
    println!("cargo:warning=Pocket Shelf build {version} ({channel}{})", if commit.is_empty() { String::new() } else { format!(", {}", &commit[..7]) });
}
