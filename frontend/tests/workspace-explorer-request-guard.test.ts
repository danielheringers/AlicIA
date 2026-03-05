import { describe, expect, it } from "vitest"

import { createWorkspaceExplorerRequestGuard } from "@/hooks/workspace-explorer-request-guard"

describe("workspace explorer request guard", () => {
  it("descarta resposta antiga quando uma nova requisicao da mesma chave inicia", () => {
    const guard = createWorkspaceExplorerRequestGuard()

    const first = guard.begin("src")
    const second = guard.begin("src")

    expect(guard.isCurrent(first)).toBe(false)
    expect(guard.isCurrent(second)).toBe(true)
  })

  it("mantem isolamento por chave de diretorio", () => {
    const guard = createWorkspaceExplorerRequestGuard()

    const rootRequest = guard.begin("__root__")
    const childRequest = guard.begin("src")

    expect(guard.isCurrent(rootRequest)).toBe(true)
    expect(guard.isCurrent(childRequest)).toBe(true)
  })

  it("invalida requisicoes pendentes ao trocar workspace", () => {
    const guard = createWorkspaceExplorerRequestGuard()

    const pending = guard.begin("__root__")
    guard.invalidateAll()

    expect(guard.isCurrent(pending)).toBe(false)

    const nextWorkspaceRequest = guard.begin("__root__")
    expect(guard.isCurrent(nextWorkspaceRequest)).toBe(true)
  })
})
