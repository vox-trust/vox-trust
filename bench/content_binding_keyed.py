#!/usr/bin/env python3
"""Follow-up to content_binding.py: does a fingerprint secret to the circle stop the forgery?

In circle mode signer and verifier share a key, so the fingerprint can be a key-dependent
random projection of the window's band-energy grid (segments x bands, column-normalised):
bit i = sign(<P_i, G>), with P drawn from the key. An attacker without the key cannot aim
at the bits, but can give the fake audio the original's whole energy grid, which matches
every projection at once. Measured per grid size:
  - bit agreement after a codec (genuine copy), to compare with
  - bit agreement after that equalisation attack (no key), and its distortion (SNR)

    python3 bench/content_binding_keyed.py --ffmpeg "$FF" --out bench/results/runs/binding
"""

import argparse
import json
import sys
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf

sys.path.insert(0, str(Path(__file__).parent))
from content_binding import FRAME, WINDOW_S, band_edges, stft_power  # noqa: E402
from neural_baselines import CONDITIONS, SR, apply  # noqa: E402

GRIDS = [(4, 9), (12, 12), (24, 16), (48, 24)]
CODECS = ("mp3-64k", "opus-24k", "amr-wb-12.65k", "noise-30db")
KEY = 12345
BITS = 16


def grid(x, segments, bands):
    p = stft_power(x)
    e = band_edges(bands - 1)
    energy = np.stack([p[:, e[i]:e[i + 1]].sum(1) for i in range(bands)], 1)
    energy = 10 * np.log10(energy + 1e-10)
    g = np.stack([energy[s].mean(0) for s in np.array_split(np.arange(len(energy)), segments)])
    return g - g.mean(1, keepdims=True)


def keyed_bits(g, key):
    projections = np.random.default_rng(key).standard_normal((BITS, g.size))
    return projections @ g.ravel() > 0


def equalise(target, source, segments, bands):
    """Gives `target` the per-segment band energies of `source`."""
    gt, gs = grid(target, segments, bands), grid(source, segments, bands)
    n = len(target)
    e = band_edges(bands - 1)
    freqs = np.fft.rfftfreq(n, 1 / SR)
    out = np.zeros(n)
    for t, idx in enumerate(np.array_split(np.arange(n), segments)):
        part = np.zeros(n)
        part[idx] = target[idx]
        gain = np.ones_like(freqs)
        for b in range(bands):
            lo, hi = e[b] * SR / FRAME, e[b + 1] * SR / FRAME
            gain[(freqs >= lo) & (freqs < hi)] = 10 ** ((gs[t, b] - gt[t, b]) / 20)
        out += np.fft.irfft(np.fft.rfft(part) * gain, n)
    return out.astype(np.float32)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ffmpeg", required=True)
    ap.add_argument("--corpus", default=str(Path(__file__).parent / "corpus"))
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    n_win = int(WINDOW_S * SR)
    windows = []
    for f in sorted(Path(args.corpus).glob("*.wav")):
        if "room-tone" in f.name:
            continue
        x, _ = sf.read(f, dtype="float32")
        windows += [x[i * n_win:(i + 1) * n_win] for i in range(len(x) // n_win)]
    rng = np.random.default_rng(3)
    conds = dict(CONDITIONS)
    report = {"windows": len(windows), "bits": BITS, "grids": {}}
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        for segments, bands in GRIDS:
            ref = [keyed_bits(grid(w, segments, bands), KEY) for w in windows]
            genuine = {}
            for c in CODECS:
                agree = [
                    np.mean(keyed_bits(grid(apply(args.ffmpeg, conds[c], w, tmp, rng)[:n_win], segments, bands), KEY) == r)
                    for w, r in zip(windows, ref)
                ]
                genuine[c] = float(np.mean(agree))
            attack_agree, attack_snr = [], []
            for _ in range(10):
                a, b = rng.choice(len(windows), 2, replace=False)
                fake = equalise(windows[b], windows[a], segments, bands)
                attack_agree.append(np.mean(keyed_bits(grid(fake, segments, bands), KEY) == ref[a]))
                attack_snr.append(10 * np.log10(np.sum(windows[b] ** 2) / np.sum((fake - windows[b]) ** 2)))
            report["grids"][f"{segments}x{bands}"] = {
                "genuine_bit_agreement": genuine,
                "attack_bit_agreement": float(np.mean(attack_agree)),
                "attack_snr_db_median": float(np.median(attack_snr)),
            }
            print(f"grid {segments}x{bands} done", file=sys.stderr, flush=True)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "keyed-results.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
