#!/usr/bin/env python3
"""Measures published neural audio watermarks on the same corpus and conditions as stdm-1.

Baselines, used as published (weights downloaded from Hugging Face):
  - AudioSeal (San Roman et al., ICML 2024), 16-bit message, `audioseal` package.
  - WavMark (Chen et al., 2023), 16-bit payload per 1 s segment plus a 16-bit pattern,
    `wavmark` package.

For each recording a random message is embedded, the marked audio goes through each
condition (the same ffmpeg arguments as crates/vox-trust-bench), and the detector runs on
the result. Reported per condition:
  - exact: share of recordings whose whole message came back exactly
  - bits:  mean share of message bits recovered (0.5 is chance)
  - false alarms: detections on the unmarked recordings after the same condition
Quality (PESQ-WB, STOI) compares the marked audio with the original, as bench/quality.py.

These models carry 16 bits; the seal needs 102. A 102-bit seal would need several messages
in a row (AudioSeal: about 7 per recording, WavMark: 7 one-second segments), so "exact" here
is an upper bound on what a seal would get.

    pip install torch torchaudio audioseal wavmark soundfile numpy pesq pystoi
    python3 bench/neural_baselines.py --ffmpeg "$FF" --out bench/results/runs/neural

On CPU this is slow (about an hour per model); --shard K/N runs every N-th recording, so
N processes can share the work, and --merge combines their files:

    for k in 0 1 2 3; do python3 bench/neural_baselines.py --ffmpeg "$FF" \
        --out bench/results/runs/neural --shard $k/4 & done; wait
    python3 bench/neural_baselines.py --ffmpeg "$FF" --out bench/results/runs/neural --merge
"""

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf

SR = 16000

# The same conditions as crates/vox-trust-bench (noise uses numpy's generator, so the noise
# itself differs from the Rust run but has the same power).
CONDITIONS = [
    ("original", None),
    ("resample-44k", ("filter", "aresample=44100,aresample=16000")),
    ("trim-1.234s", ("trim", 19744)),
    ("noise-30db", ("noise", 30.0)),
    ("noise-20db", ("noise", 20.0)),
    ("noise-10db", ("noise", 10.0)),
    ("mp3-128k", ("codec", ["-ar", "44100", "-c:a", "libmp3lame", "-b:a", "128k"], "mp3")),
    ("mp3-64k", ("codec", ["-ar", "44100", "-c:a", "libmp3lame", "-b:a", "64k"], "mp3")),
    ("aac-64k", ("codec", ["-ar", "44100", "-c:a", "aac", "-b:a", "64k"], "m4a")),
    ("opus-32k", ("codec", ["-c:a", "libopus", "-b:a", "32k", "-application", "voip"], "ogg")),
    ("opus-24k", ("codec", ["-c:a", "libopus", "-b:a", "24k", "-application", "voip"], "ogg")),
    ("opus-12k", ("codec", ["-c:a", "libopus", "-b:a", "12k", "-application", "voip"], "ogg")),
    ("amr-wb-12.65k", ("codec", ["-ar", "16000", "-c:a", "libvo_amrwbenc", "-b:a", "12.65k"], "amr")),
    ("amr-wb-23.85k", ("codec", ["-ar", "16000", "-c:a", "libvo_amrwbenc", "-b:a", "23.85k"], "amr")),
    ("g722", ("codec", ["-ar", "16000", "-c:a", "g722"], "wav")),
    ("denoise", ("filter", "afftdn=nr=20:nf=-40")),
    ("echo", ("filter", "aecho=0.8:0.7:40|60:0.3|0.2")),
    ("tempo+1%", ("filter", "atempo=1.01")),
]


def ffmpeg(ff, args):
    subprocess.run([ff, "-hide_banner", "-loglevel", "error", "-y", *args], check=True)


def apply(ff, cond, x, tmp, rng):
    if cond is None:
        return x
    kind = cond[0]
    if kind == "trim":
        return x[cond[1]:]
    if kind == "noise":
        sigma = np.sqrt(np.mean(x**2) / 10 ** (cond[1] / 10))
        return (x + sigma * rng.standard_normal(len(x))).astype(np.float32)
    src, out = tmp / "in.wav", tmp / "out.wav"
    sf.write(src, np.clip(x, -1, 32767 / 32768), SR, subtype="PCM_16")
    if kind == "filter":
        ffmpeg(ff, ["-i", str(src), "-af", cond[1], "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", str(out)])
    else:
        mid = tmp / f"mid.{cond[2]}"
        ffmpeg(ff, ["-i", str(src), *cond[1], str(mid)])
        ffmpeg(ff, ["-i", str(mid), "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", str(out)])
    y, _ = sf.read(out, dtype="float32")
    return y


class AudioSealModel:
    name = "audioseal"
    bits = 16

    def __init__(self):
        import torch
        from audioseal import AudioSeal

        self.torch = torch
        self.gen = AudioSeal.load_generator("audioseal_wm_16bits")
        self.det = AudioSeal.load_detector("audioseal_detector_16bits")

    def embed(self, x, msg):
        t = self.torch.from_numpy(x)[None, None, :]
        m = self.torch.tensor(msg)[None, :]
        with self.torch.no_grad():
            w = self.gen.get_watermark(t, SR, message=m)
        return (t + w)[0, 0].numpy()

    def detect(self, y):
        t = self.torch.from_numpy(np.ascontiguousarray(y, dtype=np.float32))[None, None, :]
        with self.torch.no_grad():
            prob, msg = self.det.detect_watermark(t, SR)
        return float(prob) >= 0.5, [int(b) for b in msg[0].tolist()]


class WavMarkModel:
    name = "wavmark"
    bits = 16

    def __init__(self):
        import wavmark

        self.wm = wavmark
        self.model = wavmark.load_model()

    def embed(self, x, msg):
        y, _ = self.wm.encode_watermark(self.model, x, np.array(msg), show_progress=False)
        return y.astype(np.float32)

    def detect(self, y):
        payload, _ = self.wm.decode_watermark(self.model, np.asarray(y, dtype=np.float32), show_progress=False)
        if payload is None:
            return False, None
        return True, [int(b) for b in payload]


def quality(ref, deg):
    from pesq import pesq
    from pystoi import stoi

    n = min(len(ref), len(deg))
    return pesq(SR, ref[:n], deg[:n], "wb"), stoi(ref[:n], deg[:n], SR)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ffmpeg", required=True)
    ap.add_argument("--corpus", default=str(Path(__file__).parent / "corpus"))
    ap.add_argument("--out", required=True)
    ap.add_argument("--models", default="audioseal,wavmark")
    ap.add_argument("--shard", default="0/1", help="K/N: this process takes every N-th file")
    ap.add_argument("--merge", action="store_true", help="combine the shard files in --out")
    ap.add_argument("--conditions", help="comma-separated subset of conditions (default: all)")
    ap.add_argument("--fa-conditions", help="subset for the false-alarm runs (default: same)")
    args = ap.parse_args()
    out = Path(args.out)
    if args.merge:
        merge(out)
        return
    k, n_shards = (int(v) for v in args.shard.split("/"))

    names = [c for c, _ in CONDITIONS]
    chosen = args.conditions.split(",") if args.conditions else names
    fa_chosen = args.fa_conditions.split(",") if args.fa_conditions else chosen
    for c in chosen + fa_chosen:
        if c not in names:
            ap.error(f"unknown condition {c}")
    conditions = [(c, cond) for c, cond in CONDITIONS if c in chosen]
    fa_conditions = [(c, cond) for c, cond in CONDITIONS if c in fa_chosen]

    files = sorted(Path(args.corpus).glob("*.wav"))[k::n_shards]
    speech = [f for f in files if "room-tone" not in f.name]
    out.mkdir(parents=True, exist_ok=True)
    try:
        import torch

        torch.set_num_threads(1)
    except ImportError:
        pass
    results = {}
    for name in args.models.split(","):
        model = {"audioseal": AudioSealModel, "wavmark": WavMarkModel}[name]()
        rng = np.random.default_rng(1)
        per = {c: {"exact": 0, "bits": [], "found": 0} for c, _ in conditions}
        false_alarms = {c: 0 for c, _ in fa_conditions}
        pesqs, stois = [], []
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            for f in speech:
                x, sr = sf.read(f, dtype="float32")
                assert sr == SR, f
                msg = [int(b) for b in rng.integers(0, 2, model.bits)]
                marked = np.clip(model.embed(x, msg), -1, 1)
                p, s = quality(x, marked)
                pesqs.append(p)
                stois.append(s)
                for c, cond in conditions:
                    found, got = model.detect(apply(args.ffmpeg, cond, marked, tmp, rng))
                    r = per[c]
                    r["found"] += found
                    if got is not None:
                        r["bits"].append(float(np.mean(np.array(got) == np.array(msg))))
                        r["exact"] += got == msg
                    else:
                        r["bits"].append(0.5)
                print(f"{name} {f.name} done", file=sys.stderr, flush=True)
            for f in files:
                x, _ = sf.read(f, dtype="float32")
                for c, cond in fa_conditions:
                    found, _ = model.detect(apply(args.ffmpeg, cond, x, tmp, rng))
                    false_alarms[c] += found
        results[name] = {
            "recordings": len(speech),
            "unmarked_runs_per_condition": len(files),
            "pesq": pesqs,
            "stoi": stois,
            "conditions": {
                c: {
                    "exact": per[c]["exact"],
                    "bits": per[c]["bits"],
                    "detected": per[c]["found"],
                    "false_alarms": false_alarms.get(c),
                }
                for c, _ in conditions
            },
        }
        (out / f"shard-{k}-of-{n_shards}.json").write_text(json.dumps(results, indent=2))


def merge(out):
    """Sums the shard files into results.json (rates over all recordings)."""
    merged = {}
    for f in sorted(out.glob("shard-*.json")):
        for name, r in json.loads(f.read_text()).items():
            m = merged.setdefault(name, {"recordings": 0, "unmarked_runs_per_condition": 0, "pesq": [], "stoi": [], "conditions": {}})
            m["recordings"] += r["recordings"]
            m["unmarked_runs_per_condition"] += r["unmarked_runs_per_condition"]
            m["pesq"] += r["pesq"]
            m["stoi"] += r["stoi"]
            for c, v in r["conditions"].items():
                mc = m["conditions"].setdefault(c, {"exact": 0, "bits": [], "detected": 0, "false_alarms": 0})
                mc["exact"] += v["exact"]
                mc["bits"] += v["bits"]
                mc["detected"] += v["detected"]
                if v["false_alarms"] is None:
                    mc["false_alarms"] = None
                elif mc["false_alarms"] is not None:
                    mc["false_alarms"] += v["false_alarms"]
    results = {}
    for name, m in merged.items():
        n = m["recordings"]
        results[name] = {
            "recordings": n,
            "unmarked_runs_per_condition": m["unmarked_runs_per_condition"],
            "pesq_wb_mean": float(np.mean(m["pesq"])),
            "pesq_wb_min": float(np.min(m["pesq"])),
            "stoi_mean": float(np.mean(m["stoi"])),
            "conditions": {
                c: {
                    "exact": v["exact"] / n,
                    "bits": float(np.mean(v["bits"])),
                    "detected": v["detected"] / n,
                    "false_alarms": v["false_alarms"],
                }
                for c, v in m["conditions"].items()
            },
        }
    (out / "results.json").write_text(json.dumps(results, indent=2))
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
