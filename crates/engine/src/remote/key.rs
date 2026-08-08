//! The credential vocabulary: what a key looks like, how one is minted, and how
//! a presented one is recognised.
//!
//! The format is `hestia-web`'s verbatim — `hst_` plus 32 random bytes as
//! base64url, stored only as a SHA-256 digest, with the first twelve characters
//! kept so a listing can name a key without holding one. Sharing the namespace
//! means automated secret scanning recognises a leaked key from either project.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Namespaced so secret scanning can attribute a leak.
const NAMESPACE: &str = "hst";

/// 256 bits: the token is random rather than chosen, so there is nothing for an
/// offline attack to shorten and a fast digest is the right one.
const TOKEN_BYTES: usize = 32;

/// How much of a key is readable enough to recognise it and useless enough to
/// print — `hst_` plus eight characters.
const PREFIX_LENGTH: usize = NAMESPACE.len() + 1 + 8;

/// A freshly minted key. The plaintext exists here and nowhere else.
pub struct Issued {
    pub token: String,
    pub digest: String,
    pub prefix: String,
}

pub fn issue() -> Issued {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).expect("system RNG must be available to mint a remote key");
    let token = format!("{NAMESPACE}_{}", URL_SAFE_NO_PAD.encode(bytes));
    Issued {
        digest: digest(&token),
        prefix: prefix(&token),
        token,
    }
}

pub fn digest(token: &str) -> String {
    crate::checksum::Hasher::hex(&Sha256::digest(token.as_bytes()))
}

/// The readable head, derived from the key itself so anything holding one
/// records the same prefix for it.
pub fn prefix(token: &str) -> String {
    token.chars().take(PREFIX_LENGTH).collect()
}

/// Whether a presented token's digest is the stored one.
///
/// Constant-time on purpose: a byte-by-byte comparison that returns early leaks
/// how much of a digest a guess got right, which is the shape of attack this
/// whole scheme exists to make pointless.
pub fn matches(stored: &str, presented: &str) -> bool {
    stored.as_bytes().ct_eq(presented.as_bytes()).into()
}

/// Whether a string is even shaped like one of ours. Checked before hashing so
/// an unrelated `Authorization` header costs nothing.
pub fn looks_like_a_key(token: &str) -> bool {
    token.len() > PREFIX_LENGTH
        && token.starts_with(NAMESPACE)
        && token[NAMESPACE.len()..].starts_with('_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_issued_key_is_namespaced_and_its_prefix_is_its_own_head() {
        let issued = issue();
        assert!(issued.token.starts_with("hst_"));
        assert_eq!(issued.prefix.len(), PREFIX_LENGTH);
        assert!(issued.token.starts_with(&issued.prefix));
        assert_eq!(prefix(&issued.token), issued.prefix);
    }

    #[test]
    fn two_issues_never_collide() {
        let (a, b) = (issue(), issue());
        assert_ne!(a.token, b.token);
        assert_ne!(a.digest, b.digest);
    }

    #[test]
    fn only_the_digest_recognises_the_token_that_made_it() {
        let issued = issue();
        assert!(matches(&issued.digest, &digest(&issued.token)));
        assert!(!matches(&issued.digest, &digest("hst_not-the-same")));
    }

    #[test]
    fn a_digest_is_sixty_four_hex_characters() {
        let d = digest("hst_anything");
        assert_eq!(d.len(), 64);
        assert!(d
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn a_foreign_authorization_header_is_rejected_before_hashing() {
        assert!(looks_like_a_key(&issue().token));
        for foreign in [
            "",
            "hst",
            "hst_",
            "hst_short",
            "Bearer hst_xxxxxxxxxxxx",
            "eyJhbGciOiJIUzI1NiJ9.e30.x",
        ] {
            assert!(!looks_like_a_key(foreign), "{foreign} should not pass");
        }
    }
}
