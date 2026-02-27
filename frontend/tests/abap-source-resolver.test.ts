import { beforeEach, describe, expect, it, vi } from "vitest"

vi.mock("@/lib/tauri-bridge", () => ({
  neuroSearchObjects: vi.fn(),
}))

import { resolveAbapSourceRef, AbapSourceResolverError } from "@/lib/abap-source-resolver"
import { neuroSearchObjects } from "@/lib/tauri-bridge"
import { type NeuroAdtObjectSummary } from "@/lib/tauri-bridge/types"

const neuroSearchObjectsMock = vi.mocked(neuroSearchObjects)

function objectSummary(
  uri: string,
  name: string,
  objectType: string,
): NeuroAdtObjectSummary {
  return { uri, name, objectType }
}

describe("resolveAbapSourceRef", () => {
  beforeEach(() => {
    neuroSearchObjectsMock.mockReset()
  })

  it("retorna ambiguous quando ampliar limite revela colisao", async () => {
    neuroSearchObjectsMock.mockImplementation(async (query, maxResults) => {
      const normalized = query.trim().toUpperCase()
      if (normalized === "ZCL_COLLIDE") {
        if (maxResults === 12) {
          return [objectSummary("/sap/bc/adt/oo/classes/zcl_collide", "ZCL_COLLIDE", "CLAS")]
        }
        if (maxResults === 100) {
          return [
            objectSummary("/sap/bc/adt/oo/classes/zcl_collide", "ZCL_COLLIDE", "CLAS"),
            objectSummary("/sap/bc/adt/oo/classes/zcl_collide2", "ZCL_COLLIDE", "CLAS"),
          ]
        }
      }
      return []
    })

    await expect(resolveAbapSourceRef("ZCL_COLLIDE")).rejects.toMatchObject({
      code: "ambiguous",
    })
    expect(neuroSearchObjectsMock).toHaveBeenCalledWith("ZCL_COLLIDE", 12)
    expect(neuroSearchObjectsMock).toHaveBeenCalledWith("ZCL_COLLIDE", 100)
  })

  it("usa sufixo .clas.abap para filtrar tipo esperado", async () => {
    neuroSearchObjectsMock.mockImplementation(async (query, maxResults) => {
      const normalized = query.trim().toUpperCase()
      if (normalized.includes("ZCL_TYPED")) {
        if (maxResults === 12 || maxResults === 100) {
          return [
            objectSummary("/sap/bc/adt/oo/classes/zcl_typed", "ZCL_TYPED", "CLAS"),
            objectSummary("/sap/bc/adt/programs/zcl_typed", "ZCL_TYPED", "PROG"),
          ]
        }
      }
      return []
    })

    const resolved = await resolveAbapSourceRef("zcl_typed.clas.abap")
    expect(resolved).toMatchObject({
      objectUri: "/sap/bc/adt/oo/classes/zcl_typed",
      displayName: "ZCL_TYPED",
      resolvedBy: "search",
    })
  })

  it("nao resolve para tipo errado quando tipo esperado nao existe", async () => {
    neuroSearchObjectsMock.mockImplementation(async (query) => {
      const normalized = query.trim().toUpperCase()
      if (normalized.includes("ZCL_MISSING_TYPE")) {
        return [objectSummary("/sap/bc/adt/programs/zcl_missing_type", "ZCL_MISSING_TYPE", "PROG")]
      }
      return []
    })

    await expect(resolveAbapSourceRef("zcl_missing_type.clas.abap")).rejects.toMatchObject({
      code: "not_found",
    })
  })

  it("resolve URI ADT direta sem chamar busca", async () => {
    const uri = "/sap/bc/adt/oo/classes/zcl_direct/source/main"
    const resolved = await resolveAbapSourceRef(uri)

    expect(resolved).toEqual({
      objectUri: uri,
      displayName: "main",
      resolvedBy: "uri",
    })
    expect(neuroSearchObjectsMock).not.toHaveBeenCalled()
  })

  it("mantem erros do resolver tipados", async () => {
    neuroSearchObjectsMock.mockImplementation(async () => {
      throw new Error("network")
    })

    await expect(resolveAbapSourceRef("ZCL_ANY")).rejects.toBeInstanceOf(
      AbapSourceResolverError,
    )
  })

  it("rejeita extensao nao-abap com invalid_ref", async () => {
    await expect(resolveAbapSourceRef("src/report.sql")).rejects.toMatchObject({
      code: "invalid_ref",
    })
    expect(neuroSearchObjectsMock).not.toHaveBeenCalled()
  })
})
