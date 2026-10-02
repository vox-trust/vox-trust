# Examples

| Example | What it shows | Run |
|---|---|---|
| [node](node/) | Seal a WAV file, verify it, tamper with it, see which second changed | `cd examples/node && npm install && node seal-verify.mjs` |
| [browser](browser/) | Record from the microphone, seal, verify, all in the page | serve the folder, open `index.html` |
| [Rust](../crates/vox-trust-core/examples/seal_and_verify.rs) | The same with `vox-trust-core` | `cargo run -p vox-trust-core --example seal_and_verify` |

Start with the [integration guide](../docs/INTEGRATION.md).
