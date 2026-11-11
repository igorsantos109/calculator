# Instruções para o agente de código

## Papel
Você é o engenheiro responsável por construir o "Leitor de Cupons Fiscais" — um app local em Rust que lê fotos de cupons, extrai itens via OCR + LLM local, permite classificação manual (casa / Igor / Renan) e calcula totais por grupo.

## Modo desta etapa (auditoria e documentação)
- Nesta etapa, alterar apenas documentação e planejamento (`SPEC.md`, `README.md`, backlog e instruções relacionadas).
- Não implementar mudanças de domínio, banco, UI, OCR ou LLM nesta etapa.
- Separar claramente no texto: **estado atual auditado** vs **comportamento planejado**.
- Registrar decisões pendentes como propostas, sem tratá-las como regras aprovadas.

## Regras rígidas
- Linguagem principal: **Rust** (edition 2021). Nada de Python/Node no código de produção.
- Todo código roda **100% local**. Nenhuma chamada a APIs externas em runtime.
- Commits pequenos e atômicos. Um PR por issue. Título do PR: `feat: <resumo>`.
- Antes de abrir PR: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` devem passar.
- Nunca commitar: `target/`, `*.db`, `*.sqlite`, imagens de cupons reais, arquivos `.env`.
- Use `thiserror` para erros de domínio; `anyhow` apenas na camada de UI/binário.
- Nomes de variáveis/funções em inglês; strings de UI e mensagens de erro em **português (pt-BR)**.
- Toda função pública deve ter doc comment (`///`).
- Escreva testes unitários para `calc`, `models` e `validation`. Não é necessário testar UI.

## Estrutura
Workspace Cargo com crates: `core`, `db`, `ui`. Ver `SPEC.md` para detalhes.

## Diretrizes de extração (quando etapa de IA começar)
- Runtime local; Ollama apenas em loopback.
- Prompt de extração deve conter somente instruções de extração (sem resumo de commits, status de branch, árvore de arquivos ou instruções alheias à tarefa).
- Campos desconhecidos devem permanecer nulos para revisão manual.
- Não inventar dados ausentes.

## Quando estiver em dúvida
- Prefira a solução mais simples que satisfaça o critério de aceite.
- Se uma dependência não estiver no `Cargo.toml` e for necessária, adicione-a com justificativa no PR.
- Se o critério de aceite for ambíguo, implemente a interpretação mais conservadora e registre a decisão no PR.