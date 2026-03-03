import { useMemo, type RefObject } from "react"
import { MessageSquareText } from "lucide-react"

import { CommandInput } from "@/components/alicia/command-input"
import { ApprovalRequest } from "@/components/alicia/approval-request"
import { UserInputRequest } from "@/components/alicia/user-input-request"
import { DiffViewer } from "@/components/alicia/diff-viewer"
import { TerminalMessage } from "@/components/alicia/terminal-message"
import {
  type ApprovalDecision,
  type RuntimeMethodCapabilities,
} from "@/lib/application/runtime-types"
import {
  encodeAgentSpawnerPayload,
  mergeAgentSpawnerPayloads,
  parseAgentSpawnerPayload,
} from "@/lib/agent-spawner-events"
import {
  parseReasoningSystemMessage,
  parseUsageSystemMessage,
} from "@/lib/runtime-statusline"
import {
  parseDiffSystemMessage,
  parseUnifiedDiffFiles,
  type ApprovalRequestState,
  type DiffFileView,
  type Message,
  type RuntimeState,
  type TurnDiffState,
  type TurnPlanState,
  type UserInputRequestState,
} from "@/lib/alicia-runtime-helpers"

interface ConversationPaneProps {
  currentModelLabel: string
  reasoningEffort: string
  messages: Message[]
  isThinking: boolean
  pendingImages: string[]
  pendingMentions: string[]
  runtimeCapabilities: RuntimeMethodCapabilities
  pendingApprovals: ApprovalRequestState[]
  pendingUserInput: UserInputRequestState | null
  turnDiff: TurnDiffState | null
  turnDiffFiles: DiffFileView[]
  turnPlan: TurnPlanState | null
  runtimeState: RuntimeState["state"]
  scrollRef: RefObject<HTMLDivElement | null>
  onSubmit: (value: string) => Promise<void>
  onSlashCommand: (command: string) => Promise<void>
  onAttachImage: () => Promise<void>
  onAttachMention: () => Promise<void>
  onRemoveImage: (index: number) => void
  onRemoveMention: (index: number) => void
  onApprovalDecision: (
    actionId: string,
    decision: ApprovalDecision,
  ) => Promise<void>
  onUserInputDecision: (response: {
    actionId: string
    decision: "submit" | "cancel"
    answers?: Record<string, { answers: string[] }>
  }) => Promise<void>
  onOpenInEditor?: (ref: string) => void
}

function approvalRisk(
  approval: ApprovalRequestState,
): "low" | "medium" | "high" {
  const signal = `${approval.reason} ${approval.command} ${approval.cwd} ${approval.grantRoot}`.toLowerCase()
  if (
    signal.includes("danger") ||
    signal.includes("network") ||
    signal.includes("outside") ||
    signal.includes("full-access")
  ) {
    return "high"
  }

  if (approval.kind === "file_change" || approval.command.trim().length > 0) {
    return "medium"
  }

  return "low"
}

function approvalDescription(approval: ApprovalRequestState): string {
  const details: string[] = []
  if (approval.reason.trim().length > 0) {
    details.push(approval.reason.trim())
  }
  if (approval.cwd.trim().length > 0) {
    details.push(`cwd: ${approval.cwd}`)
  }
  if (approval.grantRoot.trim().length > 0) {
    details.push(`grant root: ${approval.grantRoot}`)
  }
  if (details.length === 0) {
    return "No additional details provided by runtime."
  }
  return details.join(" | ")
}

function planStatusLabel(status: "pending" | "inProgress" | "completed"): string {
  if (status === "pending") {
    return "pendente"
  }
  if (status === "inProgress") {
    return "em andamento"
  }
  return "concluido"
}

function planStatusColor(status: "pending" | "inProgress" | "completed"): string {
  if (status === "completed") {
    return "text-terminal-green"
  }
  if (status === "inProgress") {
    return "text-terminal-gold"
  }
  return "text-muted-foreground"
}

export function ConversationPane({
  currentModelLabel,
  reasoningEffort,
  messages,
  isThinking,
  pendingImages,
  pendingMentions,
  runtimeCapabilities,
  pendingApprovals,
  pendingUserInput,
  turnDiff,
  turnDiffFiles,
  turnPlan,
  runtimeState,
  scrollRef,
  onSubmit,
  onSlashCommand,
  onAttachImage,
  onAttachMention,
  onRemoveImage,
  onRemoveMention,
  onApprovalDecision,
  onUserInputDecision,
  onOpenInEditor,
}: ConversationPaneProps) {
  const groupedMessages = useMemo(() => {
    const grouped: Message[] = []

    for (const message of messages) {
      if (message.type === "system") {
        if (
          parseUsageSystemMessage(message.content) ||
          parseReasoningSystemMessage(message.content)
        ) {
          continue
        }

        const incomingSpawner = parseAgentSpawnerPayload(message.content)
        if (!incomingSpawner) {
          grouped.push(message)
          continue
        }

        let mergeIndex = -1
        let previousSpawner = null as ReturnType<typeof parseAgentSpawnerPayload>

        for (let index = grouped.length - 1; index >= 0; index -= 1) {
          const candidate = grouped[index]
          if (candidate.type !== "system") {
            break
          }

          const parsedCandidate = parseAgentSpawnerPayload(candidate.content)
          if (parsedCandidate) {
            mergeIndex = index
            previousSpawner = parsedCandidate
            break
          }
        }

        if (mergeIndex < 0 || !previousSpawner) {
          grouped.push(message)
          continue
        }

        const mergedPayload = mergeAgentSpawnerPayloads(previousSpawner, incomingSpawner)
        grouped[mergeIndex] = {
          ...grouped[mergeIndex],
          content: encodeAgentSpawnerPayload(mergedPayload),
          timestamp: message.timestamp,
        }
        continue
      }

      grouped.push(message)
    }

    return grouped
  }, [messages])

  const resolvedDiffsByMessageId = useMemo(() => {
    const map = new Map<
      number,
      { title?: string; emptyMessage?: string; files: DiffFileView[] }
    >()

    const currentThreadId = turnDiff?.threadId.trim() ?? ""
    const currentTurnId = turnDiff?.turnId.trim() ?? ""

    for (const message of groupedMessages) {
      if (message.type !== "system") {
        continue
      }

      const diffPayload = parseDiffSystemMessage(message.content)
      if (!diffPayload) {
        continue
      }

      if (diffPayload.version === 1) {
        map.set(message.id, {
          title: diffPayload.title,
          emptyMessage: diffPayload.emptyMessage,
          files: parseUnifiedDiffFiles(diffPayload.diff),
        })
        continue
      }

      const payloadThreadId = diffPayload.threadId?.trim() ?? ""
      const payloadTurnId = diffPayload.turnId?.trim() ?? ""
      const threadMatches =
        payloadThreadId.length === 0 ||
        (currentThreadId.length > 0 && payloadThreadId === currentThreadId)
      const turnMatches =
        payloadTurnId.length === 0 ||
        (currentTurnId.length > 0 && payloadTurnId === currentTurnId)

      map.set(message.id, {
        title: diffPayload.title,
        emptyMessage: diffPayload.emptyMessage,
        files: threadMatches && turnMatches ? turnDiffFiles : [],
      })
    }

    return map
  }, [groupedMessages, turnDiff, turnDiffFiles])

  return (
    <section className="alicia-panel-chrome flex h-full min-h-0 flex-col overflow-hidden">
      <div className="alicia-panel-titlebar flex items-center justify-between gap-2 px-2.5">
        <div className="flex min-w-0 items-center gap-1.5">
          <MessageSquareText className="h-3 w-3 text-terminal-blue/90" />
          <span className="truncate text-[11px] font-medium text-terminal-fg/90">
            Conversa estruturada
          </span>
        </div>
        <div className="flex min-w-0 items-center gap-1.5">
          <span className="truncate rounded border border-terminal-blue/20 bg-terminal-blue/10 px-1.5 py-0 text-[10px] text-terminal-blue">
            {currentModelLabel}
          </span>
          <span className="truncate rounded border border-terminal-green/20 bg-terminal-green/10 px-1.5 py-0 text-[10px] text-terminal-green">
            {reasoningEffort}
          </span>
        </div>
      </div>
      <div ref={scrollRef} className="flex-1 overflow-y-auto bg-[var(--ide-surface-0)]">
        {groupedMessages.map((message) => (
          <TerminalMessage
            key={message.id}
            type={message.type}
            content={message.content}
            timestamp={message.timestamp}
            resolvedDiff={resolvedDiffsByMessageId.get(message.id) ?? null}
            onOpenInEditor={onOpenInEditor}
          />
        ))}

        {turnPlan && turnPlan.plan.length > 0 && (
          <div className="mx-5 my-2 ml-14 rounded-md border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-1)]/75 px-3 py-2">
            <div className="text-xs font-semibold text-terminal-fg mb-1">Plano da rodada</div>
            {turnPlan.explanation && (
              <div className="text-xs text-terminal-fg/75 mb-2">{turnPlan.explanation}</div>
            )}
            <div className="space-y-1">
              {turnPlan.plan.map((step, index) => (
                <div key={`${turnPlan.turnId}-${index}`} className="flex items-center gap-2 text-xs">
                  <span className="text-muted-foreground/60 tabular-nums w-4">{index + 1}.</span>
                  <span className="text-terminal-fg/90 flex-1">{step.step}</span>
                  <span className={planStatusColor(step.status)}>{planStatusLabel(step.status)}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {turnDiffFiles.length > 0 && (
          <div className="px-5 pb-3">
            <div className="ml-9 text-xs text-muted-foreground mb-1">Diff agregado da rodada</div>
            {turnDiffFiles.map((file, index) => (
              <div key={`${file.filename}-${index}`} className="ml-9">
                <DiffViewer
                  filename={file.filename}
                  lines={file.lines}
                  onOpenInEditor={onOpenInEditor}
                />
              </div>
            ))}
          </div>
        )}

        {pendingApprovals.map((approval) => (
          <ApprovalRequest
            key={approval.actionId}
            toolName={
              approval.kind === "file_change"
                ? `fileChange:${approval.itemId || "pending"}`
                : approval.command || `command:${approval.itemId || "pending"}`
            }
            description={approvalDescription(approval)}
            risk={approvalRisk(approval)}
            onApprove={() => {
              void onApprovalDecision(approval.actionId, "accept")
            }}
            onAlwaysApprove={() => {
              void onApprovalDecision(approval.actionId, "acceptForSession")
            }}
            onDeny={() => {
              void onApprovalDecision(approval.actionId, "decline")
            }}
            onCancel={() => {
              void onApprovalDecision(approval.actionId, "cancel")
            }}
          />
        ))}

        {pendingUserInput && (
          <UserInputRequest
            request={pendingUserInput}
            onSubmit={async (actionId, answers) => {
              await onUserInputDecision({
                actionId,
                decision: "submit",
                answers,
              })
            }}
            onCancel={async (actionId) => {
              await onUserInputDecision({
                actionId,
                decision: "cancel",
              })
            }}
          />
        )}

        {isThinking && <TerminalMessage type="agent" content="" thinking />}
      </div>

      <CommandInput
        onSubmit={onSubmit}
        onSlashCommand={onSlashCommand}
        onAttachImage={onAttachImage}
        onAttachMention={onAttachMention}
        onRemoveImage={onRemoveImage}
        onRemoveMention={onRemoveMention}
        pendingImages={pendingImages}
        pendingMentions={pendingMentions}
        runtimeCapabilities={runtimeCapabilities}
        disabled={runtimeState === "starting" || runtimeState === "stopping"}
      />
    </section>
  )
}
