//! Incremental SHA-1 / SHA-256 hashing, the native equivalent of the C++
//! `Hasher`. One definition every consumer (downloader, cache) verifies through.

use proto::download::HashAlgorithm;
use sha1::Sha1;
use sha2::{Digest, Sha256};

pub enum Hasher {
    Sha1(Sha1),
    Sha256(Sha256),
}

impl Hasher {
    pub fn new(algorithm: HashAlgorithm) -> Self {
        match algorithm {
            HashAlgorithm::Sha1 => Hasher::Sha1(Sha1::new()),
            HashAlgorithm::Sha256 => Hasher::Sha256(Sha256::new()),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        match self {
            Hasher::Sha1(h) => h.update(data),
            Hasher::Sha256(h) => h.update(data),
        }
    }

    pub fn hex_digest(self) -> String {
        match self {
            Hasher::Sha1(h) => hex(&h.finalize()),
            Hasher::Sha256(h) => hex(&h.finalize()),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
