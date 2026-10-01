#!/usr/bin/env bash
# Builds the WebAssembly core and copies it next to the demo sources in web/.
# The build is meant to be reproducible: dependencies are locked (--locked) and local paths
# are remapped so they do not leak into the binary. For identical bytes, use the same Rust
# toolchain (`rustc -V`) and the same wasm32 target.
# Usage: scripts/build-web.sh
set -euo pipefail
cd "$(dirname "$0")/.."

repo="$(pwd -P)"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
remap="--remap-path-prefix=${repo}=/vox-trust"
remap="${remap} --remap-path-prefix=${cargo_home}/=/cargo/"
if [ "${cargo_home}" != "$HOME/.cargo" ]; then
  remap="${remap} --remap-path-prefix=$HOME/.cargo/=/cargo/"
fi
export RUSTFLAGS="${RUSTFLAGS:-} ${remap}"

rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build --locked -p vox-trust-wasm --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/vox_trust_wasm.wasm web/vox_trust.wasm
echo "web/vox_trust.wasm: $(wc -c < web/vox_trust.wasm) bytes"
echo "sha256: $(sha256sum web/vox_trust.wasm 2>/dev/null | cut -d' ' -f1 || shasum -a 256 web/vox_trust.wasm | cut -d' ' -f1)"
