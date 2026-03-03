"use client"

import { useEffect, useMemo, useRef, useState } from "react"
import {
  AlertTriangle,
  CheckCircle2,
  Database,
  Loader2,
  PlugZap,
  RefreshCw,
  Search,
  Trash2,
  X,
} from "lucide-react"
import {
  listAdtNamespaces,
  listAdtPackages,
  connectAdtServer,
  listAdtServers,
  removeAdtServer,
  selectAdtServer,
  upsertAdtServer,
  searchAdtObjects,
} from "@/lib/application/adt/adt.use-cases"
import { tauriRuntimeClientAdapter } from "@/lib/infrastructure/tauri/tauri-runtime-client.adapter"
import {
  type NeuroAdtNamespaceSummary,
  type NeuroAdtObjectSummary,
  type NeuroAdtPackageSummary,
  type NeuroAdtServerRecord,
} from "@/lib/application/runtime-types"

interface AdtPanelProps {
  activeServerId: string | null
  onActiveServerIdChange: (serverId: string | null) => void
  onOpenInEditor: (ref: string) => void
  onClose: () => void
}

interface AdtServerDraft {
  id: string
  name: string
  baseUrl: string
  client: string
  language: string
  username: string
  password: string
}

const INITIAL_DRAFT: AdtServerDraft = {
  id: "",
  name: "",
  baseUrl: "",
  client: "",
  language: "",
  username: "",
  password: "",
}

type PackageCapability = "unknown" | "available" | "unavailable"
type GlobalNamespaceCapability = "unknown" | "available" | "unavailable"

function mapServerToDraft(server: NeuroAdtServerRecord): AdtServerDraft {
  return {
    id: server.id,
    name: server.name,
    baseUrl: server.baseUrl,
    client: server.client ?? "",
    language: server.language ?? "",
    username: server.username ?? "",
    password: "",
  }
}

export function AdtPanel({
  activeServerId,
  onActiveServerIdChange,
  onOpenInEditor,
  onClose,
}: AdtPanelProps) {
  const [servers, setServers] = useState<NeuroAdtServerRecord[]>([])
  const [loadingServers, setLoadingServers] = useState(false)
  const [busyKey, setBusyKey] = useState<string | null>(null)
  const [panelError, setPanelError] = useState<string | null>(null)
  const [panelInfo, setPanelInfo] = useState<string | null>(null)
  const [editingServerId, setEditingServerId] = useState<string | null>(null)
  const [draft, setDraft] = useState<AdtServerDraft>(INITIAL_DRAFT)
  const [objectQuery, setObjectQuery] = useState("")
  const [searchingObjects, setSearchingObjects] = useState(false)
  const [objectResults, setObjectResults] = useState<NeuroAdtObjectSummary[]>([])
  const [packages, setPackages] = useState<NeuroAdtPackageSummary[]>([])
  const [selectedPackage, setSelectedPackage] = useState<string | null>(null)
  const [selectedNamespace, setSelectedNamespace] = useState<string | null>(null)
  const [globalNamespaces, setGlobalNamespaces] = useState<NeuroAdtNamespaceSummary[]>([])
  const [packageNamespaces, setPackageNamespaces] = useState<NeuroAdtNamespaceSummary[]>([])
  const [loadingGlobalNamespaces, setLoadingGlobalNamespaces] = useState(false)
  const [loadingPackageNamespaces, setLoadingPackageNamespaces] = useState(false)
  const [packageCapability, setPackageCapability] =
    useState<PackageCapability>("unknown")
  const [globalNamespaceCapability, setGlobalNamespaceCapability] =
    useState<GlobalNamespaceCapability>("unknown")
  const [globalNamespaceError, setGlobalNamespaceError] = useState<string | null>(null)
  const navigationRequestIdRef = useRef(0)
  const namespaceRequestIdRef = useRef(0)

  const activeServer = useMemo(
    () => servers.find((server) => server.id === activeServerId) ?? null,
    [servers, activeServerId],
  )

  const runBusy = async (key: string, action: () => Promise<void>) => {
    setBusyKey(key)
    setPanelError(null)
    setPanelInfo(null)
    try {
      await action()
    } catch (error) {
      setPanelError(String(error))
    } finally {
      setBusyKey(null)
    }
  }

  const reloadServers = async () => {
    setLoadingServers(true)
    try {
      const response = await listAdtServers(tauriRuntimeClientAdapter)
      setServers(response.servers)
      const selected = response.selectedServerId ?? null
      if (selected !== activeServerId) {
        onActiveServerIdChange(selected)
      }
      if (!selected) {
        setPackages([])
        setGlobalNamespaces([])
        setPackageNamespaces([])
        setSelectedPackage(null)
        setSelectedNamespace(null)
        setPackageCapability("unknown")
        setGlobalNamespaceCapability("unknown")
        setGlobalNamespaceError(null)
        setLoadingGlobalNamespaces(false)
        setLoadingPackageNamespaces(false)
      }
      setPanelError(null)
      setPanelInfo(null)
    } catch (error) {
      const message = String(error)
      const unsupportedList =
        /neuro_adt_server_list|not supported|unsupported|unknown command/i.test(
          message,
        )
      setServers([])
      setPackages([])
      setGlobalNamespaces([])
      setPackageNamespaces([])
      setSelectedPackage(null)
      setSelectedNamespace(null)
      setPackageCapability("unavailable")
      setGlobalNamespaceCapability("unknown")
      setGlobalNamespaceError(null)
      setLoadingGlobalNamespaces(false)
      setLoadingPackageNamespaces(false)
      onActiveServerIdChange(null)
      if (unsupportedList) {
        setPanelError(null)
        setPanelInfo(
          "Modo legado detectado: backend sem listagem de servidores ADT. Selecao ativa foi limpa.",
        )
      } else {
        setPanelInfo(null)
        setPanelError(`Falha ao carregar servidores ADT: ${message}`)
      }
    } finally {
      setLoadingServers(false)
    }
  }

  const loadNavigationData = async (serverId: string | null) => {
    const navigationRequestId = navigationRequestIdRef.current + 1
    navigationRequestIdRef.current = navigationRequestId
    namespaceRequestIdRef.current += 1

    if (!serverId) {
      setPackages([])
      setSelectedPackage(null)
      setSelectedNamespace(null)
      setGlobalNamespaces([])
      setPackageNamespaces([])
      setPackageCapability("unknown")
      setGlobalNamespaceCapability("unknown")
      setGlobalNamespaceError(null)
      setLoadingGlobalNamespaces(false)
      setLoadingPackageNamespaces(false)
      return
    }

    try {
      setGlobalNamespaces([])
      setGlobalNamespaceCapability("unknown")
      setGlobalNamespaceError(null)
      setLoadingGlobalNamespaces(true)
      const nextPackages = await listAdtPackages(serverId, tauriRuntimeClientAdapter)
      if (navigationRequestId !== navigationRequestIdRef.current) {
        return
      }
      setPackages(nextPackages)
      setSelectedPackage(null)
      setSelectedNamespace(null)
      setPackageNamespaces([])
      setPackageCapability("available")

      try {
        const response = await listAdtNamespaces(null, serverId, tauriRuntimeClientAdapter)
        if (navigationRequestId !== navigationRequestIdRef.current) {
          return
        }
        setGlobalNamespaces(response)
        setGlobalNamespaceCapability("available")
        setGlobalNamespaceError(null)
      } catch (error) {
        if (navigationRequestId !== navigationRequestIdRef.current) {
          return
        }
        setGlobalNamespaces([])
        setGlobalNamespaceCapability("unavailable")
        setGlobalNamespaceError(`Falha ao carregar namespaces globais: ${String(error)}`)
      }
    } catch {
      if (navigationRequestId !== navigationRequestIdRef.current) {
        return
      }
      setPackages([])
      setSelectedPackage(null)
      setSelectedNamespace(null)
      setGlobalNamespaces([])
      setPackageNamespaces([])
      setPackageCapability("unavailable")
      setGlobalNamespaceCapability("unknown")
      setGlobalNamespaceError(null)
    } finally {
      if (navigationRequestId === navigationRequestIdRef.current) {
        setLoadingGlobalNamespaces(false)
      }
    }
  }

  useEffect(() => {
    void reloadServers()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    void loadNavigationData(activeServerId)
  }, [activeServerId])

  const namespaceFilterState = useMemo(() => {
    if (!selectedNamespace) {
      return {
        packages,
        hasMapping: true,
      }
    }

    const mappedPackages = new Set(
      globalNamespaces
        .filter((namespace) => namespace.name === selectedNamespace)
        .map((namespace) => namespace.packageName)
        .filter((packageName): packageName is string => Boolean(packageName)),
    )

    if (mappedPackages.size === 0) {
      return {
        packages,
        hasMapping: false,
      }
    }

    return {
      packages: packages.filter((entry) => mappedPackages.has(entry.name)),
      hasMapping: true,
    }
  }, [globalNamespaces, packages, selectedNamespace])

  const filteredPackages = namespaceFilterState.packages
  const selectedNamespaceHasMapping = namespaceFilterState.hasMapping

  const visibleNamespaces = selectedPackage ? packageNamespaces : globalNamespaces
  const namespacesLoading = selectedPackage
    ? loadingPackageNamespaces
    : loadingGlobalNamespaces

  const handleSubmitServer = () => {
    void runBusy("upsert", async () => {
      const id = draft.id.trim()
      const name = draft.name.trim()
      const baseUrl = draft.baseUrl.trim()
      if (!id || !name || !baseUrl) {
        throw new Error("Preencha id, nome e base URL do servidor ADT.")
      }

      await upsertAdtServer({
        id,
        name,
        baseUrl,
        client: draft.client.trim() || null,
        language: draft.language.trim() || null,
        username: draft.username.trim() || null,
        password: draft.password.trim() || null,
      }, tauriRuntimeClientAdapter)

      setDraft(INITIAL_DRAFT)
      setEditingServerId(null)
      await reloadServers()
      setPanelInfo(`Servidor ADT ${id} salvo.`)
    })
  }

  const handleEditServer = (server: NeuroAdtServerRecord) => {
    setEditingServerId(server.id)
    setDraft(mapServerToDraft(server))
  }

  const handleRemoveServer = (server: NeuroAdtServerRecord) => {
    if (!window.confirm(`Remover servidor ADT "${server.name}"?`)) {
      return
    }

    void runBusy(`remove:${server.id}`, async () => {
      await removeAdtServer(server.id, tauriRuntimeClientAdapter)
      if (activeServerId === server.id) {
        onActiveServerIdChange(null)
      }
      await reloadServers()
      setPanelInfo(`Servidor ADT ${server.name} removido.`)
    })
  }

  const handleSelectServer = (serverId: string) => {
    void runBusy(`select:${serverId}`, async () => {
      const selectedServerId = await selectAdtServer(serverId, tauriRuntimeClientAdapter)
      onActiveServerIdChange(selectedServerId)
      setPanelInfo(`Servidor ADT ativo: ${selectedServerId}.`)
    })
  }

  const handleConnectServer = (serverId: string) => {
    void runBusy(`connect:${serverId}`, async () => {
      const selectedServerId = await selectAdtServer(serverId, tauriRuntimeClientAdapter)
      onActiveServerIdChange(selectedServerId)

      const response = await connectAdtServer(selectedServerId, tauriRuntimeClientAdapter)
      setPanelInfo(
        response.message ??
          (response.connected
            ? `Conexao ADT ativa para ${response.serverId}.`
            : `Falha ao conectar ${response.serverId}.`),
      )
    })
  }

  const handleSearchObjects = () => {
    void runBusy("search", async () => {
      const query = objectQuery.trim()
      if (!query) {
        throw new Error("Informe um termo para buscar objetos ADT.")
      }
      setSearchingObjects(true)
      try {
        const results = await searchAdtObjects(query, 60, activeServerId, tauriRuntimeClientAdapter)
        setObjectResults(results)
      } finally {
        setSearchingObjects(false)
      }
    })
  }

  const handleLoadNamespaces = (packageName: string) => {
    if (!activeServerId) {
      return
    }

    void runBusy(`namespaces:${packageName}`, async () => {
      const namespaceRequestId = namespaceRequestIdRef.current + 1
      namespaceRequestIdRef.current = namespaceRequestId
      setSelectedPackage(packageName)
      setSelectedNamespace(null)
      setLoadingPackageNamespaces(true)
      try {
        const response = await listAdtNamespaces(packageName, activeServerId, tauriRuntimeClientAdapter)
        if (namespaceRequestId !== namespaceRequestIdRef.current) {
          return
        }
        setPackageNamespaces(response)
      } finally {
        if (namespaceRequestId === namespaceRequestIdRef.current) {
          setLoadingPackageNamespaces(false)
        }
      }
    })
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="w-full max-w-5xl border border-panel-border rounded-lg bg-panel-bg shadow-2xl flex flex-col max-h-[86vh]"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-center justify-between px-4 py-3 border-b border-panel-border shrink-0">
          <div className="flex items-center gap-2 min-w-0">
            <Database className="w-4 h-4 text-terminal-cyan shrink-0" />
            <span className="text-sm font-semibold text-terminal-fg">ADT Connections</span>
            <span className="text-[10px] text-muted-foreground/50 truncate">
              {activeServer ? `ativo: ${activeServer.name}` : "nenhum servidor ativo"}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => {
                void runBusy("reload-servers", reloadServers)
              }}
              className="inline-flex items-center gap-1 px-2 py-1 rounded border border-panel-border text-xs text-terminal-blue hover:text-terminal-blue/80 disabled:opacity-40"
              disabled={loadingServers || busyKey !== null}
            >
              {loadingServers ? (
                <Loader2 className="w-3 h-3 animate-spin" />
              ) : (
                <RefreshCw className="w-3 h-3" />
              )}
              Reload
            </button>
            <button
              onClick={onClose}
              className="p-1 rounded hover:bg-[#ffffff08] text-muted-foreground transition-colors"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        {(panelError || panelInfo) && (
          <div className="px-3 pt-3 shrink-0 space-y-2">
            {panelError && (
              <div className="px-2.5 py-2 rounded border border-terminal-red/30 bg-terminal-red/10 text-[11px] text-terminal-red/90 flex items-center gap-2">
                <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                <span>{panelError}</span>
              </div>
            )}
            {!panelError && panelInfo && (
              <div className="px-2.5 py-2 rounded border border-terminal-green/25 bg-terminal-green/10 text-[11px] text-terminal-green/90 flex items-center gap-2">
                <CheckCircle2 className="w-3.5 h-3.5 shrink-0" />
                <span>{panelInfo}</span>
              </div>
            )}
          </div>
        )}

        <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 overflow-y-auto p-3 lg:grid-cols-2">
          <section className="rounded border border-panel-border bg-background/20 p-3">
            <div className="mb-2 flex items-center justify-between">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Servidores ADT
              </h3>
              <span className="text-[10px] text-muted-foreground/60">{servers.length} cadastrados</span>
            </div>

            <div className="space-y-2">
              {servers.map((server) => {
                const selected = activeServerId === server.id
                return (
                  <div key={server.id} className="rounded border border-panel-border p-2">
                    <div className="flex items-start justify-between gap-2">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="text-sm text-terminal-fg">{server.name}</span>
                          {selected ? (
                            <span className="rounded border border-terminal-green/30 bg-terminal-green/10 px-1.5 py-0.5 text-[10px] text-terminal-green">
                              ativo
                            </span>
                          ) : null}
                        </div>
                        <div className="text-[11px] text-muted-foreground truncate">{server.baseUrl}</div>
                        <div className="text-[10px] text-muted-foreground/70">
                          id: <span className="font-mono">{server.id}</span>
                        </div>
                      </div>
                      <div className="flex flex-wrap items-center justify-end gap-2">
                        <button
                          type="button"
                          className="text-[10px] text-terminal-cyan hover:text-terminal-cyan/80"
                          onClick={() => handleEditServer(server)}
                        >
                          Editar
                        </button>
                        <button
                          type="button"
                          className="text-[10px] text-terminal-blue hover:text-terminal-blue/80"
                          onClick={() => handleSelectServer(server.id)}
                        >
                          Selecionar
                        </button>
                        <button
                          type="button"
                          className="inline-flex items-center gap-1 text-[10px] text-terminal-green hover:text-terminal-green/80"
                          onClick={() => handleConnectServer(server.id)}
                        >
                          <PlugZap className="w-3 h-3" />
                          Conectar
                        </button>
                        <button
                          type="button"
                          className="inline-flex items-center gap-1 text-[10px] text-terminal-red hover:text-terminal-red/80"
                          onClick={() => handleRemoveServer(server)}
                        >
                          <Trash2 className="w-3 h-3" />
                          Remover
                        </button>
                      </div>
                    </div>
                  </div>
                )
              })}
              {servers.length === 0 && (
                <div className="rounded border border-dashed border-panel-border px-2 py-4 text-center text-xs text-muted-foreground/70">
                  Nenhum servidor ADT cadastrado.
                </div>
              )}
            </div>
          </section>

          <section className="rounded border border-panel-border bg-background/20 p-3">
            <div className="mb-2 flex items-center justify-between">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                {editingServerId ? `Editar ${editingServerId}` : "Novo servidor"}
              </h3>
              {editingServerId ? (
                <button
                  type="button"
                  className="text-[10px] text-muted-foreground hover:text-terminal-fg"
                  onClick={() => {
                    setEditingServerId(null)
                    setDraft(INITIAL_DRAFT)
                  }}
                >
                  Cancelar edicao
                </button>
              ) : null}
            </div>
            <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
              <input
                value={draft.id}
                onChange={(event) => setDraft((previous) => ({ ...previous, id: event.target.value }))}
                placeholder="id (ex: prd)"
                className="px-2 py-1.5 text-xs rounded bg-background border border-panel-border text-terminal-fg"
              />
              <input
                value={draft.name}
                onChange={(event) =>
                  setDraft((previous) => ({ ...previous, name: event.target.value }))
                }
                placeholder="nome"
                className="px-2 py-1.5 text-xs rounded bg-background border border-panel-border text-terminal-fg"
              />
              <input
                value={draft.baseUrl}
                onChange={(event) =>
                  setDraft((previous) => ({ ...previous, baseUrl: event.target.value }))
                }
                placeholder="https://host:port"
                className="px-2 py-1.5 text-xs rounded bg-background border border-panel-border text-terminal-fg md:col-span-2"
              />
              <input
                value={draft.client}
                onChange={(event) =>
                  setDraft((previous) => ({ ...previous, client: event.target.value }))
                }
                placeholder="cliente SAP (opcional)"
                className="px-2 py-1.5 text-xs rounded bg-background border border-panel-border text-terminal-fg"
              />
              <input
                value={draft.language}
                onChange={(event) =>
                  setDraft((previous) => ({ ...previous, language: event.target.value }))
                }
                placeholder="idioma (opcional)"
                className="px-2 py-1.5 text-xs rounded bg-background border border-panel-border text-terminal-fg"
              />
              <input
                value={draft.username}
                onChange={(event) =>
                  setDraft((previous) => ({ ...previous, username: event.target.value }))
                }
                placeholder="usuario (opcional)"
                className="px-2 py-1.5 text-xs rounded bg-background border border-panel-border text-terminal-fg"
              />
              <input
                value={draft.password}
                onChange={(event) =>
                  setDraft((previous) => ({ ...previous, password: event.target.value }))
                }
                placeholder="senha (opcional)"
                type="password"
                className="px-2 py-1.5 text-xs rounded bg-background border border-panel-border text-terminal-fg"
              />
            </div>
            <div className="mt-2 flex justify-end">
              <button
                type="button"
                onClick={handleSubmitServer}
                disabled={busyKey !== null}
                className="rounded border border-terminal-green/30 px-2.5 py-1 text-xs text-terminal-green hover:text-terminal-green/80 disabled:opacity-40"
              >
                {busyKey === "upsert" ? "Salvando..." : editingServerId ? "Atualizar" : "Criar"}
              </button>
            </div>
          </section>

          <section className="rounded border border-panel-border bg-background/20 p-3">
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Navegacao ADT
            </h3>
            {activeServerId ? (
              <>
                {packageCapability === "available" ? (
                  <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
                    <div className="rounded border border-panel-border p-2">
                      <div className="mb-1 flex items-center justify-between gap-2">
                        <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                          Pacotes
                        </div>
                        {selectedNamespace ? (
                          <button
                            type="button"
                            className="text-[10px] text-terminal-blue hover:text-terminal-blue/80"
                            onClick={() => setSelectedNamespace(null)}
                          >
                            limpar filtro
                          </button>
                        ) : null}
                      </div>
                      {selectedNamespace ? (
                        <div className="mb-1 text-[10px] text-muted-foreground/80">
                          Filtrado por namespace:{" "}
                          <span className="font-mono text-terminal-blue">{selectedNamespace}</span>
                        </div>
                      ) : null}
                      {selectedNamespace && !selectedNamespaceHasMapping ? (
                        <div className="mb-1 text-[10px] text-terminal-gold/90">
                          Namespace selecionado sem mapeamento de pacote; exibindo todos os pacotes.
                        </div>
                      ) : null}
                      <div className="max-h-40 overflow-y-auto space-y-1">
                        {filteredPackages.map((entry) => (
                          <button
                            key={entry.name}
                            type="button"
                            className={`block w-full rounded px-1.5 py-1 text-left text-xs transition-colors ${
                              selectedPackage === entry.name
                                ? "bg-terminal-blue/15 text-terminal-blue"
                                : "hover:bg-[var(--ide-hover)] text-terminal-fg/80"
                            }`}
                            onClick={() => handleLoadNamespaces(entry.name)}
                          >
                            {entry.name}
                          </button>
                        ))}
                        {filteredPackages.length === 0 && (
                          <div className="text-[11px] text-muted-foreground/70">Sem pacotes retornados.</div>
                        )}
                      </div>
                    </div>
                    <div className="rounded border border-panel-border p-2">
                      <div className="mb-1 flex items-center justify-between gap-2">
                        <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                          Namespaces
                        </div>
                        {selectedPackage ? (
                          <button
                            type="button"
                            className="text-[10px] text-terminal-blue hover:text-terminal-blue/80"
                            onClick={() => setSelectedPackage(null)}
                          >
                            ver globais
                          </button>
                        ) : null}
                      </div>
                      <div className="mb-1 text-[10px] text-muted-foreground/80">
                        {selectedPackage ? `Pacote: ${selectedPackage}` : "Escopo: global"}
                      </div>
                      <div className="max-h-40 overflow-y-auto space-y-1">
                        {namespacesLoading ? (
                          <div className="flex items-center gap-1 text-[11px] text-muted-foreground/70">
                            <Loader2 className="w-3 h-3 animate-spin" />
                            Carregando...
                          </div>
                        ) : !selectedPackage &&
                          globalNamespaceCapability === "unavailable" ? (
                          <div className="text-[11px] text-terminal-gold/90">
                            {globalNamespaceError ??
                              "Namespaces globais indisponiveis neste backend."}
                          </div>
                        ) : visibleNamespaces.length === 0 ? (
                          <div className="text-[11px] text-muted-foreground/70">
                            {selectedPackage
                              ? "Nenhum namespace retornado para este pacote."
                              : "Nenhum namespace global retornado."}
                          </div>
                        ) : (
                          visibleNamespaces.map((entry) => (
                            <button
                              key={`${entry.packageName ?? "pkg"}:${entry.name}`}
                              type="button"
                              className={`block w-full rounded px-1.5 py-1 text-left text-xs transition-colors ${
                                selectedNamespace === entry.name
                                  ? "bg-terminal-blue/15 text-terminal-blue"
                                  : "hover:bg-[var(--ide-hover)] text-terminal-fg/80"
                              }`}
                              onClick={() =>
                                setSelectedNamespace((current) =>
                                  current === entry.name ? null : entry.name,
                                )
                              }
                            >
                              {entry.name}
                            </button>
                          ))
                        )}
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="rounded border border-terminal-gold/30 bg-terminal-gold/10 px-2 py-2 text-xs text-terminal-gold/90">
                    Este backend nao expoe listagem de pacotes/namespaces ADT. Use a busca de objetos abaixo.
                  </div>
                )}
              </>
            ) : (
              <div className="text-xs text-muted-foreground/70">
                Selecione um servidor ADT para navegar conteudo.
              </div>
            )}
          </section>

          <section className="rounded border border-panel-border bg-background/20 p-3">
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Busca de Objetos
            </h3>
            <div className="mb-2 flex gap-2">
              <input
                value={objectQuery}
                onChange={(event) => setObjectQuery(event.target.value)}
                placeholder="ex: ZCL_* ou nome exato"
                className="flex-1 px-2 py-1.5 text-xs rounded bg-background border border-panel-border text-terminal-fg"
              />
              <button
                type="button"
                onClick={handleSearchObjects}
                disabled={searchingObjects}
                className="inline-flex items-center gap-1 rounded border border-terminal-blue/30 px-2 py-1 text-xs text-terminal-blue hover:text-terminal-blue/80 disabled:opacity-40"
              >
                {searchingObjects ? (
                  <Loader2 className="w-3 h-3 animate-spin" />
                ) : (
                  <Search className="w-3 h-3" />
                )}
                Buscar
              </button>
            </div>
            <div className="max-h-52 space-y-1 overflow-y-auto">
              {objectResults.map((entry) => (
                <button
                  key={entry.uri}
                  type="button"
                  className="block w-full rounded border border-panel-border bg-background/40 px-2 py-1.5 text-left hover:bg-[var(--ide-hover)]"
                  onClick={() => onOpenInEditor(entry.uri)}
                >
                  <div className="text-xs text-terminal-fg">{entry.name}</div>
                  <div className="text-[11px] text-muted-foreground/70 truncate">
                    {entry.objectType ?? "OBJ"} {entry.package ? `- ${entry.package}` : ""}
                  </div>
                </button>
              ))}
              {objectResults.length === 0 && (
                <div className="text-xs text-muted-foreground/70">
                  Use a busca para abrir objetos ADT no editor com o servidor selecionado.
                </div>
              )}
            </div>
          </section>
        </div>
      </div>
    </div>
  )
}




