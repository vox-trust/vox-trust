<p align="center">
  🌐 <strong>English</strong> · <a href="README.pt-BR.md">Português (Brasil)</a> · <a href="README.es.md">Español</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.ar.md">العربية</a>
</p>

<h1 align="center">Vox Trust</h1>

<p align="center"><strong>Don't detect fake voices. Prove real ones.</strong></p>

<p align="center">
An open protocol, with a Rust reference implementation, that seals a human voice at the source and lets anyone verify it later, right in the browser.
</p>

<p align="center">
  <a href="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml/badge.svg"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-orange?logo=rust&logoColor=white">
  <img alt="WebAssembly" src="https://img.shields.io/badge/WebAssembly-no%20imports-654FF0?logo=webassembly&logoColor=white">
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue"></a>
  <img alt="Version 0.3.0" src="https://img.shields.io/badge/version-0.3.0-informational">
  <img alt="Not audited" src="https://img.shields.io/badge/security-not%20audited-red">
</p>

<p align="center">
  <a href="https://vox-trust.github.io/demo/"><strong>Live demo</strong></a> ·
  <a href="https://vox-trust.github.io">Website</a> ·
  <a href="spec/SPEC.md">Spec</a> ·
  <a href="spec/THREAT-MODEL.md">Threat model</a> ·
  <a href="docs/ROADMAP.md">Roadmap</a>
</p>

> **Status: v0.3, file mode works. Not audited.** You can seal a WAV file, verify it, and see exactly which seconds were altered, in the [browser demo](https://vox-trust.github.io/demo/) or with the command line. Sealed files survive **only bit-exact copies**. An **experimental** audio watermark that survives MP3, AAC and Opus is built and [measured](bench/results/2026-10-01-stdm-1/README.md), but it fails phone-call codecs and noise, and in-band seals can be copied into other audio, so it gives **no verdicts** yet. Do not use this to protect anyone until it has been reviewed.

## The problem

Cloning a voice now takes seconds of audio. Listening can no longer tell a real voice from a cloned one, and detectors chase a moving target: as detection improves, so does generation.

## The idea

Instead of guessing whether a voice is fake, check whether a real one was **sealed**.

1. **Seal.** The speaker's device signs the audio at the source.
2. **Verify.** Anyone with the protocol checks which key sealed it, when, and which chunks of audio were altered.
3. **Decide.** A local trust policy turns the result into one of four outcomes.

| Outcome | Meaning |
|---|---|
| **Verified** | A valid seal from a key you trust. |
| **Unsealed** | No seal (or a seal under a key you never supplied), from someone you have no expectations of. Neutral, **not** "fake". |
| **Warning** | No seal from a contact who *always* seals: worth a second look, since compression can also remove a seal. |
| **Alert** | The seal is broken, comes from a key you didn't pin for that contact, or is missing from a contact who *always* seals (strict mode). |

Two modes: **circle** (people who know each other, shared secret) and **public** (organisations and public figures, Ed25519 keys that verifiers pin).

## Try it in 30 seconds

**In the browser** (nothing is uploaded; the Rust core runs as WebAssembly): open the [demo](https://vox-trust.github.io/demo/), press *Seal*, then try to cheat with the buttons and watch the verifier catch each attempt.

**On the command line** (prebuilt binaries for Linux, macOS and Windows, with checksums and build attestations, are on the [releases page](https://github.com/vox-trust/vox-trust/releases/latest); or build from source):

```sh
cargo install --locked --git https://github.com/vox-trust/vox-trust --tag v0.3.0 vox-trust-cli

vox-trust keygen me.key                                  # asks for a passphrase
vox-trust seal speech.wav sealed.wav --mode circle --key me.key
vox-trust verify sealed.wav --circle-key me.key          # exit code 0 = verified
```

`keygen` protects the key with a passphrase (Argon2id and XChaCha20-Poly1305) and asks for it whenever the key is used. In scripts, pass `--passphrase-file` or set `VOX_TRUST_PASSPHRASE`; `--plain` writes an unprotected key.

Edit a single sample of `sealed.wav` and verify again: the exit code becomes 3 and the altered chunk is printed. Exit codes: 0 verified, 1 unsealed, 2 warning, 3 alert.

**As a Rust library:**

```rust
// A sketch: `wav_bytes` is a 16-bit PCM WAV you already have; run inside a function returning a Result.
use vox_trust_core::file::{seal_wav, verify_wav, SealParams, Signer, Trust};
use vox_trust_core::{circle, SealCheck};

let key = [7u8; 32]; // use a random 32-byte secret in real life
let sealed = seal_wav(
    &wav_bytes,
    Signer::Circle { key: &key, key_id: circle::key_id(&key) },
    SealParams { created_unix: 1_700_000_000, counter: 1, chunk_frames: 16_000 },
)?;
let trust = Trust { circle: Some((circle::key_id(&key), &key)), pinned_public: None };
assert_eq!(verify_wav(&sealed, trust)?.check, SealCheck::Valid);
```

## What this is NOT

- It is **not** deepfake detection and **not** voice biometrics.
- It does **not** prove a speaker is human or is who they claim. It proves that *a key* sealed the audio. A stolen key or a compromised device produces valid seals.
- "No seal" does **not** mean "fake": compression and noise suppression can erase a watermark.
- File mode does **not** survive MP3, AAC, resampling or re-recording. That is the carrier's job, and the carrier is experimental (below).
- It does not protect against an attacker who controls the sender's device.

Attackers, claims and the weaknesses found so far (including one that is still unsolved) are in the [threat model](spec/THREAT-MODEL.md).

## How it is tested

| Layer | What it checks | Run |
|---|---|---|
| Rust unit and integration tests | Seal layout, HMAC and Ed25519 (against RFC 4231 and RFC 8032 vectors), WAV parsing, manifests, tampering, policy, the C interface and the CLI | `cargo test --workspace` |
| Published test vectors | Byte-exact seals and manifests in [`spec/test-vectors/`](spec/test-vectors) | included above |
| Cross-check in Python | `tools/check_vectors.py` rebuilds every vector from the spec text (standard library, plus `cryptography` for Ed25519) | `pip install cryptography && python3 tools/check_vectors.py --strict` |
| WebAssembly end to end | The compiled module reproduces the vectors byte for byte, handles garbage input, and does not leak memory | `scripts/build-web.sh && node --test tests/node/*.mjs` |
| Real browser | The demo page in headless Chromium, Firefox and WebKit: sealing, six attacks, public-key pinning, five languages, accessibility, phone width, dark mode | `BROWSER=firefox node tests/browser/demo.mjs` |
| Fuzzing | Every parser of untrusted input, with invariants (sealed audio always verifies, any flipped sample bit is found in the right chunk, pairing text has one form) | `cd fuzz && cargo +nightly fuzz run verify_wav` |
| Coverage | Share of Rust lines the tests execute; CI fails below 96 % | `scripts/coverage.sh` |
| Mutation testing | Changes operators and return values in `vox-trust-core` and the key-file code, one at a time, and checks that a test fails; the few changes that cannot alter behaviour are listed with the reason in `.cargo/mutants.toml` | `cargo mutants -p vox-trust-core`; `cargo mutants -p vox-trust-cli -f crates/vox-trust-cli/src/keyfile.rs` |
| Dependencies | Known vulnerabilities, licenses, sources | `cargo deny check` |

CI runs the Rust tests on Linux, macOS and Windows.

The Python check is written by the same author, so it is a cross-check, not an independent implementation. [An independent implementation is what the spec still needs.](docs/ROADMAP.md)

## Surviving re-encoding: the experimental carrier

`crates/vox-trust-carrier` hides the 102-bit seal in the audio itself (spread-transform dither modulation on log-spectral tiles, a convolutional code, a CRC and blind synchronisation; [spec section 10.2](spec/SPEC.md)). Measured on 13 recordings in 10 languages ([full results](bench/results/2026-10-01-stdm-1/README.md)):

| Through | Seal recovered (6.4 s windows) |
|---|---|
| MP3 64 and 128 kbit/s, AAC 64 kbit/s, resampling, trimming | 100 % |
| Opus 32 kbit/s, G.722 | about 99 % |
| Opus 24 kbit/s | 93 % (100 % with 9.6 s windows) |
| AMR-WB 12.65 kbit/s (phone calls), Opus 12 kbit/s, noise at 20 dB SNR, noise reduction, echo, tempo change | 0 to 3 % |
| **Wrong seal returned, or false alarm on unmarked audio** | **never** |

Perceptual quality of the marked audio: PESQ-WB 4.40 out of about 4.64. It **did not pass** the project's own gate, and a public carrier lets anyone copy a seal into other audio ([threat model, A11](spec/THREAT-MODEL.md)), so it is used for research and measurement only: the CLI, the demo and the verdicts use file mode. Re-run it with [`bench/`](bench/README.md).

## Where this fits

Vox Trust builds on and complements existing work rather than replacing it:

- [C2PA](https://spec.c2pa.org/) content credentials: provenance for files, with watermarks as "soft bindings".
- Audio watermarks such as [AudioSeal](https://github.com/facebookresearch/audioseal) and [WavMark](https://github.com/wavmark/wavmark), which could serve as carriers.
- Research on public-key speech provenance, e.g. [MerkleSpeech](https://arxiv.org/abs/2602.10166).
- Apps that rotate family code words (e.g. Trust Onion) and detection vendors attack the same scam from other sides.
- STIR/SHAKEN attests the caller's *number*, not the voice.

## Repository layout

```
spec/        specification, threat model, test vectors
crates/
  vox-trust-core/   seal layout, circle and public modes, file mode, policy, replay helpers, pairing text
  vox-trust-wasm/   the core as a WebAssembly module (plain C interface, no imports)
  vox-trust-cli/    the `vox-trust` command-line tool
  vox-trust-carrier/  experimental in-band carrier (research only)
  vox-trust-bench/  `vt-bench`, the carrier benchmark
web/         the browser demo (published at vox-trust.github.io/demo/)
tests/       Node (WebAssembly) and browser tests
tools/       independent vector check
fuzz/        fuzz targets (cargo-fuzz) and seed inputs
bench/       carrier benchmark: corpus script, quality script, published results
scripts/     build and publish helpers
docs/        roadmap, decision records
```

## Contributing, security, license

- Read [CONTRIBUTING.md](CONTRIBUTING.md). The most useful thing right now is scrutiny: attack the [threat model](spec/THREAT-MODEL.md), or write an independent implementation from the [spec](spec/SPEC.md).
- Report vulnerabilities **privately**: see [SECURITY.md](SECURITY.md).
- Code: [Apache-2.0](LICENSE). Specification text: CC BY 4.0, see [spec/LICENSE.md](spec/LICENSE.md). Changes: [CHANGELOG.md](CHANGELOG.md).
- Governance and name usage: [GOVERNANCE.md](GOVERNANCE.md), [TRADEMARKS.md](TRADEMARKS.md). To cite: [CITATION.cff](CITATION.cff).
- Maintainer: [Roger Oliveira](https://www.linkedin.com/in/rogeroliveira/) · [@rogeroliveira84](https://github.com/rogeroliveira84).
