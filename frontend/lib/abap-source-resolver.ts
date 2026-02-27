import { neuroSearchObjects } from "@/lib/tauri-bridge"
import { type NeuroAdtObjectSummary } from "@/lib/tauri-bridge/types"

export type AbapSourceResolverErrorCode =
  | "invalid_ref"
  | "not_found"
  | "ambiguous"
  | "search_failed"

export interface AbapSourceResolverErrorShape {
  code: AbapSourceResolverErrorCode
  message: string
  ref: string
  details?: Record<string, unknown>
}

export class AbapSourceResolverError extends Error {
  readonly code: AbapSourceResolverErrorCode
  readonly ref: string
  readonly details: Record<string, unknown> | undefined

  constructor(shape: AbapSourceResolverErrorShape) {
    super(shape.message)
    this.name = "AbapSourceResolverError"
    this.code = shape.code
    this.ref = shape.ref
    this.details = shape.details
  }
}

export interface ResolvedAbapSourceRef {
  objectUri: string
  displayName: string
  resolvedBy: "uri" | "search"
}

const ABAP_SUFFIX_TO_OBJECT_TYPES: Record<string, string[]> = {
  clas: ["CLAS"],
  intf: ["INTF"],
  prog: ["PROG"],
  fugr: ["FUGR"],
  tabl: ["TABL"],
  ttyp: ["TTYP"],
  dtel: ["DTEL"],
  doma: ["DOMA"],
  view: ["VIEW"],
  msag: ["MSAG"],
  tran: ["TRAN"],
}

function isAdtUri(ref: string): boolean {
  return ref.startsWith("/sap/bc/adt/")
}

function summarizeMatches(matches: NeuroAdtObjectSummary[]): string[] {
  return matches.map((entry) =>
    [entry.name, entry.objectType].filter(Boolean).join(" "),
  )
}

function normalizeInputRef(ref: string): string {
  return ref.trim().replace(/^"+|"+$/g, "").replace(/^'+|'+$/g, "")
}

function stripGitDiffPrefix(ref: string): string {
  const normalized = ref.replace(/\\/g, "/")
  if (normalized.startsWith("a/") || normalized.startsWith("b/")) {
    return normalized.slice(2)
  }
  return normalized
}

function extractBaseName(ref: string): string {
  const normalized = ref.replace(/\\/g, "/")
  const parts = normalized.split("/").filter(Boolean)
  return parts.at(-1) ?? ref
}

function extractExtension(name: string): string | null {
  const match = /\.([a-z0-9]+)$/i.exec(name)
  if (!match) {
    return null
  }
  return match[1].toLowerCase()
}

function addSearchTerm(terms: Set<string>, value: string): void {
  const normalized = value.trim()
  if (!normalized) {
    return
  }
  terms.add(normalized)
  const upper = normalized.toUpperCase()
  if (upper !== normalized) {
    terms.add(upper)
  }
}

function deriveSearchTerms(ref: string): string[] {
  const terms = new Set<string>()
  const cleaned = stripGitDiffPrefix(ref)
  const baseName = extractBaseName(cleaned)

  addSearchTerm(terms, ref)
  addSearchTerm(terms, cleaned)
  addSearchTerm(terms, baseName)
  addSearchTerm(terms, baseName.replace(/\.[^.]+$/, ""))

  if (baseName.toLowerCase().endsWith(".abap")) {
    const parts = baseName.split(".").filter(Boolean)
    if (parts.length > 0) {
      addSearchTerm(terms, parts[0])
    }
    if (parts.length > 1) {
      addSearchTerm(terms, `${parts[0]}.${parts[1]}`)
    }
  }

  return Array.from(terms)
}

function findExactNameMatches(
  results: NeuroAdtObjectSummary[],
  term: string,
): NeuroAdtObjectSummary[] {
  const normalizedTerm = term.trim().toUpperCase()
  return results.filter(
    (entry) => entry.name.trim().toUpperCase() === normalizedTerm,
  )
}

function inferExpectedObjectTypes(baseName: string): Set<string> | null {
  if (!baseName.toLowerCase().endsWith(".abap")) {
    return null
  }

  const parts = baseName.toLowerCase().split(".").filter(Boolean)
  if (parts.length < 2) {
    return null
  }

  const mapped = ABAP_SUFFIX_TO_OBJECT_TYPES[parts[1]]
  if (!mapped || mapped.length === 0) {
    return null
  }

  return new Set(mapped.map((entry) => entry.toUpperCase()))
}

function filterByExpectedObjectTypes(
  results: NeuroAdtObjectSummary[],
  expectedObjectTypes: Set<string> | null,
): NeuroAdtObjectSummary[] {
  if (!expectedObjectTypes || expectedObjectTypes.size === 0) {
    return results
  }

  const filtered = results.filter((entry) => {
    const type = entry.objectType?.trim().toUpperCase()
    if (!type) {
      return false
    }
    for (const expected of expectedObjectTypes) {
      if (type === expected || type.startsWith(expected)) {
        return true
      }
    }
    return false
  })

  return filtered
}

const SEARCH_RESULT_LIMIT = 12
const SEARCH_RESULT_REVALIDATION_LIMIT = 100

async function revalidateUniqueMatch(
  term: string,
  expectedObjectTypes: Set<string> | null,
  matcher: (results: NeuroAdtObjectSummary[]) => NeuroAdtObjectSummary[],
): Promise<NeuroAdtObjectSummary[]> {
  const rawResults = await neuroSearchObjects(term, SEARCH_RESULT_REVALIDATION_LIMIT)
  const filteredResults = filterByExpectedObjectTypes(rawResults, expectedObjectTypes)
  return matcher(filteredResults)
}

export async function resolveAbapSourceRef(
  ref: string,
): Promise<ResolvedAbapSourceRef> {
  const normalizedRef = normalizeInputRef(ref)

  if (!normalizedRef) {
    throw new AbapSourceResolverError({
      code: "invalid_ref",
      message: "Referencia vazia para abrir fonte ABAP.",
      ref,
    })
  }

  if (isAdtUri(normalizedRef)) {
    const displayName = normalizedRef.split("/").filter(Boolean).at(-1) ?? normalizedRef
    return {
      objectUri: normalizedRef,
      displayName,
      resolvedBy: "uri",
    }
  }

  const baseName = extractBaseName(stripGitDiffPrefix(normalizedRef))
  const extension = extractExtension(baseName)
  if (extension && extension !== "abap") {
    throw new AbapSourceResolverError({
      code: "invalid_ref",
      message: `Referencia "${normalizedRef}" parece arquivo .${extension} e nao um objeto ABAP.`,
      ref: normalizedRef,
      details: { extension },
    })
  }

  const searchTerms = deriveSearchTerms(normalizedRef).slice(0, 6)
  if (searchTerms.length === 0) {
    throw new AbapSourceResolverError({
      code: "invalid_ref",
      message: `Referencia invalida para resolucao ABAP: "${normalizedRef}".`,
      ref: normalizedRef,
    })
  }
  const expectedObjectTypes = inferExpectedObjectTypes(baseName)

  try {
    const searchedTerms: string[] = []
    let firstExactAmbiguousTerm: string | null = null
    let firstExactAmbiguousCandidates: NeuroAdtObjectSummary[] = []
    let firstBroadAmbiguousTerm: string | null = null
    let firstBroadAmbiguousCandidates: NeuroAdtObjectSummary[] = []
    const singleCandidatesByUri = new Map<
      string,
      { match: NeuroAdtObjectSummary; terms: string[] }
    >()

    for (const term of searchTerms) {
      const rawResults = await neuroSearchObjects(term, SEARCH_RESULT_LIMIT)
      searchedTerms.push(term)

      const results = filterByExpectedObjectTypes(rawResults, expectedObjectTypes)
      if (results.length === 0) {
        continue
      }

      const exactMatches = findExactNameMatches(results, term)
      if (exactMatches.length === 1) {
        const revalidatedExactMatches = await revalidateUniqueMatch(
          term,
          expectedObjectTypes,
          (revalidatedResults) => findExactNameMatches(revalidatedResults, term),
        )

        if (revalidatedExactMatches.length === 1) {
          const [match] = revalidatedExactMatches
          return {
            objectUri: match.uri,
            displayName: match.name,
            resolvedBy: "search",
          }
        }

        if (revalidatedExactMatches.length > 1 && !firstExactAmbiguousTerm) {
          firstExactAmbiguousTerm = term
          firstExactAmbiguousCandidates = revalidatedExactMatches
        }
        continue
      }

      if (exactMatches.length > 1) {
        if (!firstExactAmbiguousTerm) {
          firstExactAmbiguousTerm = term
          firstExactAmbiguousCandidates = exactMatches
        }
        continue
      }

      if (results.length === 1) {
        const revalidatedBroadMatches = await revalidateUniqueMatch(
          term,
          expectedObjectTypes,
          (revalidatedResults) => revalidatedResults,
        )
        if (revalidatedBroadMatches.length !== 1) {
          if (revalidatedBroadMatches.length > 1 && !firstBroadAmbiguousTerm) {
            firstBroadAmbiguousTerm = term
            firstBroadAmbiguousCandidates = revalidatedBroadMatches
          }
          continue
        }

        const [match] = revalidatedBroadMatches
        const existing = singleCandidatesByUri.get(match.uri)
        if (existing) {
          existing.terms.push(term)
        } else {
          singleCandidatesByUri.set(match.uri, {
            match,
            terms: [term],
          })
        }
        continue
      }

      if (!firstBroadAmbiguousTerm) {
        firstBroadAmbiguousTerm = term
        firstBroadAmbiguousCandidates = results
      }
    }

    const uniqueSingleCandidates = Array.from(singleCandidatesByUri.values())
    if (uniqueSingleCandidates.length === 1 && uniqueSingleCandidates[0].terms.length >= 2) {
      const { match } = uniqueSingleCandidates[0]
      return {
        objectUri: match.uri,
        displayName: match.name,
        resolvedBy: "search",
      }
    }

    if (firstExactAmbiguousTerm && firstExactAmbiguousCandidates.length > 0) {
      throw new AbapSourceResolverError({
        code: "ambiguous",
        message: `Referencia ambigua "${normalizedRef}" (termo "${firstExactAmbiguousTerm}"). Refine o nome do objeto.`,
        ref: normalizedRef,
        details: {
          searchedTerms,
          expectedObjectTypes: expectedObjectTypes
            ? Array.from(expectedObjectTypes)
            : undefined,
          candidates: summarizeMatches(firstExactAmbiguousCandidates.slice(0, 8)),
        },
      })
    }

    if (
      (firstBroadAmbiguousTerm && firstBroadAmbiguousCandidates.length > 0) ||
      uniqueSingleCandidates.length > 1 ||
      uniqueSingleCandidates.length === 1
    ) {
      const singleCandidates = uniqueSingleCandidates.flatMap((entry) =>
        summarizeMatches([entry.match]),
      )
      const broadCandidates =
        firstBroadAmbiguousCandidates.length > 0
          ? summarizeMatches(firstBroadAmbiguousCandidates.slice(0, 8))
          : []
      throw new AbapSourceResolverError({
        code: "ambiguous",
        message: `Referencia ambigua "${normalizedRef}". Refine o nome do objeto.`,
        ref: normalizedRef,
        details: {
          searchedTerms,
          expectedObjectTypes: expectedObjectTypes
            ? Array.from(expectedObjectTypes)
            : undefined,
          candidates: Array.from(new Set([...singleCandidates, ...broadCandidates])),
        },
      })
    }

    throw new AbapSourceResolverError({
      code: "not_found",
      message: `Nenhum objeto ABAP encontrado para "${normalizedRef}".`,
      ref: normalizedRef,
      details: {
        searchedTerms,
        expectedObjectTypes: expectedObjectTypes
          ? Array.from(expectedObjectTypes)
          : undefined,
      },
    })
  } catch (error) {
    if (error instanceof AbapSourceResolverError) {
      throw error
    }

    throw new AbapSourceResolverError({
      code: "search_failed",
      message: `Falha ao resolver "${normalizedRef}" via neuroSearchObjects.`,
      ref: normalizedRef,
      details: {
        cause: String(error),
      },
    })
  }
}
