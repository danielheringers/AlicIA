import { describe, expect, it } from "vitest"

import {
  normalizeWorkspacePathForRoot,
  routeSourceEditorRef,
} from "@/lib/source-editor-ref-routing"

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

  it("trata .abap com separador de path como arquivo de workspace", () => {
    const route = routeSourceEditorRef("src/abap/zcl_demo.clas.abap")
    expect(route).toEqual({
      kind: "workspace",
      normalizedRef: "src/abap/zcl_demo.clas.abap",
      monacoLanguage: "plaintext",
    })
  })

  it("trata .abap com drive prefix como arquivo de workspace", () => {
    const route = routeSourceEditorRef("C:\\repo\\abap\\zcl_demo.clas.abap")
    expect(route).toEqual({
      kind: "workspace",
      normalizedRef: "C:/repo/abap/zcl_demo.clas.abap",
      monacoLanguage: "plaintext",
    })
  })

  it("mantem refs ABAP com namespace no fluxo ABAP", () => {
    const route = routeSourceEditorRef("/ABC/ZCL_DEMO.clas.abap")
    expect(route).toEqual({
      kind: "abap",
      normalizedRef: "/ABC/ZCL_DEMO.clas.abap",
      monacoLanguage: "abap",
    })
  })

  it("trata path absoluto unix .abap como workspace", () => {
    const route = routeSourceEditorRef("/workspace/src/zcl_demo.clas.abap")
    expect(route).toEqual({
      kind: "workspace",
      normalizedRef: "/workspace/src/zcl_demo.clas.abap",
      monacoLanguage: "plaintext",
    })
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

describe("normalizeWorkspacePathForRoot", () => {
  it("remove o prefixo absoluto quando path esta dentro do workspace", () => {
    expect(
      normalizeWorkspacePathForRoot(
        "C:/repo/alicia/frontend/app/page.tsx",
        "C:/repo/alicia/frontend",
      ),
    ).toBe("app/page.tsx")
  })

  it("mantem path relativo quando ja estiver relativo", () => {
    expect(
      normalizeWorkspacePathForRoot(
        "components/ide/file-explorer.tsx",
        "C:/repo/alicia/frontend",
      ),
    ).toBe("components/ide/file-explorer.tsx")
  })
})
