"use client"

import { ShieldAlert, Check, X, AlertTriangle } from "lucide-react"

interface ApprovalRequestProps {
  toolName: string
  description: string
  risk: "low" | "medium" | "high"
  onApprove: () => void
  onDeny: () => void
  onCancel?: () => void
  onAlwaysApprove?: () => void
}

const riskConfig = {
  low: { color: "terminal-blue", label: "Low Risk", icon: ShieldAlert },
  medium: { color: "terminal-gold", label: "Medium Risk", icon: AlertTriangle },
  high: { color: "terminal-red", label: "High Risk", icon: AlertTriangle },
}

export function ApprovalRequest({
  toolName,
  description,
  risk,
  onApprove,
  onDeny,
  onCancel,
  onAlwaysApprove,
}: ApprovalRequestProps) {
  const config = riskConfig[risk]

  return (
    <div
      className="mx-5 my-2 ml-14 overflow-hidden rounded-md border-2 border-dashed"
      style={{ borderColor: `var(--${config.color})` }}
    >
      <div
        className="flex items-center gap-2 border-b border-[var(--ide-border-subtle)] px-3 py-2"
        style={{
          backgroundColor: `color-mix(in srgb, var(--${config.color}) 8%, transparent)`,
        }}
      >
        <config.icon className="w-4 h-4" style={{ color: `var(--${config.color})` }} />
        <span className="text-xs font-semibold text-terminal-fg">Approval Required</span>
        <span
          className="text-[10px] px-1.5 py-0.5 rounded ml-auto"
          style={{
            color: `var(--${config.color})`,
            backgroundColor: `color-mix(in srgb, var(--${config.color}) 15%, transparent)`,
          }}
        >
          {config.label}
        </span>
      </div>
      <div className="bg-[var(--ide-surface-0)]/60 px-3 py-2.5">
        <p className="text-xs text-muted-foreground mb-1">
          Agent wants to execute:{" "}
          <span className="text-terminal-cyan font-mono">{toolName}</span>
        </p>
        <p className="text-xs text-terminal-fg/70">{description}</p>
        <div className="flex flex-wrap items-center gap-2 mt-3">
          <button
            onClick={onApprove}
            className="flex items-center gap-1 px-3 py-1.5 rounded text-xs bg-terminal-green/15 text-terminal-green hover:bg-terminal-green/25 border border-terminal-green/20 transition-colors"
          >
            <Check className="w-3 h-3" />
            Approve
          </button>
          <button
            onClick={onDeny}
            className="flex items-center gap-1 px-3 py-1.5 rounded text-xs bg-terminal-red/15 text-terminal-red hover:bg-terminal-red/25 border border-terminal-red/20 transition-colors"
          >
            <X className="w-3 h-3" />
            Deny
          </button>
          {onCancel && (
            <button
              onClick={onCancel}
              className="flex items-center gap-1 rounded border border-[var(--ide-border-subtle)] px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-[var(--ide-hover)] hover:text-terminal-fg"
            >
              Cancel Turn
            </button>
          )}
          {onAlwaysApprove && (
            <button
              onClick={onAlwaysApprove}
              className="flex items-center gap-1 rounded border border-[var(--ide-border-subtle)] px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-[var(--ide-hover)] hover:text-terminal-fg"
            >
              Approve for Session
            </button>
          )}
        </div>
      </div>
    </div>
  )
}
