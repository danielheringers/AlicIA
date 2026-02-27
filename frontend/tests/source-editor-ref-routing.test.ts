import { describe, expect, it } from "vitest"

import { routeSourceEditorRef } from "@/lib/source-editor-ref-routing"

describe("routeSourceEditorRef", () => {
  it("mantem refs ADT no fluxo ABAP", () => {
    const route = routeSourceEditorRef("/sap/bc/adt/oo/classes/zcl_demo/source/main")
    expect(route).toEqual({
      kind: "abap",
      normalizedRef: "/sap/bc/adt/oo/classes/zcl_demo/source/main",
      monacoLanguage: "abap",
    })
  })

  it("mantem refs .abap no fluxo ABAP", () => {
    const route = routeSourceEditorRef("zcl_demo.clas.abap")
    expect(route.kind).toBe("abap")
    expect(route.monacoLanguage).toBe("abap")
  })

  it("normaliza apenas separadores para refs de workspace", () => {
    const route = routeSourceEditorRef("a\\src\\editor\\panel.tsx")
    expect(route).toEqual({
      kind: "workspace",
      normalizedRef: "a/src/editor/panel.tsx",
      monacoLanguage: "typescript",
    })
  })

  it("detecta json como linguagem de workspace", () => {
    const route = routeSourceEditorRef("b/package.json")
    expect(route).toEqual({
      kind: "workspace",
      normalizedRef: "b/package.json",
      monacoLanguage: "json",
    })
  })
})
