<p align="center">
  🌐 <a href="README.md">English</a> · <strong>Português (Brasil)</strong> · <a href="README.es.md">Español</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.ar.md">العربية</a>
</p>

> Esta é uma tradução. O [README em inglês](README.md) e a [especificação](spec/SPEC.md) são as versões normativas.

<h1 align="center">Vox Trust</h1>

<p align="center"><strong>Não detecte vozes falsas. Prove as verdadeiras.</strong></p>

<p align="center">
Um protocolo aberto, com implementação de referência em Rust, que sela uma voz humana na origem e permite que qualquer pessoa a verifique depois, direto no navegador.
</p>

<p align="center">
  <a href="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml/badge.svg"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-orange?logo=rust&logoColor=white">
  <img alt="WebAssembly" src="https://img.shields.io/badge/WebAssembly-no%20imports-654FF0?logo=webassembly&logoColor=white">
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue"></a>
  <img alt="Version 0.3.0" src="https://img.shields.io/badge/version-0.3.0-informational">
  <img alt="Not audited" src="https://img.shields.io/badge/security-not%20audited-red">
</p>

<p align="center">
  <a href="https://vox-trust.github.io/demo/?lang=pt-BR"><strong>Demo ao vivo</strong></a> ·
  <a href="https://vox-trust.github.io/pt/">Site</a> ·
  <a href="spec/SPEC.md">Especificação</a> ·
  <a href="spec/THREAT-MODEL.md">Modelo de ameaças</a> ·
  <a href="docs/ROADMAP.md">Roteiro</a>
</p>

> **Status: v0.3, o modo arquivo funciona. Sem auditoria.** Você pode selar um arquivo WAV, verificá-lo e ver exatamente quais segundos foram alterados, na [demo no navegador](https://vox-trust.github.io/demo/?lang=pt-BR) ou pela linha de comando. Arquivos selados sobrevivem **somente a cópias idênticas bit a bit**. Uma marca d'água de áudio **experimental**, que sobrevive a MP3, AAC e Opus, foi construída e [medida](bench/results/2026-10-01-stdm-1/README.md). Mas ela falha em codecs de ligação telefônica e com ruído, e selos dentro do áudio podem ser copiados para outro áudio, então ela **ainda não dá veredito**. Não use isto para proteger ninguém antes de ele ser revisado.

## O problema

Clonar uma voz hoje leva segundos de áudio. Ouvir já não basta para distinguir uma voz real de uma clonada, e os detectores perseguem um alvo em movimento: conforme a detecção melhora, a geração também melhora.

## A ideia

Em vez de adivinhar se uma voz é falsa, verifique se uma voz real foi **selada**.

1. **Selar.** O dispositivo de quem fala assina o áudio na origem.
2. **Verificar.** Qualquer pessoa com o protocolo confere qual chave selou, quando, e quais trechos do áudio foram alterados.
3. **Decidir.** Uma política de confiança local transforma o resultado em um de três desfechos.

| Desfecho | Significado |
|---|---|
| **Verificado** | Um selo válido de uma chave em que você confia. |
| **Sem selo** | Nenhum selo, de alguém que nunca usou o protocolo. Neutro, **não** é "falso". |
| **Alerta** | O selo está quebrado, vem de uma chave que você não fixou para esse contato, ou está ausente em um contato que *sempre* sela (modo estrito). |

Dois modos: **círculo** (pessoas que se conhecem, segredo compartilhado) e **público** (organizações e figuras públicas, chaves Ed25519 que os verificadores fixam).

## Experimente em 30 segundos

**No navegador** (nada é enviado; o núcleo em Rust roda como WebAssembly): abra a [demo](https://vox-trust.github.io/demo/?lang=pt-BR), clique em *Selar* e depois tente trapacear com os botões e veja o verificador pegar cada tentativa.

**Na linha de comando** (binários prontos para Linux, macOS e Windows, com checksums e atestados de build, estão na [página de releases](https://github.com/vox-trust/vox-trust/releases/latest); ou compile do código-fonte):

```sh
cargo install --locked --git https://github.com/vox-trust/vox-trust --tag v0.3.0 vox-trust-cli

vox-trust keygen me.key                                  # asks for a passphrase
vox-trust seal speech.wav sealed.wav --mode circle --key me.key
vox-trust verify sealed.wav --circle-key me.key          # exit code 0 = verified
```

O `keygen` protege a chave com uma senha (Argon2id e XChaCha20-Poly1305) e a pede sempre que a chave é usada. Em scripts, use `--passphrase-file` ou a variável `VOX_TRUST_PASSPHRASE`; `--plain` grava uma chave sem proteção.

Edite uma única amostra de `sealed.wav` e verifique de novo: o código de saída passa a ser 3 e o trecho alterado é exibido. Códigos de saída: 0 verificado, 1 sem selo, 2 aviso, 3 alerta.

**Como biblioteca Rust:**

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

## O que isto NÃO é

- **Não** é detecção de deepfake e **não** é biometria de voz.
- **Não** prova que quem fala é humano ou é quem diz ser. Prova que *uma chave* selou o áudio. Uma chave roubada ou um dispositivo comprometido produz selos válidos.
- "Sem selo" **não** significa "falso": compressão e supressão de ruído podem apagar uma marca d'água.
- O modo arquivo **não** sobrevive a MP3, AAC, reamostragem ou regravação. Isso exige o portador (carrier), que ainda não existe.
- Não protege contra um atacante que controla o dispositivo de quem envia.

Atacantes, alegações e as fraquezas encontradas até agora (incluindo uma que continua sem solução) estão no [modelo de ameaças](spec/THREAT-MODEL.md).

## Como é testado

| Camada | O que verifica | Comando |
|---|---|---|
| Testes unitários e de integração em Rust | Layout do selo, HMAC e Ed25519 (contra os vetores das RFC 4231 e RFC 8032), leitura de WAV, manifestos, adulteração, política, a interface C e a CLI | `cargo test --workspace` |
| Vetores de teste publicados | Selos e manifestos idênticos byte a byte em [`spec/test-vectors/`](spec/test-vectors) | incluído acima |
| Verificação cruzada em Python | `tools/check_vectors.py` reconstrói cada vetor a partir do texto da especificação (biblioteca padrão, mais `cryptography` para Ed25519) | `pip install cryptography && python3 tools/check_vectors.py --strict` |
| WebAssembly de ponta a ponta | O módulo compilado reproduz os vetores byte a byte, lida com entrada inválida e não vaza memória | `scripts/build-web.sh && node --test tests/node/wasm.test.mjs` |
| Navegador real | A página da demo no Chromium, Firefox e WebKit headless: selagem, seis ataques, fixação de chave pública, acessibilidade, largura de celular, modo escuro | `BROWSER=firefox node tests/browser/demo.mjs` |
| Fuzzing | Todo parser de entrada não confiável, com invariantes (áudio selado sempre verifica, qualquer bit de amostra alterado é achado no trecho certo, o texto de pareamento tem uma só forma) | `cd fuzz && cargo +nightly fuzz run verify_wav` |
| Cobertura | Fração das linhas de Rust que os testes executam; o CI falha abaixo de 96 % | `scripts/coverage.sh` |
| Testes de mutação | Altera operadores e valores de retorno do `vox-trust-core` e do código de arquivo de chave, um de cada vez, e confere que algum teste falha; as poucas alterações que não mudam o comportamento estão listadas com o motivo em `.cargo/mutants.toml` | `cargo mutants -p vox-trust-core` |
| Dependências | Vulnerabilidades conhecidas, licenças, origens | `cargo deny check` |

O CI roda os testes de Rust em Linux, macOS e Windows.

A verificação em Python foi escrita pelo mesmo autor, portanto é uma verificação cruzada, não uma implementação independente. [Uma implementação independente é o que a especificação ainda precisa.](docs/ROADMAP.md)

## Onde isto se encaixa

O Vox Trust se apoia em trabalhos existentes e os complementa, em vez de substituí-los:

- Credenciais de conteúdo [C2PA](https://spec.c2pa.org/): proveniência para arquivos, com marcas d'água como "soft bindings".
- Marcas d'água de áudio como [AudioSeal](https://github.com/facebookresearch/audioseal) e [WavMark](https://github.com/wavmark/wavmark), que poderiam servir de portadoras.
- Pesquisa sobre proveniência de fala com chave pública, por exemplo [MerkleSpeech](https://arxiv.org/abs/2602.10166).
- Aplicativos que alternam palavras-código da família (por exemplo, Trust Onion) e fornecedores de detecção atacam o mesmo golpe por outros lados.
- STIR/SHAKEN atesta o *número* de quem liga, não a voz.

## Estrutura do repositório

```
spec/        especificação, modelo de ameaças, vetores de teste
crates/
  vox-trust-core/   layout do selo, modos círculo e público, modo arquivo, política, auxiliares de replay, texto de pareamento
  vox-trust-wasm/   o núcleo como módulo WebAssembly (interface C simples, sem imports)
  vox-trust-cli/    a ferramenta de linha de comando `vox-trust`
web/         a demo no navegador (publicada em vox-trust.github.io/demo/)
tests/       testes Node (WebAssembly) e de navegador
tools/       verificação independente dos vetores
fuzz/        alvos de fuzzing (cargo-fuzz) e entradas-semente
scripts/     auxiliares de build e publicação
docs/        roteiro
```

## Contribuir, segurança, licença

- Leia o [CONTRIBUTING.md](CONTRIBUTING.md) ([resumo em português](docs/i18n/CONTRIBUTING.pt-BR.md)). O mais útil agora é o escrutínio: ataque o [modelo de ameaças](spec/THREAT-MODEL.md) ou escreva uma implementação independente a partir da [especificação](spec/SPEC.md).
- Reporte vulnerabilidades **em particular**: veja o [SECURITY.md](SECURITY.md) ([resumo em português](docs/i18n/SECURITY.pt-BR.md)).
- Código: [Apache-2.0](LICENSE). Texto da especificação: CC BY 4.0, veja [spec/LICENSE.md](spec/LICENSE.md). Alterações: [CHANGELOG.md](CHANGELOG.md).
- Governança e uso do nome: [GOVERNANCE.md](GOVERNANCE.md), [TRADEMARKS.md](TRADEMARKS.md). Para citar: [CITATION.cff](CITATION.cff).
- Mantenedor: [Roger Oliveira](https://www.linkedin.com/in/rogeroliveira/) · [@rogeroliveira84](https://github.com/rogeroliveira84).
