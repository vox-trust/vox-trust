# Roadmap

An honest plan with the gates that decide whether to continue. No dates are promised: this is a part-time, early project.

Legend: ✅ done · ⬜ not started

## Where we are

- ✅ Draft specification 0.0: seal layout, modes, trust policy, open questions ([spec/SPEC.md](../spec/SPEC.md))
- ✅ Draft threat model ([spec/THREAT-MODEL.md](../spec/THREAT-MODEL.md))
- ✅ `vox-trust-core`: 102-bit seal packing and the trust-policy decision table, with tests; CI running
- ✅ Website in English and Portuguese, no trackers
- ⬜ Everything below

## Principles

1. **Measure before building.** The design leans on audio watermarks surviving real audio paths, so that comes first, in public.
2. **Security must not depend on the watermark surviving.** A removed or damaged watermark may only ever produce *Absent*, never a forged *Valid*.
3. **The specification is the product.** The code is one implementation of it.
4. **Claims must match the threat model.** If a sentence says "proves" or "guarantees", it needs an entry there.
5. **Adoption means independent implementations and integrations**, not star counts.

## Phase 0: measure first

Conditions to test, per carrier backend and payload size:

- **Codecs:** Opus (6/12/24/32 kbit/s, voice and audio modes, packet loss), AAC, MP3, AMR-WB, and where licensing allows, EVS (benchmark use only, never shipped).
- **Processing:** noise suppression and speech enhancement, automatic gain control, resampling (8/16/48 kHz), reverb, speaker-to-microphone replay, small time-stretches.
- **Attacks:** neural-codec resynthesis, voice conversion, re-speaking, and published watermark-removal tools used as red-team inputs.
- **Real app paths:** voice notes and calls recorded and re-sent through common messaging and conferencing apps.
- **Fairness:** speaker, language, sex and age differences in robustness.

Metrics: bit error rate, tag verification success, false-accept rate, detection delay, audio quality, CPU, memory and size (including in WebAssembly).

Deliverables: a reproducible benchmark harness in this repository, published results, and a short written decision record.

**Proposed gate (to be revised against the data):** continue with watermark carrying only if a backend verifies ≥ 95% of the time through Opus 24 kbit/s, AAC 64 kbit/s and MP3 128 kbit/s, and ≥ 80% through AMR-WB 12.65 kbit/s, with a false-accept rate per window no worse than the tag's 2^-32. If none does, the first release falls back to **file mode** (a sealed manifest, no watermark) and watermark carrying stays experimental.

## Phase 1: core (draft 0.1)

- ⬜ Circle-mode tag (HMAC) and public-mode signature (Ed25519, COSE manifest) using reviewed crates
- ⬜ Pairing format for circle mode (for example a QR payload) and a key store
- ⬜ Counter, time, clock-skew and replay rules; verifier rate limiting
- ⬜ Carrier backend interface, a first backend, and a per-backend conformance test
- ⬜ **Test vectors** (seal packing, tags, policy) published as plain data
- ⬜ Command-line `sign` and `verify` for files
- ⬜ WebAssembly build and a browser demo that **really verifies** (no mock demo)
- ⬜ Benchmark harness in the repository

**Exit:** someone other than the author reproduces the test vectors from the specification alone.

## Phase 2: review and a second implementation (draft 0.2)

- ⬜ Independent review of the threat model and the cryptographic design
- ⬜ A second implementation in another language, written from the specification
- ⬜ Splice and replay mitigations: per-chunk commitments (public mode) and continuity checks
- ⬜ Decisions on key discovery and key revocation
- ⬜ Fixes for every ambiguity the second implementation exposes

**Exit:** both implementations agree on all test vectors.

## Phase 3: public release

- ⬜ A release with a working demo, the benchmark results and the open threat model
- ⬜ A "break the seal" challenge with published rules, a scoreboard and published fixes
- ⬜ Documentation: quick start, full benchmark section, install matrix
- ⬜ Private vulnerability reporting tested end to end

**Exit:** the first outside bug reports and reviews are triaged in public.

## Phase 4: plugins and integrations

- ⬜ **Browser extension:** seal the outgoing microphone stream before the encoder, verify the decoded remote audio. Known constraints: Manifest V3 content-security rules for WebAssembly, audio work in an offscreen document, no `SharedArrayBuffer` in content scripts, and insertable streams only expose *encoded* frames.
- ⬜ **Desktop verifier:** local only, no network, visible "listening" indicator. Capture paths: WASAPI loopback (Windows), PipeWire/PulseAudio monitor sources (Linux), process taps or ScreenCaptureKit (macOS).
- ⬜ **Share-sheet flow** for voice notes on mobile (manual: seal, send, verify).
- ⬜ **Tamper localisation** shown per chunk.
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

See [spec/SPEC.md](../spec/SPEC.md), section 11.
