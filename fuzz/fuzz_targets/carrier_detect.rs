//! Any audio: the carrier detector must return, never panic, and anything it returns must
//! carry a seal whose CRC matched (it is re-checked here independently of the detector).
#![no_main]
use libfuzzer_sys::fuzz_target;
use vox_trust_carrier::{detect, fec, Params};

fuzz_target!(|data: &[u8]| {
    // Interpret the input as 16-bit PCM, repeated to reach one carrier window.
    let pcm: Vec<f32> = data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| f32::from(i16::from_le_bytes(c)) / 32768.0)
        .collect();
    if pcm.is_empty() {
        return;
    }
    let p = Params::default();
    let samples: Vec<f32> = pcm
        .iter()
        .copied()
        .cycle()
        .take(p.window_samples() + 1024)
        .collect();
    for d in detect(&samples, &p).expect("one full window is enough") {
        let info = fec::info_bits(&d.seal);
        assert_eq!(fec::seal_from_info(&info), Some(d.seal));
        assert_eq!(d.seal[12] & 0x03, 0, "pad bits must be zero");
    }
});
