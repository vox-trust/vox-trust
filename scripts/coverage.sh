#!/usr/bin/env bash
# Line coverage of the Rust workspace (needs cargo-llvm-cov and the llvm-tools-preview
# component). Fails if line coverage drops below FLOOR. The benchmark harness is excluded:
# it drives ffmpeg and is exercised by the Benchmark workflow instead.
# Usage: scripts/coverage.sh [--html]
set -euo pipefail
cd "$(dirname "$0")/.."
FLOOR=96
args=(--workspace --locked --ignore-filename-regex 'crates/vox-trust-bench/|/examples/')
if [ "${1:-}" = "--html" ]; then
  cargo llvm-cov "${args[@]}" --html
  echo "report: target/llvm-cov/html/index.html"
else
  cargo llvm-cov "${args[@]}" --summary-only --fail-under-lines "$FLOOR"
fi
