# Carrier benchmark: stdm-2, 2026-10-02

**Bottom line.** stdm-2 is stdm-1 with 9.6 s windows instead of 6.4 s, and a detector that
also searches tempo changes of up to 2 %. On the same corpus and conditions, at the same
quality, it survives Opus 24 kbit/s (100 %, was 93 %), MP3 then Opus re-sharing (98 %, was
65 %) and a 1 % tempo change (84 %, was 0 %), with no wrong seal and no false alarm. It
still fails AMR-WB 12.65 kbit/s (11 %), noise at 20 dB SNR, noise reduction and echo, so
it **does not pass** the Phase 0 gate either, and the copy attack (threat model A11) still
blocks any in-band verdict ([study](../2026-10-02-content-binding/README.md)).

Same corpus (13 recordings, 10 languages), conditions, ffmpeg 7.0.2 and method as
[stdm-1](../2026-10-01-stdm-1/README.md); 3 seals per recording, 45 windows per condition
(81 for stdm-1, whose windows are shorter).

## stdm-1 and stdm-2 side by side

| Condition | stdm-1 (6.4 s) | stdm-2 (9.6 s, tempo search) |
|---|---:|---:|
| `original` | 100.0 % | 100.0 % |
| `resample-44k` | 100.0 % | 100.0 % |
| `trim-1.234s` | 100.0 % | 100.0 % |
| `noise-30db` | 23.5 % | 51.1 % |
| `noise-20db` | 0.0 % | 0.0 % |
| `noise-10db` | 0.0 % | 0.0 % |
| `mp3-128k` | 100.0 % | 100.0 % |
| `mp3-64k` | 100.0 % | 100.0 % |
| `aac-64k` | 100.0 % | 100.0 % |
| `opus-32k` | 98.8 % | 100.0 % |
| `opus-24k` | 92.6 % | **100.0 %** |
| `opus-12k` | 2.5 % | 0.0 % |
| `amr-wb-12.65k` | 0.0 % | 11.1 % |
| `amr-wb-23.85k` | 58.0 % | 88.9 % |
| `g722` | 98.8 % | 100.0 % |
| `mp3-128k+opus-24k` | 65.4 % | **97.8 %** |
| `denoise` | 0.0 % | 6.7 % |
| `echo` | 0.0 % | 0.0 % |
| `tempo+1%` | 0.0 % | **84.4 %** |
| Wrong seals, all conditions | 0 | 0 |
| False alarms on unmarked audio | 0 in 266 runs | 0 in 266 runs |
| PESQ-WB, mean (min) | 4.40 (4.06) | 4.41 (3.93) |
| STOI, mean | 0.971 | 0.973 |
| Capacity | 15.9 bit/s | 10.6 bit/s |

"Max unmarked sync score" in `results.md` (4.01 for a threshold of 6) covers the search at
the original tempo only; the false-alarm count covers the full tempo search.

**Costs.** A seal needs 9.6 s of audio instead of 6.4 s. Detection analyses the audio at
9 tempos, so it is about 8 times slower (189 s of detection for the whole run, against
24.5 s for stdm-1 on 2026-10-01, on different machines; still much faster than real
time).

## Can tuning buy noise and phone-call robustness?

A sweep of stdm-2 parameters on the hard conditions (3 seals per recording, tempo search
off to save time; `sweep.json`):

| Variant | noise 30 dB | AMR-WB 12.65k | AMR-WB 23.85k | Opus 24k, G.722, MP3 64k | noise reduction | PESQ-WB mean (min) |
|---|---:|---:|---:|---:|---:|---:|
| stdm-2 (step 7 dB) | 51 % | 11 % | 89 % | 100 % | 7 % | 4.41 (3.93) |
| step 9 dB, max 5.5 dB | 62 % | 20 % | 96 % | 100 % | 9 % | 4.20 (3.53) |
| step 11 dB, max 6.5 dB | 64 % | 24 % | 89 % | 100 % | 16 % | 3.91 (3.07) |
| step 9 dB, valleys under 20 dB ignored | 49 % | 16 % | 84 % | 100 % | 2 % | 4.23 (3.64) |
| valleys under 20 dB ignored | 29 % | 0 % | 64 % | 91 to 100 % | 0 % | 4.43 (4.01) |
| one sync tile in 8 | 11 % | 0 % | 62 % | 96 to 100 % | 0 % | 4.42 (3.96) |

Windows of 12.8 s do not fit the shorter recordings and were dropped. **Tuning buys about
ten points of noise robustness and at most 24 % through AMR-WB 12.65k, and costs audible
quality.** The default stays at step 7 dB. AMR-WB rebuilds speech from a model of its
spectral envelope and excitation, and stdm chips live in the fine detail it discards; a
carrier for phone calls has to live in what the codec keeps (for example the envelope's
line spectral frequencies) or be trained for it, as WavMark is.

## What changed and why

- **Window:** the 9.6 s operating point was already measured on 2026-10-01 ("long window")
  and was better almost everywhere at the same quality. Longer windows put more chips in
  every group, so each coded bit survives more damage.
- **Tempo search:** a pitch-preserving speed change stretches time, so the pattern drifts
  out of alignment over a window. The detector now also analyses the audio with frames
  `256 / s` samples apart for `s` from 0.98 to 1.02 in steps of 0.005, which re-aligns it
  without moving frequencies. The format does not change; stdm-1 audio could be searched
  the same way.

## Files

`results.md` (full table), `results.json`, `quality.json`.

## Reproduce

```sh
pip install imageio-ffmpeg==0.6.0 pesq pystoi numpy soundfile
FF=$(python3 -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())")
bench/fetch-corpus.sh "$FF"
cargo build --release -p vox-trust-bench
target/release/vt-bench --ffmpeg "$FF" --out bench/results/runs/stdm-2 --seals 3
python3 bench/quality.py bench/results/runs/stdm-2
```
