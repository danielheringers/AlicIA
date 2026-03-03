# Reorganizacao Clean + DDD da Alicia (Frontend + Backend + Contrato)

This ExecPlan is a living document. Keep `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` updated as work proceeds.

This plan follows `.agent/PLANS.md` from repository root.

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
- [ ] (2026-03-03) Fase 0: baseline de contrato e smoke atual.
- [ ] (2026-03-03) Fase 1: centralizacao do contrato em `alicia/codex-bridge`.
- [ ] (2026-03-03) Fase 2: extracao de portas/casos de uso no frontend.
- [ ] (2026-03-03) Fase 3: extracao de interface Tauri no backend.
- [ ] (2026-03-03) Fase 4: separacao de contextos no backend.
- [ ] (2026-03-03) Fase 5: enforcement de fronteiras + limpeza de legados.

## Current Architecture Snapshot (Key Risks)

1. Frontend com `god component` em `app/page.tsx` concentrando UI + fluxo + regras.
2. Componentes de UI chamando infraestrutura Tauri diretamente (`apps-panel`, `mcp-panel`, `adt-panel`).
3. `main.rs` concentrando DTOs, comandos, wiring e parte da coordenacao.
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

Preencher ao final de cada fase:
1. Resultado entregue e impacto tecnico.
2. Incidentes/regressoes encontradas.
3. Ajustes de estrategia para a fase seguinte.

