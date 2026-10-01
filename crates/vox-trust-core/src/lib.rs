//! Core of the Vox Trust protocol (pre-alpha, API unstable, not audited).
//!
//! - [`seal`]: the 102-bit in-band seal layout.
//! - [`circle`]: the circle-mode authentication tag (shared secret, HMAC-SHA-256).
//! - [`pairing`]: the text exchanged in person (for example in a QR code) to share a key.
//! - [`policy`]: the trust-policy decision table.
//! - [`replay`]: replay detection and failure rate limiting for verifiers.
//! - [`wav`] and [`mod@file`]: *file mode*, a signed manifest that travels inside a WAV
//!   file and shows which chunks of audio were altered.
//!
//! There is **no audio watermark** here yet: file mode only survives bit-exact copies.
//! See the specification and threat model in the repository before relying on any of it.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod circle;
mod crypto;
pub mod file;
pub mod pairing;
pub mod policy;
pub mod replay;
pub mod seal;
pub mod wav;

pub use policy::{decide, ContactState, SealCheck, Verdict};
pub use seal::{coarse_time, DecodeError, EncodeError, Mode, Seal, SEAL_BITS, SEAL_BYTES};

/// Lower-case hexadecimal encoding, used by reports and test vectors.
pub fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0x0F) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encoding() {
        assert_eq!(to_hex(&[]), "");
        assert_eq!(to_hex(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
    }
}
