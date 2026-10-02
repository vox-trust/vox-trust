# Yardstick: published neural audio watermarks, 2026-10-02

**Bottom line.** Two published neural watermarks, used as released, on the same corpus,
conditions and quality metrics as our carriers. **WavMark survives several conditions that
defeat stdm-2**: noise reduction, echo, a 1 % tempo change, and in part AMR-WB 12.65
kbit/s, the phone-call codec (31 % of recordings exact, 69 % detected). **AudioSeal**
survives noise and echo but fails G.722, trimming and AMR-WB. Neither passes the Phase 0
gate, and neither solves the copy attack. The comparison flatters them (see "Not the same
task"), so the next step is to measure WavMark carrying a real 102-bit seal.

## Results

Share of recordings whose whole 16-bit message came back exactly (AudioSeal, WavMark) and
share of 9.6 s windows whose 102-bit seal came back exactly (stdm-2). 13 recordings; each
recording is about 8 points, so differences under 15 points are within noise.

| Condition | stdm-2 (ours) | AudioSeal | WavMark |
|---|---:|---:|---:|
| `original` | 100 % | 100 % | 100 % |
| `resample-44k` | 100 % | 100 % | not run |
| `trim-1.234s` | 100 % | 8 % | not run |
| `noise-30db` | 51 % | 100 % | not run |
| `noise-20db` | 0 % | 54 % | 0 % |
| `noise-10db` | 0 % | 8 % | not run |
| `mp3-128k`, `mp3-64k`, `aac-64k` | 100 % | 100 % | not run |
| `opus-32k` | 100 % | 100 % | not run |
| `opus-24k` | 100 % | 100 % | 100 % |
| `opus-12k` | 0 % | 100 % | not run |
| `amr-wb-12.65k` | 11 % | 0 % | **31 %** (69 % detected) |
| `amr-wb-23.85k` | 89 % | 0 % | not run |
| `g722` | 100 % | 0 % | not run |
| `denoise` | 7 % | 0 % | **100 %** |
| `echo` | 0 % | 100 % | **100 %** |
| `tempo+1%` | 84 % | 62 % | **100 %** |
| False alarms on unmarked audio | 0 in 266 runs | 0 in 252 runs | 0 in 14 runs (`original` only) |
| PESQ-WB, mean (min) | 4.41 (3.93) | 4.43 (4.28) | 4.12 (3.90) |
| STOI, mean | 0.973 | 0.998 | 0.996 |

WavMark ran on 7 conditions only, and its false alarms on unmodified audio only: its
detector searches every offset with the network and took about 5 minutes per detection on
this machine's CPU, so the full set would have taken about 7 hours. AudioSeal ran on all
conditions. `bits` (the share of message bits recovered, 0.5 being chance) and
`detected` are in `audioseal.json` and `wavmark.json`.

## Not the same task

- **Payload.** Both models carried one 16-bit message. AudioSeal decodes it from the whole
  recording; WavMark repeats it in every 1 s segment and its decoder combines the segments.
  A seal is 102 bits, so it would need several different messages (for WavMark, 7 segments
  of 16 bits at its nominal 16 bit/s), each read without that repetition. Their numbers
  here are an upper bound on what a seal would get.
- **Unit.** stdm-2 must recover a full seal from each 9.6 s window; the neural numbers are
  per recording (9.6 to 28 s, most 13 to 20 s).
- **Cost.** Both need a neural network to embed and to detect (WavMark's detector took
  minutes per recording on CPU here), against a few milliseconds for stdm-2.
- **Copy attack.** Their payload is no more bound to the audio than ours.

## Reproduce

```sh
pip install torch torchaudio audioseal wavmark soundfile numpy pesq pystoi
FF=$(python3 -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())")
for k in 0 1 2 3; do python3 bench/neural_baselines.py --ffmpeg "$FF" --models audioseal \
    --out runs/audioseal --shard $k/4 & done; wait
python3 bench/neural_baselines.py --ffmpeg "$FF" --out runs/audioseal --merge
for k in 0 1 2 3; do python3 bench/neural_baselines.py --ffmpeg "$FF" --models wavmark \
    --out runs/wavmark --shard $k/4 \
    --conditions original,opus-24k,amr-wb-12.65k,noise-20db,denoise,echo,tempo+1% \
    --fa-conditions original & done; wait
python3 bench/neural_baselines.py --ffmpeg "$FF" --out runs/wavmark --merge
```

Weights: AudioSeal `audioseal_wm_16bits` / `audioseal_detector_16bits` and WavMark's
released model, both downloaded from Hugging Face by their packages (audioseal 0.2.0,
wavmark 0.0.3, PyTorch 2.14.1 CPU).
