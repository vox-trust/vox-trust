# Carrier benchmark: stdm-1, 2026-10-01

**Bottom line.** The experimental in-band carrier survives common file codecs at a high
objective quality: MP3 and AAC 100 %, G.722 and Opus 32 kbit/s about 99 %, Opus 24 kbit/s
93 % with 6.4 s windows (100 % with 9.6 s windows). It does **not** survive AMR-WB 12.65
kbit/s (mobile calls), Opus 12 kbit/s, additive noise at 20 dB SNR or worse, noise
reduction, echo, or a 1 % tempo change. It **does not pass** the roadmap's Phase 0 gate
(at least 95 % through Opus 24 kbit/s *and* at least 80 % through AMR-WB 12.65 kbit/s).
Across every condition and operating point it **never returned a wrong seal and never
raised a false alarm** on unmarked audio. Decision: see
[docs/decisions/0001-carrier-phase-0.md](../../../docs/decisions/0001-carrier-phase-0.md).

These are measurements on a small corpus (13 recordings, about 3.5 minutes of speech, 10
languages; see [CORPUS.md](../../CORPUS.md)) with ffmpeg's encoders. They are not a
guarantee for other audio, encoders or apps, and they say nothing about the copy attack
(threat model A11), which no amount of robustness addresses.

## What was measured

For each recording, 3 different seals were embedded, the marked audio went through every
condition, and the detector ran on the result. A window counts as recovered only if the
**exact** 13-byte seal comes back (81 windows per condition with 6.4 s windows, 45 with
9.6 s). The same conditions were applied to the unmarked recordings and to a room-tone
recording without speech, to count false alarms (266 runs per operating point; every run
searches every time offset, roughly 300,000 candidate positions per operating point).

## Results

| | Default (step 7 dB, 6.4 s) | Quality (step 5 dB, 6.4 s) | Robust (step 9 dB, 6.4 s) | Long window (step 7 dB, 9.6 s) |
|---|---:|---:|---:|---:|
| Window (one seal) | 6.4 s | 6.4 s | 6.4 s | 9.6 s |
| PESQ-WB, mean (min) | 4.40 (4.06) | 4.51 (4.23) | 4.18 (3.75) | 4.41 (3.93) |
| STOI, mean | 0.971 | 0.983 | 0.956 | 0.973 |
| SNR / segmental SNR | 12.9 / 20.0 dB | 15.9 / 22.3 dB | 10.6 / 18.2 dB | 13.2 / 21.6 dB |
| `original` | 100.0% | 100.0% | 100.0% | 100.0% |
| `resample-44k` | 100.0% | 100.0% | 100.0% | 100.0% |
| `trim-1.234s` | 100.0% | 100.0% | 100.0% | 100.0% |
| `noise-20db` | 0.0% | 0.0% | 0.0% | 0.0% |
| `noise-30db` | 23.5% | 4.9% | 38.3% | 51.1% |
| `noise-10db` | 0.0% | 0.0% | 0.0% | 0.0% |
| `mp3-128k` | 100.0% | 100.0% | 100.0% | 100.0% |
| `mp3-64k` | 100.0% | 100.0% | 100.0% | 100.0% |
| `aac-64k` | 100.0% | 97.5% | 100.0% | 100.0% |
| `opus-32k` | 98.8% | 92.6% | 98.8% | 100.0% |
| `opus-24k` | 92.6% | 69.1% | 93.8% | 100.0% |
| `opus-12k` | 2.5% | 0.0% | 14.8% | 0.0% |
| `amr-wb-12.65k` | 0.0% | 0.0% | 7.4% | 11.1% |
| `amr-wb-23.85k` | 58.0% | 17.3% | 69.1% | 88.9% |
| `g722` | 98.8% | 82.7% | 98.8% | 100.0% |
| `mp3-128k+opus-24k` | 65.4% | 22.2% | 85.2% | 97.8% |
| `denoise` | 0.0% | 0.0% | 0.0% | 6.7% |
| `echo` | 0.0% | 0.0% | 0.0% | 0.0% |
| `tempo+1%` | 0.0% | 0.0% | 0.0% | 0.0% |
| Wrong seals returned, all conditions | 0 | 0 | 0 | 0 |
| False alarms on unmarked audio | 0 in 266 runs | 0 in 266 runs | 0 in 266 runs | 0 in 266 runs |
| Highest sync score on unmarked audio (threshold 6.0) | 4.12 | 4.09 | 3.74 | 4.01 |

Conditions: `resample-44k` resample to 44.1 kHz and back; `trim-1.234s` first 1.234 s removed
(blind synchronisation; only windows still complete are counted); `noise-NNdb` white noise
at that SNR; `mp3-*` LAME at 44.1 kHz; `aac-64k` ffmpeg's AAC-LC encoder at 44.1 kHz;
`opus-*` libopus in VoIP mode; `amr-wb-*` VisualOn AMR-WB encoder; `g722` ffmpeg's G.722
encoder; `mp3-128k+opus-24k` the two in a row; `denoise` ffmpeg afftdn; `echo` two
reflections at 40 and 60 ms; `tempo+1%` ffmpeg atempo (1 % faster, same pitch).

**Gate check, default operating point:** MP3 128 kbit/s 100 % (≥ 95 % ✓), AAC 64 kbit/s
100 % (≥ 95 % ✓), Opus 24 kbit/s 92.6 % (≥ 95 % ✗), AMR-WB 12.65 kbit/s 0 % (≥ 80 % ✗).
With 9.6 s windows Opus 24 kbit/s reaches 100 %, but AMR-WB 12.65 kbit/s stays at 11 %.
**Not passed** at any operating point.

## Reading the quality numbers

PESQ-WB (ITU-T P.862.2) runs from about 1.0 to 4.64 and STOI from 0 to 1; both compare the
marked audio with the original. They are objective proxies, not a listening test. A formal
listening test (for example MUSHRA) has not been done.

## Why these failures

- **AMR-WB, Opus at low rates:** these codecs rebuild speech from a model (spectral
  envelope plus excitation) and discard the fine spectral detail the chips live in.
  Watermarks designed for those codecs usually work in the domain they preserve (for example
  the spectral envelope's line spectral frequencies); that would be a different carrier.
- **Noise, noise reduction, echo:** the chips are level changes of a few dB in quiet as
  well as loud tiles; added noise, spectral subtraction and comb filtering move those levels
  by more than that.
- **Tempo:** the detector searches time offsets, not time scales.

## Files

Each folder (`default`, `quality`, `robust`, `long-window`) holds the full table
(`results.md`), machine-readable results (`results.json`) and per-file quality
(`quality.json`).

## Reproduce

```sh
pip install imageio-ffmpeg==0.6.0 pesq pystoi numpy soundfile   # the ffmpeg 7.0.2 build used here
FF=$(python3 -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())")
bench/fetch-corpus.sh "$FF"
cargo build --release -p vox-trust-bench
# stdm-1 settings; since 2026-10-02 the defaults are stdm-2 (300 columns, tempo search)
S1="--columns 200 --max-tempo 0"
target/release/vt-bench --ffmpeg "$FF" --out bench/results/runs/default --seals 3 $S1
target/release/vt-bench --ffmpeg "$FF" --out bench/results/runs/quality --seals 3 $S1 --step 5 --max-db 3.5
target/release/vt-bench --ffmpeg "$FF" --out bench/results/runs/robust --seals 3 $S1 --step 9 --max-db 5.5
target/release/vt-bench --ffmpeg "$FF" --out bench/results/runs/long-window --seals 3 --columns 300 --max-tempo 0
for r in default quality robust long-window; do python3 bench/quality.py bench/results/runs/$r; done
```

Measured with ffmpeg 7.0.2 (the static build shipped by imageio-ffmpeg 0.6.0) and Rust
1.99.0. Embedding ran at about 400x real time per core, detection (which searches every
time offset) at several hundred times real time per core.
