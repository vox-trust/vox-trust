# Test vectors

Plain JSON, draft 0.1. Every key in these files is a **public test value: never use it to protect anything.** All byte strings are lowercase hex; integers in the seal layout are big-endian, in the WAV/manifest layout as the spec says.

- `seal-v0.json`: `packing` (fields → 13 seal bytes), `circle_tags` (HMAC tag derivation, with the exact authenticated bytes), `circle_key_ids`.
- `file-v0.json`: `cases[]`, each with the original WAV, per-chunk digests, and for `circle` and `public` the full manifest, authenticator/signature and sealed WAV; the circle case also has a one-bit-tampered WAV with the expected modified chunk list.

Regenerate: `cargo run -p vox-trust-core --example gen_vectors`. Check against the spec text with a second implementation: `python3 tools/check_vectors.py --strict` (needs `pip install cryptography` for Ed25519). The Rust tests (`cargo test -p vox-trust-core`) and the WebAssembly tests reproduce them byte for byte.

A change to any vector is a change to the wire format and needs a spec update and a CHANGELOG entry.
