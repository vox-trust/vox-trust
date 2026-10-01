#!/usr/bin/env bash
# Builds the WebAssembly core and copies it next to the demo sources in web/.
# The build is reproducible: dependencies are locked (--locked), local paths are remapped so
# they do not leak into the binary, and the Rust toolchain is pinned in scripts/WASM_TOOLCHAIN
# (change it deliberately, in its own commit). Same toolchain + same commit = same bytes as
# the published release. Override with VT_WASM_TOOLCHAIN only for experiments.
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

toolchain="${VT_WASM_TOOLCHAIN:-$(cat scripts/WASM_TOOLCHAIN)}"
rustup toolchain install "${toolchain}" --profile minimal --target wasm32-unknown-unknown >/dev/null
cargo +"${toolchain}" build --locked -p vox-trust-wasm --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/vox_trust_wasm.wasm web/vox_trust.wasm
echo "toolchain: $(rustc +"${toolchain}" --version)"
echo "web/vox_trust.wasm: $(wc -c < web/vox_trust.wasm) bytes"
echo "sha256: $(sha256sum web/vox_trust.wasm 2>/dev/null | cut -d' ' -f1 || shasum -a 256 web/vox_trust.wasm | cut -d' ' -f1)"
