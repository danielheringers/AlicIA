import type {
  AccountLoginStartResponse,
  AccountLogoutResponse,
  McpLoginResponse,
  McpReloadResponse,
  NeuroAdtNamespaceSummary,
  NeuroAdtObjectSummary,
  NeuroAdtPackageSummary,
  NeuroAdtServerConnectResponse,
  NeuroAdtServerListResponse,
  NeuroAdtServerRecord,
  NeuroAdtServerUpsertRequest,
  RunCodexCommandResponse,
} from "@/lib/application/runtime-types"

export interface RuntimeClientPort {
  accountLoginStart(request: {
    type: "chatgpt" | "apiKey"
    apiKey?: string
  }): Promise<AccountLoginStartResponse>
  accountLogout(): Promise<AccountLogoutResponse>

  mcpLogin(request: { name: string; scopes?: string[]; timeoutSecs?: number }): Promise<McpLoginResponse>
  mcpReload(): Promise<McpReloadResponse>
  runCodexCommand(args: string[], cwd?: string): Promise<RunCodexCommandResponse>

  adtServerList(): Promise<NeuroAdtServerListResponse>
  adtServerUpsert(request: NeuroAdtServerUpsertRequest): Promise<NeuroAdtServerRecord>
  adtServerRemove(serverId: string): Promise<void>
  adtServerSelect(serverId: string): Promise<string>
  adtServerConnect(serverId: string): Promise<NeuroAdtServerConnectResponse>
  adtListPackages(serverId?: string | null): Promise<NeuroAdtPackageSummary[]>
  adtListNamespaces(
    packageName?: string | null,
    serverId?: string | null,
  ): Promise<NeuroAdtNamespaceSummary[]>
  adtSearchObjects(
    query: string,
    limit?: number,
    serverId?: string | null,
  ): Promise<NeuroAdtObjectSummary[]>
}