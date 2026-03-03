import { describe, expect, it } from "vitest"

import { buildSapContextInputText } from "@/hooks/use-alicia-actions"

describe("buildSapContextInputText", () => {
  it("retorna null sem contexto SAP", () => {
    expect(buildSapContextInputText(null)).toBeNull()
  })

  it("monta payload compacto com campos disponiveis", () => {
    expect(
      buildSapContextInputText({
        serverId: "adt-prd",
        workPackage: "ZPKG_CORE",
        focusedObjectUri: "/sap/bc/adt/programs/programs/ZHELLO",
      }),
    ).toBe(
      "[sap-context] serverId=adt-prd workPackage=ZPKG_CORE focusedObject=/sap/bc/adt/programs/programs/ZHELLO",
    )
  })

  it("inclui apenas campos presentes", () => {
    expect(
      buildSapContextInputText({
        serverId: "adt-dev",
      }),
    ).toBe("[sap-context] serverId=adt-dev")
  })
})
