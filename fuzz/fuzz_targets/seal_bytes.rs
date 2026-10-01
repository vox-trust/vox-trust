//! Any 13 bytes: decoding must not panic, and accepted seals re-encode to the same bytes.
#![no_main]
use libfuzzer_sys::fuzz_target;
use vox_trust_core::Seal;

fuzz_target!(|data: &[u8]| {
    if let Ok(seal) = Seal::from_bytes(data) {
        assert_eq!(
            &seal.to_bytes().expect("decoded seal must encode")[..],
            data
        );
    }
});
