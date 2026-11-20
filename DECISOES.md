# DECISOES.md — Registrado por Igor (2026-11-20)

Decisões que são de Igor e foram informadas à IA dev. A IA não decide esses itens.

## Push de branch (autorização)
- A IA dev trabalha apenas localmente na branch `agent/web-llm`.
- Push de branch: **proibido sem autorização explícita de Igor**.
- Push em `main`: **proibido em definitivo**.
- Nenhuma publicação na intranet foi aprovada.

## D10 — Conciliação (diferença ≠ 0)
- **ADIADO formalmente**. A IA não decide qual regra financeira vale.
- Comportamento interim (até Igor decidir): bloquear na camada web, documentado em `DIVERGENCIAS.md`, sem alterar o core.
- A IA deve registrar o comportamento efetivo, não criar uma regra nova.

## C1 — Ubuntu do PC de teste (192.168.1.184)
- **Aguardando resposta de Igor** (`lsb_release -a` no destino).
- Define se o C1 exige build no destino / binário estático (Ubuntu 22.04 glibc 2.35, ou 24.04 glibc 2.39) ou apenas prova de requisitos (Ubuntu 25.04+ glibc ≥ 2.41).
- O build atual (máquina de desenvolvimento: glibc 2.41, símbolo máximo exigido GLIBC_2.34) não pode ser validado no destino sem acesso à rede/à máquina 184 → C1 fica **NÃO VERIFICADA** até Igor confirmar a versão e o binário rodar ali.

## C3 — Centavos inteiros (i64) em core, banco e API
- **NÃO incluído no piloto C2**. É uma mudança de contrato e de banco (migração com backup, esquema versionado, banco legado preservado).
- Deve ser entregue como uma entrega separada, com teste próprio, depois que o piloto estiver estável.
- A IA propõe: não iniciar a migração monetária neste ciclo; apenas documentar o requisito e o plano separado.

## C4 — Rascunho x cupom confirmado
- **NÃO incluído no piloto C2**. É uma mudança de esquema (coluna `status`/`confirmado` em `receipts`) e de consulta (saldos/relatórios filtram só confirmados).
- Deve ser entregue como uma entrega separada, depois do piloto.
- A IA propõe: não misturar com o piloto; propor schema/migração em PR separado.

## Modelo do auditor no pc-sala (C10)
- **AUTORIZADO por Igor** apenas para a fase de auditoria, usando `AUDITORIA.md` + `tools/verificar.sh`.
- Troca registrada; modelo original restaurado ao fim da auditoria.
- A IA dev não acessa pc-sala, gestor ia, GPU, firewall.

## Dados reais (backups `.bak`)
- Nunca foram commitados (verificado com `git log --all` e `git ls-files`).
- Origem: `backup_antes_de_migrar()` do `crates/db` grava `cupons.db.<timestamp>.bak` ao lado do banco existente na primeira abertura com migração. Alguém rodou o app com CWD dentro do repositório e banco real presente → backups gerados na raiz.
- Ação de Igor (feita em 2026-11-20): movidos para `~/quarentena-dados-reais/` (fora do repo), permissões `600`, não lidos.
- Recomendação: mudar `backup_antes_de_migrar` para gravar num subdiretório dedicado (ex.: `./backups/`) e não na raiz do CWD, para evitar novos backups acidentais no repositório.
