//! File-mode throughput: sealing and verifying 30 s of 16 kHz mono speech-sized audio.
//! Run with `cargo bench -p vox-trust-core`.
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use vox_trust_core::file::{self, SealParams, Signer, Trust};
use vox_trust_core::{circle, wav};

fn clip(seconds: usize) -> Vec<u8> {
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let pcm: Vec<u8> = (0..seconds * 16_000)
        .flat_map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ((state >> 48) as i16 / 4).to_le_bytes()
        })
        .collect();
    wav::encode_pcm16(1, 16_000, &pcm).expect("valid PCM")
}

fn file_mode(c: &mut Criterion) {
    let audio = clip(30);
    let key = [7u8; 32];
    let key_id = circle::key_id(&key);
    let seed = [9u8; 32];
    let public = file::public_key(&seed);
    let params = SealParams {
        created_unix: 1_700_000_000,
        counter: 1,
        chunk_frames: 16_000,
    };
    let circle_trust = Trust {
        circle: Some((key_id, &key)),
        pinned_public: None,
    };
    let public_trust = Trust {
        circle: None,
        pinned_public: Some(&public),
    };
    let sealed_circle =
        file::seal_wav(&audio, Signer::Circle { key: &key, key_id }, params).unwrap();
    let sealed_public = file::seal_wav(&audio, Signer::Public { seed: &seed }, params).unwrap();

    let mut g = c.benchmark_group("file mode, 30 s at 16 kHz mono");
    g.throughput(Throughput::Bytes(audio.len() as u64));
    g.bench_function("seal circle", |b| {
        b.iter(|| {
            file::seal_wav(
                black_box(&audio),
                Signer::Circle { key: &key, key_id },
                params,
            )
            .unwrap()
        })
    });
    g.bench_function("seal public", |b| {
        b.iter(|| {
            file::seal_wav(black_box(&audio), Signer::Public { seed: &seed }, params).unwrap()
        })
    });
    g.bench_function("verify circle", |b| {
        b.iter(|| file::verify_wav(black_box(&sealed_circle), circle_trust).unwrap())
    });
    g.bench_function("verify public", |b| {
        b.iter(|| file::verify_wav(black_box(&sealed_public), public_trust).unwrap())
    });
    g.finish();
}

criterion_group!(benches, file_mode);
criterion_main!(benches);
