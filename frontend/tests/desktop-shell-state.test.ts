import { describe, expect, it } from "vitest"

import {
  resolveDesktopEscAction,
  transitionDesktopShellMode,
} from "@/lib/desktop-shell-state"

describe("desktop shell state helpers", () => {
  it("prioriza Esc: close panel > exit zen > exit focus > none", () => {
    expect(
      resolveDesktopEscAction({
        isMobile: false,
        hasActivePanel: true,
        shellMode: "zen",
      }),
    ).toBe("close-panel")

    expect(
      resolveDesktopEscAction({
        isMobile: false,
        hasActivePanel: false,
        shellMode: "zen",
      }),
    ).toBe("exit-zen")

    expect(
      resolveDesktopEscAction({
        isMobile: false,
        hasActivePanel: false,
        shellMode: "focus",
      }),
    ).toBe("exit-focus")

    expect(
      resolveDesktopEscAction({
        isMobile: false,
        hasActivePanel: false,
        shellMode: "normal",
      }),
    ).toBe("none")
  })

  it("salva snapshot ao entrar em focus e zen vindo de normal", () => {
    const focusTransition = transitionDesktopShellMode({
      isMobile: false,
      currentMode: "normal",
      nextMode: "focus",
      layout: { sidebarVisible: true, terminalVisible: false },
      snapshot: { sidebarVisible: false, terminalVisible: true },
    })

    expect(focusTransition.snapshot).toEqual({
      sidebarVisible: true,
      terminalVisible: false,
    })
    expect(focusTransition.layout).toEqual({
      sidebarVisible: false,
      terminalVisible: false,
    })

    const zenTransition = transitionDesktopShellMode({
      isMobile: false,
      currentMode: "normal",
      nextMode: "zen",
      layout: { sidebarVisible: false, terminalVisible: true },
      snapshot: { sidebarVisible: true, terminalVisible: false },
    })

    expect(zenTransition.snapshot).toEqual({
      sidebarVisible: false,
      terminalVisible: true,
    })
    expect(zenTransition.layout).toEqual({
      sidebarVisible: false,
      terminalVisible: false,
    })
  })

  it("troca focus <-> zen mantendo snapshot", () => {
    const snapshot = { sidebarVisible: true, terminalVisible: true }

    const toZen = transitionDesktopShellMode({
      isMobile: false,
      currentMode: "focus",
      nextMode: "zen",
      layout: { sidebarVisible: false, terminalVisible: false },
      snapshot,
    })

    expect(toZen.snapshot).toEqual(snapshot)
    expect(toZen.layout).toEqual({
      sidebarVisible: false,
      terminalVisible: false,
    })

    const toFocus = transitionDesktopShellMode({
      isMobile: false,
      currentMode: "zen",
      nextMode: "focus",
      layout: { sidebarVisible: false, terminalVisible: false },
      snapshot,
    })

    expect(toFocus.snapshot).toEqual(snapshot)
    expect(toFocus.layout).toEqual({
      sidebarVisible: false,
      terminalVisible: false,
    })
  })

  it("mantem estado estavel em no-op quando currentMode === nextMode (focus e zen)", () => {
    const focusLayout = { sidebarVisible: true, terminalVisible: false }
    const focusSnapshot = { sidebarVisible: false, terminalVisible: true }
    const focusNoOp = transitionDesktopShellMode({
      isMobile: false,
      currentMode: "focus",
      nextMode: "focus",
      layout: focusLayout,
      snapshot: focusSnapshot,
    })

    expect(focusNoOp.changed).toBe(false)
    expect(focusNoOp.mode).toBe("focus")
    expect(focusNoOp.layout).toEqual(focusLayout)
    expect(focusNoOp.snapshot).toEqual(focusSnapshot)

    const zenLayout = { sidebarVisible: false, terminalVisible: true }
    const zenSnapshot = { sidebarVisible: true, terminalVisible: false }
    const zenNoOp = transitionDesktopShellMode({
      isMobile: false,
      currentMode: "zen",
      nextMode: "zen",
      layout: zenLayout,
      snapshot: zenSnapshot,
    })

    expect(zenNoOp.changed).toBe(false)
    expect(zenNoOp.mode).toBe("zen")
    expect(zenNoOp.layout).toEqual(zenLayout)
    expect(zenNoOp.snapshot).toEqual(zenSnapshot)
  })

  it("retorna para normal restaurando snapshot", () => {
    const result = transitionDesktopShellMode({
      isMobile: false,
      currentMode: "zen",
      nextMode: "normal",
      layout: { sidebarVisible: false, terminalVisible: false },
      snapshot: { sidebarVisible: true, terminalVisible: false },
    })

    expect(result.mode).toBe("normal")
    expect(result.layout).toEqual({
      sidebarVisible: true,
      terminalVisible: false,
    })
  })

  it("faz no-op em mobile", () => {
    expect(
      resolveDesktopEscAction({
        isMobile: true,
        hasActivePanel: true,
        shellMode: "zen",
      }),
    ).toBe("none")

    const result = transitionDesktopShellMode({
      isMobile: true,
      currentMode: "focus",
      nextMode: "normal",
      layout: { sidebarVisible: false, terminalVisible: false },
      snapshot: { sidebarVisible: true, terminalVisible: true },
    })

    expect(result.changed).toBe(false)
    expect(result.mode).toBe("focus")
    expect(result.layout).toEqual({
      sidebarVisible: false,
      terminalVisible: false,
    })
    expect(result.snapshot).toEqual({
      sidebarVisible: true,
      terminalVisible: true,
    })
  })
})
