//! A word-sized hasher (rustc's FxHash) for the library's maps. The default
//! SipHash works in 64-bit arithmetic, which the Vita's 32-bit CPU does in
//! several instructions per step; the keys here are title ids, titles and
//! file names from the user's own data, not from a network.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

const SEED: usize = if usize::BITS == 64 { 0x51_7c_c1_b7_27_22_0a_95u64 as usize } else { 0x9e37_79b9 };

#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    hash: usize,
}

impl FxHasher {
    fn add(&mut self, word: usize) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        const W: usize = std::mem::size_of::<usize>();
        let mut chunks = bytes.chunks_exact(W);
        for chunk in &mut chunks {
            self.add(usize::from_le_bytes(chunk.try_into().expect("word")));
        }
        for &byte in chunks.remainder() {
            self.add(byte as usize);
        }
    }

    fn write_u8(&mut self, n: u8) {
        self.add(n as usize);
    }

    fn write_usize(&mut self, n: usize) {
        self.add(n);
    }

    fn finish(&self) -> u64 {
        self.hash as u64
    }
}

pub type FxBuild = BuildHasherDefault<FxHasher>;
pub type FxHashMap<K, V> = HashMap<K, V, FxBuild>;
