"use client"

import dynamic from "next/dynamic"
import { Loader2, RefreshCw, Save, X } from "lucide-react"

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

  return (
    <div className="h-full min-h-0 flex flex-col border-r border-panel-border bg-panel-bg">
      <div className="shrink-0 border-b border-panel-border px-3 py-2">
        <div className="flex items-center gap-2">
          <div className="min-w-0 flex-1">
            <p className="truncate text-xs font-medium text-terminal-fg">
              {displayName ?? "Editor de codigo"}
            </p>
            <p className="truncate text-[11px] text-muted-foreground">
              {objectUri ?? "Selecione um arquivo em file changes"}
            </p>
          </div>
          <span className="text-[10px] text-muted-foreground">{statusText}</span>
        </div>
        <div className="mt-2 flex items-center gap-2">
          <button
            onClick={onSave}
            disabled={!objectUri || loading || saving || !dirty}
            className="inline-flex items-center gap-1 rounded px-2 py-1 text-xs bg-terminal-green/15 text-terminal-green hover:bg-terminal-green/25 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {saving ? <Loader2 className="h-3 w-3 animate-spin" /> : <Save className="h-3 w-3" />}
            Salvar
          </button>
          <button
            onClick={onReload}
            disabled={!objectUri || loading || saving}
            className="inline-flex items-center gap-1 rounded px-2 py-1 text-xs bg-background/60 text-muted-foreground hover:text-terminal-fg disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <RefreshCw className="h-3 w-3" />
            Recarregar
          </button>
          <button
            onClick={onClose}
            className="ml-auto inline-flex items-center gap-1 rounded px-2 py-1 text-xs bg-background/60 text-muted-foreground hover:text-terminal-fg"
          >
            <X className="h-3 w-3" />
            Fechar
          </button>
        </div>
        {etag ? <p className="mt-1 truncate text-[10px] text-muted-foreground">ETag: {etag}</p> : null}
        {error ? <p className="mt-1 text-xs text-terminal-red">{error}</p> : null}
      </div>
      <div className="min-h-0 flex-1">
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
    </div>
  )
}
