# Benchmark

Measures how the experimental in-band carrier (`crates/vox-trust-carrier`, "stdm-2", earlier "stdm-1")
survives real audio paths, and what it costs in audio quality. Results are published
whatever they are.

- **Latest results:** [results/2026-10-02-stdm-2](results/2026-10-02-stdm-2/README.md) (earlier: [stdm-1](results/2026-10-01-stdm-1/README.md))
- **Yardstick, published neural watermarks:** [results/2026-10-02-neural-baselines](results/2026-10-02-neural-baselines/README.md)
- **Content binding study (negative result):** [results/2026-10-02-content-binding](results/2026-10-02-content-binding/README.md)
- **Corpus and licences:** [CORPUS.md](CORPUS.md)
- **Decisions taken from them:** [0001](../docs/decisions/0001-carrier-phase-0.md), [0002](../docs/decisions/0002-carrier-follow-up.md)

## Pieces

| File | What it does |
|---|---|
| `fetch-corpus.sh` | Builds `bench/corpus/` (16 kHz mono WAV) from openly licensed recordings, each source pinned to a Git commit |
| `crates/vox-trust-bench` (`vt-bench`) | Embeds seals, sends the audio through each condition (real codecs via ffmpeg, plus noise, trimming, echo, tempo), detects, counts exact recoveries, wrong seals and false alarms |
| `quality.py` | PESQ-WB (ITU-T P.862.2) and STOI of the marked audio against the original |
| `summarize.py` | Comparison table across operating points |
| `neural_baselines.py` | AudioSeal and WavMark on the same corpus and conditions (needs PyTorch) |
| `content_binding.py`, `content_binding_keyed.py` | Study of binding an in-band seal to its audio with a robust fingerprint |

## Run

```sh
pip install imageio-ffmpeg==0.6.0 pesq pystoi numpy soundfile
FF=$(python3 -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())")
bench/fetch-corpus.sh "$FF"
cargo build --release -p vox-trust-bench
target/release/vt-bench --ffmpeg "$FF" --out bench/results/runs/default --seals 3
python3 bench/quality.py bench/results/runs/default
```

`vt-bench --help` lists the options; carrier parameters can be overridden to explore other
operating points (`--step`, `--max-db`, `--columns`, `--tile-bins`, ...). Any ffmpeg with
libmp3lame, libopus and libvo-amrwbenc works; other builds may give slightly different
numbers. A run over the full corpus takes about a minute on four cores.

The **Benchmark** workflow in GitHub Actions runs the same steps on demand and uploads
the results.

## Rules for publishing results

- Report every condition, including the ones that fail.
- A window is recovered only if the exact seal comes back; a wrong seal is reported
  separately and must stay at zero.
- False alarms are measured on unmarked audio under the same conditions.
- State the corpus, the encoder build and the carrier parameters with the numbers.
