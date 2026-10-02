# vox-trust-core

Core of the [Vox Trust protocol](https://github.com/vox-trust/vox-trust): seal layout, circle and public modes, **file mode** (a signed manifest inside a WAV file that shows which chunks were altered), the trust-policy table, replay and rate-limit helpers, and the pairing text format.

**Pre-1.0, API unstable, not audited.** File mode survives only bit-exact copies; the in-band audio carrier lives in the separate, experimental [`vox-trust-carrier`](https://crates.io/crates/vox-trust-carrier) crate and gives no verdict yet. See the [spec](https://github.com/vox-trust/vox-trust/blob/main/spec/SPEC.md) and the [threat model](https://github.com/vox-trust/vox-trust/blob/main/spec/THREAT-MODEL.md).

`verify_wav` reports, besides the verdict, `authenticator_valid` and `content_matches`: when a public-key signature is genuine but the key is not pinned, the verdict stays `UnknownKey` while `content_matches` and `modified_chunks` still say whether the audio changed since it was sealed (that is integrity, not authenticity). `Valid` covers the format tag, channels, sample rate, bits per sample and the PCM bytes only, not `byte_rate`, extra chunks or chunk order. WAV parsing is strict: trailing bytes after the RIFF container and inconsistent sizes are errors, and `wav::encode_pcm16` returns a `Result`.

## Example

```rust
use vox_trust_core::file::{public_key, seal_wav, verify_wav, SealParams, Signer, Trust};
use vox_trust_core::{decide, ContactState, Verdict};

// Sender: seal with a 32-byte Ed25519 seed (from a CSPRNG, kept secret).
let params = SealParams { created_unix: now, counter: 0, chunk_frames: 16_000 };
let sealed = seal_wav(&wav_bytes, Signer::Public { seed: &seed }, params)?;

// Receiver: pinned the sender's public key once, out of band.
let pinned = public_key(&seed);
let trust = Trust { circle: None, pinned_public: Some(&pinned) };
let report = verify_wav(&sealed, trust)?;
let contact = Some(ContactState { always_seals: true, strict: false });
assert_eq!(decide(report.check, contact), Verdict::Verified);
// report.modified_chunks lists the chunks that changed after sealing.
```

A runnable version: `cargo run -p vox-trust-core --example seal_and_verify`. The [integration guide](https://github.com/vox-trust/vox-trust/blob/main/docs/INTEGRATION.md) covers JavaScript, the command line and the trust policy.

Cryptography comes from the `hmac`, `sha2` and `ed25519-dalek` crates; nothing is implemented here. The crate forbids `unsafe` code.

Licensed under Apache-2.0.
