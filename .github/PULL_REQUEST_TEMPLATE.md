**What and why:**

- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass
- [ ] If the wire format or a verdict changes: spec, test vectors, `tools/check_vectors.py` and the threat model are updated
- [ ] No new claim without a threat-model entry
