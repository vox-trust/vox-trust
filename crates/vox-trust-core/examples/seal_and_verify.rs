//! Seal a WAV file in public mode and verify it as a receiver who pinned the sender's key.
//!
//! `cargo run -p vox-trust-core --example seal_and_verify`

use vox_trust_core::file::{public_key, seal_wav, verify_wav, SealParams, Signer, Trust};
use vox_trust_core::{decide, ContactState, Verdict};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // One second of a 440 Hz tone, 16 kHz mono, 16-bit PCM.
    let pcm: Vec<u8> = (0..16_000)
        .flat_map(|i| {
            let t = i as f32 / 16_000.0;
            (((t * 440.0 * std::f32::consts::TAU).sin() * 8_000.0) as i16).to_le_bytes()
        })
        .collect();
    let wav = vox_trust_core::wav::encode_pcm16(1, 16_000, &pcm)?;

    // Sender: a 32-byte Ed25519 seed. Generate it with a CSPRNG and keep it secret.
    let seed = [7u8; 32];
    let params = SealParams {
        created_unix: 1_790_000_000,
        counter: 0,
        chunk_frames: 4_000,
    };
    let sealed = seal_wav(&wav, Signer::Public { seed: &seed }, params)?;

    // Receiver: has pinned the sender's public key (shared once, out of band).
    let pinned = public_key(&seed);
    let trust = Trust {
        circle: None,
        pinned_public: Some(&pinned),
    };
    let contact = Some(ContactState {
        always_seals: true,
        strict: false,
    });

    let report = verify_wav(&sealed, trust)?;
    assert_eq!(decide(report.check, contact), Verdict::Verified);
    println!("sealed file: {}", decide(report.check, contact));

    // Flip one bit in sample 10 000 (chunks of 4 000 frames): the seal breaks and the
    // altered chunk, number 2, is reported. The PCM data starts after a 44-byte header.
    let mut tampered = sealed.clone();
    tampered[44 + 2 * 10_000] ^= 0x40;
    let report = verify_wav(&tampered, trust)?;
    println!(
        "tampered file: {} (modified chunks {:?})",
        decide(report.check, contact),
        report.modified_chunks
    );

    // The same audio with the seal stripped, from a contact who always seals.
    let report = verify_wav(&wav, trust)?;
    println!("unsealed file: {}", decide(report.check, contact));
    Ok(())
}
