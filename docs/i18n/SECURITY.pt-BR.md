> Tradução resumida. O [SECURITY.md em inglês](../../SECURITY.md) é a versão normativa.

# Política de segurança (resumo)

O Vox Trust é **v0.x, um rascunho**. A especificação não está congelada e o código não foi revisado nem auditado. Não dependa dele para proteger ninguém.

- **Versões suportadas:** somente a última versão em `main`.
- **Metas de divulgação:** confirmar o recebimento em até 7 dias e publicar correção e aviso (advisory) em até 90 dias após um relato válido. Este é um projeto voluntário: são metas, não garantias. Não há recompensa (bug bounty) por enquanto, mas os relatos são lidos e creditados, se você quiser.
- **Escopo:** o código e a especificação deste repositório e a demo em vox-trust.github.io/demo/. Fora do escopo: ataques que exigem a chave ou o dispositivo da vítima (veja o modelo de ameaças).

## Como relatar uma vulnerabilidade

Relate **em particular**, pelo relato privado de vulnerabilidades do GitHub:

<https://github.com/vox-trust/vox-trust/security/advisories/new>

Se essa página não estiver disponível para você, abra uma issue pública mínima com o título "security contact request" **sem nenhum detalhe**; um mantenedor combinará um canal privado. Não abra issue pública para uma vulnerabilidade. Informe o que encontrou, como reproduzir e qual parte da especificação ou do código é afetada.

## Vulnerabilidade ou feedback de design?

- **Vulnerabilidade (relate em particular):** um jeito de forjar um selo que verifica, um replay ou emenda que passa pela política, uma falha no código de referência que quebra uma garantia declarada.
- **Feedback de design (abra uma issue pública):** críticas ao modelo de ameaças, à especificação em rascunho ou a uma afirmação da documentação. O escrutínio público do design é bem-vindo.

Os atacantes e limites já conhecidos estão em [spec/THREAT-MODEL.md](../../spec/THREAT-MODEL.md).
