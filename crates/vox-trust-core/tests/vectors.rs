//! Checks the published test vectors in `spec/test-vectors/` against the crate.
//!
//! If a vector legitimately changes, regenerate with:
//! `cargo run -p vox-trust-core --example gen_vectors`

mod common;

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use vox_trust_core::file::{self, FileError, Reason, Trust};
use vox_trust_core::wav::WavError;
use vox_trust_core::{Seal, SealCheck};

fn vectors_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/test-vectors")
        .join(name)
}

fn read(name: &str) -> Value {
    let text = fs::read_to_string(vectors_path(name))
        .unwrap_or_else(|e| panic!("cannot read {name}: {e}. Run the gen_vectors example."));
    serde_json::from_str(&text).unwrap()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn key32(s: &str) -> [u8; 32] {
    unhex(s).try_into().unwrap()
}

#[test]
fn seal_vectors_match_the_published_file() {
    assert_eq!(
        common::seal_vectors(),
        read("seal-v0.json"),
        "published seal vectors differ from the crate; regenerate with the gen_vectors example"
    );
}

#[test]
fn file_vectors_match_the_published_file() {
    assert_eq!(
        common::file_vectors(),
        read("file-v0.json"),
        "published file vectors differ from the crate; regenerate with the gen_vectors example"
    );
}

#[test]
fn published_seal_packing_decodes_back() {
    for v in read("seal-v0.json")["packing"].as_array().unwrap() {
        let seal = Seal::from_bytes(&unhex(v["bytes"].as_str().unwrap())).unwrap();
        assert_eq!(u64::from(seal.version), v["version"].as_u64().unwrap());
        assert_eq!(u64::from(seal.key_id), v["key_id"].as_u64().unwrap());
        assert_eq!(u64::from(seal.counter), v["counter"].as_u64().unwrap());
        assert_eq!(u64::from(seal.time), v["time"].as_u64().unwrap());
        assert_eq!(u64::from(seal.tag), v["tag"].as_u64().unwrap());
    }
}

#[test]
fn published_circle_tags_verify() {
    for v in read("seal-v0.json")["circle_tags"].as_array().unwrap() {
        let key = key32(v["key"].as_str().unwrap());
        let seal = Seal::from_bytes(&unhex(v["seal_bytes"].as_str().unwrap())).unwrap();
        assert!(seal.verify_circle(&key), "{}", v["name"]);
    }
}

#[test]
fn published_file_vectors_verify_and_localise_tampering() {
    for case in read("file-v0.json")["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();

        let circle = &case["circle"];
        let key = key32(circle["key"].as_str().unwrap());
        let trust = Trust {
            circle: Some((circle["key_id"].as_u64().unwrap() as u32, &key)),
            pinned_public: None,
        };
        let sealed = unhex(circle["sealed_wav"].as_str().unwrap());
        let report = file::verify_wav(&sealed, trust).unwrap();
        assert_eq!(report.check, SealCheck::Valid, "{name}");

        let tampered = unhex(circle["tampered_sealed_wav"].as_str().unwrap());
        let report = file::verify_wav(&tampered, trust).unwrap();
        assert_eq!(report.reason, Reason::Modified, "{name}");
        let expected: Vec<u32> = circle["tampered_expected_modified_chunks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(report.modified_chunks, expected, "{name}");

        let public = &case["public"];
        let public_key = key32(public["public_key"].as_str().unwrap());
        let trust = Trust {
            circle: None,
            pinned_public: Some(&public_key),
        };
        let sealed = unhex(public["sealed_wav"].as_str().unwrap());
        let report = file::verify_wav(&sealed, trust).unwrap();
        assert_eq!(report.check, SealCheck::Valid, "{name}");
    }
}

fn check_from_name(name: &str) -> SealCheck {
    [
        SealCheck::Valid,
        SealCheck::Invalid,
        SealCheck::UnknownKey,
        SealCheck::Absent,
    ]
    .into_iter()
    .find(|c| c.as_str() == name)
    .unwrap_or_else(|| panic!("unknown check name {name}"))
}

fn reason_from_name(name: &str) -> Reason {
    [
        Reason::None,
        Reason::NoManifest,
        Reason::Malformed,
        Reason::UnsupportedVersion,
        Reason::BadAuthenticator,
        Reason::BadSignature,
        Reason::UntrustedKey,
        Reason::FormatChanged,
        Reason::Modified,
    ]
    .into_iter()
    .find(|r| r.as_str() == name)
    .unwrap_or_else(|| panic!("unknown reason name {name}"))
}

#[test]
fn negative_vectors_report_exactly_what_is_published() {
    let file = read("file-v0.json");
    let negatives = file["negative"].as_array().unwrap();
    assert!(negatives.len() >= 8, "the negative vectors went missing");
    for v in negatives {
        let name = v["name"].as_str().unwrap();
        let wav = unhex(v["wav"].as_str().unwrap());
        let t = &v["trust"];
        let circle_key = t["circle_key"].as_str().map(key32);
        let pinned = t["pinned_public"].as_str().map(key32);
        let trust = Trust {
            circle: circle_key
                .as_ref()
                .map(|k| (t["circle_key_id"].as_u64().unwrap() as u32, k)),
            pinned_public: pinned.as_ref(),
        };
        let expected = &v["expected"];
        let result = file::verify_wav(&wav, trust);
        if let Some(error) = expected["error"].as_str() {
            let want = match error {
                "trailing_bytes" => FileError::Wav(WavError::TrailingBytes),
                other => panic!("{name}: unknown error name {other}"),
            };
            assert_eq!(result.err(), Some(want), "{name}");
            continue;
        }
        let report = result.unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            report.check,
            check_from_name(expected["check"].as_str().unwrap()),
            "{name}"
        );
        assert_eq!(
            report.reason,
            reason_from_name(expected["reason"].as_str().unwrap()),
            "{name}"
        );
        assert_ne!(
            report.check,
            SealCheck::Valid,
            "{name}: a negative vector verified"
        );
        let modified: Vec<u32> = expected["modified_chunks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(report.modified_chunks, modified, "{name}");
        assert_eq!(
            report.content_matches,
            expected["content_matches"].as_bool().unwrap(),
            "{name}"
        );
    }
}
