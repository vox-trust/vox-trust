# Roadmap

An honest plan with the gates that decide whether to continue. No dates are promised: this is a part-time, early project.

Legend: ✅ done · 🟡 partly done · ⬜ not started

## Where we are (v0.2.0)

**Usable today, for one thing:** sealing a WAV file and verifying it, with the altered chunks shown, in the browser demo or the command line. It is a **v0.x draft, not audited, and not ready to protect anyone.** Two big gaps remain, and they are the gaps that matter most:

- ⬜ **The audio watermark carrier.** Without it, seals survive only bit-exact copies. Re-encoding (MP3, AAC, WhatsApp voice notes), resampling and re-recording all read as modified. Nothing here has been measured against real codecs.
- ⬜ **Independent review, an independent implementation, and everything that depends on other people.** Only the author has looked at this code and design (plus automated adversarial reviews, which are not a substitute).

What is done:

- ✅ Specification draft 0.1: file mode fully specified; in-band seal and carrier interface as draft ([spec/SPEC.md](../spec/SPEC.md))
- ✅ Threat model draft 0.1 with the author's own adversarial findings and explicit non-claims ([spec/THREAT-MODEL.md](../spec/THREAT-MODEL.md))
- ✅ `vox-trust-core`: seal layout, circle (HMAC-SHA-256) and public (Ed25519, strict) modes, **file mode**, trust policy, replay and rate-limit helpers, pairing text; strict WAV parsing; keys redacted from `Debug` and zeroized where the code controls them
- ✅ Published test vectors including **negative vectors**, byte-exact across Rust, WebAssembly and a Python cross-check (same author, so a cross-check, not independence)
- ✅ `vox-trust` command-line tool, hardened (regular files only, size caps, atomic non-overwriting `seal`, no panics on odd input)
- ✅ WebAssembly build (reproducible, no imports) and a browser demo that **really verifies**, in **five languages** (English, Português, Español, 中文, العربية with RTL), tested in a real browser
- ✅ Website in the same five languages with SEO metadata
- ✅ CI: format, clippy, tests, docs, minimum supported Rust (1.94), dependency advisories, vectors, WebAssembly, real-browser demo, fuzzing
- ✅ Two internal adversarial review rounds, with fixes and regression tests
- 🟡 Release: v0.2.0 is on `main`; the Git tag and GitHub Release still have to be created (the tag push is blocked in the authoring environment)
- ⬜ Translations reviewed by native speakers (English is the normative text)

## Principles

1. **Measure before building.** The design leans on audio watermarks surviving real audio paths, so that comes first, in public.
2. **Security must not depend on the watermark surviving.** A removed or damaged watermark may only ever produce *Absent*, never a forged *Valid*.
3. **The specification is the product.** The code is one implementation of it.
4. **Claims must match the threat model.** If a sentence says "proves" or "guarantees", it needs an entry there.
5. **Adoption means independent implementations and integrations**, not star counts.

## Phase 0: measure first (not started: needs a carrier)

Conditions to test, per carrier backend and payload size:

- **Codecs:** Opus (6/12/24/32 kbit/s, voice and audio modes, packet loss), AAC, MP3, AMR-WB, and where licensing allows, EVS (benchmark use only, never shipped).
- **Processing:** noise suppression and speech enhancement, automatic gain control, resampling (8/16/48 kHz), reverb, speaker-to-microphone replay, small time-stretches.
- **Attacks:** neural-codec resynthesis, voice conversion, re-speaking, and published watermark-removal tools used as red-team inputs.
- **Real app paths:** voice notes and calls recorded and re-sent through common messaging and conferencing apps.
- **Fairness:** speaker, language, sex and age differences in robustness.

Metrics: bit error rate, tag verification success, false-accept rate, detection delay, audio quality, CPU, memory and size (including in WebAssembly).

Deliverables: a reproducible benchmark harness in this repository, published results, and a short written decision record.

**Proposed gate (to be revised against the data):** continue with watermark carrying only if a backend verifies ≥ 95% of the time through Opus 24 kbit/s, AAC 64 kbit/s and MP3 128 kbit/s, and ≥ 80% through AMR-WB 12.65 kbit/s, with a false-accept rate per window no worse than the tag's 2^-32 *per candidate tested*. If none does, file mode (already built) stays the only mode and watermark carrying stays experimental.

## Phase 1: core (draft 0.1)

- 🟡 Circle-mode tag (HMAC) and public-mode signature (Ed25519): done for file mode and for the in-band circle tag. COSE/C2PA alignment of the manifest is not done.
- 🟡 Pairing format: text format specified and implemented. QR rendering and a key store are application work, not done.
- ✅ Counter, time, clock-skew and replay rules; verifier rate limiting (library helpers, with tests)
- ⬜ Carrier backend interface, a first backend, and a per-backend conformance test
- ✅ **Test vectors** published as plain data, byte-exact through the Rust crate and the WebAssembly module
- ✅ Command-line `seal` and `verify` for files
- ✅ WebAssembly build and a browser demo that **really verifies** (no mock demo)
- ⬜ Benchmark harness in the repository

**Exit:** someone other than the author reproduces the test vectors from the specification alone. ⬜ (needs another person; the Python check is the author's own)

## Phase 2: review and a second implementation (draft 0.2)

- ⬜ Independent review of the threat model and the cryptographic design
- 🟡 A second implementation written from the specification: `tools/check_vectors.py` re-implements the vectors from the spec text, but by the same author. An implementation by someone else is still needed.
- 🟡 Splice and replay mitigations: **file mode** binds every chunk digest to its index and content, and replay helpers exist. In-band seals are **not** bound to content (the copy attack, an open problem).
- ⬜ Decisions on key discovery and key revocation
- ⬜ Fixes for every ambiguity the second implementation exposes

**Exit:** two implementations written by different people agree on all test vectors.

## Phase 3: public release

- 🟡 A release with a working demo and the open threat model: **code on `main` as v0.2.0**; tag and GitHub Release pending; no benchmark results (there is no carrier to measure)
- ⬜ A "break the seal" challenge with published rules, a scoreboard and published fixes
- 🟡 Documentation: README, spec and CLI help exist; full install matrix and benchmark section do not
- ⬜ Private vulnerability reporting enabled in the repository settings and tested end to end (SECURITY.md already points to it, with a fallback)
- ✅ Fuzzing of every parser of untrusted input: five cargo-fuzz targets that check invariants, not only crashes. Before v0.2.0 they ran about 170 million inputs locally with no failure; CI runs them on every push and for 15 minutes each weekly. Fuzzing finds bugs, it does not prove their absence.
- ✅ Every GitHub Action pinned by commit hash
- 🟡 Release workflow: binaries for Linux, macOS and Windows, the WebAssembly module and the demo, with `SHA256SUMS` and build-provenance attestations. Tags are created by GitHub, not GPG-signed; the attestations are the integrity check.

**Exit:** the first outside bug reports and reviews are triaged in public.

## Phase 4: plugins and integrations

- ⬜ **Browser extension:** seal the outgoing microphone stream before the encoder, verify the decoded remote audio. Known constraints: Manifest V3 content-security rules for WebAssembly, audio work in an offscreen document, no `SharedArrayBuffer` in content scripts, and insertable streams only expose *encoded* frames. It needs the carrier.
- ⬜ **Desktop verifier:** local only, no network, visible "listening" indicator. Capture paths: WASAPI loopback (Windows), PipeWire/PulseAudio monitor sources (Linux), process taps or ScreenCaptureKit (macOS).
- ⬜ **Share-sheet flow** for voice notes on mobile (manual: seal, send, verify). File mode already works for this if the messenger keeps the file bit-exact (most re-encode voice notes).
- ✅ **Tamper localisation** shown per chunk (file mode, in the demo and the CLI)
- ⬜ Integrations with open communication tools.

**Exit:** at least one integration used by someone other than the author.

## Phase 5: live audio

- ⬜ Delayed key disclosure (TESLA-style, RFC 4082) to keep seals small; the verdict arrives seconds late
- ⬜ Measurements on telephony paths (AMR-WB, EVS) and an honest account of platform limits
- ⬜ Key revocation for live use

## Phase 6: standards path

- ⬜ A bridge to C2PA manifests for audio (watermark as a soft binding)
- ⬜ An individual Internet-Draft for the wire format
- ⬜ Patent policy for the specification, published **before** any version is declared stable
- ⬜ A second maintainer and a written decision process
- ⬜ Moving the specification to a neutral home once there are two interoperating implementations and real outside interest

## What success looks like

Targets, not promises, for the first 6 to 12 months:

- At least **two independent implementations** that interoperate
- At least **three external written reviews**
- At least **one third-party integration**
- A public, reproducible benchmark
- Not: a star count

## Stop or change course if

- No backend passes the Phase 0 gate and file mode finds no use either
- A serious flaw shows that the trust policy does not hold
- The problem is solved better elsewhere, in which case the right move is to say so and contribute there

## Not planned

A consumer app that competes with existing family code-word apps; deepfake detection; securing ordinary phone calls (platforms do not expose call audio to third-party software).

## Open questions

See [spec/SPEC.md](../spec/SPEC.md), section 13.
