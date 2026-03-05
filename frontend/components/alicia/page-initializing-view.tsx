"use client"

import { Bot } from "lucide-react"

interface PageInitializingViewProps {
  initializingStatus: string
  bootLogs: string[]
}

export function PageInitializingView({
  initializingStatus,
  bootLogs,
}: PageInitializingViewProps) {
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
