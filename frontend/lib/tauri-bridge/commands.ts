import { invoke } from '@tauri-apps/api/core'

import { RUNTIME_COMMANDS } from '@/lib/tauri-bridge/generated/runtime-contract'
import type {
  CodexApprovalRespondRequest,
  CodexUserInputRespondRequest,
  CodexUserInputRespondResponse,
  CodexHelpSnapshot,
  CodexModelListResponse,
  CodexThreadArchiveRequest,
  CodexThreadArchiveResponse,
  CodexThreadCompactStartRequest,
  CodexThreadCompactStartResponse,
  CodexThreadForkRequest,
  CodexThreadForkResponse,
  CodexThreadCloseRequest,
  CodexThreadCloseResponse,
  CodexThreadListRequest,
  CodexThreadListResponse,
  CodexThreadOpenResponse,
  CodexThreadReadRequest,
  CodexThreadReadResponse,
  CodexThreadRollbackRequest,
  CodexThreadRollbackResponse,
  CodexThreadUnarchiveRequest,
  CodexThreadUnarchiveResponse,
  CodexTurnInterruptRequest,
  CodexTurnInterruptResponse,
  CodexReviewStartRequest,
  CodexReviewStartResponse,
  CodexTurnRunRequest,
  CodexTurnRunResponse,
  CodexTurnSteerRequest,
  CodexTurnSteerResponse,
  McpLoginRequest,
  McpLoginResponse,
  McpReloadResponse,
  McpServerListResponse,
  McpStartupWarmupResponse,
  AppListRequest,
  AppListResponse,
  AccountReadRequest,
  AccountReadResponse,
  AccountLoginStartRequest,
  AccountLoginStartResponse,
  AccountLogoutResponse,
  AccountRateLimitsReadResponse,
  RunCodexCommandResponse,
  WorkspaceChangesResponse,
  CodexWorkspaceReadFileRequest,
  CodexWorkspaceReadFileResponse,
  CodexWorkspaceWriteFileRequest,
  CodexWorkspaceWriteFileResponse,
  CodexWorkspaceCreateDirectoryRequest,
  CodexWorkspaceCreateDirectoryResponse,
  CodexWorkspaceRenameEntryRequest,
  CodexWorkspaceRenameEntryResponse,
  CodexWorkspaceListDirectoryRequest,
  CodexWorkspaceListDirectoryResponse,
  GitCommitApprovedReviewRequest,
  GitCommitApprovedReviewResponse,
  RuntimeCodexConfig,
  RuntimeStatusResponse,
  RuntimeCapabilitiesResponse,
  NeuroAdtObjectSummary,
  NeuroAdtExplorerState,
  NeuroAdtExplorerStatePatchRequest,
  NeuroAdtFavoriteObjectItem,
  NeuroAdtFavoritePackageItem,
  NeuroAdtPackageInventoryNode,
  NeuroAdtPackageInventoryMetadata,
  NeuroAdtPackageInventoryPackageObjects,
  NeuroAdtPackageInventoryRootMetadata,
  NeuroAdtPackageInventoryRequest,
  NeuroAdtPackageInventoryResponse,
  NeuroAdtListObjectsRequest,
  NeuroAdtListObjectsResponse,
  NeuroAdtPackageSummary,
  NeuroAdtNamespaceSummary,
  NeuroAdtServerConnectResponse,
  NeuroAdtServerListResponse,
  NeuroAdtServerRecord,
  NeuroAdtServerUpsertRequest,
  NeuroAdtSourceResponse,
  NeuroAdtUpdateSourceRequest,
  NeuroAdtUpdateSourceResponse,
  NeuroRuntimeCommandError,
  NeuroRuntimeDiagnoseResponse,
  NeuroToolSpec,
  NeuroWsDomainRequest,
  NeuroWsMessageEnvelope,
  StartCodexSessionConfig,
  StartCodexSessionResponse,
  TerminalCreateRequest,
  TerminalCreateResponse,
} from '@/lib/tauri-bridge/types'

type RawNeuroDiagnoseStatus = 'healthy' | 'degraded' | 'unavailable'

interface RawNeuroRuntimeDiagnoseComponent {
  component: string
  status: RawNeuroDiagnoseStatus
  detail: string
  latency_ms?: number | null
}

interface RawNeuroRuntimeDiagnoseResponse {
  timestamp_epoch_secs: number
  overall_status: RawNeuroDiagnoseStatus
  components: RawNeuroRuntimeDiagnoseComponent[]
  metadata: Record<string, unknown>
}

interface RawNeuroAdtObjectSummary {
  uri: string
  name: string
  object_type?: string | null
  package?: string | null
}

interface RawNeuroAdtSourceResponse {
  object_uri: string
  source: string
  etag?: string | null
}

interface RawNeuroAdtUpdateSourceRequest {
  object_uri: string
  source: string
  etag?: string | null
  server_id?: string | null
}

interface RawNeuroAdtUpdateSourceResponse {
  object_uri: string
  status_code: number
  etag?: string | null
}

interface RawNeuroAdtServerRecord {
  id?: string | null
  name?: string | null
  base_url?: string | null
  client?: string | null
  language?: string | null
  username?: string | null
}

interface RawNeuroAdtServerListResponse {
  servers?: RawNeuroAdtServerRecord[] | null
  selected_server_id?: string | null
  selectedServerId?: string | null
}

interface RawNeuroAdtServerUpsertRequest {
  id: string
  name: string
  base_url: string
  client?: string | null
  language?: string | null
  username?: string | null
  password?: string | null
}

interface RawNeuroAdtServerConnectResponse {
  server_id?: string | null
  connected?: boolean | null
  message?: string | null
}

interface RawNeuroAdtPackageSummary {
  name?: string | null
  description?: string | null
}

interface RawNeuroAdtNamespaceSummary {
  name?: string | null
  package_name?: string | null
  packageName?: string | null
}

interface RawNeuroAdtFavoritePackageItem {
  name?: string | null
  kind?: string | null
}

interface RawNeuroAdtFavoriteObjectItem {
  uri?: string | null
  name?: string | null
  object_type?: string | null
  objectType?: string | null
  package?: string | null
}

interface RawNeuroAdtExplorerState {
  working_package?: string | null
  workingPackage?: string | null
  focused_object_uri?: string | null
  focusedObjectUri?: string | null
  package_scope_roots?: string[] | null
  packageScopeRoots?: string[] | null
  favorite_packages?: RawNeuroAdtFavoritePackageItem[] | null
  favoritePackages?: RawNeuroAdtFavoritePackageItem[] | null
  favorite_objects?: RawNeuroAdtFavoriteObjectItem[] | null
  favoriteObjects?: RawNeuroAdtFavoriteObjectItem[] | null
}

interface RawNeuroAdtExplorerStatePatchRequest {
  working_package?: string | null
  focused_object_uri?: string | null
  package_scope_roots?: string[] | null
  toggle_favorite_package?: {
    name: string
    kind: string
  } | null
  toggle_favorite_object?: {
    uri: string
    name: string
    object_type?: string | null
    package?: string | null
  } | null
}

interface RawNeuroAdtListObjectsRequest {
  scope: string
  package_name?: string | null
  package_kind?: string | null
  namespace?: string | null
  server_id?: string | null
  max_results?: number | null
}

interface RawNeuroAdtListObjectsResponse {
  scope?: string | null
  objects?: RawNeuroAdtObjectSummary[] | null
  namespaces?: RawNeuroAdtNamespaceSummary[] | null
}

interface RawNeuroAdtPackageInventoryRequest {
  roots: string[]
  include_subpackages?: boolean | null
  include_objects?: boolean | null
  max_packages?: number | null
  max_objects_per_package?: number | null
  server_id?: string | null
}

interface RawNeuroAdtPackageInventoryNode {
  name?: string | null
  parent_name?: string | null
  parentName?: string | null
  depth?: number | null
  is_root?: boolean | null
  isRoot?: boolean | null
  object_count?: number | null
  objectCount?: number | null
}

interface RawNeuroAdtPackageInventoryPackageObjects {
  package_name?: string | null
  packageName?: string | null
  objects?: RawNeuroAdtObjectSummary[] | null
}

interface RawNeuroAdtPackageInventoryResponse {
  roots?: string[] | null
  packages?: RawNeuroAdtPackageInventoryNode[] | null
  objects_by_package?: RawNeuroAdtPackageInventoryPackageObjects[] | null
  objectsByPackage?: RawNeuroAdtPackageInventoryPackageObjects[] | null
  metadata?: RawNeuroAdtPackageInventoryMetadata | null
}

interface RawNeuroAdtPackageInventoryRootMetadata {
  root?: string | null
  kind?: string | null
  queries_executed?: number | null
  queriesExecuted?: number | null
  matched_packages?: number | null
  matchedPackages?: number | null
  returned_packages?: number | null
  returnedPackages?: number | null
  result_limit_hit?: boolean | null
  resultLimitHit?: boolean | null
  is_complete?: boolean | null
  isComplete?: boolean | null
  skipped_due_to_max_packages?: boolean | null
  skippedDueToMaxPackages?: boolean | null
}

interface RawNeuroAdtPackageInventoryMetadata {
  is_complete?: boolean | null
  isComplete?: boolean | null
  is_truncated?: boolean | null
  isTruncated?: boolean | null
  include_objects?: boolean | null
  includeObjects?: boolean | null
  max_packages_reached?: boolean | null
  maxPackagesReached?: boolean | null
  root_discovery_truncated?: boolean | null
  rootDiscoveryTruncated?: boolean | null
  object_results_truncated?: boolean | null
  objectResultsTruncated?: boolean | null
  max_packages?: number | null
  maxPackages?: number | null
  max_objects_per_package?: number | null
  maxObjectsPerPackage?: number | null
  returned_packages?: number | null
  returnedPackages?: number | null
  packages_with_truncated_objects?: number | null
  packagesWithTruncatedObjects?: number | null
  roots?: RawNeuroAdtPackageInventoryRootMetadata[] | null
}

type RawNeuroAdtListObjectsWireResponse =
  | RawNeuroAdtListObjectsResponse
  | RawNeuroAdtObjectSummary[]

interface RawNeuroWsMessageEnvelope {
  id: string
  domain: string
  action: string
  payload: Record<string, unknown>
  ok?: boolean | null
  error?: string | null
}

interface RawNeuroRuntimeCommandError {
  code: string
  message: string
  details?: Record<string, unknown> | null
}

interface RawNeuroCommandResponse<T> {
  ok: boolean
  data?: T | null
  error?: RawNeuroRuntimeCommandError | null
}

function mapNeuroDiagnoseResponse(
  raw: RawNeuroRuntimeDiagnoseResponse,
): NeuroRuntimeDiagnoseResponse {
  return {
    timestampEpochSecs: raw.timestamp_epoch_secs,
    overallStatus: raw.overall_status,
    components: raw.components.map(component => ({
      component: component.component,
      status: component.status,
      detail: component.detail,
      latencyMs: component.latency_ms,
    })),
    metadata: raw.metadata ?? {},
  }
}

function mapNeuroObjectSummary(raw: RawNeuroAdtObjectSummary): NeuroAdtObjectSummary {
  return {
    uri: raw.uri,
    name: raw.name,
    objectType: raw.object_type,
    package: raw.package,
  }
}

function mapNeuroSourceResponse(
  raw: RawNeuroAdtSourceResponse,
): NeuroAdtSourceResponse {
  return {
    objectUri: raw.object_uri,
    source: raw.source,
    etag: raw.etag,
  }
}

function mapNeuroUpdateSourceResponse(
  raw: RawNeuroAdtUpdateSourceResponse,
): NeuroAdtUpdateSourceResponse {
  return {
    objectUri: raw.object_uri,
    statusCode: raw.status_code,
    etag: raw.etag,
  }
}

function asStringOrNull(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null
  }
  const normalized = value.trim()
  return normalized.length > 0 ? normalized : null
}

function asFiniteNumberOrNull(value: unknown): number | null {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return null
  }
  return value
}

function asBooleanOrDefault(value: unknown, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback
}

function mapNeuroServerRecord(raw: RawNeuroAdtServerRecord): NeuroAdtServerRecord | null {
  const id = asStringOrNull(raw.id)
  const name = asStringOrNull(raw.name)
  const baseUrl = asStringOrNull(raw.base_url)
  if (!id || !name || !baseUrl) {
    return null
  }

  return {
    id,
    name,
    baseUrl,
    client: asStringOrNull(raw.client),
    language: asStringOrNull(raw.language),
    username: asStringOrNull(raw.username),
  }
}

function mapNeuroServerListResponse(raw: unknown): NeuroAdtServerListResponse {
  if (Array.isArray(raw)) {
    const servers = raw
      .map(entry => mapNeuroServerRecord((entry ?? {}) as RawNeuroAdtServerRecord))
      .filter((entry): entry is NeuroAdtServerRecord => entry !== null)
    return {
      servers,
      selectedServerId: null,
    }
  }

  const payload = (raw ?? {}) as RawNeuroAdtServerListResponse
  const sourceServers = Array.isArray(payload.servers) ? payload.servers : []
  return {
    servers: sourceServers
      .map(entry => mapNeuroServerRecord(entry))
      .filter((entry): entry is NeuroAdtServerRecord => entry !== null),
    selectedServerId: asStringOrNull(
      payload.selectedServerId ?? payload.selected_server_id,
    ),
  }
}

function mapNeuroConnectResponse(
  serverId: string,
  raw: unknown,
): NeuroAdtServerConnectResponse {
  if (typeof raw === 'boolean') {
    return {
      serverId,
      connected: raw,
      message: null,
    }
  }

  const payload = (raw ?? {}) as RawNeuroAdtServerConnectResponse
  return {
    serverId: asStringOrNull(payload.server_id) ?? serverId,
    connected: payload.connected === true,
    message: asStringOrNull(payload.message),
  }
}

function mapNeuroPackageSummary(raw: RawNeuroAdtPackageSummary): NeuroAdtPackageSummary | null {
  const name = asStringOrNull(raw.name)
  if (!name) {
    return null
  }
  return {
    name,
    description: asStringOrNull(raw.description),
  }
}

function mapNeuroNamespaceSummary(
  raw: RawNeuroAdtNamespaceSummary,
): NeuroAdtNamespaceSummary | null {
  const name = asStringOrNull(raw.name)
  if (!name) {
    return null
  }
  return {
    name,
    packageName: asStringOrNull(raw.packageName ?? raw.package_name),
  }
}

function mapNeuroFavoritePackageItem(
  raw: RawNeuroAdtFavoritePackageItem,
): NeuroAdtFavoritePackageItem | null {
  const name = asStringOrNull(raw.name)
  const kindRaw = asStringOrNull(raw.kind)
  if (!name || !kindRaw) {
    return null
  }
  const kind = kindRaw === 'namespace' ? 'namespace' : kindRaw === 'package' ? 'package' : null
  if (!kind) {
    return null
  }
  return {
    name,
    kind,
  }
}

function mapNeuroFavoriteObjectItem(
  raw: RawNeuroAdtFavoriteObjectItem,
): NeuroAdtFavoriteObjectItem | null {
  const uri = asStringOrNull(raw.uri)
  const name = asStringOrNull(raw.name)
  if (!uri || !name) {
    return null
  }
  return {
    uri,
    name,
    objectType: asStringOrNull(raw.objectType ?? raw.object_type),
    package: asStringOrNull(raw.package),
  }
}

function mapNeuroExplorerState(raw: RawNeuroAdtExplorerState): NeuroAdtExplorerState {
  const rawFavoritePackages = raw.favoritePackages ?? raw.favorite_packages
  const rawFavoriteObjects = raw.favoriteObjects ?? raw.favorite_objects
  const rawPackageScopeRoots = raw.packageScopeRoots ?? raw.package_scope_roots
  const packageScopeRoots =
    Array.isArray(rawPackageScopeRoots)
      ? rawPackageScopeRoots.reduce<string[]>((accumulator, entry) => {
          const normalized = asStringOrNull(entry)
          if (!normalized) {
            return accumulator
          }
          if (
            accumulator.some((existing) => existing.toUpperCase() === normalized.toUpperCase())
          ) {
            return accumulator
          }
          accumulator.push(normalized)
          return accumulator
        }, [])
      : []
  return {
    workingPackage: asStringOrNull(raw.workingPackage ?? raw.working_package),
    focusedObjectUri: asStringOrNull(raw.focusedObjectUri ?? raw.focused_object_uri),
    packageScopeRoots,
    favoritePackages: Array.isArray(rawFavoritePackages)
      ? rawFavoritePackages
          .map(entry => mapNeuroFavoritePackageItem(entry))
          .filter((entry): entry is NeuroAdtFavoritePackageItem => entry !== null)
      : [],
    favoriteObjects: Array.isArray(rawFavoriteObjects)
      ? rawFavoriteObjects
          .map(entry => mapNeuroFavoriteObjectItem(entry))
          .filter((entry): entry is NeuroAdtFavoriteObjectItem => entry !== null)
      : [],
  }
}

function mapNeuroListObjectsResponse(
  request: NeuroAdtListObjectsRequest,
  raw: RawNeuroAdtListObjectsWireResponse,
): NeuroAdtListObjectsResponse {
  if (Array.isArray(raw)) {
    return {
      scope: request.scope,
      objects: raw.map(mapNeuroObjectSummary),
      namespaces: [],
    }
  }

  const mappedScope = asStringOrNull(raw.scope) ?? request.scope
  const objects = Array.isArray(raw.objects) ? raw.objects.map(mapNeuroObjectSummary) : []
  const namespaces = Array.isArray(raw.namespaces)
    ? raw.namespaces
        .map(entry => mapNeuroNamespaceSummary(entry))
        .filter((entry): entry is NeuroAdtNamespaceSummary => entry !== null)
    : []

  return {
    scope:
      mappedScope === 'local_objects' ||
      mappedScope === 'favorite_packages' ||
      mappedScope === 'favorite_objects' ||
      mappedScope === 'system_library'
        ? mappedScope
        : request.scope,
    objects,
    namespaces,
  }
}

function mapNeuroPackageInventoryNode(
  raw: RawNeuroAdtPackageInventoryNode,
): NeuroAdtPackageInventoryNode | null {
  const name = asStringOrNull(raw.name)
  if (!name) {
    return null
  }

  const rawDepth = raw.depth
  const rawObjectCount = raw.objectCount ?? raw.object_count
  return {
    name,
    parentName: asStringOrNull(raw.parentName ?? raw.parent_name),
    depth:
      typeof rawDepth === 'number' && Number.isFinite(rawDepth) && rawDepth >= 0
        ? Math.trunc(rawDepth)
        : 0,
    isRoot: (raw.isRoot ?? raw.is_root) === true,
    objectCount:
      typeof rawObjectCount === 'number' &&
      Number.isFinite(rawObjectCount) &&
      rawObjectCount >= 0
        ? Math.trunc(rawObjectCount)
        : 0,
  }
}

function mapNeuroPackageInventoryPackageObjects(
  raw: RawNeuroAdtPackageInventoryPackageObjects,
): NeuroAdtPackageInventoryPackageObjects | null {
  const packageName = asStringOrNull(raw.packageName ?? raw.package_name)
  if (!packageName) {
    return null
  }

  return {
    packageName,
    objects: Array.isArray(raw.objects) ? raw.objects.map(mapNeuroObjectSummary) : [],
  }
}

function mapNeuroPackageInventoryRootMetadata(
  raw: RawNeuroAdtPackageInventoryRootMetadata,
): NeuroAdtPackageInventoryRootMetadata | null {
  const root = asStringOrNull(raw.root)
  const kind = asStringOrNull(raw.kind)
  if (!root || !kind) {
    return null
  }

  return {
    root,
    kind,
    queriesExecuted:
      Math.max(0, Math.trunc(asFiniteNumberOrNull(raw.queriesExecuted ?? raw.queries_executed) ?? 0)),
    matchedPackages:
      Math.max(0, Math.trunc(asFiniteNumberOrNull(raw.matchedPackages ?? raw.matched_packages) ?? 0)),
    returnedPackages:
      Math.max(0, Math.trunc(asFiniteNumberOrNull(raw.returnedPackages ?? raw.returned_packages) ?? 0)),
    resultLimitHit: asBooleanOrDefault(raw.resultLimitHit ?? raw.result_limit_hit),
    isComplete: asBooleanOrDefault(raw.isComplete ?? raw.is_complete),
    skippedDueToMaxPackages: asBooleanOrDefault(
      raw.skippedDueToMaxPackages ?? raw.skipped_due_to_max_packages,
    ),
  }
}

function mapNeuroPackageInventoryMetadata(
  raw: RawNeuroAdtPackageInventoryMetadata | null | undefined,
): NeuroAdtPackageInventoryMetadata | null {
  if (!raw || typeof raw !== 'object') {
    return null
  }

  const roots = Array.isArray(raw.roots)
    ? raw.roots
        .map((entry) => mapNeuroPackageInventoryRootMetadata(entry))
        .filter((entry): entry is NeuroAdtPackageInventoryRootMetadata => entry !== null)
    : []

  return {
    isComplete: asBooleanOrDefault(raw.isComplete ?? raw.is_complete),
    isTruncated: asBooleanOrDefault(raw.isTruncated ?? raw.is_truncated),
    includeObjects: asBooleanOrDefault(raw.includeObjects ?? raw.include_objects),
    maxPackagesReached: asBooleanOrDefault(raw.maxPackagesReached ?? raw.max_packages_reached),
    rootDiscoveryTruncated: asBooleanOrDefault(
      raw.rootDiscoveryTruncated ?? raw.root_discovery_truncated,
    ),
    objectResultsTruncated: asBooleanOrDefault(
      raw.objectResultsTruncated ?? raw.object_results_truncated,
    ),
    maxPackages:
      Math.max(0, Math.trunc(asFiniteNumberOrNull(raw.maxPackages ?? raw.max_packages) ?? 0)),
    maxObjectsPerPackage: Math.max(
      0,
      Math.trunc(
        asFiniteNumberOrNull(raw.maxObjectsPerPackage ?? raw.max_objects_per_package) ?? 0,
      ),
    ),
    returnedPackages:
      Math.max(0, Math.trunc(asFiniteNumberOrNull(raw.returnedPackages ?? raw.returned_packages) ?? 0)),
    packagesWithTruncatedObjects: Math.max(
      0,
      Math.trunc(
        asFiniteNumberOrNull(
          raw.packagesWithTruncatedObjects ?? raw.packages_with_truncated_objects,
        ) ?? 0,
      ),
    ),
    roots,
  }
}

function mapNeuroPackageInventoryResponse(
  raw: RawNeuroAdtPackageInventoryResponse,
): NeuroAdtPackageInventoryResponse {
  const roots = Array.isArray(raw.roots)
    ? raw.roots
        .map((entry) => asStringOrNull(entry))
        .filter((entry): entry is string => entry !== null)
    : []
  const packages = Array.isArray(raw.packages)
    ? raw.packages
        .map((entry) => mapNeuroPackageInventoryNode(entry))
        .filter((entry): entry is NeuroAdtPackageInventoryNode => entry !== null)
    : []
  const rawObjectsByPackage = raw.objectsByPackage ?? raw.objects_by_package
  const objectsByPackage = Array.isArray(rawObjectsByPackage)
    ? rawObjectsByPackage
        .map((entry) => mapNeuroPackageInventoryPackageObjects(entry))
        .filter((entry): entry is NeuroAdtPackageInventoryPackageObjects => entry !== null)
    : []

  return {
    roots,
    packages,
    objectsByPackage,
    metadata: mapNeuroPackageInventoryMetadata(raw.metadata),
  }
}

function buildLegacyListObjectsPayload(
  request: NeuroAdtListObjectsRequest,
): RawNeuroAdtListObjectsRequest | null {
  const common = {
    server_id: request.serverId ?? null,
    max_results: request.maxResults ?? null,
  }

  if (request.scope === 'local_objects') {
    return {
      ...common,
      scope: 'local_objects',
      package_name: request.packageName ?? null,
      namespace: request.namespace ?? null,
      package_kind: request.packageKind ?? null,
    }
  }

  if (request.scope === 'favorite_packages') {
    const isNamespaceScope =
      request.packageKind === 'namespace' || asStringOrNull(request.namespace) !== null
    return {
      ...common,
      scope: isNamespaceScope ? 'namespace' : 'package',
      namespace: request.namespace ?? null,
      package_name: request.packageName ?? null,
      package_kind: request.packageKind ?? null,
    }
  }

  if (request.scope === 'system_library') {
    if (asStringOrNull(request.namespace) !== null) {
      return {
        ...common,
        scope: 'namespace',
        namespace: request.namespace ?? null,
        package_name: request.packageName ?? null,
        package_kind: request.packageKind ?? null,
      }
    }
    return null
  }

  return null
}

function mapNeuroWsMessageEnvelope(raw: RawNeuroWsMessageEnvelope): NeuroWsMessageEnvelope {
  return {
    id: raw.id,
    domain: raw.domain,
    action: raw.action,
    payload: raw.payload,
    ok: raw.ok,
    error: raw.error,
  }
}

function parseJsonErrorMessage(message: string): RawNeuroRuntimeCommandError | null {
  try {
    const parsed = JSON.parse(message) as RawNeuroRuntimeCommandError
    if (parsed && typeof parsed.code === 'string' && typeof parsed.message === 'string') {
      return parsed
    }
    return null
  } catch {
    return null
  }
}

function normalizeNeuroRuntimeError(error: unknown): NeuroRuntimeCommandError {
  const unknownFallback: NeuroRuntimeCommandError = {
    code: 'unknown',
    message: error instanceof Error ? error.message : String(error),
    details: null,
  }

  if (error && typeof error === 'object') {
    const candidate = error as Partial<RawNeuroRuntimeCommandError> & {
      message?: unknown
    }

    if (typeof candidate.code === 'string' && typeof candidate.message === 'string') {
      return {
        code: candidate.code as NeuroRuntimeCommandError['code'],
        message: candidate.message,
        details: candidate.details ?? null,
      }
    }

    if (typeof candidate.message === 'string') {
      const parsed = parseJsonErrorMessage(candidate.message)
      if (parsed) {
        return {
          code: parsed.code as NeuroRuntimeCommandError['code'],
          message: parsed.message,
          details: parsed.details ?? null,
        }
      }
      return {
        code: 'unknown',
        message: candidate.message,
        details: null,
      }
    }
  }

  if (typeof error === 'string') {
    const parsed = parseJsonErrorMessage(error)
    if (parsed) {
      return {
        code: parsed.code as NeuroRuntimeCommandError['code'],
        message: parsed.message,
        details: parsed.details ?? null,
      }
    }
    return { code: 'unknown', message: error, details: null }
  }

  return unknownFallback
}

function normalizeRuntimeCommandError(
  error?: RawNeuroRuntimeCommandError | NeuroRuntimeCommandError | null,
): NeuroRuntimeCommandError {
  if (!error) {
    return {
      code: 'unknown',
      message: 'neuro command failed without details',
      details: null,
    }
  }

  return {
    code: error.code as NeuroRuntimeCommandError['code'],
    message: error.message,
    details: error.details ?? null,
  }
}

function unwrapNeuroResponse<T>(
  response: RawNeuroCommandResponse<T>,
): T {
  if (response.ok && response.data != null) {
    return response.data
  }

  throw normalizeRuntimeCommandError(response.error)
}

export async function codexRuntimeStatus(): Promise<RuntimeStatusResponse> {
  return invoke<RuntimeStatusResponse>(RUNTIME_COMMANDS.codexRuntimeStatus)
}

export async function codexRuntimeCapabilities(): Promise<RuntimeCapabilitiesResponse> {
  return invoke<RuntimeCapabilitiesResponse>(RUNTIME_COMMANDS.codexRuntimeCapabilities)
}

export async function neuroRuntimeDiagnose(): Promise<NeuroRuntimeDiagnoseResponse> {
  try {
    const response = await invoke<RawNeuroCommandResponse<RawNeuroRuntimeDiagnoseResponse>>(
      RUNTIME_COMMANDS.neuroRuntimeDiagnose,
    )
    return mapNeuroDiagnoseResponse(unwrapNeuroResponse(response))
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroSearchObjects(
  query: string,
  maxResults?: number,
  serverId?: string | null,
): Promise<NeuroAdtObjectSummary[]> {
  try {
    const response = await invoke<RawNeuroCommandResponse<RawNeuroAdtObjectSummary[]>>(
      RUNTIME_COMMANDS.neuroSearchObjects,
      {
        query,
        maxResults,
        serverId,
      },
    )
    const raw = unwrapNeuroResponse(response)
    return raw.map(mapNeuroObjectSummary)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroGetSource(
  objectUri: string,
  serverId?: string | null,
): Promise<NeuroAdtSourceResponse> {
  try {
    const response = await invoke<RawNeuroCommandResponse<RawNeuroAdtSourceResponse>>(
      RUNTIME_COMMANDS.neuroGetSource,
      { objectUri, serverId },
    )
    const raw = unwrapNeuroResponse(response)
    return mapNeuroSourceResponse(raw)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroUpdateSource(
  request: NeuroAdtUpdateSourceRequest,
): Promise<NeuroAdtUpdateSourceResponse> {
  try {
    const payload: RawNeuroAdtUpdateSourceRequest = {
      object_uri: request.objectUri,
      source: request.source,
      etag: request.etag ?? null,
      server_id: request.serverId ?? null,
    }
    const response = await invoke<RawNeuroCommandResponse<RawNeuroAdtUpdateSourceResponse>>(
      RUNTIME_COMMANDS.neuroUpdateSource,
      {
        request: payload,
      },
    )
    const raw = unwrapNeuroResponse(response)
    return mapNeuroUpdateSourceResponse(raw)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtServerList(): Promise<NeuroAdtServerListResponse> {
  try {
    const response = await invoke<RawNeuroCommandResponse<unknown>>(
      RUNTIME_COMMANDS.neuroAdtServerList,
    )
    return mapNeuroServerListResponse(unwrapNeuroResponse(response))
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtServerUpsert(
  request: NeuroAdtServerUpsertRequest,
): Promise<NeuroAdtServerRecord> {
  try {
    const payload: RawNeuroAdtServerUpsertRequest = {
      id: request.id,
      name: request.name,
      base_url: request.baseUrl,
      client: request.client ?? null,
      language: request.language ?? null,
      username: request.username ?? null,
      password: request.password ?? null,
    }

    const response = await invoke<RawNeuroCommandResponse<RawNeuroAdtServerRecord>>(
      RUNTIME_COMMANDS.neuroAdtServerUpsert,
      { request: payload },
    )
    const mapped = mapNeuroServerRecord(unwrapNeuroResponse(response))
    if (!mapped) {
      throw new Error('neuro_adt_server_upsert returned invalid server payload')
    }
    return mapped
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtServerRemove(serverId: string): Promise<void> {
  try {
    const response = await invoke<RawNeuroCommandResponse<Record<string, unknown>>>(
      RUNTIME_COMMANDS.neuroAdtServerRemove,
      { serverId },
    )
    unwrapNeuroResponse(response)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtServerSelect(serverId: string): Promise<string> {
  try {
    const response = await invoke<RawNeuroCommandResponse<Record<string, unknown>>>(
      RUNTIME_COMMANDS.neuroAdtServerSelect,
      { serverId },
    )
    const payload = unwrapNeuroResponse(response)
    return asStringOrNull(payload.selectedServerId ?? payload.selected_server_id) ?? serverId
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtServerConnect(
  serverId: string,
): Promise<NeuroAdtServerConnectResponse> {
  try {
    const response = await invoke<RawNeuroCommandResponse<unknown>>(
      RUNTIME_COMMANDS.neuroAdtServerConnect,
      { serverId },
    )
    return mapNeuroConnectResponse(serverId, unwrapNeuroResponse(response))
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtListPackages(
  serverId?: string | null,
): Promise<NeuroAdtPackageSummary[]> {
  try {
    const response = await invoke<RawNeuroCommandResponse<RawNeuroAdtPackageSummary[]>>(
      RUNTIME_COMMANDS.neuroAdtListPackages,
      { serverId },
    )
    const raw = unwrapNeuroResponse(response)
    return raw
      .map(entry => mapNeuroPackageSummary(entry))
      .filter((entry): entry is NeuroAdtPackageSummary => entry !== null)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtListNamespaces(
  packageName?: string | null,
  serverId?: string | null,
): Promise<NeuroAdtNamespaceSummary[]> {
  try {
    const response = await invoke<RawNeuroCommandResponse<RawNeuroAdtNamespaceSummary[]>>(
      RUNTIME_COMMANDS.neuroAdtListNamespaces,
      { packageName, serverId },
    )
    const raw = unwrapNeuroResponse(response)
    return raw
      .map(entry => mapNeuroNamespaceSummary(entry))
      .filter((entry): entry is NeuroAdtNamespaceSummary => entry !== null)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtExplorerStateGet(
  serverId?: string | null,
): Promise<NeuroAdtExplorerState> {
  try {
    const response = await invoke<RawNeuroCommandResponse<RawNeuroAdtExplorerState>>(
      RUNTIME_COMMANDS.neuroAdtExplorerStateGet,
      { serverId },
    )
    return mapNeuroExplorerState(unwrapNeuroResponse(response))
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtExplorerStatePatch(
  request: NeuroAdtExplorerStatePatchRequest,
  serverId?: string | null,
): Promise<NeuroAdtExplorerState> {
  try {
    const payload: RawNeuroAdtExplorerStatePatchRequest = {}
    if (request.workingPackage !== undefined) {
      payload.working_package = request.workingPackage
    }
    if (request.focusedObjectUri !== undefined) {
      payload.focused_object_uri = request.focusedObjectUri
    }
    if (request.setPackageScopeRoots !== undefined) {
      payload.package_scope_roots = request.setPackageScopeRoots ?? []
    }
    if (request.toggleFavoritePackage) {
      payload.toggle_favorite_package = {
        name: request.toggleFavoritePackage.name,
        kind: request.toggleFavoritePackage.kind,
      }
    }
    if (request.toggleFavoriteObject) {
      payload.toggle_favorite_object = {
        uri: request.toggleFavoriteObject.uri,
        name: request.toggleFavoriteObject.name,
        object_type: request.toggleFavoriteObject.objectType ?? null,
        package: request.toggleFavoriteObject.package ?? null,
      }
    }
    const response = await invoke<RawNeuroCommandResponse<RawNeuroAdtExplorerState>>(
      RUNTIME_COMMANDS.neuroAdtExplorerStatePatch,
      {
        request: payload,
        serverId,
      },
    )
    return mapNeuroExplorerState(unwrapNeuroResponse(response))
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroAdtListObjects(
  request: NeuroAdtListObjectsRequest,
): Promise<NeuroAdtListObjectsResponse> {
  const payload: RawNeuroAdtListObjectsRequest = {
    scope: request.scope,
    package_name: request.packageName ?? null,
    package_kind: request.packageKind ?? null,
    namespace: request.namespace ?? null,
    server_id: request.serverId ?? null,
    max_results: request.maxResults ?? null,
  }

  try {
    const response = await invoke<
      RawNeuroCommandResponse<RawNeuroAdtListObjectsWireResponse>
    >(
      RUNTIME_COMMANDS.neuroAdtListObjects,
      { request: payload },
    )
    return mapNeuroListObjectsResponse(request, unwrapNeuroResponse(response))
  } catch (error) {
    const normalizedError = normalizeNeuroRuntimeError(error)
    const isCompatibilityScopeError =
      normalizedError.code === 'invalid_argument' &&
      normalizedError.message.toLowerCase().includes('scope')

    if (request.scope === 'favorite_objects' && isCompatibilityScopeError) {
      return {
        scope: request.scope,
        objects: [],
        namespaces: [],
      }
    }

    if (request.scope === 'system_library' && !asStringOrNull(request.namespace)) {
      try {
        const fallbackNamespaces = await neuroAdtListNamespaces(
          request.packageName ?? null,
          request.serverId ?? null,
        )
        return {
          scope: request.scope,
          objects: [],
          namespaces: fallbackNamespaces,
        }
      } catch {
        throw normalizedError
      }
    }

    if (!isCompatibilityScopeError) {
      throw normalizedError
    }

    const legacyPayload = buildLegacyListObjectsPayload(request)
    if (!legacyPayload) {
      throw normalizedError
    }

    try {
      const legacyResponse = await invoke<
        RawNeuroCommandResponse<RawNeuroAdtListObjectsWireResponse>
      >(RUNTIME_COMMANDS.neuroAdtListObjects, {
        request: legacyPayload,
      })
      return mapNeuroListObjectsResponse(request, unwrapNeuroResponse(legacyResponse))
    } catch {
      throw normalizedError
    }
  }
}

export async function neuroAdtListPackageInventory(
  request: NeuroAdtPackageInventoryRequest,
): Promise<NeuroAdtPackageInventoryResponse> {
  try {
    const payload: RawNeuroAdtPackageInventoryRequest = {
      roots: Array.isArray(request.roots) ? request.roots : [],
      include_subpackages: request.includeSubpackages ?? null,
      include_objects: request.includeObjects ?? null,
      max_packages: request.maxPackages ?? null,
      max_objects_per_package: request.maxObjectsPerPackage ?? null,
      server_id: request.serverId ?? null,
    }
    const response = await invoke<RawNeuroCommandResponse<RawNeuroAdtPackageInventoryResponse>>(
      RUNTIME_COMMANDS.neuroAdtListPackageInventory,
      { request: payload },
    )
    return mapNeuroPackageInventoryResponse(unwrapNeuroResponse(response))
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroWsRequest(
  request: NeuroWsDomainRequest,
): Promise<NeuroWsMessageEnvelope> {
  try {
    const response = await invoke<RawNeuroCommandResponse<RawNeuroWsMessageEnvelope>>(
      RUNTIME_COMMANDS.neuroWsRequest,
      { request },
    )
    const raw = unwrapNeuroResponse(response)
    return mapNeuroWsMessageEnvelope(raw)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroListTools(): Promise<NeuroToolSpec[]> {
  try {
    const response = await invoke<RawNeuroCommandResponse<NeuroToolSpec[]>>(
      RUNTIME_COMMANDS.neuroListTools,
    )
    return unwrapNeuroResponse(response)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function neuroInvokeTool(
  toolName: string,
  argumentsPayload: Record<string, unknown>,
): Promise<Record<string, unknown>> {
  try {
    const response = await invoke<RawNeuroCommandResponse<Record<string, unknown>>>(
      RUNTIME_COMMANDS.neuroInvokeTool,
      {
        toolName,
        arguments: argumentsPayload,
      },
    )
    return unwrapNeuroResponse(response)
  } catch (error) {
    throw normalizeNeuroRuntimeError(error)
  }
}

export async function loadCodexDefaultConfig(): Promise<RuntimeCodexConfig> {
  return invoke<RuntimeCodexConfig>(RUNTIME_COMMANDS.loadCodexDefaultConfig)
}

const RUNTIME_SESSION_START_COMMAND = RUNTIME_COMMANDS.startCodexSession
const RUNTIME_SESSION_STOP_COMMAND = RUNTIME_COMMANDS.stopCodexSession

export async function codexRuntimeSessionStart(
  config?: StartCodexSessionConfig,
): Promise<StartCodexSessionResponse> {
  return invoke<StartCodexSessionResponse>(RUNTIME_SESSION_START_COMMAND, {
    config,
  })
}

export async function codexRuntimeSessionStop(): Promise<void> {
  await invoke(RUNTIME_SESSION_STOP_COMMAND)
}

export async function startCodexSession(
  config?: StartCodexSessionConfig,
): Promise<StartCodexSessionResponse> {
  return invoke<StartCodexSessionResponse>(RUNTIME_COMMANDS.startCodexSession, { config })
}

export async function stopCodexSession(): Promise<void> {
  await invoke(RUNTIME_COMMANDS.stopCodexSession)
}

export async function codexTurnRun(
  request: CodexTurnRunRequest,
): Promise<CodexTurnRunResponse> {
  return invoke<CodexTurnRunResponse>(RUNTIME_COMMANDS.codexTurnRun, { request })
}

export async function codexTurnSteer(
  request: CodexTurnSteerRequest,
): Promise<CodexTurnSteerResponse> {
  return invoke<CodexTurnSteerResponse>(RUNTIME_COMMANDS.codexTurnSteer, { request })
}

export async function codexTurnInterrupt(
  request: CodexTurnInterruptRequest,
): Promise<CodexTurnInterruptResponse> {
  return invoke<CodexTurnInterruptResponse>(RUNTIME_COMMANDS.codexTurnInterrupt, { request })
}
export async function codexReviewStart(
  request: CodexReviewStartRequest,
): Promise<CodexReviewStartResponse> {
  return invoke<CodexReviewStartResponse>(RUNTIME_COMMANDS.codexReviewStart, { request })
}

export async function codexThreadOpen(
  threadId?: string,
): Promise<CodexThreadOpenResponse> {
  return invoke<CodexThreadOpenResponse>(RUNTIME_COMMANDS.codexThreadOpen, { threadId })
}

export async function codexThreadClose(
  request: CodexThreadCloseRequest,
): Promise<CodexThreadCloseResponse> {
  return invoke<CodexThreadCloseResponse>(RUNTIME_COMMANDS.codexThreadClose, { request })
}

export async function codexThreadList(
  request?: CodexThreadListRequest,
): Promise<CodexThreadListResponse> {
  return invoke<CodexThreadListResponse>(RUNTIME_COMMANDS.codexThreadList, { request })
}

export async function codexThreadRead(
  request: CodexThreadReadRequest,
): Promise<CodexThreadReadResponse> {
  return invoke<CodexThreadReadResponse>(RUNTIME_COMMANDS.codexThreadRead, { request })
}

export async function codexThreadArchive(
  request: CodexThreadArchiveRequest,
): Promise<CodexThreadArchiveResponse> {
  return invoke<CodexThreadArchiveResponse>(RUNTIME_COMMANDS.codexThreadArchive, { request })
}

export async function codexThreadUnarchive(
  request: CodexThreadUnarchiveRequest,
): Promise<CodexThreadUnarchiveResponse> {
  return invoke<CodexThreadUnarchiveResponse>(RUNTIME_COMMANDS.codexThreadUnarchive, {
    request,
  })
}

export async function codexThreadCompactStart(
  request: CodexThreadCompactStartRequest,
): Promise<CodexThreadCompactStartResponse> {
  return invoke<CodexThreadCompactStartResponse>(RUNTIME_COMMANDS.codexThreadCompactStart, {
    request,
  })
}

export async function codexThreadRollback(
  request: CodexThreadRollbackRequest,
): Promise<CodexThreadRollbackResponse> {
  return invoke<CodexThreadRollbackResponse>(RUNTIME_COMMANDS.codexThreadRollback, { request })
}

export async function codexThreadFork(
  request: CodexThreadForkRequest,
): Promise<CodexThreadForkResponse> {
  return invoke<CodexThreadForkResponse>(RUNTIME_COMMANDS.codexThreadFork, { request })
}

export async function codexApprovalRespond(
  request: CodexApprovalRespondRequest,
): Promise<void> {
  await invoke(RUNTIME_COMMANDS.codexApprovalRespond, { request })
}

export async function codexUserInputRespond(
  request: CodexUserInputRespondRequest,
): Promise<CodexUserInputRespondResponse> {
  return invoke<CodexUserInputRespondResponse>(RUNTIME_COMMANDS.codexUserInputRespond, { request })
}

export async function sendCodexInput(text: string): Promise<void> {
  await invoke(RUNTIME_COMMANDS.sendCodexInput, { text })
}

export async function updateCodexRuntimeConfig(
  config: RuntimeCodexConfig,
): Promise<RuntimeCodexConfig> {
  return invoke<RuntimeCodexConfig>(RUNTIME_COMMANDS.updateCodexConfig, { config })
}

export async function codexConfigGet(): Promise<RuntimeCodexConfig> {
  return invoke<RuntimeCodexConfig>(RUNTIME_COMMANDS.codexConfigGet)
}

export async function codexConfigSet(
  patch: RuntimeCodexConfig,
): Promise<RuntimeCodexConfig> {
  return invoke<RuntimeCodexConfig>(RUNTIME_COMMANDS.codexConfigSet, { patch })
}

export async function runCodexCommand(
  args: string[],
  cwd?: string,
): Promise<RunCodexCommandResponse> {
  return invoke<RunCodexCommandResponse>(RUNTIME_COMMANDS.runCodexCommand, { args, cwd })
}


export async function codexWorkspaceChanges(): Promise<WorkspaceChangesResponse> {
  return invoke<WorkspaceChangesResponse>(RUNTIME_COMMANDS.gitWorkspaceChanges)
}

export async function codexWorkspaceReadFile(
  request: CodexWorkspaceReadFileRequest,
): Promise<CodexWorkspaceReadFileResponse> {
  return invoke<CodexWorkspaceReadFileResponse>(RUNTIME_COMMANDS.codexWorkspaceReadFile, {
    request,
  })
}

export async function codexWorkspaceWriteFile(
  request: CodexWorkspaceWriteFileRequest,
): Promise<CodexWorkspaceWriteFileResponse> {
  return invoke<CodexWorkspaceWriteFileResponse>(RUNTIME_COMMANDS.codexWorkspaceWriteFile, {
    request,
  })
}

export async function codexWorkspaceCreateDirectory(
  request: CodexWorkspaceCreateDirectoryRequest,
): Promise<CodexWorkspaceCreateDirectoryResponse> {
  return invoke<CodexWorkspaceCreateDirectoryResponse>(
    RUNTIME_COMMANDS.codexWorkspaceCreateDirectory,
    {
      request,
    },
  )
}

export async function codexWorkspaceRenameEntry(
  request: CodexWorkspaceRenameEntryRequest,
): Promise<CodexWorkspaceRenameEntryResponse> {
  return invoke<CodexWorkspaceRenameEntryResponse>(RUNTIME_COMMANDS.codexWorkspaceRenameEntry, {
    request,
  })
}

export async function codexWorkspaceListDirectory(
  request?: CodexWorkspaceListDirectoryRequest,
): Promise<CodexWorkspaceListDirectoryResponse> {
  return invoke<CodexWorkspaceListDirectoryResponse>(RUNTIME_COMMANDS.codexWorkspaceListDirectory, {
    request,
  })
}

export async function gitCommitApprovedReview(
  request: GitCommitApprovedReviewRequest,
): Promise<GitCommitApprovedReviewResponse> {
  return invoke<GitCommitApprovedReviewResponse>(RUNTIME_COMMANDS.gitCommitApprovedReview, {
    request,
  })
}

export async function codexModelsList(): Promise<CodexModelListResponse> {
  return invoke<CodexModelListResponse>(RUNTIME_COMMANDS.codexModelsList)
}

export async function codexWaitForMcpStartup(): Promise<McpStartupWarmupResponse> {
  return invoke<McpStartupWarmupResponse>(RUNTIME_COMMANDS.codexWaitForMcpStartup)
}

export async function codexMcpList(): Promise<McpServerListResponse> {
  return invoke<McpServerListResponse>(RUNTIME_COMMANDS.codexMcpList)
}

export async function codexAppList(
  request?: AppListRequest,
): Promise<AppListResponse> {
  return invoke<AppListResponse>(RUNTIME_COMMANDS.codexAppList, { request })
}

export async function codexAccountRead(
  request?: AccountReadRequest,
): Promise<AccountReadResponse> {
  return invoke<AccountReadResponse>(RUNTIME_COMMANDS.codexAccountRead, { request })
}

export async function codexAccountLoginStart(
  request: AccountLoginStartRequest,
): Promise<AccountLoginStartResponse> {
  return invoke<AccountLoginStartResponse>(RUNTIME_COMMANDS.codexAccountLoginStart, { request })
}

export async function codexAccountLogout(): Promise<AccountLogoutResponse> {
  return invoke<AccountLogoutResponse>(RUNTIME_COMMANDS.codexAccountLogout)
}

export async function codexAccountRateLimitsRead(): Promise<AccountRateLimitsReadResponse> {
  return invoke<AccountRateLimitsReadResponse>(RUNTIME_COMMANDS.codexAccountRateLimitsRead)
}
export async function codexMcpLogin(
  request: McpLoginRequest,
): Promise<McpLoginResponse> {
  return invoke<McpLoginResponse>(RUNTIME_COMMANDS.codexMcpLogin, { request })
}

export async function codexMcpReload(): Promise<McpReloadResponse> {
  return invoke<McpReloadResponse>(RUNTIME_COMMANDS.codexMcpReload)
}

export async function terminalCreate(
  request?: TerminalCreateRequest,
): Promise<TerminalCreateResponse> {
  return invoke<TerminalCreateResponse>(RUNTIME_COMMANDS.terminalCreate, { request })
}

export async function terminalWrite(
  terminalId: number,
  data: string,
): Promise<void> {
  await invoke(RUNTIME_COMMANDS.terminalWrite, {
    request: { terminalId, data },
  })
}

export async function terminalResize(
  terminalId: number,
  cols: number,
  rows: number,
): Promise<void> {
  await invoke(RUNTIME_COMMANDS.terminalResize, {
    request: { terminalId, cols, rows },
  })
}

export async function terminalKill(terminalId: number): Promise<void> {
  await invoke(RUNTIME_COMMANDS.terminalKill, {
    request: { terminalId },
  })
}

export async function pickImageFile(): Promise<string | null> {
  return invoke<string | null>(RUNTIME_COMMANDS.pickImageFile)
}

export async function pickMentionFile(): Promise<string | null> {
  return invoke<string | null>(RUNTIME_COMMANDS.pickMentionFile)
}

export async function pickWorkspaceFolder(): Promise<string | null> {
  return invoke<string | null>(RUNTIME_COMMANDS.pickWorkspaceFolder)
}

export async function codexHelpSnapshot(): Promise<CodexHelpSnapshot> {
  return invoke<CodexHelpSnapshot>(RUNTIME_COMMANDS.codexHelpSnapshot)
}

export async function resizeCodexPty(rows: number, cols: number): Promise<void> {
  await invoke(RUNTIME_COMMANDS.resizeCodexPty, { rows, cols })
}

