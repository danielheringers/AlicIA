# Reorganizacao Clean + DDD da Alicia (Frontend + Backend + Contrato)

This ExecPlan is a living document. Keep `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` updated as work proceeds.

This plan follows monorepo guidelines in `../AGENTS.md` (relative to `alicia/`).

## Purpose / Big Picture

Reorganizar `alicia/frontend` e `alicia/backend` para uma arquitetura Clean + DDD sem big-bang, reduzindo acoplamento entre UI, orquestracao de casos de uso, infraestrutura Tauri e contrato runtime.

Resultado observavel esperado:
1. UI deixa de depender diretamente de `tauri-bridge` nos componentes.
2. `main.rs` deixa de ser ponto central de regras e vira bootstrap + registro.
3. Contrato Tauri (commands/events/capabilities) passa a ter fonte unica de verdade.
4. Casos de uso ficam agrupados por contexto de negocio, com limites claros.

## Scope

Incluido:
1. `alicia/frontend/**`
2. `alicia/backend/**`
3. Novo subprojeto de contrato compartilhado: `alicia/codex-bridge/**`

Excluido:
1. Refactor profundo de crates em `codex/codex-rs/**` (tratado como contexto externo).
2. Mudanca de comportamento funcional de produto fora do necessario para reorganizacao.

## Progress

- [x] (2026-03-03) Diagnostico arquitetural read-only concluido com evidencias de acoplamento.
- [x] (2026-03-03) Estrategia incremental em fases aprovada.
- [x] (2026-03-03) Fase 0: baseline de contrato e smoke atual.
- [x] (2026-03-03) Fase 1: centralizacao do contrato em `alicia/codex-bridge`.
- [x] (2026-03-03) Fase 2: extracao de portas/casos de uso no frontend.
- [x] (2026-03-03) Fase 3 (slice 1): camada `interface/tauri` criada e comandos low/medium migrados.
- [x] (2026-03-03) Fase 3 (slice 2): comandos Session/Thread/Review migrados para `interface/tauri`.
- [x] (2026-03-03) Fase 3 (slice 3): handlers Account/MCP/App migrados para `interface/tauri`.
- [ ] (2026-03-03) Fase 3: extracao de interface Tauri no backend (conclusao completa).
- [ ] (2026-03-03) Fase 4: separacao de contextos no backend.
- [ ] (2026-03-03) Fase 5: enforcement de fronteiras + limpeza de legados.

## Fase 0 Baseline Snapshot (Read-Only)

Metadata:
1. Data: 2026-03-03.
2. Escopo: doc-only em `frontend/docs/**`, `backend/docs/**`, `alicia/_execplans/**`.
3. Artefatos:
   - `frontend/docs/clean-ddd-baseline-frontend.md`
   - `backend/docs/clean-ddd-baseline-backend.md`

Reconciliacao de contrato:
1. Commands Tauri registrados no backend: 70 (`src/main.rs` + `generate_handler!`).
2. Commands mapeados no bridge frontend: 69.
3. Gap identificado: `codex_native_runtime_diagnose` (backend-only, sem wrapper frontend).
4. Runtime methods reconciliados: conjunto de methods alinhado entre `types.ts` e `RUNTIME_METHOD_KEYS`.
5. Events reconciliados: canais `codex://stdout|stderr|lifecycle|event` e `terminal://data|exit` documentados em ambos os baselines.

Evidencia objetiva de baseline reproduzivel:
1. Inventario backend commands via parse de `generate_handler!` (`count=70`).
2. Inventario frontend command strings do bridge (`count=69`).
3. Diff de comandos front/back (`only_backend=codex_native_runtime_diagnose`).

Checklist de pronto da Fase 0:
1. [x] commands front/back reconciliados
2. [x] runtime methods reconciliados
3. [x] events + payloads reconciliados
4. [x] smoke matrix publicada com evidencia
5. [x] riscos e pendencias registrados

## Fase 1 Entrega e Evidencias

Metadata:
1. Data: 2026-03-03.
2. Objetivo: contrato unico em `alicia/codex-bridge` consumido por `frontend` e `backend`.
3. Artefatos centrais:
   - `codex-bridge/schema/runtime-contract.json`
   - `codex-bridge/generators/generate-runtime-contract.mjs`
   - `codex-bridge/generators/check-runtime-contract.mjs`
   - `frontend/lib/tauri-bridge/generated/runtime-contract.ts`
   - `backend/src/generated/runtime_contract.rs`

Correcoes de blocker aplicadas:
1. Saida do gerador corrigida para raiz `alicia` (evita escrita em `Neuromancer/frontend|backend`).
2. Template gerado alinhado ao consumo real:
   - frontend: `RUNTIME_COMMANDS` e `RUNTIME_CHANNELS`
   - backend: `RUNTIME_METHOD_KEYS` e `EVENT_CHANNEL_*`
3. `codex_native_runtime_diagnose` incluido no contrato TS gerado (`codexNativeRuntimeDiagnose`).
4. Fluxo oficial documentado para materializar artefatos externos:
   - `node alicia/codex-bridge/generators/generate-runtime-contract.mjs --write-external`

Validacao executada:
1. `node alicia/codex-bridge/generators/check-runtime-contract.mjs` -> OK.
2. `node alicia/codex-bridge/generators/generate-runtime-contract.mjs --write-external` -> OK.
3. `cd alicia/frontend && pnpm run lint` -> OK.
4. `cd alicia/frontend && pnpm exec tsc --noEmit` -> OK.
5. `cd alicia/frontend && pnpm run build` -> OK.
6. `cd alicia/backend && cargo check` -> OK.
7. `cd alicia/backend && cargo test` -> OK (`121 passed; 0 failed`).

Resultado:
1. Fase 1 concluida sem blocker de merge.

## Fase 2 Entrega e Evidencias

Metadata:
1. Data: 2026-03-03.
2. Objetivo: mover acesso de runtime para `application + ports + infrastructure/tauri` e remover import direto de `tauri-bridge` em `components/alicia/*`.
3. Artefatos centrais:
   - `frontend/lib/application/**`
   - `frontend/lib/infrastructure/tauri/tauri-runtime-client.adapter.ts`
   - `frontend/components/alicia/{apps-panel,mcp-panel,adt-panel,command-input,command-palette,model-picker,model-picker-parts,conversation-pane}.tsx`

Entregas:
1. Porta de runtime em `lib/application/ports/runtime-client.port.ts`.
2. Casos de uso por contexto (`account`, `mcp`, `adt`) em `lib/application/*`.
3. Adapter Tauri dedicado em `lib/infrastructure/tauri/tauri-runtime-client.adapter.ts`.
4. Contrato tipado da camada application:
   - `lib/application/runtime-types.ts`
   - `lib/application/contracts/runtime-methods.contract.ts`
5. Componentes alvo sem import direto de `@/lib/tauri-bridge*`.

Validacao executada:
1. `cd alicia/frontend && pnpm run lint` -> OK.
2. `cd alicia/frontend && pnpm exec tsc --noEmit` -> OK.
3. `cd alicia/frontend && pnpm run build` -> OK.
4. `rg -n "@/lib/tauri-bridge|@/lib/tauri-bridge/types" alicia/frontend/components/alicia` -> sem ocorrencias.
5. `rg -n "tauri-runtime-client.adapter|tauri-bridge/types|tauri-bridge/generated" alicia/frontend/lib/application` -> sem ocorrencias.

Resultado:
1. Fase 2 concluida sem blocker de merge.
2. Fronteira arquitetural reforcada para o frontend priorizado nesta fase.

## Fase 3 Slice 1 Entrega e Evidencias

Metadata:
1. Data: 2026-03-03.
2. Objetivo: iniciar Fase 3 com extracao da camada `interface/tauri` no backend, reduzindo responsabilidade direta do `main.rs`.
3. Escopo do slice:
   - criacao de `backend/src/interface/tauri/{commands,dto}`
   - migracao de comandos Tauri de baixo/medio risco
   - preservacao de comandos fora de escopo em `main.rs` para proximo slice

Entregas:
1. Camada `interface/tauri` criada:
   - `backend/src/interface/mod.rs`
   - `backend/src/interface/tauri/mod.rs`
   - `backend/src/interface/tauri/commands/*.rs`
   - `backend/src/interface/tauri/dto/*.rs`
2. Comandos migrados:
   - utility: `codex_help_snapshot`, `pick_workspace_folder`, `pick_image_file`, `pick_mention_file`
   - runtime/config: `load_codex_default_config`, `update_codex_config`, `codex_config_get`, `codex_config_set`, `codex_runtime_status`, `codex_runtime_capabilities`
   - workspace/git: `run_codex_command`, `git_workspace_changes`, `codex_workspace_*`, `git_commit_approved_review`
   - terminal: `terminal_create`, `terminal_write`, `terminal_resize`, `terminal_kill`
   - session lifecycle: `start_codex_session`, `resize_codex_pty`, `stop_codex_session`
3. `main.rs` atualizado para bootstrap + registro desses handlers migrados via `generate_handler!`.

Validacao executada:
1. `cd alicia/backend && cargo fmt --all` -> OK.
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`121 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. Verificacao de paridade: comandos fora de escopo (`codex_turn_*`, `codex_thread_*`, `codex_review_start`, `codex_approval_respond`, `codex_user_input_respond`, `send_codex_input`, `neuro_*`, `codex_native_runtime_diagnose`) seguem registrados no `generate_handler!`.

Resultado:
1. Slice 1 da Fase 3 concluido sem blocker de merge.
2. Fase 3 completa permanece em aberto para migracao dos grupos de comandos de maior acoplamento (turn/thread/review/approval/user_input/neuro).

## Fase 3 Slice 2 Entrega e Evidencias

Metadata:
1. Data: 2026-03-03.
2. Objetivo: continuar a extracao da interface Tauri no backend, removendo de `main.rs` os comandos e DTOs de Session/Thread/Review.
3. Escopo do slice:
   - migracao de handlers `codex_turn_*`, `codex_thread_*`, `codex_review_start`, `codex_approval_respond`, `codex_user_input_respond`, `send_codex_input`
   - centralizacao dos DTOs do contexto em `backend/src/interface/tauri/dto/session_turn.rs`
   - ajuste de consumo em `session_turn_runtime.rs`

Entregas:
1. Novo modulo de handlers:
   - `backend/src/interface/tauri/commands/session_turn.rs`
2. Novo modulo de DTOs:
   - `backend/src/interface/tauri/dto/session_turn.rs`
3. Exports atualizados:
   - `backend/src/interface/tauri/commands/mod.rs`
   - `backend/src/interface/tauri/dto/mod.rs`
4. `main.rs` reduzido para bootstrap + registro dos handlers migrados.
5. `session_turn_runtime.rs` ajustado para importar DTOs da camada `interface/tauri/dto`.

Validacao executada:
1. `cd alicia/backend && cargo fmt --all -- --check` -> OK.
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`121 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. Verificacao de paridade:
   - os 16 comandos migrados estao registrados no `generate_handler!`;
   - comandos fora de escopo permaneceram registrados, com contagem total de handlers mantida (`70`).

Resultado:
1. Slice 2 da Fase 3 concluido sem blocker de merge.
2. Fase 3 completa permanece em aberto para os grupos fora deste slice (principalmente `neuro_*` e comandos de runtime nao migrados).

## Fase 3 Slice 3 Entrega e Evidencias

Metadata:
1. Data: 2026-03-03.
2. Objetivo: extrair handlers do contexto Account/MCP/App para `interface/tauri`, reduzindo ainda mais o `main.rs`.
3. Escopo do slice:
   - migracao de handlers `codex_wait_for_mcp_startup`, `codex_app_list`, `codex_account_*`, `codex_mcp_*`
   - preservacao do contrato de comando e defaults de request (`unwrap_or`/`unwrap_or_default`)
   - manutencao dos grupos fora de escopo (`codex_models_list`, `codex_native_runtime_diagnose`, `neuro_*`)

Entregas:
1. Novo modulo de handlers:
   - `backend/src/interface/tauri/commands/account_mcp.rs`
2. Exports atualizados:
   - `backend/src/interface/tauri/commands/mod.rs`
3. `main.rs` reduzido para import + registro dos handlers migrados, sem handlers locais desse contexto.

Validacao executada:
1. `cd alicia/backend && cargo fmt --all -- --check` -> OK (warnings conhecidos de nightly-only no rustfmt config).
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`121 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. Verificacao de paridade:
   - 9 comandos migrados removidos como handlers locais em `main.rs` e mantidos no `generate_handler!`;
   - comandos fora de escopo (`codex_models_list`, `codex_native_runtime_diagnose`, `neuro_*`) preservados no registro.

Resultado:
1. Slice 3 da Fase 3 concluido sem blocker de merge.
2. Fase 3 completa permanece em aberto para migracao do bloco `neuro_*` e ajustes finais do `main.rs`.

## Current Architecture Snapshot (Key Risks)

1. Frontend com `god component` em `app/page.tsx` concentrando UI + fluxo + regras.
2. Componentes de UI sem import direto de `tauri-bridge` nos painéis priorizados da Fase 2; ainda existe acoplamento residual via adapter concreto em alguns fluxos.
3. `main.rs` ja perdeu utility/runtime/workspace/terminal/session-thread-review/account-mcp, mas ainda concentra wiring e comandos dos contexts restantes (`models`, `neuro`, `native diagnose`).
4. Modulos backend monoliticos (`command_runtime.rs`, `session_turn_runtime.rs`, `neuro_runtime.rs`).
5. Contrato runtime duplicado entre frontend e backend.

## Target Architecture

Frontend:
```text
frontend/
  app/
  components/
  hooks/
  lib/
    domain/
    application/
    infrastructure/tauri/
    contract/
```

Backend:
```text
backend/src/
  main.rs
  interface/tauri/commands/
  interface/tauri/events/
  application/
  domain/
  infrastructure/
```

Contrato:
```text
alicia/codex-bridge/
  schema/
  generators/
  generated/
```

Direcao de dependencia esperada:
1. Front: `UI -> application -> domain`; `application -> infrastructure` via portas.
2. Back: `interface -> application -> domain`; `infrastructure` implementa portas.

## Bounded Contexts (Initial)

1. `Session/Thread/Review`
2. `Workspace`
3. `Account/Auth/RateLimit`
4. `MCP`
5. `ADT/Neuro`
6. `RuntimeConfig/Capabilities`

## Phase Plan

### Fase 0 - Baseline e Guardrails

Objetivo:
1. Congelar comportamento atual e reduzir risco de regressao invisivel.

Entregas:
1. Snapshot de contrato atual (commands/events/method keys) documentado.
2. Checklist de smoke funcional atual consolidado.

Criterio de pronto:
1. Baseline reproduzivel em CI/local.

### Fase 1 - Contrato Unico (`alicia/codex-bridge`)

Objetivo:
1. Eliminar drift de contrato entre frontend e backend.

Entregas:
1. Definicao unica versionada de commands/events/capabilities.
2. Geracao de tipos para frontend.
3. Geracao de constantes/tipos para backend.

Criterio de pronto:
1. Front/back compilam consumindo artefatos gerados.
2. `RUNTIME_METHODS` nao fica mais duplicado manualmente.

### Fase 2 - Frontend First (Application + Ports)

Objetivo:
1. Tirar regras e infraestrutura dos componentes/pagina raiz.

Entregas:
1. Casos de uso por contexto em `lib/application/*`.
2. Portas de runtime em `lib/application/ports/*`.
3. Adapter Tauri em `lib/infrastructure/tauri/*`.
4. `app/page.tsx` reduzido para composicao/orquestracao de tela.

Criterio de pronto:
1. Componentes em `components/alicia/*` nao importam `tauri-bridge` diretamente.

### Fase 3 - Backend Interface First

Objetivo:
1. Reduzir `main.rs` para bootstrap/registro de comandos.

Entregas:
1. Comandos Tauri movidos para `interface/tauri/commands/*`.
2. DTOs de entrada/saida movidos para camada de interface.
3. Wiring claro entre interface e application services.

Criterio de pronto:
1. `main.rs` sem regras de caso de uso.

### Fase 4 - Backend por Contexto

Objetivo:
1. Separar modulos monoliticos em casos de uso por bounded context.

Entregas:
1. `application/<context>/*` com casos de uso coesos.
2. `domain/<context>/*` com regras puras.
3. `infrastructure/*` para FS/process/app-server/neuro adapters.
4. Deduplicacao de parsing rate-limit e handshake app-server.

Criterio de pronto:
1. Cada contexto com fronteira clara e sem dependencia circular.

### Fase 5 - Enforcement

Objetivo:
1. Garantir manutencao da arquitetura no tempo.

Entregas:
1. Regras de import-boundary (frontend/backend).
2. Checks automatizados de contrato.
3. Remocao de aliases/fallbacks legados ja obsoletos.

Criterio de pronto:
1. CI falha quando uma fronteira arquitetural e violada.

## Execution Model (Multi-Agent / Ownership)

Ordem padrao por fase:
1. `alicia-planner` (read-only): detalhamento da fase.
2. Implementadores em paralelo quando aplicavel:
   - `alicia-frontend` -> apenas `alicia/frontend/**`
   - `alicia-backend` -> apenas `alicia/backend/**`
   - `alicia-codex-bridge` -> apenas `alicia/codex-bridge/**`
3. `alicia-test`: validacao da fase.
4. `alicia-reviewer` (read-only): revisao de corretude/regressao/seguranca.

## Validation and Acceptance

Validacao tecnica por fase:
1. `cd alicia/frontend && pnpm lint && pnpm test && pnpm build`
2. `cd alicia/backend && cargo check && cargo test`
3. Fases finais de backend: `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings`

Smoke funcional minimo:
1. Sessao (`start/stop`, `turn.run`, `thread.open/read/list`)
2. Workspace (`read/write/list/create/rename`)
3. MCP (`list/login/reload`)
4. Account (`read/login/logout/rate limits`)
5. ADT/Neuro (`server list/select/connect`, `search/get/update source`)

## Risks and Mitigations

1. Regressao de contrato runtime.
   Mitigacao: contrato versionado + testes de serializacao/deserializacao.
2. Regressao em fluxo de eventos real-time.
   Mitigacao: testes de sequencia/event replay.
3. Refactor grande em arquivos centrais.
   Mitigacao: strangler pattern por fase, sem substituicao total imediata.
4. Incompatibilidade temporaria entre front e back durante migracao.
   Mitigacao: adapter anti-corruption com prazo de remocao definido.

## Idempotence and Recovery

1. Cada fase deve ser mergeavel de forma independente.
2. Nao usar comandos git destrutivos para “limpar” migracao.
3. Em caso de regressao, rollback por PR/fase, nao por reset global.

## Outcomes & Retrospective

Fase 0:
1. Resultado entregue: baseline contratual e smoke checklist consolidados para frontend e backend.
2. Incidentes/regressoes: nenhuma regressao funcional (escopo doc-only).
3. Riscos residuais:
   - worktree com mudancas nao relacionadas pode contaminar validacao se entrar no mesmo merge;
   - referencias `arquivo:linha` podem sofrer drift conforme edicoes futuras.
4. Ajuste para Fase 1:
   - tratar `codex_native_runtime_diagnose` explicitamente no contrato central (`alicia/codex-bridge`) como endpoint backend-only ou expor wrapper frontend.

Fase 1:
1. Resultado entregue: contrato centralizado em `alicia/codex-bridge` com geracao confiavel para frontend/backend.
2. Incidentes/regressoes: blockers iniciais de path/template foram corrigidos e validados.
3. Riscos residuais:
   - existem artefatos legados homonimos em `Neuromancer/frontend` e `Neuromancer/backend` (fora de `alicia/`) que podem gerar confusao operacional;
   - `check-runtime-contract` ainda nao compara conteudo gerado vs arquivos externos para drift automatizado.
4. Ajuste para Fase 2:
   - iniciar extracao de portas/casos de uso no frontend usando o contrato gerado como unica fonte de comandos/canais.

Fase 2:
1. Resultado entregue: frontend reorganizado com `application + ports + infrastructure/tauri` e componentes alvo desacoplados de `tauri-bridge`.
2. Incidentes/regressoes: ajustes de tipagem e concorrencia (timer MCP por servidor) corrigidos durante validacao.
3. Riscos residuais:
   - `runtime-methods.contract.ts` ainda e copia local da lista de metodos e pode sofrer drift sem guarda automatica;
   - faltam testes unitarios de use-cases/adapters e teste de concorrencia para login MCP.
4. Ajuste para Fase 3:
   - extrair interface Tauri no backend, reduzindo `main.rs` para bootstrap/registro e alinhando portas por contexto.

Fase 3 (slices 1-3):
1. Resultado entregue: camada `interface/tauri` consolidada para comandos de baixo/medio risco, Session/Thread/Review e Account/MCP/App, com `main.rs` progressivamente reduzido.
2. Incidentes/regressoes: nenhum blocker detectado em check/test/clippy e revisao read-only.
3. Riscos residuais:
   - faltam testes dedicados de serializacao/contrato para DTOs centralizados (ex.: `dto/session_turn.rs`);
   - faltam testes de integracao para validar automaticamente registro/invocacao dos comandos Tauri no `generate_handler!`.
4. Ajuste para continuidade da Fase 3:
   - migrar comandos restantes de maior acoplamento (`neuro_*`, `codex_native_runtime_diagnose` e bloco residual de `main.rs`) para `interface/tauri` mantendo paridade total.
