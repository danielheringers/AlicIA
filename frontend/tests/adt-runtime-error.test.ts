import { describe, expect, it } from "vitest"

import { formatAdtRuntimeError } from "@/lib/adt-runtime-error"

describe("formatAdtRuntimeError", () => {
  it("formata Error nativo", () => {
    expect(formatAdtRuntimeError(new Error("falha de rede"))).toBe(
      "falha de rede",
    )
  })

  it("formata objeto com code e message", () => {
    expect(
      formatAdtRuntimeError({
        code: "ADT_AUTH",
        message: "credenciais invalidas",
      }),
    ).toBe("ADT_AUTH: credenciais invalidas")
  })

  it("usa fallback para objeto sem message/code", () => {
    expect(formatAdtRuntimeError({ detail: "opaque" })).toBe("erro desconhecido")
  })
})
