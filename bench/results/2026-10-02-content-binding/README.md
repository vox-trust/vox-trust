# Study: binding an in-band seal to its audio, 2026-10-02

**Bottom line.** Committing a robust fingerprint of the audio in the seal's tag **does not
solve the copy attack** (threat model A11). It stops a naive copy, but an attacker who
reshapes the target audio's coarse band energies matches the fingerprint, including when
the fingerprint is a projection secret to the circle, at a distortion a fake recording can
afford. What survives a codec is exactly what an attacker can imitate. The problem stays
open; this result is published so the idea is not retried without something new.

Small study: 15 windows of 9.6 s from the 13 recordings of the stdm-1 corpus, the same
conditions (codecs via ffmpeg 7.0.2). Numbers are indicative, not precise.

## The idea

The tag would cover, besides the seal fields, an N-bit fingerprint of the window. The
verifier recomputes the fingerprint from the audio it received; a codec flips a few bits,
so it also tries flipping the least reliable ones (`k` flips among the `2k` least reliable
bits: 1, 3, 11 or 42 tags tried for `k` = 0 to 3, which multiplies the false-accept rate
of the 32-bit tag by the same factor).

Fingerprint, after Haitsma and Kalker (2002): log band energies between 300 and 3800 Hz,
averaged over T segments of the window; bit = sign of the band-energy difference between
neighbouring bands, minus the same in the next segment; its size is its reliability.

## Results

**A genuine seal still binds after codecs** (16-bit fingerprint, share of windows accepted
with up to `k` flips):

| Condition | bit agreement | k=0 | k=1 | k=2 | k=3 |
|---|---:|---:|---:|---:|---:|
| `mp3-64k` | 98.8 % | 80 % | 93 % | 100 % | 100 % |
| `aac-64k` | 97.9 % | 67 % | 93 % | 100 % | 100 % |
| `opus-24k` | 97.1 % | 67 % | 80 % | 87 % | 100 % |
| `opus-12k` | 94.2 % | 40 % | 67 % | 87 % | 93 % |
| `amr-wb-12.65k` | 94.2 % | 33 % | 73 % | 87 % | 93 % |
| `g722` | 95.0 % | 47 % | 73 % | 100 % | 100 % |
| `noise-30db` | 87.5 % | 27 % | 40 % | 53 % | 80 % |
| `denoise` | 90.4 % | 20 % | 47 % | 60 % | 73 % |
| `echo` | 93.8 % | 47 % | 60 % | 80 % | 93 % |
| `tempo+1%` | 98.8 % | 80 % | 93 % | 100 % | 100 % |

The fingerprint itself survives AMR-WB far better than the stdm carriers do. 32- and
64-bit fingerprints bind less often (more bits to get right): see `results.json`.

**A naive copy does not bind:** a seal moved to any of the other windows was accepted in
0 of 210 pairs at every `k` (expected about 0.06 % for 16 bits and `k = 3`).

**An adaptive copy does:** an attacker who knows the fingerprint reshapes the band energies
of the target audio, segment by segment, until its fingerprint equals the original's.
Success 20 of 20 attempts for 16, 32 and 64 bits, at a median SNR of 12, 9 and 9 dB (the
change is audible, but the result is a fake anyway, and a better attacker would do with
less).

**A secret fingerprint does not stop it either** (`keyed-results.json`). In circle mode
the fingerprint can be 16 key-dependent random projections of the band-energy grid. The
attacker cannot aim at the bits, but can give the fake the original's whole energy grid,
which matches every projection at once:

| Grid (segments x bands) | Genuine, after Opus 24k | Genuine, after AMR-WB 12.65k | Genuine, after noise 30 dB | Attack without the key | Attack SNR |
|---|---:|---:|---:|---:|---:|
| 4 x 9 | 97.9 % | 96.7 % | 86.3 % | 87.5 % | 6.7 dB |
| 12 x 12 | 97.1 % | 92.9 % | 85.8 % | 80.0 % | 6.1 dB |
| 24 x 16 | 97.9 % | 96.7 % | 83.3 % | 85.6 % | 3.6 dB |
| 48 x 24 | 93.3 % | 92.1 % | 83.3 % | 63.7 % | 1.7 dB |

At coarse grids the forged fake agrees as well as a genuine copy after light noise does, so
no threshold separates them; at the finest grid the attack degrades only by wrecking the
audio, while genuine copies also lose agreement.

## What would be needed

Something the attacker cannot reproduce without the original audio itself: a fingerprint
fine enough that matching it forces the fake to sound like the original (which so far costs
robustness), or binding through a channel the attacker does not control (for example a
signed, bit-exact reference kept by the sender, which is what file mode already is). Until
then in-band seals give no verdict.

## Reproduce

```sh
pip install numpy soundfile
FF=$(python3 -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())")
python3 bench/content_binding.py --ffmpeg "$FF" --out bench/results/runs/binding
python3 bench/content_binding_keyed.py --ffmpeg "$FF" --out bench/results/runs/binding
```
