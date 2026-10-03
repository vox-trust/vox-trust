# WavMark carrying a full 102-bit seal, 2026-10-03

**Bottom line.** WavMark was the lead from [decision 0002](../../../docs/decisions/0002-carrier-follow-up.md):
with one 16-bit message repeated over the whole recording it survived noise reduction, echo
and in part AMR-WB. Carrying a real seal (eight segments of different bits, nothing
repeated) it loses all of that: **0 % through AMR-WB 12.65 kbit/s, noise reduction, noise
at 30 and 20 dB and Opus 12 kbit/s**, and it read two wrong seals through echo. It is no
better than stdm-2 on any condition, so the lead is closed.

**This was predictable from the published numbers, and the run was stopped early.** The
WavMark paper ([arXiv 2308.12770](https://arxiv.org/abs/2308.12770)) reports a 2.35 %
bit error rate for its 32 bit/s model without repetition, over ten attacks (noise, MP3,
low-pass, speed changes); its 0.48 % figure relies on repeating the message over 10 to
20 s. A seal must get all 104 bits right: 0.9765^104 ≈ 8 % without error correction. The
paper does not measure AMR-WB, but the 16-bit run with repetition was already an upper bound
(31 %). We should have done this arithmetic before measuring; the partial run below only
confirms it.

## Results (partial)

8 of the 13 recordings, 3 to 11 seals per condition (`results.json`). Small samples:
read the numbers as "works", "partly" or "fails", not as precise rates.

| Condition | Seals exact | Segments right | stdm-2 (windows exact) |
|---|---:|---:|---:|
| `original` | 82 % | 97 % | 100 % |
| `trim-1.234s` | 67 % (3 seals) | 100 % | 100 % |
| `mp3-64k` | 80 % | 96 % | 100 % |
| `opus-24k` | 43 % | 88 % | 100 % |
| `opus-12k` | 0 % | 9 % | 0 % |
| `g722` | 60 % | 93 % | 100 % |
| `amr-wb-23.85k` | 0 % | 60 % | 89 % |
| `amr-wb-12.65k` | 0 % | 5 % | 11 % |
| `noise-30db` | 0 % | 28 % | 51 % |
| `noise-20db` | 0 % | 0 % | 0 % |
| `denoise` | 0 % | 65 % | 7 % |
| `echo` | 0 % (2 wrong seals) | 68 % | 0 % |
| `tempo+1%` | 60 % | 93 % | 84 % |
| PESQ-WB, mean (min) | 4.19 (4.03) | | 4.41 (3.93) |

Even the original audio loses seals: WavMark skips segments where the watermark would be
too loud (silences), and a seal needs all eight. False alarms on unmarked audio were not
reached before the run was stopped.

## Method

`bench/wavmark_seal.py`: the published WavMark model and its 16-bit sync pattern; each 1 s
segment carries a 3-bit index and 13 seal bits, segments 1.1 s apart, eight per seal; the
detector is WavMark's own search, and a seal is read when the eight indexes appear in order.
Same corpus, conditions and ffmpeg as stdm-2. Each detection is saved as it finishes, so
`--merge` sums whatever was completed.
