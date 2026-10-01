//! Builds the published test vectors. Shared by `tests/vectors.rs` (which checks the
//! published files) and `examples/gen_vectors.rs` (which writes them).
//!
//! The chunk digests are computed here with `sha2` directly, independently of the crate's
//! own digest function, so the test cross-checks the crate against the written spec.
#![allow(dead_code)]

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use vox_trust_core::file::{self, SealParams, Signer};
use vox_trust_core::{to_hex, wav, Mode, Seal};

pub const CREATED_UNIX: u64 = 1_700_000_000;
pub const CIRCLE_KEY_ID: u32 = 0x0102_0304;

/// 32 bytes counting up from `start`.
pub fn counting_key(start: u8) -> [u8; 32] {
    let mut key = [0u8; 32];
    for (i, b) in key.iter_mut().enumerate() {
        *b = start.wrapping_add(i as u8);
    }
    key
}

pub fn pcm_bytes(samples: &[i16]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Circle => "circle",
        Mode::Public => "public",
    }
}

pub fn seal_vectors() -> Value {
    let key = counting_key(0x00);
    let packing: Vec<Value> = [
        (
            "sample",
            Seal {
                version: 0,
                mode: Mode::Circle,
                key_id: 0xDEAD_BEEF,
                counter: 0x1234,
                time: 0xABCD,
                tag: 0x0BAD_F00D,
            },
        ),
        (
            "all-ones",
            Seal {
                version: 15,
                mode: Mode::Public,
                key_id: u32::MAX,
                counter: u16::MAX,
                time: u16::MAX,
                tag: u32::MAX,
            },
        ),
        (
            "all-zeros",
            Seal {
                version: 0,
                mode: Mode::Circle,
                key_id: 0,
                counter: 0,
                time: 0,
                tag: 0,
            },
        ),
    ]
    .into_iter()
    .map(|(name, seal)| {
        json!({
            "name": name,
            "version": seal.version,
            "mode": mode_name(seal.mode),
            "key_id": seal.key_id,
            "counter": seal.counter,
            "time": seal.time,
            "tag": seal.tag,
            "bytes": to_hex(&seal.to_bytes().unwrap()),
        })
    })
    .collect();

    let circle_tags: Vec<Value> = [
        ("basic", 42u32, 1u16, 1000u16),
        ("wrapped-counter", 0xFFFF_FFFF, 65535, 65535),
        ("zero", 0, 0, 0),
    ]
    .into_iter()
    .map(|(name, key_id, counter, time)| {
        let seal = Seal::new_circle(&key, key_id, counter, time);
        json!({
            "name": name,
            "key": to_hex(&key),
            "key_id": key_id,
            "counter": counter,
            "time": time,
            "authenticated_fields": to_hex(&seal.authenticated_fields()),
            "tag": seal.tag,
            "seal_bytes": to_hex(&seal.to_bytes().unwrap()),
        })
    })
    .collect();

    let key_ids: Vec<Value> = [counting_key(0x00), counting_key(0x40), [0u8; 32]]
        .iter()
        .map(|k| json!({ "key": to_hex(k), "key_id": vox_trust_core::circle::key_id(k) }))
        .collect();

    json!({
        "description": "Vox Trust seal vectors, draft 0.1. All keys in this file are public test values: NEVER use them to protect anything. Circle tag = first 4 bytes of HMAC-SHA-256(key, \"vox-trust/0/circle-seal\\0\" || authenticated_fields). Circle key id = first 4 bytes (big-endian) of SHA-256(\"vox-trust/0/key-id\" || key).",
        "packing": packing,
        "circle_tags": circle_tags,
        "circle_key_ids": key_ids,
    })
}

struct Case {
    name: &'static str,
    channels: u16,
    sample_rate: u32,
    samples: Vec<i16>,
    chunk_frames: u32,
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "mono-10-frames",
            channels: 1,
            sample_rate: 8000,
            samples: vec![0, 1000, -1000, 32767, -32768, 5, -5, 12345, -12345, 7],
            chunk_frames: 4,
        },
        Case {
            name: "stereo-5-frames",
            channels: 2,
            sample_rate: 16000,
            samples: vec![1, -1, 2, -2, 3, -3, 400, -400, 500, -500],
            chunk_frames: 2,
        },
    ]
}

fn chunk_digests(pcm: &[u8], channels: u16, chunk_frames: u32) -> Vec<[u8; 32]> {
    let chunk_bytes = chunk_frames as usize * channels as usize * 2;
    pcm.chunks(chunk_bytes)
        .enumerate()
        .map(|(index, chunk)| {
            let mut hasher = Sha256::new();
            hasher.update([0x01]);
            hasher.update((index as u32).to_be_bytes());
            hasher.update(chunk);
            hasher.finalize().into()
        })
        .collect()
}

fn manifest_of(sealed: &[u8]) -> Vec<u8> {
    wav::parse(sealed).unwrap().manifest.unwrap().to_vec()
}

pub fn file_vectors() -> Value {
    let circle_key = counting_key(0x00);
    let seed = counting_key(0x40);
    let public = file::public_key(&seed);

    let cases: Vec<Value> = cases()
        .into_iter()
        .map(|case| {
            let pcm = pcm_bytes(&case.samples);
            let original = wav::encode_pcm16(case.channels, case.sample_rate, &pcm).unwrap();
            let digests = chunk_digests(&pcm, case.channels, case.chunk_frames);

            let circle = file::seal_wav(
                &original,
                Signer::Circle {
                    key: &circle_key,
                    key_id: CIRCLE_KEY_ID,
                },
                SealParams {
                    created_unix: CREATED_UNIX,
                    counter: 7,
                    chunk_frames: case.chunk_frames,
                },
            )
            .unwrap();
            let public_sealed = file::seal_wav(
                &original,
                Signer::Public { seed: &seed },
                SealParams {
                    created_unix: CREATED_UNIX,
                    counter: 8,
                    chunk_frames: case.chunk_frames,
                },
            )
            .unwrap();

            // Flip one bit in the first byte of chunk 1.
            let mut tampered = circle.clone();
            let parsed = wav::parse(&circle).unwrap();
            let offset = parsed.pcm.as_ptr() as usize - circle.as_ptr() as usize
                + case.chunk_frames as usize * case.channels as usize * 2;
            tampered[offset] ^= 0x01;

            let circle_manifest = manifest_of(&circle);
            let public_manifest = manifest_of(&public_sealed);
            let n = digests.len();
            let signed_len = 46 + 32 * n;
            json!({
                "name": case.name,
                "sample_rate": case.sample_rate,
                "channels": case.channels,
                "samples": case.samples,
                "chunk_frames": case.chunk_frames,
                "original_wav": to_hex(&original),
                "chunk_digests": digests.iter().map(|d| to_hex(d)).collect::<Vec<_>>(),
                "circle": {
                    "key": to_hex(&circle_key),
                    "key_id": CIRCLE_KEY_ID,
                    "created_unix": CREATED_UNIX,
                    "counter": 7,
                    "manifest": to_hex(&circle_manifest),
                    "authenticator": to_hex(&circle_manifest[signed_len..]),
                    "sealed_wav": to_hex(&circle),
                    "tampered_sealed_wav": to_hex(&tampered),
                    "tampered_expected_modified_chunks": [1],
                },
                "public": {
                    "seed": to_hex(&seed),
                    "public_key": to_hex(&public),
                    "key_id": file::public_key_id(&public),
                    "created_unix": CREATED_UNIX,
                    "counter": 8,
                    "manifest": to_hex(&public_manifest),
                    "signature": to_hex(&public_manifest[signed_len + 32..]),
                    "sealed_wav": to_hex(&public_sealed),
                },
            })
        })
        .collect();

    json!({
        "description": "Vox Trust file-mode vectors, draft 0.1. See spec/SPEC.md, section 6 (File mode). All keys in this file are public test values: NEVER use them to protect anything.",
        "cases": cases,
    })
}
