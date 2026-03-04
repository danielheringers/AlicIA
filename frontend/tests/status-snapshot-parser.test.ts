import { describe, expect, it } from "vitest"

import { parseStatusSnapshot } from "@/components/alicia/status-snapshot-parser"

const STATUS_GOLDEN = [
  "/status",
  "mode: running",
  "session: #42 (pid 9001)",
  "thread: thread_abc123",
  "workspace: C:/Users/danie/OneDrive/Documentos/Projetos/Neuromancer",
  "model: gpt-5.2",
  "reasoning: high",
  "approval: on-request",
  "sandbox: workspace-write",
  "web search: enabled",
  "limit id: user-123",
  "remaining 5h: 88.5% remaining (11.5% used), resets in 4h 12m",
  "remaining week: 76% remaining (24% used), resets in 6d 3h",
].join("\n")

const STATUS_GOLDEN_UNAVAILABLE = [
  "/status",
  "mode: running",
  "session: #X (pid n/a)",
  "thread: n/a",
  "workspace: C:/Users/danie/OneDrive/Documentos/Projetos/Neuromancer",
  "model: gpt-5.2",
  "reasoning: high",
  "approval: on-request",
  "sandbox: workspace-write",
  "web search: enabled",
  "rate limits: unavailable",
].join("\n")

describe("parseStatusSnapshot", () => {
  it("faz parse do snapshot /status sem regressao dos campos principais", () => {
    const parsed = parseStatusSnapshot(STATUS_GOLDEN)

    expect(parsed).not.toBeNull()
    expect(parsed).toMatchObject({
      mode: "running",
      sessionId: 42,
      pid: 9001,
      thread: "thread_abc123",
      workspace: "C:/Users/danie/OneDrive/Documentos/Projetos/Neuromancer",
      model: "gpt-5.2",
      reasoning: "high",
      approval: "on-request",
      sandbox: "workspace-write",
      webSearch: "enabled",
    })
  })

  it("faz parse do ramo de rate limits", () => {
    const parsed = parseStatusSnapshot(STATUS_GOLDEN)

    expect(parsed).not.toBeNull()
    expect(parsed).toMatchObject({
      limitId: "user-123",
      remaining5h: {
        percent: 88.5,
        used: 11.5,
        resetsIn: "4h 12m",
      },
      remainingWeek: {
        percent: 76,
        used: 24,
        resetsIn: "6d 3h",
      },
    })
  })

  it("retorna null quando o conteudo nao inicia com /status", () => {
    expect(parseStatusSnapshot("mode: running")).toBeNull()
  })

  it("mantem fallback explicito no ramo /status com campos indisponiveis", () => {
    const parsed = parseStatusSnapshot(STATUS_GOLDEN_UNAVAILABLE)

    expect(parsed).not.toBeNull()
    expect(parsed?.sessionId).toBe(0)
    expect(parsed?.pid).toBe(0)
    expect(parsed?.thread).toBeNull()
    expect(parsed?.limitId).toBe("")
    expect(parsed?.remaining5h).toEqual({
      percent: 0,
      used: 0,
      resetsIn: "n/a",
    })
    expect(parsed?.remainingWeek).toEqual({
      percent: 0,
      used: 0,
      resetsIn: "n/a",
    })
  })
})
