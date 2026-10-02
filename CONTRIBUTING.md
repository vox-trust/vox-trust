# Contributing to Vox Trust

Thanks for looking. The project is pre-1.0 (specification 0.2, file mode a release candidate), so the most useful contributions right now are **scrutiny and measurement**, not features.

## Most useful right now

- **Attack the design.** Read [spec/THREAT-MODEL.md](spec/THREAT-MODEL.md) and [spec/SPEC.md](spec/SPEC.md) and open an issue with the flaw, ambiguity or missing attacker.
- **Measure.** The roadmap's first phase is measuring how audio watermarks survive real phone and app audio paths (Opus, AAC, MP3, AMR-WB, noise suppression, re-recording). Reproducible results are very welcome.
- **Independent implementations.** A spec is only real when someone other than its author can implement it. Test vectors exist in `spec/test-vectors/`; a second implementation in another language is the most valuable contribution.

## Ground rules

- Open an issue before a large change, so the direction is agreed first.
- Keep changes small and focused. Explain the *why* in the pull request.
- Rust code must pass `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.
- Changes to `vox-trust-core` or the CLI's key-file code: run `cargo mutants -p vox-trust-core` (or `-p vox-trust-cli -f crates/vox-trust-cli/src/keyfile.rs`). A missed mutant needs a test or, if it cannot change behaviour, an entry with the reason in `.cargo/mutants.toml`. `scripts/coverage.sh` must stay above its floor.
- Changes to the wire format or verdicts also need: the spec, regenerated vectors (`cargo run -p vox-trust-core --example gen_vectors`), `python3 tools/check_vectors.py --strict`, and for web/WASM changes `scripts/build-web.sh && node --test tests/node/*.mjs && node tests/browser/demo.mjs`. Parser changes: run the fuzz targets in `fuzz/` (`cargo +nightly fuzz run <target>`) for a few minutes.
- Do not add claims the project cannot back up. If a sentence says "proves" or "guarantees", it needs a threat-model entry.
- Follow the [code of conduct](CODE_OF_CONDUCT.md).

## Licensing of contributions

By submitting a contribution you agree it is licensed under the repository's licenses: Apache-2.0 for code, CC BY 4.0 for specification text (inbound = outbound). You must have the right to submit it.
