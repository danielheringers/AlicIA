import type { RuntimeClientPort } from "@/lib/application/ports/runtime-client.port"
import type {
  NeuroAdtNamespaceSummary,
  NeuroAdtObjectSummary,
  NeuroAdtPackageSummary,
  NeuroAdtServerConnectResponse,
  NeuroAdtServerListResponse,
  NeuroAdtServerRecord,
  NeuroAdtServerUpsertRequest,
} from "@/lib/application/runtime-types"

export async function listAdtServers(
  runtimeClient: RuntimeClientPort,
): Promise<NeuroAdtServerListResponse> {
  return runtimeClient.adtServerList()
}

export async function upsertAdtServer(
  request: NeuroAdtServerUpsertRequest,
  runtimeClient: RuntimeClientPort,
): Promise<NeuroAdtServerRecord> {
  return runtimeClient.adtServerUpsert(request)
}

export async function removeAdtServer(
  serverId: string,
  runtimeClient: RuntimeClientPort,
): Promise<void> {
  return runtimeClient.adtServerRemove(serverId)
}

export async function selectAdtServer(
  serverId: string,
  runtimeClient: RuntimeClientPort,
): Promise<string> {
  return runtimeClient.adtServerSelect(serverId)
}

export async function connectAdtServer(
  serverId: string,
  runtimeClient: RuntimeClientPort,
): Promise<NeuroAdtServerConnectResponse> {
  return runtimeClient.adtServerConnect(serverId)
}

export async function listAdtPackages(
  serverId: string | null | undefined,
  runtimeClient: RuntimeClientPort,
): Promise<NeuroAdtPackageSummary[]> {
  return runtimeClient.adtListPackages(serverId)
}

export async function listAdtNamespaces(
  packageName: string | null | undefined,
  serverId: string | null | undefined,
  runtimeClient: RuntimeClientPort,
): Promise<NeuroAdtNamespaceSummary[]> {
  return runtimeClient.adtListNamespaces(packageName, serverId)
}

export async function searchAdtObjects(
  query: string,
  limit: number | undefined,
  serverId: string | null | undefined,
  runtimeClient: RuntimeClientPort,
): Promise<NeuroAdtObjectSummary[]> {
  return runtimeClient.adtSearchObjects(query, limit, serverId)
}
