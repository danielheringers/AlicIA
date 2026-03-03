import { type RefObject } from "react"
import { Plus, TerminalSquare, X } from "lucide-react"
import { type TerminalTab } from "@/lib/alicia-runtime-helpers"

interface TerminalPaneProps {
  tabs: TerminalTab[]
  activeTerminalId: number | null
  terminalContainerRef: RefObject<HTMLDivElement | null>
  onSelectTab: (id: number) => void
  onCloseTab: (id: number) => void
  onCreateTab: () => void
}

export function TerminalPane({
  tabs,
  activeTerminalId,
  terminalContainerRef,
  onSelectTab,
  onCloseTab,
  onCreateTab,
}: TerminalPaneProps) {
  return (
    <section className="alicia-panel-chrome flex h-full min-h-0 flex-col overflow-hidden" aria-label="Painel de terminal">
      <div className="alicia-panel-titlebar flex h-[30px] items-center gap-1 border-b border-[var(--ide-border-strong)] bg-[var(--ide-surface-2)] px-1.5 shadow-[inset_0_-1px_0_0_#0d0e11]">
        <div className="inline-flex shrink-0 items-center gap-1 px-1 text-[10px] uppercase tracking-wide text-muted-foreground">
          <TerminalSquare className="h-3.5 w-3.5 text-terminal-blue/90" />
          terminal
        </div>
        <div className="flex min-w-0 flex-1 items-center overflow-x-auto">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`group inline-flex h-[28px] shrink-0 items-center gap-1.5 border-r border-t px-1.5 text-[10px] ${
                tab.id === activeTerminalId
                  ? "border-[var(--ide-border-strong)] bg-[var(--terminal-bg)] text-terminal-fg"
                  : "border-transparent text-muted-foreground hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)]"
              }`}
            >
              <button
                type="button"
                aria-label={`Abrir aba ${tab.title}`}
                onClick={() => onSelectTab(tab.id)}
                className="inline-flex items-center gap-1.5 text-[11px] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-terminal-blue/50"
              >
                <span>{tab.title}</span>
              </button>
              {!tab.alive && <span className="text-terminal-red">exit</span>}
              <button
                type="button"
                aria-label={`Fechar aba ${tab.title}`}
                title={`Fechar aba ${tab.title}`}
                className="terminal-tab-close ml-0.5 rounded p-0.5 text-muted-foreground opacity-100 transition hover:bg-[var(--ide-hover)] hover:text-terminal-red focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-terminal-blue/50"
                onClick={(ev) => {
                  ev.stopPropagation()
                  onCloseTab(tab.id)
                }}
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          ))}
        </div>
        <button
          type="button"
          aria-label="Criar nova aba de terminal"
          onClick={onCreateTab}
          className="ml-1 inline-flex shrink-0 items-center gap-1 rounded border border-[var(--ide-border-strong)] bg-[var(--ide-surface-1)] px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-[var(--ide-hover)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-terminal-blue/50"
        >
          <Plus className="w-3.5 h-3.5" />
          Novo
        </button>
      </div>
      <div className="border-b border-[var(--ide-border-subtle)] bg-[var(--terminal-bg)] px-2.5 py-0.5 text-[10px] text-muted-foreground/60">
        Clique no terminal para executar comandos locais.
      </div>
      <div className="min-h-0 flex-1 bg-[var(--terminal-bg)] p-1">
        <div
          ref={terminalContainerRef}
          className="h-full w-full rounded border border-[var(--ide-border-subtle)] bg-terminal-bg"
        />
      </div>
    </section>
  )
}
