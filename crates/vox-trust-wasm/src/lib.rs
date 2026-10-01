//! WebAssembly interface (plain C ABI) to `vox-trust-core`.
//!
//! No `wasm-bindgen`: JavaScript copies bytes into this module's linear memory with
//! [`vt_alloc`], calls a function, reads the answer with [`vt_result_ptr`] and
//! [`vt_result_len`], and frees what it allocated with [`vt_free`]. Every function that can
//! fail returns a status code: `0` means success and the result buffer holds the answer;
//! anything else means failure and the result buffer holds a UTF-8 error message.
//!
//! The only `unsafe` code in the project lives here, at the pointer boundary.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

use std::cell::RefCell;

use vox_trust_core::file::{self, SealParams, Signer, Trust};
use vox_trust_core::{to_hex, ContactState, SealCheck};

/// Version of this C ABI. Bumped on any incompatible change.
pub const ABI_VERSION: u32 = 1;

const OK: i32 = 0;
const ERR_ARGUMENT: i32 = 1;
const ERR_FILE: i32 = 2;

thread_local! {
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn set_result(bytes: Vec<u8>) {
    RESULT.with(|r| *r.borrow_mut() = bytes);
}

fn fail(code: i32, message: &str) -> i32 {
    set_result(message.as_bytes().to_vec());
    code
}

/// # Safety
/// `ptr` must be valid for reads of `len` bytes (or `len` must be zero).
unsafe fn input<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 || ptr.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

fn key32(bytes: &[u8]) -> Option<[u8; 32]> {
    bytes.try_into().ok()
}

/// Returns the ABI version.
#[no_mangle]
pub extern "C" fn vt_abi_version() -> u32 {
    ABI_VERSION
}

/// Allocates `len` zeroed bytes in linear memory and returns a pointer to them.
#[no_mangle]
pub extern "C" fn vt_alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len.max(1)].into_boxed_slice()) as *mut u8
}

/// Frees memory obtained from [`vt_alloc`].
///
/// # Safety
/// `ptr` must come from `vt_alloc(len)` with the same `len`, and must not be used after.
#[no_mangle]
pub unsafe extern "C" fn vt_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    let slice = std::ptr::slice_from_raw_parts_mut(ptr, len.max(1));
    drop(unsafe { Box::from_raw(slice) });
}

/// Pointer to the result buffer of the last call.
#[no_mangle]
pub extern "C" fn vt_result_ptr() -> *const u8 {
    RESULT.with(|r| r.borrow().as_ptr())
}

/// Length of the result buffer of the last call.
#[no_mangle]
pub extern "C" fn vt_result_len() -> usize {
    RESULT.with(|r| r.borrow().len())
}

/// Derives the Ed25519 public key and key id from a 32-byte seed.
///
/// On success the result is JSON: `{"public_key":"<hex>","key_id":"<hex>"}`.
///
/// # Safety
/// `seed_ptr` must be valid for reads of 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn vt_public_key(seed_ptr: *const u8) -> i32 {
    let Some(seed) = key32(unsafe { input(seed_ptr, 32) }) else {
        return fail(ERR_ARGUMENT, "seed must be 32 bytes");
    };
    let public = file::public_key(&seed);
    let json = format!(
        "{{\"public_key\":\"{}\",\"key_id\":\"{}\"}}",
        to_hex(&public),
        to_hex(&file::public_key_id(&public).to_be_bytes())
    );
    set_result(json.into_bytes());
    OK
}

/// The recommended key identifier of a 32-byte circle key (see `circle::key_id`).
///
/// Returns the identifier as a `u32`; if the key pointer is unusable, returns 0 and sets the
/// result buffer to an error message (callers should pass exactly 32 valid bytes).
///
/// **The return value is ambiguous:** `0` is also a legitimate key identifier (about one key
/// in 2^32), and a successful call leaves the result buffer untouched, so the buffer cannot
/// tell the two apart either. The C ABI is kept as is for compatibility with existing
/// callers: always pass a pointer to exactly 32 valid bytes, which makes the failure path
/// unreachable.
///
/// # Safety
/// `key_ptr` must be valid for reads of 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn vt_circle_key_id(key_ptr: *const u8) -> u32 {
    match key32(unsafe { input(key_ptr, 32) }) {
        Some(key) => vox_trust_core::circle::key_id(&key),
        None => {
            fail(ERR_ARGUMENT, "key must be 32 bytes");
            0
        }
    }
}

/// Seals a WAV file. `mode` is 0 for circle (`key` is the 32-byte secret) or 1 for public
/// (`key` is the 32-byte Ed25519 seed; `key_id` is ignored). The result is the sealed WAV.
///
/// # Safety
/// `wav_ptr` and `key_ptr` must be valid for reads of their lengths.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn vt_seal(
    wav_ptr: *const u8,
    wav_len: usize,
    mode: u32,
    key_ptr: *const u8,
    key_len: usize,
    key_id: u32,
    created_hi: u32,
    created_lo: u32,
    counter: u32,
    chunk_frames: u32,
) -> i32 {
    let wav = unsafe { input(wav_ptr, wav_len) };
    let Some(key) = key32(unsafe { input(key_ptr, key_len) }) else {
        return fail(ERR_ARGUMENT, "key must be 32 bytes");
    };
    let signer = match mode {
        0 => Signer::Circle { key: &key, key_id },
        1 => Signer::Public { seed: &key },
        _ => return fail(ERR_ARGUMENT, "mode must be 0 (circle) or 1 (public)"),
    };
    let params = SealParams {
        created_unix: (u64::from(created_hi) << 32) | u64::from(created_lo),
        counter,
        chunk_frames,
    };
    match file::seal_wav(wav, signer, params) {
        Ok(sealed) => {
            set_result(sealed);
            OK
        }
        Err(e) => fail(ERR_FILE, &e.to_string()),
    }
}

/// Verifies a WAV file. `circle_key_len` is 32 to supply a circle key (with `circle_key_id`)
/// or 0 for none; `pinned_len` is 32 to supply a pinned Ed25519 public key or 0 for none.
/// The result is the JSON report.
///
/// # Safety
/// All pointers must be valid for reads of their lengths.
#[no_mangle]
pub unsafe extern "C" fn vt_verify(
    wav_ptr: *const u8,
    wav_len: usize,
    circle_key_ptr: *const u8,
    circle_key_len: usize,
    circle_key_id: u32,
    pinned_ptr: *const u8,
    pinned_len: usize,
) -> i32 {
    let wav = unsafe { input(wav_ptr, wav_len) };
    let circle_key = match circle_key_len {
        0 => None,
        _ => match key32(unsafe { input(circle_key_ptr, circle_key_len) }) {
            Some(k) => Some(k),
            None => return fail(ERR_ARGUMENT, "circle key must be 32 bytes"),
        },
    };
    let pinned = match pinned_len {
        0 => None,
        _ => match key32(unsafe { input(pinned_ptr, pinned_len) }) {
            Some(k) => Some(k),
            None => return fail(ERR_ARGUMENT, "pinned public key must be 32 bytes"),
        },
    };
    let trust = Trust {
        circle: circle_key.as_ref().map(|k| (circle_key_id, k)),
        pinned_public: pinned.as_ref(),
    };
    match file::verify_wav(wav, trust) {
        Ok(report) => {
            set_result(report.to_json().into_bytes());
            OK
        }
        Err(e) => fail(ERR_FILE, &e.to_string()),
    }
}

/// Applies the trust policy. `check`: 0 valid, 1 invalid, 2 unknown key, 3 absent.
/// `has_contact`: 0 if the verifier has no record of the speaker. Returns the verdict code:
/// 0 verified, 1 unsealed, 2 warning, 3 alert (any other `check` is treated as invalid).
#[no_mangle]
pub extern "C" fn vt_decide(check: u32, has_contact: u32, always_seals: u32, strict: u32) -> u32 {
    let check = match check {
        0 => SealCheck::Valid,
        2 => SealCheck::UnknownKey,
        3 => SealCheck::Absent,
        _ => SealCheck::Invalid,
    };
    let contact = (has_contact != 0).then_some(ContactState {
        always_seals: always_seals != 0,
        strict: strict != 0,
    });
    vox_trust_core::decide(check, contact).code()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result() -> Vec<u8> {
        RESULT.with(|r| r.borrow().clone())
    }

    fn wav() -> Vec<u8> {
        let pcm: Vec<u8> = (0..400i16).flat_map(|s| (s * 50).to_le_bytes()).collect();
        vox_trust_core::wav::encode_pcm16(1, 8000, &pcm).unwrap()
    }

    #[test]
    fn alloc_and_free_round_trip() {
        let p = vt_alloc(16);
        assert!(!p.is_null());
        unsafe { vt_free(p, 16) };
        let p = vt_alloc(0); // zero-length requests are still valid pointers
        assert!(!p.is_null());
        unsafe { vt_free(p, 0) };
    }

    #[test]
    fn circle_seal_verify_through_the_c_abi() {
        let key = [5u8; 32];
        let original = wav();
        let status = unsafe {
            vt_seal(
                original.as_ptr(),
                original.len(),
                0,
                key.as_ptr(),
                32,
                77,
                0,
                1_700_000_000,
                3,
                100,
            )
        };
        assert_eq!(status, OK);
        let sealed = result();

        let status = unsafe {
            vt_verify(
                sealed.as_ptr(),
                sealed.len(),
                key.as_ptr(),
                32,
                77,
                std::ptr::null(),
                0,
            )
        };
        assert_eq!(status, OK);
        let json = String::from_utf8(result()).unwrap();
        assert!(json.starts_with("{\"check\":\"valid\""), "{json}");
        assert!(json.contains("\"created_unix\":1700000000"));

        // Without a key the seal is from an unknown key.
        let status = unsafe {
            vt_verify(
                sealed.as_ptr(),
                sealed.len(),
                std::ptr::null(),
                0,
                0,
                std::ptr::null(),
                0,
            )
        };
        assert_eq!(status, OK);
        assert!(String::from_utf8(result())
            .unwrap()
            .contains("\"check\":\"unknown_key\""));
    }

    #[test]
    fn public_seal_verify_through_the_c_abi() {
        let seed = [8u8; 32];
        assert_eq!(unsafe { vt_public_key(seed.as_ptr()) }, OK);
        let json = String::from_utf8(result()).unwrap();
        let public_hex = json.split('"').nth(3).unwrap().to_string();
        let public: Vec<u8> = (0..32)
            .map(|i| u8::from_str_radix(&public_hex[2 * i..2 * i + 2], 16).unwrap())
            .collect();

        let original = wav();
        let status = unsafe {
            vt_seal(
                original.as_ptr(),
                original.len(),
                1,
                seed.as_ptr(),
                32,
                0,
                0,
                5,
                1,
                50,
            )
        };
        assert_eq!(status, OK);
        let mut sealed = result();
        let status = unsafe {
            vt_verify(
                sealed.as_ptr(),
                sealed.len(),
                std::ptr::null(),
                0,
                0,
                public.as_ptr(),
                32,
            )
        };
        assert_eq!(status, OK);
        assert!(String::from_utf8(result())
            .unwrap()
            .starts_with("{\"check\":\"valid\""));

        // Tamper with the audio: the report localises it.
        let at = sealed.len() / 3;
        sealed[at] ^= 0xFF;
        unsafe {
            vt_verify(
                sealed.as_ptr(),
                sealed.len(),
                std::ptr::null(),
                0,
                0,
                public.as_ptr(),
                32,
            )
        };
        let json = String::from_utf8(result()).unwrap();
        assert!(json.contains("\"reason\":\"modified\""), "{json}");
    }

    #[test]
    fn errors_return_a_status_and_a_message() {
        let bad_key = [0u8; 5];
        let w = wav();
        let status =
            unsafe { vt_seal(w.as_ptr(), w.len(), 0, bad_key.as_ptr(), 5, 0, 0, 0, 0, 10) };
        assert_eq!(status, ERR_ARGUMENT);
        assert_eq!(result(), b"key must be 32 bytes");

        let key = [1u8; 32];
        let status = unsafe { vt_seal(w.as_ptr(), w.len(), 9, key.as_ptr(), 32, 0, 0, 0, 0, 10) };
        assert_eq!(status, ERR_ARGUMENT);

        let junk = b"not a wav file";
        let status = unsafe {
            vt_seal(
                junk.as_ptr(),
                junk.len(),
                0,
                key.as_ptr(),
                32,
                0,
                0,
                0,
                0,
                10,
            )
        };
        assert_eq!(status, ERR_FILE);
        assert!(!result().is_empty());

        let status = unsafe {
            vt_verify(
                junk.as_ptr(),
                junk.len(),
                std::ptr::null(),
                0,
                0,
                std::ptr::null(),
                0,
            )
        };
        assert_eq!(status, ERR_FILE);

        let status =
            unsafe { vt_verify(w.as_ptr(), w.len(), key.as_ptr(), 7, 0, std::ptr::null(), 0) };
        assert_eq!(status, ERR_ARGUMENT);
    }

    #[test]
    fn decide_matches_the_core_policy() {
        // valid -> verified regardless of contact
        assert_eq!(vt_decide(0, 0, 0, 0), 0);
        assert_eq!(vt_decide(0, 1, 1, 1), 0);
        // absent: no contact -> unsealed; pinned -> warning; pinned+strict -> alert
        assert_eq!(vt_decide(3, 0, 0, 0), 1);
        assert_eq!(vt_decide(3, 1, 1, 0), 2);
        assert_eq!(vt_decide(3, 1, 1, 1), 3);
        // unknown key: unsealed unless pinned
        assert_eq!(vt_decide(2, 1, 0, 0), 1);
        assert_eq!(vt_decide(2, 1, 1, 0), 3);
        // invalid -> alert
        assert_eq!(vt_decide(1, 0, 0, 0), 3);
        assert_eq!(vt_decide(99, 0, 0, 0), 3);
    }

    #[test]
    fn abi_version_is_one() {
        assert_eq!(vt_abi_version(), 1);
    }
}
