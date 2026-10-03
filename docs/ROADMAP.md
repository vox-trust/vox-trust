# Roadmap

An honest plan with the gates that decide whether to continue. No dates are promised: this is a part-time, early project.

Legend: ✅ done · 🟡 partly done · ⬜ not started

## Where we are (v0.6.0)

**Usable today, for one thing:** sealing a WAV file and verifying it, with the altered chunks shown, in the browser demo, the browser extension, the command line or your own app (JavaScript, Rust, Python, Android, iOS, an MCP server; [integration guide](INTEGRATION.md)). It is a **pre-1.0 (specification 0.2), not audited, and not ready to protect anyone.** Two big gaps remain, and they are the gaps that matter most:

- 🟡 **The audio watermark carrier.** An experimental carrier (stdm-1, now stdm-2) is built and measured against real codecs: it survives MP3, AAC, G.722, Opus at 24 kbit/s and above and small speed changes, but not phone-call codecs (AMR-WB), noise, noise reduction or echo, so it **did not pass the Phase 0 gate**. And because in-band seals are not bound to the audio, anyone can copy a seal into other audio (the copy attack). Until that is solved, only file mode gives verdicts. [Results](../bench/results/2026-10-01-stdm-1/README.md) · [decision](decisions/0001-carrier-phase-0.md)
- ⬜ **Independent review, an independent implementation, and everything that depends on other people.** Only the author has looked at this code and design (plus automated adversarial reviews, which are not a substitute).

What is done:

- ✅ Specification version 0.2: file mode fully specified, its version-0 format a release candidate with a defined time epoch, counter rules, versioning rules and a registry of identifiers (section 13); in-band seal as draft; the experimental carrier stdm-1 specified in enough detail to reimplement ([spec/SPEC.md](../spec/SPEC.md))
- ✅ Threat model version 0.2 with the author's own adversarial findings and explicit non-claims ([spec/THREAT-MODEL.md](../spec/THREAT-MODEL.md))
- ✅ `vox-trust-core`: seal layout, circle (HMAC-SHA-256) and public (Ed25519, strict) modes, **file mode**, trust policy, replay and rate-limit helpers, pairing text; strict WAV parsing; keys redacted from `Debug` and zeroized where the code controls them
- ✅ Published test vectors including **negative vectors**, byte-exact across Rust, WebAssembly and a Python cross-check (same author, so a cross-check, not independence)
- ✅ `vox-trust` command-line tool, hardened (regular files only, size caps, atomic non-overwriting `seal`, no panics on odd input); key files protected by a passphrase by default (Argon2id, XChaCha20-Poly1305), with a known-answer vector from an independent implementation
- ✅ WebAssembly build (reproducible, no imports) and a browser demo that **really verifies**, in **five languages** (English, Português, Español, 中文, العربية with RTL), tested in a real browser
- ✅ Website in the same five languages with SEO metadata
- ✅ CI: format, clippy, tests on Linux, macOS and Windows, docs, minimum supported Rust (1.94), line coverage with a floor, dependency advisories, licenses and sources (`cargo deny`), vectors, WebAssembly, the demo in Chromium, Firefox and WebKit, fuzzing; mutation testing of the core and the key-file code on every change to them and weekly
- ✅ Two internal adversarial review rounds, with fixes and regression tests
- ✅ Releases (from v0.2.0) published by a workflow, with binaries for Linux, macOS and Windows, the WebAssembly module and the demo, `SHA256SUMS` and build-provenance attestations; the WebAssembly module is reproducible byte for byte with the pinned toolchain
- ✅ Phase 0 measurement: carrier, reproducible benchmark (real codecs, perceptual quality, false alarms), published results and a decision record
- ⬜ Translations reviewed by native speakers (English is the normative text)

## Principles

1. **Measure before building.** The design leans on audio watermarks surviving real audio paths, so that comes first, in public.
2. **Security must not depend on the watermark surviving.** A removed or damaged watermark may only ever produce *Absent*, never a forged *Valid*.
3. **The specification is the product.** The code is one implementation of it.
4. **Claims must match the threat model.** If a sentence says "proves" or "guarantees", it needs an entry there.
5. **Adoption means independent implementations and integrations**, not star counts.

## Phase 0: measure first (done for stdm-1 and stdm-2: gate not passed)

**Outcome, 2026-10-01** ([results](../bench/results/2026-10-01-stdm-1/README.md), [decision record](decisions/0001-carrier-phase-0.md)):

- ✅ Carrier stdm-1 (spread-transform dither modulation on log-spectral tiles) and a reproducible benchmark harness with real codecs (ffmpeg), perceptual quality (PESQ-WB, STOI) and false-alarm measurement, on 13 recordings in 10 languages
- ✅ Survives: MP3 and AAC (100 %), G.722 and Opus 32 kbit/s (about 99 %), Opus 24 kbit/s (93 % with 6.4 s windows, 100 % with 9.6 s), resampling, trimming; PESQ-WB 4.40
- ❌ Fails: AMR-WB 12.65 kbit/s, Opus 12 kbit/s, noise at 20 dB SNR or worse, noise reduction, echo, tempo changes
- ✅ No wrong seal and no false alarm in any condition
- ❌ **Gate not passed** (Opus 24 kbit/s 92.6 % at the default point, AMR-WB 12.65 kbit/s 0 %). As planned, file mode stays the only mode and the carrier stays experimental.
- ✅ **stdm-2** (2026-10-02): 9.6 s windows and a detector tempo search. Opus 24 kbit/s 100 %, MP3 then Opus 98 %, 1 % tempo change 84 % (all were lower or 0); AMR-WB 12.65 kbit/s, noise, noise reduction and echo still fail ([results](../bench/results/2026-10-02-stdm-2/README.md))
- ✅ Published neural watermarks (AudioSeal, WavMark) measured on the same harness as a yardstick: WavMark survives noise reduction, echo, tempo and partly AMR-WB, where stdm-2 fails ([results](../bench/results/2026-10-02-neural-baselines/README.md))
- ✅ WavMark carrying a full 102-bit seal: measured 2026-10-03, negative (fails AMR-WB, noise reduction and noise without the repetition it relies on); [results](../bench/results/2026-10-03-wavmark-seal/README.md)
- ❌ **Content binding by a robust fingerprint** (2026-10-02): stops a naive copy, defeated by an adaptive attacker, also with a fingerprint secret to the circle ([study](../bench/results/2026-10-02-content-binding/README.md)). The copy attack stays open.
- ⬜ Still to measure: packet loss, AGC, reverb, speaker-to-microphone replay, neural-codec resynthesis, real app paths, fairness across speakers, a larger and noisier corpus, a listening test
- ⬜ A carrier designed for model-based speech codecs (AMR-WB, low-rate Opus)

The plan below is the original Phase 0 brief, kept for reference.

Conditions to test, per carrier backend and payload size:

- **Codecs:** Opus (6/12/24/32 kbit/s, voice and audio modes, packet loss), AAC, MP3, AMR-WB, and where licensing allows, EVS (benchmark use only, never shipped).
- **Processing:** noise suppression and speech enhancement, automatic gain control, resampling (8/16/48 kHz), reverb, speaker-to-microphone replay, small time-stretches.
- **Attacks:** neural-codec resynthesis, voice conversion, re-speaking, and published watermark-removal tools used as red-team inputs.
- **Real app paths:** voice notes and calls recorded and re-sent through common messaging and conferencing apps.
- **Fairness:** speaker, language, sex and age differences in robustness.

Metrics: bit error rate, tag verification success, false-accept rate, detection delay, audio quality, CPU, memory and size (including in WebAssembly).

Deliverables: a reproducible benchmark harness in this repository, published results, and a short written decision record.

**Proposed gate (to be revised against the data):** continue with watermark carrying only if a backend verifies ≥ 95% of the time through Opus 24 kbit/s, AAC 64 kbit/s and MP3 128 kbit/s, and ≥ 80% through AMR-WB 12.65 kbit/s, with a false-accept rate per window no worse than the tag's 2^-32 *per candidate tested*. If none does, file mode (already built) stays the only mode and watermark carrying stays experimental.

## Phase 1: core (specification 0.1 and 0.2)

- 🟡 Circle-mode tag (HMAC) and public-mode signature (Ed25519): done for file mode and for the in-band circle tag. COSE/C2PA alignment of the manifest is not done.
- 🟡 Pairing format: text format specified and implemented. QR rendering and a key store are application work, not done.
- ✅ Counter, time, clock-skew and replay rules; verifier rate limiting (library helpers, with tests)
- 🟡 Carrier backend interface and a first backend (stdm-1, experimental, specified in the spec's section 10.2, with unit tests and a fuzz target); a per-backend conformance test is not done
- ✅ **Test vectors** published as plain data, byte-exact through the Rust crate and the WebAssembly module
- ✅ Command-line `seal` and `verify` for files
- ✅ WebAssembly build and a browser demo that **really verifies** (no mock demo)
- ✅ Benchmark harness in the repository (`bench/`, plus an on-demand CI workflow)

**Exit:** someone other than the author reproduces the test vectors from the specification alone. ⬜ (needs another person; the Python check is the author's own)

## Phase 2: review and a second implementation (towards 1.0)

- ⬜ Independent review of the threat model and the cryptographic design
- 🟡 A second implementation written from the specification: `tools/check_vectors.py` re-implements the vectors from the spec text, but by the same author. An implementation by someone else is still needed.
- 🟡 Splice and replay mitigations: **file mode** binds every chunk digest to its index and content, and replay helpers exist. In-band seals are **not** bound to content (the copy attack, an open problem).
- ⬜ Decisions on key discovery and key revocation
- ⬜ Fixes for every ambiguity the second implementation exposes

**Exit:** two implementations written by different people agree on all test vectors.

## Phase 3: public release

- 🟡 A release with a working demo, the open threat model and published benchmark results: **v0.3.0** (the carrier did not pass its gate); **v0.4.0** adds protected keys, the versioning rules and mutation-tested core code; **v0.5.0** the stdm-2 carrier, a benchmark against published neural watermarks and a comparison with other approaches. **v0.5.1** packages for developers (npm, crates.io) and an integration guide. **v0.6.0** bindings for Android, iOS and Python, a browser extension, an MCP server, and WavMark measured with a full seal. A 1.0 waits for an outside review.
- ⬜ A "break the seal" challenge with published rules, a scoreboard and published fixes
- 🟡 Documentation: README, spec and CLI help exist; full install matrix and benchmark section do not
- ⬜ Private vulnerability reporting enabled in the repository settings and tested end to end (SECURITY.md already points to it, with a fallback)
- ✅ Fuzzing of every parser of untrusted input: six cargo-fuzz targets (including the carrier's detector) that check invariants, not only crashes. Locally they ran about 170 million inputs with no failure; CI runs them on every push and for 15 minutes each weekly. Fuzzing finds bugs, it does not prove their absence.
- ✅ Every GitHub Action pinned by commit hash
- ✅ Release workflow: binaries for Linux, macOS and Windows, the WebAssembly module and the demo, with `SHA256SUMS` and build-provenance attestations. Tags are created by GitHub, not GPG-signed; the attestations are the integrity check.

**Exit:** the first outside bug reports and reviews are triaged in public.

## Phase 4: plugins and integrations

- ✅ **Browser extension, file verifier** (v0.6.0): verifies sealed WAV files and links offline in Chrome, Edge, Firefox and Safari, with contacts from pairing text. It does not touch live audio.
- ⬜ **Browser extension, live calls:** seal the outgoing microphone stream before the encoder, verify the decoded remote audio. Known constraints: Manifest V3 content-security rules for WebAssembly, audio work in an offscreen document, no `SharedArrayBuffer` in content scripts, and insertable streams only expose *encoded* frames. It needs the carrier.
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

See [spec/SPEC.md](../spec/SPEC.md), section 14.
