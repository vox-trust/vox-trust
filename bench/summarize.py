#!/usr/bin/env python3
"""Builds the comparison table of a benchmark folder from its runs' JSON files.

Usage: python3 bench/summarize.py bench/results/DATE-NAME
The folder holds one sub-folder per operating point (default, quality, robust,
long-window), each with results.json and quality.json from vt-bench and quality.py.
Prints a Markdown table; the folder's README.md embeds it.
"""
import json
import sys
from pathlib import Path

POINTS = [
    ("default", "Default (step 7 dB, 6.4 s)"),
    ("quality", "Quality (step 5 dB, 6.4 s)"),
    ("robust", "Robust (step 9 dB, 6.4 s)"),
    ("long-window", "Long window (step 7 dB, 9.6 s)"),
]


def main() -> None:
    root = Path(sys.argv[1])
    points = [(n, label) for n, label in POINTS if (root / n / "results.json").exists()]
    runs, quality, rows = {}, {}, {}
    for name, _ in points:
        runs[name] = json.loads((root / name / "results.json").read_text())
        quality[name] = json.loads((root / name / "quality.json").read_text())
        rows[name] = {r["condition"]: r for r in runs[name]["rows"]}
    order = [r["condition"] for r in runs[points[0][0]]["rows"]]

    def cells(fn):
        return " | ".join(fn(n) for n, _ in points)

    def rate(r):
        return f"{100 * r['recovered'] / max(1, r['windows']):.1f}%"

    out = ["| | " + " | ".join(label for _, label in points) + " |", "|---|" + "---:|" * len(points)]
    out.append("| Window (one seal) | " + cells(lambda n: f"{runs[n]['window_seconds']:.1f} s") + " |")
    out.append("| PESQ-WB, mean (min) | " + cells(lambda n: f"{quality[n]['pesq_wb_mean']:.2f} ({quality[n]['pesq_wb_min']:.2f})") + " |")
    out.append("| STOI, mean | " + cells(lambda n: f"{quality[n]['stoi_mean']:.3f}") + " |")
    out.append("| SNR / segmental SNR | " + cells(lambda n: f"{runs[n]['snr_db']:.1f} / {runs[n]['segmental_snr_db']:.1f} dB") + " |")
    for c in order:
        out.append(f"| `{c}` | " + cells(lambda n: rate(rows[n][c])) + " |")
    out.append("| Wrong seals returned, all conditions | " + cells(lambda n: str(sum(r["wrong_seals"] for r in rows[n].values()))) + " |")
    out.append("| False alarms on unmarked audio | " + cells(lambda n: f"{sum(r['false_alarms'] for r in rows[n].values())} in {sum(r['unmarked_files'] for r in rows[n].values())} runs") + " |")
    out.append("| Highest sync score on unmarked audio (threshold 6.0) | " + cells(lambda n: f"{max(r['max_unmarked_sync_score'] for r in rows[n].values()):.2f}") + " |")
    print("\n".join(out))


if __name__ == "__main__":
    main()
