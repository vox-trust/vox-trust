# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The project is pre-1.0: anything may change between minor versions.

## [Unreleased]

Carrier research: a better experimental carrier, a yardstick from published neural watermarks, and a measured negative result on content binding.

### Added
- **stdm-2** (experimental carrier): stdm-1 with 9.6 s windows. Through Opus 24 kbit/s 100 % (was 93 %), MP3 then Opus 98 % (was 65 %), AMR-WB 23.85 kbit/s 89 % (was 58 %), at the same quality (PESQ-WB 4.41); no wrong seal, no false alarm. `Params::stdm1()` keeps the old configuration; the two do not read each other's seals.
- Carrier detector **tempo search** (`max_tempo_pct`, on by default at ±2 %): it re-aligns audio played faster or slower without moving frequencies. A 1 % tempo change goes from 0 % to 84 % of windows recovered.
- `bench/neural_baselines.py`: AudioSeal and WavMark, used as published, on the same corpus, conditions and quality metrics as our carriers ([results](bench/results/2026-10-02-neural-baselines/README.md)). WavMark survives noise reduction, echo, tempo and partly AMR-WB, where stdm-2 fails; it is the lead for the next carrier, to be measured with a full 102-bit seal.
- `bench/content_binding.py` and `bench/content_binding_keyed.py`: a study of binding in-band seals to the audio with a robust fingerprint. **Negative result**: it stops a naive copy but an adaptive attacker defeats it, also with a fingerprint secret to the circle ([study](bench/results/2026-10-02-content-binding/README.md)). The copy attack stays open.

- README (five languages) and website: a feature comparison with Google SynthID, Meta AudioSeal, Resemble PerTh, deepfake detectors and C2PA, sourced cell by cell in `docs/COMPARISON.md`.
- A parameter sweep of stdm-2 on noise and AMR-WB: tuning buys about ten points of noise robustness and at most 24 % through AMR-WB 12.65 kbit/s at an audible cost; the default is unchanged.

### Changed
- Documents state their status the way standards do: "Specification, version 0.2" with a status line (file mode a release candidate, in-band parts experimental, 1.0 after an outside review and a second implementation), instead of "DRAFT" titles. Stale labels fixed (the governance page said 0.0, the threat model and file-mode vectors 0.1).
- Specification section 10: stdm-2 next to stdm-1, the optional tempo search, and the content-binding result in 10.3; `stdm-2` added to the registry. Threat model A2, A11 and A13 updated with the new measurements.
- CLI: without `--counter`, `seal` stores the creation time in seconds as the counter, so it grows between seals instead of always being 0.
- CLI: `--passphrase-file` with a plain (unprotected) key is a usage error instead of being ignored.
- WebAssembly: `vt_alloc` returns null when memory cannot be had, instead of trapping; the JavaScript wrapper reports it (`core_oom`, translated).
- CLI help: says which `verify --json` fields are only claims until `authenticated` is true.

## [0.4.0] - 2026-10-02

Closing the code side before an outside audit: format decisions, protected keys, and tests that check the tests.

### Added
- **Protected key files.** `vox-trust keygen` protects the secret with a passphrase by default: Argon2id (RFC 9106 second recommended option: 64 MiB, 3 passes, 4 lanes) and XChaCha20-Poly1305, with the parameters, salt and nonce authenticated. `--passphrase-file` or `VOX_TRUST_PASSPHRASE` for scripts, `--plain` for an unprotected key, and `vox-trust protect` to convert an existing plain key. A wrong passphrase exits with code 77. The known-answer vector is rebuilt in CI by a separate Python implementation (`tools/keyfile_kat.py`, with argon2-cffi and pycryptodome), not by the Rust code it tests. `--passphrase-file -` reads the passphrase from standard input, and a pipe works too. A protected key is checked before the passphrase is asked for, the key derived from the passphrase is wiped from memory, and blank lines around a key are accepted.
- Specification section 13, *Versioning, stability and registries*: which changes need a new version, and a registry of every identifier (format version, modes, chunk ID, domain strings, pairing prefix, carrier name).
- `coarse_time()` and `replay::DEFAULT_SKEW_MINUTES` in `vox-trust-core`.
- **Tests that check the tests:** line coverage with a floor of 96 % (`scripts/coverage.sh`); mutation testing of the core and the key-file code (`cargo mutants`, new workflow), where every surviving mutant was killed by a new test or, if it cannot change behaviour, listed with the reason in `.cargo/mutants.toml`; CI on macOS and Windows; the browser demo tested in Firefox and WebKit as well as Chromium; `cargo deny` for advisories, licenses and sources.
- Benchmarks (`cargo bench`): file-mode sealing and verification, carrier embedding and detection.
- Releases carry a CycloneDX software bill of materials for the CLI, the core library and the WebAssembly module, covered by `SHA256SUMS` and the attestations.

### Changed
- Specification draft 0.2. The seal's `time` field is defined as whole minutes since the Unix epoch, modulo 2^16 (it was left to implementations); the counter must increase, survive restarts and never repeat under one key; the default live clock skew is 10 minutes. In file mode the creation time and counter are informational and never a reason to reject. The test vectors gain `coarse_times`.
- **Breaking for scripts:** `keygen` now asks for a passphrase unless `--plain`, `--passphrase-file` or `VOX_TRUST_PASSPHRASE` is given.

### Fixed
- CLI: an option given twice (`--mode circle --mode public`) is now an error; before, the first value was used silently.
- CLI: `seal` uses a random temporary name, so a file planted at a predictable name can no longer block it, or make it claim the output already exists.
- CLI: input files are opened first and then checked, so a path swapped for a FIFO cannot hang the program; the audio is read before the key, so a bad input costs no passphrase prompt.
- A manifest with another format version now reads as `unsupported_version` whatever its length (it read as `malformed` when shorter than a version-0 header).
- README: the trust policy has four outcomes, including **Warning**.
- `ReplayGuard::default()` allowed a clock skew of 0 minutes, so it accepted only seals stamped in the verifier's current minute. It now uses the 10-minute default.

## [0.3.0] - 2026-10-01

Phase 0: an experimental in-band carrier, measured against real codecs. It did not pass the gate, so file mode stays the only mode that gives verdicts.

### Added
- **`vox-trust-carrier`** (experimental, research only): carries the 102-bit seal inside the audio with spread-transform dither modulation on log-spectral STFT tiles, a rate-1/2 convolutional code with soft Viterbi decoding, a CRC-16 and blind synchronisation. A damaged watermark reads as *Absent*, never as another seal. Specified in the spec's section 10.2.
- **Benchmark** (`bench/`, `vt-bench`): a reproducible corpus of 13 openly licensed recordings in 10 languages, 19 conditions (MP3, AAC, Opus, AMR-WB, G.722 via ffmpeg, noise, noise reduction, echo, tempo, trimming), exact-seal recovery, wrong seals, false alarms on unmarked audio, and perceptual quality (PESQ-WB, STOI). On-demand CI workflow.
- **Results** for four operating points and a decision record (`docs/decisions/0001-carrier-phase-0.md`): MP3 and AAC 100 %, Opus 32 kbit/s and G.722 about 99 %, Opus 24 kbit/s 93 % (100 % with 9.6 s windows); AMR-WB 12.65 kbit/s, noise at 20 dB SNR, noise reduction, echo and tempo changes fail; no wrong seal and no false alarm anywhere; PESQ-WB 4.40.
- Fuzz target for the carrier's detector.

### Changed
- Specification and threat model: section 10 now specifies the experimental carrier; the copy attack (A11) is described as easy with a public carrier, which blocks any in-band verdict; carrier robustness (A2) and candidate gating (A13) are now measured.
- The WebAssembly build pins its Rust toolchain in `scripts/WASM_TOOLCHAIN` (1.99.0). With it, a local build of the v0.2.0 commit reproduces the published `vox_trust-v0.2.0.wasm` byte for byte (SHA-256 `15660a0b…751c8`).
- README: prebuilt binaries on the releases page.

## [0.2.0] - 2026-10-01

Second hardening round, five languages and a simpler, better-indexed demo.

### Added
- **Languages:** the demo and the website in English, Português (Brasil), Español, 中文(简体) and العربية (right-to-left), with a language selector; READMEs and security/contributing summaries in the same languages (English stays normative).
- Negative test vectors (10 malformed or tampered files with expected results), re-implemented in the Python cross-check.
- `Display`/`as_str` on `SealCheck`, `Verdict`, `Mode`; `Display` on `Reason`.
- Demo: SEO metadata, no-JavaScript note, visible error when WebAssembly fails to load, plural-aware translations.
- **Fuzzing:** five cargo-fuzz targets (WAV parser, file verification, seal-then-verify round trip, pairing text, seal bytes), each asserting invariants, not only "no crash"; short runs on every push, long runs weekly.
- **Release workflow:** the command-line tool for Linux, macOS and Windows, the WebAssembly module and the demo, with `SHA256SUMS` and build-provenance attestations; it creates the tag and the GitHub Release.

### Changed (breaking for library users)
- `#[non_exhaustive]` on the public enums and on `Report`; `Pairing`, `Signer` and `Trust` print `[REDACTED]` for keys in `Debug`, and `Pairing` compares keys in constant time; `#![deny(missing_docs)]`.
- Pairing labels now reject every Unicode format character (invisible/tag characters); text normalization is documented as not performed.
- Demo: removed the decorative verdict icon, step badges and waveform canvas.

### Fixed
- CLI no longer says "signed by public key" for a signature that did not verify; no panic when stdout is closed; `seal` cannot replace a file created in a race.
- Demo: file names are always bidi-isolated and stripped of control/format characters; attacks only use the demo's own sealed audio; stronger input-border contrast.
- Secrets in CLI buffers and signing keys are zeroized.
- The minimum-supported-Rust CI job really runs on 1.94 (before, `rust-toolchain.toml` silently made it use stable).

### Infra
- Every GitHub Action is pinned to a full commit hash.

## [0.1.1] - 2026-10-01

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
