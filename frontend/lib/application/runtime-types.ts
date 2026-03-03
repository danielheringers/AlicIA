import { APPLICATION_RUNTIME_METHODS } from "@/lib/application/contracts/runtime-methods.contract"

export type RuntimeMethod = (typeof APPLICATION_RUNTIME_METHODS)[number]
export type RuntimeMethodCapabilities = Record<RuntimeMethod, boolean>

export type ApprovalDecision =
  | "accept"
  | "acceptForSession"
  | "decline"
  | "cancel"
  | "acceptWithExecpolicyAmendment"

export type AccountAuthType =
  | "api_key"
  | "chatgpt"
  | "chatgpt_auth_tokens"
  | "unknown"
  | "none"

export interface AccountLoginStartResponse {
  type: AccountAuthType
  loginId?: string | null
  authUrl?: string | null
  started: boolean
  elapsedMs: number
}

export interface AccountLogoutResponse {
  loggedOut: boolean
  elapsedMs: number
}

export interface CodexReasoningEffortOption {
  reasoningEffort: "none" | "minimal" | "low" | "medium" | "high" | "xhigh"
  description: string
}

export interface CodexModel {
  id: string
  model: string
  displayName: string
  description: string
  supportedReasoningEfforts: CodexReasoningEffortOption[]
  defaultReasoningEffort: "none" | "minimal" | "low" | "medium" | "high" | "xhigh"
  supportsPersonality: boolean
  isDefault: boolean
  upgrade?: string | null
}

export interface McpLoginResponse {
  name: string
  authorizationUrl?: string | null
  started: boolean
  elapsedMs: number
}

export interface McpReloadResponse {
  reloaded: boolean
  elapsedMs: number
}

export interface NeuroAdtObjectSummary {
  uri: string
  name: string
  objectType?: string | null
  package?: string | null
}

export interface NeuroAdtPackageSummary {
  name: string
  description?: string | null
}

export interface NeuroAdtNamespaceSummary {
  name: string
  packageName?: string | null
}

export interface NeuroAdtServerRecord {
  id: string
  name: string
  baseUrl: string
  client?: string | null
  language?: string | null
  username?: string | null
}

export interface NeuroAdtServerListResponse {
  servers: NeuroAdtServerRecord[]
  selectedServerId?: string | null
}

export interface NeuroAdtServerUpsertRequest {
  id: string
  name: string
  baseUrl: string
  client?: string | null
  language?: string | null
  username?: string | null
  password?: string | null
}

export interface NeuroAdtServerConnectResponse {
  serverId: string
  connected: boolean
  message?: string | null
}

export interface RunCodexCommandResponse {
  stdout: string
  stderr: string
  status: number
  success: boolean
}


