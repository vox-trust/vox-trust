//! Thin wrappers over the audited primitives (HMAC-SHA-256, SHA-256, Ed25519).
//!
//! No cryptography is implemented here: this module only fixes how the protocol calls
//! reviewed crates, so every call site does it the same way.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

fn mac(key: &[u8], parts: &[&[u8]]) -> HmacSha256 {
    let mut mac =
        <HmacSha256 as KeyInit>::new_from_slice(key).expect("HMAC accepts keys of any length");
    for part in parts {
        mac.update(part);
    }
    mac
}

/// HMAC-SHA-256 over the concatenation of `parts`.
pub(crate) fn hmac_sha256(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    mac(key, parts).finalize().into_bytes().into()
}

/// Constant-time check of a full 32-byte HMAC-SHA-256 tag.
pub(crate) fn hmac_verify(key: &[u8], parts: &[&[u8]], tag: &[u8; 32]) -> bool {
    mac(key, parts).verify_slice(tag).is_ok()
}

/// SHA-256 over the concatenation of `parts`.
pub(crate) fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// Ed25519 public key for a 32-byte seed.
pub(crate) fn ed25519_public(seed: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(seed).verifying_key().to_bytes()
}

/// Ed25519 signature (deterministic, RFC 8032) with a 32-byte seed.
pub(crate) fn ed25519_sign(seed: &[u8; 32], message: &[u8]) -> [u8; 64] {
    SigningKey::from_bytes(seed).sign(message).to_bytes()
}

/// Strict Ed25519 verification: rejects malleable or small-order encodings.
pub(crate) fn ed25519_verify(public: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    match VerifyingKey::from_bytes(public) {
        Ok(key) => key
            .verify_strict(message, &Signature::from_bytes(signature))
            .is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::to_hex;

    #[test]
    fn sha256_known_answer() {
        // FIPS 180-2 example: "abc"
        assert_eq!(
            to_hex(&sha256(&[b"a", b"bc"])),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hmac_sha256_known_answers_rfc4231() {
        // RFC 4231 test case 1
        assert_eq!(
            to_hex(&hmac_sha256(&[0x0b; 20], &[b"Hi ", b"There"])),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        // RFC 4231 test case 2
        assert_eq!(
            to_hex(&hmac_sha256(b"Jefe", &[b"what do ya want for nothing?"])),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_verify_accepts_good_and_rejects_bad() {
        let tag = hmac_sha256(b"k", &[b"m"]);
        assert!(hmac_verify(b"k", &[b"m"], &tag));
        assert!(!hmac_verify(b"k", &[b"x"], &tag));
        assert!(!hmac_verify(b"other", &[b"m"], &tag));
        let mut flipped = tag;
        flipped[31] ^= 1;
        assert!(!hmac_verify(b"k", &[b"m"], &flipped));
    }

    fn unhex<const N: usize>(s: &str) -> [u8; N] {
        let mut out = [0u8; N];
        for i in 0..N {
            out[i] = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
        }
        out
    }

    #[test]
    fn ed25519_known_answer_rfc8032_test1() {
        let seed: [u8; 32] =
            unhex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let public: [u8; 32] =
            unhex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
        let signature: [u8; 64] = unhex(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        );
        assert_eq!(ed25519_public(&seed), public);
        assert_eq!(ed25519_sign(&seed, b""), signature);
        assert!(ed25519_verify(&public, b"", &signature));
        assert!(!ed25519_verify(&public, b"x", &signature));
        let mut bad = signature;
        bad[0] ^= 1;
        assert!(!ed25519_verify(&public, b"", &bad));
    }
}
