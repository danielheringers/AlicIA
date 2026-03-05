import { useCallback, useEffect, useRef, useState } from "react"
import {
  codexWorkspaceCreateDirectory,
  codexWorkspaceListDirectory,
  codexWorkspaceRenameEntry,
  codexWorkspaceWriteFile,
  type CodexWorkspaceDirectoryEntry,
} from "@/lib/tauri-bridge"
import type { AliciaState } from "@/lib/alicia-types"
import type { RuntimeMethod } from "@/lib/application/runtime-types"
import { isRuntimeMethodSupported } from "@/lib/alicia-runtime-helpers"
import { createWorkspaceExplorerRequestGuard } from "@/hooks/workspace-explorer-request-guard"

type ExplorerDirectoryPathMap = Record<string, CodexWorkspaceDirectoryEntry[]>
type ExplorerDirectoryStatusMap = Record<string, boolean>

const EXPLORER_ROOT_KEY = "__root__"

interface UseWorkspaceExplorerControllerOptions {
  runtimeConnected: boolean
  runtimeWorkspace: string
  runtimeCapabilities: AliciaState["runtimeCapabilities"]
  setRuntimeWorkspace: (workspace: string) => void
  addMessage: (type: "system", content: string) => void
  refreshWorkspaceChanges: () => Promise<void>
  ensureWorkspaceWriteReady: (
    operation: string,
    requiredMethods: RuntimeMethod[],
  ) => boolean
  normalizeRelativeWorkspacePath: (rawPath: string) => string
  toEditorErrorMessage: (error: unknown) => string
  onOpenInEditor: (ref: string) => void
}

interface UseWorkspaceExplorerControllerResult {
  explorerRootPath: string
  explorerRootEntries: CodexWorkspaceDirectoryEntry[]
  explorerChildrenByPath: ExplorerDirectoryPathMap
  explorerLoadedPaths: ExplorerDirectoryStatusMap
  explorerLoadingPaths: ExplorerDirectoryStatusMap
  explorerTreeVersion: number
  explorerRootLoading: boolean
  resetWorkspaceExplorerState: () => void
  loadWorkspaceDirectory: (
    path?: string,
    options?: {
      silent?: boolean
      asRoot?: boolean
    },
  ) => Promise<Awaited<ReturnType<typeof codexWorkspaceListDirectory>> | null>
  refreshExplorerRoot: (options?: { silent?: boolean }) => Promise<
    Awaited<ReturnType<typeof codexWorkspaceListDirectory>> | null
  >
  handleLoadExplorerDirectory: (path: string) => Promise<void>
  handleCreateWorkspaceFile: (relativePath: string) => Promise<boolean>
  handleCreateWorkspaceFolder: (relativePath: string) => Promise<boolean>
  handleRenameWorkspaceEntry: (
    path: string,
    newName: string,
  ) => Promise<boolean | string>
}

function normalizeExplorerPath(path: string | null | undefined): string {
  return String(path ?? "")
    .replace(/\\/g, "/")
    .replace(/^\.\/+/, "")
    .replace(/^\/+/, "")
    .replace(/\/+$/, "")
}

function pathBasename(path: string): string {
  const normalized = normalizeExplorerPath(path)
  const parts = normalized.split("/").filter(Boolean)
  return parts.at(-1) ?? normalized
}

function pathDirname(path: string): string {
  const normalized = normalizeExplorerPath(path)
  const separator = normalized.lastIndexOf("/")
  if (separator < 0) {
    return ""
  }
  return normalized.slice(0, separator)
}

function sortExplorerEntries(
  entries: CodexWorkspaceDirectoryEntry[],
): CodexWorkspaceDirectoryEntry[] {
  return [...entries].sort((left, right) => {
    if (left.kind !== right.kind) {
      return left.kind === "directory" ? -1 : 1
    }
    return left.name.localeCompare(right.name, "pt-BR", { sensitivity: "base" })
  })
}

export function useWorkspaceExplorerController({
  runtimeConnected,
  runtimeWorkspace,
  runtimeCapabilities,
  setRuntimeWorkspace,
  addMessage,
  refreshWorkspaceChanges,
  ensureWorkspaceWriteReady,
  normalizeRelativeWorkspacePath,
  toEditorErrorMessage,
  onOpenInEditor,
}: UseWorkspaceExplorerControllerOptions): UseWorkspaceExplorerControllerResult {
  const [explorerRootPath, setExplorerRootPath] = useState("")
  const [explorerRootEntries, setExplorerRootEntries] = useState<
    CodexWorkspaceDirectoryEntry[]
  >([])
  const [explorerChildrenByPath, setExplorerChildrenByPath] =
    useState<ExplorerDirectoryPathMap>({})
  const [explorerLoadedPaths, setExplorerLoadedPaths] =
    useState<ExplorerDirectoryStatusMap>({})
  const [explorerLoadingPaths, setExplorerLoadingPaths] =
    useState<ExplorerDirectoryStatusMap>({})
  const [explorerTreeVersion, setExplorerTreeVersion] = useState(0)
  const [explorerRootLoading, setExplorerRootLoading] = useState(false)
  const explorerRequestGuardRef = useRef(createWorkspaceExplorerRequestGuard())

  const normalizeWorkspaceDirectoryEntries = useCallback(
    (entries: unknown): CodexWorkspaceDirectoryEntry[] => {
      if (!Array.isArray(entries)) {
        return []
      }

      const normalized: CodexWorkspaceDirectoryEntry[] = []
      for (const entry of entries) {
        if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
          continue
        }
        const record = entry as Record<string, unknown>
        const kind = record.kind === "directory" ? "directory" : "file"
        const rawPath =
          typeof record.path === "string" && record.path.trim().length > 0
            ? record.path
            : typeof record.name === "string"
              ? record.name
              : ""
        const path = normalizeExplorerPath(rawPath)
        if (!path) {
          continue
        }
        const name =
          typeof record.name === "string" && record.name.trim().length > 0
            ? record.name.trim()
            : pathBasename(path)
        if (typeof record.hasChildren === "boolean") {
          normalized.push({
            name,
            path,
            kind,
            hasChildren: record.hasChildren,
          })
          continue
        }
        normalized.push({
          name,
          path,
          kind,
        })
      }

      return sortExplorerEntries(normalized)
    },
    [],
  )

  const upsertExplorerEntry = useCallback(
    (parentPath: string, nextEntry: CodexWorkspaceDirectoryEntry) => {
      const normalizedParentPath = normalizeExplorerPath(parentPath)
      if (!normalizedParentPath) {
        setExplorerRootEntries((previous) => {
          const merged = [...previous.filter((entry) => entry.path !== nextEntry.path), nextEntry]
          return sortExplorerEntries(merged)
        })
        return
      }

      setExplorerChildrenByPath((previous) => {
        const currentEntries = previous[normalizedParentPath] ?? []
        const merged = [
          ...currentEntries.filter((entry) => entry.path !== nextEntry.path),
          nextEntry,
        ]
        return {
          ...previous,
          [normalizedParentPath]: sortExplorerEntries(merged),
        }
      })
    },
    [],
  )

  const markExplorerDirectoryHasChildren = useCallback((directoryPath: string) => {
    const normalizedPath = normalizeExplorerPath(directoryPath)
    if (!normalizedPath) {
      return
    }

    const updateEntry = (entry: CodexWorkspaceDirectoryEntry) =>
      entry.path === normalizedPath ? { ...entry, hasChildren: true } : entry

    setExplorerRootEntries((previous) => previous.map(updateEntry))
    setExplorerChildrenByPath((previous) => {
      const next: ExplorerDirectoryPathMap = {}
      for (const [path, entries] of Object.entries(previous)) {
        next[path] = entries.map(updateEntry)
      }
      return next
    })
  }, [])

  const loadWorkspaceDirectory = useCallback(
    async (
      path?: string,
      options: {
        silent?: boolean
        asRoot?: boolean
      } = {},
    ) => {
      if (!isRuntimeMethodSupported(runtimeCapabilities, "workspace.directory.list")) {
        if (!options.silent) {
          addMessage(
            "system",
            '[explorer] Listagem de diretorio indisponivel neste runtime. Metodo nao suportado: workspace.directory.list.',
          )
        }
        return null
      }

      const requestPath = normalizeExplorerPath(path)
      const asRoot = options.asRoot === true
      const loadingKey = !requestPath || asRoot ? EXPLORER_ROOT_KEY : requestPath
      const requestToken = explorerRequestGuardRef.current.begin(loadingKey)

      setExplorerLoadingPaths((previous) => ({ ...previous, [loadingKey]: true }))
      if (!requestPath || asRoot) {
        setExplorerRootLoading(true)
      }

      try {
        const response = await codexWorkspaceListDirectory(
          requestPath ? { path: requestPath } : undefined,
        )

        if (!explorerRequestGuardRef.current.isCurrent(requestToken)) {
          return null
        }

        const responsePath = normalizeExplorerPath(response.path)
        const entries = normalizeWorkspaceDirectoryEntries(response.entries)

        if (!requestPath || asRoot) {
          const nextRootPath = asRoot ? requestPath || responsePath : responsePath
          setExplorerRootPath(nextRootPath)
          setExplorerRootEntries(entries)
          setExplorerChildrenByPath({})
          setExplorerLoadedPaths({
            [EXPLORER_ROOT_KEY]: true,
          })
          setExplorerTreeVersion((previous) => previous + 1)
        } else {
          setExplorerChildrenByPath((previous) => ({
            ...previous,
            [requestPath]: entries,
          }))
          setExplorerLoadedPaths((previous) => ({
            ...previous,
            [loadingKey]: true,
          }))
        }

        if (typeof response.cwd === "string" && response.cwd.trim().length > 0) {
          const nextWorkspace = response.cwd.trim()
          if (nextWorkspace !== runtimeWorkspace) {
            setRuntimeWorkspace(nextWorkspace)
          }
        }

        return response
      } catch (error) {
        if (!explorerRequestGuardRef.current.isCurrent(requestToken)) {
          return null
        }

        if (!options.silent) {
          addMessage(
            "system",
            `[explorer] Falha ao carregar diretorio: ${String(error)}`,
          )
        }
        return null
      } finally {
        if (explorerRequestGuardRef.current.isCurrent(requestToken)) {
          setExplorerLoadingPaths((previous) => ({ ...previous, [loadingKey]: false }))
          if (!requestPath || asRoot) {
            setExplorerRootLoading(false)
          }
        }
      }
    },
    [
      addMessage,
      normalizeWorkspaceDirectoryEntries,
      runtimeCapabilities,
      runtimeWorkspace,
      setRuntimeWorkspace,
    ],
  )

  const refreshExplorerRoot = useCallback(
    async (options: { silent?: boolean } = {}) => {
      return loadWorkspaceDirectory(undefined, options)
    },
    [loadWorkspaceDirectory],
  )

  const handleLoadExplorerDirectory = useCallback(
    async (path: string) => {
      await loadWorkspaceDirectory(path)
    },
    [loadWorkspaceDirectory],
  )

  const handleCreateWorkspaceFile = useCallback(
    async (relativePath: string) => {
      if (!ensureWorkspaceWriteReady("criar arquivos no workspace", ["workspace.file.write"])) {
        return false
      }

      let normalizedPath = ""
      try {
        normalizedPath = normalizeRelativeWorkspacePath(relativePath)
      } catch (error) {
        addMessage(
          "system",
          `[explorer] Caminho invalido para novo arquivo: ${toEditorErrorMessage(error)}`,
        )
        return false
      }

      try {
        await codexWorkspaceWriteFile({
          path: normalizedPath,
          content: "",
        })
        const parentPath = pathDirname(normalizedPath)
        upsertExplorerEntry(parentPath, {
          name: pathBasename(normalizedPath),
          path: normalizedPath,
          kind: "file",
          hasChildren: false,
        })
        markExplorerDirectoryHasChildren(parentPath)
        await refreshWorkspaceChanges()
        addMessage("system", `[explorer] Arquivo criado: ${normalizedPath}`)
        onOpenInEditor(normalizedPath)
        return true
      } catch (error) {
        addMessage(
          "system",
          `[explorer] Falha ao criar arquivo "${normalizedPath}": ${toEditorErrorMessage(error)}`,
        )
        return false
      }
    },
    [
      addMessage,
      ensureWorkspaceWriteReady,
      markExplorerDirectoryHasChildren,
      normalizeRelativeWorkspacePath,
      onOpenInEditor,
      refreshWorkspaceChanges,
      toEditorErrorMessage,
      upsertExplorerEntry,
    ],
  )

  const handleCreateWorkspaceFolder = useCallback(
    async (relativePath: string) => {
      if (
        !ensureWorkspaceWriteReady("criar pastas no workspace", [
          "workspace.directory.create",
        ])
      ) {
        return false
      }

      let normalizedPath = ""
      try {
        normalizedPath = normalizeRelativeWorkspacePath(relativePath)
      } catch (error) {
        addMessage(
          "system",
          `[explorer] Caminho invalido para nova pasta: ${toEditorErrorMessage(error)}`,
        )
        return false
      }

      try {
        await codexWorkspaceCreateDirectory({
          path: normalizedPath,
        })
        const parentPath = pathDirname(normalizedPath)
        upsertExplorerEntry(parentPath, {
          name: pathBasename(normalizedPath),
          path: normalizedPath,
          kind: "directory",
          hasChildren: false,
        })
        markExplorerDirectoryHasChildren(parentPath)
        await refreshWorkspaceChanges()
        addMessage("system", `[explorer] Pasta criada: ${normalizedPath}`)
        return true
      } catch (error) {
        addMessage(
          "system",
          `[explorer] Falha ao criar pasta "${normalizedPath}": ${toEditorErrorMessage(error)}`,
        )
        return false
      }
    },
    [
      addMessage,
      ensureWorkspaceWriteReady,
      markExplorerDirectoryHasChildren,
      normalizeRelativeWorkspacePath,
      refreshWorkspaceChanges,
      toEditorErrorMessage,
      upsertExplorerEntry,
    ],
  )

  const handleRenameWorkspaceEntry = useCallback(
    async (path: string, newName: string) => {
      if (
        !ensureWorkspaceWriteReady("renomear itens no workspace", [
          "workspace.entry.rename",
        ])
      ) {
        return false
      }

      let normalizedPath = ""
      try {
        normalizedPath = normalizeRelativeWorkspacePath(path)
      } catch (error) {
        addMessage(
          "system",
          `[explorer] Caminho invalido para renomear item: ${toEditorErrorMessage(error)}`,
        )
        return false
      }

      const trimmedName = newName.trim()
      if (!trimmedName) {
        addMessage("system", "[explorer] Informe um novo nome para renomear.")
        return false
      }
      if (trimmedName === "." || trimmedName === "..") {
        addMessage("system", "[explorer] Nome invalido para renomeacao.")
        return false
      }
      if (trimmedName.includes("/") || trimmedName.includes("\\")) {
        addMessage(
          "system",
          "[explorer] Renomeacao so permite alterar o nome dentro do mesmo diretorio.",
        )
        return false
      }

      const currentName = pathBasename(normalizedPath)
      if (trimmedName === currentName) {
        return normalizedPath
      }

      try {
        const response = await codexWorkspaceRenameEntry({
          path: normalizedPath,
          newName: trimmedName,
        })
        const nextPath = normalizeRelativeWorkspacePath(response.newPath)
        await refreshExplorerRoot({ silent: true })
        await refreshWorkspaceChanges()
        addMessage("system", `[explorer] Item renomeado: ${normalizedPath} -> ${nextPath}`)
        return nextPath
      } catch (error) {
        addMessage(
          "system",
          `[explorer] Falha ao renomear "${normalizedPath}": ${toEditorErrorMessage(error)}`,
        )
        return false
      }
    },
    [
      addMessage,
      ensureWorkspaceWriteReady,
      normalizeRelativeWorkspacePath,
      refreshExplorerRoot,
      refreshWorkspaceChanges,
      toEditorErrorMessage,
    ],
  )

  const resetWorkspaceExplorerState = useCallback(() => {
    explorerRequestGuardRef.current.invalidateAll()
    setExplorerRootPath("")
    setExplorerRootEntries([])
    setExplorerChildrenByPath({})
    setExplorerLoadedPaths({})
    setExplorerLoadingPaths({})
    setExplorerRootLoading(false)
  }, [])

  useEffect(() => {
    explorerRequestGuardRef.current.invalidateAll()
    setExplorerLoadingPaths({})
    setExplorerRootLoading(false)
  }, [runtimeWorkspace])
  useEffect(() => {
    if (!runtimeConnected) {
      return
    }
    void refreshExplorerRoot({ silent: true })
  }, [refreshExplorerRoot, runtimeConnected, runtimeWorkspace])

  return {
    explorerRootPath,
    explorerRootEntries,
    explorerChildrenByPath,
    explorerLoadedPaths,
    explorerLoadingPaths,
    explorerTreeVersion,
    explorerRootLoading,
    resetWorkspaceExplorerState,
    loadWorkspaceDirectory,
    refreshExplorerRoot,
    handleLoadExplorerDirectory,
    handleCreateWorkspaceFile,
    handleCreateWorkspaceFolder,
    handleRenameWorkspaceEntry,
  }
}




