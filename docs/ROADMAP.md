# Roadmap

Honest plan, with the gates that decide whether to continue. Dates are not promised; this is a part-time, early project.

## Phase 0: measure first

The whole design leans on audio watermarks surviving real audio paths. Before building more, **measure**, in public.

Conditions to test, per backend and payload size:

- **Codecs:** Opus (6/12/24/32 kbit/s), AAC, MP3, AMR-WB, and where licensing allows, EVS (benchmark use only).
- **Processing:** noise suppression and speech enhancement, automatic gain control, resampling (8/16/48 kHz), reverb, speaker-to-microphone replay, small time-stretches.
- **Attacks:** neural-codec resynthesis, voice conversion, re-speaking, and published watermark-removal tools used as red-team inputs.
- **Real app paths:** voice notes and calls recorded and re-sent through common messaging and conferencing apps.

Metrics: bit error rate, tag verification success, false-accept rate, detection delay, audio quality, CPU and size (including in WebAssembly).

**Proposed gate (to be revised against the data):** continue with watermark carrying only if a backend verifies ≥ 95% of the time through Opus 24 kbit/s, AAC 64 kbit/s and MP3 128 kbit/s, and ≥ 80% through AMR-WB 12.65 kbit/s, with a false-accept rate per window no worse than the tag's 2^-32. If none does, the first release falls back to **file mode** (sealed manifest, no watermark) and watermark carrying stays experimental.

## Phase 1: core

Specification draft 0.1, `vox-trust-core` (seal, signing, verification, policy), test vectors, a CLI, and a browser (WebAssembly) demo that really verifies. The benchmark harness lives in the repository.

## Phase 2: review

Independent review of the threat model and the cryptographic design, and a **second implementation in another language** to check that the spec is implementable from the text alone.

## Phase 3: public release

A public announcement with a working demo, the benchmark results, and a "break the seal" challenge with published rules.

## Later

Browser extension, tamper localisation per chunk, a desktop verifier, live audio (delayed key disclosure), a bridge to C2PA manifests, and moving the specification to a neutral home once there are two interoperating implementations and outside interest.

## Not planned

A consumer app that competes with existing family code-word apps; deepfake detection; securing ordinary phone calls.
