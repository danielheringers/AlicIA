export type AdtChatIntent =
  | {
      type: "connect"
      serverId?: string | null
    }
  | {
      type: "status"
    }
  | {
      type: "invalid"
      message: string
    }

function normalizeIntentInput(input: string): string {
  return input
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
}

export function parseAdtSlashIntent(input: string): AdtChatIntent | null {
  const trimmed = input.trim()
  if (!trimmed.startsWith("/")) {
    return null
  }

  const [name = "", ...rest] = trimmed.split(/\s+/)
  if (name.toLowerCase() !== "/adt") {
    return null
  }

  const subcommand = (rest[0] ?? "").trim().toLowerCase()
  if (!subcommand) {
    return { type: "connect", serverId: null }
  }

  if (subcommand === "connect") {
    const serverId = rest.slice(1).join(" ").trim()
    return {
      type: "connect",
      serverId: serverId || null,
    }
  }

  if (subcommand === "status") {
    if (rest.length > 1) {
      return { type: "invalid", message: "uso: /adt status" }
    }
    return { type: "status" }
  }

  return {
    type: "invalid",
    message: "uso: /adt [connect [server-id] | status]",
  }
}

export function isExplicitAdtConnectionTestIntent(input: string): boolean {
  const normalized = normalizeIntentInput(input)
  if (!/\badt\b/.test(normalized)) {
    return false
  }

  const hasTestVerb = /\b(teste|testar|testa|valida|validar|verifica|verificar|check)\b/.test(
    normalized,
  )
  if (!hasTestVerb) {
    return false
  }

  return /\b(conexao|connection|connect|conectar)\b/.test(normalized)
}

export function isExplicitAdtStatusIntent(input: string): boolean {
  const normalized = normalizeIntentInput(input)
  if (!/\badt\b/.test(normalized)) {
    return false
  }

  const directPatterns = [
    /\bvoce esta com acesso ao adt\b/,
    /\btem acesso ao adt\b/,
    /\badt esta (online|disponivel)\b/,
    /\bcan you check adt status\b/,
    /\bcheck adt status\b/,
    /\bis adt (online|available)\b/,
  ]
  if (directPatterns.some((pattern) => pattern.test(normalized))) {
    return true
  }

  if (!normalized.includes("?")) {
    return false
  }

  const technicalContext =
    /\b(api|endpoint|payload|hook|componente|component|funcao|function|codigo|code|regex|teste|test|implement|erro|bug|fix|arquivo|frontend|backend|typescript|javascript|rust)\b/
  if (technicalContext.test(normalized)) {
    return false
  }

  return (
    /\b(tem|ha|há) acesso ao adt\b/.test(normalized) ||
    /\bvoce esta com acesso ao adt\b/.test(normalized) ||
    /\b(qual|como).*\bstatus do adt\b/.test(normalized) ||
    /\badt (esta )?(online|disponivel)\b/.test(normalized) ||
    /\bcan you check adt status\b/.test(normalized) ||
    /\bis adt (online|available)\b/.test(normalized)
  )
}
