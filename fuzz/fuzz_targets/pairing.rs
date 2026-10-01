//! Any text: decoding must not panic, and every accepted text is the one canonical form.
#![no_main]
use libfuzzer_sys::fuzz_target;
use vox_trust_core::pairing::Pairing;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(pairing) = Pairing::decode(text) {
        let encoded = pairing.encode().expect("a decoded pairing must re-encode");
        assert_eq!(encoded, text, "pairing text must have exactly one form");
        assert!(Pairing::decode(&encoded).unwrap() == pairing);
    }
});
