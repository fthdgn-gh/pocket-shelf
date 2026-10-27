//! The app's name, author and version, from `vita/Cargo.toml`: `title_name`
//! under `[package.metadata.vita]`, `authors` and `version` under
//! `[package]`.

use std::path::Path;

pub struct Manifest {
    pub title: String,
    pub author: Option<String>,
    pub version: String,
}

/// The quoted value of the first `key = "..."` line (a list's first item
/// for `authors = ["..."]`).
fn value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(key)?.trim_start().strip_prefix('=')?;
        rest.split('"').nth(1).map(String::from)
    })
}

pub fn read(root: &Path) -> Result<Manifest, String> {
    let path = root.join("vita/Cargo.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Manifest {
        title: value(&text, "title_name").ok_or("no title_name in vita/Cargo.toml")?,
        author: value(&text, "authors"),
        version: value(&text, "version").ok_or("no version in vita/Cargo.toml")?,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn values_are_read_from_their_lines() {
        let text = "[package]\nname = \"x\"\nversion = \"0.2.0\"\nauthors = [\"fthdgn\"]\n\n[package.metadata.vita]\ntitle_name = \"Pocket Shelf\"\n";
        assert_eq!(super::value(text, "version").as_deref(), Some("0.2.0"));
        assert_eq!(super::value(text, "authors").as_deref(), Some("fthdgn"));
        assert_eq!(super::value(text, "title_name").as_deref(), Some("Pocket Shelf"));
        assert_eq!(super::value(text, "missing"), None);
    }
}
