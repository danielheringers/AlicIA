"use client"

import { useEffect, useMemo, useState } from "react"
import {
  GitBranch,
  CircleDot,
  Cpu,
  Shield,
  PlugZap,
  ShieldCheck,
  ShieldOff,
  Zap,
  Brain,
  ArrowUpRight,
  ArrowDownLeft,
  Clock,
  Gauge,
  AppWindow,
  Database,
} from "lucide-react"
import {
  type AliciaState,
  type ApprovalPreset,
  APPROVAL_PRESETS,
} from "@/lib/alicia-types"
import { type UsageStats } from "@/lib/runtime-statusline"

interface RuntimeStatus {
  connected: boolean
  state: "idle" | "starting" | "running" | "stopping" | "error"
  sessionId: number | null
}

interface StatusBarProps {
  state: AliciaState
  modelLabel: string
  runtime: RuntimeStatus
  usage: UsageStats | null
  reasoning: string | null
  isThinking: boolean
  panelToolbar?: React.ReactNode
  adtActiveServerId?: string | null
  onOpenPanel: (panel: AliciaState["activePanel"]) => void
}

const approvalIcons: Record<ApprovalPreset, typeof Shield> = {
  "read-only": ShieldCheck,
  auto: Shield,
  "full-access": ShieldOff,
}

const approvalColors: Record<ApprovalPreset, string> = {
  "read-only": "text-terminal-blue",
  auto: "text-terminal-green",
  "full-access": "text-terminal-red",
}

const effortColors: Record<string, string> = {
  none: "#77797c",
  minimal: "#77797c",
  low: "#6c95eb",
  medium: "#c9a26d",
  high: "#39CC9B",
  xhigh: "#9a94e9",
}

function TokenRing({
  percent,
  color,
  size = 14,
}: {
  percent: number
  color: string
  size?: number
}) {
  const r = (size - 2) / 2
  const circ = 2 * Math.PI * r
  const offset = circ - (percent / 100) * circ

  return (
    <svg width={size} height={size} className="shrink-0 -rotate-90">
      <circle
        cx={size / 2}
        cy={size / 2}
        r={r}
        fill="none"
        stroke="currentColor"
        strokeWidth={1.5}
        className="text-muted-foreground/10"
      />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={r}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
        strokeDasharray={circ}
        strokeDashoffset={offset}
        strokeLinecap="round"
        className="transition-all duration-700 ease-out"
      />
    </svg>
  )
}

function ReasoningPulse({
  effort,
  label,
  active,
}: {
  effort: string
  label: string
  active: boolean
}) {
  const color = effortColors[effort] || "#77797c"

  return (
    <div className="flex min-w-0 items-center gap-1.5">
      <div className="relative flex items-center justify-center w-3 h-3 shrink-0">
        <div
          className={`absolute w-full h-full rounded-full opacity-30 ${active ? "animate-ping" : ""}`}
          style={{ backgroundColor: color, animationDuration: "2s" }}
        />
        <div className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: color }} />
      </div>
      <Brain className="w-2.5 h-2.5 shrink-0" style={{ color }} />
      <span className="max-w-[220px] truncate" style={{ color }} title={label}>
        {label}
      </span>
    </div>
  )
}

function formatNum(value: number): string {
  return value.toLocaleString("en-US")
}

function formatElapsed(seconds: number): string {
  const minutes = Math.floor(seconds / 60)
  const remaining = seconds % 60
  return `${minutes}:${String(remaining).padStart(2, "0")}`
}

function SessionElapsed({
  running,
  sessionId,
}: {
  running: boolean
  sessionId: number | null
}) {
  const [elapsedSeconds, setElapsedSeconds] = useState(0)

  useEffect(() => {
    if (!running) {
      return
    }

    const interval = window.setInterval(() => {
      setElapsedSeconds((previous) => previous + 1)
    }, 1000)

    return () => {
      window.clearInterval(interval)
    }
  }, [running, sessionId])

  return <span className="tabular-nums">{formatElapsed(elapsedSeconds)}</span>
}

function authModeLabel(mode: AliciaState["account"]["authMode"]): string {
  if (mode === "chatgpt") return "chatgpt"
  if (mode === "api_key") return "api-key"
  if (mode === "chatgpt_auth_tokens") return "tokens"
  if (mode === "none") return "logged-out"
  return "unknown"
}

function authModeColor(mode: AliciaState["account"]["authMode"]): string {
  if (mode === "chatgpt" || mode === "api_key" || mode === "chatgpt_auth_tokens") {
    return "text-terminal-green"
  }
  if (mode === "none") {
    return "text-terminal-gold"
  }
  return "text-muted-foreground"
}

function Separator() {
  return <div className="mx-0.5 h-2 w-px bg-terminal-blue/25" />
}

export function StatusBar({
  state,
  modelLabel,
  runtime,
  usage,
  reasoning,
  isThinking,
  panelToolbar,
  adtActiveServerId = null,
  onOpenPanel,
}: StatusBarProps) {
  const ApprovalIcon = approvalIcons[state.approvalPreset]
  const connectedMcps = state.mcpServers.filter((server) => server.status === "connected")
  const totalTools = state.mcpServers.reduce((sum, server) => sum + server.tools.length, 0)

  const elapsedKey = `${runtime.sessionId ?? "none"}:${runtime.state === "running" ? "running" : "idle"}`

  const reasoningLabel = useMemo(() => {
    if (!isThinking) {
      return "idle"
    }
    if (reasoning && reasoning.trim().length > 0) {
      return reasoning.trim()
    }
    if (runtime.state === "starting") {
      return "starting"
    }
    return "thinking"
  }, [isThinking, reasoning, runtime.state])

  const usagePercent = useMemo(() => {
    if (!usage) {
      return 0
    }
    return Math.min((usage.total / 128000) * 100, 100)
  }, [usage])

  const effortColor = effortColors[state.reasoningEffort] || "#77797c"
  const diagnosticsModified = state.fileChanges.filter((entry) => entry.status === "modified").length
  const diagnosticsAdded = state.fileChanges.filter((entry) => entry.status === "added").length
  const approvalDescription =
    APPROVAL_PRESETS[state.approvalPreset]?.description || state.sandboxMode
  const accountLabel = authModeLabel(state.account.authMode)
  const accountColorClass = authModeColor(state.account.authMode)

  return (
    <div className="alicia-status-bar flex h-[var(--ide-status-height)] shrink-0 select-none items-center justify-between gap-1 px-1.5 text-[10px] text-muted-foreground">
      <div className="flex min-w-0 flex-1 items-center gap-0.5">
        <div className="flex items-center gap-1 rounded px-1 py-0 transition-colors hover:bg-[var(--ide-hover)]">
          <GitBranch className="w-3 h-3 text-terminal-purple" />
          <span className="text-terminal-purple">main</span>
        </div>

        <Separator />

        <div className="flex items-center gap-1 rounded px-1 py-0">
          <CircleDot
            className={`w-3 h-3 ${runtime.connected ? "text-terminal-green" : "text-terminal-red"}`}
          />
          <span className="tabular-nums">{diagnosticsModified}</span>
          <span className="text-terminal-gold ml-0.5 tabular-nums">{diagnosticsAdded}</span>
        </div>

        <Separator />

        <div className="flex min-w-0 flex-1 items-center rounded px-1 py-0">
          <ReasoningPulse effort={state.reasoningEffort} label={reasoningLabel} active={isThinking} />
        </div>
      </div>

      <div className="flex h-full min-w-0 flex-1 items-center justify-center px-1 lg:min-w-[260px]">
        <div className="max-w-full overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
          {panelToolbar}
        </div>
      </div>

      <div className="flex min-w-0 shrink-0 items-center gap-0">
        <div className="hidden md:flex items-center gap-1 rounded alicia-status-chip px-1.5 py-0 text-[9px]">
          <TokenRing percent={usagePercent} color={effortColor} size={14} />
          <Zap className="w-2.5 h-2.5 text-terminal-gold" />
          <span className="text-terminal-fg/80 tabular-nums">
            {usage ? formatNum(usage.total) : "-"}
          </span>
          <span className="text-muted-foreground/30">|</span>
          <ArrowDownLeft className="w-2.5 h-2.5 text-terminal-blue/60" />
          <span className="tabular-nums">{usage ? formatNum(usage.input) : "-"}</span>
          <ArrowUpRight className="w-2.5 h-2.5 text-terminal-green/60" />
          <span className="tabular-nums">{usage ? formatNum(usage.output) : "-"}</span>
        </div>

        <Separator />

        <button
          type="button"
          aria-label="Abrir permissoes"
          onClick={() => onOpenPanel("permissions")}
          title={approvalDescription}
            className={`flex items-center gap-1 rounded px-1 py-0 transition-colors hover:bg-[var(--ide-hover)] ${approvalColors[state.approvalPreset]}`}
        >
          <ApprovalIcon className="w-3 h-3" />
          <span>
            {state.sandboxMode === "workspace-write"
              ? "auto"
              : state.sandboxMode === "read-only"
                ? "ro"
                : "full"}
          </span>
        </button>

        <Separator />

        <button
          type="button"
          aria-label="Abrir painel MCP"
          onClick={() => onOpenPanel("mcp")}
          className="flex items-center gap-1 rounded px-1 py-0 transition-colors hover:bg-[var(--ide-hover)]"
        >
          <PlugZap className="w-3 h-3 text-terminal-purple" />
          <span>{connectedMcps.length}</span>
          <span className="text-muted-foreground/30">({totalTools})</span>
        </button>

        <Separator />

        <button
          type="button"
          aria-label="Abrir painel ADT"
          onClick={() => onOpenPanel("adt")}
          className="flex min-w-0 max-w-[160px] items-center gap-1 rounded px-1 py-0 transition-colors hover:bg-[var(--ide-hover)]"
        >
          <Database className="w-3 h-3 text-terminal-cyan" />
          <span className="truncate text-terminal-fg/80" title={adtActiveServerId ?? "sem-servidor"}>
            {adtActiveServerId ?? "adt"}
          </span>
        </button>

        <Separator />

        <button
          type="button"
          aria-label="Abrir painel de apps"
          onClick={() => onOpenPanel("apps")}
          className="flex min-w-0 max-w-[168px] items-center gap-1 rounded px-1 py-0 transition-colors hover:bg-[var(--ide-hover)]"
        >
          <AppWindow className="w-3 h-3 text-terminal-cyan" />
          <span className={`${accountColorClass} truncate`} title={accountLabel}>
            {accountLabel}
          </span>
          <span className="shrink-0 text-muted-foreground/30">({state.apps.length})</span>
        </button>

        <Separator />

        <button
          type="button"
          aria-label="Abrir seletor de modelo"
          onClick={() => onOpenPanel("model")}
          className="flex min-w-0 max-w-[240px] items-center gap-1 rounded px-1 py-0 transition-colors hover:bg-[var(--ide-hover)]"
        >
          <Cpu className="w-3 h-3 text-terminal-cyan shrink-0" />
          <span className="min-w-0 truncate text-terminal-fg/80" title={modelLabel}>
            {modelLabel}
          </span>
          <span
            className="shrink-0 rounded px-1 text-[8px] font-semibold uppercase tracking-wide"
            style={{ color: effortColor, backgroundColor: `${effortColor}15` }}
          >
            {state.reasoningEffort}
          </span>
        </button>

        <Separator />

        <div className="hidden lg:flex items-center gap-1 px-1 py-0">
          <Gauge className="w-2.5 h-2.5" />
          <span>{runtime.state}</span>
        </div>

        <Separator />

        <div className="hidden lg:flex items-center gap-1 px-1 py-0">
          <Clock className="w-2.5 h-2.5" />
          <SessionElapsed
            key={elapsedKey}
            running={runtime.state === "running"}
            sessionId={runtime.sessionId}
          />
        </div>

        <Separator />

        <span className="hidden xl:inline px-1">UTF-8</span>
        <Separator />
        <span className="hidden xl:inline px-1 text-terminal-gold">Rust</span>
      </div>
    </div>
  )
}
