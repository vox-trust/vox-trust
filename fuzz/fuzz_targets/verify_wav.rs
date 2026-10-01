//! Any input: `verify_wav` must return, never panic, and the report must be coherent.
#![no_main]
use libfuzzer_sys::fuzz_target;
use vox_trust_core::file::{self, Trust};
use vox_trust_core::{circle, SealCheck};

const CIRCLE_KEY: [u8; 32] = [7; 32];
const SEED: [u8; 32] = [9; 32];

fuzz_target!(|data: &[u8]| {
    let pinned = file::public_key(&SEED);
    let trusts = [
        Trust {
            circle: None,
            pinned_public: None,
        },
        Trust {
            circle: Some((circle::key_id(&CIRCLE_KEY), &CIRCLE_KEY)),
            pinned_public: Some(&pinned),
        },
    ];
    for trust in trusts {
        let Ok(report) = file::verify_wav(data, trust) else {
            continue;
        };
        if report.authenticated {
            assert!(report.authenticator_valid);
        }
        if !report.authenticator_valid {
            assert!(
                report.modified_chunks.is_empty(),
                "unauthenticated manifests must not localize tampering"
            );
            assert!(!report.content_matches);
        }
        if report.check == SealCheck::Valid {
            assert!(
                report.authenticated && report.content_matches && report.modified_chunks.is_empty()
            );
        }
        let _ = report.to_json();
    }
});
