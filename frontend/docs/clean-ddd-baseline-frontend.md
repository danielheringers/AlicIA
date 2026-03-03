# Clean DDD Baseline - Frontend (Fase 0)

Escopo: `alicia/frontend/**`.
Objetivo: baseline do contrato atual entre frontend e runtime bridge, mais smoke checklist funcional.

## 1) Commands `invoke` usados/exportados

Fonte principal: `lib/tauri-bridge/commands.ts` (funcoes exportadas que encapsulam `invoke`) e barrel export em `lib/tauri-bridge.ts`.

Referencias:
- `invoke` e wrappers exportados: `alicia/frontend/lib/tauri-bridge/commands.ts:881-1567`
- barrel export do bridge: `alicia/frontend/lib/tauri-bridge.ts:3-5`

### 1.1 Sessao, turnos, threads, review

| Command (Tauri) | Wrapper exportado (frontend) | Referencia |
|---|---|---|
| `codex_runtime_status` | `codexRuntimeStatus` | `commands.ts:881-883` |
| `codex_runtime_capabilities` | `codexRuntimeCapabilities` | `commands.ts:885-887` |
| `load_codex_default_config` | `loadCodexDefaultConfig` | `commands.ts:1267-1269` |
| `start_codex_session` | `codexRuntimeSessionStart`, `startCodexSession` | `commands.ts:1274-1290` |
| `stop_codex_session` | `codexRuntimeSessionStop`, `stopCodexSession` | `commands.ts:1282-1294` |
| `codex_turn_run` | `codexTurnRun` | `commands.ts:1296-1300` |
| `codex_turn_steer` | `codexTurnSteer` | `commands.ts:1302-1306` |
| `codex_turn_interrupt` | `codexTurnInterrupt` | `commands.ts:1308-1312` |
| `codex_review_start` | `codexReviewStart` | `commands.ts:1313-1317` |
| `codex_thread_open` | `codexThreadOpen` | `commands.ts:1319-1323` |
| `codex_thread_close` | `codexThreadClose` | `commands.ts:1325-1329` |
| `codex_thread_list` | `codexThreadList` | `commands.ts:1331-1335` |
| `codex_thread_read` | `codexThreadRead` | `commands.ts:1337-1341` |
| `codex_thread_archive` | `codexThreadArchive` | `commands.ts:1343-1347` |
| `codex_thread_unarchive` | `codexThreadUnarchive` | `commands.ts:1349-1355` |
| `codex_thread_compact_start` | `codexThreadCompactStart` | `commands.ts:1357-1363` |
| `codex_thread_rollback` | `codexThreadRollback` | `commands.ts:1365-1369` |
| `codex_thread_fork` | `codexThreadFork` | `commands.ts:1371-1375` |
| `codex_approval_respond` | `codexApprovalRespond` | `commands.ts:1377-1381` |
| `codex_user_input_respond` | `codexUserInputRespond` | `commands.ts:1383-1387` |
| `send_codex_input` | `sendCodexInput` | `commands.ts:1389-1391` |

### 1.2 Config, workspace, git

| Command (Tauri) | Wrapper exportado (frontend) | Referencia |
|---|---|---|
| `update_codex_config` | `updateCodexRuntimeConfig` | `commands.ts:1393-1397` |
| `codex_config_get` | `codexConfigGet` | `commands.ts:1399-1401` |
| `codex_config_set` | `codexConfigSet` | `commands.ts:1403-1407` |
| `run_codex_command` | `runCodexCommand` | `commands.ts:1409-1414` |
| `git_workspace_changes` | `codexWorkspaceChanges` | `commands.ts:1417-1419` |
| `codex_workspace_read_file` | `codexWorkspaceReadFile` | `commands.ts:1421-1427` |
| `codex_workspace_write_file` | `codexWorkspaceWriteFile` | `commands.ts:1429-1435` |
| `codex_workspace_create_directory` | `codexWorkspaceCreateDirectory` | `commands.ts:1437-1446` |
| `codex_workspace_rename_entry` | `codexWorkspaceRenameEntry` | `commands.ts:1448-1454` |
| `codex_workspace_list_directory` | `codexWorkspaceListDirectory` | `commands.ts:1456-1462` |
| `git_commit_approved_review` | `gitCommitApprovedReview` | `commands.ts:1464-1470` |

### 1.3 Models, MCP, apps, account

| Command (Tauri) | Wrapper exportado (frontend) | Referencia |
|---|---|---|
| `codex_models_list` | `codexModelsList` | `commands.ts:1472-1474` |
| `codex_wait_for_mcp_startup` | `codexWaitForMcpStartup` | `commands.ts:1476-1478` |
| `codex_mcp_list` | `codexMcpList` | `commands.ts:1480-1482` |
| `codex_app_list` | `codexAppList` | `commands.ts:1484-1488` |
| `codex_account_read` | `codexAccountRead` | `commands.ts:1490-1494` |
| `codex_account_login_start` | `codexAccountLoginStart` | `commands.ts:1496-1500` |
| `codex_account_logout` | `codexAccountLogout` | `commands.ts:1502-1504` |
| `codex_account_rate_limits_read` | `codexAccountRateLimitsRead` | `commands.ts:1506-1508` |
| `codex_mcp_login` | `codexMcpLogin` | `commands.ts:1509-1513` |
| `codex_mcp_reload` | `codexMcpReload` | `commands.ts:1515-1517` |

### 1.4 Terminal e pickers

| Command (Tauri) | Wrapper exportado (frontend) | Referencia |
|---|---|---|
| `terminal_create` | `terminalCreate` | `commands.ts:1519-1523` |
| `terminal_write` | `terminalWrite` | `commands.ts:1525-1532` |
| `terminal_resize` | `terminalResize` | `commands.ts:1534-1542` |
| `terminal_kill` | `terminalKill` | `commands.ts:1544-1548` |
| `pick_image_file` | `pickImageFile` | `commands.ts:1550-1552` |
| `pick_mention_file` | `pickMentionFile` | `commands.ts:1554-1556` |
| `pick_workspace_folder` | `pickWorkspaceFolder` | `commands.ts:1558-1560` |
| `codex_help_snapshot` | `codexHelpSnapshot` | `commands.ts:1562-1564` |
| `resize_codex_pty` | `resizeCodexPty` | `commands.ts:1566-1568` |

### 1.5 Neuro/ADT e neuro ws/tooling

| Command (Tauri) | Wrapper exportado (frontend) | Referencia |
|---|---|---|
| `neuro_runtime_diagnose` | `neuroRuntimeDiagnose` | `commands.ts:889-898` |
| `neuro_search_objects` | `neuroSearchObjects` | `commands.ts:900-919` |
| `neuro_get_source` | `neuroGetSource` | `commands.ts:921-935` |
| `neuro_update_source` | `neuroUpdateSource` | `commands.ts:937-958` |
| `neuro_adt_server_list` | `neuroAdtServerList` | `commands.ts:960-969` |
| `neuro_adt_server_upsert` | `neuroAdtServerUpsert` | `commands.ts:971-998` |
| `neuro_adt_server_remove` | `neuroAdtServerRemove` | `commands.ts:999-1009` |
| `neuro_adt_server_select` | `neuroAdtServerSelect` | `commands.ts:1011-1022` |
| `neuro_adt_server_connect` | `neuroAdtServerConnect` | `commands.ts:1024-1036` |
| `neuro_adt_list_packages` | `neuroAdtListPackages` | `commands.ts:1038-1054` |
| `neuro_adt_list_namespaces` | `neuroAdtListNamespaces` | `commands.ts:1055-1071` |
| `neuro_adt_explorer_state_get` | `neuroAdtExplorerStateGet` | `commands.ts:1073-1085` |
| `neuro_adt_explorer_state_patch` | `neuroAdtExplorerStatePatch` | `commands.ts:1087-1127` |
| `neuro_adt_list_objects` | `neuroAdtListObjects` | `commands.ts:1129-1199` |
| `neuro_adt_list_package_inventory` | `neuroAdtListPackageInventory` | `commands.ts:1201-1221` |
| `neuro_ws_request` | `neuroWsRequest` | `commands.ts:1223-1236` |
| `neuro_list_tools` | `neuroListTools` | `commands.ts:1238-1247` |
| `neuro_invoke_tool` | `neuroInvokeTool` | `commands.ts:1249-1265` |

## 2) Eventos escutados no frontend

### 2.1 Canais Tauri escutados (transport)

Fonte: `alicia/frontend/lib/tauri-bridge/listeners.ts`.

| Canal | Uso | Referencia |
|---|---|---|
| `codex://stdout` | stream de stdout do runtime | `listeners.ts:115-117` |
| `codex://stderr` | stream de stderr do runtime | `listeners.ts:118-120` |
| `codex://lifecycle` | mudancas de lifecycle (started/stopped/error) | `listeners.ts:121-123` |
| `codex://event` | envelope de evento estruturado (timeline) | `listeners.ts:124-126` |
| `terminal://data` | dados de PTY/terminal | `listeners.ts:144-146` |
| `terminal://exit` | saida de PTY/terminal | `listeners.ts:147-149` |

Pontos de assinatura no bootstrap:
- `listenToCodexEvents(...)`: `alicia/frontend/hooks/use-alicia-bootstrap.ts:119`
- `listenToTerminalEvents(...)`: `alicia/frontend/hooks/use-alicia-bootstrap.ts:120`

### 2.2 Tipos de evento de runtime efetivamente tratados

Fonte: `alicia/frontend/lib/alicia-event-handlers.ts`.

| Tipo de evento | Efeito principal no frontend | Referencia |
|---|---|---|
| `lifecycle` | atualiza estado de runtime (`error`/`idle`) e limpa estado pendente | `alicia-event-handlers.ts:90-114` |
| `stdout` | adiciona mensagem de sistema | `alicia-event-handlers.ts:117-120` |
| `stderr` | adiciona mensagem de sistema | `alicia-event-handlers.ts:122-125` |
| `thread.started` | define `threadIdRef`, loga inicio | `alicia-event-handlers.ts:177-184` |
| `turn.started` | ativa thinking e estado de review routing | `alicia-event-handlers.ts:185-196` |
| `turn.completed` | encerra thinking/finaliza turno | `alicia-event-handlers.ts:197-202` |
| `turn.failed` | encerra thinking e reporta erro | `alicia-event-handlers.ts:203-210` |
| `mcp.oauth_login.completed` | feedback de oauth MCP | `alicia-event-handlers.ts:212-234` |
| `thread.token_usage.updated` | mensagem de uso de tokens | `alicia-event-handlers.ts:236-254` |
| `turn.diff.updated` | atualiza diff do turno | `alicia-event-handlers.ts:256-274` |
| `turn.plan.updated` | atualiza plano do turno | `alicia-event-handlers.ts:276-303` |
| `approval.requested` | popula fila de aprovacoes | `alicia-event-handlers.ts:305-344` |
| `approval.resolved` | remove aprovacao da fila | `alicia-event-handlers.ts:346-354` |
| `user_input.requested` | abre solicitacao de input do usuario | `alicia-event-handlers.ts:356-437` |
| `user_input.resolved` | limpa solicitacao pendente/feedback | `alicia-event-handlers.ts:439-466` |
| `item.started` / `item.updated` / `item.completed` | streaming/mensagens estruturadas e transicao de review mode | `alicia-event-handlers.ts:468-527` |
| `terminal data/exit` | atualiza buffers, escrita no xterm e estado `alive` da aba | `alicia-event-handlers.ts:547-563` |

## 3) Runtime methods (contrato de capabilities)

Fonte de verdade de contrato de capabilities:
- `alicia/frontend/lib/tauri-bridge/types.ts:43-95` (`RUNTIME_METHODS`)
- tipo de resposta de capabilities: `types.ts:105-109`

Lista atual do contrato:
- `thread.open`, `thread.close`, `thread.list`, `thread.read`, `thread.archive`, `thread.unarchive`, `thread.compact.start`, `thread.rollback`, `thread.fork`
- `turn.run`, `review.start`, `turn.steer`, `turn.interrupt`
- `approval.respond`, `user_input.respond`, `tool.call.dynamic`
- `mcp.warmup`, `mcp.list`, `mcp.login`, `mcp.reload`
- `app.list`, `account.read`, `account.login.start`, `account.logout`, `account.rate_limits.read`, `account.rateLimits.read`
- `config.get`, `config.set`
- `workspace.file.read`, `workspace.file.write`, `workspace.directory.list`, `workspace.directory.create`, `workspace.entry.rename`
- `neuro.runtime.diagnose`, `neuro.search.objects`, `neuro.get.source`, `neuro.update.source`
- `neuro.adt.server.list`, `neuro.adt.server.upsert`, `neuro.adt.server.remove`, `neuro.adt.server.select`, `neuro.adt.server.connect`
- `neuro.adt.list.packages`, `neuro.adt.list.namespaces`, `neuro.adt.explorer.state.get`, `neuro.adt.explorer.state.patch`, `neuro.adt.list.objects`, `neuro.adt.list.package_inventory`
- `neuro.ws.request`, `neuro.mcp.list_tools`, `neuro.mcp.invoke`

Guard rails principais no frontend:
- normalizacao e fallback de capabilities: `alicia/frontend/lib/alicia-runtime-helpers.ts:93-137`
- checks de suporte em runtime core/actions/page:
  - `use-alicia-runtime-core.ts:147-170,180-217,230-270,288-368`
  - `use-alicia-actions.ts:329-341,704-756,801-935,984-1001,1130-1189`
  - `app/page.tsx:783-797,1447-1457,2130-2150,2235-2241,2261-2263,2653-2668,2796-2811`

## 4) Smoke checklist funcional atual (frontend)

Checklist focado em fluxos-chave e ponto de disparo no frontend.

| Fluxo de smoke | Como disparar (frontend) | Contrato bridge/metodos envolvidos | Referencia |
|---|---|---|---|
| Bootstrap runtime + listeners | App init (mount da `page`) | `listenToCodexEvents`, `listenToTerminalEvents`, `codexRuntimeStatus`, `codexRuntimeCapabilities`, `loadCodexDefaultConfig` | `app/page.tsx:1101-1137`, `use-alicia-bootstrap.ts:98-161` |
| Garantir/iniciar sessao | Submit no chat ou `/new` | `codexRuntimeSessionStart/Stop`; metodo `thread.list` para sincronia posterior | `use-alicia-actions.ts:527-533,649-653`, `use-alicia-runtime-core.ts:486-546` |
| Enviar turn de chat | Enter no input principal (`onSubmit`) | `codexTurnRun` (`turn.run`) | `app/page.tsx:3138`, `use-alicia-actions.ts:525-533` |
| Slash `/review` e `/review-file` | Command input (`onSlashCommand`) | `codexReviewStart`; metodo `review.start` | `app/page.tsx:3139`, `use-alicia-actions.ts:681-756` |
| Aprovar/rejeitar approval request | UI de approval (`onApprovalDecision`) | `codexApprovalRespond` (`approval.respond`) | `app/page.tsx:1260-1267` |
| Responder user input request | UI de user input (`onUserInputDecision`) | `codexUserInputRespond` (`user_input.respond`) | `app/page.tsx:1271-1291` |
| Explorar arquivos do workspace | Sidebar/File Explorer | `codexWorkspaceListDirectory`; metodo `workspace.directory.list` | `app/page.tsx:775-863,3388-3406` |
| Abrir/editar/salvar arquivo workspace | Abrir ref no editor + save | `codexWorkspaceReadFile`, `codexWorkspaceWriteFile`; metodos `workspace.file.read/write` | `app/page.tsx:2260-2335,2638-2766,2776-2845` |
| Criar pasta/arquivo e renomear no workspace | Acoes no explorer/editor | `codexWorkspaceCreateDirectory`, `codexWorkspaceWriteFile`, `codexWorkspaceRenameEntry`; metodos `workspace.directory.create`, `workspace.file.write`, `workspace.entry.rename` | `app/page.tsx:2457-2636` |
| Review commit de arquivos aprovados | Acao em painel review | `gitCommitApprovedReview` | `app/page.tsx:1295-1386` |
| Selecionar pasta de workspace | Acao de trocar workspace | `pickWorkspaceFolder` + restart sessao | `app/page.tsx:3025-3104` |
| Terminal criar/escrever/redimensionar/fechar | Nova aba terminal, input e resize | `terminalCreate`, `terminalWrite`, `terminalResize`, `terminalKill` | `use-alicia-runtime-core.ts:558-613`, `app/page.tsx:1225-1258`, `use-alicia-terminal-runtime.ts:86-103` |
| MCP list/login/reload | Painel MCP (`/mcp`) | `codexMcpList`, `codexMcpLogin`, `codexMcpReload` (com fallback CLI em alguns casos) | `use-alicia-runtime-core.ts:230-270`, `components/alicia/mcp-panel.tsx:344-416` |
| Apps/Auth login/logout | Painel Apps (`/apps`) | `codexAppList`, `codexAccountRead`, `codexAccountLoginStart`, `codexAccountLogout`, `codexAccountRateLimitsRead` | `use-alicia-runtime-core.ts:273-404`, `components/alicia/apps-panel.tsx:165-193` |
| ADT server + busca + source ABAP | Painel ADT / SAP Explorer / abrir ref ABAP | `neuro_adt_*`, `neuro_search_objects`, `neuro_get_source`, `neuro_update_source` | `components/alicia/adt-panel.tsx:123-393`, `app/page.tsx:1447-1495,2235-2241,2667-2733,2810-2817` |

## 5) Contrato esperado com backend e risco de compatibilidade

- O frontend depende de dois contratos simultaneos:
  - comandos Tauri (`invoke`) com nomes exatos em snake_case;
  - payloads/eventos estruturados (especialmente `codex://event` com `type` e campos esperados).
- Se backend remover/renomear comandos ou campos sem atualizar `RuntimeCapabilitiesResponse.methods`, o frontend entra em fallback parcial e/ou exibe erro de recurso nao suportado.
- Pontos mais sensiveis:
  - alias de rate limits (`account.rate_limits.read` e `account.rateLimits.read`): `alicia-runtime-helpers.ts:115-128`;
  - envelope de evento estruturado e validacao de contrato: `alicia-event-handlers.ts:146-176`;
  - compatibilidade ADT em `neuro_adt_list_objects` com fallback legacy payload: `commands.ts:1150-1197`.
