//! Carrier speed: embedding and detecting in 10 s of 16 kHz audio.
//! Run with `cargo bench -p vox-trust-carrier`.
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use vox_trust_carrier::{detect, embed, Params, SAMPLE_RATE};

fn audio(seconds: usize) -> Vec<f32> {
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    (0..seconds * SAMPLE_RATE as usize)
        .map(|i| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let noise = (state >> 40) as f32 / (1u64 << 24) as f32 - 0.5;
            let t = i as f32 / SAMPLE_RATE as f32;
            0.3 * (2.0 * std::f32::consts::PI * 180.0 * t).sin() + 0.05 * noise
        })
        .collect()
}

fn carrier(c: &mut Criterion) {
    let p = Params::default();
    let clean = audio(10);
    let seal = [0x5a; 13];
    let mut sealed_bytes = seal;
    sealed_bytes[12] &= 0xFC;
    let marked = embed(&clean, &sealed_bytes, &p).unwrap();

    let mut g = c.benchmark_group("carrier stdm-1, 10 s at 16 kHz");
    g.sample_size(10);
    g.throughput(Throughput::Elements(clean.len() as u64));
    g.bench_function("embed", |b| {
        b.iter(|| embed(black_box(&clean), &sealed_bytes, &p).unwrap())
    });
    g.bench_function("detect", |b| {
        b.iter(|| detect(black_box(&marked), &p).unwrap())
    });
    g.finish();
}

criterion_group!(benches, carrier);
criterion_main!(benches);
