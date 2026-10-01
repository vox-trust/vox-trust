# vox-trust-core

Core of the [Vox Trust protocol](https://github.com/vox-trust/vox-trust): seal layout, circle and public modes, **file mode** (a signed manifest inside a WAV file that shows which chunks were altered), the trust-policy table, replay and rate-limit helpers, and the pairing text format.

**Pre-1.0, API unstable, not audited.** File mode survives only bit-exact copies; there is no audio watermark carrier yet. See the [spec](https://github.com/vox-trust/vox-trust/blob/main/spec/SPEC.md) and the [threat model](https://github.com/vox-trust/vox-trust/blob/main/spec/THREAT-MODEL.md).

`verify_wav` reports, besides the verdict, `authenticator_valid` and `content_matches`: when a public-key signature is genuine but the key is not pinned, the verdict stays `UnknownKey` while `content_matches` and `modified_chunks` still say whether the audio changed since it was sealed (that is integrity, not authenticity). `Valid` covers the format tag, channels, sample rate, bits per sample and the PCM bytes only, not `byte_rate`, extra chunks or chunk order. WAV parsing is strict: trailing bytes after the RIFF container and inconsistent sizes are errors, and `wav::encode_pcm16` returns a `Result`.

Cryptography comes from the `hmac`, `sha2` and `ed25519-dalek` crates; nothing is implemented here. The crate forbids `unsafe` code.

Licensed under Apache-2.0.
