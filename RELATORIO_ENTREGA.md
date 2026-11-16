# Relatório de Entrega — Leitor de Cupons Fiscais

## Contexto
Projeto: leitor de cupons fiscais (workspace Rust: core, db, ui → ui retirado).
Checkout: /home/igor/ia-sh/calculador
Remoto publicado: igorsantos109/calculator, main = c8ce9bf.
Branch local: agent/web-llm.

## SHA inicial / final
- SHA inicial (main): c8ce9bf
- SHA final (agent/web-llm): dcdb499 (4 commits locais, não publicados)

## Estado por ID (tabela de entrega — após ETAPA 2)

| ID   | Estado        | Commit    | Prova                                                        |
|------|---------------|-----------|--------------------------------------------------------------|
| D1   | CONCLUÍDO     | 916e106   | testes 20.05 vs 20.00 ok; 100.05 vs 100.00 ok; 0.06 rejeita |
| D2   | CONCLUÍDO     | 916e106   | módulo core::parse, 9 testes passando                        |
| D3   | CONCLUÍDO     | 916e106   | testes órfão/cascade/FK passando (3 testes)                  |
| D4   | CONCLUÍDO     | 916e106   | testes backup (arquivo existente/novo) passando (2 testes)   |
| D5   | CONCLUÍDO     | 916e106   | parse_grupo erro; ORDER BY id; histórico id DESC (3 testes)  |
| D6   | CONCLUÍDO     | 916e106   | SchemaVersionAusenteBancoLegado removido; ordem de módulos   |
| D7   | CONCLUÍDO     | 916e106   | varredura: sem conversão silenciosa de quantidade            |
| D8   | PARCIAL       | —         | CI --locked OK; doc-tests/integração pendentes (ETAPA 3-4)   |
| ui   | REMOVIDO      | a968874   | crates/ui retirado do workspace (reversível)                 |
| ETAPA 2 | CONCLUÍDO   | dcdb499   | llm.rs reescrito para llama-server; 12 testes com servidor falso |
| ETAPA 3 | PENDENTE   | —         | crate crates/web (servidor HTTP)                             |
| ETAPA 4 | PENDENTE   | —         | página interativa (HTML/CSS/JS)                              |
| ETAPA 5 | PENDENTE   | —         | pacote + ROTEIRO_TESTE.md para Ubuntu 192.168.1.184          |

## Testes por pacote (após ETAPA 2)
- receipt_core: 34 aprovados, 0 falhos, 1 ignorado (OCR fixture)
- db: 22 aprovados, 0 falhos, 0 ignorados
- doc-tests: 0

## Dependências (offline, --locked)
- receipt_core: serde, serde_json, thiserror, image, imageproc, alvio-ocr, tracing,
  reqwest (0.12, json), tokio (1, full)
- dev: wiremock (0.6)
- db: receipt_core, rusqlite (bundled), thiserror, serde, serde_json
- dev: tempfile (3)
- ui removido (eframe, egui, rfd, dirs, etc. retirados do lock)

## Decisões pendentes de Igor
- D10: diferença de conciliação ≠ 0 bloqueia o cupom ou vai para alguém? Até decidir, bloquear na camada web e documentar em DIVERGENCIAS.md.
- Versão do Ubuntu alvo (para comparar glibc do build: 2.41).
- Chave do llama-server (se houver) — nunca gravar em versão.
- Firewall/rede para 192.168.1.184.

## Limitações (NÃO VERIFICADO)
- OCR real (alvio-ocr sem fixture, teste ignorado)
- LLM real / servidor llama-server real (testes usam servidor HTTP falso)
- Navegador real
- Rede real / intranet
- Banco real (~cupons.db nunca usado; testes usam banco temporário)

## Como rodar (após ETAPA 2)
```bash
cd /home/igor/ia-sh/calculador
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline --locked -- -D warnings
cargo test --workspace --offline --locked
cargo build --workspace --release --offline --locked
```

## Próxima ação
Aguardar liberação de Igor para PARADA 2, depois iniciar ETAPA 3 (crate crates/web).
