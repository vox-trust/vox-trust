# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The project is pre-1.0: anything may change between minor versions.

## [Unreleased]

Hardening after an adversarial review of v0.1.0. Not audited by anyone else.

### Changed (breaking for library users)
- `wav::encode_pcm16`, `push_chunk` and `finish_riff` return `Result` instead of panicking or wrapping on overflow.
- `wav::parse` (and so `with_manifest`) rejects bytes after the RIFF end and size mismatches; before, trailing bytes were ignored or silently dropped.
- `verify_circle` rejects seals whose version is not 0.
- `WavError` has new variants; `Report` has new fields `authenticator_valid` and `content_matches` (also in the JSON).
- Pairing text has one canonical form: lowercase hex, canonical percent-escapes, no empty label, and labels with bidi/zero-width/separator characters are rejected.
- `FailureLimiter` memory is bounded.

### Fixed
- A valid signature under a not-yet-pinned key now still reports whether the audio matches (`content_matches`), so a verifier can decide to pin on the evidence.
- CLI: no panic on non-UTF-8 arguments; only regular files are read, with size caps; `seal` refuses to overwrite and writes atomically; `keygen` cleans up on failure; `--help` works on every subcommand.
- Demo: a stale "Verified" can no longer stay on screen after an error; forged seals with up to a million chunks no longer freeze the page; file pickers work from the keyboard; strict Content-Security-Policy; PBKDF2 raised to 600,000 iterations; download name reflects the verdict; WebAssembly buffers are zeroed and always freed.
- The WebAssembly build is reproducible (`--locked`, path remapping) and prints its SHA-256.

### Added
- MSRV (Rust 1.94) and dependency-advisory CI jobs, `--locked` everywhere, Dependabot, issue and PR templates, code of conduct, test-vector README, `rust-toolchain.toml`.

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
