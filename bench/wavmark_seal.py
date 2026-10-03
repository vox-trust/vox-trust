#!/usr/bin/env python3
"""Measures WavMark carrying a real 102-bit seal, on the same corpus and conditions as stdm-2.

bench/neural_baselines.py gave WavMark its easiest task: one 16-bit message repeated in
every 1 s segment, combined over the whole recording. A seal is 102 bits, so here every
segment carries different bits and nothing is repeated within a seal:

  - WavMark's model hides 32 bits in 1 s of audio: a 16-bit sync pattern (its default) and
    16 payload bits. Segments are 1.1 s apart, as in the published encoder.
  - Payload = 3-bit segment index + 13 seal bits. Eight segments (8.8 s) carry 104 bits,
    of which the seal uses 102. A recording carries as many whole seals as fit, all with
    the same seal (the receiver may combine nothing: each seal is read on its own).
  - The detector is WavMark's own search (every 50 ms offset, keep positions whose 16 sync
    bits match exactly), then nearby hits are merged by majority. A seal is read when the
    eight indexes appear in order, each 0.8 to 1.3 segments after the previous one.

Reported per condition: `exact` (share of embedded seals read back with all 102 bits
right), `wrong` (complete seals read with wrong bits: a real seal's tag would reject them,
but they show how often the carrier lies), `segments` (share of embedded segments found
with correct bits), and on unmarked audio `false_seals` (complete seals found at all).

    pip install torch wavmark soundfile numpy pesq pystoi
    python3 bench/wavmark_seal.py --ffmpeg "$FF" --out bench/results/runs/wavmark-seal --shard 0/4

On CPU each detection takes minutes; --shard K/N splits the files between processes, each
detection is saved under OUT/units/ as soon as it is done (a rerun resumes), and --merge
sums them into OUT/results.json.
"""

import argparse
import hashlib
import json
import sys
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf

sys.path.insert(0, str(Path(__file__).parent))
from neural_baselines import CONDITIONS, SR, apply, quality  # noqa: E402

SEG = 16000  # samples carrying one 32-bit message
STRIDE = 17600  # segment spacing (1 s cover + 0.1 s shift area, as the published encoder)
IDX_BITS, DATA_BITS, N_SEG = 3, 13, 8
SEAL_BITS = 102
CYCLE = N_SEG * STRIDE
DETECT_STEP = 800  # WavMark's decoder step (shift_range 0.1 x 0.5)
MERGE = 8000  # hits closer than this belong to the same segment


def bits(value, n):
    return [(value >> (n - 1 - i)) & 1 for i in range(n)]


def segment_payloads(seal):
    padded = list(seal) + [0] * (N_SEG * DATA_BITS - len(seal))
    return [bits(k, IDX_BITS) + padded[k * DATA_BITS:(k + 1) * DATA_BITS] for k in range(N_SEG)]


class WavMarkSeal:
    def __init__(self):
        import torch
        import wavmark
        from wavmark.utils import wm_add_util

        self.torch = torch
        self.add = wm_add_util
        self.pattern = np.array(wm_add_util.fix_pattern[:16])
        self.model = wavmark.load_model()

    def embed(self, x, seal):
        y = x.copy()
        payloads = segment_payloads(seal)
        n_cycles = len(x) // CYCLE
        skipped = 0
        for c in range(n_cycles):
            for k, payload in enumerate(payloads):
                start = c * CYCLE + k * STRIDE
                chunk = x[start:start + SEG]
                wm = np.concatenate([self.pattern, payload])
                marked, state = self.add.encode_trunck_with_snr_check(k, chunk, wm, "cpu", self.model, 20, 38)
                skipped += state == "skip"
                y[start:start + SEG] = marked
        return y.astype(np.float32), n_cycles, skipped

    def hits(self, y):
        """(position, segment index, 13 data bits) for each segment the sync pattern finds."""
        points = list(range(0, len(y) - SEG, DETECT_STEP))
        raw = []
        for i in range(0, len(points), 16):
            batch = np.array([y[p:p + SEG] for p in points[i:i + 16]], dtype=np.float32)
            with self.torch.no_grad():
                out = (self.model.decode(self.torch.from_numpy(batch)) >= 0.5).int().numpy()
            for p, msg in zip(points[i:i + 16], out):
                if np.array_equal(msg[:16], self.pattern):
                    raw.append((p, msg[16:]))
        groups = []
        for p, payload in raw:
            if groups and p - groups[-1][0][0] < MERGE:
                groups[-1].append((p, payload))
            else:
                groups.append([(p, payload)])
        out = []
        for g in groups:
            vote = (np.mean([m for _, m in g], axis=0) >= 0.5).astype(int)
            idx = int("".join(map(str, vote[:IDX_BITS])), 2)
            out.append((g[len(g) // 2][0], idx, [int(b) for b in vote[IDX_BITS:]]))
        return out

    @staticmethod
    def seals(hits):
        """Complete seals: indexes 0..7 in order, each 0.8 to 1.3 strides after the last."""
        found = []
        for i, (p0, idx, data) in enumerate(hits):
            if idx != 0:
                continue
            chain, prev = [data], p0
            for k in range(1, N_SEG):
                nxt = next((h for h in hits[i + 1:] if h[1] == k and 0.8 * STRIDE <= h[0] - prev <= 1.3 * STRIDE), None)
                if nxt is None:
                    break
                chain.append(nxt[2])
                prev = nxt[0]
            if len(chain) == N_SEG:
                found.append([b for d in chain for b in d][:SEAL_BITS])
        return found


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ffmpeg", required=True)
    ap.add_argument("--corpus", default=str(Path(__file__).parent / "corpus"))
    ap.add_argument("--out", required=True)
    ap.add_argument("--shard", default="0/1")
    ap.add_argument("--merge", action="store_true")
    ap.add_argument("--conditions", help="comma-separated subset (default: all)")
    ap.add_argument("--fa-conditions", default="original", help="conditions for the unmarked runs")
    args = ap.parse_args()
    out = Path(args.out)
    if args.merge:
        merge(out)
        return
    k, n_shards = (int(v) for v in args.shard.split("/"))
    names = [c for c, _ in CONDITIONS]
    chosen = args.conditions.split(",") if args.conditions else names
    fa_chosen = args.fa_conditions.split(",") if args.fa_conditions else []
    for c in chosen + fa_chosen:
        if c not in names:
            ap.error(f"unknown condition {c}")
    files = sorted(Path(args.corpus).glob("*.wav"))[k::n_shards]
    speech = [f for f in files if "room-tone" not in f.name]
    units = out / "units"
    units.mkdir(parents=True, exist_ok=True)

    import torch

    torch.set_num_threads(1)
    model = WavMarkSeal()
    seed_of = lambda *parts: int(hashlib.sha256("/".join(parts).encode()).hexdigest()[:8], 16)  # noqa: E731
    # Every detection is saved as soon as it is done, so an interrupted run resumes where it
    # stopped (each unit's randomness depends only on its file and condition).
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        for f in speech:
            todo = [(c, cond) for c, cond in CONDITIONS if c in chosen and not (units / f"{f.stem}__{c}.json").exists()]
            if not todo and (units / f"{f.stem}__embed.json").exists():
                continue
            x, sr = sf.read(f, dtype="float32")
            assert sr == SR, f
            seal = [int(b) for b in np.random.default_rng(seed_of("seal", f.name)).integers(0, 2, SEAL_BITS)]
            marked, n_cycles, skipped = model.embed(x, seal)
            marked = np.clip(marked, -1, 1)
            if not (units / f"{f.stem}__embed.json").exists():
                p, s_ = quality(x, marked)
                (units / f"{f.stem}__embed.json").write_text(json.dumps(
                    {"pesq": p, "stoi": s_, "skipped_segments": skipped, "cycles": n_cycles}))
            payloads = segment_payloads(seal)
            for c, cond in todo:
                rng = np.random.default_rng(seed_of("cond", f.name, c))
                y = apply(args.ffmpeg, cond, marked, tmp, rng)
                # Seals still whole after the condition (trimming drops the start).
                lost = cond[1] if cond and cond[0] == "trim" else 0
                whole = [cy for cy in range(n_cycles) if cy * CYCLE >= lost]
                hits = model.hits(y)
                read = model.seals(hits)
                good = sum(1 for h in hits if h[2] == payloads[h[1]][IDX_BITS:])
                unit = {
                    "seals": len(whole),
                    "exact": min(sum(sl == seal for sl in read), len(whole)),
                    "wrong": sum(sl != seal for sl in read),
                    "segments": len(whole) * N_SEG,
                    "segments_ok": min(good, len(whole) * N_SEG),
                }
                (units / f"{f.stem}__{c}.json").write_text(json.dumps(unit))
                print(f"{f.name} {c}: {unit}", file=sys.stderr, flush=True)
        for f in files:
            for c, cond in CONDITIONS:
                path = units / f"unmarked-{f.stem}__{c}.json"
                if c not in fa_chosen or path.exists():
                    continue
                x, _ = sf.read(f, dtype="float32")
                hits = model.hits(apply(args.ffmpeg, cond, x, tmp, np.random.default_rng(seed_of("fa", f.name, c))))
                path.write_text(json.dumps({"runs": 1, "sync_hits": len(hits), "false_seals": len(model.seals(hits))}))
                print(f"{f.name} unmarked {c}: {len(hits)} sync hits", file=sys.stderr, flush=True)


def merge(out):
    units = out / "units"
    embeds = [json.loads(p.read_text()) for p in sorted(units.glob("*__embed.json"))]
    conditions, unmarked = {}, {}
    for p in sorted(units.glob("*__*.json")):
        stem, cond = p.stem.rsplit("__", 1)
        if cond == "embed":
            continue
        group = unmarked if stem.startswith("unmarked-") else conditions
        acc = group.setdefault(cond, {})
        for key, n in json.loads(p.read_text()).items():
            acc[key] = acc.get(key, 0) + n
    summary = {
        "recordings": len(embeds),
        "pesq_wb_mean": float(np.mean([e["pesq"] for e in embeds])),
        "pesq_wb_min": float(np.min([e["pesq"] for e in embeds])),
        "stoi_mean": float(np.mean([e["stoi"] for e in embeds])),
        "skipped_segments": sum(e["skipped_segments"] for e in embeds),
        "seals_embedded": sum(e["cycles"] for e in embeds),
        "conditions": {
            c: {"seals": v["seals"], "exact": v["exact"] / v["seals"] if v["seals"] else None, "wrong": v["wrong"],
                "segments": v["segments_ok"] / v["segments"] if v["segments"] else None}
            for c, v in conditions.items()
        },
        "unmarked": unmarked,
    }
    (out / "results.json").write_text(json.dumps(summary, indent=2))
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
