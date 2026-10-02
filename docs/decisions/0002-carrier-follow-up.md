# 0002: Carrier follow-up: stdm-2, neural baselines, content binding

- **Status:** accepted, 2026-10-02
- **Data:** [stdm-2](../../bench/results/2026-10-02-stdm-2/README.md),
  [neural baselines](../../bench/results/2026-10-02-neural-baselines/README.md),
  [content binding](../../bench/results/2026-10-02-content-binding/README.md)
- **Follows:** [0001](0001-carrier-phase-0.md)

## Context

Decision 0001 left three directions: longer windows, a yardstick from published watermarks
before investing further, and content binding, the research problem that blocks any
in-band verdict. All three were measured on the same corpus, conditions and quality
metrics.

## What was measured

1. **stdm-2** (stdm-1 with 9.6 s windows) plus a detector tempo search. At the same quality
   (PESQ-WB 4.41): Opus 24 kbit/s 100 % (was 93 %), MP3 then Opus 98 % (was 65 %), AMR-WB
   23.85 kbit/s 89 % (was 58 %), a 1 % tempo change 84 % (was 0 %). AMR-WB 12.65 kbit/s
   11 %, noise at 20 dB SNR, noise reduction and echo still fail. No wrong seal, no false
   alarm.
2. **AudioSeal and WavMark**, as published, each carrying one 16-bit message per
   recording. AudioSeal survives noise at 30 dB, echo and Opus 12 kbit/s, where stdm-2
   fails, but fails G.722, trimming and AMR-WB. **WavMark survives noise reduction, echo
   and a 1 % tempo change, and partly AMR-WB 12.65 kbit/s** (31 % exact, 69 % detected),
   at a lower quality (PESQ-WB 4.12); it fails noise at 20 dB. These numbers are an upper
   bound for a seal: the message is repeated and combined over the whole recording.
3. **Content binding** with a robust fingerprint in the tag: a naive copy no longer binds,
   but an attacker who reshapes the fake's band energies matches the fingerprint, also
   when it is a projection secret to the circle.

## Decision

1. **stdm-2 replaces stdm-1 as the default experimental carrier**, and the detector searches
   tempo changes by default. stdm-1 stays available (`Params::stdm1()`); the two do not
   read each other's seals, as the registry rules require.
2. **No switch to a neural watermark yet, but WavMark is the lead to follow.** It is the
   only carrier measured that survives noise reduction and partly AMR-WB. Before any
   switch it must be measured carrying a real 102-bit seal (different bits in each 1 s
   segment, no repetition), and its cost (a network in every verifier, minutes per
   detection on CPU) weighed. Both models remain the yardstick for future carriers.
3. **The gate is still not passed, and in-band verdicts stay off.** The copy attack is the
   blocker, and the obvious fix (a robust fingerprint) is now measured not to work.

## Consequences and next steps

- Next measurement: WavMark (or a WavMark-like network) with a full 102-bit payload, on the
  whole condition set.
- Phone calls (AMR-WB 12.65 kbit/s) defeat stdm-2 and AudioSeal and only partly let WavMark
  through: a carrier built for model-based speech codecs remains the open robustness
  problem.
- Content binding needs something an attacker cannot reproduce without the original audio;
  until then file mode is the only mode with verdicts.
- The detector is about 8 times slower with the tempo search; acceptable for files, to be
  revisited for live audio.
