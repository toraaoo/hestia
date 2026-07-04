//! Xbox request signing: an ECDSA P-256 proof key and the FILETIME-stamped
//! `Signature` header. One cross-platform implementation (`p256`) — the C++ tree
//! needed an OpenSSL/CNG split; Rust does not.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use rand_core::{OsRng, RngCore};

/// A per-login ECDSA P-256 proof key. Its public point (x, y) is advertised in
/// the Xbox proof-of-possession JWK; `sign` produces the raw 64-byte (r‖s)
/// signature the `Signature` header carries.
pub struct ProofKey {
    signing_key: SigningKey,
    id: String,
    x: String,
    y: String,
}

impl ProofKey {
    pub fn generate() -> ProofKey {
        let signing_key = SigningKey::random(&mut OsRng);
        let point = signing_key.verifying_key().to_encoded_point(false);
        let x = point.x().expect("P-256 public point has an x coordinate");
        let y = point.y().expect("P-256 public point has a y coordinate");
        ProofKey {
            id: format_uuid_v4(&random_bytes(16)),
            x: base64url_nopad(x),
            y: base64url_nopad(y),
            signing_key,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn x(&self) -> &str {
        &self.x
    }
    pub fn y(&self) -> &str {
        &self.y
    }

    /// ECDSA/SHA-256 over `message`, serialized as fixed-width raw r‖s (64 bytes).
    /// Xbox verifies against the advertised proof key, so a fresh (randomized or
    /// deterministic) signature is accepted — byte-identity with any other signer
    /// is neither required nor possible.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature: Signature = self.signing_key.sign(message);
        signature.to_bytes().to_vec()
    }
}

pub fn random_bytes(count: usize) -> Vec<u8> {
    let mut out = vec![0u8; count];
    OsRng.fill_bytes(&mut out);
    out
}

pub fn format_uuid_v4(bytes: &[u8]) -> String {
    assert_eq!(bytes.len(), 16, "a UUID needs exactly 16 bytes");
    let mut b = [0u8; 16];
    b.copy_from_slice(bytes);
    b[6] = (b[6] & 0x0F) | 0x40; // version 4
    b[8] = (b[8] & 0x3F) | 0x80; // variant
    let mut out = String::with_capacity(36);
    for (i, byte) in b.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub fn base64_standard(data: &[u8]) -> String {
    STANDARD.encode(data)
}

pub fn base64url_nopad(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Build the base64 `Signature` header: a length-delimited message
/// (version, FILETIME, method, path, authorization, body) is signed, then the
/// (version ‖ FILETIME ‖ signature) envelope is base64-encoded.
pub fn xbox_signature_header(
    key: &ProofKey,
    url_path: &str,
    authorization: &str,
    body: &str,
    unix_time: i64,
) -> String {
    let filetime = (unix_time as u64).wrapping_add(11_644_473_600) * 10_000_000;

    let mut message = Vec::new();
    message.extend_from_slice(&1u32.to_be_bytes());
    message.push(0);
    message.extend_from_slice(&filetime.to_be_bytes());
    message.push(0);
    message.extend_from_slice(b"POST");
    message.push(0);
    message.extend_from_slice(url_path.as_bytes());
    message.push(0);
    message.extend_from_slice(authorization.as_bytes());
    message.push(0);
    message.extend_from_slice(body.as_bytes());
    message.push(0);

    let signature = key.sign(&message);
    debug_assert_eq!(signature.len(), 64, "ECDSA signature must be 64 bytes");

    let mut envelope = Vec::with_capacity(4 + 8 + signature.len());
    envelope.extend_from_slice(&1u32.to_be_bytes());
    envelope.extend_from_slice(&filetime.to_be_bytes());
    envelope.extend_from_slice(&signature);
    base64_standard(&envelope)
}
