"use client"

import {
  codexAccountLoginStart,
  codexAccountLogout,
  codexMcpLogin,
  codexMcpReload,
  neuroAdtListNamespaces,
  neuroAdtListPackages,
  neuroAdtServerConnect,
  neuroAdtServerList,
  neuroAdtServerRemove,
  neuroAdtServerSelect,
  neuroAdtServerUpsert,
  neuroSearchObjects,
  runCodexCommand,
} from "@/lib/tauri-bridge"
import type { RuntimeClientPort } from "@/lib/application/ports/runtime-client.port"

export const tauriRuntimeClientAdapter: RuntimeClientPort = {
  accountLoginStart: (request) => codexAccountLoginStart(request),
  accountLogout: () => codexAccountLogout(),

  mcpLogin: (request) => codexMcpLogin(request),
  mcpReload: () => codexMcpReload(),
  runCodexCommand: (args, cwd) => runCodexCommand(args, cwd),

  adtServerList: () => neuroAdtServerList(),
  adtServerUpsert: (request) => neuroAdtServerUpsert(request),
  adtServerRemove: (serverId) => neuroAdtServerRemove(serverId),
  adtServerSelect: (serverId) => neuroAdtServerSelect(serverId),
  adtServerConnect: (serverId) => neuroAdtServerConnect(serverId),
  adtListPackages: (serverId) => neuroAdtListPackages(serverId),
  adtListNamespaces: (packageName, serverId) =>
    neuroAdtListNamespaces(packageName, serverId),
  adtSearchObjects: (query, limit, serverId) =>
    neuroSearchObjects(query, limit, serverId),
}