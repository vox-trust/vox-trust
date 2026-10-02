//! The pairing text exchanged in person, for example inside a QR code (spec section 9).
//!
//! ```text
//! voxtrust:0:circle:<64 hex: the shared secret>[:<label>]
//! voxtrust:0:public:<64 hex: the Ed25519 public key>[:<label>]
//! ```
//!
//! The label is optional, non-empty, UTF-8, at most 64 bytes after decoding, free of
//! control and invisible formatting characters (Unicode Cc, Cf, Zl, Zp: bidi overrides,
//! zero-width characters, line and paragraph separators), and percent-encoded
//! (everything except `A-Z a-z 0-9 - . _ ~` is written as `%XX`).
//!
//! **No Unicode normalization is performed.** Labels are compared and stored byte for byte,
//! so two labels that look identical (precomposed `é` versus `e` + combining accent, or
//! look-alike letters from other scripts) can be different pairings. A label is a hint for
//! humans, never an identity: rely on the key, and compare key identifiers when it matters.
//!
//! Every value has exactly one text form: keys are lowercase hexadecimal, `%XX` uses
//! uppercase hexadecimal, and unreserved characters are never escaped. Anything else is
//! rejected, so two different texts never mean the same pairing.
//!
//! **A circle pairing text contains a secret.** Show it only in person, never send it, and
//! do not keep it in logs or screenshots. A public pairing text is safe to share.

use core::fmt;

use crate::crypto;
use crate::to_hex;

const PREFIX: &str = "voxtrust:0:";
/// Maximum length of a decoded label, in bytes.
pub const MAX_LABEL_BYTES: usize = 64;

/// A parsed pairing text.
///
/// `Debug` never prints the secret of a circle pairing, and `==` compares circle keys in
/// constant time.
#[derive(Clone)]
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

impl fmt::Debug for Pairing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pairing::Circle { label, .. } => f
                .debug_struct("Circle")
                .field("key", &"[REDACTED]")
                .field("label", label)
                .finish(),
            Pairing::Public { public_key, label } => f
                .debug_struct("Public")
                .field("public_key", &to_hex(public_key))
                .field("label", label)
                .finish(),
        }
    }
}

impl PartialEq for Pairing {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Pairing::Circle { key: a, label: la }, Pairing::Circle { key: b, label: lb }) => {
                crypto::ct_eq32(a, b) & (la == lb)
            }
            (
                Pairing::Public {
                    public_key: a,
                    label: la,
                },
                Pairing::Public {
                    public_key: b,
                    label: lb,
                },
            ) => a == b && la == lb,
            _ => false,
        }
    }
}

impl Eq for Pairing {}

/// Why a pairing text was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PairingError {
    /// Does not start with `voxtrust:0:`.
    BadPrefix,
    /// Wrong number of fields, or an unknown mode.
    BadFormat,
    /// The key is not exactly 64 lowercase hexadecimal characters.
    BadKey,
    /// The label is empty, is not canonical percent-encoded UTF-8, contains control or
    /// invisible formatting characters, or is longer than [`MAX_LABEL_BYTES`].
    BadLabel,
}

impl fmt::Display for PairingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PairingError::BadPrefix => "not a Vox Trust pairing text (version 0)",
            PairingError::BadFormat => "malformed pairing text",
            PairingError::BadKey => "the key must be 64 lowercase hexadecimal characters",
            PairingError::BadLabel => "the label is invalid or too long",
        })
    }
}

impl std::error::Error for PairingError {}

/// Characters of Unicode categories Cc, Cf, Zl and Zp, which can hide or reorder text.
///
/// Cc is `char::is_control`; Zl and Zp are U+2028 and U+2029; Cf is the explicit list below
/// (the standard library has no category lookup), as of Unicode 16.
fn is_forbidden_char(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{00AD}'
                | '\u{0600}'..='\u{0605}'
                | '\u{061C}'
                | '\u{06DD}'
                | '\u{070F}'
                | '\u{0890}'..='\u{0891}'
                | '\u{08E2}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200F}'
                | '\u{2028}'..='\u{2029}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{206F}'
                | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}'
                | '\u{110BD}'
                | '\u{110CD}'
                | '\u{13430}'..='\u{1343F}'
                | '\u{1BCA0}'..='\u{1BCA3}'
                | '\u{1D173}'..='\u{1D17A}'
                | '\u{E0001}'
                | '\u{E0020}'..='\u{E007F}'
        )
}

/// Checks the rules shared by encoding and decoding.
fn validate_label(label: &str) -> Result<(), PairingError> {
    if label.is_empty() || label.len() > MAX_LABEL_BYTES || label.chars().any(is_forbidden_char) {
        return Err(PairingError::BadLabel);
    }
    Ok(())
}

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

fn encode_label(label: &str) -> String {
    let mut out = String::new();
    for byte in label.bytes() {
        if is_unreserved(byte) {
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
                // Canonical form: uppercase hex digits only.
                if !hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
                {
                    return Err(PairingError::BadLabel);
                }
                let byte = u8::from_str_radix(hex, 16).map_err(|_| PairingError::BadLabel)?;
                // Canonical form: unreserved characters are never escaped.
                if is_unreserved(byte) {
                    return Err(PairingError::BadLabel);
                }
                out.push(byte);
                i += 3;
            }
            b if is_unreserved(b) => {
                out.push(b);
                i += 1;
            }
            _ => return Err(PairingError::BadLabel),
        }
    }
    let label = String::from_utf8(out).map_err(|_| PairingError::BadLabel)?;
    validate_label(&label)?;
    Ok(label)
}

fn parse_key(text: &str) -> Result<[u8; 32], PairingError> {
    if text.len() != 64
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
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
            validate_label(label)?;
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

    #[test]
    fn rejects_invisible_and_formatting_characters() {
        let key = "ab".repeat(32);
        for c in [
            '\u{200B}',
            '\u{200E}',
            '\u{202E}',
            '\u{2028}',
            '\u{2029}',
            '\u{2060}',
            '\u{2066}',
            '\u{2069}',
            '\u{206F}',
            '\u{FEFF}',
            '\u{00AD}',
            '\u{061C}',
            '\u{180E}',
            '\u{0085}',
            '\u{007F}',
            '\u{0600}',
            '\u{0605}',
            '\u{06DD}',
            '\u{070F}',
            '\u{0890}',
            '\u{0891}',
            '\u{08E2}',
            '\u{110BD}',
            '\u{110CD}',
            '\u{13430}',
            '\u{1343F}',
            '\u{1BCA0}',
            '\u{1BCA3}',
            '\u{1D173}',
            '\u{1D17A}',
            '\u{FFF9}',
            '\u{FFFB}',
            '\u{E0001}',
            '\u{E0020}',
            '\u{E007F}',
        ] {
            let label = format!("a{c}b");
            let pairing = Pairing::Circle {
                key: KEY,
                label: Some(label.clone()),
            };
            assert_eq!(pairing.encode(), Err(PairingError::BadLabel), "{c:?}");
            let text = format!("voxtrust:0:circle:{key}:{}", encode_label(&label));
            assert_eq!(Pairing::decode(&text), Err(PairingError::BadLabel), "{c:?}");
        }
        // Ordinary non-ASCII text is fine.
        let ok = Pairing::Circle {
            key: KEY,
            label: Some("Zoë 日本 \u{200D}".replace('\u{200D}', "")),
        };
        assert_eq!(Pairing::decode(&ok.encode().unwrap()).unwrap(), ok);
    }

    #[test]
    fn neighbours_of_the_forbidden_ranges_are_allowed() {
        // Letters and marks next to Cf code points must not be caught by an off-by-one.
        for c in [
            '\u{05FF}',
            '\u{0606}',
            '\u{06DC}',
            '\u{06DE}',
            '\u{070E}',
            '\u{0710}',
            '\u{FFF8}',
            '\u{FFFC}',
            '\u{E0000}',
            '\u{E0080}',
        ] {
            assert!(validate_label(&format!("a{c}b")).is_ok(), "{c:?}");
        }
    }

    #[test]
    fn labels_are_not_normalized() {
        // Precomposed and decomposed e-acute look identical but are different labels.
        let composed = Pairing::Public {
            public_key: KEY,
            label: Some("caf\u{E9}".into()),
        };
        let decomposed = Pairing::Public {
            public_key: KEY,
            label: Some("cafe\u{301}".into()),
        };
        assert_ne!(composed, decomposed);
        assert_ne!(composed.encode().unwrap(), decomposed.encode().unwrap());
        assert_eq!(
            Pairing::decode(&decomposed.encode().unwrap()).unwrap(),
            decomposed
        );
    }

    #[test]
    fn debug_never_prints_the_circle_secret() {
        let circle = Pairing::Circle {
            key: [0xAB; 32],
            label: Some("home".into()),
        };
        let shown = format!("{circle:?}");
        assert!(
            shown.contains("[REDACTED]") && shown.contains("home"),
            "{shown}"
        );
        assert!(!shown.contains("abab") && !shown.contains("171"), "{shown}");
    }

    #[test]
    fn equality_compares_key_label_and_mode() {
        let a = Pairing::Circle {
            key: KEY,
            label: None,
        };
        assert_eq!(a, a.clone());
        let mut other = KEY;
        other[31] ^= 1;
        assert_ne!(
            a,
            Pairing::Circle {
                key: other,
                label: None
            }
        );
        assert_ne!(
            a,
            Pairing::Circle {
                key: KEY,
                label: Some("x".into())
            }
        );
        assert_ne!(
            a,
            Pairing::Public {
                public_key: KEY,
                label: None
            }
        );
    }

    #[test]
    fn rejects_an_empty_label() {
        let key = "ab".repeat(32);
        assert_eq!(
            Pairing::decode(&format!("voxtrust:0:circle:{key}:")),
            Err(PairingError::BadLabel)
        );
        assert_eq!(
            Pairing::Circle {
                key: KEY,
                label: Some(String::new())
            }
            .encode(),
            Err(PairingError::BadLabel)
        );
    }

    #[test]
    fn each_value_has_one_text_form() {
        let key = "ab".repeat(32);
        // Uppercase key hex.
        assert_eq!(
            Pairing::decode(&format!("voxtrust:0:circle:{}", key.to_uppercase())),
            Err(PairingError::BadKey)
        );
        assert_eq!(
            Pairing::decode(&format!("voxtrust:0:circle:{}Ab", &key[..62])),
            Err(PairingError::BadKey)
        );
        // Lowercase percent hex, and escaped unreserved characters.
        for label in ["a%3ab", "%41", "%7e", "%2d", "a%2E"] {
            assert_eq!(
                Pairing::decode(&format!("voxtrust:0:circle:{key}:{label}")),
                Err(PairingError::BadLabel),
                "{label}"
            );
        }
        // The canonical spellings are accepted and re-encode identically.
        for label in ["a%3Ab", "A%20b", "x-y.z_~"] {
            let text = format!("voxtrust:0:circle:{key}:{label}");
            assert_eq!(Pairing::decode(&text).unwrap().encode().unwrap(), text);
        }
    }

    #[test]
    fn errors_have_distinct_messages() {
        let all = [
            PairingError::BadPrefix,
            PairingError::BadFormat,
            PairingError::BadKey,
            PairingError::BadLabel,
        ]
        .map(|e| e.to_string());
        for (i, a) in all.iter().enumerate() {
            assert!(!a.is_empty());
            assert!(all[i + 1..].iter().all(|b| a != b));
        }
    }
}
