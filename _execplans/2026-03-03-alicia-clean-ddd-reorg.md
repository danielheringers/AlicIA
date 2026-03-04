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
- [x] (2026-03-03) Fase 3 (slice 4): bloco neuro/native/models migrado; `main.rs` sem handlers Tauri locais.
- [x] (2026-03-03) Fase 3: extracao de interface Tauri no backend (conclusao completa).
- [x] (2026-03-04) Fase 4 (slice 1): contexto Workspace Filesystem extraido para application/domain/infrastructure.
- [x] (2026-03-04) Fase 4 (slice 2): contexto Account/MCP/App List extraido para application/domain/infrastructure.
- [x] (2026-03-04) Fase 4 (slice 3): recorte ADT/Neuro (server registry/select/connect) extraido para application/domain/infrastructure.
- [x] (2026-03-04) Fase 4 (slice 4): desacoplamento neuro_adt de neuro_runtime com contracts/ports/tipos proprios.
- [x] (2026-03-04) Fase 4 (slice 5): extracao Session/Thread/Review (thread list/read + review validation) para clean layers.
- [x] (2026-03-04) Fase 4 (slice 6): shared thread catalog extraido para runtime_bridge com desacoplamento de session_turn_runtime.
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

## Fase 3 Slice 4 Entrega e Evidencias

Metadata:
1. Data: 2026-03-03.
2. Objetivo: concluir a extracao de interface Tauri no backend, removendo o bloco restante de handlers (`neuro_*`, `codex_native_runtime_diagnose`, `codex_models_list`) do `main.rs`.
3. Escopo do slice:
   - criacao de `commands/neuro.rs`, `commands/native_runtime.rs`, `commands/models.rs`
   - extracao de DTOs de models para `dto/models.rs`
   - ajustes de imports em `command_runtime.rs` e `models_runtime.rs`

Entregas:
1. Handlers migrados para interface:
   - `backend/src/interface/tauri/commands/neuro.rs`
   - `backend/src/interface/tauri/commands/native_runtime.rs`
   - `backend/src/interface/tauri/commands/models.rs`
2. DTOs de modelagem movidos para:
   - `backend/src/interface/tauri/dto/models.rs`
3. `main.rs` reduzido a bootstrap + registro, com `0` ocorrencias de `#[tauri::command]`.
4. Reexports atualizados em:
   - `backend/src/interface/tauri/commands/mod.rs`
   - `backend/src/interface/tauri/dto/mod.rs`

Validacao executada:
1. `cd alicia/backend && cargo fmt --all -- --check` -> OK (warnings conhecidos do rustfmt nightly-only).
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`121 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. `node alicia/codex-bridge/generators/check-runtime-contract.mjs` -> OK.
6. `cd alicia/frontend && pnpm run build` -> OK.
7. `cd alicia/backend && cargo check --no-default-features --features custom-protocol` -> FAIL (erro preexistente fora do escopo deste slice, reproduzido em baseline limpo no `HEAD`, sem blocker para fluxo default).

Resultado:
1. Slice 4 da Fase 3 concluido sem blocker para fluxo default.
2. Fase 3 marcada como concluida (extração de interface Tauri no backend finalizada).

## Fase 4 Slice 1 Entrega e Evidencias

Metadata:
1. Data: 2026-03-04.
2. Objetivo: iniciar separacao por bounded context no backend, extraindo o contexto `Workspace Filesystem` para `application/domain/infrastructure` sem alterar contrato Tauri.
3. Escopo do slice:
   - migracao de `codex_workspace_*_impl` e helpers de path/filesystem para camadas internas
   - `command_runtime.rs` convertido em fachada/delegacao para workspace
   - hardening de seguranca em write com symlink dangling e comportamento deterministico em `has_children`

Entregas:
1. Estrutura Clean criada para workspace:
   - `backend/src/application/workspace/{mod.rs,use_cases.rs}`
   - `backend/src/domain/workspace/{mod.rs,path_policy.rs}`
   - `backend/src/infrastructure/filesystem/{mod.rs,workspace_fs.rs}`
   - modulos-raiz: `backend/src/{application,domain,infrastructure}/mod.rs`
2. Delegacao em `command_runtime.rs`:
   - `codex_workspace_read_file_impl`
   - `codex_workspace_write_file_impl`
   - `codex_workspace_create_directory_impl`
   - `codex_workspace_list_directory_impl`
   - `codex_workspace_rename_entry_impl`
3. Fixes de seguranca/regressao aplicados no slice:
   - bloqueio de write via symlink dangling no target
   - validacao deterministica de symlinks em `has_children`
4. Testes novos adicionados:
   - `workspace_write_rejects_dangling_symlink_target`
   - `has_children_rejects_outside_symlink_with_mixed_children_deterministically`

Validacao executada:
1. `cd alicia/backend && cargo fmt --all -- --check` -> OK.
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`123 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. Verificacao de paridade: 5 comandos workspace seguem registrados no `generate_handler!`.
6. Revisao tecnica final: findings ALTA/MEDIA resolvidos; sem blocker de merge para este slice.

Resultado:
1. Slice 1 da Fase 4 concluido sem blocker de merge.
2. Fase 4 completa permanece em aberto para separacao dos demais contexts (`Session/Thread/Review`, `Account/MCP`, `ADT/Neuro`).

## Fase 4 Slice 2 Entrega e Evidencias

Metadata:
1. Data: 2026-03-04.
2. Objetivo: extrair o contexto Account/MCP/App List de `command_runtime.rs` para camadas `application/domain/infrastructure`, mantendo contrato Tauri inalterado.
3. Escopo do slice:
   - migracao de `codex_app_list_impl`, `codex_account_*`, `codex_mcp_*`, `codex_wait_for_mcp_startup_impl`
   - criacao de modulos `application/account_mcp`, `domain/account_mcp`, `infrastructure/runtime_bridge`
   - adicao de testes para validacao de payloads/account/mcp e fallback de method nao suportado

Entregas:
1. Estrutura Clean criada para Account/MCP:
   - `backend/src/application/account_mcp/{mod.rs,use_cases.rs}`
   - `backend/src/domain/account_mcp/{mod.rs,validation.rs}`
   - `backend/src/infrastructure/runtime_bridge/{mod.rs,app_server.rs,mcp_native.rs}`
   - modulos-raiz atualizados: `backend/src/{application,domain,infrastructure}/mod.rs`
2. `command_runtime.rs` reduzido para delegacao dos fluxos:
   - `codex_wait_for_mcp_startup_impl`
   - `codex_app_list_impl`
   - `codex_account_read_impl`
   - `codex_account_login_start_impl`
   - `codex_account_logout_impl`
   - `codex_account_rate_limits_read_impl`
   - `codex_mcp_list_impl`
   - `codex_mcp_login_impl`
   - `codex_mcp_reload_impl`
3. Testes adicionais cobrindo riscos levantados na revisao:
   - payload e validacao de account/app list (`build_app_list_payload`, `validate_account_login_start_request`, `is_unsupported_method_error_for`)
   - fallback de `app/list` nao suportado com retorno vazio e estavel
   - mapeamento/aggregacao de MCP nativo (`mcp_entry_from_config`, `auth_status_label`, composicao da lista consolidada)

Validacao executada:
1. `cd alicia/backend && cargo fmt --all -- --check` -> OK.
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`136 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. `node alicia/codex-bridge/generators/check-runtime-contract.mjs` -> OK (`tauriCommands=70`).
6. `cd alicia/frontend && pnpm run build` -> OK.
7. Verificacao de paridade: 9 comandos Account/MCP/App seguem registrados no `generate_handler!`.

Resultado:
1. Slice 2 da Fase 4 concluido sem blocker de merge.
2. Fase 4 completa permanece em aberto para separacao dos contexts restantes (`Session/Thread/Review` e `ADT/Neuro`).
## Fase 4 Slice 3 Entrega e Evidencias

Metadata:
1. Data: 2026-03-04.
2. Objetivo: extrair o recorte `ADT/Neuro - Server Registry & Selection` de `neuro_runtime.rs` para `application/domain/infrastructure`, preservando contrato Tauri.
3. Escopo do slice:
   - migracao de `neuro_adt_server_list`, `neuro_adt_server_upsert`, `neuro_adt_server_remove`, `neuro_adt_server_select`, `neuro_adt_server_connect`
   - extracao de regras de normalizacao/invariantes do registry para `domain/neuro_adt`
   - extracao de persistencia do store JSON para `infrastructure/filesystem/neuro_server_store`

Entregas:
1. Estrutura Clean criada para o recorte ADT/Neuro server registry:
   - `backend/src/application/neuro_adt/{mod.rs,use_cases.rs}`
   - `backend/src/domain/neuro_adt/{mod.rs,server_store.rs}`
   - `backend/src/infrastructure/filesystem/neuro_server_store.rs`
   - modulos-raiz atualizados: `backend/src/{application,domain}/mod.rs` e `backend/src/infrastructure/filesystem/mod.rs`
2. `neuro_runtime.rs` reduzido para delegacao nos comandos server_*:
   - `neuro_adt_server_list_impl`
   - `neuro_adt_server_upsert_impl`
   - `neuro_adt_server_remove_impl`
   - `neuro_adt_server_select_impl`
   - `neuro_adt_server_connect_impl`
3. Testes novos adicionados para o slice:
   - dominio (`normalize/upsert/select/remove` + invariantes de active server)
   - infraestrutura (`load/save/parse error` + permissao `0600` em Unix)
   - aplicacao (`server_*` com cobertura de invalidacao de cache e contrato de resposta)

Validacao executada:
1. `cd alicia/backend && cargo fmt --all -- --check` -> OK.
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`146 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. `node alicia/codex-bridge/generators/check-runtime-contract.mjs` -> OK (`runtimeMethods=51`, `tauriCommands=70`, `tauriEventChannels=6`).
6. Verificacao de paridade: comandos Tauri `neuro_adt_server_*` e assinaturas publicas permanecem estaveis.

Revisao tecnica final:
1. Sem findings de severidade alta.
2. 1 finding medio registrado como divida de transicao: acoplamento residual de tipos/erros de `neuro_runtime` nas camadas extraidas.
3. 1 finding baixo de superficie de segredo (`password` com visibilidade `pub(crate)`), sem exploracao observada neste slice.

Resultado:
1. Slice 3 da Fase 4 concluido sem blocker alto de merge.
2. Fase 4 permanece em aberto para reduzir acoplamentos residuais e atacar os blocos restantes (`Session/Thread/Review` e demais fluxos ADT/Neuro).
## Fase 4 Slice 4 Entrega e Evidencias

Metadata:
1. Data: 2026-03-04.
2. Objetivo: remover acoplamento residual de `application/domain/infrastructure/neuro_adt` com `neuro_runtime`, mantendo contrato Tauri externo inalterado.
3. Escopo do slice:
   - introducao de contratos/ports/tipos/erro proprios do contexto `neuro_adt`
   - adaptacao de `neuro_runtime` para atuar como adapter/fachada do contexto
   - correcoes dos findings medios da revisao anterior (mapping de erro connect + lock global de env para testes)

Entregas:
1. Contratos e ports da camada de aplicacao:
   - `backend/src/application/neuro_adt/{contracts.rs,ports.rs}`
   - `backend/src/application/neuro_adt/use_cases.rs` refatorado para depender de contratos/ports do contexto
2. Tipos e erro do dominio:
   - `backend/src/domain/neuro_adt/{types.rs,error.rs,server_store.rs}`
   - `server_store.rs` desacoplado de `crate::neuro_runtime`
3. Infra de store desacoplada:
   - `backend/src/infrastructure/filesystem/neuro_server_store.rs` usando tipos/erro do contexto
4. Adapter em `neuro_runtime.rs` preservando API externa dos comandos:
   - `neuro_adt_server_*_impl` sem mudanca de assinatura
   - mapping de erro do connect com preservacao de `NeuroRuntimeErrorCode`
   - lock global de env de testes unificado com `domain/neuro_adt/types`
5. Criterio de desacoplamento validado:
   - `rg -n "crate::neuro_runtime" src/application/neuro_adt src/domain/neuro_adt src/infrastructure/filesystem/neuro_server_store.rs` -> sem ocorrencias

Validacao executada:
1. `cd alicia/backend && cargo fmt --all -- --check` -> OK.
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`148 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. `node alicia/codex-bridge/generators/check-runtime-contract.mjs` -> OK (`runtimeMethods=51`, `tauriCommands=70`, `tauriEventChannels=6`).
6. Teste especifico de regressao de erro connect:
   - `cargo test connect_failure_preserves_runtime_error_code_from_get_or_init_mapping` -> OK.

Revisao tecnica final:
1. Sem findings de severidade alta.
2. Sem findings de severidade media apos os fixes.
3. Riscos residuais documentados como baixo impacto (panic-safety em helper de teste de env e ausencia de teste explicito de serializacao DTO).

Resultado:
1. Slice 4 da Fase 4 concluido sem blocker de merge.
2. Contexto `neuro_adt` (server registry/select/connect) ficou com fronteira clean mais estrita e sem dependencia direta de `neuro_runtime` nas camadas internas.
## Fase 4 Slice 5 Entrega e Evidencias

Metadata:
1. Data: 2026-03-04.
2. Objetivo: iniciar decomposicao de `session_turn_runtime.rs` extraindo o recorte read-only de `thread list/read` e validacao de `review.start` para `application/domain/infrastructure`.
3. Escopo do slice:
   - extracao de `codex_thread_list_impl`
   - extracao de `codex_thread_read_impl`
   - extracao da validacao de input de `codex_review_start_impl` (target/delivery)

Entregas:
1. Camada de aplicacao criada para o contexto:
   - `backend/src/application/session_thread_review/{mod.rs,use_cases.rs}`
2. Camada de dominio criada:
   - `backend/src/domain/session_thread_review/{mod.rs,thread_query.rs,review_policy.rs}`
3. Camada de infraestrutura criada para catalogo de threads:
   - `backend/src/infrastructure/runtime_bridge/session_thread_catalog.rs`
4. Modulos-raiz atualizados:
   - `backend/src/{application,domain}/mod.rs`
   - `backend/src/infrastructure/runtime_bridge/mod.rs`
5. `session_turn_runtime.rs` reduzido para delegacao nos pontos do recorte sem mudanca de assinatura externa.
6. Testes novos adicionados no contexto extraido:
   - `application/session_thread_review/use_cases.rs` (3)
   - `domain/session_thread_review/review_policy.rs` (8)
   - `domain/session_thread_review/thread_query.rs` (6)

Validacao executada:
1. `cd alicia/backend && cargo fmt --all -- --check` -> OK.
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`154 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. `node alicia/codex-bridge/generators/check-runtime-contract.mjs` -> OK (`runtimeMethods=51`, `tauriCommands=70`, `tauriEventChannels=6`).
6. Verificacao de contrato/assinatura: sem drift em comandos/assinaturas Tauri de thread/review.

Revisao tecnica final:
1. Sem findings de severidade alta/media.
2. 1 finding baixo registrado: acoplamento residual do adapter `session_thread_catalog` com helpers de `session_turn_runtime`.

Resultado:
1. Slice 5 da Fase 4 concluido sem blocker de merge.
2. Fase 4 continua com proxima prioridade em reduzir acoplamento residual deste adapter e extrair fluxos mutaveis restantes de Session/Thread/Review.
## Fase 4 Slice 6 Entrega e Evidencias

Metadata:
1. Data: 2026-03-04.
2. Objetivo: remover acoplamento residual entre `session_thread_catalog` e helpers de `session_turn_runtime` via modulo neutro em `infrastructure/runtime_bridge`.
3. Escopo do slice:
   - criacao de shared module para config/mapeamentos de thread list/read
   - migracao de `session_thread_catalog` para dependencia direta do shared module
   - manutencao de wrappers finos em `session_turn_runtime` sem alterar assinatura externa

Entregas:
1. Shared module criado:
   - `backend/src/infrastructure/runtime_bridge/session_thread_shared.rs`
2. Adapter atualizado:
   - `backend/src/infrastructure/runtime_bridge/session_thread_catalog.rs`
   - paginacao refatorada para loader injetado (testavel)
3. Exports atualizados:
   - `backend/src/infrastructure/runtime_bridge/mod.rs`
4. Ajuste de visibilidade:
   - wrappers legados em `session_turn_runtime.rs` reduzidos para privados (sem `pub(crate)`) para diminuir superficie de acoplamento
5. Testes adicionados/expandidos no adapter:
   - paginacao multipagina com stop em cursor repetido
   - filtros (`source/cwd`) e forwarding de model provider
   - fallback de summary sem rollout path (via shared module)

Validacao executada:
1. `cd alicia/backend && cargo fmt --all -- --check` -> OK.
2. `cd alicia/backend && cargo check` -> OK.
3. `cd alicia/backend && cargo test` -> OK (`157 passed; 0 failed`).
4. `cd alicia/backend && cargo clippy --all-targets --all-features -- -D warnings` -> OK.
5. `node alicia/codex-bridge/generators/check-runtime-contract.mjs` -> OK (`runtimeMethods=51`, `tauriCommands=70`, `tauriEventChannels=6`).
6. Verificacao read-only:
   - `session_thread_catalog` sem dependencia direta de `session_turn_runtime`
   - sem mudancas em `backend/src/interface/tauri/commands/session_turn.rs`
   - sem mudancas em `backend/src/interface/tauri/dto/session_turn.rs`

Revisao tecnica final:
1. Sem findings de severidade alta/media.
2. Findings de baixa severidade registrados:
   - gaps de teste adicional para `read_thread` (caminhos rollout/fallback/not found)
   - branch `archived` e mapeamento de erro de producao ainda sem teste dedicado

Resultado:
1. Slice 6 da Fase 4 concluido sem blocker de merge.
2. Acoplamento residual do adapter com `session_turn_runtime` foi removido no caminho principal.
## Current Architecture Snapshot (Key Risks)

1. Frontend com `god component` em `app/page.tsx` concentrando UI + fluxo + regras.
2. Componentes de UI sem import direto de `tauri-bridge` nos painéis priorizados da Fase 2; ainda existe acoplamento residual via adapter concreto em alguns fluxos.
3. `main.rs` agora esta focado em bootstrap/estado/registro de handlers; comandos e DTOs foram movidos para `interface/tauri`.
4. `session_turn_runtime.rs` avancou na decomposicao (thread list/read + shared helpers extraidos), mas ainda concentra fluxos mutaveis; `command_runtime.rs` e `neuro_runtime.rs` seguem com blocos residuais especificos.
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

Fase 3 (slices 1-4):
1. Resultado entregue: extracao de interface Tauri concluida no backend; `main.rs` reduzido para bootstrap/registro sem handlers locais.
2. Incidentes/regressoes:
   - sem blocker no fluxo default (`cargo check`, `cargo test`, `clippy`, contrato e build frontend OK);
   - falha em `cargo check --no-default-features --features custom-protocol` identificada como preexistente e fora do escopo do slice.
3. Riscos residuais:
   - faltam testes de integracao para validar automaticamente binding/registro Tauri no `generate_handler!`;
   - faltam testes dedicados de serializacao para DTOs migrados (ex.: `dto/models.rs`, `dto/session_turn.rs`).
4. Ajuste para Fase 4:
   - iniciar separacao interna por bounded context (`application/domain/infrastructure`) sobre a base ja desacoplada de interface.

Fase 4 (slice 1):
1. Resultado entregue: contexto Workspace Filesystem extraido para `application/domain/infrastructure` com `command_runtime` atuando como fachada.
2. Incidentes/regressoes: findings de seguranca iniciais (symlink dangling write e nao determinismo em `has_children`) foram corrigidos no mesmo slice com testes adicionais.
3. Riscos residuais:
   - TOCTOU de filesystem ainda existe como risco baixo em operacoes de write/create/rename;
   - acoplamento de `application/workspace` com `tauri::State` e DTOs de interface ainda precisa ser reduzido em slices seguintes.
4. Ajuste para continuidade da Fase 4:
   - separar proximo contexto (`Session/Thread/Review` ou `Account/MCP`) com o mesmo padrao de extracao incremental e testes de regressao.

Fase 4 (slice 2):
1. Resultado entregue: contexto Account/MCP/App List extraido para `application/domain/infrastructure`, com `command_runtime` consolidado como fachada para este bounded context.
2. Incidentes/regressoes:
   - sem regressao funcional observada nas validacoes tecnicas (fmt/check/test/clippy, contrato e build frontend);
   - findings de cobertura identificados na revisao foram tratados no proprio slice com testes adicionais.
3. Riscos residuais:
   - ainda existe duplicacao parcial de helper de execucao de comando/app-server entre modulos de runtime;
   - dependencias de `tauri::State` nos use-cases de aplicacao permanecem como divida tecnica para slices seguintes.
4. Ajuste para continuidade da Fase 4:
   - priorizar extracao de `Session/Thread/Review` ou `ADT/Neuro` para reduzir os blocos monoliticos restantes de runtime.

Fase 4 (slice 3):
1. Resultado entregue: recorte ADT/Neuro de server registry/select/connect extraido para `application/domain/infrastructure`, com `neuro_runtime` atuando como fachada dos comandos `server_*`.
2. Incidentes/regressoes:
   - sem regressao funcional detectada nas validacoes tecnicas e no checker de contrato;
   - revisao final sem findings de severidade alta.
3. Riscos residuais:
   - acoplamento de transicao entre camadas extraidas e tipos/erros de `neuro_runtime` ainda presente;
   - gap de testes E2E Tauri e de concorrencia de escrita simultanea no store de servers.
4. Ajuste para continuidade da Fase 4:
   - extrair contratos/ports proprios do contexto ADT/Neuro e seguir para separacao de `Session/Thread/Review`.

Fase 4 (slice 4):
1. Resultado entregue: desacoplamento de `application/domain/infrastructure/neuro_adt` de `neuro_runtime` com contracts/ports/tipos/erro proprios e adapter dedicado no runtime.
2. Incidentes/regressoes:
   - findings medios da rodada anterior (mapping de erro connect e lock global de env) corrigidos no mesmo slice;
   - validacoes finais sem findings alta/media.
3. Riscos residuais:
   - helpers `with_env_overrides` de testes nao sao panic-safe (risco baixo, escopo de testes);
   - falta teste explicito de serializacao dos DTOs extraidos de `neuro_adt`.
4. Ajuste para continuidade da Fase 4:
   - priorizar separacao de `Session/Thread/Review` e/ou blocos ADT/Neuro restantes fora do recorte server registry.

Fase 4 (slice 5):
1. Resultado entregue: recorte read-only de Session/Thread/Review extraido para `application/domain/infrastructure` com `session_turn_runtime` atuando como fachada.
2. Incidentes/regressoes:
   - sem findings de severidade alta/media nas validacoes finais;
   - contrato Tauri preservado sem drift de comandos/assinaturas.
3. Riscos residuais:
   - adapter `session_thread_catalog` ainda depende de helpers de `session_turn_runtime` (acoplamento baixo);
   - faltam testes diretos de adapter para cenarios avancados de paginacao/filtros em rollout real.
4. Ajuste para continuidade da Fase 4:
   - extrair helpers compartilhados para modulo neutro e seguir com fluxos mutaveis de Session/Thread/Review.

