#!/usr/bin/env python3
"""Perceptual quality of the marked audio written by vt-bench.

Compares every bench/<run>/marked/NAME.wav with bench/corpus/NAME.wav using:
  - PESQ, wideband (ITU-T P.862.2), MOS-LQO from 1.0 to about 4.64 (higher is better);
  - STOI (short-time objective intelligibility), 0 to 1.

Usage: python3 bench/quality.py RUN_DIR [CORPUS_DIR]
Needs: pip install pesq pystoi numpy soundfile
Writes RUN_DIR/quality.json and prints a Markdown table.
"""
import json
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from pesq import pesq
from pystoi import stoi


def main() -> None:
    run = Path(sys.argv[1])
    corpus = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).parent / "corpus"
    rows = []
    for marked in sorted((run / "marked").glob("*.wav")):
        ref, rate = sf.read(corpus / marked.name, dtype="float32")
        deg, rate2 = sf.read(marked, dtype="float32")
        assert rate == rate2 == 16000, marked
        n = min(len(ref), len(deg))
        ref, deg = ref[:n], deg[:n]
        rows.append(
            {
                "file": marked.stem,
                "pesq_wb": round(float(pesq(16000, ref, deg, "wb")), 3),
                "stoi": round(float(stoi(ref, deg, 16000, extended=False)), 4),
            }
        )
    pesq_values = np.array([r["pesq_wb"] for r in rows])
    stoi_values = np.array([r["stoi"] for r in rows])
    summary = {
        "files": len(rows),
        "pesq_wb_mean": round(float(pesq_values.mean()), 3),
        "pesq_wb_min": round(float(pesq_values.min()), 3),
        "stoi_mean": round(float(stoi_values.mean()), 4),
        "stoi_min": round(float(stoi_values.min()), 4),
        "rows": rows,
    }
    (run / "quality.json").write_text(json.dumps(summary, indent=2) + "\n")
    print("| File | PESQ-WB | STOI |")
    print("|---|---:|---:|")
    for r in rows:
        print(f"| {r['file']} | {r['pesq_wb']:.2f} | {r['stoi']:.3f} |")
    print(
        f"| **mean (min)** | **{summary['pesq_wb_mean']:.2f}** ({summary['pesq_wb_min']:.2f}) "
        f"| **{summary['stoi_mean']:.3f}** ({summary['stoi_min']:.3f}) |"
    )


if __name__ == "__main__":
    main()
