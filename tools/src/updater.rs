//! The updater app's installable package in `vita/package/updater/`, which
//! Pocket Shelf's VPK carries as `app0:updater/`: the `eboot.bin` and
//! `param.sfo` that `updater/Makefile` builds, the bubble icon, and
//! `sce_sys/package/head.bin`, which the system's package installer needs.
//! `head.bin` is written as `make_head_bin` in `updater/main.c` writes it on
//! the console (the template and the fold of SHA-1 digests are VitaShell's,
//! GPL-3.0).

use std::fs;
use std::path::Path;

use sha1::{Digest, Sha1};

const TITLE_ID: &str = "POCKTUPDR";

/// VitaShell's `fpkg_hmac`: a SHA-1 of the data, its bytes folded into a
/// 64-byte block, and the first 16 bytes of that block's SHA-1.
fn fpkg_hmac(data: &[u8]) -> [u8; 16] {
    let sha1 = Sha1::digest(data);
    let mut buf = [0u8; 64];
    buf[0..8].copy_from_slice(&sha1[4..12]);
    buf[8..16].copy_from_slice(&sha1[4..12]);
    buf[16..20].copy_from_slice(&sha1[12..16]);
    buf[20] = sha1[16];
    buf[21] = sha1[1];
    buf[22] = sha1[2];
    buf[23] = sha1[3];
    buf.copy_within(16..24, 24);
    let out = Sha1::digest(buf);
    out[..16].try_into().unwrap()
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

/// The template `head.bin` with the title's content id and its three HMACs.
pub fn head_bin(template: &[u8], title_id: &str) -> Vec<u8> {
    let mut head = template.to_vec();
    head[0x30..0x30 + 48].fill(0);
    let content_id = format!("EP9000-{title_id}_00-0000000000000000");
    head[0x30..0x30 + content_id.len()].copy_from_slice(content_id.as_bytes());
    let length = u32_at(&head, 0xd0);
    let hmac = fpkg_hmac(&head[..length]);
    head[length..length + 16].copy_from_slice(&hmac);
    let (offset, length, out) = (u32_at(&head, 0x8), u32_at(&head, 0x10), u32_at(&head, 0xd4));
    let hmac = fpkg_hmac(&head[offset..offset + length - 64]);
    head[out..out + 16].copy_from_slice(&hmac);
    let length = u32_at(&head, 0xe8);
    let hmac = fpkg_hmac(&head[..length]);
    head[length..length + 16].copy_from_slice(&hmac);
    head
}

pub fn run(root: &Path) -> Result<(), String> {
    let build = root.join(".build/updater");
    let out = root.join("vita/package/updater");
    let read = |path: &Path| fs::read(path).map_err(|e| format!("{}: {e} (run `make -C updater` first)", path.display()));
    let eboot = read(&build.join("eboot.bin"))?;
    let sfo = read(&build.join("param.sfo"))?;
    let icon = read(&root.join("vita/package/sce_sys/icon0.png"))?;
    let head = head_bin(&read(&root.join("updater/head.bin"))?, TITLE_ID);
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(out.join("sce_sys/package")).map_err(|e| e.to_string())?;
    for (path, bytes) in [("eboot.bin", &eboot), ("sce_sys/param.sfo", &sfo), ("sce_sys/icon0.png", &icon), ("sce_sys/package/head.bin", &head)] {
        fs::write(out.join(path), bytes).map_err(|e| format!("{path}: {e}"))?;
    }
    println!("Pocket Shelf Updater ({TITLE_ID}) packaged in {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    /// The committed package's `head.bin` is what the template gives.
    #[test]
    fn head_bin_matches_the_committed_package() {
        let root = super::super::root();
        let template = std::fs::read(root.join("updater/head.bin")).unwrap();
        let committed = std::fs::read(root.join("vita/package/updater/sce_sys/package/head.bin")).unwrap();
        assert_eq!(super::head_bin(&template, super::TITLE_ID), committed);
    }
}
