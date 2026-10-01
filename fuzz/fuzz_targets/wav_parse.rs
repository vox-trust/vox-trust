//! Any input: `wav::parse` and `wav::with_manifest` must return, never panic.
#![no_main]
use libfuzzer_sys::fuzz_target;
use vox_trust_core::wav;

fuzz_target!(|data: &[u8]| {
    if let Ok(parsed) = wav::parse(data) {
        assert!(parsed.pcm.len() <= data.len());
        let manifest = &data[..data.len().min(64)];
        if let Ok(rewritten) = wav::with_manifest(data, manifest) {
            let again = wav::parse(&rewritten).expect("with_manifest output must parse");
            assert_eq!(again.pcm, parsed.pcm);
            assert_eq!(again.manifest, Some(manifest));
        }
    }
});
