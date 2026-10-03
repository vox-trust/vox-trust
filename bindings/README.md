# Bindings: Kotlin (Android), Swift (iOS, macOS), Python

`vox-trust-ffi` exposes the core through [UniFFI](https://mozilla.github.io/uniffi-rs/), which
generates idiomatic bindings for each language from one Rust crate. The interface mirrors the
JavaScript package:

| Function | What it does |
|---|---|
| `seal_wav(wav, mode, key, key_id, created_unix, counter, chunk_frames)` | Seals a 16-bit PCM WAV file, returns the sealed bytes |
| `verify_wav(wav, trust)` | Returns a `Report`: `check`, `modified_chunks`, `key_id`... |
| `decide(check, contact)` | The trust policy: `Verified`, `Unsealed`, `Warning` or `Alert` |
| `public_key(seed)`, `public_key_id(pk)`, `circle_key_id(key)` | Key helpers |

Kotlin and Swift use camelCase (`sealWav`, `verifyWav`). Errors are `VoxTrustError`
(`VoxTrustException` in Kotlin).

**Status:** pre-1.0, not audited, like the rest of the project. Each language runs the same
round trip in CI ([`.github/workflows/bindings.yml`](../.github/workflows/bindings.yml)).

## Get it

| Platform | How |
|---|---|
| Python 3.9+ | `pip install vox-trust` ([python/](python/)) |
| Android | `vox-trust-android-<version>.zip` from the [releases page](https://github.com/vox-trust/vox-trust/releases): `jniLibs/` plus the Kotlin file; needs JNA |
| iOS, macOS | `vox-trust-apple-<version>.zip` from the releases page: `VoxTrustFFI.xcframework` plus `VoxTrust.swift` |

Maven Central and Swift Package Manager distribution are not set up yet.

## Build and test locally

```sh
cd bindings
cargo test
# Python
(cd python && maturin build --release -o ../target/wheels) && pip install target/wheels/vox_trust-*.whl
python -m pytest python/tests
# Kotlin on the JVM (the same code Android uses)
cargo build --release -p vox-trust-ffi
cargo run --release --bin uniffi-bindgen -- generate --library target/release/libvox_trust_ffi.so \
  --language kotlin --out-dir kotlin/generated
(cd kotlin && gradle test)
# Swift (macOS)
swift/test.sh
# Release archives
scripts/build-android.sh v0.5.1   # needs the Android NDK and cargo-ndk
scripts/build-apple.sh v0.5.1     # macOS with Xcode
```

This is a separate Cargo workspace so that the core keeps its small dependency set. UniFFI
itself is licensed MPL-2.0 (file-level copyleft); the code here stays Apache-2.0.
