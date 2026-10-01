//! Circle mode: the in-band seal is authenticated with a shared secret (draft spec, section 5).
//!
//! `tag` is the first 4 bytes of
//! `HMAC-SHA-256(K, "vox-trust/0/circle-seal\0" || version || mode || key_id || counter || time)`.
//!
//! **A 32-bit tag is weak on its own.** It only makes sense behind a verifier that tests a
//! small, bounded number of candidates per piece of audio and rate-limits failures; see the
//! threat model. It also authenticates *membership of the circle*, not *which member*, unless
//! each signer (and direction) has its own key.

use crate::crypto;
use crate::seal::{Mode, Seal};

/// Length in bytes of a circle key.
pub const KEY_LEN: usize = 32;

const SEAL_DOMAIN: &[u8] = b"vox-trust/0/circle-seal\0";
const KEY_ID_DOMAIN: &[u8] = b"vox-trust/0/key-id";

/// The recommended identifier of a circle key: the first 4 bytes (big-endian) of
/// `SHA-256("vox-trust/0/key-id" || key)`.
///
/// It lets a verifier pick the right key without revealing it. Collisions are possible
/// (32 bits), so a verifier with several keys must still check the tag.
pub fn key_id(key: &[u8; KEY_LEN]) -> u32 {
    let digest = crypto::sha256(&[KEY_ID_DOMAIN, key]);
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}

/// Computes the truncated tag for the authenticated fields of a seal.
pub fn seal_tag(key: &[u8; KEY_LEN], fields: &[u8; 10]) -> u32 {
    let full = crypto::hmac_sha256(key, &[SEAL_DOMAIN, fields]);
    u32::from_be_bytes([full[0], full[1], full[2], full[3]])
}

impl Seal {
    /// Builds a version-0 circle seal and computes its tag.
    pub fn new_circle(key: &[u8; KEY_LEN], key_id: u32, counter: u16, time: u16) -> Seal {
        let mut seal = Seal {
            version: 0,
            mode: Mode::Circle,
            key_id,
            counter,
            time,
            tag: 0,
        };
        seal.tag = seal_tag(key, &seal.authenticated_fields());
        seal
    }

    /// Checks the tag of a circle seal in constant time.
    ///
    /// Returns `false` for any seal that is not a circle seal.
    pub fn verify_circle(&self, key: &[u8; KEY_LEN]) -> bool {
        if self.mode != Mode::Circle {
            return false;
        }
        let expected = seal_tag(key, &self.authenticated_fields());
        // Compare as bytes without early exit.
        let diff = expected ^ self.tag;
        diff == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; KEY_LEN] = [7u8; KEY_LEN];

    #[test]
    fn a_fresh_seal_verifies() {
        let seal = Seal::new_circle(&KEY, 42, 1, 1000);
        assert!(seal.verify_circle(&KEY));
    }

    #[test]
    fn any_changed_field_or_key_fails() {
        let seal = Seal::new_circle(&KEY, 42, 1, 1000);
        let mut wrong_key = KEY;
        wrong_key[0] ^= 1;
        assert!(!seal.verify_circle(&wrong_key));
        for tampered in [
            Seal { key_id: 43, ..seal },
            Seal { counter: 2, ..seal },
            Seal { time: 1001, ..seal },
            Seal { version: 1, ..seal },
            Seal {
                tag: seal.tag ^ 1,
                ..seal
            },
        ] {
            assert!(!tampered.verify_circle(&KEY), "{tampered:?}");
        }
    }

    #[test]
    fn a_public_mode_seal_never_verifies_as_circle() {
        let seal = Seal::new_circle(&KEY, 42, 1, 1000);
        let as_public = Seal {
            mode: Mode::Public,
            tag: seal_tag(&KEY, &[0, 1, 0, 0, 0, 42, 0, 1, 3, 232]),
            ..seal
        };
        assert!(!as_public.verify_circle(&KEY));
    }

    #[test]
    fn key_id_is_stable_and_key_dependent() {
        assert_eq!(key_id(&KEY), key_id(&KEY));
        let mut other = KEY;
        other[31] ^= 1;
        assert_ne!(key_id(&KEY), key_id(&other));
    }

    #[test]
    fn tag_is_deterministic_and_depends_on_the_domain_fields() {
        let a = Seal::new_circle(&KEY, 42, 1, 1000);
        let b = Seal::new_circle(&KEY, 42, 1, 1000);
        assert_eq!(a, b);
        assert_ne!(a.tag, Seal::new_circle(&KEY, 42, 2, 1000).tag);
    }
}
