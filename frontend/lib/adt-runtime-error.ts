interface RuntimeErrorLike {
  message?: unknown
  code?: unknown
}

function isRuntimeErrorLike(value: unknown): value is RuntimeErrorLike {
  return typeof value === "object" && value !== null
}

function normalizeCode(code: unknown): string | null {
  if (typeof code === "string") {
    const normalized = code.trim()
    return normalized.length > 0 ? normalized : null
  }

  if (typeof code === "number" && Number.isFinite(code)) {
    return String(code)
  }

  return null
}

function normalizeMessage(message: unknown): string | null {
  if (typeof message !== "string") {
    return null
  }

  const normalized = message.trim()
  return normalized.length > 0 ? normalized : null
}

export function formatAdtRuntimeError(
  error: unknown,
  fallback = "erro desconhecido",
): string {
  if (error instanceof Error) {
    const message = normalizeMessage(error.message)
    return message ?? fallback
  }

  if (isRuntimeErrorLike(error)) {
    const code = normalizeCode(error.code)
    const message = normalizeMessage(error.message)

    if (code && message) {
      return `${code}: ${message}`
    }
    if (message) {
      return message
    }
    if (code) {
      return code
    }
  }

  if (typeof error === "string") {
    const normalized = error.trim()
    return normalized.length > 0 ? normalized : fallback
  }

  return fallback
}
