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
  <img alt="Version 0.1.0" src="https://img.shields.io/badge/version-0.1.0-informational">
  <img alt="Not audited" src="https://img.shields.io/badge/security-not%20audited-red">
</p>

<p align="center">
  <a href="https://vox-trust.github.io/demo/"><strong>Live demo</strong></a> ·
  <a href="https://vox-trust.github.io">Website</a> ·
  <a href="spec/SPEC.md">Spec</a> ·
  <a href="spec/THREAT-MODEL.md">Threat model</a> ·
  <a href="docs/ROADMAP.md">Roadmap</a>
</p>

> **Status: v0.1, file mode works. Not audited.** You can seal a WAV file, verify it, and see exactly which seconds were altered, in the [browser demo](https://vox-trust.github.io/demo/) or with the command line. Sealed files survive **only bit-exact copies**. The audio watermark that would survive re-encoding **is not built yet**. Do not use this to protect anyone until it has been reviewed.

## The problem

Cloning a voice now takes seconds of audio. Listening can no longer tell a real voice from a cloned one, and detectors chase a moving target: as detection improves, so does generation.

## The idea

Instead of guessing whether a voice is fake, check whether a real one was **sealed**.

1. **Seal.** The speaker's device signs the audio at the source.
2. **Verify.** Anyone with the protocol checks which key sealed it, when, and which chunks of audio were altered.
3. **Decide.** A local trust policy turns the result into one of three outcomes.

| Outcome | Meaning |
|---|---|
| **Verified** | A valid seal from a key you trust. |
| **Unsealed** | No seal, from someone who never used the protocol. Neutral, **not** "fake". |
| **Alert** | The seal is broken, comes from a key you didn't pin for that contact, or is missing from a contact who *always* seals (strict mode). |

Two modes: **circle** (people who know each other, shared secret) and **public** (organisations and public figures, Ed25519 keys that verifiers pin).

## Try it in 30 seconds

**In the browser** (nothing is uploaded; the Rust core runs as WebAssembly): open the [demo](https://vox-trust.github.io/demo/), press *Seal*, then try to cheat with the buttons and watch the verifier catch each attempt.

**On the command line:**

```sh
cargo install --git https://github.com/vox-trust/vox-trust vox-trust-cli

vox-trust keygen me.key
vox-trust seal speech.wav sealed.wav --mode circle --key me.key
vox-trust verify sealed.wav --circle-key me.key          # exit code 0 = verified
```

Edit a single sample of `sealed.wav` and verify again: the exit code becomes 3 and the altered chunk is printed. Exit codes: 0 verified, 1 unsealed, 2 warning, 3 alert.

**As a Rust library:**

```rust
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
- File mode does **not** survive MP3, AAC, resampling or re-recording. That needs the carrier, which does not exist yet.
- It does not protect against an attacker who controls the sender's device.

Attackers, claims and the weaknesses found so far (including one that is still unsolved) are in the [threat model](spec/THREAT-MODEL.md).

## How it is tested

| Layer | What it checks | Run |
|---|---|---|
| Rust unit and integration tests | Seal layout, HMAC and Ed25519 (against RFC 4231 and RFC 8032 vectors), WAV parsing, manifests, tampering, policy, the C interface and the CLI | `cargo test --workspace` |
| Published test vectors | Byte-exact seals and manifests in [`spec/test-vectors/`](spec/test-vectors) | included above |
| Independent re-implementation | `tools/check_vectors.py` rebuilds every vector from the spec text with Python's standard library | `python3 tools/check_vectors.py --strict` |
| WebAssembly end to end | The compiled module reproduces the vectors byte for byte, handles garbage input, and does not leak memory | `scripts/build-web.sh && node --test tests/node/wasm.test.mjs` |
| Real browser | The demo page in headless Chromium: sealing, six attacks, public-key pinning, accessibility, phone width, dark mode | `node tests/browser/demo.mjs` |

The Python check is written by the same author, so it is a cross-check, not an independent implementation. [An independent implementation is what the spec still needs.](docs/ROADMAP.md)

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
web/         the browser demo (published at vox-trust.github.io/demo/)
tests/       Node (WebAssembly) and browser tests
tools/       independent vector check
scripts/     build and publish helpers
docs/        roadmap
```

## Contributing, security, license

- Read [CONTRIBUTING.md](CONTRIBUTING.md). The most useful thing right now is scrutiny: attack the [threat model](spec/THREAT-MODEL.md), or write an independent implementation from the [spec](spec/SPEC.md).
- Report vulnerabilities **privately**: see [SECURITY.md](SECURITY.md).
- Code: [Apache-2.0](LICENSE). Specification text: CC BY 4.0, see [spec/LICENSE.md](spec/LICENSE.md). Changes: [CHANGELOG.md](CHANGELOG.md).
- Governance and name usage: [GOVERNANCE.md](GOVERNANCE.md), [TRADEMARKS.md](TRADEMARKS.md). To cite: [CITATION.cff](CITATION.cff).
- Maintainer: [Roger Oliveira](https://www.linkedin.com/in/rogeroliveira/) · [@rogeroliveira84](https://github.com/rogeroliveira84).
