#!/usr/bin/env python3
"""Study: can an in-band seal be bound to the audio it sits in (threat model A11)?

Idea. The seal's tag would cover a short robust fingerprint of its window, so a seal copied
into other audio no longer verifies. The verifier recomputes the fingerprint from what it
received; codecs flip a few bits, so it also tries flipping the least reliable ones (each
try is one more chance for a forged tag to pass, which is counted in the false-accept cost).

Fingerprint (after Haitsma and Kalker, "A highly robust audio fingerprinting system", 2002):
log band energies on the carrier's STFT (16 kHz, 512/256), averaged over T time segments of
the window and B+1 bands between 300 and 3800 Hz; bit (t, b) is the sign of the energy
difference between bands b and b+1, minus the same difference in segment t+1. Its absolute
value is the bit's reliability. N = (T-1) * B bits.

Measured on the stdm-1 corpus and conditions (bench/neural_baselines.py has the same list):
  - bit agreement between a window and the same window after each condition
  - acceptance with up to `flips` reliability-ordered flips (a genuine seal still binds)
  - naive copy: acceptance of a fingerprint from a different window (a moved seal binds)
  - adaptive copy: an attacker who knows the fingerprint reshapes the target audio's band
    energies until it matches, and the resulting distortion (SNR) is reported

    pip install numpy soundfile
    python3 bench/content_binding.py --ffmpeg "$FF" --out bench/results/runs/binding
"""

import argparse
import itertools
import json
import sys
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf

sys.path.insert(0, str(Path(__file__).parent))
from neural_baselines import CONDITIONS, SR, apply  # noqa: E402

FRAME, HOP = 512, 256
WINDOW_S = 9.6


def band_edges(n_bands):
    # Log-spaced between 300 and 3800 Hz, as bin indices.
    hz = np.geomspace(300, 3800, n_bands + 2)
    return np.round(hz / (SR / FRAME)).astype(int)


def stft_power(x):
    win = np.sin(np.pi * np.arange(FRAME) / FRAME)
    n = 1 + (len(x) - FRAME) // HOP
    idx = np.arange(FRAME)[None, :] + HOP * np.arange(n)[:, None]
    return np.abs(np.fft.rfft(x[idx] * win, axis=1)) ** 2


def fingerprint(x, segments, bands):
    """Returns (bits, reliability) for one window of audio."""
    p = stft_power(x)
    edges = band_edges(bands)
    e = np.stack([p[:, edges[i]:edges[i + 1]].sum(axis=1) for i in range(bands + 1)], axis=1)
    e = 10 * np.log10(e + 1e-10)
    seg = np.array_split(np.arange(len(e)), segments)
    es = np.stack([e[s].mean(axis=0) for s in seg])  # segments x (bands+1)
    d = es[:, :-1] - es[:, 1:]  # band differences
    v = (d[:-1] - d[1:]).ravel()  # time differences of band differences
    return v > 0, np.abs(v)


def accepts(ref_bits, bits, rel, flips):
    """Whether `bits`, with up to `flips` of its least reliable bits flipped, equals `ref_bits`.
    Returns (accepted, tries): tries is how many tags a verifier checks."""
    order = np.argsort(rel)
    cand = order[: max(flips * 2, flips)]  # flip only among the least reliable 2*flips bits
    tries = 0
    for k in range(flips + 1):
        for combo in itertools.combinations(cand, k):
            tries += 1
            b = bits.copy()
            b[list(combo)] ^= True
            if np.array_equal(b, ref_bits):
                return True, None
    return False, tries


def tries_for(flips):
    pool = max(flips * 2, flips)
    return sum(len(list(itertools.combinations(range(pool), k))) for k in range(flips + 1))


def adaptive_forge(target, ref_bits, segments, bands, max_iter=40):
    """Reshapes `target`'s band energies per segment until its fingerprint equals ref_bits;
    returns (success, SNR of the change in dB)."""
    y = target.copy()
    edges = band_edges(bands)
    n = len(y)
    seg_bounds = np.array_split(np.arange(n), segments)
    for _ in range(max_iter):
        bits, rel = fingerprint(y, segments, bands)
        wrong = np.flatnonzero(bits != ref_bits)
        if len(wrong) == 0:
            break
        spec = np.fft.rfft(y)
        freqs = np.fft.rfftfreq(n, 1 / SR)
        gain = np.ones((segments, bands + 1))
        for w in wrong:
            t, b = divmod(int(w), bands)
            # Push d[t,b] - d[t+1,b] across zero: raise band b in segment t, lower b+1.
            s = 1.12 if ref_bits[w] else 1 / 1.12
            gain[t, b] *= s
            gain[t, b + 1] /= s
            gain[t + 1, b] /= s
            gain[t + 1, b + 1] *= s
        out = np.zeros_like(y)
        for t, idx in enumerate(seg_bounds):
            part = np.zeros(n)
            part[idx] = y[idx]
            sp = np.fft.rfft(part)
            g = np.ones_like(freqs)
            for b in range(bands + 1):
                lo, hi = edges[b] * SR / FRAME, edges[b + 1] * SR / FRAME
                g[(freqs >= lo) & (freqs < hi)] = gain[t, b]
            out += np.fft.irfft(sp * g, n)
        y = out.astype(np.float32)
    bits, _ = fingerprint(y, segments, bands)
    noise = y - target
    snr = 10 * np.log10(np.sum(target**2) / max(np.sum(noise**2), 1e-20))
    return bool(np.array_equal(bits, ref_bits)), float(snr)


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
        for i in range(len(x) // n_win):
            windows.append((f.name, i, x[i * n_win:(i + 1) * n_win]))
    print(f"{len(windows)} windows of {WINDOW_S} s", file=sys.stderr)

    configs = [(3, 8), (5, 8), (9, 8)]  # (segments, bands) -> 16, 32, 64 bits
    flip_levels = [0, 1, 2, 3]
    rng = np.random.default_rng(7)
    report = {"windows": len(windows), "window_s": WINDOW_S, "configs": {}}
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        degraded = {}
        for c, cond in CONDITIONS:
            if c == "trim-1.234s":
                continue  # alignment comes from the carrier's synchronisation
            degraded[c] = [apply(args.ffmpeg, cond, w, tmp, rng)[:n_win] for _, _, w in windows]
            print(f"condition {c} done", file=sys.stderr, flush=True)
        for segments, bands in configs:
            nbits = (segments - 1) * bands
            ref = [fingerprint(w, segments, bands) for _, _, w in windows]
            conf = {"bits": nbits, "conditions": {}}
            for c, ys in degraded.items():
                agree, acc = [], {k: 0 for k in flip_levels}
                for (rb, _), y in zip(ref, ys):
                    if len(y) < n_win // 2:
                        continue
                    b, r = fingerprint(y, segments, bands)
                    agree.append(np.mean(b == rb))
                    for k in flip_levels:
                        acc[k] += accepts(rb, b, r, k)[0]
                conf["conditions"][c] = {
                    "bit_agreement": float(np.mean(agree)),
                    "accepted": {str(k): acc[k] / len(agree) for k in flip_levels},
                }
            # Naive copy: a fingerprint from window j presented as window i's.
            pairs = [(i, j) for i in range(len(ref)) for j in range(len(ref)) if i != j]
            naive = {k: 0 for k in flip_levels}
            for i, j in pairs:
                for k in flip_levels:
                    naive[k] += accepts(ref[i][0], ref[j][0], ref[j][1], k)[0]
            conf["naive_copy_accept"] = {str(k): naive[k] / len(pairs) for k in flip_levels}
            conf["naive_copy_theory"] = {
                str(k): tries_for(k) / 2**nbits for k in flip_levels
            }
            conf["tag_tries"] = {str(k): tries_for(k) for k in flip_levels}
            # Adaptive copy on a sample of pairs.
            sample = rng.choice(len(pairs), size=min(20, len(pairs)), replace=False)
            forged = [adaptive_forge(windows[pairs[s][1]][2], ref[pairs[s][0]][0], segments, bands) for s in sample]
            conf["adaptive_copy"] = {
                "attempts": len(forged),
                "success": sum(ok for ok, _ in forged) / len(forged),
                "snr_db_median": float(np.median([snr for _, snr in forged])),
            }
            report["configs"][f"{nbits}-bit"] = conf
            print(f"{nbits}-bit done", file=sys.stderr, flush=True)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "results.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
