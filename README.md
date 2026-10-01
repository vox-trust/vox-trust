<h1 align="center">Vox Trust</h1>

<p align="center"><strong>Don't detect fake voices. Prove real ones.</strong></p>

<p align="center">
An open protocol, with a Rust reference implementation, that seals a human voice at the source and lets anyone verify it later.
</p>

<p align="center">
  <a href="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue"></a>
  <img alt="Status: pre-alpha" src="https://img.shields.io/badge/status-pre--alpha-orange">
  <img alt="Spec: draft 0.0" src="https://img.shields.io/badge/spec-draft%200.0-lightgrey">
</p>

<p align="center">
  <a href="https://vox-trust.github.io">Website</a> ·
  <a href="spec/SPEC.md">Draft spec</a> ·
  <a href="spec/THREAT-MODEL.md">Threat model</a> ·
  <a href="docs/ROADMAP.md">Roadmap</a>
</p>

> **Pre-alpha.** The spec is a draft, the code is a skeleton, nothing has been reviewed or audited, and there is **no live demo yet**. Do not use this to protect anyone or to make security decisions. We will not ship a demo that fakes verification.

## The problem

Cloning a voice now takes seconds of audio. Listening can no longer tell a real voice from a cloned one, and detectors chase a moving target: as detection improves, so does generation.

## The idea

Instead of guessing whether a voice is fake, check whether a real one was **sealed**.

1. **Seal.** The speaker's device adds a signed seal to the audio at the source.
2. **Carry.** The seal rides in a pluggable audio watermark, so it can survive re-recording and re-encoding (how well is an open measurement question, see the [roadmap](docs/ROADMAP.md)).
3. **Verify.** Anyone with the protocol checks which key sealed the audio, when, and which seconds were altered.
4. **Decide.** A local trust policy turns the result into one of three outcomes.

| Outcome | Meaning |
|---|---|
| **Verified** | A valid seal from a key you trust. |
| **Unsealed** | No seal, from someone who never used the protocol. Neutral, **not** "fake". |
| **Alert** | The seal is invalid, or it is missing from a contact who *always* seals (strict mode). |

Two modes are planned: a **circle** mode for people who know each other (shared secret, short codes) and a **public** mode for organisations and public figures (public-key signatures, keys published by the signer).

## What this is NOT

- It is **not** deepfake detection and **not** voice biometrics.
- It does **not** prove a speaker is human or is who they claim. It proves that *a key* sealed the audio. A stolen key or a compromised device produces valid seals.
- "No seal" does **not** mean "fake": compression and noise suppression can erase a watermark.
- It does not protect against an attacker who controls the sender's device.

The full list of attackers and limits is in the [threat model](spec/THREAT-MODEL.md).

## Where this fits

Vox Trust builds on and complements existing work rather than replacing it:

- [C2PA](https://spec.c2pa.org/) content credentials: provenance for files, with watermarks as "soft bindings".
- Audio watermarks such as [AudioSeal](https://github.com/facebookresearch/audioseal) and [WavMark](https://github.com/wavmark/wavmark), which can serve as backends.
- Research on public-key speech provenance, e.g. [MerkleSpeech](https://arxiv.org/abs/2602.10166).
- Apps that rotate family code words (e.g. Trust Onion) and detection vendors; they attack the same scam from other sides.
- STIR/SHAKEN attests the caller's *number*, not the voice.

## Repository layout

```
spec/        draft specification and threat model
crates/      Rust workspace (reference implementation)
docs/        roadmap and notes
```

Currently the workspace contains only `vox-trust-core`: the seal layout and the trust-policy decision logic, with no cryptography and no audio yet.

## Contributing, security, license

- Read [CONTRIBUTING.md](CONTRIBUTING.md). Design criticism of the threat model is welcome as a public issue.
- Report vulnerabilities **privately**: see [SECURITY.md](SECURITY.md).
- Code: [Apache-2.0](LICENSE). Specification text: CC BY 4.0, see [spec/LICENSE.md](spec/LICENSE.md).
- Governance and name usage: [GOVERNANCE.md](GOVERNANCE.md), [TRADEMARKS.md](TRADEMARKS.md).
- To cite this work: [CITATION.cff](CITATION.cff).
