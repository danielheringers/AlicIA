"use client"

import dynamic from "next/dynamic"
import {
  ChevronRight,
  Circle,
  FileCode2,
  Loader2,
  MoreHorizontal,
  RefreshCw,
  Replace,
  Save,
  Search,
  SplitSquareHorizontal,
  X,
} from "lucide-react"

const MonacoEditor = dynamic(() => import("@monaco-editor/react"), {
  ssr: false,
  loading: () => (
    <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
      Carregando editor...
    </div>
  ),
})

interface SourceEditorPanelProps {
  objectUri: string | null
  displayName: string | null
  language: string
  source: string
  etag: string | null
  dirty: boolean
  loading: boolean
  saving: boolean
  error: string | null
  onChangeSource: (value: string) => void
  onReload: () => void
  onSave: () => void
  onClose: () => void
}

export function SourceEditorPanel({
  objectUri,
  displayName,
  language,
  source,
  etag,
  dirty,
  loading,
  saving,
  error,
  onChangeSource,
  onReload,
  onSave,
  onClose,
}: SourceEditorPanelProps) {
  const statusText = loading
    ? "Carregando..."
    : saving
      ? "Salvando..."
      : error
        ? "Erro"
        : dirty
          ? "Alterado"
          : objectUri
            ? "Sincronizado"
            : "Sem arquivo"

  const pathSegments = (objectUri ?? "").replace(/\\/g, "/").split("/").filter(Boolean)
  const languageLabel = language === "abap" ? "ABAP" : language
  const lineCount = source.split("\n").length
  const disabledToolbarButtonClass =
    "rounded p-1 text-muted-foreground opacity-60 disabled:cursor-not-allowed"

  return (
    <section className="alicia-panel-chrome flex h-full min-h-0 flex-col overflow-hidden">
      <div className="alicia-panel-titlebar flex h-[30px] items-center justify-between border-b border-[var(--ide-border-subtle)] bg-[var(--ide-surface-1)]">
        <div className="flex min-w-0 flex-1 items-center overflow-x-auto">
          <div className="inline-flex h-[30px] min-w-0 max-w-full items-center gap-1.5 border-r border-[var(--ide-border-subtle)] bg-[var(--ide-tab-active)] px-2.5 text-[10px] text-terminal-fg">
            <FileCode2 className="h-3.5 w-3.5 shrink-0 text-terminal-blue" />
            <span className="truncate">{displayName ?? "Editor de codigo"}</span>
            {dirty ? <Circle className="h-2.5 w-2.5 shrink-0 fill-terminal-gold text-terminal-gold" /> : null}
          </div>
        </div>
        <div className="flex items-center gap-0.5 px-1.5">
          <button
            type="button"
            className={disabledToolbarButtonClass}
            aria-label="Buscar"
            aria-disabled="true"
            disabled
            title="Buscar"
          >
            <Search className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            className={disabledToolbarButtonClass}
            aria-label="Substituir"
            aria-disabled="true"
            disabled
            title="Substituir"
          >
            <Replace className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            className={disabledToolbarButtonClass}
            aria-label="Dividir editor"
            aria-disabled="true"
            disabled
            title="Dividir editor"
          >
            <SplitSquareHorizontal className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            className={disabledToolbarButtonClass}
            aria-label="Mais acoes"
            aria-disabled="true"
            disabled
            title="Mais acoes"
          >
            <MoreHorizontal className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      <div className="shrink-0 border-b border-[var(--ide-border-subtle)] bg-[var(--terminal-bg)] px-2.5 py-0.5">
        <div className="flex min-w-0 items-center gap-1 overflow-x-auto text-[10px] text-muted-foreground">
          {pathSegments.length > 0 ? (
            pathSegments.map((segment, index) => (
              <div key={`${segment}-${index}`} className="inline-flex shrink-0 items-center gap-1">
                {index > 0 ? <ChevronRight className="h-3 w-3" /> : null}
                <span className={index === pathSegments.length - 1 ? "text-terminal-fg/90" : ""}>
                  {segment}
                </span>
              </div>
            ))
          ) : (
            <span>Selecione um arquivo em file changes</span>
          )}
        </div>
      </div>

      <div className="shrink-0 border-b border-[var(--ide-border-subtle)] bg-[var(--ide-surface-1)] px-2.5 py-1">
        <div className="flex items-center gap-1.5">
          <button
            onClick={onSave}
            disabled={!objectUri || loading || saving || !dirty}
            className="inline-flex items-center gap-1 rounded border border-terminal-green/25 bg-terminal-green/15 px-2 py-0.5 text-[10px] text-terminal-green hover:bg-terminal-green/25 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {saving ? <Loader2 className="h-3 w-3 animate-spin" /> : <Save className="h-3 w-3" />}
            Salvar
          </button>
          <button
            onClick={onReload}
            disabled={!objectUri || loading || saving}
            className="inline-flex items-center gap-1 rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-0)] px-2 py-0.5 text-[10px] text-muted-foreground hover:bg-[var(--ide-hover)] hover:text-terminal-fg disabled:cursor-not-allowed disabled:opacity-50"
          >
            <RefreshCw className="h-3 w-3" />
            Recarregar
          </button>
          <span className="rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-0)] px-1.5 py-0 text-[10px] text-muted-foreground">
            {statusText}
          </span>
          <button
            onClick={onClose}
            className="ml-auto inline-flex items-center gap-1 rounded border border-[var(--ide-border-subtle)] bg-[var(--ide-surface-0)] px-2 py-0.5 text-[10px] text-muted-foreground hover:bg-[var(--ide-hover)] hover:text-terminal-fg"
          >
            <X className="h-3 w-3" />
            Fechar
          </button>
        </div>
        {etag ? <p className="mt-0.5 truncate text-[10px] text-muted-foreground">ETag: {etag}</p> : null}
        {error ? <p className="mt-1 text-xs text-terminal-red">{error}</p> : null}
      </div>

      <div className="min-h-0 flex-1 bg-[var(--ide-surface-0)]">
        {objectUri ? (
          <MonacoEditor
            height="100%"
            language={language}
            theme="vs-dark"
            value={source}
            onChange={(value) => onChangeSource(value ?? "")}
            options={{
              automaticLayout: true,
              minimap: { enabled: false },
              fontSize: 13,
              scrollBeyondLastLine: false,
              tabSize: 2,
              readOnly: loading || saving,
            }}
          />
        ) : (
          <div className="flex h-full items-center justify-center px-4 text-center text-xs text-muted-foreground">
            Abra um arquivo da lista de changes para carregar o codigo.
          </div>
        )}
      </div>

      <div className="flex h-6 shrink-0 items-center justify-between border-t border-[var(--ide-border-subtle)] bg-[var(--terminal-bg)] px-3 text-[10px] text-muted-foreground">
        <span>{languageLabel}</span>
        <span>{lineCount} linhas</span>
        <span>{dirty ? "Unsaved changes" : "Read/Write"}</span>
      </div>
    </section>
  )
}
