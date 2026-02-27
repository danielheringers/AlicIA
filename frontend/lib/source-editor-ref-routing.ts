export type SourceEditorRefKind = "abap" | "workspace"

export interface SourceEditorRefRoute {
  kind: SourceEditorRefKind
  normalizedRef: string
  monacoLanguage: string
}

const MONACO_LANGUAGE_BY_EXTENSION: Record<string, string> = {
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  cjs: "javascript",
  mjs: "javascript",
  json: "json",
  css: "css",
  scss: "scss",
  less: "less",
  html: "html",
  md: "markdown",
  yml: "yaml",
  yaml: "yaml",
  xml: "xml",
  sh: "shell",
  bash: "shell",
  sql: "sql",
  py: "python",
  go: "go",
  rs: "rust",
  java: "java",
  toml: "toml",
}

function stripOuterQuotes(value: string): string {
  return value.replace(/^"+|"+$/g, "").replace(/^'+|'+$/g, "")
}

function normalizeRefInput(ref: string): string {
  return stripOuterQuotes(ref.trim())
}

function isAbapRef(ref: string): boolean {
  if (ref.startsWith("/sap/bc/adt/")) {
    return true
  }
  return ref.toLowerCase().endsWith(".abap")
}

function normalizeWorkspaceRef(ref: string): string {
  return ref.replace(/\\/g, "/")
}

function inferWorkspaceLanguage(path: string): string {
  const normalized = path.replace(/\\/g, "/")
  const baseName = normalized.split("/").filter(Boolean).at(-1) ?? normalized
  if (baseName === "Dockerfile") {
    return "dockerfile"
  }

  const extensionMatch = /\.([a-z0-9]+)$/i.exec(baseName)
  if (!extensionMatch) {
    return "plaintext"
  }

  const extension = extensionMatch[1].toLowerCase()
  return MONACO_LANGUAGE_BY_EXTENSION[extension] ?? "plaintext"
}

export function routeSourceEditorRef(ref: string): SourceEditorRefRoute {
  const normalizedRef = normalizeRefInput(ref)
  if (isAbapRef(normalizedRef)) {
    return {
      kind: "abap",
      normalizedRef,
      monacoLanguage: "abap",
    }
  }

  const workspacePath = normalizeWorkspaceRef(normalizedRef)
  return {
    kind: "workspace",
    normalizedRef: workspacePath,
    monacoLanguage: inferWorkspaceLanguage(workspacePath),
  }
}
