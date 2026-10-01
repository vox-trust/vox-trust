# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The project is pre-1.0: anything may change between minor versions.

## [0.1.0] - 2026-10-01

First release with working code. **File mode** works end to end; the in-band audio carrier does not exist yet. Not audited.

### Added
- **File mode:** a signed manifest inside a WAV file (`VOXT` chunk) with a SHA-256 digest per chunk of audio. It detects any change to the samples, their order or the audio format, and reports which chunks changed. Circle mode authenticates with HMAC-SHA-256; public mode with Ed25519 (strict verification).
- Circle seal tag (HMAC-SHA-256 truncated to 32 bits) for the 102-bit in-band seal, and a recommended circle key identifier.
- Trust policy with four checks (valid, invalid, unknown key, absent) and four verdicts (verified, unsealed, warning, alert).
- Replay guard (serial-number arithmetic for 16-bit counters, clock-skew check) and failure rate limiter.
- Pairing text format for exchanging keys in person, for example in a QR code.
- Strict RIFF/WAVE reader and writer for 16-bit PCM.
- `vox-trust` command-line tool: `keygen`, `pubkey`, `seal`, `verify`, with scriptable exit codes.
- WebAssembly build of the core (about 95 KB, no imports) behind a plain C interface, and a JavaScript wrapper.
- Browser demo: seal, six attack buttons, verifier with per-chunk result, public-key pinning.
- Published test vectors, and `tools/check_vectors.py`, an independent re-implementation of the vectors from the spec text.
- Tests at five layers: Rust, vectors, the independent Python check, WebAssembly in Node, and the demo in headless Chromium.

### Changed
- `Seal::to_bytes` returns a `Result` instead of panicking on an out-of-range version.
- Specification and threat model updated to draft 0.1, including the weaknesses found in the author's own review (in-band seals are not bound to the audio content; a 32-bit tag multiplies with the verifier's search; a shared key cannot say which member sealed).

### Known limitations
- Sealed files survive only bit-exact copies. Re-encoding, resampling or re-recording reads as modified.
- No audio watermark carrier, so no measurements of watermark survival yet.
- No independent review and no independent implementation yet.

## [0.0.1] - 2026-10-01

Initial public draft: specification 0.0, threat model, and a crate with the seal layout and the trust-policy table.
