"use client"

import {
  ChevronDown,
  ChevronRight,
  FileCode2,
  FileJson,
  FilePlus2,
  FilePenLine,
  FileText,
  Folder,
  FolderPlus,
  GitBranch,
  PencilLine,
  RefreshCw,
} from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { type CodexWorkspaceDirectoryEntry } from "@/lib/tauri-bridge"
import { cn } from "@/lib/utils"

interface FileChangeItem {
  name: string
  status: "modified" | "added" | "deleted" | "renamed" | "copied" | "untracked" | "unmerged"
  fromPath?: string
}

interface IdeFileExplorerProps {
  workspaceLabel?: string
  fileChanges?: FileChangeItem[]
  treeEntries: CodexWorkspaceDirectoryEntry[]
  treeChildrenByPath: Record<string, CodexWorkspaceDirectoryEntry[]>
  loadedDirectoryPaths: Record<string, boolean>
  loadingDirectoryPaths: Record<string, boolean>
  treeVersion?: number
  loadingRoot?: boolean
  branchLabel?: string
  onLoadDirectory?: (path: string) => void | Promise<void>
  onRefresh?: () => void
  onOpenFile: (ref: string) => void
  onCreateFile?: (relativePath: string) => boolean | Promise<boolean> | void | Promise<void>
  onCreateFolder?: (relativePath: string) => boolean | Promise<boolean> | void | Promise<void>
  onRenameEntry?: (
    path: string,
    newName: string,
  ) => string | boolean | void | Promise<string | boolean | void>
}

const statusMap: Record<FileChangeItem["status"], { label: string; className: string }> = {
  modified: { label: "M", className: "text-terminal-blue" },
  added: { label: "A", className: "text-terminal-green" },
  deleted: { label: "D", className: "text-terminal-red" },
  renamed: { label: "R", className: "text-terminal-cyan" },
  copied: { label: "C", className: "text-terminal-cyan" },
  untracked: { label: "U", className: "text-terminal-gold" },
  unmerged: { label: "!", className: "text-terminal-red" },
}

interface CreateDraft {
  kind: "file" | "folder"
  parentPath: string
  value: string
  error: string | null
}

interface RenameDraft {
  path: string
  parentPath: string
  kind: "file" | "directory"
  value: string
  error: string | null
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/^\.\/+/, "").replace(/^\/+/, "").replace(/\/+$/, "")
}

function dirname(path: string): string {
  const normalized = normalizePath(path)
  const separator = normalized.lastIndexOf("/")
  if (separator === -1) {
    return ""
  }
  return normalized.slice(0, separator)
}

function joinPath(parent: string, name: string): string {
  const cleanParent = normalizePath(parent)
  const cleanName = normalizePath(name)
  if (!cleanParent) {
    return cleanName
  }
  return `${cleanParent}/${cleanName}`
}

function inlineRowPadding(depth: number): string {
  return `${12 + depth * 10}px`
}

function isEditableElement(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false
  }
  const tagName = target.tagName.toLowerCase()
  if (tagName === "input" || tagName === "textarea") {
    return true
  }
  if (target.isContentEditable) {
    return true
  }
  return Boolean(target.closest("[contenteditable='true']"))
}

function iconForFile(path: string) {
  if (path.endsWith(".tsx") || path.endsWith(".ts") || path.endsWith(".jsx") || path.endsWith(".js")) {
    return <FileCode2 className="h-3 w-3 shrink-0 text-terminal-blue" />
  }
  if (path.endsWith(".json")) {
    return <FileJson className="h-3 w-3 shrink-0 text-terminal-gold" />
  }
  if (path.endsWith(".md")) {
    return <FilePenLine className="h-3 w-3 shrink-0 text-terminal-cyan" />
  }
  return <FileText className="h-3 w-3 shrink-0 text-muted-foreground/80" />
}

export function IdeFileExplorer({
  workspaceLabel = "workspace",
  fileChanges,
  treeEntries,
  treeChildrenByPath,
  loadedDirectoryPaths,
  loadingDirectoryPaths,
  treeVersion,
  loadingRoot = false,
  branchLabel = "main",
  onLoadDirectory,
  onRefresh,
  onOpenFile,
  onCreateFile,
  onCreateFolder,
  onRenameEntry,
}: IdeFileExplorerProps) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null)
  const [selectedKind, setSelectedKind] = useState<"file" | "directory" | null>(null)
  const [expandedDirectories, setExpandedDirectories] = useState<Set<string>>(
    () => new Set(),
  )
  const [createDraft, setCreateDraft] = useState<CreateDraft | null>(null)
  const [renameDraft, setRenameDraft] = useState<RenameDraft | null>(null)
  const draftInputRef = useRef<HTMLInputElement | null>(null)
  const renameInputRef = useRef<HTMLInputElement | null>(null)
  const hasHeaderActions = Boolean(onRefresh || onCreateFile || onCreateFolder)

  useEffect(() => {
    if (!createDraft) {
      return
    }
    draftInputRef.current?.focus()
    draftInputRef.current?.select()
  }, [createDraft])

  useEffect(() => {
    if (!renameDraft) {
      return
    }
    renameInputRef.current?.focus()
    renameInputRef.current?.select()
  }, [renameDraft])

  useEffect(() => {
    setExpandedDirectories(new Set())
    setCreateDraft(null)
    setRenameDraft(null)
  }, [treeVersion])

  const fileChangeByPath = useMemo(() => {
    const map = new Map<string, FileChangeItem>()
    for (const item of fileChanges ?? []) {
      map.set(normalizePath(item.name), item)
    }
    return map
  }, [fileChanges])

  const treeEntryByPath = useMemo(() => {
    const byPath = new Map<string, CodexWorkspaceDirectoryEntry>()
    const stack: CodexWorkspaceDirectoryEntry[] = [...treeEntries]

    while (stack.length > 0) {
      const current = stack.pop()
      if (!current) {
        continue
      }
      const normalizedPath = normalizePath(current.path)
      if (byPath.has(normalizedPath)) {
        continue
      }
      byPath.set(normalizedPath, current)
      const children = treeChildrenByPath[normalizedPath] ?? []
      if (children.length > 0) {
        stack.push(...children)
      }
    }

    return byPath
  }, [treeChildrenByPath, treeEntries])

  const resolveCreateParentPath = (): string => {
    if (!selectedPath || !selectedKind) {
      return ""
    }
    return selectedKind === "directory" ? selectedPath : dirname(selectedPath)
  }

  const startCreateInline = (kind: "file" | "folder") => {
    if (kind === "file" && !onCreateFile) {
      return
    }
    if (kind === "folder" && !onCreateFolder) {
      return
    }

    const parentPath = resolveCreateParentPath()
    if (parentPath) {
      setExpandedDirectories((previous) => {
        const next = new Set(previous)
        next.add(parentPath)
        return next
      })
      if (!loadedDirectoryPaths[parentPath] && !loadingDirectoryPaths[parentPath]) {
        void onLoadDirectory?.(parentPath)
      }
    }

    setRenameDraft(null)
    setCreateDraft({
      kind,
      parentPath,
      value: "",
      error: null,
    })
  }

  const submitCreateInline = async () => {
    if (!createDraft) {
      return
    }
    const trimmed = createDraft.value.trim()
    if (!trimmed) {
      setCreateDraft((previous) =>
        previous
          ? {
              ...previous,
              error: "Informe um nome.",
            }
          : previous,
      )
      return
    }
    if (trimmed.includes("/") || trimmed.includes("\\")) {
      setCreateDraft((previous) =>
        previous
          ? {
              ...previous,
              error: "Use apenas o nome. Selecione a pasta de destino no explorer.",
            }
          : previous,
      )
      return
    }

    const relativePath = joinPath(createDraft.parentPath, trimmed)
    try {
      const callback = createDraft.kind === "file" ? onCreateFile : onCreateFolder
      if (!callback) {
        setCreateDraft(null)
        return
      }
      const result = await callback(relativePath)
      if (result === false) {
        return
      }
      setSelectedPath(relativePath)
      setSelectedKind(createDraft.kind === "file" ? "file" : "directory")
      if (createDraft.kind === "folder") {
        setExpandedDirectories((previous) => {
          const next = new Set(previous)
          next.add(relativePath)
          return next
        })
      }
      setCreateDraft(null)
    } catch (error) {
      setCreateDraft((previous) =>
        previous
          ? {
              ...previous,
              error: String(error),
            }
          : previous,
      )
    }
  }

  const startRenameInline = (
    path: string,
    name: string,
    kind: "file" | "directory",
  ) => {
    if (!onRenameEntry) {
      return
    }
    setCreateDraft(null)
    setSelectedPath(path)
    setSelectedKind(kind)
    setRenameDraft({
      path,
      parentPath: dirname(path),
      kind,
      value: name,
      error: null,
    })
  }

  const submitRenameInline = async () => {
    if (!renameDraft || !onRenameEntry) {
      return
    }
    const trimmed = renameDraft.value.trim()
    if (!trimmed) {
      setRenameDraft((previous) =>
        previous
          ? {
              ...previous,
              error: "Informe um nome.",
            }
          : previous,
      )
      return
    }
    if (trimmed === "." || trimmed === "..") {
      setRenameDraft((previous) =>
        previous
          ? {
              ...previous,
              error: "Nome invalido.",
            }
          : previous,
      )
      return
    }
    if (trimmed.includes("/") || trimmed.includes("\\")) {
      setRenameDraft((previous) =>
        previous
          ? {
              ...previous,
              error: "Renomeacao so permite alterar o nome no mesmo diretorio.",
            }
          : previous,
      )
      return
    }

    try {
      const result = await onRenameEntry(renameDraft.path, trimmed)
      if (result === false) {
        return
      }
      const nextPath =
        typeof result === "string" && result.trim()
          ? normalizePath(result)
          : joinPath(renameDraft.parentPath, trimmed)
      setSelectedPath(nextPath)
      setSelectedKind(renameDraft.kind)
      setRenameDraft(null)
    } catch (error) {
      setRenameDraft((previous) =>
        previous
          ? {
              ...previous,
              error: String(error),
            }
          : previous,
      )
    }
  }

  const renderInlineInputRow = (parentPath: string, depth: number) => {
    if (!createDraft || createDraft.parentPath !== parentPath) {
      return null
    }
    const icon =
      createDraft.kind === "file" ? (
        <FilePlus2 className="h-3 w-3 shrink-0 text-terminal-blue" />
      ) : (
        <FolderPlus className="h-3 w-3 shrink-0 text-terminal-gold" />
      )

    return (
      <div
        className="space-y-1 py-0.5"
        style={{ paddingLeft: inlineRowPadding(depth) }}
      >
        <div className="flex items-center gap-1.5 border-l border-terminal-blue bg-[var(--ide-active)] px-3 py-0.5 text-[11px] text-foreground">
          <span className="w-3 shrink-0" aria-hidden="true" />
          {icon}
          <input
            ref={draftInputRef}
            value={createDraft.value}
            onChange={(event) => {
              const value = event.target.value
              setCreateDraft((previous) =>
                previous
                  ? {
                      ...previous,
                      value,
                      error: null,
                    }
                  : previous,
              )
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault()
                void submitCreateInline()
              }
              if (event.key === "Escape") {
                event.preventDefault()
                setCreateDraft(null)
              }
            }}
            onBlur={() => {
              setCreateDraft(null)
            }}
            className="w-full bg-transparent outline-none placeholder:text-muted-foreground/60"
            placeholder={createDraft.kind === "file" ? "nome-do-arquivo" : "nome-da-pasta"}
            aria-label={createDraft.kind === "file" ? "Nome do novo arquivo" : "Nome da nova pasta"}
          />
        </div>
        {createDraft.error ? (
          <p className="px-1 text-[10px] text-terminal-red">{createDraft.error}</p>
        ) : null}
      </div>
    )
  }

  const renderTree = (
    entries: CodexWorkspaceDirectoryEntry[],
    parentPath: string,
    depth: number,
  ) => {
    if (entries.length === 0) {
      return renderInlineInputRow(parentPath, depth)
    }

    return (
      <>
        {entries.map((entry) => {
          const entryPath = normalizePath(entry.path)
          const isDirectory = entry.kind === "directory"
          const isExpanded = expandedDirectories.has(entryPath)
          const isSelected = selectedPath === entryPath
          const children = treeChildrenByPath[entryPath] ?? []
          const isLoadingChildren = Boolean(loadingDirectoryPaths[entryPath])
          const isLoadedChildren = Boolean(loadedDirectoryPaths[entryPath])
          const change = fileChangeByPath.get(entryPath)
          const indicator = change ? statusMap[change.status] : null
          const isRenaming = renameDraft?.path === entryPath

          return (
            <div key={entryPath}>
              {isRenaming ? (
                <div className="space-y-1 py-0.5">
                  <div
                    className="flex items-center gap-1.5 border-l border-terminal-blue bg-[var(--ide-active)] px-3 py-0.5 text-[11px] text-foreground"
                    style={{ paddingLeft: inlineRowPadding(depth) }}
                  >
                    {isDirectory ? (
                      isExpanded ? (
                        <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground/70" />
                      ) : (
                        <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground/70" />
                      )
                    ) : (
                      <span className="w-3 shrink-0" aria-hidden="true" />
                    )}
                    {isDirectory ? (
                      <Folder className="h-3 w-3 shrink-0 text-terminal-gold" />
                    ) : (
                      iconForFile(entryPath)
                    )}
                    <input
                      ref={renameInputRef}
                      value={renameDraft.value}
                      onChange={(event) => {
                        const value = event.target.value
                        setRenameDraft((previous) =>
                          previous
                            ? {
                                ...previous,
                                value,
                                error: null,
                              }
                            : previous,
                        )
                      }}
                      onKeyDown={(event) => {
                        if (event.key === "Enter") {
                          event.preventDefault()
                          void submitRenameInline()
                        }
                        if (event.key === "Escape") {
                          event.preventDefault()
                          setRenameDraft(null)
                        }
                      }}
                      onBlur={() => {
                        setRenameDraft(null)
                      }}
                      className="w-full bg-transparent outline-none"
                      placeholder="novo-nome"
                      aria-label={`Renomear ${entry.name}`}
                    />
                  </div>
                  {renameDraft.error ? (
                    <p className="px-1 text-[10px] text-terminal-red" style={{ paddingLeft: inlineRowPadding(depth) }}>
                      {renameDraft.error}
                    </p>
                  ) : null}
                </div>
              ) : (
                <div
                  className={cn(
                    "group flex w-full items-center gap-1.5 border-l border-transparent px-3 py-0.5 text-[11px] text-muted-foreground transition-colors hover:bg-[var(--ide-hover)] hover:text-foreground",
                    isSelected && "border-terminal-blue bg-[var(--ide-active)] text-foreground",
                  )}
                  style={{ paddingLeft: inlineRowPadding(depth) }}
                >
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
                    onClick={() => {
                      setSelectedPath(entryPath)
                      setSelectedKind(isDirectory ? "directory" : "file")
                      if (isDirectory) {
                        setExpandedDirectories((previous) => {
                          const next = new Set(previous)
                          if (next.has(entryPath)) {
                            next.delete(entryPath)
                          } else {
                            next.add(entryPath)
                          }
                          return next
                        })
                        if (
                          !isExpanded &&
                          entry.hasChildren !== false &&
                          !isLoadedChildren &&
                          !isLoadingChildren
                        ) {
                          void onLoadDirectory?.(entryPath)
                        }
                        return
                      }
                      onOpenFile(entryPath)
                    }}
                    title={entryPath}
                  >
                    {isDirectory ? (
                      isExpanded ? (
                        <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground/70" />
                      ) : (
                        <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground/70" />
                      )
                    ) : (
                      <span className="w-3 shrink-0" aria-hidden="true" />
                    )}
                    {isDirectory ? (
                      <Folder className="h-3 w-3 shrink-0 text-terminal-gold" />
                    ) : (
                      iconForFile(entryPath)
                    )}
                    <span className="flex-1 truncate">{entry.name}</span>
                    {indicator ? (
                      <span
                        className={cn("text-[10px] font-bold", indicator.className)}
                        title={`Status git: ${change?.status}`}
                      >
                        {indicator.label}
                      </span>
                    ) : null}
                  </button>
                  {onRenameEntry ? (
                    <button
                      type="button"
                      aria-label={`Renomear ${entry.name}`}
                      title={`Renomear ${entry.name}`}
                      className="shrink-0 rounded border border-transparent p-0.5 text-muted-foreground opacity-0 transition-all hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)] hover:text-foreground focus-visible:opacity-100 group-hover:opacity-100"
                      onClick={(event) => {
                        event.preventDefault()
                        event.stopPropagation()
                        startRenameInline(entryPath, entry.name, isDirectory ? "directory" : "file")
                      }}
                    >
                      <PencilLine className="h-3 w-3" />
                    </button>
                  ) : null}
                </div>
              )}

              {isDirectory && isExpanded ? (
                <div>
                  {isLoadingChildren ? (
                    <p className="px-3 py-1 text-[10px] text-muted-foreground/70" style={{ paddingLeft: `${22 + depth * 10}px` }}>
                      Carregando...
                    </p>
                  ) : null}
                  {!isLoadingChildren && isLoadedChildren && children.length === 0 ? (
                    <p className="px-3 py-1 text-[10px] text-muted-foreground/70" style={{ paddingLeft: `${22 + depth * 10}px` }}>
                      Pasta vazia.
                    </p>
                  ) : null}
                  {!isLoadingChildren ? renderTree(children, entryPath, depth + 1) : null}
                </div>
              ) : null}
            </div>
          )
        })}
        {renderInlineInputRow(parentPath, depth)}
      </>
    )
  }

  return (
    <section
      className="flex h-full min-h-0 flex-col bg-[var(--ide-surface-1)]"
      onKeyDown={(event) => {
        if (event.key !== "F2") {
          return
        }
        if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) {
          return
        }
        if (createDraft || renameDraft) {
          return
        }
        if (!selectedPath) {
          return
        }
        if (isEditableElement(event.target)) {
          return
        }

        const selectedEntry = treeEntryByPath.get(selectedPath)
        if (!selectedEntry) {
          return
        }

        event.preventDefault()
        startRenameInline(
          normalizePath(selectedEntry.path),
          selectedEntry.name,
          selectedEntry.kind === "directory" ? "directory" : "file",
        )
      }}
    >
      <header className="flex items-center justify-between border-b border-[var(--ide-border-subtle)] px-3 py-1 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
        <span className="text-terminal-fg/75">Explorer</span>
        {hasHeaderActions ? (
          <div className="flex items-center gap-1">
            {onCreateFile ? (
              <button
                type="button"
                aria-label="Criar novo arquivo"
                title="Novo arquivo"
                onClick={() => {
                  startCreateInline("file")
                }}
                className="rounded-[5px] border border-transparent p-1 text-muted-foreground transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)] hover:text-foreground"
              >
                <FilePlus2 className="h-3 w-3" />
              </button>
            ) : null}
            {onCreateFolder ? (
              <button
                type="button"
                aria-label="Criar nova pasta"
                title="Nova pasta"
                onClick={() => {
                  startCreateInline("folder")
                }}
                className="rounded-[5px] border border-transparent p-1 text-muted-foreground transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)] hover:text-foreground"
              >
                <FolderPlus className="h-3 w-3" />
              </button>
            ) : null}
            {onRefresh ? (
              <button
                type="button"
                aria-label="Atualizar alterações"
                title="Atualizar alterações"
                onClick={onRefresh}
                className="rounded-[5px] border border-transparent p-1 text-muted-foreground transition-colors hover:border-[var(--ide-border-subtle)] hover:bg-[var(--ide-hover)] hover:text-foreground"
              >
                <RefreshCw className="h-3 w-3" />
              </button>
            ) : null}
          </div>
        ) : null}
      </header>

      <div className="flex items-center gap-1 border-b border-[var(--ide-border-subtle)] bg-[var(--ide-surface-0)]/55 px-3 py-1 text-[11px] text-muted-foreground">
        <ChevronDown className="h-3 w-3" />
        <span className="truncate font-semibold uppercase tracking-wide text-[10px]">{workspaceLabel}</span>
        <span className="ml-auto flex items-center gap-1 text-[10px] text-terminal-fg/75">
          <GitBranch className="h-3 w-3" />
          {branchLabel}
        </span>
        <span className="rounded bg-[var(--ide-surface-3)] px-1.5 py-0.5 text-[10px] text-terminal-fg/80">{(fileChanges ?? []).length}</span>
      </div>

      <div className="flex-1 overflow-y-auto py-1">
        {loadingRoot ? (
          <p className="px-3 py-2 text-xs text-muted-foreground/70">Carregando estrutura do workspace...</p>
        ) : treeEntries.length === 0 && !createDraft ? (
          <p className="px-3 py-2 text-xs text-muted-foreground/70">Pasta vazia.</p>
        ) : (
          renderTree(treeEntries, "", 0)
        )}
      </div>
    </section>
  )
}
