#!/usr/bin/env bash
# Builds VoxTrustFFI.xcframework (iOS, iOS simulator, macOS) and the Swift bindings into
# dist/vox-trust-apple-<tag>.zip. Runs on macOS with Xcode. Usage: bindings/scripts/build-apple.sh v0.6.0
set -euo pipefail
tag="$1"
cd "$(dirname "$0")/.."
targets="aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin"
rustup target add $targets >/dev/null
for t in $targets; do
  cargo build --locked --release -p vox-trust-ffi --target "$t"
done
work="target/apple" && stage="$work/vox-trust-apple-${tag}"
rm -rf "$work" && mkdir -p "$work/headers" "$work/ios-sim" "$work/macos" "$stage"
cargo run --locked --release --bin uniffi-bindgen -- generate --no-format \
  --library target/aarch64-apple-darwin/release/libvox_trust_ffi.dylib --language swift --out-dir "$work/gen"
cp "$work/gen/VoxTrustFFI.h" "$work/headers/"
cp "$work/gen/VoxTrustFFI.modulemap" "$work/headers/module.modulemap"
lib=libvox_trust_ffi.a
lipo -create target/aarch64-apple-ios-sim/release/$lib target/x86_64-apple-ios/release/$lib -output "$work/ios-sim/$lib"
lipo -create target/aarch64-apple-darwin/release/$lib target/x86_64-apple-darwin/release/$lib -output "$work/macos/$lib"
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/$lib -headers "$work/headers" \
  -library "$work/ios-sim/$lib" -headers "$work/headers" \
  -library "$work/macos/$lib" -headers "$work/headers" \
  -output "$stage/VoxTrustFFI.xcframework"
cp "$work/gen/VoxTrust.swift" ../LICENSE ../NOTICE "$stage/"
cat > "$stage/README.md" <<'TXT'
# Vox Trust for iOS and macOS

- Drag `VoxTrustFFI.xcframework` into your Xcode target (Frameworks, Libraries, and Embedded Content).
- Add `VoxTrust.swift` to the same target.

Then `sealWav`, `verifyWav` and `decide` are available.
Guide: https://github.com/vox-trust/vox-trust/blob/main/docs/INTEGRATION.md
Pre-1.0, not audited.
TXT
mkdir -p ../dist
(cd "$work" && zip -qry "../../../dist/vox-trust-apple-${tag}.zip" "vox-trust-apple-${tag}")
echo "dist/vox-trust-apple-${tag}.zip"
