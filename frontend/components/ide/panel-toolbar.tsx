"use client"

import {
  Columns3,
  MessageSquare,
  PanelLeft,
  PanelBottom,
  TerminalSquare,
  FileCode2,
  Focus,
  Sparkles,
} from "lucide-react"
import { cn } from "@/lib/utils"

export type IdeViewMode = "chat" | "split" | "editor"
export type IdeShellMode = "normal" | "focus" | "zen"

interface IdePanelToolbarProps {
  shellMode: IdeShellMode
  viewMode: IdeViewMode
  sidebarVisible: boolean
  terminalVisible: boolean
  compact?: boolean
  onToggleSidebar: () => void
  onToggleTerminal: () => void
  onChangeViewMode: (mode: IdeViewMode) => void
  onChangeShellMode: (mode: IdeShellMode) => void
}

export function IdePanelToolbar({
  shellMode,
  viewMode,
  sidebarVisible,
  terminalVisible,
  compact = false,
  onToggleSidebar,
  onToggleTerminal,
  onChangeViewMode,
  onChangeShellMode,
}: IdePanelToolbarProps) {
  const immersiveMode = shellMode !== "normal"

  return (
    <div
      className={cn(
        "flex items-center gap-1",
        compact
          ? "h-full rounded-sm px-1 text-muted-foreground"
          : "h-[var(--ide-toolbar-height)] shrink-0 border-b border-[var(--ide-border-subtle)] bg-[var(--ide-surface-1)] px-2",
      )}
      role="toolbar"
      aria-label="Controles de layout"
    >
      <ToolbarButton
        label={sidebarVisible ? "Ocultar painel lateral" : "Mostrar painel lateral"}
        active={sidebarVisible}
        disabled={immersiveMode}
        compact={compact}
        onClick={onToggleSidebar}
      >
        <PanelLeft className="h-3.5 w-3.5" />
      </ToolbarButton>

      <span className="mx-1 h-3 w-px bg-[var(--ide-border-subtle)]" />

      <ToolbarButton
        label="Somente chat"
        active={viewMode === "chat"}
        compact={compact}
        onClick={() => onChangeViewMode("chat")}
      >
        <MessageSquare className="h-3.5 w-3.5" />
      </ToolbarButton>
      <ToolbarButton
        label="Chat + editor"
        active={viewMode === "split"}
        compact={compact}
        onClick={() => onChangeViewMode("split")}
      >
        <Columns3 className="h-3.5 w-3.5" />
      </ToolbarButton>
      <ToolbarButton
        label="Somente editor"
        active={viewMode === "editor"}
        compact={compact}
        onClick={() => onChangeViewMode("editor")}
      >
        <FileCode2 className="h-3.5 w-3.5" />
      </ToolbarButton>

      <span className="mx-1 h-3 w-px bg-[var(--ide-border-subtle)]" />

      <ToolbarButton
        label={terminalVisible ? "Ocultar terminal" : "Mostrar terminal"}
        active={terminalVisible}
        disabled={immersiveMode}
        compact={compact}
        onClick={onToggleTerminal}
      >
        <TerminalSquare className="h-3.5 w-3.5" />
      </ToolbarButton>

      <span className="mx-1 h-3 w-px bg-[var(--ide-border-subtle)]" />

      <ToolbarButton
        label={shellMode === "focus" ? "Sair do modo foco (Esc)" : "Entrar no modo foco"}
        active={shellMode === "focus"}
        compact={compact}
        onClick={() => onChangeShellMode(shellMode === "focus" ? "normal" : "focus")}
      >
        <Focus className="h-3.5 w-3.5" />
      </ToolbarButton>

      <ToolbarButton
        label={shellMode === "zen" ? "Sair do modo zen (Esc)" : "Entrar no modo zen"}
        active={shellMode === "zen"}
        compact={compact}
        onClick={() => onChangeShellMode(shellMode === "zen" ? "normal" : "zen")}
      >
        <Sparkles className="h-3.5 w-3.5" />
      </ToolbarButton>

      <div
        className={cn(
          "ml-auto flex items-center rounded px-1.5 py-0.5 text-[10px] text-muted-foreground",
          compact
            ? "border border-transparent"
            : "border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-0)]",
        )}
      >
        <PanelBottom className="mr-1 h-3 w-3" />
        {immersiveMode ? shellMode : "layout"}
      </div>
    </div>
  )
}

function ToolbarButton({
  label,
  active,
  disabled = false,
  compact = false,
  onClick,
  children,
}: {
  label: string
  active: boolean
  disabled?: boolean
  compact?: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      aria-pressed={active}
      aria-disabled={disabled}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "flex items-center justify-center rounded-[5px] border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-terminal-blue/50",
        compact ? "h-5 w-5" : "h-6 w-6",
        active
          ? "border-[var(--ide-border-strong)] bg-[var(--ide-active)] text-terminal-blue"
          : "border-transparent text-muted-foreground hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)] hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-transparent disabled:hover:bg-transparent disabled:hover:text-muted-foreground",
      )}
    >
      {children}
    </button>
  )
}