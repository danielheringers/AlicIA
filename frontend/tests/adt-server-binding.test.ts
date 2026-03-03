import { describe, expect, it } from "vitest"

import {
  requireBoundAdtServerIdForAbap,
  selectBoundAdtServerId,
} from "@/lib/adt-server-binding"

describe("selectBoundAdtServerId", () => {
  it("mantem servidor pinado quando ainda disponivel", () => {
    const selected = selectBoundAdtServerId("srv-a", {
      availableServerIds: ["srv-a", "srv-b"],
      selectedServerId: "srv-b",
    })
    expect(selected).toBe("srv-a")
  })

  it("usa servidor selecionado quando pin nao esta disponivel", () => {
    const selected = selectBoundAdtServerId("srv-missing", {
      availableServerIds: ["srv-a", "srv-b"],
      selectedServerId: "srv-b",
    })
    expect(selected).toBe("srv-b")
  })
})

describe("requireBoundAdtServerIdForAbap", () => {
  it("retorna servidor vinculado para save/reload", () => {
    expect(
      requireBoundAdtServerIdForAbap("srv-a", "salvar fonte ABAP"),
    ).toBe("srv-a")
  })

  it("falha quando save/reload nao tem servidor vinculado", () => {
    expect(() =>
      requireBoundAdtServerIdForAbap(null, "recarregar fonte ABAP"),
    ).toThrow(/servidor ADT nao vinculado/i)
  })
})
