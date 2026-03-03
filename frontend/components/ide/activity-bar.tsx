"use client"

import {
  Blocks,
  Bot,
  Bug,
  ChevronLeft,
  Database,
  Files,
  GitBranch,
  Search,
  Settings,
  User,
} from "lucide-react"
import { cn } from "@/lib/utils"

export type IdeSidebarView = "agent" | "explorer" | "sap"

interface IdeActivityBarProps {
  activeView: IdeSidebarView
  sidebarVisible: boolean
  onToggleSidebar: () => void
  onSelectView: (view: IdeSidebarView) => void
}

type ActivityItem = {
  id: IdeSidebarView | "search" | "git" | "debug" | "extensions"
  label: string
  icon: typeof Bot
  targetView?: IdeSidebarView
}

const topItems: ActivityItem[] = [
  { id: "explorer", label: "Explorador", icon: Files, targetView: "explorer" },
  { id: "search", label: "Pesquisar", icon: Search },
  { id: "git", label: "Controle de fonte", icon: GitBranch },
  { id: "debug", label: "Executar e depurar", icon: Bug },
  { id: "extensions", label: "Extensoes", icon: Blocks },
  { id: "sap", label: "SAP Explorer", icon: Database, targetView: "sap" },
  { id: "agent", label: "Alicia AI", icon: Bot, targetView: "agent" },
]

const bottomItems = [
  { id: "profile", label: "Perfil", icon: User },
  { id: "settings", label: "Configuracoes", icon: Settings },
]

export function IdeActivityBar({
  activeView,
  sidebarVisible,
  onToggleSidebar,
  onSelectView,
}: IdeActivityBarProps) {
  return (
    <aside
      className="flex h-full w-[var(--ide-activity-width)] shrink-0 flex-col items-center border-r border-[var(--ide-border-strong)] bg-[var(--ide-app-bg)] py-1"
      aria-label="Barra de atividade"
    >
      <div className="flex flex-1 flex-col items-center gap-0.5">
        {topItems.map(({ id, icon: Icon, label, targetView }) => {
          const isInteractive = Boolean(targetView)
          const selected =
            isInteractive && targetView ? sidebarVisible && activeView === targetView : false

          return (
            <button
              key={id}
              type="button"
              aria-label={label}
              aria-pressed={selected}
              aria-disabled={!isInteractive}
              title={isInteractive ? label : `${label} (em breve)`}
              disabled={!isInteractive}
              onClick={() => {
                if (!targetView) {
                  return
                }
                if (targetView === activeView) {
                  onToggleSidebar()
                  return
                }
                onSelectView(targetView)
              }}
              className={cn(
                "group relative flex h-10 w-10 items-center justify-center rounded-[5px] border border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-terminal-blue/50",
                isInteractive
                  ? "text-muted-foreground hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)] hover:text-foreground"
                  : "cursor-default text-muted-foreground/35",
                selected &&
                  "border-[var(--ide-border-strong)] bg-[var(--ide-active)] text-foreground",
              )}
            >
              {selected ? (
                <span className="absolute -left-[5px] top-1/2 h-5 w-[2px] -translate-y-1/2 rounded-r bg-terminal-blue" />
              ) : null}
              <Icon className="h-[18px] w-[18px]" />
              <span className="pointer-events-none absolute left-[calc(100%+8px)] hidden whitespace-nowrap rounded border border-[var(--ide-border-strong)] bg-[var(--ide-surface-2)] px-1.5 py-1 text-[10px] text-foreground shadow-lg group-hover:block group-focus-visible:block">
                {isInteractive ? label : `${label} (em breve)`}
              </span>
            </button>
          )
        })}
      </div>

      <div className="flex flex-col items-center gap-0.5 pb-1">
        <button
          type="button"
          aria-label={sidebarVisible ? "Ocultar painel lateral" : "Mostrar painel lateral"}
          aria-expanded={sidebarVisible}
          title={sidebarVisible ? "Ocultar painel lateral" : "Mostrar painel lateral"}
          onClick={onToggleSidebar}
          className="group relative flex h-10 w-10 items-center justify-center rounded-[5px] border border-transparent text-muted-foreground transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)] hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-terminal-blue/50"
        >
          <ChevronLeft
            className="h-4 w-4 transition-transform"
            style={{ transform: sidebarVisible ? "rotate(0deg)" : "rotate(180deg)" }}
          />
          <span className="pointer-events-none absolute left-[calc(100%+8px)] hidden whitespace-nowrap rounded border border-[var(--ide-border-strong)] bg-[var(--ide-surface-2)] px-1.5 py-1 text-[10px] text-foreground shadow-lg group-hover:block group-focus-visible:block">
            {sidebarVisible ? "Recolher painel" : "Expandir painel"}
          </span>
        </button>

        {bottomItems.map(({ id, icon: Icon, label }) => (
          <button
            key={id}
            type="button"
            disabled
            aria-label={label}
            aria-disabled
            title={`${label} (em breve)`}
            className="group relative flex h-10 w-10 cursor-default items-center justify-center rounded-[5px] border border-transparent text-muted-foreground/35"
          >
            <Icon className="h-[18px] w-[18px]" />
            <span className="pointer-events-none absolute left-[calc(100%+8px)] hidden whitespace-nowrap rounded border border-[var(--ide-border-strong)] bg-[var(--ide-surface-2)] px-1.5 py-1 text-[10px] text-foreground shadow-lg group-hover:block group-focus-visible:block">
              {label} (em breve)
            </span>
          </button>
        ))}
      </div>
    </aside>
  )
}
