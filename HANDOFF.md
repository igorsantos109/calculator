# Handoff — Leitor de Cupons Fiscais

## Objetivo
Aplicativo local para registrar cupons fiscais brasileiros, revisar itens, classificá-los em Casa/Igor/Renan, calcular totais e persistir em SQLite.
Interface: SOMENTE página web (servida pelo próprio programa). egui e rfd removidos do projeto.
Backend LLM: llama-server (llama.cpp) em outro PC da LAN, via POST {llm_base_url}/chat/completions (formato OpenAI).

## Critérios de conclusão
- Fluxo manual completo funcional sem OCR/LLM
- Valores monetários em centavos inteiros
- Separação rascunho/cupom confirmado
- Persistência SQLite com schema versionado
- OCR e LLM opcionais e isolados
- CI (fmt, clippy, test, build) passando com --locked
- Relatórios por grupo, responsabilidade individual e saldo

## Restrições
- Rust edition 2021
- Produção sem Python/Node
- UI e erros em pt-BR; identificadores em inglês
- thiserror no domínio; anyhow só em UI/binário
- Não logar conteúdo integral de cupons
- Preservar dados, alterações locais, configurações existentes
- Não usar GitHub Copilot
- Não push/merge/branch sem autorização
- Não instalar deps/baixar modelos sem autorização
- Não modificar servidores de inferência, systemd, gestor ia, GPU
- Não expor segredos
- Não commitar target/, *.db, *.sqlite, imagens reais, .env
- Nunca usar ~/cupons.db; testes usam banco temporário
- Não acessar pc-sala, serviços, gestor "ia", GPU, firewall, configs de inferência
- llm_base_url do pc-sala é só um endereço configurável; nos testes usar servidor HTTP falso

## Estado atual (após PARADA 1 / ETAPA 1)
- Checkout: /home/igor/ia-sh/calculador
- Branch: agent/web-llm (a partir de main c8ce9bf)
- HEAD local: a968874 (2 commits à frente de c8ce9bf)
- origin/main: c8ce9bf (igual a main local) — sem push, sem pull
- Working tree limpo (salvo DIVERGENCIAS.md não rastreado, pré-existente)
- Crates no workspace: receipt_core (core), db. crates/ui retirado do workspace
  (commit separado, reversível — restaurar members no Cargo.toml reintegra)

## Commits locais (agent/web-llm)
1. 916e106 — ETAPA1: corrigir D1-D7 (validar_total em centavos, parsing centralizado,
   FK, backup, grupo)
   - D1: validar_total compara em centavos inteiros (round(x*100)), |diff| ≤ 5 centavos.
     Testes: 20.05 vs 20.00 ok; 100.05 vs 100.00 ok; 0.06 vs 0.00 rejeita.
   - D2: módulo core::parse (parse_valor, parse_quantidade, validar_data,
     reais_para_centavos, centavos_para_reais). Rejeita NaN/inf; aceita "5,50" e
     "1.234,56"; datas AAAA-MM-DD ou DD/MM/AAAA.
   - D3: enable_foreign_keys() ativa PRAGMA foreign_keys=ON fora de transação, antes
     de migrar; removido PRAGMA do SQL da migração v1. Testes órfão/cascade.
   - D4: Repositorio::new cria backup com timestamp antes de migrar banco existente;
     bancos novos não geram backup. Novo erro DbError::BackupFalhou.
   - D5: parse_grupo retorna erro para valor desconhecido (sem fallback Casa);
     buscar_itens_por_receipt_id ORDER BY id; listar_cupons ORDER BY id DESC.
   - D6: removido SchemaVersionAusenteBancoLegado (não usado); ordem de módulos em
     core/lib.rs: pub mod parse, pub mod ocr, depois mod tests.
   - D7: varredura: sem max(1.0)/conversão silenciosa de quantidade no core/db.
2. a968874 — ETAPA1: retirar crates/ui do workspace (commit separado, reversível)
   - Cargo.toml raiz: members = [crates/core, crates/db]
   - Cargo.lock: removidas dependências do ui (eframe, egui, rfd, dirs, etc.)
   - Arquivos de crates/ui mantidos no disco para reversão.

## Plano e progresso
| ID  | Tarefa                                                | Estado         |
|-----|-------------------------------------------------------|----------------|
| ETAPA 0 | Reconciliação + baseline (fmt/clippy/test/build)      | CONCLUÍDA      |
| ETAPA 1 | Depuração profunda D1-D7 + remover ui do workspace    | CONCLUÍDA      |
| PARADA 1 | Relatório + saída bruta + commits locais              | AGUARDANDO Igor|
| ETAPA 2 | Reescrever llm.rs para llama-server (API OpenAI)      | PENDENTE       |
| ETAPA 3 | Novo crate crates/web (servidor HTTP, API, testes)    | PENDENTE       |
| ETAPA 4 | Página interativa (HTML/CSS/JS embutidos)             | PENDENTE       |
| ETAPA 5 | Pacote + ROTEIRO_TESTE.md para Ubuntu 192.168.1.184   | PENDENTE       |

## Totais de testes (após ETAPA 1)
- receipt_core: 33 aprovados, 0 falhos, 1 ignorado (OCR fixture)
- db: 22 aprovados, 0 falhos, 0 ignorados
- doc-tests: 0

## Ambiente de build
- cargo 1.99.1, rustc 1.99.1
- glibc: 2.41 (Debian 2.41-12+deb13u4) — importante para compatibilidade no Ubuntu alvo
- Offline: --offline --locked funcionando

## Decisões pendentes de Igor
- D10: diferença de conciliação ≠ 0 bloqueia o cupom ou vai para alguém? Até decidir, bloquear na camada web e documentar em DIVERGENCIAS.md (não alterar core).
- Versão do Ubuntu alvo (para comparar glibc do build com o destino).
- Chave do llama-server (se houver) — nunca gravar em versão.
- Firewall/rede para 192.168.1.184.

## Arquivos alterados (ETAPA 1)
- Criado: crates/core/src/parse.rs
- Modificados: crates/core/src/lib.rs, crates/core/src/ocr.rs, crates/core/Cargo.toml,
  crates/db/src/lib.rs, crates/db/Cargo.toml, Cargo.toml (raiz), Cargo.lock

## Operações em andamento
- Nenhuma (PARADA 1)

## Última atualização
2026-11-16 — ETAPA 1 concluída. HEAD em a968874. Aguardando liberação para ETAPA 2.