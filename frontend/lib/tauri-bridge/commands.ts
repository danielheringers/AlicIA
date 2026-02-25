import { invoke } from '@tauri-apps/api/core'

import type {
  CodexApprovalRespondRequest,
  CodexUserInputRespondRequest,
  CodexUserInputRespondResponse,
  CodexHelpSnapshot,
  CodexModelListResponse,
  CodexThreadArchiveRequest,
  CodexThreadArchiveResponse,
  CodexThreadCompactStartRequest,
  CodexThreadCompactStartResponse,
  CodexThreadForkRequest,
  CodexThreadForkResponse,
  CodexThreadCloseRequest,
  CodexThreadCloseResponse,
  CodexThreadListRequest,
  CodexThreadListResponse,
  CodexThreadOpenResponse,
  CodexThreadReadRequest,
  CodexThreadReadResponse,
  CodexThreadRollbackRequest,
  CodexThreadRollbackResponse,
  CodexThreadUnarchiveRequest,
  CodexThreadUnarchiveResponse,
  CodexTurnInterruptRequest,
  CodexTurnInterruptResponse,
  CodexReviewStartRequest,
  CodexReviewStartResponse,
  CodexTurnRunRequest,
  CodexTurnRunResponse,
  CodexTurnSteerRequest,
  CodexTurnSteerResponse,
  McpLoginRequest,
  McpLoginResponse,
  McpReloadResponse,
  McpServerListResponse,
  McpStartupWarmupResponse,
  AppListRequest,
  AppListResponse,
  AccountReadRequest,
  AccountReadResponse,
  AccountLoginStartRequest,
  AccountLoginStartResponse,
  AccountLogoutResponse,
  AccountRateLimitsReadResponse,
  RunCodexCommandResponse,
  WorkspaceChangesResponse,
  GitCommitApprovedReviewRequest,
  GitCommitApprovedReviewResponse,
  RuntimeCodexConfig,
  RuntimeStatusResponse,
  RuntimeCapabilitiesResponse,
  NeuroAdtObjectSummary,
  NeuroAdtSourceResponse,
  NeuroAdtUpdateSourceRequest,
  NeuroAdtUpdateSourceResponse,
  NeuroRuntimeCommandError,
  NeuroRuntimeDiagnoseResponse,
  NeuroWsDomainRequest,
  NeuroWsMessageEnvelope,
  StartCodexSessionConfig,
  StartCodexSessionResponse,
  TerminalCreateRequest,
  TerminalCreateResponse,
} from '@/lib/tauri-bridge/types'

type RawNeuroDiagnoseStatus = 'healthy' | 'degraded' | 'unavailable'

interface RawNeuroRuntimeDiagnoseComponent {
  component: string
  status: RawNeuroDiagnoseStatus
  detail: string
  latency_ms?: number | null
}

interface RawNeuroRuntimeDiagnoseResponse {
  timestamp_epoch_secs: number
  overall_status: RawNeuroDiagnoseStatus
  components: RawNeuroRuntimeDiagnoseComponent[]
  metadata: Record<string, unknown>
}

interface RawNeuroAdtObjectSummary {
  uri: string
  name: string
  object_type?: string | null
  package?: string | null
}

interface RawNeuroAdtSourceResponse {
  object_uri: string
  source: string
  etag?: string | null
}

interface RawNeuroAdtUpdateSourceRequest {
  object_uri: string
  source: string
  etag?: string | null
}

interface RawNeuroAdtUpdateSourceResponse {
  object_uri: string
  status_code: number
  etag?: string | null
}

interface RawNeuroWsMessageEnvelope {
  id: string
  domain: string
  action: string
  payload: Record<string, unknown>
  ok?: boolean | null
  error?: string | null
}

interface RawNeuroRuntimeCommandError {
  code: string
  message: string
  details?: Record<string, unknown> | null
}

function mapNeuroDiagnoseResponse(
  raw: RawNeuroRuntimeDiagnoseResponse,
): NeuroRuntimeDiagnoseResponse {
  return {
    timestampEpochSecs: raw.timestamp_epoch_secs,
    overallStatus: raw.overall_status,
    components: raw.components.map(component => ({
      component: component.component,
      status: component.status,
      detail: component.detail,
      latencyMs: component.latency_ms,
    })),
    metadata: raw.metadata ?? {},
  }
}

function mapNeuroObjectSummary(raw: RawNeuroAdtObjectSummary): NeuroAdtObjectSummary {
  return {
    uri: raw.uri,
    name: raw.name,
    objectType: raw.object_type,
    package: raw.package,
  }
}

function mapNeuroSourceResponse(
  raw: RawNeuroAdtSourceResponse,
): NeuroAdtSourceResponse {
  return {
    objectUri: raw.object_uri,
    source: raw.source,
    etag: raw.etag,
  }
}

function mapNeuroUpdateSourceResponse(
  raw: RawNeuroAdtUpdateSourceResponse,
): NeuroAdtUpdateSourceResponse {
  return {
    objectUri: raw.object_uri,
    statusCode: raw.status_code,
    etag: raw.etag,
  }
}

function mapNeuroWsMessageEnvelope(raw: RawNeuroWsMessageEnvelope): NeuroWsMessageEnvelope {
  return {
    id: raw.id,
    domain: raw.domain,
    action: raw.action,
    payload: raw.payload,
    ok: raw.ok,
    error: raw.error,
  }
}

function parseJsonErrorMessage(message: string): RawNeuroRuntimeCommandError | null {
  try {
    const parsed = JSON.parse(message) as RawNeuroRuntimeCommandError
    if (parsed && typeof parsed.code === 'string' && typeof parsed.message === 'string') {
      return parsed
    }
    return null
  } catch {
    return null
  }
}

function normalizeNeuroRuntimeError(error: unknown): NeuroRuntimeCommandError {
  const unknownFallback: NeuroRuntimeCommandError = {
    code: 'unknown',
    message: error instanceof Error ? error.message : String(error),
    details: null,
  }

  if (error && typeof error === 'object') {
    const candidate = error as Partial<RawNeuroRuntimeCommandError> & {
      message?: unknown
    }

    if (typeof candidate.code === 'string' && typeof candidate.message === 'string') {
      return {
        code: candidate.code as NeuroRuntimeCommandError['code'],
        message: candidate.message,
        details: candidate.details ?? null,
      }
    }

    if (typeof candidate.message === 'string') {
      const parsed = parseJsonErrorMessage(candidate.message)
      if (parsed) {
        return {
          code: parsed.code as NeuroRuntimeCommandError['code'],
          message: parsed.message,
          details: parsed.details ?? null,
        }
      }
      return {
        code: 'unknown',
        message: candidate.message,
        details: null,
      }
    }
  }

  if (typeof error === 'string') {
    const parsed = parseJsonErrorMessage(error)
    if (parsed) {
      return {
        code: parsed.code as NeuroRuntimeCommandError['code'],
        message: parsed.message,
        details: parsed.details ?? null,
      }
    }
    return { code: 'unknown', message: error, details: null }
  }

  return unknownFallback
}

export async function codexRuntimeStatus(): Promise<RuntimeStatusResponse> {
  return invoke<RuntimeStatusResponse>('codex_runtime_status')
}

export async function codexRuntimeCapabilities(): Promise<RuntimeCapabilitiesResponse> {
  return invoke<RuntimeCapabilitiesResponse>('codex_runtime_capabilities')
}

export async function neuroRuntimeDiagnose(): Promise<NeuroRuntimeDiagnoseResponse> {
  try {
    const raw = await invoke<RawNeuroRuntimeDiagnoseResponse>('neuro_runtime_diagnose')
    return mapNeuroDiagnoseResponse(raw)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroSearchObjects(
  query: string,
  maxResults?: number,
): Promise<NeuroAdtObjectSummary[]> {
  try {
    const raw = await invoke<RawNeuroAdtObjectSummary[]>('neuro_search_objects', {
      query,
      maxResults,
    })
    return raw.map(mapNeuroObjectSummary)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroGetSource(objectUri: string): Promise<NeuroAdtSourceResponse> {
  try {
    const raw = await invoke<RawNeuroAdtSourceResponse>('neuro_get_source', { objectUri })
    return mapNeuroSourceResponse(raw)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroUpdateSource(
  request: NeuroAdtUpdateSourceRequest,
): Promise<NeuroAdtUpdateSourceResponse> {
  try {
    const payload: RawNeuroAdtUpdateSourceRequest = {
      object_uri: request.objectUri,
      source: request.source,
      etag: request.etag ?? null,
    }
    const raw = await invoke<RawNeuroAdtUpdateSourceResponse>('neuro_update_source', {
      request: payload,
    })
    return mapNeuroUpdateSourceResponse(raw)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroWsRequest(
  request: NeuroWsDomainRequest,
): Promise<NeuroWsMessageEnvelope> {
  try {
    const raw = await invoke<RawNeuroWsMessageEnvelope>('neuro_ws_request', { request })
    return mapNeuroWsMessageEnvelope(raw)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function loadCodexDefaultConfig(): Promise<RuntimeCodexConfig> {
  return invoke<RuntimeCodexConfig>('load_codex_default_config')
}

const RUNTIME_SESSION_START_COMMAND = 'start_codex_session'
const RUNTIME_SESSION_STOP_COMMAND = 'stop_codex_session'

export async function codexRuntimeSessionStart(
  config?: StartCodexSessionConfig,
): Promise<StartCodexSessionResponse> {
  return invoke<StartCodexSessionResponse>(RUNTIME_SESSION_START_COMMAND, {
    config,
  })
}

export async function codexRuntimeSessionStop(): Promise<void> {
  await invoke(RUNTIME_SESSION_STOP_COMMAND)
}

export async function startCodexSession(
  config?: StartCodexSessionConfig,
): Promise<StartCodexSessionResponse> {
  return invoke<StartCodexSessionResponse>('start_codex_session', { config })
}

export async function stopCodexSession(): Promise<void> {
  await invoke('stop_codex_session')
}

export async function codexTurnRun(
  request: CodexTurnRunRequest,
): Promise<CodexTurnRunResponse> {
  return invoke<CodexTurnRunResponse>('codex_turn_run', { request })
}

export async function codexTurnSteer(
  request: CodexTurnSteerRequest,
): Promise<CodexTurnSteerResponse> {
  return invoke<CodexTurnSteerResponse>('codex_turn_steer', { request })
}

export async function codexTurnInterrupt(
  request: CodexTurnInterruptRequest,
): Promise<CodexTurnInterruptResponse> {
  return invoke<CodexTurnInterruptResponse>('codex_turn_interrupt', { request })
}
export async function codexReviewStart(
  request: CodexReviewStartRequest,
): Promise<CodexReviewStartResponse> {
  return invoke<CodexReviewStartResponse>('codex_review_start', { request })
}

export async function codexThreadOpen(
  threadId?: string,
): Promise<CodexThreadOpenResponse> {
  return invoke<CodexThreadOpenResponse>('codex_thread_open', { threadId })
}

export async function codexThreadClose(
  request: CodexThreadCloseRequest,
): Promise<CodexThreadCloseResponse> {
  return invoke<CodexThreadCloseResponse>('codex_thread_close', { request })
}

export async function codexThreadList(
  request?: CodexThreadListRequest,
): Promise<CodexThreadListResponse> {
  return invoke<CodexThreadListResponse>('codex_thread_list', { request })
}

export async function codexThreadRead(
  request: CodexThreadReadRequest,
): Promise<CodexThreadReadResponse> {
  return invoke<CodexThreadReadResponse>('codex_thread_read', { request })
}

export async function codexThreadArchive(
  request: CodexThreadArchiveRequest,
): Promise<CodexThreadArchiveResponse> {
  return invoke<CodexThreadArchiveResponse>('codex_thread_archive', { request })
}

export async function codexThreadUnarchive(
  request: CodexThreadUnarchiveRequest,
): Promise<CodexThreadUnarchiveResponse> {
  return invoke<CodexThreadUnarchiveResponse>('codex_thread_unarchive', {
    request,
  })
}

export async function codexThreadCompactStart(
  request: CodexThreadCompactStartRequest,
): Promise<CodexThreadCompactStartResponse> {
  return invoke<CodexThreadCompactStartResponse>('codex_thread_compact_start', {
    request,
  })
}

export async function codexThreadRollback(
  request: CodexThreadRollbackRequest,
): Promise<CodexThreadRollbackResponse> {
  return invoke<CodexThreadRollbackResponse>('codex_thread_rollback', { request })
}

export async function codexThreadFork(
  request: CodexThreadForkRequest,
): Promise<CodexThreadForkResponse> {
  return invoke<CodexThreadForkResponse>('codex_thread_fork', { request })
}

export async function codexApprovalRespond(
  request: CodexApprovalRespondRequest,
): Promise<void> {
  await invoke('codex_approval_respond', { request })
}

export async function codexUserInputRespond(
  request: CodexUserInputRespondRequest,
): Promise<CodexUserInputRespondResponse> {
  return invoke<CodexUserInputRespondResponse>('codex_user_input_respond', { request })
}

export async function sendCodexInput(text: string): Promise<void> {
  await invoke('send_codex_input', { text })
}

export async function updateCodexRuntimeConfig(
  config: RuntimeCodexConfig,
): Promise<RuntimeCodexConfig> {
  return invoke<RuntimeCodexConfig>('update_codex_config', { config })
}

export async function codexConfigGet(): Promise<RuntimeCodexConfig> {
  return invoke<RuntimeCodexConfig>('codex_config_get')
}

export async function codexConfigSet(
  patch: RuntimeCodexConfig,
): Promise<RuntimeCodexConfig> {
  return invoke<RuntimeCodexConfig>('codex_config_set', { patch })
}

export async function runCodexCommand(
  args: string[],
  cwd?: string,
): Promise<RunCodexCommandResponse> {
  return invoke<RunCodexCommandResponse>('run_codex_command', { args, cwd })
}


export async function codexWorkspaceChanges(): Promise<WorkspaceChangesResponse> {
  return invoke<WorkspaceChangesResponse>("git_workspace_changes")
}
export async function gitCommitApprovedReview(
  request: GitCommitApprovedReviewRequest,
): Promise<GitCommitApprovedReviewResponse> {
  return invoke<GitCommitApprovedReviewResponse>('git_commit_approved_review', {
    request,
  })
}

export async function codexModelsList(): Promise<CodexModelListResponse> {
  return invoke<CodexModelListResponse>('codex_models_list')
}

export async function codexWaitForMcpStartup(): Promise<McpStartupWarmupResponse> {
  return invoke<McpStartupWarmupResponse>('codex_wait_for_mcp_startup')
}

export async function codexMcpList(): Promise<McpServerListResponse> {
  return invoke<McpServerListResponse>('codex_mcp_list')
}

export async function codexAppList(
  request?: AppListRequest,
): Promise<AppListResponse> {
  return invoke<AppListResponse>('codex_app_list', { request })
}

export async function codexAccountRead(
  request?: AccountReadRequest,
): Promise<AccountReadResponse> {
  return invoke<AccountReadResponse>('codex_account_read', { request })
}

export async function codexAccountLoginStart(
  request: AccountLoginStartRequest,
): Promise<AccountLoginStartResponse> {
  return invoke<AccountLoginStartResponse>('codex_account_login_start', { request })
}

export async function codexAccountLogout(): Promise<AccountLogoutResponse> {
  return invoke<AccountLogoutResponse>('codex_account_logout')
}

export async function codexAccountRateLimitsRead(): Promise<AccountRateLimitsReadResponse> {
  return invoke<AccountRateLimitsReadResponse>('codex_account_rate_limits_read')
}
export async function codexMcpLogin(
  request: McpLoginRequest,
): Promise<McpLoginResponse> {
  return invoke<McpLoginResponse>('codex_mcp_login', { request })
}

export async function codexMcpReload(): Promise<McpReloadResponse> {
  return invoke<McpReloadResponse>('codex_mcp_reload')
}

export async function terminalCreate(
  request?: TerminalCreateRequest,
): Promise<TerminalCreateResponse> {
  return invoke<TerminalCreateResponse>('terminal_create', { request })
}

export async function terminalWrite(
  terminalId: number,
  data: string,
): Promise<void> {
  await invoke('terminal_write', {
    request: { terminalId, data },
  })
}

export async function terminalResize(
  terminalId: number,
  cols: number,
  rows: number,
): Promise<void> {
  await invoke('terminal_resize', {
    request: { terminalId, cols, rows },
  })
}

export async function terminalKill(terminalId: number): Promise<void> {
  await invoke('terminal_kill', {
    request: { terminalId },
  })
}

export async function pickImageFile(): Promise<string | null> {
  return invoke<string | null>('pick_image_file')
}

export async function pickMentionFile(): Promise<string | null> {
  return invoke<string | null>('pick_mention_file')
}

export async function codexHelpSnapshot(): Promise<CodexHelpSnapshot> {
  return invoke<CodexHelpSnapshot>('codex_help_snapshot')
}

export async function resizeCodexPty(rows: number, cols: number): Promise<void> {
  await invoke('resize_codex_pty', { rows, cols })
}

