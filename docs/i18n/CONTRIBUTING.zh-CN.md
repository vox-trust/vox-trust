> 本文为摘要译文。[英文 CONTRIBUTING.md](../../CONTRIBUTING.md) 为规范性版本。

# 参与贡献 Vox Trust（摘要）

项目目前是 v0.x 草案，因此现阶段最有价值的贡献是**审视与测量**，而不是新功能。

## 目前最有价值的事

- **攻击设计。** 阅读 [spec/THREAT-MODEL.md](../../spec/THREAT-MODEL.md) 和 [spec/SPEC.md](../../spec/SPEC.md)，并就缺陷、歧义或遗漏的攻击者提交 issue。
- **测量。** 路线图的第一阶段是测量音频水印在真实的电话和应用音频链路（Opus、AAC、MP3、AMR-WB、降噪、重新录音）中的存活情况。欢迎可复现的结果。
- **独立实现。** 只有当作者之外的人也能实现时，规范才算真正成立。`spec/test-vectors/` 中有测试向量；用另一种语言写出第二个实现是最有价值的贡献。

## 基本规则

- 做大的改动之前先开 issue，先对齐方向。
- 改动要小而聚焦。在 pull request 中说明*为什么*这样改。
- Rust 代码必须通过 `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings` 和 `cargo test --workspace`。
- 修改 `vox-trust-core` 或命令行的密钥文件代码时：运行 `cargo mutants -p vox-trust-core`（或 `-p vox-trust-cli -f crates/vox-trust-cli/src/keyfile.rs`）。未被捕获的变异体需要补一个测试；若它不可能改变行为，则在 `.cargo/mutants.toml` 中注明原因。`scripts/coverage.sh` 必须保持在下限之上。
- 修改线路格式（wire format）或结论判定，还需要：更新规范、重新生成测试向量（`cargo run -p vox-trust-core --example gen_vectors`）、运行 `python3 tools/check_vectors.py --strict`；涉及 web/WASM 的改动还需运行 `scripts/build-web.sh && node --test tests/node/wasm.test.mjs && node tests/browser/demo.mjs`。
- 不要加入项目无法支撑的说法。如果某句话写了“证明”或“保证”，就需要在威胁模型中有对应条目。
- 遵守[行为准则](../../CODE_OF_CONDUCT.md)。

## 贡献的许可

提交贡献即表示你同意其按本仓库的许可证授权：代码采用 Apache-2.0，规范文本采用 CC BY 4.0（入站 = 出站）。你必须有权提交该贡献。
