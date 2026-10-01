//! Prints, for each 16 kHz mono WAV given, the synchronisation score and the raw coded-bit
//! error rate at the true window positions after embedding (no distortion). Carrier
//! parameters can be overridden with VT_MAX_DB, VT_STEP, VT_SYNC_EVERY, VT_COLUMNS,
//! VT_TILE_BINS, VT_TILE_FRAMES and VT_VALLEY.
//! Usage: cargo run --release -p vox-trust-carrier --example diagnose -- FILE.wav...
use vox_trust_carrier::{embed, fec, Params, Scan};

fn read(path: &str) -> Vec<f32> {
    let bytes = std::fs::read(path).unwrap();
    let wav = vox_trust_core::wav::parse(&bytes).unwrap();
    assert_eq!(
        (wav.channels, wav.sample_rate),
        (1, 16_000),
        "need 16 kHz mono"
    );
    wav.pcm
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| f32::from(i16::from_le_bytes(c)) / 32768.0)
        .collect()
}

fn main() {
    let mut p = Params::default();
    let env = |k: &str| std::env::var(k).ok().map(|v| v.parse::<f32>().unwrap());
    if let Some(v) = env("VT_MAX_DB") {
        p.max_db = v;
    }
    if let Some(v) = env("VT_STEP") {
        p.step_db = v;
    }
    if let Some(v) = env("VT_SYNC_EVERY") {
        p.sync_every = v as usize;
    }
    if let Some(v) = env("VT_COLUMNS") {
        p.columns = v as usize;
    }
    if let Some(v) = env("VT_TILE_BINS") {
        p.tile_bins = v as usize;
    }
    if let Some(v) = env("VT_TILE_FRAMES") {
        p.tile_frames = v as usize;
    }
    if let Some(v) = env("VT_VALLEY") {
        p.valley_db = v;
    }
    let seal = [
        0x5Au8, 0xC3, 0x0F, 0xF0, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x0C,
    ];
    let coded = fec::encode(&fec::info_bits(&seal));
    for path in std::env::args().skip(1) {
        let audio = read(&path);
        let marked = embed(&audio, &seal, &p).unwrap();
        let scan = Scan::new(&marked, &p).unwrap();
        let windows = audio.len() / p.window_samples();
        let mut line = format!("{:<28}", path.rsplit('/').next().unwrap());
        for w in 0..windows {
            let col0 = w * p.columns;
            let soft = scan.soft_bits(0, col0);
            let errs = soft
                .iter()
                .zip(coded.iter())
                .filter(|(s, &c)| (**s > 0.0) != c)
                .count();
            line += &format!(
                "  w{w}: sync {:5.2} ber {:4.1}%",
                scan.sync_score(0, col0),
                100.0 * errs as f32 / 248.0
            );
        }
        println!("{line}");
    }
}
