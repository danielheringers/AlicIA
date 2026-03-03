"use client"

import {
  ChevronDown,
  ChevronRight,
  RefreshCw,
  Star,
  Target,
  Box,
  Boxes,
  LibraryBig,
  Package,
} from "lucide-react"
import { useMemo, useState, type ReactNode } from "react"
import {
  type NeuroAdtExplorerState,
  type NeuroAdtFavoritePackageItem,
  type NeuroAdtNamespaceSummary,
  type NeuroAdtObjectSummary,
  type NeuroAdtPackageInventoryResponse,
} from "@/lib/tauri-bridge"
import { cn } from "@/lib/utils"

interface IdeSapExplorerProps {
  serverId?: string | null
  state: NeuroAdtExplorerState
  localObjects: NeuroAdtObjectSummary[]
  favoriteObjects: NeuroAdtObjectSummary[]
  systemNamespaces: NeuroAdtNamespaceSummary[]
  favoritePackageObjects: Record<string, NeuroAdtObjectSummary[]>
  namespaceObjects: Record<string, NeuroAdtObjectSummary[]>
  packageInventoryObjects?: Record<string, NeuroAdtObjectSummary[]>
  packageInventory?: NeuroAdtPackageInventoryResponse | null
  packageScopeRoots?: string[]
  packageScopeRootInput?: string
  packageScopePresets?: string[]
  loadingState?: boolean
  loadingLocalObjects?: boolean
  loadingSystemNamespaces?: boolean
  loadingFavoritePackages?: boolean
  loadingFavoriteObjects?: boolean
  loadingPackageInventory?: boolean
  loadingPackageObjects?: Record<string, boolean>
  loadingPackageInventoryObjects?: Record<string, boolean>
  loadingNamespaceObjects?: Record<string, boolean>
  onRefresh?: () => void
  onToggleFavoritePackage: (item: NeuroAdtFavoritePackageItem) => void
  onToggleFavoriteObject: (object: NeuroAdtObjectSummary) => void
  onSelectWorkingPackage: (packageName: string) => void
  onOpenFile: (uri: string) => void
  onLoadFavoritePackageObjects: (item: NeuroAdtFavoritePackageItem) => void
  onLoadNamespaceObjects: (namespace: string) => void
  onLoadPackageInventoryObjects?: (packageName: string) => void
  onChangePackageScopeRootInput?: (value: string) => void
  onAddPackageScopeRoot?: () => void
  onRemovePackageScopeRoot?: (root: string) => void
  onApplyPackageScopeRoots?: () => void
  onTogglePackageScopePreset?: (root: string) => void
}

function favoritePackageKey(item: NeuroAdtFavoritePackageItem): string {
  return `${item.kind}:${item.name.toUpperCase()}`
}

function iconForPackageKind(kind: NeuroAdtFavoritePackageItem["kind"]) {
  return kind === "namespace" ? (
    <Boxes className="h-3 w-3 shrink-0 text-terminal-cyan" />
  ) : (
    <Package className="h-3 w-3 shrink-0 text-terminal-gold" />
  )
}

function objectSubtitle(object: NeuroAdtObjectSummary): string {
  const tags = [object.objectType, object.package].filter(Boolean)
  return tags.join(" • ")
}

function TreeObjectRow({
  object,
  isFavorite,
  isFocused,
  onOpenFile,
  onToggleFavorite,
}: {
  object: NeuroAdtObjectSummary
  isFavorite: boolean
  isFocused: boolean
  onOpenFile: (uri: string) => void
  onToggleFavorite: (object: NeuroAdtObjectSummary) => void
}) {
  return (
    <div
      className={cn(
        "group flex items-center gap-1.5 border-l border-transparent px-3 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-[var(--ide-hover)] hover:text-foreground",
        isFocused && "border-terminal-blue bg-[var(--ide-active)] text-foreground",
      )}
    >
      <button
        type="button"
        className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
        title={object.uri}
        onClick={() => {
          onOpenFile(object.uri)
        }}
      >
        <Box className="h-3 w-3 shrink-0 text-terminal-blue" />
        <span className="truncate">{object.name}</span>
        {objectSubtitle(object) ? (
          <span className="truncate text-[10px] text-muted-foreground/70">{objectSubtitle(object)}</span>
        ) : null}
      </button>
      <button
        type="button"
        aria-label={isFavorite ? `Desfavoritar ${object.name}` : `Favoritar ${object.name}`}
        title={isFavorite ? "Remover dos favoritos" : "Adicionar aos favoritos"}
        onClick={() => {
          onToggleFavorite(object)
        }}
        className={cn(
          "rounded border border-transparent p-0.5 transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)]",
          isFavorite ? "text-terminal-gold" : "text-muted-foreground/70",
        )}
      >
        <Star className={cn("h-3 w-3", isFavorite && "fill-current")} />
      </button>
    </div>
  )
}

function TreeSection({
  title,
  icon,
  loading,
  emptyMessage,
  children,
}: {
  title: string
  icon: ReactNode
  loading?: boolean
  emptyMessage: string
  children: ReactNode
}) {
  const [expanded, setExpanded] = useState(true)
  const hasContent = useMemo(() => {
    if (loading) {
      return true
    }
    if (Array.isArray(children)) {
      return children.length > 0
    }
    return Boolean(children)
  }, [children, loading])

  return (
    <section className="border-b border-[var(--ide-border-subtle)]">
      <button
        type="button"
        className="flex w-full items-center gap-1.5 px-3 py-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground hover:bg-[var(--ide-hover)]"
        onClick={() => {
          setExpanded((previous) => !previous)
        }}
        aria-expanded={expanded}
      >
        {expanded ? (
          <ChevronDown className="h-3 w-3" />
        ) : (
          <ChevronRight className="h-3 w-3" />
        )}
        {icon}
        <span className="truncate">{title}</span>
      </button>

      {expanded ? (
        <div className="pb-1">
          {loading ? (
            <p className="px-6 py-1 text-[11px] text-muted-foreground/70">Carregando...</p>
          ) : hasContent ? (
            children
          ) : (
            <p className="px-6 py-1 text-[11px] text-muted-foreground/70">{emptyMessage}</p>
          )}
        </div>
      ) : null}
    </section>
  )
}

export function IdeSapExplorer({
  serverId,
  state,
  localObjects,
  favoriteObjects,
  systemNamespaces,
  favoritePackageObjects,
  namespaceObjects,
  packageInventoryObjects = {},
  packageInventory = null,
  packageScopeRoots = [],
  packageScopeRootInput = "",
  packageScopePresets = [],
  loadingState = false,
  loadingLocalObjects = false,
  loadingSystemNamespaces = false,
  loadingFavoritePackages = false,
  loadingFavoriteObjects = false,
  loadingPackageInventory = false,
  loadingPackageObjects = {},
  loadingPackageInventoryObjects = {},
  loadingNamespaceObjects = {},
  onRefresh,
  onToggleFavoritePackage,
  onToggleFavoriteObject,
  onSelectWorkingPackage,
  onOpenFile,
  onLoadFavoritePackageObjects,
  onLoadNamespaceObjects,
  onLoadPackageInventoryObjects,
  onChangePackageScopeRootInput,
  onAddPackageScopeRoot,
  onRemovePackageScopeRoot,
  onApplyPackageScopeRoots,
  onTogglePackageScopePreset,
}: IdeSapExplorerProps) {
  const [expandedFavoritePackages, setExpandedFavoritePackages] = useState<Record<string, boolean>>({})
  const [expandedNamespaces, setExpandedNamespaces] = useState<Record<string, boolean>>({})
  const [expandedInventoryPackages, setExpandedInventoryPackages] = useState<Record<string, boolean>>({})

  const favoriteObjectByUri = useMemo(() => {
    const set = new Set<string>()
    for (const item of state.favoriteObjects) {
      set.add(item.uri)
    }
    return set
  }, [state.favoriteObjects])

  const favoritePackageByKey = useMemo(() => {
    const set = new Set<string>()
    for (const item of state.favoritePackages) {
      set.add(favoritePackageKey(item))
    }
    return set
  }, [state.favoritePackages])

  const packageInventoryObjectsByPackage = useMemo(() => {
    const map = new Map<string, NeuroAdtObjectSummary[]>()
    for (const entry of packageInventory?.objectsByPackage ?? []) {
      map.set(entry.packageName, entry.objects)
    }
    return map
  }, [packageInventory])

  const activeScopePresetKeys = useMemo(() => {
    const active = new Set<string>()
    for (const entry of packageScopeRoots) {
      const normalized = entry.trim()
      if (!normalized) {
        continue
      }
      active.add(normalized.toUpperCase())
    }
    return active
  }, [packageScopeRoots])

  return (
    <section className="flex h-full min-h-0 flex-col bg-[var(--ide-surface-1)]">
      <header className="flex items-center justify-between border-b border-[var(--ide-border-subtle)] px-3 py-1 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
        <span className="text-terminal-fg/75">SAP Explorer</span>
        <div className="flex items-center gap-2">
          <span className="max-w-[140px] truncate text-[10px] text-terminal-cyan" title={serverId ?? "Sem servidor ADT ativo"}>
            {serverId ?? "ADT inativo"}
          </span>
          {onRefresh ? (
            <button
              type="button"
              aria-label="Atualizar SAP Explorer"
              title="Atualizar SAP Explorer"
              onClick={onRefresh}
              className="rounded-[5px] border border-transparent p-1 text-muted-foreground transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)] hover:text-foreground"
            >
              <RefreshCw className="h-3 w-3" />
            </button>
          ) : null}
        </div>
      </header>

      <div className="border-b border-[var(--ide-border-subtle)] bg-[var(--ide-surface-0)]/55 px-3 py-1 text-[11px] text-muted-foreground">
        <div className="flex items-center gap-2">
          <Target className="h-3 w-3 text-terminal-gold" />
          <span className="truncate">Pacote de trabalho:</span>
          <span className="truncate text-terminal-fg" title={state.workingPackage ?? "Nenhum"}>
            {state.workingPackage ?? "Nenhum"}
          </span>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        <TreeSection
          title="Local Objects ($TMP)"
          icon={<Package className="h-3 w-3 text-terminal-gold" />}
          loading={loadingState || loadingLocalObjects}
          emptyMessage="Nenhum objeto local encontrado."
        >
          {localObjects.map((object) => (
            <TreeObjectRow
              key={object.uri}
              object={object}
              isFavorite={favoriteObjectByUri.has(object.uri)}
              isFocused={state.focusedObjectUri === object.uri}
              onOpenFile={onOpenFile}
              onToggleFavorite={onToggleFavoriteObject}
            />
          ))}
        </TreeSection>

        <TreeSection
          title="Favorite Packages"
          icon={<Package className="h-3 w-3 text-terminal-gold" />}
          loading={loadingState || loadingFavoritePackages}
          emptyMessage="Nenhum pacote favorito."
        >
          {state.favoritePackages.map((item) => {
            const itemKey = favoritePackageKey(item)
            const isExpanded = expandedFavoritePackages[itemKey] === true
            const objects = favoritePackageObjects[itemKey] ?? []
            const isLoading = loadingPackageObjects[itemKey] === true
            const isWorkingPackage = state.workingPackage === item.name

            return (
              <div key={itemKey}>
                <div className="group flex items-center gap-1.5 px-3 py-1 text-[11px] text-muted-foreground hover:bg-[var(--ide-hover)] hover:text-foreground">
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
                    onClick={() => {
                      setExpandedFavoritePackages((previous) => ({
                        ...previous,
                        [itemKey]: !isExpanded,
                      }))
                      if (!isExpanded && objects.length === 0 && !isLoading) {
                        onLoadFavoritePackageObjects(item)
                      }
                    }}
                  >
                    {isExpanded ? (
                      <ChevronDown className="h-3 w-3" />
                    ) : (
                      <ChevronRight className="h-3 w-3" />
                    )}
                    {iconForPackageKind(item.kind)}
                    <span className="truncate">{item.name}</span>
                    {isWorkingPackage ? (
                      <span className="rounded bg-[var(--ide-surface-3)] px-1 text-[10px] text-terminal-gold">
                        work
                      </span>
                    ) : null}
                  </button>

                  <button
                    type="button"
                    aria-label={`Selecionar ${item.name} como pacote de trabalho`}
                    title="Selecionar pacote de trabalho"
                    onClick={() => {
                      onSelectWorkingPackage(item.name)
                    }}
                    className="rounded border border-transparent p-0.5 text-terminal-gold transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)]"
                  >
                    <Target className="h-3 w-3" />
                  </button>

                  <button
                    type="button"
                    aria-label={`Remover ${item.name} dos favoritos`}
                    title="Remover dos favoritos"
                    onClick={() => {
                      onToggleFavoritePackage(item)
                    }}
                    className="rounded border border-transparent p-0.5 text-terminal-gold transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)]"
                  >
                    <Star className="h-3 w-3 fill-current" />
                  </button>
                </div>

                {isExpanded ? (
                  <div className="pl-4">
                    {isLoading ? (
                      <p className="px-3 py-1 text-[11px] text-muted-foreground/70">Carregando objetos...</p>
                    ) : objects.length === 0 ? (
                      <p className="px-3 py-1 text-[11px] text-muted-foreground/70">Nenhum objeto listado.</p>
                    ) : (
                      objects.map((object) => (
                        <TreeObjectRow
                          key={object.uri}
                          object={object}
                          isFavorite={favoriteObjectByUri.has(object.uri)}
                          isFocused={state.focusedObjectUri === object.uri}
                          onOpenFile={onOpenFile}
                          onToggleFavorite={onToggleFavoriteObject}
                        />
                      ))
                    )}
                  </div>
                ) : null}
              </div>
            )
          })}
        </TreeSection>

        <TreeSection
          title="Package Inventory (Recursive)"
          icon={<LibraryBig className="h-3 w-3 text-terminal-cyan" />}
          loading={loadingPackageInventory}
          emptyMessage="Nenhum pacote encontrado para os filtros."
        >
          <div className="px-3 pb-2">
            <div className="flex items-center gap-1.5">
              <input
                value={packageScopeRootInput}
                onChange={(event) => {
                  onChangePackageScopeRootInput?.(event.target.value)
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault()
                    onAddPackageScopeRoot?.()
                  }
                }}
                placeholder="/S4TAX/ ou Z*"
                className="h-7 min-w-0 flex-1 rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-0)] px-2 text-[11px] text-terminal-fg outline-none focus:border-terminal-cyan"
              />
              <button
                type="button"
                onClick={() => {
                  onAddPackageScopeRoot?.()
                }}
                className="h-7 shrink-0 rounded border border-[var(--ide-border-subtle)] px-2 text-[11px] text-terminal-cyan hover:bg-[var(--ide-hover)]"
              >
                Adicionar
              </button>
            </div>
            {packageScopeRoots.length > 0 ? (
              <div className="mt-2 flex flex-wrap gap-1">
                {packageScopeRoots.map((root) => (
                  <div
                    key={`scope-root-${root}`}
                    className="inline-flex items-center gap-1 rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-0)] px-1.5 py-0.5 text-[10px] text-terminal-fg"
                  >
                    <span className="max-w-[220px] truncate" title={root}>
                      {root}
                    </span>
                    <button
                      type="button"
                      onClick={() => {
                        onRemovePackageScopeRoot?.(root)
                      }}
                      className="rounded border border-transparent px-1 text-muted-foreground transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)] hover:text-foreground"
                    >
                      Remover
                    </button>
                  </div>
                ))}
              </div>
            ) : (
              <p className="mt-2 text-[10px] text-muted-foreground/70">
                Nenhum root ativo.
              </p>
            )}
            <div className="mt-2">
              <button
                type="button"
                onClick={() => {
                  onApplyPackageScopeRoots?.()
                }}
                className="h-7 rounded border border-[var(--ide-border-subtle)] px-2 text-[11px] text-terminal-cyan hover:bg-[var(--ide-hover)]"
              >
                Aplicar e salvar
              </button>
            </div>
            {packageScopePresets.length > 0 ? (
              <div className="mt-1 flex flex-wrap gap-1">
                {packageScopePresets.map((preset) => {
                  const key = preset.trim().toUpperCase()
                  const isActive = key.length > 0 && activeScopePresetKeys.has(key)
                  return (
                    <button
                      key={`scope-preset-${preset}`}
                      type="button"
                      onClick={() => {
                        onTogglePackageScopePreset?.(preset)
                      }}
                      className={cn(
                        "h-6 rounded border px-2 text-[10px] transition-colors",
                        isActive
                          ? "border-terminal-cyan bg-terminal-cyan/15 text-terminal-cyan"
                          : "border-[var(--ide-border-subtle)] text-muted-foreground hover:bg-[var(--ide-hover)] hover:text-foreground",
                      )}
                    >
                      {isActive ? "Remover " : "Adicionar "}
                      {preset}
                    </button>
                  )
                })}
              </div>
            ) : null}
            {packageInventory?.metadata ? (
              <p className="mt-1 text-[10px] text-muted-foreground/70">
                metadata: completo {packageInventory.metadata.isComplete ? "sim" : "nao"} | truncado{" "}
                {packageInventory.metadata.isTruncated ? "sim" : "nao"} | includeObjects{" "}
                {packageInventory.metadata.includeObjects ? "sim" : "nao"}
              </p>
            ) : null}
            {packageInventory?.roots && packageInventory.roots.length > 0 ? (
              <p className="mt-1 text-[10px] text-muted-foreground/70">
                filtros: {packageInventory.roots.join(", ")}
              </p>
            ) : null}
          </div>

          {(packageInventory?.packages ?? []).map((node) => {
            const rowKey = `${node.parentName ?? "__root__"}::${node.name}`
            const hasInventoryObjects = Object.prototype.hasOwnProperty.call(
              packageInventoryObjects,
              node.name,
            )
            const fallbackObjects = packageInventoryObjectsByPackage.get(node.name)
            const objects = hasInventoryObjects
              ? packageInventoryObjects[node.name] ?? []
              : fallbackObjects ?? []
            const isExpanded = expandedInventoryPackages[node.name] === true
            const isWorkingPackage = state.workingPackage === node.name
            const isFavorite = favoritePackageByKey.has(`package:${node.name.toUpperCase()}`)
            const isLoading = loadingPackageInventoryObjects[node.name] === true
            const hasObjectsLoaded = hasInventoryObjects || fallbackObjects !== undefined

            return (
              <div key={rowKey}>
                <div
                  className="group flex items-center gap-1.5 py-1 pr-2 text-[11px] text-muted-foreground hover:bg-[var(--ide-hover)] hover:text-foreground"
                  style={{ paddingLeft: `${12 + Math.max(0, node.depth) * 12}px` }}
                >
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
                    onClick={() => {
                      const nextExpanded = !isExpanded
                      setExpandedInventoryPackages((previous) => ({
                        ...previous,
                        [node.name]: nextExpanded,
                      }))
                      if (nextExpanded && !hasObjectsLoaded && !isLoading) {
                        onLoadPackageInventoryObjects?.(node.name)
                      }
                    }}
                  >
                    {isExpanded ? (
                      <ChevronDown className="h-3 w-3" />
                    ) : (
                      <ChevronRight className="h-3 w-3" />
                    )}
                    <Package className="h-3 w-3 text-terminal-gold" />
                    <span className="truncate">{node.name}</span>
                    <span className="text-[10px] text-muted-foreground/70">({node.objectCount})</span>
                    {isWorkingPackage ? (
                      <span className="rounded bg-[var(--ide-surface-3)] px-1 text-[10px] text-terminal-gold">
                        work
                      </span>
                    ) : null}
                  </button>

                  <button
                    type="button"
                    aria-label={`Selecionar ${node.name} como pacote de trabalho`}
                    title="Selecionar pacote de trabalho"
                    onClick={() => {
                      onSelectWorkingPackage(node.name)
                    }}
                    className="rounded border border-transparent p-0.5 text-terminal-gold transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)]"
                  >
                    <Target className="h-3 w-3" />
                  </button>

                  <button
                    type="button"
                    aria-label={isFavorite ? `Desfavoritar ${node.name}` : `Favoritar ${node.name}`}
                    title={isFavorite ? "Remover dos favoritos" : "Adicionar aos favoritos"}
                    onClick={() => {
                      onToggleFavoritePackage({
                        kind: "package",
                        name: node.name,
                      })
                    }}
                    className={cn(
                      "rounded border border-transparent p-0.5 transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)]",
                      isFavorite ? "text-terminal-gold" : "text-muted-foreground/70",
                    )}
                  >
                    <Star className={cn("h-3 w-3", isFavorite && "fill-current")} />
                  </button>
                </div>

                {isExpanded ? (
                  <div>
                    {isLoading ? (
                      <p className="px-6 py-1 text-[11px] text-muted-foreground/70">Carregando objetos...</p>
                    ) : objects.length === 0 ? (
                      <p className="px-6 py-1 text-[11px] text-muted-foreground/70">Nenhum objeto listado.</p>
                    ) : (
                      objects.map((object) => (
                        <TreeObjectRow
                          key={object.uri}
                          object={object}
                          isFavorite={favoriteObjectByUri.has(object.uri)}
                          isFocused={state.focusedObjectUri === object.uri}
                          onOpenFile={onOpenFile}
                          onToggleFavorite={onToggleFavoriteObject}
                        />
                      ))
                    )}
                  </div>
                ) : null}
              </div>
            )
          })}
        </TreeSection>

        <TreeSection
          title="Favorite Objects"
          icon={<Star className="h-3 w-3 text-terminal-gold" />}
          loading={loadingState || loadingFavoriteObjects}
          emptyMessage="Nenhum objeto favorito."
        >
          {(favoriteObjects.length > 0 ? favoriteObjects : state.favoriteObjects).map((object) => (
            <TreeObjectRow
              key={object.uri}
              object={object}
              isFavorite
              isFocused={state.focusedObjectUri === object.uri}
              onOpenFile={onOpenFile}
              onToggleFavorite={onToggleFavoriteObject}
            />
          ))}
        </TreeSection>

        <TreeSection
          title="System Library (namespaces)"
          icon={<LibraryBig className="h-3 w-3 text-terminal-cyan" />}
          loading={loadingState || loadingSystemNamespaces}
          emptyMessage="Nenhum namespace encontrado."
        >
          {systemNamespaces.map((namespace) => {
            const namespaceName = namespace.name
            const itemKey = `namespace:${namespaceName.toUpperCase()}`
            const isExpanded = expandedNamespaces[namespaceName] === true
            const objects = namespaceObjects[namespaceName] ?? []
            const isLoading = loadingNamespaceObjects[namespaceName] === true
            const isFavorite = favoritePackageByKey.has(itemKey)

            return (
              <div key={namespaceName}>
                <div className="group flex items-center gap-1.5 px-3 py-1 text-[11px] text-muted-foreground hover:bg-[var(--ide-hover)] hover:text-foreground">
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
                    onClick={() => {
                      setExpandedNamespaces((previous) => ({
                        ...previous,
                        [namespaceName]: !isExpanded,
                      }))
                      if (!isExpanded && objects.length === 0 && !isLoading) {
                        onLoadNamespaceObjects(namespaceName)
                      }
                    }}
                  >
                    {isExpanded ? (
                      <ChevronDown className="h-3 w-3" />
                    ) : (
                      <ChevronRight className="h-3 w-3" />
                    )}
                    <Boxes className="h-3 w-3 text-terminal-cyan" />
                    <span className="truncate">{namespaceName}</span>
                  </button>

                  <button
                    type="button"
                    aria-label={isFavorite ? `Desfavoritar ${namespaceName}` : `Favoritar ${namespaceName}`}
                    title={isFavorite ? "Remover namespace dos favoritos" : "Favoritar namespace"}
                    onClick={() => {
                      onToggleFavoritePackage({
                        kind: "namespace",
                        name: namespaceName,
                      })
                    }}
                    className={cn(
                      "rounded border border-transparent p-0.5 transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)]",
                      isFavorite ? "text-terminal-gold" : "text-muted-foreground/70",
                    )}
                  >
                    <Star className={cn("h-3 w-3", isFavorite && "fill-current")} />
                  </button>
                </div>

                {isExpanded ? (
                  <div className="pl-4">
                    {isLoading ? (
                      <p className="px-3 py-1 text-[11px] text-muted-foreground/70">Carregando objetos...</p>
                    ) : objects.length === 0 ? (
                      <p className="px-3 py-1 text-[11px] text-muted-foreground/70">Nenhum objeto listado.</p>
                    ) : (
                      objects.map((object) => (
                        <TreeObjectRow
                          key={object.uri}
                          object={object}
                          isFavorite={favoriteObjectByUri.has(object.uri)}
                          isFocused={state.focusedObjectUri === object.uri}
                          onOpenFile={onOpenFile}
                          onToggleFavorite={onToggleFavoriteObject}
                        />
                      ))
                    )}
                  </div>
                ) : null}
              </div>
            )
          })}
        </TreeSection>
      </div>
    </section>
  )
}
