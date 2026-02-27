"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { Bot } from "lucide-react"
import { TitleBar } from "@/components/alicia/title-bar"
import { Sidebar } from "@/components/alicia/sidebar"
import { ConversationPane } from "@/components/alicia/conversation-pane"
import { StatusBar } from "@/components/alicia/status-bar"
import { ModelPicker } from "@/components/alicia/model-picker"
import { PermissionsPanel } from "@/components/alicia/permissions-panel"
import { McpPanel } from "@/components/alicia/mcp-panel"
import { AppsPanel } from "@/components/alicia/apps-panel"
import { SessionPicker } from "@/components/alicia/session-picker"
import { ReviewMode } from "@/components/alicia/review-mode"
import { TerminalPane } from "@/components/alicia/terminal-pane"
import { SourceEditorPanel } from "@/components/alicia/source-editor-panel"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { type AliciaState } from "@/lib/alicia-types"
import {
  codexApprovalRespond,
  codexRuntimeSessionStop,
  codexUserInputRespond,
  isTauriRuntime,
  pickImageFile,
  pickMentionFile,
  gitCommitApprovedReview,
  codexWorkspaceChanges,
  codexWorkspaceReadFile,
  codexWorkspaceWriteFile,
  neuroGetSource,
  neuroUpdateSource,
  terminalResize,
  terminalWrite,
  type ApprovalDecision,
  type CodexModel,
  type NeuroRuntimeCommandError,
  type RuntimeCodexConfig,
} from "@/lib/tauri-bridge"
import {
  INITIAL_ALICIA_STATE,
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
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable"
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
import { routeSourceEditorRef, type SourceEditorRefKind } from "@/lib/source-editor-ref-routing"

interface SourceEditorState {
  visible: boolean
  refKind: SourceEditorRefKind | null
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
  const [sourceEditor, setSourceEditor] = useState<SourceEditorState>(
    INITIAL_SOURCE_EDITOR_STATE,
  )
  const [activeMainTab, setActiveMainTab] = useState<MainContentTab>("chat")
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
      if (refRoute.kind === "workspace" && runtime.state !== "running") {
        setSourceEditorWithRef({
          visible: true,
          refKind: "workspace",
          objectUri: refRoute.normalizedRef,
          workspaceRoot: activeWorkspace,
          displayName: inferDisplayNameFromPath(refRoute.normalizedRef),
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
        objectUri: refRoute.kind === "workspace" ? refRoute.normalizedRef : null,
        workspaceRoot: activeWorkspace,
        displayName:
          refRoute.kind === "workspace"
            ? inferDisplayNameFromPath(refRoute.normalizedRef)
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
            const resolved = await resolveAbapSourceRef(refRoute.normalizedRef)
            const loaded = await neuroGetSource(resolved.objectUri)

            if (sourceOperationSeqRef.current !== nextRequestId) {
              return
            }

            setSourceEditorWithRef({
              visible: true,
              refKind: "abap",
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
            sourceBaselineRef.current = loaded.source
            return
          }

          const loaded = await codexWorkspaceReadFile({
            path: refRoute.normalizedRef,
          })

          if (sourceOperationSeqRef.current !== nextRequestId) {
            return
          }

          setSourceEditorWithRef({
            visible: true,
            refKind: "workspace",
            objectUri: loaded.path,
            workspaceRoot: activeWorkspace,
            displayName: inferDisplayNameFromPath(loaded.path),
            language: refRoute.monacoLanguage,
            source: loaded.source,
            etag: null,
            dirty: false,
            loading: false,
            saving: false,
            error: null,
          })
          sourceBaselineRef.current = loaded.source
        } catch (error) {
          if (sourceOperationSeqRef.current !== nextRequestId) {
            return
          }

          setSourceEditorWithRef({
            visible: true,
            refKind: refRoute.kind,
            objectUri: refRoute.kind === "workspace" ? refRoute.normalizedRef : null,
            workspaceRoot: activeWorkspace,
            displayName:
              refRoute.kind === "workspace"
                ? inferDisplayNameFromPath(refRoute.normalizedRef)
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
      inferDisplayNameFromPath,
      runtime.state,
      runtime.workspace,
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
      }
    },
    [handleOpenSourceFromRef, isMobile],
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
    const requestId = beginSourceOperation()

    setSourceEditorWithRef((previous) => ({
      ...previous,
      loading: true,
      error: null,
    }))

    try {
      if (refKind === "abap") {
        const loaded = await neuroGetSource(objectUri)

        if (
          sourceOperationSeqRef.current !== requestId ||
          sourceEditorRef.current.objectUri !== objectUri
        ) {
          return
        }

        sourceBaselineRef.current = loaded.source
        setSourceEditorWithRef((previous) => ({
          ...previous,
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

      sourceBaselineRef.current = loaded.source
      setSourceEditorWithRef((previous) => ({
        ...previous,
        source: loaded.source,
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

  if (initializing) {
    return (
      <div className="h-screen w-screen flex items-center justify-center bg-background p-4">
        <div className="w-full max-w-3xl border border-panel-border bg-panel-bg rounded-md shadow-md">
          <div className="flex items-center gap-2 px-4 py-3 border-b border-panel-border text-terminal-fg/90 text-sm">
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
    <div className="h-screen w-screen flex flex-col bg-background overflow-hidden">
      <TitleBar connected={runtime.connected} workspace={runtime.workspace} />
      {isMobile ? (
        <div className="flex-1 min-h-0 flex flex-col">
          <div className="h-[40%] min-h-[260px] border-b border-panel-border">
            <Sidebar
              state={aliciaState}
              modelLabel={currentModelLabel}
              sessionPid={runtime.pid}
              runtimeState={runtime.state}
              onOpenPanel={(panel) => {
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
                setAliciaState((prev) => ({ ...prev, activePanel: panel }))
              }}
              onStartSession={() => {
                threadIdRef.current = null
                reviewRoutingRef.current = false
                setMessages([])
                setPendingApprovals([])
                setPendingUserInput(null)
                setTurnDiff(null)
                setTurnPlan(null)
                void ensureRuntimeSession(true)
                void refreshWorkspaceChanges()
              }}
              onStopSession={() => {
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
                  } catch (error) {
                    addMessage("system", `[session] failed to stop: ${String(error)}`)
                  }
                })()
              }}
              onResumeSession={() => {
                void openSessionPanel("resume")
              }}
              onForkSession={() => {
                void openSessionPanel("fork")
              }}
              onSelectSession={(sessionId) => {
                void handleSessionSelect(sessionId, "switch")
              }}
              onStartReview={() => {
                setAliciaState((prev) => ({ ...prev, activePanel: "review" }))
                void refreshWorkspaceChanges()
              }}
              onOpenFileChangeInEditor={onOpenInEditor}
            />
          </div>

          <Tabs
            value={activeMainTab}
            onValueChange={handleMainTabChange}
            className="flex-1 min-h-0 gap-0"
          >
            <div className="px-3 py-2 border-b border-panel-border">
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
            </TabsContent>

            <TabsContent value="editor" forceMount className="flex-1 min-h-0 data-[state=inactive]:hidden">
              {sourceEditor.visible ? (
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
                  onChangeSource={(value) => {
                    setSourceEditorWithRef((previous) => ({
                      ...previous,
                      source: value,
                      dirty: Boolean(previous.objectUri) && value !== sourceBaselineRef.current,
                      error: null,
                    }))
                  }}
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
              )}
            </TabsContent>

            <TabsContent value="terminal" forceMount className="flex-1 min-h-0 data-[state=inactive]:hidden">
              <TerminalPane
                tabs={terminalTabs}
                activeTerminalId={activeTerminalId}
                terminalContainerRef={terminalContainerRef}
                onSelectTab={setActiveTerminalId}
                onCloseTab={(id) => {
                  void closeTerminalTab(id)
                }}
                onCreateTab={() => {
                  void createTerminalTab(runtime.workspace)
                }}
              />
            </TabsContent>
          </Tabs>
        </div>
      ) : (
        <ResizablePanelGroup direction="horizontal" className="flex-1 min-h-0">
          <ResizablePanel defaultSize={20} minSize={14} maxSize={28}>
            <Sidebar
              state={aliciaState}
              modelLabel={currentModelLabel}
              sessionPid={runtime.pid}
              runtimeState={runtime.state}
              onOpenPanel={(panel) => {
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
                setAliciaState((prev) => ({ ...prev, activePanel: panel }))
              }}
              onStartSession={() => {
                threadIdRef.current = null
                reviewRoutingRef.current = false
                setMessages([])
                setPendingApprovals([])
                setPendingUserInput(null)
                setTurnDiff(null)
                setTurnPlan(null)
                void ensureRuntimeSession(true)
                void refreshWorkspaceChanges()
              }}
              onStopSession={() => {
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
                  } catch (error) {
                    addMessage("system", `[session] failed to stop: ${String(error)}`)
                  }
                })()
              }}
              onResumeSession={() => {
                void openSessionPanel("resume")
              }}
              onForkSession={() => {
                void openSessionPanel("fork")
              }}
              onSelectSession={(sessionId) => {
                void handleSessionSelect(sessionId, "switch")
              }}
              onStartReview={() => {
                setAliciaState((prev) => ({ ...prev, activePanel: "review" }))
                void refreshWorkspaceChanges()
              }}
              onOpenFileChangeInEditor={onOpenInEditor}
            />
          </ResizablePanel>
          <ResizableHandle withHandle />
          <>
            <ResizablePanel defaultSize={28} minSize={18}>
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
                onChangeSource={(value) => {
                  setSourceEditorWithRef((previous) => ({
                    ...previous,
                    source: value,
                    dirty: Boolean(previous.objectUri) && value !== sourceBaselineRef.current,
                    error: null,
                  }))
                }}
                onReload={() => {
                  void handleReloadSource()
                }}
                onSave={() => {
                  void handleSaveSource()
                }}
                onClose={handleCloseEditor}
              />
            </ResizablePanel>
            <ResizableHandle withHandle />
          </>
          <ResizablePanel defaultSize={52} minSize={40}>
            <ResizablePanelGroup direction="vertical">
              <ResizablePanel defaultSize={62} minSize={40}>
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
              </ResizablePanel>
              <ResizableHandle withHandle />
              <ResizablePanel defaultSize={38} minSize={20}>
                <TerminalPane
                  tabs={terminalTabs}
                  activeTerminalId={activeTerminalId}
                  terminalContainerRef={terminalContainerRef}
                  onSelectTab={setActiveTerminalId}
                  onCloseTab={(id) => {
                    void closeTerminalTab(id)
                  }}
                  onCreateTab={() => {
                    void createTerminalTab(runtime.workspace)
                  }}
                />
              </ResizablePanel>
            </ResizablePanelGroup>
          </ResizablePanel>
        </ResizablePanelGroup>
      )}
      <StatusBar
        state={aliciaState}
        modelLabel={currentModelLabel}
        runtime={{
          connected: runtime.connected,
          state: runtime.state,
          sessionId: runtime.sessionId,
        }}
        usage={statusSignals.usage}
        reasoning={statusSignals.reasoning}
        isThinking={isThinking}
        onOpenPanel={(panel) => {
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
          setAliciaState((prev) => ({ ...prev, activePanel: panel }))
        }}
      />
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
          onNewSession={() => {
            threadIdRef.current = null
            reviewRoutingRef.current = false
            setMessages([])
            setPendingApprovals([])
            setPendingUserInput(null)
            setTurnDiff(null)
            setTurnPlan(null)
            void ensureRuntimeSession(true)
            void refreshWorkspaceChanges()
          }}
          onClose={() => setAliciaState((prev) => ({ ...prev, activePanel: null }))}
        />
      )}
    </div>
  )
}




