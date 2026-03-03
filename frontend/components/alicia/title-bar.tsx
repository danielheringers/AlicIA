"use client"

import { useEffect, useRef, useState } from "react"
import { Circle, FolderOpen, Wifi } from "lucide-react"
import { cn } from "@/lib/utils"

interface TitleBarProps {
  connected: boolean
  workspace: string
  version?: string
  onOpenWorkspaceFolder?: () => void | Promise<void>
  openWorkspaceFolderBusy?: boolean
  onOpenAdtConnections?: () => void
}

const MENU_ITEMS = ["Editar", "Selecao", "Exibir", "Ir", "Executar", "Terminal", "Ajuda"]

export function TitleBar({
  connected,
  workspace,
  version = "v0.1.0-alpha",
  onOpenWorkspaceFolder,
  openWorkspaceFolderBusy = false,
  onOpenAdtConnections,
}: TitleBarProps) {
  const workspaceLabel = workspace && workspace.trim().length > 0 ? workspace : "workspace"
  const [fileMenuOpen, setFileMenuOpen] = useState(false)
  const fileMenuRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    if (!fileMenuOpen) {
      return
    }

    const handlePointerDown = (event: MouseEvent) => {
      if (
        fileMenuRef.current &&
        event.target instanceof Node &&
        !fileMenuRef.current.contains(event.target)
      ) {
        setFileMenuOpen(false)
      }
    }

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setFileMenuOpen(false)
      }
    }

    window.addEventListener("mousedown", handlePointerDown)
    window.addEventListener("keydown", handleEscape)
    return () => {
      window.removeEventListener("mousedown", handlePointerDown)
      window.removeEventListener("keydown", handleEscape)
    }
  }, [fileMenuOpen])

  return (
    <header
      className="flex h-[var(--ide-titlebar-height)] shrink-0 items-center gap-1.5 border-b border-[var(--ide-border-subtle)] bg-[var(--ide-surface-1)] px-2 select-none"
      aria-label="Barra superior"
    >
      <div className="flex items-center gap-1.5" aria-hidden="true">
        <span className="h-2.5 w-2.5 rounded-full bg-[#f87171]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#fbbf24]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#22d3a5]" />
      </div>

      <div className="flex min-w-0 items-center gap-2">
        <div className="flex items-center gap-1.5">
          <span className="text-[10px] font-semibold tracking-[0.08em] text-terminal-green">ALICIA</span>
          <span className="rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-2)] px-1 py-0 text-[9px] text-muted-foreground">
            {version}
          </span>
        </div>

        <nav className="relative flex items-center gap-0.5" aria-label="Menu principal">
          <div ref={fileMenuRef} className="relative">
            <button
              type="button"
              className="rounded px-1 py-0 text-[10px] text-muted-foreground hover:bg-[var(--ide-hover)] hover:text-terminal-fg"
              onClick={() => {
                setFileMenuOpen((previous) => !previous)
              }}
              aria-haspopup="menu"
              aria-expanded={fileMenuOpen}
            >
              Arquivo
            </button>
            {fileMenuOpen ? (
              <div
                role="menu"
                className="absolute left-0 top-full z-50 mt-1 min-w-[11rem] rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-2)] p-1 shadow-lg shadow-black/35"
              >
                <button
                  type="button"
                  role="menuitem"
                  className="w-full rounded px-2 py-1 text-left text-xs text-terminal-fg/90 transition-colors hover:bg-[var(--ide-hover)] disabled:cursor-not-allowed disabled:opacity-50"
                  onClick={() => {
                    setFileMenuOpen(false)
                    void onOpenWorkspaceFolder?.()
                  }}
                  disabled={openWorkspaceFolderBusy || !onOpenWorkspaceFolder}
                >
                  Abrir pasta...
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className="w-full rounded px-2 py-1 text-left text-xs text-terminal-fg/90 transition-colors hover:bg-[var(--ide-hover)] disabled:cursor-not-allowed disabled:opacity-50"
                  onClick={() => {
                    setFileMenuOpen(false)
                    onOpenAdtConnections?.()
                  }}
                  disabled={!onOpenAdtConnections}
                >
                  Conexoes ADT...
                </button>
              </div>
            ) : null}
          </div>
          {MENU_ITEMS.map((item) => (
            <button
              key={item}
              type="button"
              disabled
              className="hidden rounded px-1 py-0 text-[10px] text-muted-foreground hover:bg-[var(--ide-hover)] hover:text-terminal-fg lg:inline-flex"
              aria-label={`${item} (somente visual)`}
            >
              {item}
            </button>
          ))}
        </nav>
      </div>

      <div className="ml-auto flex min-w-0 items-center gap-0.5 text-[10px] text-muted-foreground">
        <div className="inline-flex min-w-0 items-center gap-1 rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-2)] px-1 py-0">
          <Wifi className={cn("h-3 w-3", connected ? "text-terminal-green" : "text-terminal-red")} />
          <Circle
            className={cn(
              "h-2 w-2",
              connected
                ? "fill-terminal-green text-terminal-green status-pulse"
                : "fill-terminal-red text-terminal-red",
            )}
            aria-hidden="true"
          />
          <span className="sr-only">{connected ? "Connected" : "Offline"}</span>
          <span
            className={cn("hidden max-w-[96px] truncate sm:inline", connected && "text-terminal-green/90")}
            aria-hidden="true"
          >
            {connected ? "Conectado" : "Offline"}
          </span>
        </div>

        <span className="hidden text-muted-foreground/40 sm:inline">|</span>

        <div className="flex min-w-0 items-center gap-1 rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-2)] px-1 py-0 text-terminal-fg/80">
          <FolderOpen className="h-3 w-3 shrink-0 text-muted-foreground" />
          <span className="max-w-[40vw] truncate" title={workspaceLabel}>
            {workspaceLabel}
          </span>
        </div>
      </div>
    </header>
  )
}
