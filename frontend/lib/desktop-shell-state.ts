export type DesktopShellMode = "normal" | "focus" | "zen"

export interface DesktopLayoutState {
  sidebarVisible: boolean
  terminalVisible: boolean
}

export type DesktopLayoutSnapshot = DesktopLayoutState

export type DesktopEscAction =
  | "close-panel"
  | "exit-zen"
  | "exit-focus"
  | "none"

export interface ResolveDesktopEscActionInput {
  isMobile: boolean
  hasActivePanel: boolean
  shellMode: DesktopShellMode
}

export interface DesktopShellTransitionInput {
  isMobile: boolean
  currentMode: DesktopShellMode
  nextMode: DesktopShellMode
  layout: DesktopLayoutState
  snapshot: DesktopLayoutSnapshot
}

export interface DesktopShellTransitionResult {
  changed: boolean
  mode: DesktopShellMode
  layout: DesktopLayoutState
  snapshot: DesktopLayoutSnapshot
}

export function resolveDesktopEscAction(
  input: ResolveDesktopEscActionInput,
): DesktopEscAction {
  if (input.isMobile) {
    return "none"
  }
  if (input.hasActivePanel) {
    return "close-panel"
  }
  if (input.shellMode === "zen") {
    return "exit-zen"
  }
  if (input.shellMode === "focus") {
    return "exit-focus"
  }
  return "none"
}

export function transitionDesktopShellMode(
  input: DesktopShellTransitionInput,
): DesktopShellTransitionResult {
  const { isMobile, currentMode, nextMode, layout, snapshot } = input

  if (isMobile) {
    return {
      changed: false,
      mode: currentMode,
      layout,
      snapshot,
    }
  }

  if (nextMode === "normal") {
    if (currentMode === "normal") {
      return {
        changed: false,
        mode: currentMode,
        layout,
        snapshot,
      }
    }
    return {
      changed: true,
      mode: "normal",
      layout: snapshot,
      snapshot,
    }
  }

  const nextSnapshot =
    currentMode === "normal"
      ? {
          sidebarVisible: layout.sidebarVisible,
          terminalVisible: layout.terminalVisible,
        }
      : snapshot

  if (currentMode === nextMode) {
    return {
      changed: false,
      mode: currentMode,
      layout,
      snapshot,
    }
  }

  return {
    changed: true,
    mode: nextMode,
    layout: {
      sidebarVisible: false,
      terminalVisible: false,
    },
    snapshot: nextSnapshot,
  }
}
