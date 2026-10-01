//! Key files: a 32-byte secret, either plain (64 hex characters) or protected with a
//! passphrase.
//!
//! Protected format, one line of ASCII:
//!
//! ```text
//! vox-trust-key:1:argon2id:m=<KiB>,t=<passes>,p=<lanes>:<salt, 32 hex>:<nonce, 48 hex>:<sealed key, 96 hex>
//! ```
//!
//! The passphrase goes through Argon2id (version 0x13) with the stated parameters and the
//! 16-byte salt, giving a 32-byte key for XChaCha20-Poly1305. The sealed key is the 32-byte
//! secret encrypted under that key and the 24-byte nonce, followed by the 16-byte tag. The
//! associated data is everything before the sealed key, including the last `:`, so the
//! parameters, salt and nonce cannot be changed without detection. New files use the RFC 9106
//! second recommended option (64 MiB, 3 passes, 4 lanes); reading accepts other parameters
//! within bounds that keep a crafted file from exhausting memory or time.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

const PREFIX: &str = "vox-trust-key:1:argon2id:";

/// Argon2id parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    /// Memory in KiB.
    pub memory_kib: u32,
    /// Passes over memory.
    pub passes: u32,
    /// Lanes.
    pub lanes: u32,
}

/// RFC 9106, section 4, second recommended option.
pub const DEFAULT_COST: Cost = Cost {
    memory_kib: 64 * 1024,
    passes: 3,
    lanes: 4,
};

const MAX_MEMORY_KIB: u32 = 1024 * 1024;
const MAX_PASSES: u32 = 16;
const MAX_LANES: u32 = 16;

impl Cost {
    /// Within the bounds a reader accepts, so a crafted file cannot exhaust memory or time.
    /// Argon2 adds its own checks (for example, at least 8 KiB of memory per lane).
    fn is_supported(self) -> bool {
        self.memory_kib <= MAX_MEMORY_KIB && self.passes <= MAX_PASSES && self.lanes <= MAX_LANES
    }
}

/// Why a key file could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFileError {
    /// Not a plain or protected key file.
    Malformed,
    /// Protected, with parameters outside the accepted bounds.
    UnsupportedCost,
    /// Protected, and the passphrase is wrong or the file was changed.
    WrongPassphrase,
}

impl core::fmt::Display for KeyFileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            KeyFileError::Malformed => "not a key file (64 hex characters, or a protected key)",
            KeyFileError::UnsupportedCost => "protected key uses unsupported Argon2 parameters",
            KeyFileError::WrongPassphrase => "wrong passphrase, or the key file was changed",
        })
    }
}

/// `true` if the text is a protected key (so a passphrase is needed).
pub fn is_protected(text: &str) -> bool {
    text.trim_start().starts_with(PREFIX)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != 2 * N
        || !text
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return None;
    }
    let mut out = [0u8; N];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

fn derive(
    passphrase: &[u8],
    salt: &[u8; 16],
    cost: Cost,
) -> Result<Zeroizing<[u8; 32]>, KeyFileError> {
    let params = Params::new(cost.memory_kib, cost.passes, cost.lanes, Some(32))
        .map_err(|_| KeyFileError::UnsupportedCost)?;
    let mut out = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase, salt, &mut *out)
        .map_err(|_| KeyFileError::UnsupportedCost)?;
    Ok(out)
}

/// Seals `secret` under `passphrase`, with the given salt and nonce (callers pass random
/// values; tests pass fixed ones).
pub fn protect(
    secret: &[u8; 32],
    passphrase: &[u8],
    cost: Cost,
    salt: [u8; 16],
    nonce: [u8; 24],
) -> Result<String, KeyFileError> {
    let key = derive(passphrase, &salt, cost)?;
    let header = format!(
        "{PREFIX}m={},t={},p={}:{}:{}:",
        cost.memory_kib,
        cost.passes,
        cost.lanes,
        hex(&salt),
        hex(&nonce)
    );
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    let sealed = cipher
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: secret,
                aad: header.as_bytes(),
            },
        )
        .map_err(|_| KeyFileError::Malformed)?;
    Ok(format!("{header}{}", hex(&sealed)))
}

fn parse_cost(text: &str) -> Option<Cost> {
    let mut parts = text.split(',');
    let mut field = |name: &str| -> Option<u32> {
        let value = parts.next()?.strip_prefix(name)?.strip_prefix('=')?;
        // One text per number: digits only (`parse` would take a `+`), no leading zero.
        // `parse` itself rejects an empty or overflowing value.
        if !value.bytes().all(|c| c.is_ascii_digit()) || (value.len() > 1 && value.starts_with('0'))
        {
            return None;
        }
        value.parse().ok()
    };
    let cost = Cost {
        memory_kib: field("m")?,
        passes: field("t")?,
        lanes: field("p")?,
    };
    parts.next().is_none().then_some(cost)
}

/// Opens a protected key.
pub fn open(text: &str, passphrase: &[u8]) -> Result<Zeroizing<[u8; 32]>, KeyFileError> {
    let line = text.trim_end_matches(['\n', '\r']);
    let rest = line.strip_prefix(PREFIX).ok_or(KeyFileError::Malformed)?;
    let fields: Vec<&str> = rest.split(':').collect();
    let [cost, salt, nonce, sealed] = fields[..] else {
        return Err(KeyFileError::Malformed);
    };
    let cost = parse_cost(cost).ok_or(KeyFileError::Malformed)?;
    if !cost.is_supported() {
        return Err(KeyFileError::UnsupportedCost);
    }
    let salt: [u8; 16] = unhex(salt).ok_or(KeyFileError::Malformed)?;
    let nonce: [u8; 24] = unhex(nonce).ok_or(KeyFileError::Malformed)?;
    let sealed: [u8; 48] = unhex(sealed).ok_or(KeyFileError::Malformed)?;
    let header = &line[..line.len() - 96];
    let key = derive(passphrase, &salt, cost)?;
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    let plain = Zeroizing::new(
        cipher
            .decrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: &sealed,
                    aad: header.as_bytes(),
                },
            )
            .map_err(|_| KeyFileError::WrongPassphrase)?,
    );
    let mut out = Zeroizing::new([0u8; 32]);
    out.copy_from_slice(&plain);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Small cost so the tests stay fast; real files use DEFAULT_COST.
    const FAST: Cost = Cost {
        memory_kib: 64,
        passes: 1,
        lanes: 1,
    };
    const SECRET: [u8; 32] = [0x11; 32];

    #[test]
    fn round_trip() {
        let text = protect(&SECRET, b"correct horse", FAST, [1; 16], [2; 24]).unwrap();
        assert!(is_protected(&text));
        assert_eq!(*open(&text, b"correct horse").unwrap(), SECRET);
        assert_eq!(
            *open(&format!("{text}\n"), b"correct horse").unwrap(),
            SECRET
        );
    }

    /// Fixes the format: if this changes, existing key files would no longer open.
    #[test]
    fn known_answer() {
        let text = protect(&SECRET, b"correct horse", FAST, [1; 16], [2; 24]).unwrap();
        assert_eq!(
            text, KNOWN,
            "the protected key format changed; existing key files would not open"
        );
        assert_eq!(*open(KNOWN, b"correct horse").unwrap(), SECRET);
    }

    const KNOWN: &str = include_str!("keyfile-known-answer.txt");

    #[test]
    fn wrong_passphrase_and_tampering_are_rejected() {
        let text = protect(&SECRET, b"correct horse", FAST, [1; 16], [2; 24]).unwrap();
        assert_eq!(
            open(&text, b"wrong").unwrap_err(),
            KeyFileError::WrongPassphrase
        );
        // Any change to the header (parameters, salt, nonce) or the sealed key is detected.
        for (i, c) in text.char_indices().skip(PREFIX.len()) {
            if !c.is_ascii_hexdigit() {
                continue;
            }
            let flipped = if c == '0' { '1' } else { '0' };
            let mut bad = text.clone();
            bad.replace_range(i..i + 1, &flipped.to_string());
            assert!(
                open(&bad, b"correct horse").is_err(),
                "change at {i} accepted"
            );
        }
    }

    #[test]
    fn malformed_and_oversized_files_are_rejected() {
        let text = protect(&SECRET, b"pw", FAST, [1; 16], [2; 24]).unwrap();
        assert_eq!(open("", b"pw").unwrap_err(), KeyFileError::Malformed);
        assert_eq!(
            open(&"ab".repeat(32), b"pw").unwrap_err(),
            KeyFileError::Malformed
        );
        assert_eq!(
            open(&text.replace(":argon2id:", ":argon2i:"), b"pw").unwrap_err(),
            KeyFileError::Malformed
        );
        assert_eq!(
            open(&text.to_uppercase(), b"pw").unwrap_err(),
            KeyFileError::Malformed
        );
        assert_eq!(
            open(&format!("{text}:extra"), b"pw").unwrap_err(),
            KeyFileError::Malformed
        );
        assert_eq!(
            open(&text.replace("m=64,", "m=064,"), b"pw").unwrap_err(),
            KeyFileError::Malformed
        );
        let huge = text.replace("m=64,", "m=4194304,");
        assert_eq!(
            open(&huge, b"pw").unwrap_err(),
            KeyFileError::UnsupportedCost
        );
        let lanes = text.replace("p=1:", "p=99:");
        assert_eq!(
            open(&lanes, b"pw").unwrap_err(),
            KeyFileError::UnsupportedCost
        );
    }

    #[test]
    fn cost_bounds_are_inclusive() {
        let max = Cost {
            memory_kib: MAX_MEMORY_KIB,
            passes: MAX_PASSES,
            lanes: MAX_LANES,
        };
        assert!(max.is_supported() && DEFAULT_COST.is_supported());
        for over in [
            Cost {
                memory_kib: MAX_MEMORY_KIB + 1,
                ..max
            },
            Cost {
                passes: MAX_PASSES + 1,
                ..max
            },
            Cost {
                lanes: MAX_LANES + 1,
                ..max
            },
        ] {
            assert!(!over.is_supported(), "{over:?}");
        }
    }

    #[test]
    fn every_field_has_one_text_form() {
        let text = protect(&SECRET, b"pw", FAST, [0xab; 16], [2; 24]).unwrap();
        let salt = "ab".repeat(16);
        let malformed = [
            text.replace("m=64,", "m=+64,"),
            text.replace("m=64,", "m=,"),
            text.replace("t=1,", "t=01,"),
            text.replace("m=64,", "m=4294967296,"),
            text.replace(&salt, &salt.to_uppercase()),
            text.replace(&salt, &salt[2..]),
        ];
        for bad in malformed {
            assert_eq!(
                open(&bad, b"pw").unwrap_err(),
                KeyFileError::Malformed,
                "{bad}"
            );
        }
        // Canonical numbers out of range are a cost problem, not a format problem.
        for cost in ["m=0,", "m=4294967295,"] {
            let bad = text.replace("m=64,", cost);
            assert_eq!(
                open(&bad, b"pw").unwrap_err(),
                KeyFileError::UnsupportedCost,
                "{bad}"
            );
        }
    }

    #[test]
    fn default_cost_is_rfc_9106_second_option() {
        assert_eq!(
            DEFAULT_COST,
            Cost {
                memory_kib: 65_536,
                passes: 3,
                lanes: 4
            }
        );
    }
}
