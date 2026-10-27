//! `vita/package/THIRD_PARTY_NOTICES.txt`, which the VPK carries: the
//! licenses of everything compiled into Pocket Shelf's executable. MIT, BSD,
//! ISC and Apache-2.0 ask for their notices to travel with the program.
//!
//! - the Rust crates of the Vita build, from `cargo metadata` and each
//!   crate's own license files in Cargo's registry;
//! - Rust's standard library, built from source for the Vita, and the crates
//!   it uses (from the toolchain's `library/Cargo.lock`);
//! - vita2d (`tools/notices/vita2d.txt`), the only C library whose code is
//!   linked in (libpng, libjpeg and zlib, which it also names, contribute no
//!   code: pictures are decoded by the Rust `png` crate);
//! - Figtree, the font built into the executable (`fonts/OFL-Figtree.txt`).
//!
//! The updater app in the VPK is a separate program under the GPL-3.0; its
//! license travels in `updater/LICENSE.txt` (`make updater`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const TARGET: &str = "armv7-sony-vita-newlibeabihf";

struct Crate {
    name: String,
    version: String,
    license: String,
    repository: String,
    /// The crate's license files: name and text.
    files: Vec<(String, String)>,
}

/// `LICENSE*`, `LICENCE*`, `COPYING*`, `COPYRIGHT*`, `NOTICE*` in a crate's
/// folder, by name.
fn license_files(dir: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<(String, String)> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let upper = name.to_uppercase();
            let wanted = ["LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE", "UNLICENSE"].iter().any(|p| upper.starts_with(p));
            let text = wanted.then(|| fs::read_to_string(e.path()).ok()).flatten()?;
            Some((name, text.trim_end().to_string()))
        })
        .collect();
    files.sort();
    files
}

/// The registry crates the Vita build links, from `cargo metadata`.
fn crates(root: &Path) -> Result<Vec<Crate>, String> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--filter-platform", TARGET, "--manifest-path"])
        .arg(root.join("vita/Cargo.toml"))
        .output()
        .map_err(|e| format!("cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!("cargo metadata: {}", String::from_utf8_lossy(&out.stderr)));
    }
    let meta: Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    // The crates the executable links: the root's normal dependencies, and
    // theirs; build scripts' and macros' own dependencies run at build time
    // only and are left out.
    let nodes: std::collections::HashMap<&str, &Value> = meta["resolve"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| Some((n["id"].as_str()?, n)))
        .collect();
    // Procedural macros run inside the compiler: neither they nor what they
    // use reach the executable.
    let macros: std::collections::HashSet<&str> = meta["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["targets"].as_array().into_iter().flatten().any(|t| t["kind"].as_array().into_iter().flatten().any(|k| k == "proc-macro")))
        .filter_map(|p| p["id"].as_str())
        .collect();
    let mut resolved: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut queue: Vec<&str> = meta["resolve"]["root"].as_str().into_iter().collect();
    while let Some(id) = queue.pop() {
        if macros.contains(id) || !resolved.insert(id) {
            continue;
        }
        for dep in nodes.get(id).and_then(|n| n["deps"].as_array()).into_iter().flatten() {
            let normal = dep["dep_kinds"].as_array().into_iter().flatten().any(|k| k["kind"].is_null());
            if let (true, Some(pkg)) = (normal, dep["pkg"].as_str()) {
                queue.push(pkg);
            }
        }
    }
    let mut found = Vec::new();
    for p in meta["packages"].as_array().into_iter().flatten() {
        // Registry crates only; the workspace's own crates are Pocket Shelf.
        if p["source"].is_null() || !resolved.contains(p["id"].as_str().unwrap_or("")) {
            continue;
        }
        let text = |key: &str| p[key].as_str().unwrap_or("").to_string();
        let dir = PathBuf::from(text("manifest_path")).parent().map(Path::to_path_buf).unwrap_or_default();
        found.push(Crate {
            name: text("name"),
            version: text("version"),
            license: text("license"),
            repository: text("repository"),
            files: license_files(&dir),
        });
    }
    found.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    found.dedup_by(|a, b| a.name == b.name && a.version == b.version);
    Ok(found)
}

/// The toolchain's sources: `<sysroot>/lib/rustlib/src/rust`, the nightly the
/// Vita crate pins.
fn rust_src(root: &Path) -> Option<PathBuf> {
    let out = Command::new("rustc").current_dir(root.join("vita")).args(["--print", "sysroot"]).output().ok()?;
    let sysroot = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    Some(sysroot)
}

/// The crates the standard library uses, from its `Cargo.lock` (registry
/// packages only).
fn std_crates(sysroot: &Path) -> Vec<String> {
    let lock = fs::read_to_string(sysroot.join("lib/rustlib/src/rust/library/Cargo.lock")).unwrap_or_default();
    let mut names = Vec::new();
    for block in lock.split("[[package]]").skip(1) {
        let field = |key: &str| block.lines().find_map(|l| l.strip_prefix(key).map(|v| v.trim().trim_matches('"').to_string()));
        if field("source = ").is_some_and(|s| s.starts_with("registry")) {
            if let (Some(name), Some(version)) = (field("name = "), field("version = ")) {
                names.push(format!("{name} {version}"));
            }
        }
    }
    names
}

fn rule(title: &str) -> String {
    format!("\n{}\n{title}\n{}\n\n", "=".repeat(78), "=".repeat(78))
}

pub fn run(root: &Path) -> Result<(), String> {
    let crates = crates(root)?;
    let sysroot = rust_src(root).ok_or("rustc --print sysroot failed")?;
    let licenses = sysroot.join("share/doc/rust/licenses");
    let read = |path: &Path| fs::read_to_string(path).map(|t| t.trim_end().to_string()).map_err(|e| format!("{}: {e}", path.display()));

    let mut out = String::new();
    out.push_str(
        "Pocket Shelf: third-party notices\n\n\
         Pocket Shelf is MIT-licensed (https://github.com/fthdgn-gh/pocket-shelf).\n\
         Its executable includes the software below, under the licenses that follow.\n\
         The updater app in updater/ is a separate program under the GPL-3.0\n\
         (updater/LICENSE.txt; source: https://github.com/fthdgn-gh/pocket-shelf/tree/main/updater).\n\
         Generated by `make notices` (tools/src/notices.rs).\n",
    );

    out.push_str(&rule("Contents"));
    out.push_str("Rust standard library (core, alloc, std) - MIT OR Apache-2.0 - https://github.com/rust-lang/rust\n");
    out.push_str("vita2d - MIT - https://github.com/xerpi/libvita2d\n");
    out.push_str("Figtree font - OFL-1.1 - https://github.com/erikdkennedy/figtree\n");
    out.push_str("C runtime (newlib and the VitaSDK libraries) - BSD-style licenses - https://github.com/vitasdk\n");
    for c in &crates {
        out.push_str(&format!("{} {} - {} - {}\n", c.name, c.version, c.license, c.repository));
    }

    out.push_str(&rule("Rust standard library"));
    out.push_str(
        "The standard library is built from source for the Vita. It and the crates it\n\
         uses are licensed under MIT OR Apache-2.0 (compiler_builtins under Apache-2.0\n\
         WITH LLVM-exception); copyright the Rust Project Developers and the authors of\n\
         each crate. Crates: ",
    );
    out.push_str(&std_crates(&sysroot).join(", "));
    out.push_str(".\n\n");
    out.push_str(&read(&licenses.join("MIT.txt"))?);
    out.push_str("\n\n");
    out.push_str(&read(&licenses.join("Apache-2.0.txt"))?);
    out.push('\n');

    out.push_str(&rule("vita2d"));
    out.push_str(&read(&root.join("tools/notices/vita2d.txt"))?);
    out.push('\n');

    out.push_str(&rule("Figtree font"));
    out.push_str(&read(&root.join("fonts/OFL-Figtree.txt"))?);
    out.push('\n');

    out.push_str(&rule("C runtime"));
    out.push_str(
        "The executable links the C runtime of VitaSDK (https://github.com/vitasdk): newlib\n\
         (https://sourceware.org/newlib/, its files under BSD-style licenses, see\n\
         https://sourceware.org/newlib/COPYING.NEWLIB) and the VitaSDK system library stubs.\n",
    );

    // Each license text once, with the crates that carry it.
    out.push_str(&rule("Rust crates"));
    let mut texts: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut without = Vec::new();
    for c in &crates {
        if c.files.is_empty() {
            without.push(format!("{} {} ({})", c.name, c.version, c.license));
        }
        for (file, text) in &c.files {
            texts.entry(text.clone()).or_default().push(format!("{} {} ({file})", c.name, c.version));
        }
    }
    let mut blocks: Vec<(Vec<String>, String)> = texts.into_iter().map(|(text, users)| (users, text)).collect();
    blocks.sort();
    for (users, text) in blocks {
        out.push_str(&format!("--- {}\n\n{text}\n\n", users.join(", ")));
    }
    if !without.is_empty() {
        out.push_str(&format!(
            "--- No license file in the published crate; licensed as stated: {}\n",
            without.join(", ")
        ));
    }

    let path = root.join("vita/package/THIRD_PARTY_NOTICES.txt");
    fs::write(&path, out.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("THIRD_PARTY_NOTICES.txt: {} crates, {} KB", crates.len(), out.len() / 1024);
    Ok(())
}
