//! Error detection and correction for the 102-bit seal.
//!
//! Information bits: the 102 seal bits followed by a 16-bit CRC (CRC-16/CCITT-FALSE over the
//! 13 seal bytes). They are protected by the classic rate-1/2, constraint-length-7
//! convolutional code (generators 171 and 133 octal, as in IEEE 802.11 and many others),
//! terminated with 6 zero bits, and decoded with a soft-decision Viterbi decoder.

use crate::SEAL_BYTES;

/// Seal bits carried (the two reserved pad bits of the 13-byte seal are not sent).
pub const SEAL_BITS: usize = 102;
/// CRC bits appended to the seal.
pub const CRC_BITS: usize = 16;
/// Information bits fed to the encoder.
pub const INFO_BITS: usize = SEAL_BITS + CRC_BITS;
const TAIL_BITS: usize = 6;
/// Coded bits per seal.
pub const CODED_BITS: usize = 2 * (INFO_BITS + TAIL_BITS);

const G0: u8 = 0o171;
const G1: u8 = 0o133;

/// CRC-16/CCITT-FALSE (polynomial 0x1021, initial value 0xFFFF, no reflection).
pub fn crc16(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in bytes {
        crc ^= u16::from(b) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn bit(bytes: &[u8], i: usize) -> bool {
    bytes[i / 8] >> (7 - i % 8) & 1 == 1
}

/// The information bits for a seal: 102 seal bits, then the CRC, most significant bit first.
pub fn info_bits(seal: &[u8; SEAL_BYTES]) -> [bool; INFO_BITS] {
    let mut out = [false; INFO_BITS];
    for (i, o) in out.iter_mut().take(SEAL_BITS).enumerate() {
        *o = bit(seal, i);
    }
    let crc = crc16(seal).to_be_bytes();
    for i in 0..CRC_BITS {
        out[SEAL_BITS + i] = bit(&crc, i);
    }
    out
}

/// Rebuilds the seal from decoded information bits, or `None` if the CRC does not match.
pub fn seal_from_info(info: &[bool; INFO_BITS]) -> Option<[u8; SEAL_BYTES]> {
    let mut seal = [0u8; SEAL_BYTES];
    for (i, &b) in info.iter().take(SEAL_BITS).enumerate() {
        if b {
            seal[i / 8] |= 1 << (7 - i % 8);
        }
    }
    let mut crc = 0u16;
    for &b in &info[SEAL_BITS..] {
        crc = (crc << 1) | u16::from(b);
    }
    (crc16(&seal) == crc).then_some(seal)
}

fn parity(x: u8) -> bool {
    x.count_ones() % 2 == 1
}

fn branch(state: u8, input: bool) -> (u8, bool, bool) {
    let full = (state << 1 | u8::from(input)) & 0x7F;
    (full & 0x3F, parity(full & G0), parity(full & G1))
}

/// Convolutionally encodes the information bits (with a zero tail).
pub fn encode(info: &[bool; INFO_BITS]) -> [bool; CODED_BITS] {
    let mut out = [false; CODED_BITS];
    let mut state = 0u8;
    let tail = [false; TAIL_BITS];
    for (i, &b) in info.iter().chain(tail.iter()).enumerate() {
        let (next, a, c) = branch(state, b);
        out[2 * i] = a;
        out[2 * i + 1] = c;
        state = next;
    }
    out
}

/// Soft-decision Viterbi decoding. `soft[i] > 0` means coded bit `i` is more likely 1.
/// Returns the information bits of the path with the highest correlation.
pub fn decode(soft: &[f32; CODED_BITS]) -> [bool; INFO_BITS] {
    const STATES: usize = 64;
    let steps = INFO_BITS + TAIL_BITS;
    let mut metric = [f32::NEG_INFINITY; STATES];
    metric[0] = 0.0;
    // decisions[t][s] = input bit that led into state s at step t, and the predecessor.
    let mut from = vec![[0u8; STATES]; steps];
    for t in 0..steps {
        let (s0, s1) = (soft[2 * t], soft[2 * t + 1]);
        let mut next = [f32::NEG_INFINITY; STATES];
        let inputs: &[bool] = if t < INFO_BITS {
            &[false, true]
        } else {
            &[false]
        };
        for (state, &m) in metric.iter().enumerate() {
            if m == f32::NEG_INFINITY {
                continue;
            }
            for &input in inputs {
                let (ns, a, c) = branch(state as u8, input);
                let gain = if a { s0 } else { -s0 } + if c { s1 } else { -s1 };
                let candidate = m + gain;
                if candidate > next[ns as usize] {
                    next[ns as usize] = candidate;
                    from[t][ns as usize] = state as u8;
                }
            }
        }
        metric = next;
    }
    let mut info = [false; INFO_BITS];
    let mut state = 0u8;
    for t in (0..steps).rev() {
        let prev = from[t][state as usize];
        if t < INFO_BITS {
            info[t] = state & 1 == 1;
        }
        state = prev;
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEAL: [u8; SEAL_BYTES] = [
        0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x10,
    ];

    #[test]
    fn crc16_ccitt_false_check_value() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn info_bits_round_trip_and_reject_a_wrong_crc() {
        let mut seal = SEAL;
        seal[12] &= 0xFC; // pad bits are not carried
        let mut info = info_bits(&seal);
        assert_eq!(seal_from_info(&info), Some(seal));
        info[3] = !info[3];
        assert_eq!(seal_from_info(&info), None);
    }

    #[test]
    fn viterbi_decodes_clean_and_corrects_errors() {
        let mut seal = SEAL;
        seal[12] &= 0xFC;
        let info = info_bits(&seal);
        let coded = encode(&info);
        let mut soft = [0f32; CODED_BITS];
        for (s, &c) in soft.iter_mut().zip(coded.iter()) {
            *s = if c { 1.0 } else { -1.0 };
        }
        assert_eq!(decode(&soft), info);
        // Flip 12 well-spread coded bits (about 5%): the code must still recover.
        for i in (0..CODED_BITS).step_by(CODED_BITS / 12) {
            soft[i] = -soft[i];
        }
        assert_eq!(seal_from_info(&decode(&soft)), Some(seal));
    }

    #[test]
    fn coded_length() {
        assert_eq!(CODED_BITS, 248);
    }
}
