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

function isWorkspaceLikePath(ref: string): boolean {
  if (ref.includes("/") || ref.includes("\\")) {
    return true
  }
  return /^[a-z]:/i.test(ref)
}

function isAbapRef(ref: string): boolean {
  if (ref.startsWith("/sap/bc/adt/")) {
    return true
  }
  if (!ref.toLowerCase().endsWith(".abap")) {
    return false
  }
  if (isNamespacedAbapRef(ref)) {
    return true
  }
  return !isWorkspaceLikePath(ref)
}

function normalizeWorkspaceRef(ref: string): string {
  return ref.replace(/\\/g, "/")
}

function isNamespacedAbapRef(ref: string): boolean {
  if (!ref.startsWith("/") || ref.includes("\\")) {
    return false
  }
  const match = /^\/([A-Z0-9_]+)\/([^/]+\.abap)$/u.exec(ref)
  if (!match) {
    return false
  }
  return true
}

export function normalizeWorkspacePathForRoot(
  workspacePath: string,
  workspaceRoot: string | null | undefined,
): string {
  const normalizedPath = normalizeWorkspaceRef(workspacePath).trim()
  if (!normalizedPath) {
    return normalizedPath
  }

  const normalizedRoot = normalizeWorkspaceRef(workspaceRoot ?? "")
    .trim()
    .replace(/\/+$/, "")

  if (!normalizedRoot) {
    return normalizedPath.replace(/^\.\/+/, "")
  }

  const lowerPath = normalizedPath.toLowerCase()
  const lowerRoot = normalizedRoot.toLowerCase()
  const rootPrefix = `${lowerRoot}/`

  if (lowerPath === lowerRoot) {
    return normalizedPath
  }

  if (lowerPath.startsWith(rootPrefix)) {
    return normalizedPath
      .slice(normalizedRoot.length + 1)
      .replace(/^\/+/, "")
  }

  return normalizedPath.replace(/^\.\/+/, "")
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
