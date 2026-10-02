# Test vectors

Plain JSON, for specification version 0.2 (format version 0). Every key in these files is a **public test value: never use it to protect anything.** All byte strings are lowercase hex; integers in the seal layout are big-endian, in the WAV/manifest layout as the spec says.

- `seal-v0.json`: `packing` (fields → 13 seal bytes), `circle_tags` (HMAC tag derivation, with the exact authenticated bytes), `circle_key_ids`, `coarse_times` (Unix seconds → the 16-bit `time` field).
- `file-v0.json`: `cases[]` (positive vectors) and `negative[]`; each case has the original WAV, per-chunk digests, and for `circle` and `public` the full manifest, authenticator/signature and sealed WAV; the circle case also has a one-bit-tampered WAV with the expected modified chunk list.
  - `negative[]`: files that must **not** verify as `valid`. Each entry has `name`, `description`, `wav` (hex), `trust` and `expected`. `trust` is `{circle_key, circle_key_id, pinned_public}` where each field is hex (or an integer for the id) or `null` when the verifier has no such key. `expected` is either `{check, reason, modified_chunks, content_matches}` (the names are the `check` and `reason` strings of the JSON report, `modified_chunks` is the list of altered chunk indices, empty when the format differs) or `{"error": "<name>"}` when the container itself is rejected before any seal is read (`trailing_bytes`: bytes after the RIFF container). Covered: bad Ed25519 signature, wrong key id in a public manifest, truncated manifest, bad magic, unsupported version, wrong circle key, changed audio format (sample rate), a genuine signature under an unpinned key (`unknown_key` with `content_matches` true), a circle seal with no key, and trailing bytes.

Regenerate: `cargo run -p vox-trust-core --example gen_vectors`. Check against the spec text with a second implementation: `python3 tools/check_vectors.py --strict` (needs `pip install cryptography` for Ed25519). The Rust tests (`cargo test -p vox-trust-core`) and the WebAssembly tests reproduce them byte for byte.

A change to any vector is a change to the wire format and needs a spec update and a CHANGELOG entry.
