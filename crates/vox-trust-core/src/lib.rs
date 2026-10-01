//! Core types for the Vox Trust protocol (pre-alpha, API unstable).
//!
//! This crate holds the seal layout and the trust-policy decision table from the
//! draft spec. It contains **no cryptography and no audio processing**.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Number of meaningful bits in a packed seal.
pub const SEAL_BITS: usize = 102;
/// Number of bytes of a packed seal (the last 2 bits are reserved and zero).
pub const SEAL_BYTES: usize = 13;

const PAD_BITS: u32 = 2;

/// How a seal is authenticated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Signer and verifier share a secret.
    Circle,
    /// The signer's public key is published.
    Public,
}

impl Mode {
    fn bits(self) -> u128 {
        match self {
            Mode::Circle => 0,
            Mode::Public => 1,
        }
    }

    fn from_bits(bits: u8) -> Result<Self, DecodeError> {
        match bits {
            0 => Ok(Mode::Circle),
            1 => Ok(Mode::Public),
            other => Err(DecodeError::UnknownMode(other)),
        }
    }
}

/// A seal payload as laid out in the draft spec (section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seal {
    /// Layout version (4 bits). `0` during the draft.
    pub version: u8,
    /// Authentication mode.
    pub mode: Mode,
    /// Identifies which secret or key the verifier should use.
    pub key_id: u32,
    /// Per-key counter.
    pub counter: u16,
    /// Coarse time in minutes since a fixed epoch, modulo 2^16.
    pub time: u16,
    /// Authentication tag.
    pub tag: u32,
}

/// Why a byte string could not be decoded as a seal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The input was not exactly [`SEAL_BYTES`] bytes.
    Length(usize),
    /// The reserved trailing bits were not zero.
    ReservedBits,
    /// The mode field held a reserved value.
    UnknownMode(u8),
}

impl Seal {
    /// Packs the seal into 13 bytes, most significant bit first.
    ///
    /// # Panics
    /// Panics if `version` does not fit in 4 bits.
    pub fn to_bytes(&self) -> [u8; SEAL_BYTES] {
        assert!(self.version < 16, "version must fit in 4 bits");
        let packed: u128 = (u128::from(self.version) << 98)
            | (self.mode.bits() << 96)
            | (u128::from(self.key_id) << 64)
            | (u128::from(self.counter) << 48)
            | (u128::from(self.time) << 32)
            | u128::from(self.tag);
        let aligned = packed << PAD_BITS;
        let all = aligned.to_be_bytes();
        let mut out = [0u8; SEAL_BYTES];
        out.copy_from_slice(&all[16 - SEAL_BYTES..]);
        out
    }

    /// Unpacks a seal from 13 bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() != SEAL_BYTES {
            return Err(DecodeError::Length(bytes.len()));
        }
        let mut all = [0u8; 16];
        all[16 - SEAL_BYTES..].copy_from_slice(bytes);
        let aligned = u128::from_be_bytes(all);
        if aligned & ((1 << PAD_BITS) - 1) != 0 {
            return Err(DecodeError::ReservedBits);
        }
        let packed = aligned >> PAD_BITS;
        Ok(Seal {
            version: ((packed >> 98) & 0xF) as u8,
            mode: Mode::from_bits(((packed >> 96) & 0x3) as u8)?,
            key_id: ((packed >> 64) & 0xFFFF_FFFF) as u32,
            counter: ((packed >> 48) & 0xFFFF) as u16,
            time: ((packed >> 32) & 0xFFFF) as u16,
            tag: (packed & 0xFFFF_FFFF) as u32,
        })
    }
}

/// What the verifier found in a piece of audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealCheck {
    /// A seal was found and its tag verifies under a key the verifier trusts.
    Valid,
    /// A seal was found but it does not verify.
    Invalid,
    /// No seal could be extracted.
    Absent,
}

/// What the verifier knows about the claimed speaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContactState {
    /// The verifier has pinned this contact as someone who always seals.
    pub always_seals: bool,
    /// The verifier wants a missing seal from this contact to be an alert.
    pub strict: bool,
}

/// The outcome shown to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// A valid seal from a trusted key.
    Verified,
    /// No seal from someone who never used the protocol. Neutral, never "fake".
    Unsealed,
    /// A missing seal from a contact who always seals.
    Warning,
    /// An invalid seal, or a missing seal in strict mode for a pinned contact.
    Alert,
}

/// Applies the trust-policy table of the draft spec (section 7).
///
/// `contact` is `None` when the verifier has no record of the claimed speaker.
pub fn decide(check: SealCheck, contact: Option<ContactState>) -> Verdict {
    match check {
        SealCheck::Valid => Verdict::Verified,
        SealCheck::Invalid => Verdict::Alert,
        SealCheck::Absent => match contact {
            Some(c) if c.always_seals && c.strict => Verdict::Alert,
            Some(c) if c.always_seals => Verdict::Warning,
            _ => Verdict::Unsealed,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Seal {
        Seal {
            version: 0,
            mode: Mode::Circle,
            key_id: 0xDEAD_BEEF,
            counter: 0x1234,
            time: 0xABCD,
            tag: 0x0BAD_F00D,
        }
    }

    #[test]
    fn seal_is_102_bits_in_13_bytes() {
        assert_eq!(SEAL_BITS, 4 + 2 + 32 + 16 + 16 + 32);
        assert_eq!(SEAL_BYTES, SEAL_BITS.div_ceil(8));
    }

    #[test]
    fn round_trips() {
        let seal = sample();
        assert_eq!(Seal::from_bytes(&seal.to_bytes()), Ok(seal));
        let public = Seal {
            version: 15,
            mode: Mode::Public,
            key_id: u32::MAX,
            counter: u16::MAX,
            time: u16::MAX,
            tag: u32::MAX,
        };
        assert_eq!(Seal::from_bytes(&public.to_bytes()), Ok(public));
    }

    #[test]
    fn layout_is_msb_first_with_zero_padding() {
        // version=0 mode=0 key_id=0xDEADBEEF counter=0x1234 time=0xABCD tag=0x0BADF00D
        // 102 bits then 2 zero bits: the value shifted left by 2 within 13 bytes.
        let bytes = sample().to_bytes();
        assert_eq!(
            bytes,
            [0x03, 0x7A, 0xB6, 0xFB, 0xBC, 0x48, 0xD2, 0xAF, 0x34, 0x2E, 0xB7, 0xC0, 0x34]
        );
        assert_eq!(bytes[SEAL_BYTES - 1] & 0b11, 0);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(Seal::from_bytes(&[0u8; 12]), Err(DecodeError::Length(12)));
        let mut bytes = sample().to_bytes();
        bytes[SEAL_BYTES - 1] |= 0b01;
        assert_eq!(Seal::from_bytes(&bytes), Err(DecodeError::ReservedBits));
        let mut reserved_mode = sample().to_bytes();
        // mode field = 2 (reserved): bits 3..2 of the first byte, after the version nibble.
        reserved_mode[0] = (reserved_mode[0] & 0b0000_0011) | 0b0000_1000;
        assert_eq!(
            Seal::from_bytes(&reserved_mode),
            Err(DecodeError::UnknownMode(2))
        );
    }

    const PINNED: ContactState = ContactState {
        always_seals: true,
        strict: false,
    };
    const STRICT: ContactState = ContactState {
        always_seals: true,
        strict: true,
    };
    const NEVER: ContactState = ContactState {
        always_seals: false,
        strict: false,
    };

    #[test]
    fn policy_table() {
        for contact in [None, Some(NEVER), Some(PINNED), Some(STRICT)] {
            assert_eq!(decide(SealCheck::Valid, contact), Verdict::Verified);
            assert_eq!(decide(SealCheck::Invalid, contact), Verdict::Alert);
        }
        assert_eq!(decide(SealCheck::Absent, None), Verdict::Unsealed);
        assert_eq!(decide(SealCheck::Absent, Some(NEVER)), Verdict::Unsealed);
        assert_eq!(decide(SealCheck::Absent, Some(PINNED)), Verdict::Warning);
        assert_eq!(decide(SealCheck::Absent, Some(STRICT)), Verdict::Alert);
    }
}
