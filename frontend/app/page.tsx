"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { Bot, Minimize2 } from "lucide-react"
import { TitleBar } from "@/components/alicia/title-bar"
import { Sidebar } from "@/components/alicia/sidebar"
import { ConversationPane } from "@/components/alicia/conversation-pane"
import { StatusBar } from "@/components/alicia/status-bar"
import { ModelPicker } from "@/components/alicia/model-picker"
import { PermissionsPanel } from "@/components/alicia/permissions-panel"
import { McpPanel } from "@/components/alicia/mcp-panel"
import { AppsPanel } from "@/components/alicia/apps-panel"
import { AdtPanel } from "@/components/alicia/adt-panel"
import { SessionPicker } from "@/components/alicia/session-picker"
import { ReviewMode } from "@/components/alicia/review-mode"
import { TerminalPane } from "@/components/alicia/terminal-pane"
import { SourceEditorPanel } from "@/components/alicia/source-editor-panel"
import { AliciaDesktopShell } from "@/components/ide/desktop-shell"
import { IdeSapExplorer } from "@/components/ide/sap-explorer"
import { type IdeSidebarView } from "@/components/ide/activity-bar"
import {
  IdePanelToolbar,
  type IdeShellMode,
  type IdeViewMode,
} from "@/components/ide/panel-toolbar"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { type AliciaState } from "@/lib/alicia-types"
import {
  codexApprovalRespond,
  codexRuntimeSessionStart,
  codexRuntimeStatus,
  codexRuntimeSessionStop,
  codexUserInputRespond,
  isTauriRuntime,
  pickImageFile,
  pickMentionFile,
  gitCommitApprovedReview,
  codexWorkspaceChanges,
  codexWorkspaceCreateDirectory,
  codexWorkspaceListDirectory,
  codexWorkspaceReadFile,
  codexWorkspaceRenameEntry,
  codexWorkspaceWriteFile,
  neuroAdtExplorerStateGet,
  neuroAdtExplorerStatePatch,
  neuroAdtListPackageInventory,
  neuroAdtListNamespaces,
  neuroAdtListObjects,
  neuroGetSource,
  neuroAdtServerList,
  neuroUpdateSource,
  pickWorkspaceFolder,
  terminalResize,
  terminalWrite,
  type ApprovalDecision,
  type CodexModel,
  type CodexWorkspaceDirectoryEntry,
  type NeuroAdtExplorerState,
  type NeuroAdtFavoritePackageItem,
  type NeuroAdtListObjectsRequest,
  type NeuroAdtPackageInventoryResponse,
  type NeuroAdtNamespaceSummary,
  type NeuroAdtObjectSummary,
  type NeuroRuntimeCommandError,
  type RuntimeMethod,
  type RuntimeCodexConfig,
} from "@/lib/tauri-bridge"
import {
  INITIAL_ALICIA_STATE,
  isRuntimeMethodSupported,
  mapDiffFilesToFileChanges,
  parseUnifiedDiffFiles,
  type ApprovalRequestState,
  type DiffFileView,
  type Message,
  type MessageChannel,
  readModelsCache,
  type RuntimeState,
  type TerminalTab,
  timestampNow,
  type TurnDiffState,
  type TurnPlanState,
  type UserInputRequestState,
} from "@/lib/alicia-runtime-helpers"
import {
  createCodexEventHandler,
  createTerminalEventHandler,
} from "@/lib/alicia-event-handlers"
import { useAliciaTerminalRuntime } from "@/hooks/use-alicia-terminal-runtime"
import { useAliciaActions } from "@/hooks/use-alicia-actions"
import { useAliciaBootstrap } from "@/hooks/use-alicia-bootstrap"
import { useAliciaRuntimeCore } from "@/hooks/use-alicia-runtime-core"
import { useIsMobile } from "@/hooks/use-mobile"
import {
  parseReasoningSystemMessage,
  parseUsageSystemMessage,
  type UsageStats,
} from "@/lib/runtime-statusline"
import {
  encodeAgentSpawnerPayload,
  mergeAgentSpawnerPayloads,
  parseAgentSpawnerPayload,
} from "@/lib/agent-spawner-events"
import {
  AbapSourceResolverError,
  resolveAbapSourceRef,
} from "@/lib/abap-source-resolver"
import {
  selectBoundAdtServerId,
} from "@/lib/adt-server-binding"
import {
  normalizeWorkspacePathForRoot,
  routeSourceEditorRef,
  type SourceEditorRefKind,
} from "@/lib/source-editor-ref-routing"
import {
  resolveDesktopEscAction,
  transitionDesktopShellMode,
} from "@/lib/desktop-shell-state"

interface SourceEditorState {
  visible: boolean
  refKind: SourceEditorRefKind | null
  adtServerId: string | null
  objectUri: string | null
  workspaceRoot: string | null
  displayName: string | null
  language: string
  source: string
  etag: string | null
  dirty: boolean
  loading: boolean
  saving: boolean
  error: string | null
}

const INITIAL_SOURCE_EDITOR_STATE: SourceEditorState = {
  visible: false,
  refKind: null,
  adtServerId: null,
  objectUri: null,
  workspaceRoot: null,
  displayName: null,
  language: "plaintext",
  source: "",
  etag: null,
  dirty: false,
  loading: false,
  saving: false,
  error: null,
}

type MainContentTab = "chat" | "editor" | "terminal"

type ExplorerDirectoryPathMap = Record<string, CodexWorkspaceDirectoryEntry[]>
type ExplorerDirectoryStatusMap = Record<string, boolean>
type SapObjectMap = Record<string, NeuroAdtObjectSummary[]>
type SapLoadingMap = Record<string, boolean>
type SapObjectMapByServer = Record<string, SapObjectMap>
type SapLoadingMapByServer = Record<string, SapLoadingMap>
type SapPackageRootsByServer = Record<string, string[]>
type SapPackageRootInputByServer = Record<string, string>

const EXPLORER_ROOT_KEY = "__root__"
const SAP_TMP_PACKAGE = "$TMP"
const SAP_PACKAGE_SCOPE_PRESETS = ["/S4TAX/", "/1BEA/", "Z*", "Y*", SAP_TMP_PACKAGE]
const INITIAL_SAP_LOADING_STATE = {
  state: false,
  localObjects: false,
  favoriteObjects: false,
  favoritePackages: false,
  systemNamespaces: false,
  packageInventory: false,
}

const INITIAL_SAP_EXPLORER_STATE: NeuroAdtExplorerState = {
  workingPackage: null,
  focusedObjectUri: null,
  packageScopeRoots: [],
  favoritePackages: [],
  favoriteObjects: [],
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

function normalizeSapPackageScopeRoot(raw: string): string | null {
  const normalized = String(raw ?? "").trim()
  return normalized.length > 0 ? normalized : null
}

function sapServerScopeKey(serverId: string | null | undefined): string {
  const normalized = String(serverId ?? "").trim()
  return normalized.length > 0 ? normalized : "__none__"
}

function deriveSapNamespacesFromPackageScopeRoots(
  roots: string[] | null | undefined,
): NeuroAdtNamespaceSummary[] {
  if (!Array.isArray(roots) || roots.length === 0) {
    return []
  }

  const seen = new Set<string>()
  const derived: NeuroAdtNamespaceSummary[] = []
  for (const root of roots) {
    const normalized = String(root ?? "").trim()
    if (!/^\/[^/\s]+\/$/.test(normalized)) {
      continue
    }
    const dedupeKey = normalized.toUpperCase()
    if (seen.has(dedupeKey)) {
      continue
    }
    seen.add(dedupeKey)
    derived.push({
      name: normalized,
      packageName: null,
    })
  }
  return derived
}

function mergeSapNamespaces(
  backendNamespaces: NeuroAdtNamespaceSummary[] | null | undefined,
  derivedNamespaces: NeuroAdtNamespaceSummary[] | null | undefined,
): NeuroAdtNamespaceSummary[] {
  const mergedByKey = new Map<string, NeuroAdtNamespaceSummary>()
  const pushNamespace = (
    candidate: NeuroAdtNamespaceSummary | null | undefined,
    preferPackageName: boolean,
  ) => {
    const rawName = String(candidate?.name ?? "").trim()
    if (!rawName) {
      return
    }
    const key = rawName.toUpperCase()
    const packageName = candidate?.packageName ?? null
    const existing = mergedByKey.get(key)
    if (!existing) {
      mergedByKey.set(key, {
        name: rawName,
        packageName,
      })
      return
    }
    if (preferPackageName && packageName && !existing.packageName) {
      mergedByKey.set(key, {
        ...existing,
        packageName,
      })
    }
  }

  for (const namespace of backendNamespaces ?? []) {
    pushNamespace(namespace, true)
  }
  for (const namespace of derivedNamespaces ?? []) {
    pushNamespace(namespace, false)
  }

  return Array.from(mergedByKey.values()).sort((left, right) =>
    left.name.localeCompare(right.name, "pt-BR", { sensitivity: "base" }),
  )
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

export default function AliciaTerminal() {
  const isMobile = useIsMobile()
  const [initializing, setInitializing] = useState(true)
  const [initializingStatus, setInitializingStatus] = useState(
    "Initializing Alicia runtime...",
  )
  const [bootLogs, setBootLogs] = useState<string[]>([])
  const [messages, setMessages] = useState<Message[]>([])
  const [reviewMessages, setReviewMessages] = useState<Message[]>([])
  const [isThinking, setIsThinking] = useState(false)
  const [pendingApprovals, setPendingApprovals] = useState<ApprovalRequestState[]>([])
  const [pendingUserInput, setPendingUserInput] = useState<UserInputRequestState | null>(null)
  const [turnDiff, setTurnDiff] = useState<TurnDiffState | null>(null)
  const [turnPlan, setTurnPlan] = useState<TurnPlanState | null>(null)
  const [pendingImages, setPendingImages] = useState<string[]>([])
  const [pendingMentions, setPendingMentions] = useState<string[]>([])
  const [sessionPickerMode, setSessionPickerMode] = useState<
    "resume" | "fork" | "list"
  >("list")
  const [sessionsPanelLoading, setSessionsPanelLoading] = useState(false)
  const [terminalTabs, setTerminalTabs] = useState<TerminalTab[]>([])
  const [activeTerminalId, setActiveTerminalId] = useState<number | null>(null)
  const [aliciaState, setAliciaState] = useState<AliciaState>(INITIAL_ALICIA_STATE)
  const [availableModels, setAvailableModels] = useState<CodexModel[]>([])
  const [modelsLoading, setModelsLoading] = useState(false)
  const [modelsError, setModelsError] = useState<string | null>(null)
  const [modelsCachedAt, setModelsCachedAt] = useState<number | null>(null)
  const [modelsFromCache, setModelsFromCache] = useState(false)
  const [activeAdtServerId, setActiveAdtServerId] = useState<string | null>(null)
  const [sapExplorerState, setSapExplorerState] = useState<NeuroAdtExplorerState>(
    INITIAL_SAP_EXPLORER_STATE,
  )
  const [sapLocalObjects, setSapLocalObjects] = useState<NeuroAdtObjectSummary[]>([])
  const [sapFavoriteObjects, setSapFavoriteObjects] = useState<NeuroAdtObjectSummary[]>([])
  const [sapSystemNamespaces, setSapSystemNamespaces] = useState<NeuroAdtNamespaceSummary[]>(
    [],
  )
  const [sapFavoritePackageObjects, setSapFavoritePackageObjects] =
    useState<SapObjectMap>({})
  const [sapNamespaceObjects, setSapNamespaceObjects] = useState<SapObjectMap>({})
  const [sapPackageScopeRootsByServer, setSapPackageScopeRootsByServer] =
    useState<SapPackageRootsByServer>({})
  const [sapPackageScopeRootInputByServer, setSapPackageScopeRootInputByServer] =
    useState<SapPackageRootInputByServer>({})
  const [sapPackageInventory, setSapPackageInventory] =
    useState<NeuroAdtPackageInventoryResponse | null>(null)
  const [sapPackageInventoryObjectsByServer, setSapPackageInventoryObjectsByServer] =
    useState<SapObjectMapByServer>({})
  const [sapLoading, setSapLoading] = useState(INITIAL_SAP_LOADING_STATE)
  const [sapPackageObjectsLoading, setSapPackageObjectsLoading] =
    useState<SapLoadingMap>({})
  const [sapPackageInventoryObjectsLoadingByServer, setSapPackageInventoryObjectsLoadingByServer] =
    useState<SapLoadingMapByServer>({})
  const [sapNamespaceObjectsLoading, setSapNamespaceObjectsLoading] =
    useState<SapLoadingMap>({})
  const [sourceEditor, setSourceEditor] = useState<SourceEditorState>(
    INITIAL_SOURCE_EDITOR_STATE,
  )
  const [activeMainTab, setActiveMainTab] = useState<MainContentTab>("chat")
  const [desktopSidebarVisible, setDesktopSidebarVisible] = useState(true)
  const [desktopSidebarView, setDesktopSidebarView] =
    useState<IdeSidebarView>("agent")
  const [desktopViewMode, setDesktopViewMode] = useState<IdeViewMode>("split")
  const [desktopTerminalVisible, setDesktopTerminalVisible] = useState(true)
  const [desktopShellMode, setDesktopShellMode] = useState<IdeShellMode>("normal")
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
  const [switchingWorkspaceFolder, setSwitchingWorkspaceFolder] = useState(false)
  const [runtime, setRuntime] = useState<RuntimeState>({
    connected: isTauriRuntime(),
    state: "idle",
    sessionId: null,
    pid: null,
    workspace: ".",
  })

  const idRef = useRef(1)
  const scrollRef = useRef<HTMLDivElement>(null)
  const shouldAutoScrollRef = useRef(true)
  const terminalContainerRef = useRef<HTMLDivElement>(null)
  const codexUnlistenRef = useRef<(() => void) | null>(null)
  const terminalUnlistenRef = useRef<(() => void) | null>(null)
  const runtimeConfigRef = useRef<RuntimeCodexConfig | null>(null)
  const threadIdRef = useRef<string | null>(null)
  const reviewMessagesBySessionRef = useRef<Map<string, Message[]>>(new Map())
  const activeReviewSessionKeyRef = useRef<string | null>(null)
  const terminalBuffersRef = useRef<Map<number, string>>(new Map())
  const activeTerminalRef = useRef<number | null>(null)
  const xtermRef = useRef<import("xterm").Terminal | null>(null)
  const fitAddonRef = useRef<import("xterm-addon-fit").FitAddon | null>(null)
  const seenEventSeqRef = useRef<Set<number>>(new Set())
  const streamedAgentTextRef = useRef<Map<string, string>>(new Map())
  const bootstrappedRef = useRef(false)
  const autoTerminalCreatedRef = useRef(false)
  const reviewRoutingRef = useRef(false)
  const turnDiffFilesRef = useRef<DiffFileView[]>([])
  const wasReviewThinkingRef = useRef(false)
  const sourceOperationSeqRef = useRef(0)
  const sourceBaselineRef = useRef("")
  const sourceEditorRef = useRef<SourceEditorState>(INITIAL_SOURCE_EDITOR_STATE)
  const activeAdtServerIdRef = useRef<string | null>(null)
  const lastAutoRefreshedAdtServerIdRef = useRef<string | null>(null)
  const sapPackageScopeRootsByServerRef = useRef<SapPackageRootsByServer>({})
  const refreshSapExplorerRef = useRef<() => Promise<void>>(async () => {})
  const desktopLayoutSnapshotRef = useRef({
    sidebarVisible: true,
    terminalVisible: true,
  })

  const [isReviewComplete, setIsReviewComplete] = useState(false)

  useEffect(() => {
    const cached = readModelsCache()
    if (!cached || cached.data.length === 0) {
      return
    }
    setAvailableModels(cached.data)
    setModelsCachedAt(cached.cachedAt)
    setModelsFromCache(true)
  }, [])

  useEffect(() => {
    const nowThinking = isThinking && reviewRoutingRef.current
    if (nowThinking) {
      wasReviewThinkingRef.current = true
      setIsReviewComplete(false)
    } else if (wasReviewThinkingRef.current && !isThinking) {
      wasReviewThinkingRef.current = false
      setIsReviewComplete(true)
    }
  }, [isThinking])

  useEffect(() => {
    activeAdtServerIdRef.current = activeAdtServerId
  }, [activeAdtServerId])

  useEffect(() => {
    sapPackageScopeRootsByServerRef.current = sapPackageScopeRootsByServer
  }, [sapPackageScopeRootsByServer])

  const activeSapServerKey = useMemo(
    () => sapServerScopeKey(activeAdtServerId),
    [activeAdtServerId],
  )
  const sapPackageScopeRoots = useMemo(() => {
    const scopedRoots = sapPackageScopeRootsByServer[activeSapServerKey]
    return Array.isArray(scopedRoots) ? scopedRoots : []
  }, [activeSapServerKey, sapPackageScopeRootsByServer])
  const sapPackageScopeRootInput = sapPackageScopeRootInputByServer[activeSapServerKey] ?? ""
  const sapPackageInventoryObjects = sapPackageInventoryObjectsByServer[activeSapServerKey] ?? {}
  const sapPackageInventoryObjectsLoading =
    sapPackageInventoryObjectsLoadingByServer[activeSapServerKey] ?? {}

  useEffect(() => {
    if (isMobile || desktopShellMode !== "normal") {
      return
    }
    desktopLayoutSnapshotRef.current = {
      sidebarVisible: desktopSidebarVisible,
      terminalVisible: desktopTerminalVisible,
    }
  }, [desktopShellMode, desktopSidebarVisible, desktopTerminalVisible, isMobile])

  const applyDesktopShellModeTransition = useCallback(
    (nextMode: IdeShellMode) => {
      const transition = transitionDesktopShellMode({
        isMobile,
        currentMode: desktopShellMode,
        nextMode,
        layout: {
          sidebarVisible: desktopSidebarVisible,
          terminalVisible: desktopTerminalVisible,
        },
        snapshot: desktopLayoutSnapshotRef.current,
      })

      desktopLayoutSnapshotRef.current = transition.snapshot
      if (!transition.changed) {
        return
      }

      setDesktopShellMode(transition.mode)
      setDesktopSidebarVisible(transition.layout.sidebarVisible)
      setDesktopTerminalVisible(transition.layout.terminalVisible)
    },
    [desktopShellMode, desktopSidebarVisible, desktopTerminalVisible, isMobile],
  )

  const handleDesktopShellModeChange = useCallback(
    (nextMode: IdeShellMode) => {
      applyDesktopShellModeTransition(nextMode)
    },
    [applyDesktopShellModeTransition],
  )

  useEffect(() => {
    if (isMobile) {
      return
    }

    const handleEsc = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) {
        return
      }

      const escAction = resolveDesktopEscAction({
        isMobile,
        hasActivePanel: Boolean(aliciaState.activePanel),
        shellMode: desktopShellMode,
      })

      if (escAction === "close-panel") {
        event.preventDefault()
        setAliciaState((previous) =>
          previous.activePanel ? { ...previous, activePanel: null } : previous,
        )
        return
      }

      if (escAction === "exit-zen" || escAction === "exit-focus") {
        event.preventDefault()
        applyDesktopShellModeTransition("normal")
      }
    }

    window.addEventListener("keydown", handleEsc)
    return () => {
      window.removeEventListener("keydown", handleEsc)
    }
  }, [
    aliciaState.activePanel,
    applyDesktopShellModeTransition,
    desktopShellMode,
    isMobile,
  ])

  const setSourceEditorWithRef = useCallback(
    (
      updater:
        | SourceEditorState
        | ((previous: SourceEditorState) => SourceEditorState),
    ) => {
      setSourceEditor((previous) => {
        const next =
          typeof updater === "function"
            ? (updater as (previous: SourceEditorState) => SourceEditorState)(
                previous,
              )
            : updater
        sourceEditorRef.current = next
        return next
      })
    },
    [],
  )

  const nextMessageId = useCallback(() => {
    idRef.current += 1
    return idRef.current
  }, [])

  const addMessage = useCallback(
    (
      type: Message["type"],
      content: string,
      channel: MessageChannel = "chat",
    ) => {
      if (!content.trim()) {
        return
      }

      const timestamp = timestampNow()
      const setTargetMessages = channel === "review" ? setReviewMessages : setMessages

      setTargetMessages((prev) => {
        if (type !== "system" || prev.length === 0) {
          return [
            ...prev,
            {
              id: nextMessageId(),
              channel,
              type,
              content,
              timestamp,
            },
          ]
        }

        const incomingSpawner = parseAgentSpawnerPayload(content)
        const last = prev[prev.length - 1]
        if (!incomingSpawner || last.type !== "system") {
          return [
            ...prev,
            {
              id: nextMessageId(),
              channel,
              type,
              content,
              timestamp,
            },
          ]
        }

        const previousSpawner = parseAgentSpawnerPayload(last.content)
        if (!previousSpawner) {
          return [
            ...prev,
            {
              id: nextMessageId(),
              channel,
              type,
              content,
              timestamp,
            },
          ]
        }

        const mergedPayload = mergeAgentSpawnerPayloads(
          previousSpawner,
          incomingSpawner,
        )
        const mergedContent = encodeAgentSpawnerPayload(mergedPayload)

        return [
          ...prev.slice(0, -1),
          {
            ...last,
            content: mergedContent,
            timestamp,
          },
        ]
      })
    },
    [nextMessageId],
  )

  const turnDiffFiles = useMemo(
    () => parseUnifiedDiffFiles(turnDiff?.diff ?? ""),
    [turnDiff],
  )
  const activeSessionId = useMemo(
    () => aliciaState.sessions.find((session) => session.active)?.id ?? null,
    [aliciaState.sessions],
  )
  const activeReviewSessionKey = threadIdRef.current ?? activeSessionId ?? "__sessionless__"

  useEffect(() => {
    if (activeReviewSessionKeyRef.current === null) {
      activeReviewSessionKeyRef.current = activeReviewSessionKey
      const initialMessages =
        reviewMessagesBySessionRef.current.get(activeReviewSessionKey) ?? []
      setReviewMessages(initialMessages)
      return
    }

    if (activeReviewSessionKeyRef.current === activeReviewSessionKey) {
      return
    }

    const previousKey = activeReviewSessionKeyRef.current
    if (previousKey) {
      reviewMessagesBySessionRef.current.set(previousKey, reviewMessages)
    }

    activeReviewSessionKeyRef.current = activeReviewSessionKey
    const nextMessages =
      reviewMessagesBySessionRef.current.get(activeReviewSessionKey) ?? []
    setReviewMessages(nextMessages)
  }, [activeReviewSessionKey, reviewMessages])

  useEffect(() => {
    reviewMessagesBySessionRef.current.set(activeReviewSessionKey, reviewMessages)
  }, [activeReviewSessionKey, reviewMessages])

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
      if (
        !isRuntimeMethodSupported(
          aliciaState.runtimeCapabilities,
          "workspace.directory.list",
        )
      ) {
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

      setExplorerLoadingPaths((previous) => ({ ...previous, [loadingKey]: true }))
      if (!requestPath || asRoot) {
        setExplorerRootLoading(true)
      }

      try {
        const response = await codexWorkspaceListDirectory(
          requestPath ? { path: requestPath } : undefined,
        )
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
          setRuntime((previous) =>
            previous.workspace === nextWorkspace
              ? previous
              : {
                  ...previous,
                  workspace: nextWorkspace,
                },
          )
        }

        return response
      } catch (error) {
        if (!options.silent) {
          addMessage(
            "system",
            `[explorer] Falha ao carregar diretorio: ${String(error)}`,
          )
        }
        return null
      } finally {
        setExplorerLoadingPaths((previous) => ({ ...previous, [loadingKey]: false }))
        if (!requestPath || asRoot) {
          setExplorerRootLoading(false)
        }
      }
    },
    [addMessage, aliciaState.runtimeCapabilities, normalizeWorkspaceDirectoryEntries],
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

  const refreshWorkspaceChanges = useCallback(async () => {
    const normalizeStatus = (value: unknown): AliciaState["fileChanges"][number]["status"] => {
      const status = String(value ?? "").trim().toLowerCase()
      if (status === "added" || status === "a" || status === "new") return "added"
      if (status === "deleted" || status === "d" || status === "removed" || status === "missing") {
        return "deleted"
      }
      if (status === "renamed" || status === "r") return "renamed"
      if (status === "copied" || status === "c") return "copied"
      if (status === "untracked" || status === "??") return "untracked"
      if (status === "unmerged" || status === "u" || status === "uu") return "unmerged"
      return "modified"
    }

    try {
      const response = await codexWorkspaceChanges()
      const filesSource = Array.isArray((response as { files?: unknown[] }).files)
        ? (response as { files: unknown[] }).files
        : []

      const fileChanges = filesSource
        .map((entry) => {
          if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
            return null
          }

          const record = entry as Record<string, unknown>
          const name =
            (typeof record.path === "string" && record.path.trim()) ||
            (typeof record.name === "string" && record.name.trim()) ||
            ""

          if (!name) {
            return null
          }

          const fromPath =
            typeof record.fromPath === "string" && record.fromPath.trim()
              ? record.fromPath.trim()
              : undefined

          if (fromPath) {
            return {
              name,
              status: normalizeStatus(record.status),
              fromPath,
            }
          }

          return {
            name,
            status: normalizeStatus(record.status),
          }
        })
        .filter(
          (entry): entry is AliciaState["fileChanges"][number] => entry !== null,
        )

      setAliciaState((previous) => ({
        ...previous,
        fileChanges,
      }))
      return
    } catch {
      const fallback = mapDiffFilesToFileChanges(turnDiffFilesRef.current)
      setAliciaState((previous) => ({
        ...previous,
        fileChanges: fallback,
      }))
    }
  }, [])

  useEffect(() => {
    turnDiffFilesRef.current = turnDiffFiles
    void refreshWorkspaceChanges()
  }, [turnDiffFiles, refreshWorkspaceChanges])

  useEffect(() => {
    if (!runtime.connected) {
      return
    }
    void refreshExplorerRoot({ silent: true })
  }, [runtime.connected, runtime.workspace, refreshExplorerRoot])

  const statusSignals = useMemo(() => {
    let usage: UsageStats | null = null
    let reasoning: string | null = null

    for (const message of messages) {
      if (message.type !== "system") {
        continue
      }

      const parsedUsage = parseUsageSystemMessage(message.content)
      if (parsedUsage) {
        usage = parsedUsage
        continue
      }

      const parsedReasoning = parseReasoningSystemMessage(message.content)
      if (parsedReasoning) {
        reasoning = parsedReasoning
      }
    }

    return { usage, reasoning }
  }, [messages])
  const isNearBottom = useCallback((container: HTMLDivElement) => {
    const thresholdPx = 96
    const remaining =
      container.scrollHeight - container.scrollTop - container.clientHeight
    return remaining <= thresholdPx
  }, [])

  const scrollConversationToBottom = useCallback((force = false) => {
    const container = scrollRef.current
    if (!container) {
      return
    }

    if (!force && !shouldAutoScrollRef.current) {
      return
    }

    container.scrollTop = container.scrollHeight
    shouldAutoScrollRef.current = true
  }, [])

  useEffect(() => {
    const container = scrollRef.current
    if (!container) {
      return
    }

    const syncAutoScroll = () => {
      shouldAutoScrollRef.current = isNearBottom(container)
    }

    scrollConversationToBottom(true)
    syncAutoScroll()
    container.addEventListener("scroll", syncAutoScroll, { passive: true })

    return () => {
      container.removeEventListener("scroll", syncAutoScroll)
    }
  }, [initializing, isNearBottom, scrollConversationToBottom])

  useEffect(() => {
    const frameId = window.requestAnimationFrame(() => {
      scrollConversationToBottom()
    })

    return () => {
      window.cancelAnimationFrame(frameId)
    }
  }, [
    messages,
    isThinking,
    pendingApprovals.length,
    turnDiff?.turnId,
    turnPlan?.turnId,
    scrollConversationToBottom,
  ])

  const appendBootLog = useCallback((message: string) => {
    const now = new Date()
    const stamp = `${String(now.getHours()).padStart(2, "0")}:${String(now
      .getMinutes())
      .padStart(2, "0")}:${String(now.getSeconds()).padStart(2, "0")}`
    setBootLogs((prev) => [...prev.slice(-19), `${stamp} ${message}`])
  }, [])

  const {
    setActiveSessionEntry,
    refreshThreadList,
    refreshMcpServers,
    refreshAppsAndAuth,
    refreshModelsCatalog,
    openModelPanel,
    ensureRuntimeSession,
    createTerminalTab,
    closeTerminalTab,
    currentModelLabel,
  } = useAliciaRuntimeCore({
    addMessage,
    runtime,
    setRuntime,
    aliciaState,
    setAliciaState,
    availableModels,
    setAvailableModels,
    modelsLoading,
    setModelsLoading,
    setModelsError,
    setModelsCachedAt,
    setModelsFromCache,
    threadIdRef,
    seenEventSeqRef,
    streamedAgentTextRef,
    terminalBuffersRef,
    xtermRef,
    setTerminalTabs,
    setActiveTerminalId,
  })

  const openSessionPanel = useCallback(
    (mode: "resume" | "fork" | "list") => {
      setSessionPickerMode(mode)
      setAliciaState((prev) => ({ ...prev, activePanel: "sessions" }))
      setSessionsPanelLoading(true)
      void refreshThreadList({
        activeThreadId: threadIdRef.current,
        notifyOnError: false,
      }).finally(() => {
        setSessionsPanelLoading(false)
      })
    },
    [refreshThreadList],
  )

  const handleCodexEvent = useMemo(
    () =>
      createCodexEventHandler({
        addMessage,
        setRuntime,
        setIsThinking,
        setPendingApprovals,
        setPendingUserInput,
        setTurnDiff,
        setTurnPlan,
        seenEventSeqRef,
        streamedAgentTextRef,
        threadIdRef,
        reviewRoutingRef,
        onRefreshWorkspaceChanges: () => {
          void refreshWorkspaceChanges()
        },
      }),
    [addMessage, refreshWorkspaceChanges],
  )

  const handleTerminalEvent = useMemo(
    () =>
      createTerminalEventHandler({
        setTerminalTabs,
        terminalBuffersRef,
        activeTerminalRef,
        xtermRef,
      }),
    [],
  )

  useAliciaBootstrap({
    bootstrappedRef,
    autoTerminalCreatedRef,
    codexUnlistenRef,
    terminalUnlistenRef,
    runtimeConfigRef,
    handleCodexEvent,
    handleTerminalEvent,
    addMessage,
    setRuntime,
    setAliciaState,
    setInitializingStatus,
    setInitializing,
    setActiveSessionEntry,
    ensureRuntimeSession,
    refreshModelsCatalog,
    refreshThreadList,
    refreshMcpServers,
    refreshAppsAndAuth,
    createTerminalTab,
    onBootLog: appendBootLog,
  })

  const sapContextSnapshot = useMemo(
    () => ({
      serverId: activeAdtServerId,
      workPackage: sapExplorerState.workingPackage ?? null,
      focusedObjectUri: sapExplorerState.focusedObjectUri ?? null,
    }),
    [
      activeAdtServerId,
      sapExplorerState.focusedObjectUri,
      sapExplorerState.workingPackage,
    ],
  )

  const handleAdtServerSelectionChange = useCallback(
    (
      nextServerId: string | null,
      options?: { refreshExplorer?: boolean },
    ) => {
      setActiveAdtServerId((previous) => {
        if (previous === nextServerId) {
          if (nextServerId && options?.refreshExplorer) {
            lastAutoRefreshedAdtServerIdRef.current = nextServerId
            void refreshSapExplorerRef.current()
          }
          return previous
        }
        lastAutoRefreshedAdtServerIdRef.current = null
        return nextServerId
      })
    },
    [],
  )

  const {
    handleSubmit,
    handleSlashCommand,
    handleModelSelect,
    handlePermissionSelect,
    handleSessionSelect,
    sessionActionPending,
  } = useAliciaActions({
    addMessage,
    aliciaState,
    ensureRuntimeSession,
    pendingImages,
    pendingMentions,
    setPendingImages,
    setPendingMentions,
    setMessages,
    setPendingApprovals,
    setPendingUserInput,
    setTurnDiff,
    setTurnPlan,
    turnDiff,
    setIsThinking,
    threadIdRef,
    openModelPanel,
    openSessionPanel,
    refreshMcpServers,
    refreshAppsAndAuth,
    refreshThreadList,
    setAliciaState,
    setRuntime,
    runtimeConfigRef,
    availableModels,
    reviewRoutingRef,
    refreshWorkspaceChanges,
    onAdtServerSelectionChange: handleAdtServerSelectionChange,
    sapContextSnapshot,
  })

  const handleTerminalResize = useCallback(
    async (terminalId: number, cols: number, rows: number) => {
      try {
        await terminalResize(terminalId, cols, rows)
      } catch {
        // best effort
      }
    },
    [],
  )

  const handleTerminalWrite = useCallback(
    async (terminalId: number, data: string) => {
      try {
        await terminalWrite(terminalId, data)
      } catch {
        // best effort
      }
    },
    [],
  )

  useAliciaTerminalRuntime({
    initializing,
    layoutMode: isMobile ? "mobile" : "desktop",
    activeTerminalId,
    activeTerminalRef,
    terminalContainerRef,
    terminalBuffersRef,
    xtermRef,
    fitAddonRef,
    onTerminalResize: handleTerminalResize,
    onTerminalWrite: handleTerminalWrite,
  })

  const handleApprovalDecision = useCallback(
    async (actionId: string, decision: ApprovalDecision) => {
      try {
        await codexApprovalRespond({ actionId, decision })
      } catch (error) {
        addMessage("system", `[approval] failed: ${String(error)}`)
      }
    },
    [addMessage],
  )

  const handleUserInputDecision = useCallback(
    async (response: {
      actionId: string
      decision: "submit" | "cancel"
      answers?: Record<string, { answers: string[] }>
    }) => {
      try {
        await codexUserInputRespond({
          actionId: response.actionId,
          decision: response.decision,
          answers: response.answers,
        })
        setPendingUserInput((previous) => {
          if (!previous || previous.actionId === response.actionId) {
            return null
          }
          return previous
        })
      } catch (error) {
        addMessage("system", `[user input] failed: ${String(error)}`)
      }
    },
    [addMessage],
  )
  const handleCommitApprovedReview = useCallback(
    async (payload: {
      approvedPaths: string[]
      message: string
      comments: Record<string, string>
    }) => {
      const approvedPaths = payload.approvedPaths
        .map((entry) => entry.trim())
        .filter((entry) => entry.length > 0)

      if (approvedPaths.length === 0) {
        addMessage("system", "[review] no approved files selected for commit", "review")
        return
      }

      const changeByPath = new Map(
        aliciaState.fileChanges.map((entry) => [entry.name, entry]),
      )
      const workspaceConflicts = aliciaState.fileChanges
        .filter((entry) => entry.status === "unmerged")
        .map((entry) => entry.name)
      if (workspaceConflicts.length > 0) {
        addMessage(
          "system",
          `[review] resolve conflicts before commit: ${workspaceConflicts.join(", ")}`,
          "review",
        )
        return
      }

      const expandedPaths = Array.from(
        new Set(
          approvedPaths.flatMap((path) => {
            const change = changeByPath.get(path)
            if (
              change &&
              (change.status === "renamed" || change.status === "copied") &&
              typeof change.fromPath === "string" &&
              change.fromPath.trim().length > 0
            ) {
              return [path, change.fromPath.trim()]
            }
            return [path]
          }),
        ),
      )

      const commitMessage = payload.message.trim()
      if (!commitMessage) {
        addMessage("system", "[review] commit message is required", "review")
        return
      }

      const reviewNotes = approvedPaths
        .map((path) => {
          const note = payload.comments[path]?.trim()
          return note ? `${path}: ${note}` : null
        })
        .filter((entry): entry is string => Boolean(entry))
      const activeSessionCwd =
        aliciaState.sessions.find((session) => session.active)?.cwd?.trim() || null

      try {
        const response = await gitCommitApprovedReview({
          paths: expandedPaths,
          message: commitMessage,
          cwd: activeSessionCwd ?? undefined,
        })

        if (!response.success) {
          const addError = response.add.stderr.trim()
          const commitError = response.commit.stderr.trim()
          const failure = addError || commitError || "git add/commit failed"
          throw new Error(failure)
        }

        addMessage(
          "system",
          `[review] committed ${approvedPaths.length} approved file(s): ${approvedPaths.join(", ")}`,
          "review",
        )
        if (reviewNotes.length > 0) {
          addMessage("system", `[review] notes\n${reviewNotes.join("\n")}`, "review")
        }
      } catch (error) {
        addMessage("system", `[review] commit failed: ${String(error)}`, "review")
      } finally {
        await refreshWorkspaceChanges()
      }
    },
    [addMessage, aliciaState.fileChanges, aliciaState.sessions, refreshWorkspaceChanges],
  )

  const toEditorErrorMessage = useCallback((error: unknown): string => {
    if (error instanceof AbapSourceResolverError) {
      if (error.code === "ambiguous") {
        const candidates = Array.isArray(error.details?.candidates)
          ? (error.details?.candidates as string[]).slice(0, 3)
          : []
        if (candidates.length > 0) {
          return `${error.message} Candidatos: ${candidates.join(", ")}.`
        }
      }
      return error.message
    }

    if (error && typeof error === "object") {
      const candidate = error as Partial<NeuroRuntimeCommandError>
      if (typeof candidate.message === "string" && typeof candidate.code === "string") {
        return `Falha (${candidate.code}): ${candidate.message}`
      }
    }

    if (error instanceof Error) {
      return error.message
    }

    return String(error)
  }, [])

  const beginSourceOperation = useCallback(() => {
    sourceOperationSeqRef.current += 1
    return sourceOperationSeqRef.current
  }, [])

  const inferDisplayNameFromPath = useCallback((path: string): string => {
    const normalized = path.replace(/\\/g, "/")
    const parts = normalized.split("/").filter(Boolean)
    return parts.at(-1) ?? normalized
  }, [])

  const supportsRuntimeMethod = useCallback(
    (method: RuntimeMethod) =>
      isRuntimeMethodSupported(aliciaState.runtimeCapabilities, method),
    [aliciaState.runtimeCapabilities],
  )

  const buildUnsupportedEditorMessage = useCallback(
    (operation: string, methods: RuntimeMethod[]) => {
      const message = `Editor de codigo: operacao "${operation}" indisponivel neste runtime. Metodos nao suportados: ${methods.join(", ")}.`
      addMessage("system", message)
      return message
    },
    [addMessage],
  )

  const getUnsupportedRuntimeMethods = useCallback(
    (methods: RuntimeMethod[]): RuntimeMethod[] =>
      methods.filter((method) => !supportsRuntimeMethod(method)),
    [supportsRuntimeMethod],
  )

  const supportsSapExplorer = useCallback(
    () =>
      supportsRuntimeMethod("neuro.adt.explorer.state.get") &&
      supportsRuntimeMethod("neuro.adt.explorer.state.patch") &&
      supportsRuntimeMethod("neuro.adt.list.objects"),
    [supportsRuntimeMethod],
  )

  const supportsSapPackageInventory = useCallback(
    () => supportsRuntimeMethod("neuro.adt.list.package_inventory"),
    [supportsRuntimeMethod],
  )

  const listSapObjects = useCallback(
    async (request: NeuroAdtListObjectsRequest) => {
      const requestServerId =
        request.serverId !== undefined ? request.serverId : activeAdtServerId
      return neuroAdtListObjects({
        ...request,
        serverId: requestServerId ?? null,
      })
    },
    [activeAdtServerId],
  )

  const loadSapExplorerState = useCallback(async (): Promise<NeuroAdtExplorerState | null> => {
    if (!supportsSapExplorer()) {
      return null
    }
    const requestServerId = activeAdtServerIdRef.current
    setSapLoading((previous) => ({ ...previous, state: true }))
    try {
      const nextState = await neuroAdtExplorerStateGet(requestServerId)
      if (activeAdtServerIdRef.current !== requestServerId) {
        return null
      }
      setSapExplorerState(nextState)
      const scopeKey = sapServerScopeKey(requestServerId)
      setSapPackageScopeRootsByServer((previous) => ({
        ...previous,
        [scopeKey]: nextState.packageScopeRoots,
      }))
      setSapPackageScopeRootInputByServer((previous) => ({
        ...previous,
        [scopeKey]: "",
      }))
      return nextState
    } catch {
      if (activeAdtServerIdRef.current === requestServerId) {
        const scopeKey = sapServerScopeKey(requestServerId)
        setSapPackageScopeRootsByServer((previous) => ({
          ...previous,
          [scopeKey]: [],
        }))
        setSapPackageScopeRootInputByServer((previous) => ({
          ...previous,
          [scopeKey]: "",
        }))
      }
      return null
    } finally {
      if (activeAdtServerIdRef.current === requestServerId) {
        setSapLoading((previous) => ({ ...previous, state: false }))
      }
    }
  }, [supportsSapExplorer])

  const loadSapLocalObjects = useCallback(async () => {
    if (!supportsSapExplorer()) {
      setSapLocalObjects([])
      return
    }
    const requestServerId = activeAdtServerIdRef.current
    setSapLoading((previous) => ({ ...previous, localObjects: true }))
    try {
      const response = await listSapObjects({
        scope: "local_objects",
        packageName: SAP_TMP_PACKAGE,
        maxResults: 120,
        serverId: requestServerId,
      })
      if (activeAdtServerIdRef.current !== requestServerId) {
        return
      }
      setSapLocalObjects(response.objects)
    } catch {
      if (activeAdtServerIdRef.current === requestServerId) {
        setSapLocalObjects([])
      }
    } finally {
      if (activeAdtServerIdRef.current === requestServerId) {
        setSapLoading((previous) => ({ ...previous, localObjects: false }))
      }
    }
  }, [listSapObjects, supportsSapExplorer])

  const loadSapFavoriteObjects = useCallback(async () => {
    if (!supportsSapExplorer()) {
      setSapFavoriteObjects([])
      return
    }
    const requestServerId = activeAdtServerIdRef.current
    setSapLoading((previous) => ({ ...previous, favoriteObjects: true }))
    try {
      const response = await listSapObjects({
        scope: "favorite_objects",
        maxResults: 150,
        serverId: requestServerId,
      })
      if (activeAdtServerIdRef.current !== requestServerId) {
        return
      }
      setSapFavoriteObjects(response.objects)
    } catch {
      if (activeAdtServerIdRef.current === requestServerId) {
        setSapFavoriteObjects([])
      }
    } finally {
      if (activeAdtServerIdRef.current === requestServerId) {
        setSapLoading((previous) => ({ ...previous, favoriteObjects: false }))
      }
    }
  }, [listSapObjects, supportsSapExplorer])

  const loadSapSystemNamespaces = useCallback(async (scopeRootsOverride?: string[] | null) => {
    if (!supportsSapExplorer()) {
      setSapSystemNamespaces([])
      return
    }
    const requestServerId = activeAdtServerIdRef.current
    const scopeKey = sapServerScopeKey(requestServerId)
    const scopedRoots = sapPackageScopeRootsByServerRef.current[scopeKey] ?? []
    const derivedNamespaces = deriveSapNamespacesFromPackageScopeRoots(
      scopeRootsOverride ?? scopedRoots,
    )
    setSapLoading((previous) => ({ ...previous, systemNamespaces: true }))
    try {
      let backendNamespaces: NeuroAdtNamespaceSummary[] = []
      const response = await listSapObjects({
        scope: "system_library",
        maxResults: 5000,
        serverId: requestServerId,
      })
      if (activeAdtServerIdRef.current !== requestServerId) {
        return
      }
      if (Array.isArray(response.namespaces) && response.namespaces.length > 0) {
        backendNamespaces = response.namespaces
      } else {
        backendNamespaces = await neuroAdtListNamespaces(
          null,
          requestServerId,
        )
        if (activeAdtServerIdRef.current !== requestServerId) {
          return
        }
      }

      setSapSystemNamespaces(
        mergeSapNamespaces(backendNamespaces, derivedNamespaces),
      )
    } catch {
      try {
        const backendNamespaces = await neuroAdtListNamespaces(
          null,
          requestServerId,
        )
        if (activeAdtServerIdRef.current !== requestServerId) {
          return
        }
        setSapSystemNamespaces(
          mergeSapNamespaces(backendNamespaces, derivedNamespaces),
        )
      } catch {
        if (activeAdtServerIdRef.current === requestServerId) {
          setSapSystemNamespaces(mergeSapNamespaces([], derivedNamespaces))
        }
      }
    } finally {
      if (activeAdtServerIdRef.current === requestServerId) {
        setSapLoading((previous) => ({ ...previous, systemNamespaces: false }))
      }
    }
  }, [listSapObjects, supportsSapExplorer])

  const loadSapFavoritePackageObjects = useCallback(
    async (item: NeuroAdtFavoritePackageItem) => {
      if (!supportsSapExplorer()) {
        return
      }
      const requestServerId = activeAdtServerIdRef.current
      const itemKey = `${item.kind}:${item.name.toUpperCase()}`
      setSapPackageObjectsLoading((previous) => ({ ...previous, [itemKey]: true }))
      try {
        const response = await listSapObjects({
          scope: "favorite_packages",
          packageKind: item.kind,
          packageName: item.kind === "package" ? item.name : undefined,
          namespace: item.kind === "namespace" ? item.name : undefined,
          maxResults: 150,
          serverId: requestServerId,
        })
        if (activeAdtServerIdRef.current !== requestServerId) {
          return
        }
        setSapFavoritePackageObjects((previous) => ({
          ...previous,
          [itemKey]: response.objects,
        }))
      } catch {
        if (activeAdtServerIdRef.current !== requestServerId) {
          return
        }
        setSapFavoritePackageObjects((previous) => ({
          ...previous,
          [itemKey]: [],
        }))
      } finally {
        if (activeAdtServerIdRef.current === requestServerId) {
          setSapPackageObjectsLoading((previous) => ({ ...previous, [itemKey]: false }))
        }
      }
    },
    [listSapObjects, supportsSapExplorer],
  )

  const loadSapNamespaceObjects = useCallback(
    async (namespace: string) => {
      if (!supportsSapExplorer()) {
        return
      }
      const normalized = namespace.trim()
      if (!normalized) {
        return
      }
      const requestServerId = activeAdtServerIdRef.current
      setSapNamespaceObjectsLoading((previous) => ({ ...previous, [normalized]: true }))
      try {
        const response = await listSapObjects({
          scope: "system_library",
          namespace: normalized,
          maxResults: 150,
          serverId: requestServerId,
        })
        if (activeAdtServerIdRef.current !== requestServerId) {
          return
        }
        setSapNamespaceObjects((previous) => ({
          ...previous,
          [normalized]: response.objects,
        }))
      } catch {
        if (activeAdtServerIdRef.current !== requestServerId) {
          return
        }
        setSapNamespaceObjects((previous) => ({
          ...previous,
          [normalized]: [],
        }))
      } finally {
        if (activeAdtServerIdRef.current === requestServerId) {
          setSapNamespaceObjectsLoading((previous) => ({ ...previous, [normalized]: false }))
        }
      }
    },
    [listSapObjects, supportsSapExplorer],
  )

  const loadSapPackageInventoryObjects = useCallback(
    async (packageName: string) => {
      if (!supportsSapExplorer()) {
        return
      }
      const normalizedPackageName = packageName.trim()
      if (!normalizedPackageName) {
        return
      }
      const requestServerId = activeAdtServerIdRef.current
      const scopeKey = sapServerScopeKey(requestServerId)
      setSapPackageInventoryObjectsLoadingByServer((previous) => ({
        ...previous,
        [scopeKey]: {
          ...(previous[scopeKey] ?? {}),
          [normalizedPackageName]: true,
        },
      }))
      try {
        const response = await listSapObjects({
          scope: "favorite_packages",
          packageKind: "package",
          packageName: normalizedPackageName,
          maxResults: 180,
          serverId: requestServerId,
        })
        if (activeAdtServerIdRef.current !== requestServerId) {
          return
        }
        setSapPackageInventoryObjectsByServer((previous) => ({
          ...previous,
          [scopeKey]: {
            ...(previous[scopeKey] ?? {}),
            [normalizedPackageName]: response.objects,
          },
        }))
      } catch {
        if (activeAdtServerIdRef.current !== requestServerId) {
          return
        }
        setSapPackageInventoryObjectsByServer((previous) => ({
          ...previous,
          [scopeKey]: {
            ...(previous[scopeKey] ?? {}),
            [normalizedPackageName]: [],
          },
        }))
      } finally {
        if (activeAdtServerIdRef.current === requestServerId) {
          setSapPackageInventoryObjectsLoadingByServer((previous) => ({
            ...previous,
            [scopeKey]: {
              ...(previous[scopeKey] ?? {}),
              [normalizedPackageName]: false,
            },
          }))
        }
      }
    },
    [listSapObjects, supportsSapExplorer],
  )

  const loadSapPackageInventory = useCallback(
    async (
      scopeRootsOverride?: string[] | null,
    ): Promise<NeuroAdtPackageInventoryResponse | null> => {
    if (!supportsSapPackageInventory()) {
      setSapPackageInventory(null)
      return null
    }

    const requestServerId = activeAdtServerIdRef.current
    const scopeKey = sapServerScopeKey(requestServerId)
    const scopeRoots = Array.isArray(scopeRootsOverride)
      ? scopeRootsOverride
      : sapPackageScopeRootsByServerRef.current[scopeKey] ?? []
    if (scopeRoots.length === 0) {
      setSapPackageInventory(null)
      setSapPackageInventoryObjectsByServer((previous) => ({
        ...previous,
        [scopeKey]: {},
      }))
      setSapPackageInventoryObjectsLoadingByServer((previous) => ({
        ...previous,
        [scopeKey]: {},
      }))
      return null
    }

    setSapLoading((previous) => ({ ...previous, packageInventory: true }))
    try {
      const response = await neuroAdtListPackageInventory({
        roots: scopeRoots,
        includeSubpackages: true,
        includeObjects: false,
        maxPackages: 800,
        maxObjectsPerPackage: 80,
        serverId: requestServerId,
      })
      if (activeAdtServerIdRef.current !== requestServerId) {
        return null
      }
      setSapPackageInventory(response)
      setSapPackageInventoryObjectsByServer((previous) => ({
        ...previous,
        [scopeKey]: {},
      }))
      setSapPackageInventoryObjectsLoadingByServer((previous) => ({
        ...previous,
        [scopeKey]: {},
      }))
      return response
    } catch {
      if (activeAdtServerIdRef.current === requestServerId) {
        setSapPackageInventory(null)
        setSapPackageInventoryObjectsByServer((previous) => ({
          ...previous,
          [scopeKey]: {},
        }))
        setSapPackageInventoryObjectsLoadingByServer((previous) => ({
          ...previous,
          [scopeKey]: {},
        }))
      }
      return null
    } finally {
      if (activeAdtServerIdRef.current === requestServerId) {
        setSapLoading((previous) => ({ ...previous, packageInventory: false }))
      }
    }
  }, [supportsSapPackageInventory])

  const refreshSapExplorer = useCallback(async () => {
    if (!supportsSapExplorer()) {
      setSapExplorerState(INITIAL_SAP_EXPLORER_STATE)
      setSapLocalObjects([])
      setSapFavoriteObjects([])
      setSapSystemNamespaces([])
      setSapFavoritePackageObjects({})
      setSapNamespaceObjects({})
      setSapPackageScopeRootsByServer({})
      setSapPackageScopeRootInputByServer({})
      setSapPackageInventory(null)
      setSapPackageInventoryObjectsByServer({})
      setSapLoading(INITIAL_SAP_LOADING_STATE)
      setSapPackageObjectsLoading({})
      setSapPackageInventoryObjectsLoadingByServer({})
      setSapNamespaceObjectsLoading({})
      return
    }

    const state = await loadSapExplorerState()
    const inventoryRoots = state ? state.packageScopeRoots : undefined
    await Promise.all([
      loadSapLocalObjects(),
      loadSapFavoriteObjects(),
      loadSapSystemNamespaces(inventoryRoots),
      loadSapPackageInventory(inventoryRoots),
    ])
    if (state) {
      setSapFavoritePackageObjects({})
      setSapNamespaceObjects({})
    }
  }, [
    loadSapExplorerState,
    loadSapFavoriteObjects,
    loadSapLocalObjects,
    loadSapPackageInventory,
    loadSapSystemNamespaces,
    supportsSapExplorer,
  ])

  useEffect(() => {
    refreshSapExplorerRef.current = refreshSapExplorer
  }, [refreshSapExplorer])

  const patchSapExplorerState = useCallback(
    async (patch: {
      workingPackage?: string | null
      focusedObjectUri?: string | null
      setPackageScopeRoots?: string[] | null
      toggleFavoritePackage?: NeuroAdtFavoritePackageItem
      toggleFavoriteObject?: NeuroAdtObjectSummary
    }) => {
      if (!supportsSapExplorer()) {
        return
      }
      const requestServerId = activeAdtServerIdRef.current

      setSapLoading((previous) => ({ ...previous, state: true }))
      try {
        const nextState = await neuroAdtExplorerStatePatch(
          {
            workingPackage: patch.workingPackage,
            focusedObjectUri: patch.focusedObjectUri,
            setPackageScopeRoots: patch.setPackageScopeRoots,
            toggleFavoritePackage: patch.toggleFavoritePackage,
            toggleFavoriteObject: patch.toggleFavoriteObject
              ? {
                  uri: patch.toggleFavoriteObject.uri,
                  name: patch.toggleFavoriteObject.name,
                  objectType: patch.toggleFavoriteObject.objectType ?? null,
                  package: patch.toggleFavoriteObject.package ?? null,
                }
              : undefined,
          },
          requestServerId,
        )
        if (activeAdtServerIdRef.current !== requestServerId) {
          return
        }
        setSapExplorerState(nextState)
        if (patch.setPackageScopeRoots !== undefined) {
          const scopeKey = sapServerScopeKey(requestServerId)
          setSapPackageScopeRootsByServer((previous) => ({
            ...previous,
            [scopeKey]: nextState.packageScopeRoots,
          }))
          setSapPackageScopeRootInputByServer((previous) => ({
            ...previous,
            [scopeKey]: "",
          }))
          void loadSapSystemNamespaces(nextState.packageScopeRoots)
        }
        if (patch.toggleFavoriteObject) {
          void loadSapFavoriteObjects()
        }
        if (patch.toggleFavoritePackage) {
          const itemKey = `${patch.toggleFavoritePackage.kind}:${patch.toggleFavoritePackage.name.toUpperCase()}`
          setSapFavoritePackageObjects((previous) => {
            const next = { ...previous }
            delete next[itemKey]
            return next
          })
        }
      } catch {
        // optional capability / eventual consistency with backend
      } finally {
        if (activeAdtServerIdRef.current === requestServerId) {
          setSapLoading((previous) => ({ ...previous, state: false }))
        }
      }
    },
    [loadSapFavoriteObjects, loadSapSystemNamespaces, supportsSapExplorer],
  )

  const setSapPackageScopeRootInput = useCallback((value: string) => {
    const requestServerId = activeAdtServerIdRef.current
    const scopeKey = sapServerScopeKey(requestServerId)
    setSapPackageScopeRootInputByServer((previous) => ({
      ...previous,
      [scopeKey]: value,
    }))
  }, [])

  const addSapPackageScopeRoot = useCallback(() => {
    setSapPackageScopeRootInputByServer((previous) => {
      const next = { ...previous }
      const rawValue = next[activeSapServerKey] ?? ""
      const normalizedRoot = normalizeSapPackageScopeRoot(rawValue)
      if (!normalizedRoot) {
        return previous
      }
      setSapPackageScopeRootsByServer((rootsByServer) => {
        const currentRoots = rootsByServer[activeSapServerKey] ?? []
        const rootKey = normalizedRoot.toUpperCase()
        if (currentRoots.some((entry) => entry.toUpperCase() === rootKey)) {
          return rootsByServer
        }
        return {
          ...rootsByServer,
          [activeSapServerKey]: [...currentRoots, normalizedRoot],
        }
      })
      next[activeSapServerKey] = ""
      return next
    })
  }, [activeSapServerKey])

  const removeSapPackageScopeRoot = useCallback(
    (root: string) => {
      const normalizedRoot = normalizeSapPackageScopeRoot(root)
      if (!normalizedRoot) {
        return
      }
      const rootKey = normalizedRoot.toUpperCase()
      setSapPackageScopeRootsByServer((previous) => {
        const currentRoots = previous[activeSapServerKey] ?? []
        const nextRoots = currentRoots.filter((entry) => entry.toUpperCase() !== rootKey)
        return {
          ...previous,
          [activeSapServerKey]: nextRoots,
        }
      })
    },
    [activeSapServerKey],
  )

  const applySapPackageScopeRoots = useCallback(async () => {
    const requestServerId = activeAdtServerIdRef.current
    const scopeKey = sapServerScopeKey(requestServerId)
    const scopeRoots = sapPackageScopeRootsByServerRef.current[scopeKey] ?? []
    const response = await loadSapPackageInventory(scopeRoots)
    const canonicalRoots = response?.roots ?? scopeRoots
    setSapPackageScopeRootsByServer((previous) => ({
      ...previous,
      [scopeKey]: canonicalRoots,
    }))
    await patchSapExplorerState({
      setPackageScopeRoots: canonicalRoots,
    })
  }, [loadSapPackageInventory, patchSapExplorerState])

  const toggleSapPackageScopePreset = useCallback(
    (presetRoot: string) => {
      const normalizedPreset = normalizeSapPackageScopeRoot(presetRoot)
      if (!normalizedPreset) {
        return
      }
      const presetKey = normalizedPreset.toUpperCase()
      setSapPackageScopeRootsByServer((previous) => {
        const currentRoots = previous[activeSapServerKey] ?? []
        const existingIndex = currentRoots.findIndex(
          (entry) => entry.toUpperCase() === presetKey,
        )
        const nextRoots = [...currentRoots]
        if (existingIndex >= 0) {
          nextRoots.splice(existingIndex, 1)
        } else {
          nextRoots.push(normalizedPreset)
        }
        return {
          ...previous,
          [activeSapServerKey]: nextRoots,
        }
      })
    },
    [activeSapServerKey],
  )

  const refreshActiveAdtServerSelection = useCallback(async () => {
    try {
      const response = await neuroAdtServerList()
      const nextServerId = response.selectedServerId ?? null
      handleAdtServerSelectionChange(nextServerId, { refreshExplorer: false })
    } catch {
      // optional backend capability
    }
  }, [handleAdtServerSelectionChange])

  const resolveBoundAdtServerIdForOpen = useCallback(
    async (preferredServerId: string | null): Promise<string | null> => {
      try {
        const response = await neuroAdtServerList()
        const resolved = selectBoundAdtServerId(preferredServerId, {
          availableServerIds: response.servers.map((server) => server.id),
          selectedServerId: response.selectedServerId ?? null,
        })
        if (resolved && resolved !== activeAdtServerId) {
          setActiveAdtServerId(resolved)
        }
        return resolved
      } catch {
        return preferredServerId
      }
    },
    [activeAdtServerId],
  )

  useEffect(() => {
    void refreshActiveAdtServerSelection()
  }, [refreshActiveAdtServerSelection])

  useEffect(() => {
    if (!activeAdtServerId) {
      lastAutoRefreshedAdtServerIdRef.current = null
      setSapExplorerState(INITIAL_SAP_EXPLORER_STATE)
      setSapLocalObjects([])
      setSapFavoriteObjects([])
      setSapSystemNamespaces([])
      setSapFavoritePackageObjects({})
      setSapNamespaceObjects({})
      setSapPackageInventory(null)
      setSapPackageInventoryObjectsByServer({})
      setSapLoading(INITIAL_SAP_LOADING_STATE)
      setSapPackageObjectsLoading({})
      setSapPackageInventoryObjectsLoadingByServer({})
      setSapNamespaceObjectsLoading({})
      return
    }
    if (lastAutoRefreshedAdtServerIdRef.current === activeAdtServerId) {
      return
    }
    lastAutoRefreshedAdtServerIdRef.current = activeAdtServerId
    void refreshSapExplorerRef.current()
  }, [activeAdtServerId])

  const syncRuntimeStatus = useCallback(async () => {
    try {
      const status = await codexRuntimeStatus()
      setRuntime((prev) => ({
        ...prev,
        connected: true,
        state: status.sessionId != null ? "running" : "idle",
        sessionId: status.sessionId ?? null,
        pid: status.pid ?? null,
        workspace: status.workspace,
      }))
      return status
    } catch {
      return null
    }
  }, [])

  const ensureWorkspaceWriteReady = useCallback(
    (operation: string, requiredMethods: RuntimeMethod[]): boolean => {
      if (runtime.state !== "running") {
        addMessage(
          "system",
          `[explorer] Sessao inativa. Inicie uma sessao antes de ${operation}.`,
        )
        return false
      }

      const unsupported = getUnsupportedRuntimeMethods(requiredMethods)
      if (unsupported.length > 0) {
        addMessage(
          "system",
          `[explorer] Operacao "${operation}" indisponivel neste runtime. Metodos nao suportados: ${unsupported.join(", ")}.`,
        )
        return false
      }

      return true
    },
    [addMessage, getUnsupportedRuntimeMethods, runtime.state],
  )

  const normalizeRelativeWorkspacePath = useCallback(
    (rawPath: string): string => {
      const trimmed = rawPath.trim()
      if (!trimmed) {
        throw new Error("Informe um caminho relativo valido.")
      }

      const slashNormalized = trimmed.replace(/\\/g, "/")
      const looksAbsolute =
        slashNormalized.startsWith("/") ||
        slashNormalized.startsWith("//") ||
        /^[a-z]:\//i.test(slashNormalized)

      const rootRelative = normalizeWorkspacePathForRoot(
        slashNormalized,
        runtime.workspace,
      )
      const sanitized = rootRelative
        .replace(/^\.\/+/, "")
        .replace(/^\/+/, "")
        .replace(/\/+/g, "/")
        .replace(/\/+$/, "")

      if (!sanitized) {
        throw new Error("Informe um caminho relativo valido.")
      }
      if (sanitized.split("/").some((segment) => segment === "..")) {
        throw new Error(
          "Caminhos com '..' nao sao permitidos para criacao no workspace.",
        )
      }
      if (
        /^[a-z]:/i.test(sanitized) ||
        sanitized.startsWith("/") ||
        sanitized.startsWith("//")
      ) {
        throw new Error(
          "O caminho precisa estar dentro do workspace ativo.",
        )
      }
      if (
        looksAbsolute &&
        sanitized.toLowerCase() === slashNormalized.toLowerCase()
      ) {
        throw new Error(
          "Use um caminho relativo ao workspace ativo.",
        )
      }

      return sanitized
    },
    [runtime.workspace],
  )

  const handleOpenSourceFromRef = useCallback(
    (ref: string): boolean => {
      const currentEditor = sourceEditorRef.current
      if (
        currentEditor.visible &&
        currentEditor.dirty &&
        !window.confirm(
          "Existem alteracoes nao salvas no editor de codigo. Deseja descartar e abrir outro arquivo?",
        )
      ) {
        return false
      }

      const refRoute = routeSourceEditorRef(ref)
      const activeWorkspace = runtime.workspace
      const workspaceObjectUri =
        refRoute.kind === "workspace"
          ? normalizeWorkspacePathForRoot(refRoute.normalizedRef, activeWorkspace)
          : null
      const normalizedWorkspaceObjectUri =
        workspaceObjectUri && workspaceObjectUri.trim().length > 0
          ? workspaceObjectUri
          : refRoute.kind === "workspace"
            ? refRoute.normalizedRef
            : null
      const requestedAdtServerId =
        refRoute.kind === "abap" ? activeAdtServerId : null
      if (refRoute.kind === "abap") {
        const requiredAbapMethods: RuntimeMethod[] = refRoute.normalizedRef.startsWith(
          "/sap/bc/adt/",
        )
          ? ["neuro.get.source"]
          : ["neuro.search.objects", "neuro.get.source"]
        const unsupported = getUnsupportedRuntimeMethods(requiredAbapMethods)
        if (unsupported.length > 0) {
          setSourceEditorWithRef({
            visible: true,
            refKind: "abap",
            adtServerId: requestedAdtServerId,
            objectUri: null,
            workspaceRoot: activeWorkspace,
            displayName: null,
            language: "abap",
            source: "",
            etag: null,
            dirty: false,
            loading: false,
            saving: false,
            error: buildUnsupportedEditorMessage("abrir referencia ABAP", unsupported),
          })
          return true
        }
      }
      if (refRoute.kind === "workspace") {
        const unsupported = getUnsupportedRuntimeMethods(["workspace.file.read"])
        if (unsupported.length > 0) {
          setSourceEditorWithRef({
            visible: true,
            refKind: "workspace",
            adtServerId: null,
            objectUri: normalizedWorkspaceObjectUri,
            workspaceRoot: activeWorkspace,
            displayName: inferDisplayNameFromPath(normalizedWorkspaceObjectUri ?? ""),
            language: refRoute.monacoLanguage,
            source: "",
            etag: null,
            dirty: false,
            loading: false,
            saving: false,
            error: buildUnsupportedEditorMessage(
              "abrir arquivo do workspace",
              unsupported,
            ),
          })
          return true
        }
      }
      if (refRoute.kind === "workspace" && runtime.state !== "running") {
        setSourceEditorWithRef({
          visible: true,
          refKind: "workspace",
          adtServerId: null,
          objectUri: normalizedWorkspaceObjectUri,
          workspaceRoot: activeWorkspace,
          displayName: inferDisplayNameFromPath(normalizedWorkspaceObjectUri ?? ""),
          language: refRoute.monacoLanguage,
          source: "",
          etag: null,
          dirty: false,
          loading: false,
          saving: false,
          error:
            "Inicie uma sessao ativa antes de abrir arquivos do workspace no editor de codigo.",
        })
        return true
      }
      const nextRequestId = beginSourceOperation()
      sourceBaselineRef.current = ""

      setSourceEditorWithRef((previous) => ({
        ...previous,
        visible: true,
        refKind: refRoute.kind,
        adtServerId: requestedAdtServerId,
        objectUri: normalizedWorkspaceObjectUri,
        workspaceRoot: activeWorkspace,
        displayName:
          refRoute.kind === "workspace"
            ? inferDisplayNameFromPath(normalizedWorkspaceObjectUri ?? "")
            : null,
        language: refRoute.monacoLanguage,
        source: "",
        etag: null,
        dirty: false,
        loading: true,
        saving: false,
        error: null,
      }))

      void (async () => {
        try {
          if (refRoute.kind === "abap") {
            const boundAdtServerId = await resolveBoundAdtServerIdForOpen(
              requestedAdtServerId,
            )
            const resolved = await resolveAbapSourceRef(refRoute.normalizedRef, {
              serverId: boundAdtServerId,
            })
            const loaded = await neuroGetSource(
              resolved.objectUri,
              boundAdtServerId,
            )

            if (sourceOperationSeqRef.current !== nextRequestId) {
              return
            }

            setSourceEditorWithRef({
              visible: true,
              refKind: "abap",
              adtServerId: boundAdtServerId ?? null,
              objectUri: loaded.objectUri,
              workspaceRoot: activeWorkspace,
              displayName: resolved.displayName,
              language: "abap",
              source: loaded.source,
              etag: loaded.etag ?? null,
              dirty: false,
              loading: false,
              saving: false,
              error: null,
            })
            void patchSapExplorerState({
              focusedObjectUri: loaded.objectUri,
            })
            sourceBaselineRef.current = loaded.source
            return
          }

          const loaded = await codexWorkspaceReadFile({
            path: normalizedWorkspaceObjectUri ?? refRoute.normalizedRef,
          })
          const normalizedLoadedPath = normalizeWorkspacePathForRoot(
            loaded.path,
            activeWorkspace,
          )
          const loadedObjectUri =
            normalizedLoadedPath.trim().length > 0 ? normalizedLoadedPath : loaded.path

          if (sourceOperationSeqRef.current !== nextRequestId) {
            return
          }

          setSourceEditorWithRef({
            visible: true,
            refKind: "workspace",
            adtServerId: null,
            objectUri: loadedObjectUri,
            workspaceRoot: activeWorkspace,
            displayName: inferDisplayNameFromPath(loadedObjectUri),
            language: refRoute.monacoLanguage,
            source: loaded.content,
            etag: null,
            dirty: false,
            loading: false,
            saving: false,
            error: null,
          })
          sourceBaselineRef.current = loaded.content
        } catch (error) {
          if (sourceOperationSeqRef.current !== nextRequestId) {
            return
          }

          setSourceEditorWithRef({
            visible: true,
            refKind: refRoute.kind,
            adtServerId: requestedAdtServerId,
            objectUri: normalizedWorkspaceObjectUri,
            workspaceRoot: activeWorkspace,
            displayName:
              refRoute.kind === "workspace"
                ? inferDisplayNameFromPath(normalizedWorkspaceObjectUri ?? "")
                : null,
            language: refRoute.monacoLanguage,
            source: "",
            etag: null,
            dirty: false,
            loading: false,
            saving: false,
            error: toEditorErrorMessage(error),
          })
          sourceBaselineRef.current = ""
        }
      })()

      return true
    },
    [
      beginSourceOperation,
      buildUnsupportedEditorMessage,
      getUnsupportedRuntimeMethods,
      inferDisplayNameFromPath,
      runtime.state,
      runtime.workspace,
      activeAdtServerId,
      patchSapExplorerState,
      resolveBoundAdtServerIdForOpen,
      setSourceEditorWithRef,
      toEditorErrorMessage,
    ],
  )

  const handleMainTabChange = useCallback((value: string) => {
    if (value === "chat" || value === "editor" || value === "terminal") {
      setActiveMainTab(value)
    }
  }, [])

  const onOpenInEditor = useCallback(
    (ref: string) => {
      const opened = handleOpenSourceFromRef(ref)
      if (opened && isMobile) {
        setActiveMainTab("editor")
      } else if (opened) {
        setDesktopViewMode((previous) => (previous === "chat" ? "split" : previous))
      }
    },
    [handleOpenSourceFromRef, isMobile],
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
      upsertExplorerEntry,
      toEditorErrorMessage,
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
      upsertExplorerEntry,
      toEditorErrorMessage,
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

  const handleReloadSource = useCallback(async () => {
    const currentEditor = sourceEditorRef.current
    if (!currentEditor.objectUri || !currentEditor.refKind) {
      return
    }
    if (currentEditor.refKind === "workspace" && runtime.state !== "running") {
      setSourceEditorWithRef((previous) => ({
        ...previous,
        loading: false,
        error:
          "Sessao inativa. Inicie uma sessao antes de recarregar arquivos do workspace.",
      }))
      return
    }
    if (currentEditor.refKind === "workspace") {
      const unsupported = getUnsupportedRuntimeMethods(["workspace.file.read"])
      if (unsupported.length > 0) {
        setSourceEditorWithRef((previous) => ({
          ...previous,
          loading: false,
          error: buildUnsupportedEditorMessage(
            "recarregar arquivo do workspace",
            unsupported,
          ),
        }))
        return
      }
    }
    if (currentEditor.refKind === "abap") {
      const unsupported = getUnsupportedRuntimeMethods(["neuro.get.source"])
      if (unsupported.length > 0) {
        setSourceEditorWithRef((previous) => ({
          ...previous,
          loading: false,
          error: buildUnsupportedEditorMessage("recarregar fonte ABAP", unsupported),
        }))
        return
      }
    }
    if (
      currentEditor.refKind === "workspace" &&
      currentEditor.workspaceRoot !== runtime.workspace
    ) {
      setSourceEditorWithRef((previous) => ({
        ...previous,
        loading: false,
        error:
          "Workspace ativo mudou desde a abertura do arquivo. Reabra o arquivo para evitar gravar no workspace errado.",
      }))
      return
    }
    if (
      currentEditor.dirty &&
      !window.confirm(
        "Existem alteracoes nao salvas no editor de codigo. Deseja recarregar e descartar as alteracoes locais?",
      )
    ) {
      return
    }
    const objectUri = currentEditor.objectUri
    const refKind = currentEditor.refKind
    let boundAdtServerId: string | null = null
    if (refKind === "abap") {
      boundAdtServerId = currentEditor.adtServerId ?? null
    }
    const requestId = beginSourceOperation()

    setSourceEditorWithRef((previous) => ({
      ...previous,
      loading: true,
      error: null,
    }))

    try {
      if (refKind === "abap") {
        const loaded = await neuroGetSource(objectUri, boundAdtServerId)

        if (
          sourceOperationSeqRef.current !== requestId ||
          sourceEditorRef.current.objectUri !== objectUri
        ) {
          return
        }

        sourceBaselineRef.current = loaded.source
        setSourceEditorWithRef((previous) => ({
          ...previous,
          adtServerId: boundAdtServerId,
          source: loaded.source,
          etag: loaded.etag ?? null,
          dirty: false,
          loading: false,
          error: null,
        }))
        return
      }

      const loaded = await codexWorkspaceReadFile({ path: objectUri })

      if (
        sourceOperationSeqRef.current !== requestId ||
        sourceEditorRef.current.objectUri !== objectUri
      ) {
        return
      }

      sourceBaselineRef.current = loaded.content
      setSourceEditorWithRef((previous) => ({
        ...previous,
        source: loaded.content,
        etag: null,
        dirty: false,
        loading: false,
        error: null,
      }))
    } catch (error) {
      if (
        sourceOperationSeqRef.current !== requestId ||
        sourceEditorRef.current.objectUri !== objectUri
      ) {
        return
      }
      setSourceEditorWithRef((previous) => ({
        ...previous,
        loading: false,
        error: `Falha ao recarregar codigo: ${toEditorErrorMessage(error)}`,
      }))
    }
  }, [
    beginSourceOperation,
    buildUnsupportedEditorMessage,
    getUnsupportedRuntimeMethods,
    runtime.state,
    runtime.workspace,
    setSourceEditorWithRef,
    toEditorErrorMessage,
  ])

  const handleSaveSource = useCallback(async () => {
    const currentEditor = sourceEditorRef.current
    if (
      !currentEditor.objectUri ||
      !currentEditor.refKind ||
      currentEditor.loading ||
      currentEditor.saving
    ) {
      return
    }
    if (currentEditor.refKind === "workspace" && runtime.state !== "running") {
      setSourceEditorWithRef((previous) => ({
        ...previous,
        saving: false,
        error:
          "Sessao inativa. Inicie uma sessao antes de salvar arquivos do workspace.",
      }))
      return
    }
    if (currentEditor.refKind === "workspace") {
      const unsupported = getUnsupportedRuntimeMethods(["workspace.file.write"])
      if (unsupported.length > 0) {
        setSourceEditorWithRef((previous) => ({
          ...previous,
          saving: false,
          error: buildUnsupportedEditorMessage(
            "salvar arquivo do workspace",
            unsupported,
          ),
        }))
        return
      }
    }
    if (currentEditor.refKind === "abap") {
      const unsupported = getUnsupportedRuntimeMethods(["neuro.update.source"])
      if (unsupported.length > 0) {
        setSourceEditorWithRef((previous) => ({
          ...previous,
          saving: false,
          error: buildUnsupportedEditorMessage("salvar fonte ABAP", unsupported),
        }))
        return
      }
    }
    if (
      currentEditor.refKind === "workspace" &&
      currentEditor.workspaceRoot !== runtime.workspace
    ) {
      setSourceEditorWithRef((previous) => ({
        ...previous,
        saving: false,
        error:
          "Workspace ativo mudou desde a abertura do arquivo. Reabra o arquivo para evitar gravar no workspace errado.",
      }))
      return
    }
    const objectUri = currentEditor.objectUri
    const refKind = currentEditor.refKind
    const sourceToSave = currentEditor.source
    const etag = currentEditor.etag ?? undefined
    let boundAdtServerId: string | null = null
    if (refKind === "abap") {
      boundAdtServerId = currentEditor.adtServerId ?? null
    }
    const requestId = beginSourceOperation()

    setSourceEditorWithRef((previous) => ({
      ...previous,
      saving: true,
      error: null,
    }))

    try {
      if (refKind === "abap") {
        const result = await neuroUpdateSource({
          objectUri,
          source: sourceToSave,
          etag,
          serverId: boundAdtServerId,
        })

        if (
          sourceOperationSeqRef.current !== requestId ||
          sourceEditorRef.current.objectUri !== objectUri
        ) {
          return
        }

        const sourceStillMatches = sourceEditorRef.current.source === sourceToSave
        if (sourceStillMatches) {
          sourceBaselineRef.current = sourceToSave
        }

        setSourceEditorWithRef((previous) => ({
          ...previous,
          adtServerId: boundAdtServerId,
          etag: result.etag ?? previous.etag,
          dirty: sourceStillMatches ? false : previous.dirty,
          saving: false,
          error: null,
        }))
        return
      }

      await codexWorkspaceWriteFile({
        path: objectUri,
        content: sourceToSave,
      })

      if (
        sourceOperationSeqRef.current !== requestId ||
        sourceEditorRef.current.objectUri !== objectUri
      ) {
        return
      }

      const sourceStillMatches = sourceEditorRef.current.source === sourceToSave
      if (sourceStillMatches) {
        sourceBaselineRef.current = sourceToSave
      }

      setSourceEditorWithRef((previous) => ({
        ...previous,
        etag: null,
        dirty: sourceStillMatches ? false : previous.dirty,
        saving: false,
        error: null,
      }))
    } catch (error) {
      if (
        sourceOperationSeqRef.current !== requestId ||
        sourceEditorRef.current.objectUri !== objectUri
      ) {
        return
      }
      setSourceEditorWithRef((previous) => ({
        ...previous,
        saving: false,
        error: `Falha ao salvar codigo: ${toEditorErrorMessage(error)}`,
      }))
    }
  }, [
    beginSourceOperation,
    buildUnsupportedEditorMessage,
    getUnsupportedRuntimeMethods,
    runtime.state,
    runtime.workspace,
    setSourceEditorWithRef,
    toEditorErrorMessage,
  ])

  const handleCloseEditor = useCallback(() => {
    const currentEditor = sourceEditorRef.current
    if (
      currentEditor.dirty &&
      !window.confirm(
        "Existem alteracoes nao salvas no editor de codigo. Deseja fechar e descartar as alteracoes?",
      )
    ) {
      return
    }

    sourceOperationSeqRef.current += 1
    sourceBaselineRef.current = ""
    setSourceEditorWithRef(INITIAL_SOURCE_EDITOR_STATE)
  }, [setSourceEditorWithRef])

  const handleOpenPanel = useCallback(
    (panel: AliciaState["activePanel"]) => {
      if (!panel) {
        setAliciaState((prev) => ({ ...prev, activePanel: null }))
        return
      }
      if (panel === "model") {
        void openModelPanel(true)
        return
      }
      if (panel === "sessions") {
        void openSessionPanel("list")
        return
      }
      if (panel === "apps") {
        setAliciaState((prev) => ({ ...prev, activePanel: "apps" }))
        void refreshAppsAndAuth({ throwOnError: false })
        return
      }
      if (panel === "adt") {
        setAliciaState((prev) => ({ ...prev, activePanel: "adt" }))
        void refreshActiveAdtServerSelection()
        return
      }
      setAliciaState((prev) => ({ ...prev, activePanel: panel }))
    },
    [
      openModelPanel,
      openSessionPanel,
      refreshActiveAdtServerSelection,
      refreshAppsAndAuth,
    ],
  )

  const resetSessionUiState = useCallback(() => {
    threadIdRef.current = null
    reviewRoutingRef.current = false
    setMessages([])
    setPendingApprovals([])
    setPendingUserInput(null)
    setTurnDiff(null)
    setTurnPlan(null)
  }, [])

  const handleStartSession = useCallback(() => {
    void (async () => {
      resetSessionUiState()
      const started = await ensureRuntimeSession(true)
      await syncRuntimeStatus()
      if (started) {
        await refreshWorkspaceChanges()
      }
    })()
  }, [ensureRuntimeSession, refreshWorkspaceChanges, resetSessionUiState, syncRuntimeStatus])

  const handleStopSession = useCallback(() => {
    void (async () => {
      try {
        await codexRuntimeSessionStop()
        reviewRoutingRef.current = false
        setPendingApprovals([])
        setPendingUserInput(null)
        setTurnDiff(null)
        setTurnPlan(null)
        setRuntime((prev) => ({
          ...prev,
          state: "idle",
          sessionId: null,
          pid: null,
        }))
        await syncRuntimeStatus()
      } catch (error) {
        addMessage("system", `[session] failed to stop: ${String(error)}`)
      }
    })()
  }, [addMessage, syncRuntimeStatus])

  const handleStartReview = useCallback(() => {
    setAliciaState((prev) => ({ ...prev, activePanel: "review" }))
    void refreshWorkspaceChanges()
  }, [refreshWorkspaceChanges])

  const handleOpenWorkspaceFolder = useCallback(async () => {
    if (switchingWorkspaceFolder) {
      return
    }

    setSwitchingWorkspaceFolder(true)
    try {
      const selectedPath = await pickWorkspaceFolder()
      if (!selectedPath) {
        return
      }

      const nextWorkspace = selectedPath.trim()
      if (!nextWorkspace) {
        return
      }

      if (runtime.sessionId != null) {
        await codexRuntimeSessionStop()
        setRuntime((previous) => ({
          ...previous,
          state: "idle",
          sessionId: null,
          pid: null,
        }))
      }

      resetSessionUiState()
      sourceOperationSeqRef.current += 1
      sourceBaselineRef.current = ""
      setSourceEditorWithRef(INITIAL_SOURCE_EDITOR_STATE)
      setExplorerRootPath("")
      setExplorerRootEntries([])
      setExplorerChildrenByPath({})
      setExplorerLoadedPaths({})

      const started = await codexRuntimeSessionStart({
        cwd: nextWorkspace,
      })
      setActiveSessionEntry(started.sessionId, aliciaState.model)
      setRuntime((previous) => ({
        ...previous,
        connected: true,
        state: "running",
        sessionId: started.sessionId,
        pid: started.pid,
        workspace: nextWorkspace,
      }))

      const loaded = await loadWorkspaceDirectory(undefined, { asRoot: true })
      if (!loaded) {
        return
      }

      await syncRuntimeStatus()
      await refreshWorkspaceChanges()
      addMessage("system", `[workspace] Pasta ativa alterada para: ${nextWorkspace}`)
    } catch (error) {
      await syncRuntimeStatus()
      await refreshThreadList({ notifyOnError: false })
      addMessage(
        "system",
        `[workspace] Falha ao abrir pasta: ${String(error)}`,
      )
    } finally {
      setSwitchingWorkspaceFolder(false)
    }
  }, [
    addMessage,
    aliciaState.model,
    loadWorkspaceDirectory,
    refreshThreadList,
    resetSessionUiState,
    refreshWorkspaceChanges,
    runtime.sessionId,
    setActiveSessionEntry,
    setSourceEditorWithRef,
    switchingWorkspaceFolder,
    syncRuntimeStatus,
  ])

  const handleCreateTerminalTab = useCallback(() => {
    void createTerminalTab(runtime.workspace)
  }, [createTerminalTab, runtime.workspace])

  const handleEditorSourceChange = useCallback(
    (value: string) => {
      setSourceEditorWithRef((previous) => ({
        ...previous,
        source: value,
        dirty: Boolean(previous.objectUri) && value !== sourceBaselineRef.current,
        error: null,
      }))
    },
    [setSourceEditorWithRef],
  )

  const conversationPaneNode = (
    <ConversationPane
      currentModelLabel={currentModelLabel}
      reasoningEffort={aliciaState.reasoningEffort}
      messages={messages}
      isThinking={isThinking}
      pendingImages={pendingImages}
      pendingMentions={pendingMentions}
      runtimeCapabilities={aliciaState.runtimeCapabilities}
      pendingApprovals={pendingApprovals}
      pendingUserInput={pendingUserInput}
      turnDiff={turnDiff}
      turnDiffFiles={turnDiffFiles}
      turnPlan={turnPlan}
      runtimeState={runtime.state}
      scrollRef={scrollRef}
      onSubmit={handleSubmit}
      onSlashCommand={handleSlashCommand}
      onAttachImage={async () => {
        const picked = await pickImageFile()
        if (picked) setPendingImages((prev) => [...prev, picked])
      }}
      onAttachMention={async () => {
        const picked = await pickMentionFile()
        if (picked) setPendingMentions((prev) => [...prev, picked])
      }}
      onRemoveImage={(index) => {
        setPendingImages((prev) => prev.filter((_, i) => i !== index))
      }}
      onRemoveMention={(index) => {
        setPendingMentions((prev) => prev.filter((_, i) => i !== index))
      }}
      onApprovalDecision={handleApprovalDecision}
      onUserInputDecision={handleUserInputDecision}
      onOpenInEditor={onOpenInEditor}
    />
  )

  const editorPaneNode = sourceEditor.visible ? (
    <SourceEditorPanel
      objectUri={sourceEditor.objectUri}
      displayName={sourceEditor.displayName}
      language={sourceEditor.language}
      source={sourceEditor.source}
      etag={sourceEditor.etag}
      dirty={sourceEditor.dirty}
      loading={sourceEditor.loading}
      saving={sourceEditor.saving}
      error={sourceEditor.error}
      onChangeSource={handleEditorSourceChange}
      onReload={() => {
        void handleReloadSource()
      }}
      onSave={() => {
        void handleSaveSource()
      }}
      onClose={handleCloseEditor}
    />
  ) : (
    <div className="h-full w-full flex items-center justify-center px-4 text-center text-sm text-muted-foreground">
      Abra uma referencia de codigo na sidebar, no diff da conversa ou no review para carregar o editor.
    </div>
  )

  const sapSidebarNode = (
    <IdeSapExplorer
      serverId={activeAdtServerId}
      state={sapExplorerState}
      localObjects={sapLocalObjects}
      favoriteObjects={sapFavoriteObjects}
      systemNamespaces={sapSystemNamespaces}
      favoritePackageObjects={sapFavoritePackageObjects}
      namespaceObjects={sapNamespaceObjects}
      packageInventoryObjects={sapPackageInventoryObjects}
      packageInventory={sapPackageInventory}
      packageScopeRoots={sapPackageScopeRoots}
      packageScopeRootInput={sapPackageScopeRootInput}
      packageScopePresets={SAP_PACKAGE_SCOPE_PRESETS}
      loadingState={sapLoading.state}
      loadingLocalObjects={sapLoading.localObjects}
      loadingFavoriteObjects={sapLoading.favoriteObjects}
      loadingFavoritePackages={sapLoading.favoritePackages}
      loadingSystemNamespaces={sapLoading.systemNamespaces}
      loadingPackageInventory={sapLoading.packageInventory}
      loadingPackageObjects={sapPackageObjectsLoading}
      loadingPackageInventoryObjects={sapPackageInventoryObjectsLoading}
      loadingNamespaceObjects={sapNamespaceObjectsLoading}
      onRefresh={() => {
        void refreshSapExplorer()
      }}
      onToggleFavoritePackage={(item) => {
        void patchSapExplorerState({
          toggleFavoritePackage: item,
        })
      }}
      onToggleFavoriteObject={(object) => {
        void patchSapExplorerState({
          toggleFavoriteObject: object,
        })
      }}
      onSelectWorkingPackage={(packageName) => {
        void patchSapExplorerState({
          workingPackage: packageName,
        })
      }}
      onOpenFile={(uri) => {
        onOpenInEditor(uri)
      }}
      onLoadFavoritePackageObjects={(item) => {
        void loadSapFavoritePackageObjects(item)
      }}
      onLoadNamespaceObjects={(namespace) => {
        void loadSapNamespaceObjects(namespace)
      }}
      onLoadPackageInventoryObjects={(packageName) => {
        void loadSapPackageInventoryObjects(packageName)
      }}
      onChangePackageScopeRootInput={(value) => {
        setSapPackageScopeRootInput(value)
      }}
      onAddPackageScopeRoot={() => {
        addSapPackageScopeRoot()
      }}
      onRemovePackageScopeRoot={(root) => {
        removeSapPackageScopeRoot(root)
      }}
      onApplyPackageScopeRoots={() => {
        void applySapPackageScopeRoots()
      }}
      onTogglePackageScopePreset={toggleSapPackageScopePreset}
    />
  )

  const terminalPaneNode = (
    <TerminalPane
      tabs={terminalTabs}
      activeTerminalId={activeTerminalId}
      terminalContainerRef={terminalContainerRef}
      onSelectTab={setActiveTerminalId}
      onCloseTab={(id) => {
        void closeTerminalTab(id)
      }}
      onCreateTab={handleCreateTerminalTab}
    />
  )

  if (initializing) {
    return (
      <div className="alicia-shell-root flex items-center justify-center bg-[var(--ide-app-bg)] p-4">
        <div className="w-full max-w-3xl rounded-md border border-[var(--ide-border-strong)] bg-[var(--ide-surface-1)] shadow-lg shadow-black/30">
          <div className="flex items-center gap-2 border-b border-[var(--ide-border-subtle)] px-4 py-3 text-sm text-terminal-fg/90">
            <Bot className="w-4 h-4 text-terminal-green spin-slow" />
            {initializingStatus}
          </div>
          <div className="p-4 font-mono text-xs text-terminal-fg/75 max-h-64 overflow-y-auto">
            {bootLogs.length === 0 ? (
              <div className="text-terminal-fg/45">
                Aguardando logs de inicializacao...
              </div>
            ) : (
              bootLogs.map((line, index) => (
                <div key={`boot-log-${index}`} className="leading-relaxed">
                  {line}
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="alicia-shell-root flex flex-col bg-[var(--ide-surface-0)]">
      {!isMobile && desktopShellMode === "zen" ? null : (
        <TitleBar
          connected={runtime.connected}
          workspace={runtime.workspace}
          onOpenWorkspaceFolder={handleOpenWorkspaceFolder}
          openWorkspaceFolderBusy={switchingWorkspaceFolder}
          onOpenAdtConnections={() => {
            setAliciaState((previous) => ({ ...previous, activePanel: "adt" }))
            void refreshActiveAdtServerSelection()
          }}
        />
      )}
      {isMobile ? (
        <div className="flex-1 min-h-0 flex flex-col">
          <div className="flex items-center gap-2 border-b border-[var(--ide-border-subtle)] px-3 py-2">
            <button
              type="button"
              onClick={() => {
                void handleOpenWorkspaceFolder()
              }}
              disabled={switchingWorkspaceFolder}
              className="rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-2)] px-2 py-1 text-xs text-terminal-fg transition-colors hover:bg-[var(--ide-hover)] disabled:cursor-not-allowed disabled:opacity-60"
            >
              {switchingWorkspaceFolder ? "Abrindo..." : "Abrir pasta..."}
            </button>
            <span className="min-w-0 truncate text-xs text-muted-foreground" title={runtime.workspace}>
              {runtime.workspace}
            </span>
          </div>
          <div className="h-[40%] min-h-[260px] border-b border-[var(--ide-border-subtle)]">
            <Sidebar
              state={aliciaState}
              modelLabel={currentModelLabel}
              sessionPid={runtime.pid}
              runtimeState={runtime.state}
              onOpenPanel={handleOpenPanel}
              onStartSession={handleStartSession}
              onStopSession={handleStopSession}
              onResumeSession={() => {
                void openSessionPanel("resume")
              }}
              onForkSession={() => {
                void openSessionPanel("fork")
              }}
              onSelectSession={(sessionId) => {
                void handleSessionSelect(sessionId, "switch")
              }}
              onStartReview={handleStartReview}
              onOpenFileChangeInEditor={onOpenInEditor}
            />
          </div>

          <Tabs
            value={activeMainTab}
            onValueChange={handleMainTabChange}
            className="flex-1 min-h-0 gap-0"
          >
            <div className="border-b border-[var(--ide-border-subtle)] px-3 py-2">
              <TabsList className="grid h-8 w-full grid-cols-3">
                <TabsTrigger value="chat" className="text-xs">
                  Chat
                </TabsTrigger>
                <TabsTrigger value="editor" className="text-xs">
                  Editor
                </TabsTrigger>
                <TabsTrigger value="terminal" className="text-xs">
                  Terminal
                </TabsTrigger>
              </TabsList>
            </div>

            <TabsContent value="chat" forceMount className="flex-1 min-h-0 data-[state=inactive]:hidden">
              {conversationPaneNode}
            </TabsContent>

            <TabsContent value="editor" forceMount className="flex-1 min-h-0 data-[state=inactive]:hidden">
              {editorPaneNode}
            </TabsContent>

            <TabsContent value="terminal" forceMount className="flex-1 min-h-0 data-[state=inactive]:hidden">
              {terminalPaneNode}
            </TabsContent>
          </Tabs>
        </div>
      ) : (
        <AliciaDesktopShell
          shellMode={desktopShellMode}
          sidebarView={desktopSidebarView}
          sidebarVisible={desktopSidebarVisible}
          viewMode={desktopViewMode}
          terminalVisible={desktopTerminalVisible}
          fileChanges={aliciaState.fileChanges}
          workspaceLabel={pathBasename(explorerRootPath || runtime.workspace) || "workspace"}
          treeEntries={explorerRootEntries}
          treeChildrenByPath={explorerChildrenByPath}
          loadedDirectoryPaths={explorerLoadedPaths}
          loadingDirectoryPaths={explorerLoadingPaths}
          treeVersion={explorerTreeVersion}
          loadingExplorerRoot={explorerRootLoading}
          onSelectSidebarView={(view) => {
            setDesktopSidebarView(view)
            setDesktopSidebarVisible(true)
            if (view === "sap") {
              void refreshSapExplorer()
            }
          }}
          onToggleSidebar={() => {
            if (desktopShellMode !== "normal") {
              return
            }
            setDesktopSidebarVisible((previous) => !previous)
          }}
          onTerminalVisibilityChange={(visible) => {
            if (desktopShellMode !== "normal") {
              return
            }
            setDesktopTerminalVisible(visible)
          }}
          onOpenFileInEditor={onOpenInEditor}
          onLoadExplorerDirectory={handleLoadExplorerDirectory}
          onRefreshFileChanges={() => {
            void refreshExplorerRoot({ silent: true })
            void refreshWorkspaceChanges()
          }}
          onCreateFile={(relativePath) => {
            return handleCreateWorkspaceFile(relativePath)
          }}
          onCreateFolder={(relativePath) => {
            return handleCreateWorkspaceFolder(relativePath)
          }}
          onRenameEntry={(path, newName) => {
            return handleRenameWorkspaceEntry(path, newName)
          }}
          sapSidebar={sapSidebarNode}
          agentSidebar={
            <Sidebar
              state={aliciaState}
              modelLabel={currentModelLabel}
              sessionPid={runtime.pid}
              runtimeState={runtime.state}
              onOpenPanel={handleOpenPanel}
              onStartSession={handleStartSession}
              onStopSession={handleStopSession}
              onResumeSession={() => {
                void openSessionPanel("resume")
              }}
              onForkSession={() => {
                void openSessionPanel("fork")
              }}
              onSelectSession={(sessionId) => {
                void handleSessionSelect(sessionId, "switch")
              }}
              onStartReview={handleStartReview}
              onOpenFileChangeInEditor={onOpenInEditor}
            />
          }
          conversationPane={conversationPaneNode}
          editorPane={editorPaneNode}
          terminalPane={terminalPaneNode}
        />
      )}
      {!isMobile && desktopShellMode === "zen" ? null : (
        <StatusBar
          state={aliciaState}
          modelLabel={currentModelLabel}
          adtActiveServerId={activeAdtServerId}
          runtime={{
            connected: runtime.connected,
            state: runtime.state,
            sessionId: runtime.sessionId,
          }}
          usage={statusSignals.usage}
          reasoning={statusSignals.reasoning}
          isThinking={isThinking}
          panelToolbar={
            <IdePanelToolbar
              compact
              shellMode={desktopShellMode}
              viewMode={desktopViewMode}
              sidebarVisible={desktopSidebarVisible}
              terminalVisible={desktopTerminalVisible}
              onToggleSidebar={() => {
                if (desktopShellMode !== "normal") {
                  return
                }
                setDesktopSidebarVisible((previous) => !previous)
              }}
              onToggleTerminal={() => {
                if (desktopShellMode !== "normal") {
                  return
                }
                setDesktopTerminalVisible((previous) => !previous)
              }}
              onChangeViewMode={setDesktopViewMode}
              onChangeShellMode={handleDesktopShellModeChange}
            />
          }
          onOpenPanel={handleOpenPanel}
        />
      )}
      {!isMobile && desktopShellMode === "zen" ? (
        <div className="zen-exit-overlay">
          <button
            type="button"
            className="zen-exit-button"
            onClick={() => {
              handleDesktopShellModeChange("normal")
            }}
            aria-label="Sair do modo zen"
          >
            <Minimize2 className="h-3.5 w-3.5" />
            Sair do zen
            <kbd className="ml-2 rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-2)] px-1 py-0.5 text-[10px] text-muted-foreground">
              Esc
            </kbd>
          </button>
        </div>
      ) : null}
      {aliciaState.activePanel === "model" && (
        <ModelPicker
          currentModel={aliciaState.model}
          currentEffort={aliciaState.reasoningEffort}
          models={availableModels}
          loading={modelsLoading}
          error={modelsError}
          cachedAt={modelsCachedAt}
          stale={modelsFromCache}
          onRetry={() => {
            void refreshModelsCatalog(true)
          }}
          onSelect={handleModelSelect}
          onClose={() => setAliciaState((prev) => ({ ...prev, activePanel: null }))}
        />
      )}
      {aliciaState.activePanel === "permissions" && (
        <PermissionsPanel
          currentPreset={aliciaState.approvalPreset}
          currentSandbox={aliciaState.sandboxMode}
          onSelect={handlePermissionSelect}
          onClose={() => setAliciaState((prev) => ({ ...prev, activePanel: null }))}
        />
      )}
      {aliciaState.activePanel === "mcp" && (
        <McpPanel
          servers={aliciaState.mcpServers}
          onRefresh={refreshMcpServers}
          onClose={() => setAliciaState((prev) => ({ ...prev, activePanel: null }))}
        />
      )}
      {aliciaState.activePanel === "adt" && (
        <AdtPanel
          activeServerId={activeAdtServerId}
          onActiveServerIdChange={(nextServerId) => {
            handleAdtServerSelectionChange(nextServerId, {
              refreshExplorer: true,
            })
          }}
          onOpenInEditor={onOpenInEditor}
          onClose={() => setAliciaState((prev) => ({ ...prev, activePanel: null }))}
        />
      )}
      {aliciaState.activePanel === "apps" && (
        <AppsPanel
          apps={aliciaState.apps}
          account={aliciaState.account}
          rateLimits={aliciaState.rateLimits}
          rateLimitsByLimitId={aliciaState.rateLimitsByLimitId}
          onRefresh={refreshAppsAndAuth}
          onClose={() => setAliciaState((prev) => ({ ...prev, activePanel: null }))}
        />
      )}
      {aliciaState.activePanel === "review" && (
        <ReviewMode
          fileChanges={aliciaState.fileChanges}
          turnDiffFiles={turnDiffFiles}
          pendingApprovals={pendingApprovals}
          reviewMessages={reviewMessages}
          isReviewThinking={isThinking && reviewRoutingRef.current}
          isReviewComplete={isReviewComplete}
          onRunReview={() => {
            void refreshWorkspaceChanges()
            void handleSlashCommand("/review")
          }}
          onRunReviewFile={(selectedPath) => {
            void refreshWorkspaceChanges()
            const escapedPath = selectedPath.replace(/\\/g, "\\\\").replace(/"/g, '\\"')
            void handleSlashCommand(`/review-file "${escapedPath}"`)
          }}
          onCommitApproved={handleCommitApprovedReview}
          onOpenInEditor={onOpenInEditor}
          onClose={() => setAliciaState((prev) => ({ ...prev, activePanel: null }))}
        />
      )}
      {aliciaState.activePanel === "sessions" && (
        <SessionPicker
          sessions={aliciaState.sessions}
          mode={sessionPickerMode}
          loading={sessionsPanelLoading}
          busyAction={sessionActionPending}
          onSelect={handleSessionSelect}
          onNewSession={handleStartSession}
          onClose={() => setAliciaState((prev) => ({ ...prev, activePanel: null }))}
        />
      )}
    </div>
  )
}




