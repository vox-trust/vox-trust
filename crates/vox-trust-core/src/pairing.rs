//! The pairing text exchanged in person, for example inside a QR code (draft spec, section 9).
//!
//! ```text
//! voxtrust:0:circle:<64 hex: the shared secret>[:<label>]
//! voxtrust:0:public:<64 hex: the Ed25519 public key>[:<label>]
//! ```
//!
//! The label is optional, UTF-8, at most 64 bytes after decoding, and percent-encoded
//! (everything except `A-Z a-z 0-9 - . _ ~` is written as `%XX`).
//!
//! **A circle pairing text contains a secret.** Show it only in person, never send it, and
//! do not keep it in logs or screenshots. A public pairing text is safe to share.

use core::fmt;

use crate::to_hex;

const PREFIX: &str = "voxtrust:0:";
/// Maximum length of a decoded label, in bytes.
pub const MAX_LABEL_BYTES: usize = 64;

/// A parsed pairing text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pairing {
    /// A shared secret for circle mode.
    Circle {
        /// The 32-byte shared secret.
        key: [u8; 32],
        /// Optional human-readable label.
        label: Option<String>,
    },
    /// A public key for public mode.
    Public {
        /// The 32-byte Ed25519 public key.
        public_key: [u8; 32],
        /// Optional human-readable label.
        label: Option<String>,
    },
}

/// Why a pairing text was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingError {
    /// Does not start with `voxtrust:0:`.
    BadPrefix,
    /// Wrong number of fields, or an unknown mode.
    BadFormat,
    /// The key is not exactly 64 hexadecimal characters.
    BadKey,
    /// The label is not valid percent-encoded UTF-8, contains control characters, or is
    /// longer than [`MAX_LABEL_BYTES`].
    BadLabel,
}

impl fmt::Display for PairingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PairingError::BadPrefix => "not a Vox Trust pairing text (version 0)",
            PairingError::BadFormat => "malformed pairing text",
            PairingError::BadKey => "the key must be 64 hexadecimal characters",
            PairingError::BadLabel => "the label is invalid or too long",
        })
    }
}

impl std::error::Error for PairingError {}

fn encode_label(label: &str) -> String {
    let mut out = String::new();
    for byte in label.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn decode_label(text: &str) -> Result<String, PairingError> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = text.get(i + 1..i + 3).ok_or(PairingError::BadLabel)?;
                if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(PairingError::BadLabel);
                }
                out.push(u8::from_str_radix(hex, 16).map_err(|_| PairingError::BadLabel)?);
                i += 3;
            }
            b if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') => {
                out.push(b);
                i += 1;
            }
            _ => return Err(PairingError::BadLabel),
        }
    }
    let label = String::from_utf8(out).map_err(|_| PairingError::BadLabel)?;
    if label.len() > MAX_LABEL_BYTES || label.chars().any(char::is_control) {
        return Err(PairingError::BadLabel);
    }
    Ok(label)
}

fn parse_key(text: &str) -> Result<[u8; 32], PairingError> {
    if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(PairingError::BadKey);
    }
    let mut key = [0u8; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte =
            u8::from_str_radix(&text[2 * i..2 * i + 2], 16).map_err(|_| PairingError::BadKey)?;
    }
    Ok(key)
}

impl Pairing {
    /// Renders the pairing text. Labels longer than [`MAX_LABEL_BYTES`] bytes are rejected.
    pub fn encode(&self) -> Result<String, PairingError> {
        let (mode, key, label) = match self {
            Pairing::Circle { key, label } => ("circle", key, label),
            Pairing::Public { public_key, label } => ("public", public_key, label),
        };
        let mut out = format!("{PREFIX}{mode}:{}", to_hex(key));
        if let Some(label) = label {
            if label.len() > MAX_LABEL_BYTES || label.chars().any(char::is_control) {
                return Err(PairingError::BadLabel);
            }
            out.push(':');
            out.push_str(&encode_label(label));
        }
        Ok(out)
    }

    /// Parses a pairing text strictly.
    pub fn decode(text: &str) -> Result<Pairing, PairingError> {
        let rest = text.strip_prefix(PREFIX).ok_or(PairingError::BadPrefix)?;
        let fields: Vec<&str> = rest.split(':').collect();
        if !(2..=3).contains(&fields.len()) {
            return Err(PairingError::BadFormat);
        }
        let key = parse_key(fields[1])?;
        let label = fields.get(2).map(|l| decode_label(l)).transpose()?;
        match fields[0] {
            "circle" => Ok(Pairing::Circle { key, label }),
            "public" => Ok(Pairing::Public {
                public_key: key,
                label,
            }),
            _ => Err(PairingError::BadFormat),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [0xAB; 32];

    #[test]
    fn round_trips_both_modes_with_and_without_label() {
        for pairing in [
            Pairing::Circle {
                key: KEY,
                label: None,
            },
            Pairing::Circle {
                key: KEY,
                label: Some("Mãe 💙 (casa)".into()),
            },
            Pairing::Public {
                public_key: KEY,
                label: None,
            },
            Pairing::Public {
                public_key: KEY,
                label: Some("Roger Oliveira".into()),
            },
        ] {
            let text = pairing.encode().unwrap();
            assert_eq!(Pairing::decode(&text).unwrap(), pairing, "{text}");
        }
    }

    #[test]
    fn exact_wire_format() {
        let text = Pairing::Public {
            public_key: KEY,
            label: Some("A b:c".into()),
        }
        .encode()
        .unwrap();
        assert_eq!(
            text,
            format!("voxtrust:0:public:{}:A%20b%3Ac", "ab".repeat(32))
        );
        assert_eq!(
            Pairing::Circle {
                key: KEY,
                label: None
            }
            .encode()
            .unwrap(),
            format!("voxtrust:0:circle:{}", "ab".repeat(32))
        );
    }

    #[test]
    fn rejects_malformed_texts() {
        let key = "ab".repeat(32);
        let cases = [
            ("".to_string(), PairingError::BadPrefix),
            (format!("voxtrust:1:circle:{key}"), PairingError::BadPrefix),
            (format!("VOXTRUST:0:circle:{key}"), PairingError::BadPrefix),
            ("voxtrust:0:circle".to_string(), PairingError::BadFormat),
            (
                format!("voxtrust:0:circle:{key}:a:b"),
                PairingError::BadFormat,
            ),
            (format!("voxtrust:0:other:{key}"), PairingError::BadFormat),
            ("voxtrust:0:circle:abcd".to_string(), PairingError::BadKey),
            (
                format!("voxtrust:0:circle:{}zz", &key[..62]),
                PairingError::BadKey,
            ),
            (
                format!("voxtrust:0:circle:{key}:%ZZ"),
                PairingError::BadLabel,
            ),
            (
                format!("voxtrust:0:circle:{key}:%4"),
                PairingError::BadLabel,
            ),
            (
                format!("voxtrust:0:circle:{key}:raw space"),
                PairingError::BadLabel,
            ),
            (
                format!("voxtrust:0:circle:{key}:%FF"),
                PairingError::BadLabel,
            ), // invalid UTF-8
            (
                format!("voxtrust:0:circle:{key}:%00"),
                PairingError::BadLabel,
            ), // control character
        ];
        for (text, expected) in cases {
            assert_eq!(Pairing::decode(&text), Err(expected), "{text}");
        }
    }

    #[test]
    fn label_length_is_bounded_in_both_directions() {
        let ok = "x".repeat(MAX_LABEL_BYTES);
        let too_long = "x".repeat(MAX_LABEL_BYTES + 1);
        assert!(Pairing::Circle {
            key: KEY,
            label: Some(ok)
        }
        .encode()
        .is_ok());
        assert_eq!(
            Pairing::Circle {
                key: KEY,
                label: Some(too_long.clone())
            }
            .encode(),
            Err(PairingError::BadLabel)
        );
        let text = format!("voxtrust:0:circle:{}:{too_long}", "ab".repeat(32));
        assert_eq!(Pairing::decode(&text), Err(PairingError::BadLabel));
    }
}
