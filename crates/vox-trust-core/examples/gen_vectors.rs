//! Writes the published test vectors to `spec/test-vectors/`.
//!
//! Run: `cargo run -p vox-trust-core --example gen_vectors`

#[path = "../tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/test-vectors");
    fs::create_dir_all(&dir).expect("create test-vectors directory");
    for (name, value) in [
        ("seal-v0.json", common::seal_vectors()),
        ("file-v0.json", common::file_vectors()),
    ] {
        let mut text = serde_json::to_string_pretty(&value).unwrap();
        text.push('\n');
        fs::write(dir.join(name), text).expect("write vectors");
        println!("wrote spec/test-vectors/{name}");
    }
}
