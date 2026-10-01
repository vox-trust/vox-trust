# 0001: Phase 0 carrier measurement, and what follows

- **Status:** accepted, 2026-10-01
- **Data:** [bench/results/2026-10-01-stdm-1](../../bench/results/2026-10-01-stdm-1/README.md)

## Context

The roadmap's Phase 0 asks whether an audio watermark can carry the 102-bit seal through real
audio paths, with a gate: at least 95 % of windows through Opus 24 kbit/s, AAC 64 kbit/s and
MP3 128 kbit/s, and at least 80 % through AMR-WB 12.65 kbit/s, with false accepts no worse
than the tag's own 2^-32 per candidate. If no backend passes, "file mode stays the only mode
and watermark carrying stays experimental".

## What was built and measured

A first carrier, stdm-1: spread-transform dither modulation (Chen and Wornell, 2001) on
normalised log-magnitude STFT tiles, a rate-1/2 convolutional code with a CRC-16, and blind
synchronisation over every time offset. Before it, plain and improved spread spectrum were
tried on the same features and rejected: with the distortion kept small they could not cancel
the speech's own interference (20 to 30 % raw bit errors before any codec).

Measured on 13 recordings in 10 languages, four operating points:

- **Works:** MP3 (64 and 128 kbit/s) and AAC 64 kbit/s 100 %; G.722 and Opus 32 kbit/s about
  99 %; Opus 24 kbit/s 93 % (6.4 s windows) or 100 % (9.6 s windows); resampling and blind
  synchronisation after trimming 100 %. PESQ-WB 4.40 on average for the default point.
- **Fails:** AMR-WB 12.65 kbit/s (0 to 11 %), Opus 12 kbit/s, white noise at 20 dB SNR or
  worse, noise reduction, echo, a 1 % tempo change.
- **Never:** a wrong seal returned, or a false alarm on unmarked audio (0 in 266 runs per
  operating point; the highest synchronisation score on unmarked audio was 4.1 for a
  threshold of 6.0, before the CRC and the seal's own tag).

## Decision

1. **The gate is not passed.** As the roadmap says, file mode remains the only supported
   mode; the carrier stays experimental, in its own crate, not used by the CLI, the
   WebAssembly module or the demo.
2. **No verdict from an in-band seal is shown to users** until the copy attack (threat model
   A11) is addressed: a public carrier lets anyone move a seal from one recording to another.
   This is independent of robustness and is the bigger blocker.
3. **Keep the benchmark** as the yardstick: any future carrier (another classic design, a
   codec-domain design for AMR, or a neural watermark) is measured on the same harness, with
   the same rules, and published whatever the result.

## Consequences and next steps

- Most promising measured direction for file-sharing paths: longer windows (9.6 s reached
  100 % through Opus 24 kbit/s at the same quality).
- For phone calls (AMR-WB), a different carrier is needed; this one is not the right design.
- Content binding (signing something robust about the audio itself, so a moved seal no longer
  verifies) is the research problem to solve before any in-band verdict.
- A larger and harder corpus (spontaneous, noisy, phone-recorded speech) and a formal listening
  test are needed before any robustness claim beyond "on this corpus".
