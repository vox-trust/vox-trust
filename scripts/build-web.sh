#!/usr/bin/env bash
# Builds the WebAssembly core and copies it next to the demo sources in web/.
# Usage: scripts/build-web.sh
set -euo pipefail
cd "$(dirname "$0")/.."

rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build -p vox-trust-wasm --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/vox_trust_wasm.wasm web/vox_trust.wasm
echo "web/vox_trust.wasm: $(wc -c < web/vox_trust.wasm) bytes"
