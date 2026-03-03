# Clean DDD Baseline - Alicia Backend

Data de levantamento: 2026-03-03
Escopo: somente `alicia/backend/**`
Objetivo: baseline de contrato backend sem alteracao de comportamento.

## 1) Commands Tauri registrados

Fonte primaria de registro: `tauri::generate_handler![...]` em `src/main.rs:1575` ate `src/main.rs:1645`.

### 1.1 Sessao e turno Codex
- `start_codex_session` (`src/main.rs:1576`, handler `src/main.rs:1332`, impl `src/session_lifecycle_runtime.rs:128`)
- `stop_codex_session` (`src/main.rs:1618`, handler `src/main.rs:1346`, impl `src/session_lifecycle_runtime.rs:149`)
- `resize_codex_pty` (`src/main.rs:1619`, handler `src/main.rs:1341`, impl `src/session_lifecycle_runtime.rs:136`)
- `send_codex_input` (`src/main.rs:1617`, handler `src/main.rs:1323`, impl `src/session_turn_runtime.rs:2988`)
- `codex_turn_run` (`src/main.rs:1577`, handler `src/main.rs:1198`, impl `src/session_turn_runtime.rs:1527`)
- `codex_review_start` (`src/main.rs:1587`, handler `src/main.rs:1296`, impl `src/session_turn_runtime.rs:1680`)
- `codex_turn_steer` (`src/main.rs:1588`, handler `src/main.rs:1280`, impl `src/session_turn_runtime.rs:2671`)
- `codex_turn_interrupt` (`src/main.rs:1589`, handler `src/main.rs:1288`, impl `src/session_turn_runtime.rs:2703`)
- `codex_approval_respond` (`src/main.rs:1590`, handler `src/main.rs:1305`, impl `src/session_turn_runtime.rs:2745`)
- `codex_user_input_respond` (`src/main.rs:1591`, handler `src/main.rs:1314`, impl `src/session_turn_runtime.rs:2856`)

### 1.2 Threads Codex
- `codex_thread_open` (`src/main.rs:1578`, handler `src/main.rs:1207`, impl `src/session_turn_runtime.rs:1697`)
- `codex_thread_close` (`src/main.rs:1579`, handler `src/main.rs:1216`, impl `src/session_turn_runtime.rs:1748`)
- `codex_thread_list` (`src/main.rs:1580`, handler `src/main.rs:1224`, impl `src/session_turn_runtime.rs:1825`)
- `codex_thread_read` (`src/main.rs:1581`, handler `src/main.rs:1232`, impl `src/session_turn_runtime.rs:1954`)
- `codex_thread_archive` (`src/main.rs:1582`, handler `src/main.rs:1240`, impl `src/session_turn_runtime.rs:2032`)
- `codex_thread_unarchive` (`src/main.rs:1583`, handler `src/main.rs:1248`, impl `src/session_turn_runtime.rs:2190`)
- `codex_thread_compact_start` (`src/main.rs:1584`, handler `src/main.rs:1256`, impl `src/session_turn_runtime.rs:2298`)
- `codex_thread_rollback` (`src/main.rs:1585`, handler `src/main.rs:1264`, impl `src/session_turn_runtime.rs:2320`)
- `codex_thread_fork` (`src/main.rs:1586`, handler `src/main.rs:1272`, impl `src/session_turn_runtime.rs:2404`)

### 1.3 Runtime/status/config
- `codex_runtime_status` (`src/main.rs:1595`, handler `src/main.rs:875`)
- `codex_runtime_capabilities` (`src/main.rs:1596`, handler `src/main.rs:901`, impl `src/command_runtime.rs:1713`)
- `codex_native_runtime_diagnose` (`src/main.rs:1597`, handler `src/main.rs:908`)
- `load_codex_default_config` (`src/main.rs:1616`, handler `src/main.rs:1163`)
- `update_codex_config` (`src/main.rs:1592`, handler `src/main.rs:1173`)
- `codex_config_get` (`src/main.rs:1593`, handler `src/main.rs:1183`)
- `codex_config_set` (`src/main.rs:1594`, handler `src/main.rs:1188`)

### 1.4 Terminal e comandos de sistema
- `terminal_create` (`src/main.rs:1620`, handler `src/main.rs:1351`, impl `src/terminal_runtime.rs:50`)
- `terminal_write` (`src/main.rs:1621`, handler `src/main.rs:1360`, impl `src/terminal_runtime.rs:169`)
- `terminal_resize` (`src/main.rs:1622`, handler `src/main.rs:1365`, impl `src/terminal_runtime.rs:193`)
- `terminal_kill` (`src/main.rs:1623`, handler `src/main.rs:1373`, impl `src/terminal_runtime.rs:215`)
- `run_codex_command` (`src/main.rs:1624`, handler `src/main.rs:1382`, impl `src/command_runtime.rs:401`)
- `git_commit_approved_review` (`src/main.rs:1625`, handler `src/main.rs:1390`, impl `src/command_runtime.rs:1182`)
- `git_workspace_changes` (`src/main.rs:1626`, handler `src/main.rs:1411`, impl `src/command_runtime.rs:1497`)

### 1.5 Workspace/files
- `codex_workspace_read_file` (`src/main.rs:1627`, handler `src/main.rs:1426`, impl `src/command_runtime.rs:1515`)
- `codex_workspace_write_file` (`src/main.rs:1628`, handler `src/main.rs:1434`, impl `src/command_runtime.rs:1569`)
- `codex_workspace_create_directory` (`src/main.rs:1629`, handler `src/main.rs:1442`, impl `src/command_runtime.rs:1630`)
- `codex_workspace_list_directory` (`src/main.rs:1630`, handler `src/main.rs:1450`, impl `src/command_runtime.rs:1648`)
- `codex_workspace_rename_entry` (`src/main.rs:1631`, handler `src/main.rs:1458`, impl `src/command_runtime.rs:1664`)
- `pick_workspace_folder` (`src/main.rs:1642`, handler `src/main.rs:1537`, impl `src/command_runtime.rs:1688`)
- `pick_image_file` (`src/main.rs:1643`, handler `src/main.rs:1542`)
- `pick_mention_file` (`src/main.rs:1644`, handler `src/main.rs:1553`)

### 1.6 Models/app/account/MCP
- `codex_models_list` (`src/main.rs:1632`, handler `src/main.rs:1465`, impl `src/command_runtime.rs:1694`)
- `codex_app_list` (`src/main.rs:1633`, handler `src/main.rs:1475`, impl `src/command_runtime.rs:1766`)
- `codex_account_read` (`src/main.rs:1634`, handler `src/main.rs:1492`, impl `src/command_runtime.rs:1844`)
- `codex_account_login_start` (`src/main.rs:1635`, handler `src/main.rs:1500`, impl `src/command_runtime.rs:1885`)
- `codex_account_logout` (`src/main.rs:1636`, handler `src/main.rs:1508`, impl `src/command_runtime.rs:1977`)
- `codex_account_rate_limits_read` (`src/main.rs:1637`, handler `src/main.rs:1513`, impl `src/command_runtime.rs:2015`)
- `codex_mcp_list` (`src/main.rs:1638`, handler `src/main.rs:1519`, impl `src/command_runtime.rs:2057`)
- `codex_mcp_login` (`src/main.rs:1639`, handler `src/main.rs:1524`, impl `src/command_runtime.rs:2075`)
- `codex_mcp_reload` (`src/main.rs:1640`, handler `src/main.rs:1532`, impl `src/command_runtime.rs:2153`)
- `codex_wait_for_mcp_startup` (`src/main.rs:1641`, handler `src/main.rs:1469`, impl `src/command_runtime.rs:1735`)

### 1.7 Neuro runtime
- `neuro_runtime_diagnose` (`src/main.rs:1598`, handler `src/main.rs:915`, impl `src/neuro_runtime.rs:1760`)
- `neuro_search_objects` (`src/main.rs:1599`, handler `src/main.rs:927`, impl `src/neuro_runtime.rs:2277`)
- `neuro_get_source` (`src/main.rs:1600`, handler `src/main.rs:944`, impl `src/neuro_runtime.rs:2298`)
- `neuro_update_source` (`src/main.rs:1601`, handler `src/main.rs:958`, impl `src/neuro_runtime.rs:2318`)
- `neuro_adt_server_list` (`src/main.rs:1602`, handler `src/main.rs:971`, impl `src/neuro_runtime.rs:2334`)
- `neuro_adt_server_upsert` (`src/main.rs:1603`, handler `src/main.rs:983`, impl `src/neuro_runtime.rs:2345`)
- `neuro_adt_server_remove` (`src/main.rs:1604`, handler `src/main.rs:996`, impl `src/neuro_runtime.rs:2359`)
- `neuro_adt_server_select` (`src/main.rs:1605`, handler `src/main.rs:1009`, impl `src/neuro_runtime.rs:2382`)
- `neuro_adt_server_connect` (`src/main.rs:1606`, handler `src/main.rs:1022`, impl `src/neuro_runtime.rs:2399`)
- `neuro_adt_list_packages` (`src/main.rs:1607`, handler `src/main.rs:1035`, impl `src/neuro_runtime.rs:2440`)
- `neuro_adt_list_namespaces` (`src/main.rs:1608`, handler `src/main.rs:1048`, impl `src/neuro_runtime.rs:2481`)
- `neuro_adt_explorer_state_get` (`src/main.rs:1609`, handler `src/main.rs:1064`, impl `src/neuro_runtime.rs:3458`)
- `neuro_adt_explorer_state_patch` (`src/main.rs:1610`, handler `src/main.rs:1077`, impl `src/neuro_runtime.rs:3476`)
- `neuro_adt_list_objects` (`src/main.rs:1611`, handler `src/main.rs:1095`, impl `src/neuro_runtime.rs:3506`)
- `neuro_adt_list_package_inventory` (`src/main.rs:1612`, handler `src/main.rs:1108`, impl `src/neuro_runtime.rs:2530`)
- `neuro_ws_request` (`src/main.rs:1613`, handler `src/main.rs:1121`, impl `src/neuro_runtime.rs:3597`)
- `neuro_list_tools` (`src/main.rs:1614`, handler `src/main.rs:1137`, impl `src/neuro_runtime.rs:3616`)
- `neuro_invoke_tool` (`src/main.rs:1615`, handler `src/main.rs:1149`, impl `src/neuro_runtime.rs:3626`)

### 1.8 Auxiliar
- `codex_help_snapshot` (`src/main.rs:1645`, handler `src/main.rs:1560`)

## 2) Eventos emitidos

### 2.1 Canais Tauri (event names)
Definidos em `src/events_runtime.rs`:
- `codex://lifecycle` (`src/events_runtime.rs:63`)
- `codex://stdout` (via `emit_stream`, `src/events_runtime.rs:71`)
- `codex://stderr` (via `emit_stream`, `src/events_runtime.rs:75`)
- `codex://event` (`src/events_runtime.rs:91`)
- `terminal://data` (`src/events_runtime.rs:106`)
- `terminal://exit` (`src/events_runtime.rs:121`)

Status de lifecycle atualmente emitidos:
- `started` (`src/session_lifecycle_runtime.rs:60`)
- `stopped` (`src/session_lifecycle_runtime.rs:207`)
- `error` (`src/session_turn_runtime.rs:1504`, `src/session_turn_runtime.rs:1657`)

### 2.2 Tipos de payload em `codex://event`

Tipos emitidos diretamente por fluxo de sessao/turno:
- `thread.started` (`src/session_turn_runtime.rs:1431`)
- `approval.resolved` (`src/session_turn_runtime.rs:2845`)
- `user_input.resolved` (`src/session_turn_runtime.rs:772`)

Tipos produzidos pelo tradutor nativo (`src/codex_event_translator.rs`):
- Top-level: `turn.started` (`src/codex_event_translator.rs:221`), `turn.failed` (`src/codex_event_translator.rs:238`), `turn.completed` (`src/codex_event_translator.rs:245`), `thread.token_usage.updated` (`src/codex_event_translator.rs:280`), `turn.diff.updated` (`src/codex_event_translator.rs:288`), `turn.plan.updated` (`src/codex_event_translator.rs:294`), `item.updated` (`src/codex_event_translator.rs:317`), `approval.requested` (`src/codex_event_translator.rs:387`), `user_input.requested` (`src/codex_event_translator.rs:469`), `item.started` (`src/codex_event_translator.rs:916`), `item.completed` (`src/codex_event_translator.rs:928`)
- `item.type`: `command_execution` (`src/codex_event_translator.rs:938`), `mcp_tool_call` (`src/codex_event_translator.rs:977`), `file_change` (`src/codex_event_translator.rs:994`), `collab_tool_call` (`src/codex_event_translator.rs:1150`), `entered_review_mode`/`exited_review_mode` (mapeados em `src/codex_event_translator.rs:1210` e `src/codex_event_translator.rs:1219`)
- Conteudo/mensagens normalizadas: `agent_message` (`src/codex_event_translator.rs:319`), `reasoning` (`src/codex_event_translator.rs:1317`), `web_search` (`src/codex_event_translator.rs:1338`), `user_message` (`src/codex_event_translator.rs:1344`), `context_compaction` (`src/codex_event_translator.rs:1349`)
- Subtipos de mudanca de arquivo: `add` (`src/codex_event_translator.rs:1273`), `delete` (`src/codex_event_translator.rs:1274`), `update` (`src/codex_event_translator.rs:1276`)

Nota: `neuro_runtime` tambem escreve telemetria em stderr (`[neuro-telemetry]`), fora do barramento Tauri (`src/neuro_runtime.rs:620`, `src/neuro_runtime.rs:624`).

## 3) Runtime method keys/capabilities

Contrato exposto por `codex_runtime_capabilities`:
- Struct de resposta: `RuntimeCapabilitiesResponse { methods, contract }` (`src/main.rs:267`)
- Implementacao: `codex_runtime_capabilities_impl` (`src/command_runtime.rs:1713`)
- Versao de contrato: `alicia.runtime.capabilities.v2` (`src/command_runtime.rs:287`)

### 3.1 Method keys atuais
Fonte: `RUNTIME_METHOD_KEYS` (`src/command_runtime.rs:289` a `src/command_runtime.rs:340`)

`thread.open`, `thread.close`, `thread.list`, `thread.read`, `thread.archive`, `thread.unarchive`, `thread.compact.start`, `thread.rollback`, `thread.fork`, `turn.run`, `review.start`, `turn.steer`, `turn.interrupt`, `approval.respond`, `user_input.respond`, `tool.call.dynamic`, `mcp.warmup`, `mcp.list`, `mcp.login`, `mcp.reload`, `app.list`, `account.read`, `account.login.start`, `account.logout`, `account.rate_limits.read`, `account.rateLimits.read`, `config.get`, `config.set`, `workspace.file.read`, `workspace.file.write`, `workspace.directory.create`, `workspace.directory.list`, `workspace.entry.rename`, `neuro.runtime.diagnose`, `neuro.search.objects`, `neuro.get.source`, `neuro.update.source`, `neuro.adt.server.list`, `neuro.adt.server.upsert`, `neuro.adt.server.remove`, `neuro.adt.server.select`, `neuro.adt.server.connect`, `neuro.adt.list.packages`, `neuro.adt.list.namespaces`, `neuro.adt.explorer.state.get`, `neuro.adt.explorer.state.patch`, `neuro.adt.list.objects`, `neuro.adt.list.package_inventory`, `neuro.ws.request`, `neuro.mcp.list_tools`, `neuro.mcp.invoke`.

### 3.2 Regras de habilitacao
- Default: todos `true`, exceto `tool.call.dynamic=false` (`src/command_runtime.rs:343`, `src/command_runtime.rs:348`)
- Transporte nativo: reforca `tool.call.dynamic=false` via `disable_methods_for_native_transport` (`src/command_runtime.rs:279` a `src/command_runtime.rs:283`)

### 3.3 Tauri capability (desktop permission)
- Arquivo: `capabilities/default.json`
- Identificador: `default`
- Permissao ativa: `core:default` (`capabilities/default.json:6`)

## 4) Smoke checklist funcional (baseline atual)

Checklist proposto para validar regressao funcional minima do backend atual (sem mudar comportamento):

1. Sessao sobe/desce corretamente
- Comandos: `start_codex_session`, `codex_runtime_status`, `stop_codex_session`
- Esperado: lifecycle `started` e `stopped` em `codex://lifecycle`
- Referencias: `src/session_lifecycle_runtime.rs:60`, `src/session_lifecycle_runtime.rs:205`, `src/main.rs:1576`, `src/main.rs:1618`

2. Turno nativo emite stream estruturado
- Comandos: `codex_turn_run` (ou `send_codex_input`)
- Esperado: eventos em `codex://event` incluindo `turn.started` e encerramento (`turn.completed` ou `turn.failed`)
- Referencias: `src/session_turn_runtime.rs:1527`, `src/codex_event_translator.rs:221`, `src/codex_event_translator.rs:245`, `src/codex_event_translator.rs:238`

3. Fluxo de aprovacao e input do usuario fecha pendencias
- Comandos: `codex_approval_respond`, `codex_user_input_respond`
- Esperado: emissao de `approval.resolved` e `user_input.resolved`
- Referencias: `src/session_turn_runtime.rs:2745`, `src/session_turn_runtime.rs:2845`, `src/session_turn_runtime.rs:2856`, `src/session_turn_runtime.rs:772`

4. Operacoes de thread continuam funcionais
- Comandos: `codex_thread_open/list/read/archive/unarchive/compact_start/rollback/fork/close`
- Esperado: ciclo CRUD + historico sem quebrar contrato de comandos
- Referencias: `src/session_turn_runtime.rs:1697`, `src/session_turn_runtime.rs:1825`, `src/session_turn_runtime.rs:1954`, `src/session_turn_runtime.rs:2032`, `src/session_turn_runtime.rs:2190`, `src/session_turn_runtime.rs:2298`, `src/session_turn_runtime.rs:2320`, `src/session_turn_runtime.rs:2404`, `src/session_turn_runtime.rs:1748`

5. Terminal externo segue emitindo dados e encerramento
- Comandos: `terminal_create`, `terminal_write`, `terminal_resize`, `terminal_kill`
- Esperado: `terminal://data` e `terminal://exit`
- Referencias: `src/terminal_runtime.rs:50`, `src/terminal_runtime.rs:169`, `src/terminal_runtime.rs:193`, `src/terminal_runtime.rs:215`, `src/events_runtime.rs:106`, `src/events_runtime.rs:121`

6. Workspace/Git continuam protegidos por raiz de workspace
- Comandos: `codex_workspace_*`, `git_workspace_changes`, `git_commit_approved_review`
- Esperado: leitura/escrita/listagem/rename dentro de workspace + parsing de status git
- Referencias: `src/command_runtime.rs:1497`, `src/command_runtime.rs:1515`, `src/command_runtime.rs:1569`, `src/command_runtime.rs:1630`, `src/command_runtime.rs:1648`, `src/command_runtime.rs:1664`, `src/command_runtime.rs:1182`

7. MCP/account/app/models respondem ao contrato atual
- Comandos: `codex_models_list`, `codex_app_list`, `codex_account_*`, `codex_mcp_*`, `codex_wait_for_mcp_startup`
- Esperado: payloads sem quebra de schema e capabilities coerentes
- Referencias: `src/command_runtime.rs:1694`, `src/command_runtime.rs:1766`, `src/command_runtime.rs:1844`, `src/command_runtime.rs:1885`, `src/command_runtime.rs:1977`, `src/command_runtime.rs:2015`, `src/command_runtime.rs:2057`, `src/command_runtime.rs:2075`, `src/command_runtime.rs:2153`, `src/command_runtime.rs:1735`

8. Neuro runtime permanece operacional no baseline
- Comandos: `neuro_runtime_diagnose` + principais comandos ADT/ws/tools
- Esperado: respostas sem quebrar contrato e telemetria `neuro.command` em stderr
- Referencias: `src/neuro_runtime.rs:1760`, `src/neuro_runtime.rs:2277`, `src/neuro_runtime.rs:2298`, `src/neuro_runtime.rs:2318`, `src/neuro_runtime.rs:2334`, `src/neuro_runtime.rs:3597`, `src/neuro_runtime.rs:3616`, `src/neuro_runtime.rs:3626`, `src/neuro_runtime.rs:30`, `src/neuro_runtime.rs:620`

## 5) Observacoes de risco/rollback

Nao houve alteracao de comportamento nem de codigo Rust; somente documentacao de baseline.
Rollback tecnico: remover este arquivo de docs se necessario.
