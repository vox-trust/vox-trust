use super::*;

const SEAL: [u8; SEAL_BYTES] = [
    0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x10, 0x32, 0x54, 0x76, 0x94,
];

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
}

/// A deterministic speech-like signal: a gliding harmonic voice with syllable-rate
/// amplitude modulation, some breath noise and short pauses.
fn voice(seconds: f32, seed: u64) -> Vec<f32> {
    let n = (seconds * SAMPLE_RATE as f32) as usize;
    let mut rng = Lcg(seed);
    let mut phase = 0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let f0 = 120.0 + 40.0 * (2.0 * PI * 0.7 * t).sin();
            phase += 2.0 * PI * f0 / SAMPLE_RATE as f32;
            let syllable = (2.0 * PI * 3.5 * t).sin().max(0.0);
            let pause = if (t % 2.3) > 2.0 { 0.02 } else { 1.0 };
            let harmonics: f32 = (1..25)
                .map(|h| (phase * h as f32).sin() / (h as f32).powf(1.2))
                .sum();
            pause * (0.25 * syllable * harmonics + 0.01 * rng.next())
        })
        .collect()
}

use std::f32::consts::PI;

fn found(audio: &[f32]) -> Vec<Detection> {
    detect(audio, &Params::default()).unwrap()
}

fn expected_seal() -> [u8; SEAL_BYTES] {
    let mut s = SEAL;
    s[12] &= 0xFC;
    s
}

#[test]
fn round_trip_finds_every_window() {
    let p = Params::default();
    let audio = voice(14.0, 1);
    let marked = embed(&audio, &SEAL, &p).unwrap();
    let hits = found(&marked);
    let windows = audio.len() / p.window_samples();
    assert_eq!(hits.len(), windows, "{hits:?}");
    for h in &hits {
        assert_eq!(h.seal, expected_seal());
        assert_eq!(h.start_sample % p.window_samples(), 0);
    }
}

#[test]
fn unmarked_audio_is_absent() {
    for seed in 1..4 {
        assert!(found(&voice(14.0, seed)).is_empty());
    }
}

#[test]
fn survives_a_delay_and_a_gain_change() {
    let p = Params::default();
    let marked = embed(&voice(14.0, 2), &SEAL, &p).unwrap();
    let mut rng = Lcg(9);
    let mut shifted: Vec<f32> = (0..1_234).map(|_| 0.001 * rng.next()).collect();
    shifted.extend(marked.iter().map(|x| 0.3 * x));
    let hits = found(&shifted);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].seal, expected_seal());
    // The detector works on a quarter-hop grid: the position is right to within 64 samples.
    let offset = hits[0].start_sample % p.window_samples();
    assert!(offset.abs_diff(1_234) <= HOP / 4, "offset {offset}");
}

/// Damage may make the seal unreadable, but must never turn it into a different seal: a
/// broken carrier reads as *Absent*.
#[test]
fn damage_yields_absent_never_a_different_seal() {
    let p = Params::default();
    let audio = voice(14.0, 3);
    let marked = embed(&audio, &SEAL, &p).unwrap();
    let power = marked.iter().map(|x| x * x).sum::<f32>() / marked.len() as f32;
    for (seed, snr_db) in [(5u64, 30.0f32), (6, 20.0), (7, 10.0), (8, 0.0)] {
        let sigma = (power / 10f32.powf(snr_db / 10.0)).sqrt() * 3f32.sqrt();
        let mut rng = Lcg(seed);
        let noisy: Vec<f32> = marked.iter().map(|x| x + sigma * rng.next()).collect();
        for h in found(&noisy) {
            assert_eq!(
                h.seal,
                expected_seal(),
                "noise at {snr_db} dB produced another seal"
            );
        }
    }
}

#[test]
fn audio_after_the_last_window_is_unchanged() {
    let p = Params::default();
    let audio = voice(9.0, 4);
    let marked = embed(&audio, &SEAL, &p).unwrap();
    let end = p.window_samples() + FRAME;
    assert_eq!(&marked[end..], &audio[end..]);
    assert_ne!(&marked[..end], &audio[..end]);
}

#[test]
fn the_change_is_small() {
    let p = Params::default();
    let audio = voice(14.0, 5);
    let marked = embed(&audio, &SEAL, &p).unwrap();
    let signal: f32 = audio.iter().map(|x| x * x).sum();
    let noise: f32 = audio
        .iter()
        .zip(&marked)
        .map(|(a, b)| (a - b).powi(2))
        .sum();
    let snr = 10.0 * (signal / noise).log10();
    assert!(snr > 10.0, "SNR {snr} dB");
}

#[test]
fn a_different_pattern_does_not_detect() {
    let p = Params::default();
    let marked = embed(&voice(14.0, 6), &SEAL, &p).unwrap();
    let other = Params {
        pattern_seed: 42,
        ..Params::default()
    };
    assert!(detect(&marked, &other).unwrap().is_empty());
}

#[test]
fn errors() {
    let p = Params::default();
    assert_eq!(
        embed(&voice(1.0, 7), &SEAL, &p),
        Err(CarrierError::TooShort)
    );
    assert_eq!(
        detect(&voice(1.0, 7), &p).unwrap_err(),
        CarrierError::TooShort
    );
    let bad = Params {
        columns: 3,
        ..Params::default()
    };
    assert_eq!(
        embed(&voice(14.0, 7), &SEAL, &bad),
        Err(CarrierError::BadParams)
    );
}

#[test]
fn capacity_of_the_default_configuration() {
    let p = Params::default();
    assert_eq!(p.window_samples(), 102_400);
    assert!((p.capacity_bps() - 15.9375).abs() < 1e-3);
}

#[test]
fn audio_of_exactly_one_window_is_found() {
    let p = Params::default();
    // Just long enough for one window on the unshifted grid, not on the shifted ones.
    let audio = voice(20.0, 8)[..p.window_samples() + FRAME / 2].to_vec();
    let marked = embed(&audio, &SEAL, &p).unwrap();
    let hits = found(&marked);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].seal, expected_seal());
}

#[test]
fn out_of_range_positions_do_not_panic() {
    let p = Params::default();
    let scan = Scan::new(&voice(8.0, 9), &p).unwrap();
    assert_eq!(scan.sync_score(0, 10_000), 0.0);
    assert_eq!(scan.sync_score(7, 0), 0.0);
    assert_eq!(scan.sync_score(HOP, 0), 0.0);
    assert!(scan.soft_bits(0, 10_000).iter().all(|&s| s == 0.0));
}
