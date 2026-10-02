> Tradução resumida. O [CONTRIBUTING.md em inglês](../../CONTRIBUTING.md) é a versão normativa.

# Contribuindo com o Vox Trust (resumo)

O projeto está antes da 1.0 (especificação 0.2, modo arquivo como release candidate), então as contribuições mais úteis agora são **escrutínio e medição**, não funcionalidades.

## O mais útil agora

- **Ataque o design.** Leia [spec/THREAT-MODEL.md](../../spec/THREAT-MODEL.md) e [spec/SPEC.md](../../spec/SPEC.md) e abra uma issue com a falha, a ambiguidade ou o atacante que falta.
- **Meça.** A primeira fase do roteiro é medir como as marcas d'água de áudio sobrevivem a caminhos reais de áudio de telefone e de aplicativos (Opus, AAC, MP3, AMR-WB, supressão de ruído, regravação). Resultados reproduzíveis são muito bem-vindos.
- **Implementações independentes.** Uma especificação só é real quando alguém que não é seu autor consegue implementá-la. Há vetores de teste em `spec/test-vectors/`; uma segunda implementação em outra linguagem é a contribuição mais valiosa.

## Regras básicas

- Abra uma issue antes de uma mudança grande, para acertar a direção primeiro.
- Mantenha as mudanças pequenas e focadas. Explique o *porquê* no pull request.
- O código Rust deve passar em `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test --workspace`.
- Mudanças no `vox-trust-core` ou no código de arquivo de chave da CLI: rode `cargo mutants -p vox-trust-core` (ou `-p vox-trust-cli -f crates/vox-trust-cli/src/keyfile.rs`). Um mutante não pego precisa de um teste ou, se não puder mudar o comportamento, de uma entrada com o motivo em `.cargo/mutants.toml`. O `scripts/coverage.sh` deve continuar acima do piso.
- Mudanças no formato de transmissão (wire format) ou nos veredictos também exigem: a especificação, vetores regenerados (`cargo run -p vox-trust-core --example gen_vectors`), `python3 tools/check_vectors.py --strict` e, para mudanças em web/WASM, `scripts/build-web.sh && node --test tests/node/wasm.test.mjs && node tests/browser/demo.mjs`.
- Não acrescente afirmações que o projeto não consegue sustentar. Se uma frase diz "prova" ou "garante", ela precisa de uma entrada no modelo de ameaças.
- Siga o [código de conduta](../../CODE_OF_CONDUCT.md).

## Licenciamento das contribuições

Ao enviar uma contribuição, você concorda que ela é licenciada sob as licenças do repositório: Apache-2.0 para código, CC BY 4.0 para o texto da especificação (entrada = saída). Você precisa ter o direito de enviá-la.
