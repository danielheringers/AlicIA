"use client"

import { AdtPanel } from "@/components/alicia/adt-panel"
import { AppsPanel } from "@/components/alicia/apps-panel"
import { McpPanel } from "@/components/alicia/mcp-panel"
import { ModelPicker } from "@/components/alicia/model-picker"
import { PermissionsPanel } from "@/components/alicia/permissions-panel"
import { ReviewMode } from "@/components/alicia/review-mode"
import { SessionPicker } from "@/components/alicia/session-picker"
import type {
  AccountRateLimitSnapshot,
  AccountState,
  AliciaState,
  ApprovalPreset,
  ConnectedApp,
  FileChange,
  McpServer,
  ReasoningEffort,
  SandboxMode,
  Session,
} from "@/lib/alicia-types"
import type {
  ApprovalRequestState,
  DiffFileView,
  Message,
} from "@/lib/alicia-runtime-helpers"
import type { CodexModel } from "@/lib/tauri-bridge"

interface PageActivePanelsProps {
  activePanel: AliciaState["activePanel"]
  currentModel: string
  currentEffort: ReasoningEffort
  models: CodexModel[]
  modelsLoading: boolean
  modelsError: string | null
  modelsCachedAt: number | null
  modelsFromCache: boolean
  onRetryModels: () => void
  onSelectModel: (modelId: string, effort: ReasoningEffort) => void
  currentPreset: ApprovalPreset
  currentSandbox: SandboxMode
  onSelectPermission: (preset: ApprovalPreset) => void
  mcpServers: McpServer[]
  onRefreshMcpServers: (options?: { throwOnError?: boolean }) => Promise<unknown>
  activeAdtServerId: string | null
  onAdtServerSelectionChange: (
    nextServerId: string | null,
    options?: { refreshExplorer?: boolean },
  ) => void
  onOpenInEditor: (ref: string) => void
  apps: ConnectedApp[]
  account: AccountState
  rateLimits: AccountRateLimitSnapshot | null
  rateLimitsByLimitId: Record<string, AccountRateLimitSnapshot>
  onRefreshAppsAndAuth: (options?: {
    throwOnError?: boolean
    forceRefetch?: boolean
    refreshToken?: boolean
  }) => Promise<unknown>
  fileChanges: FileChange[]
  turnDiffFiles: DiffFileView[]
  pendingApprovals: ApprovalRequestState[]
  reviewMessages: Message[]
  isReviewThinking: boolean
  isReviewComplete: boolean
  onRunReview: () => void
  onRunReviewFile: (selectedPath: string) => void
  onCommitApprovedReview: (payload: {
    approvedPaths: string[]
    message: string
    comments: Record<string, string>
  }) => Promise<void>
  sessions: Session[]
  sessionPickerMode: "resume" | "fork" | "list"
  sessionsPanelLoading: boolean
  sessionActionPending: {
    sessionId: string
    action: "resume" | "fork" | "switch"
  } | null
  onSessionSelect: (sessionId: string, action: "resume" | "fork" | "switch") => void
  onStartSession: () => void
  onClosePanel: () => void
}

export function PageActivePanels({
  activePanel,
  currentModel,
  currentEffort,
  models,
  modelsLoading,
  modelsError,
  modelsCachedAt,
  modelsFromCache,
  onRetryModels,
  onSelectModel,
  currentPreset,
  currentSandbox,
  onSelectPermission,
  mcpServers,
  onRefreshMcpServers,
  activeAdtServerId,
  onAdtServerSelectionChange,
  onOpenInEditor,
  apps,
  account,
  rateLimits,
  rateLimitsByLimitId,
  onRefreshAppsAndAuth,
  fileChanges,
  turnDiffFiles,
  pendingApprovals,
  reviewMessages,
  isReviewThinking,
  isReviewComplete,
  onRunReview,
  onRunReviewFile,
  onCommitApprovedReview,
  sessions,
  sessionPickerMode,
  sessionsPanelLoading,
  sessionActionPending,
  onSessionSelect,
  onStartSession,
  onClosePanel,
}: PageActivePanelsProps) {
  return (
    <>
      {activePanel === "model" && (
        <ModelPicker
          currentModel={currentModel}
          currentEffort={currentEffort}
          models={models}
          loading={modelsLoading}
          error={modelsError}
          cachedAt={modelsCachedAt}
          stale={modelsFromCache}
          onRetry={onRetryModels}
          onSelect={onSelectModel}
          onClose={onClosePanel}
        />
      )}
      {activePanel === "permissions" && (
        <PermissionsPanel
          currentPreset={currentPreset}
          currentSandbox={currentSandbox}
          onSelect={onSelectPermission}
          onClose={onClosePanel}
        />
      )}
      {activePanel === "mcp" && (
        <McpPanel
          servers={mcpServers}
          onRefresh={onRefreshMcpServers}
          onClose={onClosePanel}
        />
      )}
      {activePanel === "adt" && (
        <AdtPanel
          activeServerId={activeAdtServerId}
          onActiveServerIdChange={(nextServerId) => {
            onAdtServerSelectionChange(nextServerId, {
              refreshExplorer: true,
            })
          }}
          onOpenInEditor={onOpenInEditor}
          onClose={onClosePanel}
        />
      )}
      {activePanel === "apps" && (
        <AppsPanel
          apps={apps}
          account={account}
          rateLimits={rateLimits}
          rateLimitsByLimitId={rateLimitsByLimitId}
          onRefresh={onRefreshAppsAndAuth}
          onClose={onClosePanel}
        />
      )}
      {activePanel === "review" && (
        <ReviewMode
          fileChanges={fileChanges}
          turnDiffFiles={turnDiffFiles}
          pendingApprovals={pendingApprovals}
          reviewMessages={reviewMessages}
          isReviewThinking={isReviewThinking}
          isReviewComplete={isReviewComplete}
          onRunReview={onRunReview}
          onRunReviewFile={onRunReviewFile}
          onCommitApproved={onCommitApprovedReview}
          onOpenInEditor={onOpenInEditor}
          onClose={onClosePanel}
        />
      )}
      {activePanel === "sessions" && (
        <SessionPicker
          sessions={sessions}
          mode={sessionPickerMode}
          loading={sessionsPanelLoading}
          busyAction={sessionActionPending}
          onSelect={onSessionSelect}
          onNewSession={onStartSession}
          onClose={onClosePanel}
        />
      )}
    </>
  )
}
