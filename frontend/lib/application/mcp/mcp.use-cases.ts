import type { RuntimeClientPort } from "@/lib/application/ports/runtime-client.port"
import type {
  McpLoginResponse,
  McpReloadResponse,
  RunCodexCommandResponse,
} from "@/lib/application/runtime-types"

export async function reloadMcpConfig(
  runtimeClient: RuntimeClientPort,
): Promise<McpReloadResponse> {
  return runtimeClient.mcpReload()
}

export async function loginMcpServer(
  name: string,
  runtimeClient: RuntimeClientPort,
): Promise<McpLoginResponse> {
  return runtimeClient.mcpLogin({ name })
}

export async function executeCodexCommand(
  args: string[],
  runtimeClient: RuntimeClientPort,
  cwd?: string,
): Promise<RunCodexCommandResponse> {
  return runtimeClient.runCodexCommand(args, cwd)
}
