"use client"

import { useEffect, useMemo, useRef } from "react"
import { type ImperativePanelHandle } from "react-resizable-panels"
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable"
import { IdeActivityBar, type IdeSidebarView } from "@/components/ide/activity-bar"
import { IdeFileExplorer } from "@/components/ide/file-explorer"
import { type IdeShellMode, type IdeViewMode } from "@/components/ide/panel-toolbar"
import { type CodexWorkspaceDirectoryEntry } from "@/lib/tauri-bridge"
import { cn } from "@/lib/utils"

interface FileChangeItem {
  name: string
  status: "modified" | "added" | "deleted" | "renamed" | "copied" | "untracked" | "unmerged"
  fromPath?: string
}

interface AliciaDesktopShellProps {
  shellMode: IdeShellMode
  sidebarView: IdeSidebarView
  sidebarVisible: boolean
  viewMode: IdeViewMode
  terminalVisible: boolean
  fileChanges: FileChangeItem[]
  workspaceLabel: string
  treeEntries: CodexWorkspaceDirectoryEntry[]
  treeChildrenByPath: Record<string, CodexWorkspaceDirectoryEntry[]>
  loadedDirectoryPaths: Record<string, boolean>
  loadingDirectoryPaths: Record<string, boolean>
  treeVersion?: number
  loadingExplorerRoot?: boolean
  onSelectSidebarView: (view: IdeSidebarView) => void
  onToggleSidebar: () => void
  onTerminalVisibilityChange: (visible: boolean) => void
  onOpenFileInEditor: (ref: string) => void
  onLoadExplorerDirectory: (path: string) => void | Promise<void>
  onRefreshFileChanges: () => void
  onCreateFile: (relativePath: string) => boolean | Promise<boolean> | void | Promise<void>
  onCreateFolder: (relativePath: string) => boolean | Promise<boolean> | void | Promise<void>
  onRenameEntry: (
    path: string,
    newName: string,
  ) => string | boolean | void | Promise<string | boolean | void>
  sapSidebar: React.ReactNode
  agentSidebar: React.ReactNode
  conversationPane: React.ReactNode
  editorPane: React.ReactNode
  terminalPane: React.ReactNode
}

export function AliciaDesktopShell({
  shellMode,
  sidebarView,
  sidebarVisible,
  viewMode,
  terminalVisible,
  fileChanges,
  workspaceLabel,
  treeEntries,
  treeChildrenByPath,
  loadedDirectoryPaths,
  loadingDirectoryPaths,
  treeVersion,
  loadingExplorerRoot = false,
  onSelectSidebarView,
  onToggleSidebar,
  onTerminalVisibilityChange,
  onOpenFileInEditor,
  onLoadExplorerDirectory,
  onRefreshFileChanges,
  onCreateFile,
  onCreateFolder,
  onRenameEntry,
  sapSidebar,
  agentSidebar,
  conversationPane,
  editorPane,
  terminalPane,
}: AliciaDesktopShellProps) {
  const isFocusMode = shellMode === "focus"
  const isZenMode = shellMode === "zen"
  const showActivityBar = !isFocusMode && !isZenMode
  const showSidebar = sidebarVisible && !isFocusMode && !isZenMode
  const showTerminal = terminalVisible && !isFocusMode && !isZenMode
  const showChat = viewMode === "chat" || viewMode === "split"
  const showEditor = viewMode === "editor" || viewMode === "split"
  const terminalPanelRef = useRef<ImperativePanelHandle | null>(null)
  const terminalRegionRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const terminalPanel = terminalPanelRef.current
    if (!terminalPanel) {
      return
    }

    if (showTerminal) {
      terminalPanel.expand()
      return
    }

    const activeElement = document.activeElement
    if (
      activeElement instanceof HTMLElement &&
      terminalRegionRef.current?.contains(activeElement)
    ) {
      activeElement.blur()
    }

    terminalPanel.collapse()
  }, [showTerminal])

  const sidebarContent = useMemo(() => {
    if (sidebarView === "explorer") {
      return (
        <IdeFileExplorer
          workspaceLabel={workspaceLabel}
          fileChanges={fileChanges}
          treeEntries={treeEntries}
          treeChildrenByPath={treeChildrenByPath}
          loadedDirectoryPaths={loadedDirectoryPaths}
          loadingDirectoryPaths={loadingDirectoryPaths}
          treeVersion={treeVersion}
          loadingRoot={loadingExplorerRoot}
          onLoadDirectory={onLoadExplorerDirectory}
          onOpenFile={onOpenFileInEditor}
          onRefresh={onRefreshFileChanges}
          onCreateFile={onCreateFile}
          onCreateFolder={onCreateFolder}
          onRenameEntry={onRenameEntry}
        />
      )
    }
    if (sidebarView === "sap") {
      return sapSidebar
    }
    return agentSidebar
  }, [
    sapSidebar,
    agentSidebar,
    fileChanges,
    onCreateFile,
    onCreateFolder,
    onRenameEntry,
    onLoadExplorerDirectory,
    onOpenFileInEditor,
    onRefreshFileChanges,
    sidebarView,
    treeChildrenByPath,
    treeEntries,
    loadedDirectoryPaths,
    loadingDirectoryPaths,
    treeVersion,
    loadingExplorerRoot,
    workspaceLabel,
  ])

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden bg-[var(--ide-surface-0)]">
      {showActivityBar ? (
        <IdeActivityBar
          activeView={sidebarView}
          sidebarVisible={sidebarVisible}
          onToggleSidebar={onToggleSidebar}
          onSelectView={onSelectSidebarView}
        />
      ) : null}

      <ResizablePanelGroup direction="horizontal" className="min-h-0 flex-1">
        {showSidebar ? (
          <>
            <ResizablePanel defaultSize={15} minSize={10} maxSize={24}>
              <div className="h-full min-h-0 border-r border-[var(--ide-border-subtle)]">{sidebarContent}</div>
            </ResizablePanel>
            <ResizableHandle />
          </>
        ) : null}

        <ResizablePanel defaultSize={showSidebar ? 82 : 100} minSize={30}>
          <div className="flex h-full min-h-0 flex-col bg-[var(--ide-surface-0)]">
            <ResizablePanelGroup direction="vertical" className="min-h-0 flex-1">
              <ResizablePanel defaultSize={66} minSize={20}>
                {showChat && showEditor ? (
                  <ResizablePanelGroup direction="horizontal" className="h-full min-h-0">
                    <ResizablePanel defaultSize={58} minSize={24}>
                      <div className="h-full min-h-0">{conversationPane}</div>
                    </ResizablePanel>
                    <ResizableHandle />
                    <ResizablePanel defaultSize={42} minSize={24}>
                      <div className="h-full min-h-0">{editorPane}</div>
                    </ResizablePanel>
                  </ResizablePanelGroup>
                ) : showChat ? (
                  <div className="h-full min-h-0">{conversationPane}</div>
                ) : (
                  <div className="h-full min-h-0">{editorPane}</div>
                )}
              </ResizablePanel>

              <ResizableHandle
                className={cn(!showTerminal && "pointer-events-none h-0 opacity-0")}
              />
              <ResizablePanel
                ref={terminalPanelRef}
                defaultSize={34}
                minSize={0}
                maxSize={70}
                collapsible
                collapsedSize={0}
                onCollapse={() => {
                  onTerminalVisibilityChange(false)
                }}
                onExpand={() => {
                  onTerminalVisibilityChange(true)
                }}
              >
                <div
                  ref={terminalRegionRef}
                  className={cn(
                    "h-full min-h-0",
                    !showTerminal && "invisible pointer-events-none",
                  )}
                  aria-hidden={!showTerminal}
                >
                  {terminalPane}
                </div>
              </ResizablePanel>
            </ResizablePanelGroup>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  )
}
