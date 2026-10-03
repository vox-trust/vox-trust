#!/usr/bin/env bash
# Builds the library, generates the Swift bindings and runs main.swift against them (macOS).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release -p vox-trust-ffi
out=target/swift-test
rm -rf "$out" && mkdir -p "$out"
cargo run --locked --release --bin uniffi-bindgen -- generate --no-format \
  --library target/release/libvox_trust_ffi.dylib --language swift --out-dir "$out"
swiftc -O -o "$out/test" "$out/VoxTrust.swift" swift/main.swift \
  -I "$out" -Xcc -fmodule-map-file="$out/VoxTrustFFI.modulemap" \
  -L target/release -lvox_trust_ffi
DYLD_LIBRARY_PATH=target/release "$out/test"
