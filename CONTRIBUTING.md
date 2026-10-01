# Contributing to Vox Trust

Thanks for looking. The project is pre-alpha, so the most useful contributions right now are **scrutiny and measurement**, not features.

## Most useful right now

- **Attack the design.** Read [spec/THREAT-MODEL.md](spec/THREAT-MODEL.md) and [spec/SPEC.md](spec/SPEC.md) and open an issue with the flaw, ambiguity or missing attacker.
- **Measure.** The roadmap's first phase is measuring how audio watermarks survive real phone and app audio paths (Opus, AAC, MP3, AMR-WB, noise suppression, re-recording). Reproducible results are very welcome.
- **Independent implementations.** A spec is only real when someone other than its author can implement it. Once test vectors exist, a second implementation in another language is the most valuable contribution.

## Ground rules

- Open an issue before a large change, so the direction is agreed first.
- Keep changes small and focused. Explain the *why* in the pull request.
- Rust code must pass `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test`.
- Do not add claims the project cannot back up. If a sentence says "proves" or "guarantees", it needs a threat-model entry.
- Be kind. Disagreement about design is fine; contempt is not.

## Licensing of contributions

By submitting a contribution you agree it is licensed under the repository's licenses: Apache-2.0 for code, CC BY 4.0 for specification text (inbound = outbound). You must have the right to submit it.
