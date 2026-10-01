//! Arbitrary PCM: sealing then verifying must give Valid; flipping one sample bit must not.
#![no_main]
use libfuzzer_sys::fuzz_target;
use vox_trust_core::file::{self, SealParams, Signer, Trust};
use vox_trust_core::{circle, wav, SealCheck};

const KEY: [u8; 32] = [3; 32];
const SEED: [u8; 32] = [5; 32];

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let channels = u16::from(data[0] % 2) + 1;
    let chunk_frames = u32::from(data[1] % 16) + 1;
    let flip_at = usize::from(data[2]);
    let frame = usize::from(channels) * 2;
    let pcm = &data[3..3 + (data.len() - 3) / frame * frame];
    if pcm.is_empty() {
        return;
    }
    let original = wav::encode_pcm16(channels, 16_000, pcm).expect("valid PCM must encode");
    let key_id = circle::key_id(&KEY);
    let pinned = file::public_key(&SEED);
    let params = SealParams {
        created_unix: 1_700_000_000,
        counter: 1,
        chunk_frames,
    };
    let cases = [
        (
            Signer::Circle { key: &KEY, key_id },
            Trust {
                circle: Some((key_id, &KEY)),
                pinned_public: None,
            },
        ),
        (
            Signer::Public { seed: &SEED },
            Trust {
                circle: None,
                pinned_public: Some(&pinned),
            },
        ),
    ];
    for (signer, trust) in cases {
        let sealed =
            file::seal_wav(&original, signer, params).expect("sealing valid WAV must work");
        let report = file::verify_wav(&sealed, trust).expect("sealed WAV must verify");
        assert_eq!(report.check, SealCheck::Valid);

        let pcm_offset = {
            let parsed = wav::parse(&sealed).unwrap();
            parsed.pcm.as_ptr() as usize - sealed.as_ptr() as usize
        };
        let mut tampered = sealed.clone();
        let i = pcm_offset + flip_at % pcm.len();
        tampered[i] ^= 0x01;
        let report = file::verify_wav(&tampered, trust).expect("tampered WAV still parses");
        assert_ne!(
            report.check,
            SealCheck::Valid,
            "a flipped sample bit must be detected"
        );
        let chunk = ((i - pcm_offset) / frame) as u32 / chunk_frames;
        assert_eq!(report.modified_chunks, vec![chunk]);
    }
});
