import { describe, expect, it } from "vitest"

import {
  isExplicitAdtConnectionTestIntent,
  isExplicitAdtStatusIntent,
  parseAdtSlashIntent,
} from "@/lib/adt-chat-intents"

describe("parseAdtSlashIntent", () => {
  it("interpreta /adt como connect no servidor ativo", () => {
    expect(parseAdtSlashIntent("/adt")).toEqual({
      type: "connect",
      serverId: null,
    })
  })

  it("interpreta /adt connect sem id", () => {
    expect(parseAdtSlashIntent("/adt connect")).toEqual({
      type: "connect",
      serverId: null,
    })
  })

  it("interpreta /adt connect com server-id", () => {
    expect(parseAdtSlashIntent("/adt connect srv-prd")).toEqual({
      type: "connect",
      serverId: "srv-prd",
    })
  })

  it("interpreta /adt status", () => {
    expect(parseAdtSlashIntent("/adt status")).toEqual({
      type: "status",
    })
  })

  it("retorna invalid para subcomando desconhecido", () => {
    expect(parseAdtSlashIntent("/adt ping")).toEqual({
      type: "invalid",
      message: "uso: /adt [connect [server-id] | status]",
    })
  })

  it("ignora slash commands que nao sao ADT", () => {
    expect(parseAdtSlashIntent("/status")).toBeNull()
  })
})

describe("isExplicitAdtConnectionTestIntent", () => {
  it("detecta frase explicita de teste de conexao ADT", () => {
    expect(
      isExplicitAdtConnectionTestIntent("Teste a conexão com o ADT"),
    ).toBe(true)
  })

  it("aceita variacao sem acento", () => {
    expect(
      isExplicitAdtConnectionTestIntent("pode testar conexao do adt?"),
    ).toBe(true)
  })

  it("nao dispara quando falta ADT", () => {
    expect(
      isExplicitAdtConnectionTestIntent("teste a conexao com o backend"),
    ).toBe(false)
  })

  it("nao dispara quando falta verbo de teste", () => {
    expect(isExplicitAdtConnectionTestIntent("conectar no adt")).toBe(false)
  })
})

describe("isExplicitAdtStatusIntent", () => {
  it("detecta pergunta de acesso ADT", () => {
    expect(isExplicitAdtStatusIntent("voce esta com acesso ao adt?")).toBe(true)
  })

  it("detecta pergunta curta de acesso", () => {
    expect(isExplicitAdtStatusIntent("tem acesso ao adt?")).toBe(true)
  })

  it("detecta pedido de status em ingles", () => {
    expect(isExplicitAdtStatusIntent("can you check ADT status?")).toBe(true)
  })

  it("nao dispara sem ADT", () => {
    expect(isExplicitAdtStatusIntent("tem acesso ao backend?")).toBe(false)
  })

  it("nao dispara em mensagem tecnica sem pergunta explicita", () => {
    expect(
      isExplicitAdtStatusIntent("ajuste o hook de ADT para mapear status da conexao"),
    ).toBe(false)
  })

  it("nao dispara em pergunta tecnica que so cita ADT", () => {
    expect(
      isExplicitAdtStatusIntent("no endpoint ADT, qual status code devemos retornar?"),
    ).toBe(false)
  })

  it("nao dispara em pedido de implementacao com ADT", () => {
    expect(
      isExplicitAdtStatusIntent("implemente o parser de status do ADT no frontend"),
    ).toBe(false)
  })
})
