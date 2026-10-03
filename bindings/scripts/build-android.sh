#!/usr/bin/env bash
# Builds the Android libraries and the Kotlin bindings into dist/vox-trust-android-<tag>.zip.
# Needs the Android NDK (ANDROID_NDK_HOME) and cargo-ndk. Usage: bindings/scripts/build-android.sh v0.6.0
set -euo pipefail
tag="$1"
cd "$(dirname "$0")/.."
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android >/dev/null
stage="target/android/vox-trust-android-${tag}"
rm -rf "$stage" && mkdir -p "$stage/kotlin"
cargo ndk --platform 24 -t arm64-v8a -t armeabi-v7a -t x86_64 -o "$stage/jniLibs" \
  build --locked --release -p vox-trust-ffi
cargo build --locked --release -p vox-trust-ffi
cargo run --locked --release --bin uniffi-bindgen -- generate --no-format \
  --library target/release/libvox_trust_ffi.so --language kotlin --out-dir "$stage/kotlin"
cp ../LICENSE ../NOTICE "$stage/"
cat > "$stage/README.md" <<'TXT'
# Vox Trust for Android

- `jniLibs/<abi>/libvox_trust_ffi.so`: copy `jniLibs/` into `app/src/main/`.
- `kotlin/io/github/voxtrust/vox_trust_ffi.kt`: add to your sources.
- Add `implementation("net.java.dev.jna:jna:5.17.0@aar")` to your dependencies.

Then `sealWav`, `verifyWav` and `decide` are available from `io.github.voxtrust`.
Guide: https://github.com/vox-trust/vox-trust/blob/main/docs/INTEGRATION.md
Pre-1.0, not audited.
TXT
mkdir -p ../dist
(cd target/android && zip -qr "../../../dist/vox-trust-android-${tag}.zip" "vox-trust-android-${tag}")
echo "dist/vox-trust-android-${tag}.zip"
