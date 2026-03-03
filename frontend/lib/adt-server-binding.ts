interface AdtServerBindingSnapshot {
  availableServerIds: string[]
  selectedServerId: string | null | undefined
}

function normalizeServerId(value: string | null | undefined): string | null {
  if (typeof value !== "string") {
    return null
  }
  const normalized = value.trim()
  return normalized.length > 0 ? normalized : null
}

export function selectBoundAdtServerId(
  preferredServerId: string | null | undefined,
  snapshot: AdtServerBindingSnapshot,
): string | null {
  const normalizedPreferred = normalizeServerId(preferredServerId)
  const availableIds = new Set(
    snapshot.availableServerIds
      .map((serverId) => normalizeServerId(serverId))
      .filter((serverId): serverId is string => Boolean(serverId)),
  )

  if (normalizedPreferred && availableIds.has(normalizedPreferred)) {
    return normalizedPreferred
  }

  const selected = normalizeServerId(snapshot.selectedServerId)
  if (selected && availableIds.has(selected)) {
    return selected
  }

  return null
}

export function requireBoundAdtServerIdForAbap(
  adtServerId: string | null | undefined,
  operation: string,
): string {
  const normalized = normalizeServerId(adtServerId)
  if (normalized) {
    return normalized
  }
  throw new Error(
    `Nao foi possivel ${operation}: servidor ADT nao vinculado ao editor ABAP. Reabra a fonte selecionando um servidor ADT.`,
  )
}
