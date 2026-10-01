//! The 102-bit in-band seal layout (draft spec, section 4).

use core::fmt;

/// Number of meaningful bits in a packed seal.
pub const SEAL_BITS: usize = 102;
/// Number of bytes of a packed seal (the last 2 bits are reserved and zero).
pub const SEAL_BYTES: usize = 13;
/// Highest layout version that fits the 4-bit field.
pub const MAX_VERSION: u8 = 15;

/// The seal's coarse `time` field for a moment given in seconds since the Unix epoch (UTC):
/// whole minutes since the epoch, modulo 2^16 (spec section 4). It wraps every 65,536
/// minutes, about 45.5 days; compare stamps with [`crate::replay::minute_distance`].
pub fn coarse_time(unix_seconds: u64) -> u16 {
    ((unix_seconds / 60) % 65_536) as u16
}

const PAD_BITS: u32 = 2;

/// How a seal is authenticated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mode {
    /// Signer and verifier share a secret.
    Circle,
    /// The signer's public key is published.
    Public,
}

impl Mode {
    /// Stable lower-case name (`"circle"` or `"public"`), used in JSON reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Circle => "circle",
            Mode::Public => "public",
        }
    }

    pub(crate) fn bits(self) -> u8 {
        match self {
            Mode::Circle => 0,
            Mode::Public => 1,
        }
    }

    pub(crate) fn from_bits(bits: u8) -> Result<Self, DecodeError> {
        match bits {
            0 => Ok(Mode::Circle),
            1 => Ok(Mode::Public),
            other => Err(DecodeError::UnknownMode(other)),
        }
    }
}

/// A seal payload as laid out in the draft spec.
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
#[non_exhaustive]
pub enum DecodeError {
    /// The input was not exactly [`SEAL_BYTES`] bytes.
    Length(usize),
    /// The reserved trailing bits were not zero.
    ReservedBits,
    /// The mode field held a reserved value.
    UnknownMode(u8),
}

/// Why a seal could not be encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodeError {
    /// The version does not fit in 4 bits.
    VersionOutOfRange(u8),
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Length(n) => write!(f, "seal must be {SEAL_BYTES} bytes, got {n}"),
            DecodeError::ReservedBits => f.write_str("reserved bits of the seal are not zero"),
            DecodeError::UnknownMode(m) => write!(f, "unknown seal mode {m}"),
        }
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::VersionOutOfRange(v) => {
                write!(f, "seal version {v} does not fit in 4 bits")
            }
        }
    }
}

impl std::error::Error for DecodeError {}
impl std::error::Error for EncodeError {}

impl Seal {
    /// Packs the seal into 13 bytes, most significant bit first.
    pub fn to_bytes(&self) -> Result<[u8; SEAL_BYTES], EncodeError> {
        if self.version > MAX_VERSION {
            return Err(EncodeError::VersionOutOfRange(self.version));
        }
        let packed: u128 = (u128::from(self.version) << 98)
            | (u128::from(self.mode.bits()) << 96)
            | (u128::from(self.key_id) << 64)
            | (u128::from(self.counter) << 48)
            | (u128::from(self.time) << 32)
            | u128::from(self.tag);
        let aligned = packed << PAD_BITS;
        let all = aligned.to_be_bytes();
        let mut out = [0u8; SEAL_BYTES];
        out.copy_from_slice(&all[16 - SEAL_BYTES..]);
        Ok(out)
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

    /// The fields covered by the tag, as fixed-width big-endian bytes:
    /// `version (1) | mode (1) | key_id (4) | counter (2) | time (2)`.
    pub fn authenticated_fields(&self) -> [u8; 10] {
        let mut out = [0u8; 10];
        out[0] = self.version;
        out[1] = self.mode.bits();
        out[2..6].copy_from_slice(&self.key_id.to_be_bytes());
        out[6..8].copy_from_slice(&self.counter.to_be_bytes());
        out[8..10].copy_from_slice(&self.time.to_be_bytes());
        out
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
        assert_eq!(Seal::from_bytes(&seal.to_bytes().unwrap()), Ok(seal));
        let public = Seal {
            version: 15,
            mode: Mode::Public,
            key_id: u32::MAX,
            counter: u16::MAX,
            time: u16::MAX,
            tag: u32::MAX,
        };
        assert_eq!(Seal::from_bytes(&public.to_bytes().unwrap()), Ok(public));
    }

    #[test]
    fn layout_is_msb_first_with_zero_padding() {
        let bytes = sample().to_bytes().unwrap();
        assert_eq!(
            bytes,
            [0x03, 0x7A, 0xB6, 0xFB, 0xBC, 0x48, 0xD2, 0xAF, 0x34, 0x2E, 0xB7, 0xC0, 0x34]
        );
        assert_eq!(bytes[SEAL_BYTES - 1] & 0b11, 0);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(Seal::from_bytes(&[0u8; 12]), Err(DecodeError::Length(12)));
        let mut bytes = sample().to_bytes().unwrap();
        bytes[SEAL_BYTES - 1] |= 0b01;
        assert_eq!(Seal::from_bytes(&bytes), Err(DecodeError::ReservedBits));
        let mut reserved_mode = sample().to_bytes().unwrap();
        // mode field = 2 (reserved): bits 3..2 of the first byte, after the version nibble.
        reserved_mode[0] = (reserved_mode[0] & 0b0000_0011) | 0b0000_1000;
        assert_eq!(
            Seal::from_bytes(&reserved_mode),
            Err(DecodeError::UnknownMode(2))
        );
    }

    #[test]
    fn encoding_a_bad_version_is_an_error_not_a_panic() {
        let mut seal = sample();
        seal.version = 16;
        assert_eq!(seal.to_bytes(), Err(EncodeError::VersionOutOfRange(16)));
    }

    #[test]
    fn authenticated_fields_layout() {
        assert_eq!(
            sample().authenticated_fields(),
            [0, 0, 0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0xAB, 0xCD]
        );
    }

    #[test]
    fn coarse_time_is_minutes_since_the_unix_epoch_modulo_2_16() {
        assert_eq!(coarse_time(0), 0);
        assert_eq!(coarse_time(59), 0);
        assert_eq!(coarse_time(60), 1);
        assert_eq!(coarse_time(65_535 * 60 + 59), 65_535);
        assert_eq!(coarse_time(65_536 * 60), 0);
        // 2023-11-14T22:13:20Z: 28,333,333 minutes since the epoch.
        assert_eq!(coarse_time(1_700_000_000), (28_333_333 % 65_536) as u16);
        assert_eq!(coarse_time(u64::MAX), ((u64::MAX / 60) % 65_536) as u16);
    }

    #[test]
    fn display() {
        assert_eq!(Mode::Circle.to_string(), Mode::Circle.as_str());
        assert_eq!(Mode::Public.to_string(), Mode::Public.as_str());
        assert_eq!(
            DecodeError::Length(5).to_string(),
            "seal must be 13 bytes, got 5"
        );
        assert!(!DecodeError::ReservedBits.to_string().is_empty());
        assert_eq!(
            DecodeError::UnknownMode(3).to_string(),
            "unknown seal mode 3"
        );
        assert_eq!(
            EncodeError::VersionOutOfRange(16).to_string(),
            "seal version 16 does not fit in 4 bits"
        );
    }
}
