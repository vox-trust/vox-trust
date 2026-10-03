<p align="center">
  🌐 <a href="README.md">English</a> · <a href="README.pt-BR.md">Português (Brasil)</a> · <a href="README.es.md">Español</a> · <strong>简体中文</strong> · <a href="README.ar.md">العربية</a>
</p>

> 本文为译文。[英文 README](README.md) 与[规范](spec/SPEC.md)为规范性版本。

<h1 align="center">Vox Trust</h1>

<p align="center"><strong>不要去检测虚假的声音，而要证明真实的声音。</strong></p>

<p align="center">
一个开放协议，附带 Rust 参考实现：在源头为人声签章，之后任何人都可以直接在浏览器中验证。
</p>

<p align="center">
  <a href="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml/badge.svg"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-orange?logo=rust&logoColor=white">
  <img alt="WebAssembly" src="https://img.shields.io/badge/WebAssembly-no%20imports-654FF0?logo=webassembly&logoColor=white">
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue"></a>
  <img alt="Version 0.6.0" src="https://img.shields.io/badge/version-0.6.0-informational">
  <a href="https://crates.io/crates/vox-trust-core"><img alt="crates.io" src="https://img.shields.io/crates/v/vox-trust-core"></a>
  <a href="https://www.npmjs.com/package/vox-trust"><img alt="npm" src="https://img.shields.io/npm/v/vox-trust"></a>
  <img alt="Not audited" src="https://img.shields.io/badge/security-not%20audited-red">
</p>

<p align="center">
  <a href="https://vox-trust.github.io/demo/?lang=zh-Hans"><strong>在线演示</strong></a> ·
  <a href="https://vox-trust.github.io/zh/">网站</a> ·
  <a href="spec/SPEC.md">规范</a> ·
  <a href="spec/THREAT-MODEL.md">威胁模型</a> ·
  <a href="docs/ROADMAP.md">路线图</a>
</p>

> **状态：v0.5，文件模式可用。未经审计。** 你可以为一个 WAV 文件签章、验证它，并准确看到哪几秒被改动过，既可以在[浏览器演示](https://vox-trust.github.io/demo/?lang=zh-Hans)中进行，也可以使用命令行。已签章的文件**只有在逐位完全相同的副本中**才能保留签章。一种**实验性**音频水印已经实现并经过[测量](bench/results/2026-10-02-stdm-2/README.md)：它能经受 MP3、AAC、Opus 和轻微的变速，但无法经受电话通话编解码器和噪声；而且音频内的签章可以被复制到其他音频中，因此它**目前不给出验证结论**。在通过评审之前，请不要用它来保护任何人。

## 问题

如今克隆一个人的声音只需要几秒钟的音频。仅凭耳朵已经无法分辨真实声音与克隆声音，而检测器追逐的是一个不断移动的目标：检测能力提升，生成能力也随之提升。

## 思路

与其猜测一段声音是否伪造，不如检查真实的声音是否被**签章**。

1. **签章。** 说话者的设备在源头对音频签名。
2. **验证。** 任何掌握该协议的人都可以检查是哪把密钥签的章、何时签的，以及音频的哪些片段被改动过。
3. **决定。** 本地信任策略把结果转换为四种结论之一。

| 结论 | 含义 |
|---|---|
| **已验证** | 来自你所信任的密钥的有效签章。 |
| **无签章** | 没有签章（或签章来自你从未提供过的密钥），且你对对方没有预期。属于中性结果，**不代表**“伪造”。 |
| **警告** | 一个*始终*签章的联系人这次没有签章：值得再看一眼，因为压缩也可能去掉签章。 |
| **警报** | 签章已损坏，或来自你没有为该联系人固定的密钥，或来自一个*始终*签章的联系人却缺少签章（严格模式）。 |

两种模式：**圈子**（彼此认识的人，共享密钥）和**公开**（组织和公众人物，验证者固定其 Ed25519 密钥）。

## 30 秒上手

**在浏览器中**（不会上传任何内容；Rust 核心以 WebAssembly 方式运行）：打开[演示](https://vox-trust.github.io/demo/?lang=zh-Hans)，点击*签章*，然后用各个按钮尝试作弊，看验证器如何识破每一次企图。

**在命令行中**（Linux、macOS 和 Windows 的预编译程序及校验和与构建证明见[发布页面](https://github.com/vox-trust/vox-trust/releases/latest)；也可以从源码构建）：

```sh
cargo install --locked --git https://github.com/vox-trust/vox-trust --tag v0.6.0 vox-trust-cli

vox-trust keygen me.key                                  # asks for a passphrase
vox-trust seal speech.wav sealed.wav --mode circle --key me.key
vox-trust verify sealed.wav --circle-key me.key          # exit code 0 = verified
```

`keygen` 会用口令保护密钥（Argon2id 与 XChaCha20-Poly1305），每次使用密钥时都会询问口令。在脚本中可使用 `--passphrase-file` 或环境变量 `VOX_TRUST_PASSPHRASE`；`--plain` 会生成不受保护的密钥。

修改 `sealed.wav` 中的任意一个采样点后再次验证：退出码变为 3，并打印出被改动的片段。退出码：0 已验证，1 无签章，2 警告，3 警报。

**作为 Rust 库：**

```rust
// A sketch: `wav_bytes` is a 16-bit PCM WAV you already have; run inside a function returning a Result.
use vox_trust_core::file::{seal_wav, verify_wav, SealParams, Signer, Trust};
use vox_trust_core::{circle, SealCheck};

let key = [7u8; 32]; // use a random 32-byte secret in real life
let sealed = seal_wav(
    &wav_bytes,
    Signer::Circle { key: &key, key_id: circle::key_id(&key) },
    SealParams { created_unix: 1_700_000_000, counter: 1, chunk_frames: 16_000 },
)?;
let trust = Trust { circle: Some((circle::key_id(&key), &key)), pinned_public: None };
assert_eq!(verify_wav(&sealed, trust)?.check, SealCheck::Valid);
```

## 集成到你的应用

Vox Trust 是一个开放协议，设计为嵌入人们已在使用的应用：即时通讯、语音信箱、播客与新闻编辑工具、客服平台。发送时加封，接收时验证。

```sh
npm install vox-trust        # browsers and Node 20+, WebAssembly, no dependencies, types included
cargo add vox-trust-core     # Rust, no unsafe, no I/O
pip install vox-trust        # Python 3.9+
npx -y vox-trust-mcp         # MCP server: AI assistants verify recordings
```

```js
import { load } from "vox-trust";
const vt = await load();
const sealed = vt.seal(wav, { mode: "public", key: seed, createdUnix, chunkFrames: 16000 });
const report = vt.verify(sealed, { pinnedPublicKey });
vt.decide(report.check, { alwaysSeals: true, strict: false }); // "verified" | "unsealed" | "warning" | "alert"
```

[集成指南](docs/INTEGRATION.md)（英文）约需 10 分钟：密钥、配对、四种判定及各自应如何显示，并附可运行的 Node、浏览器（麦克风）和 Rust [示例](examples/)。Android（Kotlin）与 iOS（Swift）库随每个 [release](https://github.com/vox-trust/vox-trust/releases) 附带（[bindings](bindings/)）。Apache-2.0，无需调用任何服务，无需账号。

## 它不是什么

- 它**不是**深度伪造检测，也**不是**声纹识别。
- 它**不能**证明说话者是人类，或确是其自称的那个人。它只证明*某把密钥*为这段音频签了章。被盗的密钥或被入侵的设备同样会产生有效签章。
- “无签章”**不**意味着“伪造”：压缩和降噪都可能抹去水印。
- 文件模式**无法**经受 MP3、AAC、重采样或重新录音。那需要载体（carrier），而它目前还不存在。
- 它无法防御控制了发送方设备的攻击者。

攻击者、各项主张以及目前已发现的弱点（包括一个仍未解决的弱点）都在[威胁模型](spec/THREAT-MODEL.md)中。

## 如何测试

| 层级 | 检查内容 | 运行 |
|---|---|---|
| Rust 单元测试与集成测试 | 签章布局、HMAC 与 Ed25519（对照 RFC 4231 和 RFC 8032 的测试向量）、WAV 解析、清单、篡改、策略、C 接口和命令行 | `cargo test --workspace` |
| 已发布的测试向量 | [`spec/test-vectors/`](spec/test-vectors) 中逐字节一致的签章与清单 | 已包含在上一项中 |
| Python 交叉校验 | `tools/check_vectors.py` 依据规范文本重建每一个向量（标准库，Ed25519 另需 `cryptography`） | `pip install cryptography && python3 tools/check_vectors.py --strict` |
| WebAssembly 端到端 | 编译后的模块逐字节复现测试向量，能处理垃圾输入，且不泄漏内存 | `scripts/build-web.sh && node --test tests/node/wasm.test.mjs` |
| 真实浏览器 | 在无头 Chromium、Firefox 和 WebKit 中测试演示页面：签章、六种攻击、公钥固定、无障碍、手机宽度、深色模式 | `BROWSER=firefox node tests/browser/demo.mjs` |
| 模糊测试 | 所有解析不可信输入的代码，并检查不变量（签章后的音频总能通过验证，任何被翻转的采样位都能在正确的分段中被发现，配对文本只有一种形式） | `cd fuzz && cargo +nightly fuzz run verify_wav` |
| 覆盖率 | 测试执行到的 Rust 代码行比例；低于 96 % 时 CI 失败 | `scripts/coverage.sh` |
| 变异测试 | 逐一修改 `vox-trust-core` 和密钥文件代码中的运算符与返回值，并检查是否有测试失败；少数不会改变行为的修改连同原因列在 `.cargo/mutants.toml` 中 | `cargo mutants -p vox-trust-core`; `cargo mutants -p vox-trust-cli -f crates/vox-trust-cli/src/keyfile.rs` |
| 依赖 | 已知漏洞、许可证、来源 | `cargo deny check` |

CI 在 Linux、macOS 和 Windows 上运行 Rust 测试。

Python 校验由同一位作者编写，因此它是交叉校验，而不是独立实现。[规范仍然需要的正是一个独立实现。](docs/ROADMAP.md)

## 横向对比

业内大多数方案给 **AI 生成的音频**打标记，以便日后识别。Vox Trust 反其道而行：为**真人语音**作保，并提供任何人都能核验的证明。

| | <small>**Vox Trust**</small> | <small>Google SynthID</small> | <small>Meta AudioSeal</small> | <small>Resemble PerTh</small> | <small>深度伪造检测器¹</small> | <small>C2PA</small> |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| <small>为真人录音作保</small> | <small>✅</small> | <small>❌ 标记 AI 输出</small> | <small>❌ 标记 AI 输出</small> | <small>❌ 标记 AI 输出</small> | <small>⚠️ 估计</small> | <small>✅ 若录音应用签名</small> |
| <small>证明与说话者本人的密钥绑定</small> | <small>✅</small> | <small>❌</small> | <small>❌</small> | <small>❌</small> | <small>❌</small> | <small>✅ 签名者证书</small> |
| <small>指出被改动的秒数</small> | <small>✅</small> | <small>❌</small> | <small>⚠️ 区分有无标记的区域</small> | <small>❌</small> | <small>❌</small> | <small>❌ 仅整个文件</small> |
| <small>任何人都能离线验证</small> | <small>✅ 浏览器内</small> | <small>❌ 需 Google 检测器</small> | <small>✅</small> | <small>⚠️</small> | <small>❌</small> | <small>✅</small> |
| <small>规范与代码开放</small> | <small>✅</small> | <small>❌ 音频未开放</small> | <small>✅ 代码</small> | <small>⚠️ 代码</small> | <small>❌</small> | <small>✅ 规范</small> |
| <small>经受 MP3、AAC、Opus</small> | <small>⚠️ 实验性水印，实测 100 %</small> | <small>✅ 官方声称</small> | <small>✅ 实测</small> | <small>✅ 官方声称</small> | <small>不适用</small> | <small>❌ 元数据常被剥离</small> |
| <small>经受电话通话（AMR-WB）</small> | <small>❌ 11 %</small> | <small>未公布</small> | <small>❌ 实测 0 %</small> | <small>未公布</small> | <small>✅</small> | <small>❌</small> |
| <small>验证成本</small> | <small>毫秒级，无需 AI 模型</small> | <small>云服务</small> | <small>神经网络</small> | <small>神经网络</small> | <small>云服务</small> | <small>毫秒级</small> |
| <small>公开基准测试（含失败项）</small> | <small>✅</small> | <small>❌</small> | <small>学术论文</small> | <small>❌</small> | <small>厂商数据</small> | <small>不适用</small> |

“实测”指在[我们的基准测试](bench/results/2026-10-02-neural-baselines/README.md)中以相同语料和编解码器运行；“官方声称”指厂商公布的数据。¹ 例如 [Pindrop Pulse](https://www.pindrop.com/article/pindrop-pulse-for-audio-deepfake-detection/)：它估计声音是否为合成，这很有用，但只是概率，并不能证明是谁在说话。每个单元格的来源见[对比说明](docs/COMPARISON.md)（英文）。

## 定位

Vox Trust 建立在既有工作之上并与之互补，而不是取代它们：

- [C2PA](https://spec.c2pa.org/) 内容凭证：面向文件的来源证明，水印可作为“soft binding”（软绑定）。
- [AudioSeal](https://github.com/facebookresearch/audioseal)、[WavMark](https://github.com/wavmark/wavmark) 等音频水印，可用作载体。
- 关于公钥语音来源证明的研究，例如 [MerkleSpeech](https://arxiv.org/abs/2602.10166)。
- 轮换家庭暗号的应用（例如 Trust Onion）和检测厂商，是从其他方向应对同一种诈骗。
- STIR/SHAKEN 证明的是来电*号码*，而不是声音。

## 仓库结构

```
spec/        规范、威胁模型、测试向量
crates/
  vox-trust-core/   签章布局、圈子与公开模式、文件模式、策略、重放辅助、配对文本
  vox-trust-wasm/   作为 WebAssembly 模块的核心（纯 C 接口，无 imports）
  vox-trust-cli/    命令行工具 `vox-trust`
web/         浏览器演示（发布于 vox-trust.github.io/demo/）
bindings/    Kotlin（Android）、Swift（iOS、macOS）与 Python 绑定（UniFFI）
mcp/         MCP 服务器，供 AI 助手验证录音
npm/         npm 包 `vox-trust`（WebAssembly + JavaScript 封装 + 类型）
extension/   适用于 Chrome、Edge、Firefox 和 Safari 的浏览器扩展
examples/    集成示例：Node、浏览器、Rust
tests/       Node（WebAssembly）与浏览器测试
tools/       独立的向量校验
fuzz/        模糊测试目标（cargo-fuzz）与种子输入
scripts/     构建与发布辅助脚本
docs/        路线图
```

## 贡献、安全、许可

- 请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)（[中文摘要](docs/i18n/CONTRIBUTING.zh-CN.md)）。目前最有价值的是审视：攻击[威胁模型](spec/THREAT-MODEL.md)，或依据[规范](spec/SPEC.md)编写一个独立实现。
- 请**私下**报告漏洞：见 [SECURITY.md](SECURITY.md)（[中文摘要](docs/i18n/SECURITY.zh-CN.md)）。
- 代码：[Apache-2.0](LICENSE)。规范文本：CC BY 4.0，见 [spec/LICENSE.md](spec/LICENSE.md)。变更记录：[CHANGELOG.md](CHANGELOG.md)。
- 治理与名称使用：[GOVERNANCE.md](GOVERNANCE.md)、[TRADEMARKS.md](TRADEMARKS.md)。引用：[CITATION.cff](CITATION.cff)。
- 维护者：[Roger Oliveira](https://www.linkedin.com/in/rogeroliveira/) · [@rogeroliveira84](https://github.com/rogeroliveira84)。
