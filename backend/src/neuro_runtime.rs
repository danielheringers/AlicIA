use neuro_adt_core::AdtClientError;
use neuro_adt_ws::NeuroWsClientError;
use neuro_engine::NeuroEngineError;
use neuro_mcp::{NeuroMcpError, NeuroMcpFacade, NeuroToolSpec};
use neuro_types::{
    AdtAuth, AdtHttpConfig, AdtHttpEndpoints, DiagnoseStatus, NeuroEngineConfig, NeuroRuntimeError,
    NeuroRuntimeErrorCode, RuntimeDiagnoseComponent, RuntimeDiagnoseResponse, SafetyPolicy,
    WsClientConfig, WsDomainRequest, WsMessageEnvelope,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::env;
use std::future::Future;

use std::sync::Arc;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::State;

pub use crate::application::neuro_adt::contracts::{
    AdtServerConnectResponse, AdtServerListResponse, AdtServerRecord, AdtServerRemoveResponse,
    AdtServerSelectResponse, AdtServerUpsertRequest,
};
use crate::application::neuro_adt::ports::{AdtServerConnectivity, NeuroAdtFuture, NeuroAdtPort};
use crate::application::neuro_adt::use_cases as neuro_adt_use_cases;
use crate::domain::neuro_adt::error::NeuroAdtError;
use crate::domain::neuro_adt::server_store as neuro_server_domain;
#[cfg(test)]
use crate::domain::neuro_adt::types::AdtServerUpsertInput;
pub(crate) use crate::domain::neuro_adt::types::ENV_DEFAULT_SERVER_ID;
#[cfg(test)]
pub(crate) use crate::domain::neuro_adt::types::NEURO_ADT_SERVER_STORE_PATH_ENV;
pub use crate::domain::neuro_adt::types::{
    AdtFavoriteObject, AdtFavoritePackage, AdtFavoritePackageKind,
};
use crate::domain::neuro_adt::types::{AdtServerStore, StoredAdtExplorerState, StoredAdtServer};
use crate::infrastructure::filesystem::neuro_server_store as neuro_server_store_fs;
use crate::AppState;

const DEFAULT_ADT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_WS_TIMEOUT_SECS: u64 = 15;
const DEFAULT_ADT_CSRF_FETCH_PATH: &str = "/sap/bc/adt/core/discovery";
const DEFAULT_ADT_SEARCH_PATH: &str =
    "/sap/bc/adt/repository/informationsystem/search?operation=quickSearch";
const NEURO_COMMAND_TELEMETRY_EVENT: &str = "neuro.command";
const NEURO_RUNTIME_INIT_TELEMETRY_EVENT: &str = "neuro.runtime_init";
const NEURO_RUNTIME_INIT_TELEMETRY_VERBOSE_ENV: &str = "NEURO_RUNTIME_INIT_TELEMETRY_VERBOSE";

const DEFAULT_PACKAGE_DISCOVERY_QUERY: &str = "*";
const DEFAULT_NAMESPACE_DISCOVERY_QUERY: &str = "*";
const DEFAULT_DISCOVERY_MAX_RESULTS: u32 = 5000;
const DEFAULT_PACKAGE_INVENTORY_MAX_OBJECTS_PER_PACKAGE: u32 = 250;
const MAX_PACKAGE_INVENTORY_MAX_PACKAGES: u32 = DEFAULT_DISCOVERY_MAX_RESULTS;
const MAX_PACKAGE_INVENTORY_MAX_OBJECTS_PER_PACKAGE: u32 = 1000;
const MAX_PACKAGE_INVENTORY_DISCOVERY_QUERIES_PER_ROOT: usize = 40;
const PACKAGE_INVENTORY_DISCOVERY_BUCKETS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_/$";
const MAX_PACKAGE_SCOPE_ROOTS: usize = 200;
const MAX_PACKAGE_SCOPE_ROOT_LENGTH: usize = 120;

#[derive(Debug, Clone)]
struct EnvValue {
    value: String,
    source: &'static str,
}

#[derive(Debug, Clone)]
struct SapConfig {
    url: Option<EnvValue>,
    user: Option<EnvValue>,
    password: Option<EnvValue>,
    client: Option<EnvValue>,
    language: Option<EnvValue>,
    insecure_tls: bool,
    insecure_tls_source: Option<&'static str>,
    timeout_secs: u64,
    timeout_source: Option<&'static str>,
    csrf_fetch_path: EnvValue,
    search_objects_path: EnvValue,
}

impl SapConfig {
    fn is_ready(&self) -> bool {
        self.url.is_some() && self.user.is_some() && self.password.is_some()
    }
}

#[derive(Debug, Clone)]
struct WsConfig {
    url: Option<EnvValue>,
    timeout_secs: u64,
    timeout_source: Option<&'static str>,
    headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct SafetyConfig {
    read_only: bool,
    read_only_source: Option<&'static str>,
    blocked_source_patterns: Vec<String>,
    blocked_source_patterns_source: Option<&'static str>,
    allowed_ws_domains: Vec<String>,
    allowed_ws_domains_source: Option<&'static str>,
    require_etag_for_updates: bool,
    require_etag_for_updates_source: Option<&'static str>,
}

#[derive(Debug, Clone)]
struct RuntimeConfig {
    sap: SapConfig,
    ws: WsConfig,
    safety: SafetyConfig,
    server_selection: ServerSelection,
}

#[derive(Debug, Clone)]
struct ServerSelection {
    cache_key: String,
    server_id: Option<String>,
    server_name: Option<String>,
    source: &'static str,
}

#[derive(Debug, Clone)]
struct RuntimeInitTarget {
    cache_key: String,
    resolved_server_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtPackageSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtNamespaceSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtExplorerStateResponse {
    pub server_id: String,
    #[serde(default)]
    pub selected_work_package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_package: Option<String>,
    #[serde(default)]
    pub package_scope_roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_object_uri: Option<String>,
    #[serde(default)]
    pub favorite_packages: Vec<AdtFavoritePackage>,
    #[serde(default)]
    pub favorite_objects: Vec<AdtFavoriteObject>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AdtFavoritePackageToggleRequest {
    Name(String),
    LegacyItem {
        name: String,
        #[serde(default)]
        kind: Option<AdtFavoritePackageKind>,
    },
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AdtExplorerStatePatchRequest {
    #[serde(default)]
    pub server_id: Option<String>,
    #[serde(default)]
    pub toggle_favorite_namespace: Option<String>,
    #[serde(default)]
    pub toggle_favorite_package: Option<AdtFavoritePackageToggleRequest>,
    #[serde(default)]
    pub toggle_favorite_object: Option<AdtFavoriteObject>,
    #[serde(default)]
    pub set_work_package: Option<Option<String>>,
    #[serde(default)]
    pub working_package: Option<Option<String>>,
    #[serde(default, alias = "package_scope_roots", alias = "packageScopeRoots")]
    pub package_scope_roots: Option<Vec<String>>,
    #[serde(default, alias = "focusedObjectUri")]
    pub focused_object_uri: Option<Option<String>>,
}

impl AdtExplorerStatePatchRequest {
    fn requested_work_package(&self) -> Option<Option<String>> {
        self.set_work_package
            .clone()
            .or_else(|| self.working_package.clone())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdtListObjectsScope {
    LocalObjects,
    FavoritePackages,
    FavoriteObjects,
    SystemLibrary,
    Namespace,
    Package,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdtListObjectsResponseScope {
    LocalObjects,
    FavoritePackages,
    FavoriteObjects,
    SystemLibrary,
}

impl AdtListObjectsScope {
    fn response_scope(self) -> AdtListObjectsResponseScope {
        match self {
            AdtListObjectsScope::LocalObjects => AdtListObjectsResponseScope::LocalObjects,
            AdtListObjectsScope::FavoritePackages | AdtListObjectsScope::Package => {
                AdtListObjectsResponseScope::FavoritePackages
            }
            AdtListObjectsScope::FavoriteObjects => AdtListObjectsResponseScope::FavoriteObjects,
            AdtListObjectsScope::SystemLibrary | AdtListObjectsScope::Namespace => {
                AdtListObjectsResponseScope::SystemLibrary
            }
        }
    }

    fn is_legacy_namespace(self) -> bool {
        matches!(self, AdtListObjectsScope::Namespace)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtListObjectsResponse {
    pub scope: AdtListObjectsResponseScope,
    #[serde(default)]
    pub objects: Vec<neuro_types::AdtObjectSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespaces: Option<Vec<AdtNamespaceSummary>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdtListObjectsRequest {
    pub scope: AdtListObjectsScope,
    #[serde(default, alias = "namespace")]
    pub namespace: Option<String>,
    #[serde(default, alias = "packageName")]
    pub package_name: Option<String>,
    #[serde(default, alias = "packageKind")]
    pub package_kind: Option<AdtFavoritePackageKind>,
    #[serde(default, alias = "maxResults")]
    pub max_results: Option<u32>,
    #[serde(default, alias = "serverId")]
    pub server_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdtPackageInventoryRequest {
    pub roots: Vec<String>,
    #[serde(default, alias = "includeSubpackages")]
    pub include_subpackages: Option<bool>,
    #[serde(default, alias = "includeObjects", alias = "include_objects")]
    pub include_objects: Option<bool>,
    #[serde(default, alias = "maxPackages")]
    pub max_packages: Option<u32>,
    #[serde(
        default,
        alias = "maxObjectsPerPackage",
        alias = "max_objects_per_package"
    )]
    pub max_objects_per_package: Option<u32>,
    #[serde(default, alias = "serverId")]
    pub server_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtPackageInventoryResponse {
    pub roots: Vec<String>,
    pub packages: Vec<AdtPackageInventoryNode>,
    pub objects_by_package: Vec<AdtPackageInventoryPackageObjects>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AdtPackageInventoryMetadata>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtPackageInventoryMetadata {
    pub is_complete: bool,
    pub is_truncated: bool,
    pub include_objects: bool,
    pub max_packages_reached: bool,
    pub root_discovery_truncated: bool,
    pub object_results_truncated: bool,
    pub max_packages: u32,
    pub max_objects_per_package: u32,
    pub returned_packages: u32,
    pub packages_with_truncated_objects: u32,
    pub roots: Vec<AdtPackageInventoryRootMetadata>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtPackageInventoryRootMetadata {
    pub root: String,
    pub kind: String,
    pub queries_executed: u32,
    pub matched_packages: u32,
    pub returned_packages: u32,
    pub result_limit_hit: bool,
    pub is_complete: bool,
    pub skipped_due_to_max_packages: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtPackageInventoryNode {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_name: Option<String>,
    pub depth: u32,
    pub is_root: bool,
    pub object_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtPackageInventoryPackageObjects {
    pub package_name: String,
    pub objects: Vec<neuro_types::AdtObjectSummary>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum AdtPackageRootKind {
    Package,
    Namespace,
    Pattern,
}

#[derive(Debug, Clone)]
struct AdtPackageRootSpec {
    kind: AdtPackageRootKind,
    query: String,
    response_value: String,
    dedupe_key: String,
}

#[derive(Debug, Clone)]
struct AdtPackageInventoryVisit {
    name: String,
    parent_name: Option<String>,
    depth: u32,
    is_root: bool,
    recurse_subpackages: bool,
}

#[derive(Debug, Clone)]
struct AdtPackageInventoryRootSeed {
    name: String,
    recurse_subpackages: bool,
}

#[derive(Debug, Clone)]
struct AdtNodeStructureRef {
    name: String,
    object_type: Option<String>,
    parent_name: Option<String>,
}

#[derive(Debug, Clone)]
struct AdtPackageInventoryRootDiscovery {
    packages: Vec<String>,
    queries_executed: u32,
    result_limit_hit: bool,
    is_complete: bool,
}

#[derive(Debug, Clone)]
struct AdtPackageInventoryRootDiscoverySummary {
    packages: Vec<AdtPackageInventoryRootSeed>,
    roots: Vec<AdtPackageInventoryRootMetadata>,
    max_packages_reached: bool,
    root_discovery_truncated: bool,
}

#[derive(Debug, Clone)]
struct AdtPackageInventoryObjectsResult {
    objects: Vec<neuro_types::AdtObjectSummary>,
    is_truncated: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdtUpdateSourceCommandRequest {
    pub object_uri: String,
    pub source: String,
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub server_id: Option<String>,
}

impl AdtUpdateSourceCommandRequest {
    fn into_update_request(self) -> neuro_types::AdtUpdateSourceRequest {
        neuro_types::AdtUpdateSourceRequest {
            object_uri: self.object_uri,
            source: self.source,
            etag: self.etag,
        }
    }
}

pub struct NeuroRuntime {
    config: RuntimeConfig,
    engine: Arc<neuro_engine::NeuroEngine>,
}

impl NeuroRuntime {
    async fn initialize(server_id: Option<&str>) -> Result<Self, NeuroRuntimeError> {
        let config = resolve_runtime_config(server_id)?;
        let base_url = config
            .sap
            .url
            .as_ref()
            .map(|value| value.value.clone())
            .ok_or_else(|| {
                runtime_error(
                    NeuroRuntimeErrorCode::RuntimeInitError,
                    "missing SAP URL: set NEURO_SAP_URL or SAP_URL".to_string(),
                    None,
                )
            })?;

        let auth = match (&config.sap.user, &config.sap.password) {
            (Some(user), Some(password)) => AdtAuth::Basic {
                username: user.value.clone(),
                password: password.value.clone(),
            },
            _ => AdtAuth::Anonymous,
        };

        let ws = config.ws.url.as_ref().map(|url| WsClientConfig {
            url: url.value.clone(),
            request_timeout_secs: config.ws.timeout_secs,
            connect_headers: config.ws.headers.clone(),
        });

        let engine = Arc::new(
            neuro_engine::NeuroEngine::new(NeuroEngineConfig {
                adt: AdtHttpConfig {
                    base_url,
                    auth,
                    timeout_secs: config.sap.timeout_secs,
                    csrf_fetch_path: config.sap.csrf_fetch_path.value.clone(),
                    endpoints: AdtHttpEndpoints {
                        search_objects_path: config.sap.search_objects_path.value.clone(),
                    },
                    insecure_tls: config.sap.insecure_tls,
                    sap_client: config.sap.client.as_ref().map(|value| value.value.clone()),
                    sap_language: config
                        .sap
                        .language
                        .as_ref()
                        .map(|value| value.value.clone()),
                },
                ws,
                safety: SafetyPolicy {
                    read_only: config.safety.read_only,
                    blocked_source_patterns: config.safety.blocked_source_patterns.clone(),
                    allowed_ws_domains: config.safety.allowed_ws_domains.clone(),
                    require_etag_for_updates: config.safety.require_etag_for_updates,
                },
            })
            .await
            .map_err(map_engine_error)?,
        );

        Ok(Self { config, engine })
    }

    pub(crate) fn selected_server_id(&self) -> Option<String> {
        self.config.server_selection.server_id.clone()
    }

    pub(crate) async fn adt_http_connectivity(&self) -> (bool, Option<String>) {
        let report = self.engine.diagnose().await;
        let component = report
            .components
            .iter()
            .find(|entry| entry.component == "adt_http");

        match component {
            Some(component) if component.status == DiagnoseStatus::Healthy => {
                (true, Some(component.detail.clone()))
            }
            Some(component) => (false, Some(component.detail.clone())),
            None => (
                false,
                Some("ADT diagnose component unavailable".to_string()),
            ),
        }
    }
}

#[cfg(test)]
pub(crate) fn shared_env_lock() -> &'static Mutex<()> {
    crate::domain::neuro_adt::types::shared_env_lock()
}

#[cfg(test)]
fn telemetry_events() -> &'static Mutex<Vec<String>> {
    static EVENTS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(test)]
fn push_test_telemetry_event(event: String) {
    telemetry_events()
        .lock()
        .expect("telemetry event lock poisoned")
        .push(event);
}

#[cfg(test)]
fn drain_test_telemetry_events() -> Vec<String> {
    std::mem::take(
        &mut *telemetry_events()
            .lock()
            .expect("telemetry event lock poisoned"),
    )
}

fn emit_neuro_telemetry(payload: Value) {
    let line = payload.to_string();
    #[cfg(test)]
    push_test_telemetry_event(line.clone());
    eprintln!("[neuro-telemetry] {line}");
}

fn latency_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn error_code_value(code: &NeuroRuntimeErrorCode) -> Value {
    serde_json::to_value(code).unwrap_or_else(|_| json!(format!("{code:?}")))
}

fn emit_neuro_command_telemetry<T>(
    command_name: &'static str,
    duration: Duration,
    result: &Result<T, NeuroRuntimeError>,
) {
    let mut payload = serde_json::Map::new();
    payload.insert("event".to_string(), json!(NEURO_COMMAND_TELEMETRY_EVENT));
    payload.insert("command".to_string(), json!(command_name));
    payload.insert("success".to_string(), json!(result.is_ok()));
    payload.insert("latencyMs".to_string(), json!(latency_millis(duration)));

    if let Err(error) = result {
        payload.insert("errorCode".to_string(), error_code_value(&error.code));
        payload.insert("errorMessage".to_string(), json!(error.message.as_str()));
    }

    emit_neuro_telemetry(Value::Object(payload));
}

fn emit_neuro_runtime_init_telemetry(
    status: &'static str,
    duration: Duration,
    error: Option<&NeuroRuntimeError>,
) {
    if !should_emit_neuro_runtime_init_telemetry(status) {
        return;
    }

    let mut payload = serde_json::Map::new();
    payload.insert(
        "event".to_string(),
        json!(NEURO_RUNTIME_INIT_TELEMETRY_EVENT),
    );
    payload.insert("status".to_string(), json!(status));
    payload.insert("success".to_string(), json!(error.is_none()));
    payload.insert("latencyMs".to_string(), json!(latency_millis(duration)));

    if let Some(error) = error {
        payload.insert("errorCode".to_string(), error_code_value(&error.code));
        payload.insert("errorMessage".to_string(), json!(error.message.as_str()));
    }

    emit_neuro_telemetry(Value::Object(payload));
}

fn should_emit_neuro_runtime_init_telemetry(status: &str) -> bool {
    match status {
        "cache_hit" | "cache_hit_after_gate" => runtime_init_telemetry_verbose_enabled(),
        _ => true,
    }
}

fn runtime_init_telemetry_verbose_enabled() -> bool {
    env::var(NEURO_RUNTIME_INIT_TELEMETRY_VERBOSE_ENV)
        .ok()
        .as_deref()
        .and_then(parse_env_bool)
        == Some(true)
}

async fn run_with_neuro_command_telemetry<T, F>(
    command_name: &'static str,
    operation: F,
) -> Result<T, NeuroRuntimeError>
where
    F: Future<Output = Result<T, NeuroRuntimeError>>,
{
    let started_at = Instant::now();
    let result = operation.await;
    emit_neuro_command_telemetry(command_name, started_at.elapsed(), &result);
    result
}

fn first_non_empty_env(keys: &[&'static str]) -> Option<EnvValue> {
    keys.iter().find_map(|key| {
        env::var(key).ok().and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(EnvValue {
                    value: trimmed.to_string(),
                    source: key,
                })
            }
        })
    })
}

fn parse_env_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

fn parse_env_u64(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok()
}

fn parse_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize_optional_field(value: Option<String>) -> Option<String> {
    value.and_then(|entry| {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_favorite_package(entry: AdtFavoritePackage) -> Option<AdtFavoritePackage> {
    let name = normalize_optional_field(Some(entry.name))?;
    Some(AdtFavoritePackage {
        kind: entry.kind,
        name,
    })
}

fn normalize_favorite_object(entry: AdtFavoriteObject) -> Option<AdtFavoriteObject> {
    let uri = normalize_optional_field(Some(entry.uri))?;
    let name = normalize_optional_field(Some(entry.name))?;
    Some(AdtFavoriteObject {
        uri,
        name,
        object_type: normalize_optional_field(entry.object_type),
        package: normalize_optional_field(entry.package),
    })
}

fn favorite_package_cmp_key(entry: &AdtFavoritePackage) -> String {
    format!("{:?}:{}", entry.kind, entry.name.to_ascii_uppercase())
}

fn favorite_object_cmp_key(entry: &AdtFavoriteObject) -> String {
    entry.uri.to_ascii_uppercase()
}

fn normalize_package_scope_roots(entries: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut normalized = Vec::<String>::new();

    for entry in entries {
        let Some(trimmed) = normalize_optional_field(Some(entry)) else {
            continue;
        };
        let capped = trimmed
            .chars()
            .take(MAX_PACKAGE_SCOPE_ROOT_LENGTH)
            .collect::<String>();
        let Some(root) = normalize_optional_field(Some(capped)) else {
            continue;
        };
        let key = root.to_ascii_uppercase();
        if !seen.insert(key) {
            continue;
        }
        normalized.push(root);
        if normalized.len() >= MAX_PACKAGE_SCOPE_ROOTS {
            break;
        }
    }

    normalized
}

fn normalize_explorer_state(state: &mut StoredAdtExplorerState) {
    state.selected_work_package = normalize_optional_field(state.selected_work_package.take());
    state.package_scope_roots =
        normalize_package_scope_roots(std::mem::take(&mut state.package_scope_roots));
    state.focused_object_uri = normalize_optional_field(state.focused_object_uri.take());

    let mut package_seen = HashSet::<String>::new();
    let mut normalized_packages = Vec::<AdtFavoritePackage>::new();
    for entry in std::mem::take(&mut state.favorite_packages) {
        let normalized = match normalize_favorite_package(entry) {
            Some(value) => value,
            None => continue,
        };
        let key = favorite_package_cmp_key(&normalized);
        if !package_seen.insert(key) {
            continue;
        }
        normalized_packages.push(normalized);
    }
    normalized_packages.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| {
                left.name
                    .to_ascii_uppercase()
                    .cmp(&right.name.to_ascii_uppercase())
            })
            .then_with(|| left.name.cmp(&right.name))
    });
    state.favorite_packages = normalized_packages;

    let mut object_seen = HashSet::<String>::new();
    let mut normalized_objects = Vec::<AdtFavoriteObject>::new();
    for entry in std::mem::take(&mut state.favorite_objects) {
        let normalized = match normalize_favorite_object(entry) {
            Some(value) => value,
            None => continue,
        };
        let key = favorite_object_cmp_key(&normalized);
        if !object_seen.insert(key) {
            continue;
        }
        normalized_objects.push(normalized);
    }
    normalized_objects.sort_by(|left, right| {
        left.name
            .to_ascii_uppercase()
            .cmp(&right.name.to_ascii_uppercase())
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| {
                left.uri
                    .to_ascii_uppercase()
                    .cmp(&right.uri.to_ascii_uppercase())
            })
            .then_with(|| left.uri.cmp(&right.uri))
    });
    state.favorite_objects = normalized_objects;
}

fn normalize_server_store(store: &mut AdtServerStore) {
    neuro_server_domain::normalize_server_registry(store);

    let mut normalized_explorer_state = BTreeMap::<String, StoredAdtExplorerState>::new();
    for (raw_server_id, mut state) in std::mem::take(&mut store.explorer_state_by_server) {
        let Some(server_id) = normalize_optional_field(Some(raw_server_id)) else {
            continue;
        };
        normalize_explorer_state(&mut state);
        normalized_explorer_state.entry(server_id).or_insert(state);
    }
    store.explorer_state_by_server = normalized_explorer_state;
}

pub(crate) fn load_server_store() -> Result<AdtServerStore, NeuroRuntimeError> {
    let mut parsed = neuro_server_store_fs::load_server_store().map_err(map_neuro_adt_error)?;
    normalize_server_store(&mut parsed);
    Ok(parsed)
}

pub(crate) fn save_server_store(store: &AdtServerStore) -> Result<(), NeuroRuntimeError> {
    let mut normalized_store = store.clone();
    normalize_server_store(&mut normalized_store);
    neuro_server_store_fs::save_server_store(&normalized_store).map_err(map_neuro_adt_error)
}
#[cfg(test)]
fn selected_server_id(store: &AdtServerStore) -> Option<String> {
    neuro_server_domain::selected_server_id(store)
}

fn cache_key_for_server(server_id: &str) -> String {
    format!("server:{server_id}")
}

fn resolve_server_selection(
    requested_server_id: Option<&str>,
) -> Result<(Option<StoredAdtServer>, ServerSelection), NeuroRuntimeError> {
    let mut store = load_server_store()?;
    normalize_server_store(&mut store);

    let requested_server_id = requested_server_id
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(server_id) = requested_server_id {
        let server = store
            .servers
            .iter()
            .find(|entry| entry.id == server_id)
            .cloned()
            .ok_or_else(|| {
                runtime_error(
                    NeuroRuntimeErrorCode::InvalidArgument,
                    format!("ADT server `{server_id}` is not configured"),
                    None,
                )
            })?;
        return Ok((
            Some(server.clone()),
            ServerSelection {
                cache_key: cache_key_for_server(server.id.as_str()),
                server_id: Some(server.id.clone()),
                server_name: Some(server.name.clone()),
                source: "command.server_id",
            },
        ));
    }

    if let Some(server) = store.servers.iter().find(|entry| entry.active).cloned() {
        return Ok((
            Some(server.clone()),
            ServerSelection {
                cache_key: cache_key_for_server(server.id.as_str()),
                server_id: Some(server.id.clone()),
                server_name: Some(server.name.clone()),
                source: "server_store.active",
            },
        ));
    }

    Ok((
        None,
        ServerSelection {
            cache_key: "env_default".to_string(),
            server_id: None,
            server_name: None,
            source: "env",
        },
    ))
}

fn resolve_runtime_init_target(
    requested_server_id: Option<&str>,
) -> Result<RuntimeInitTarget, NeuroRuntimeError> {
    let (_, selection) = resolve_server_selection(requested_server_id)?;
    Ok(RuntimeInitTarget {
        cache_key: selection.cache_key,
        resolved_server_id: selection.server_id,
    })
}

fn server_env_value(value: &Option<String>) -> Option<EnvValue> {
    value.as_ref().map(|entry| EnvValue {
        value: entry.clone(),
        source: "server_store",
    })
}

fn server_required_env_value(value: &str) -> EnvValue {
    EnvValue {
        value: value.to_string(),
        source: "server_store",
    }
}

fn parse_env_bool_with_default(
    keys: &[&'static str],
    default: bool,
) -> Result<(bool, Option<&'static str>), NeuroRuntimeError> {
    match first_non_empty_env(keys) {
        Some(entry) => {
            let parsed = parse_env_bool(&entry.value).ok_or_else(|| {
                runtime_error(
                    NeuroRuntimeErrorCode::InvalidArgument,
                    format!(
                        "invalid boolean value for {}: expected true/false style value",
                        entry.source
                    ),
                    None,
                )
            })?;
            Ok((parsed, Some(entry.source)))
        }
        None => Ok((default, None)),
    }
}

fn parse_env_u64_with_default(
    keys: &[&'static str],
    default: u64,
) -> Result<(u64, Option<&'static str>), NeuroRuntimeError> {
    match first_non_empty_env(keys) {
        Some(entry) => {
            let parsed = parse_env_u64(&entry.value).ok_or_else(|| {
                runtime_error(
                    NeuroRuntimeErrorCode::InvalidArgument,
                    format!(
                        "invalid integer value for {}: expected unsigned integer",
                        entry.source
                    ),
                    None,
                )
            })?;
            Ok((parsed, Some(entry.source)))
        }
        None => Ok((default, None)),
    }
}

fn parse_csv_env(keys: &[&'static str]) -> Option<(Vec<String>, &'static str)> {
    first_non_empty_env(keys).map(|entry| (parse_csv(&entry.value), entry.source))
}

fn env_or_default(keys: &[&'static str], default: &'static str) -> EnvValue {
    first_non_empty_env(keys).unwrap_or(EnvValue {
        value: default.to_string(),
        source: "default",
    })
}

fn collect_ws_headers_from_prefix(prefix: &str) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    for (name, value) in env::vars() {
        if let Some(raw_header) = name.strip_prefix(prefix) {
            let header_name = raw_header.trim().to_ascii_lowercase().replace('_', "-");
            let trimmed_value = value.trim();
            if !header_name.is_empty() && !trimmed_value.is_empty() {
                headers.insert(header_name, trimmed_value.to_string());
            }
        }
    }
    headers
}

fn resolve_runtime_config(
    requested_server_id: Option<&str>,
) -> Result<RuntimeConfig, NeuroRuntimeError> {
    let (server_override, server_selection) = resolve_server_selection(requested_server_id)?;

    let (sap_insecure_tls, sap_insecure_tls_source) =
        parse_env_bool_with_default(&["NEURO_SAP_INSECURE", "SAP_INSECURE"], false)?;
    let (sap_timeout_secs, sap_timeout_source) = parse_env_u64_with_default(
        &["NEURO_SAP_TIMEOUT_SECS", "SAP_TIMEOUT_SECS"],
        DEFAULT_ADT_TIMEOUT_SECS,
    )?;

    let sap_url = server_override
        .as_ref()
        .map(|server| server_required_env_value(server.base_url.as_str()))
        .or_else(|| first_non_empty_env(&["NEURO_SAP_URL", "SAP_URL"]));
    let (sap_user, sap_password) = if let Some(server) = server_override.as_ref() {
        // Prevent credential mixing when a server store entry is selected.
        (
            server_env_value(&server.username),
            server_env_value(&server.password),
        )
    } else {
        (
            first_non_empty_env(&["NEURO_SAP_USER", "SAP_USER", "SAP_USERNAME"]),
            first_non_empty_env(&["NEURO_SAP_PASSWORD", "SAP_PASSWORD", "SAP_PASS"]),
        )
    };
    let sap_client = server_override
        .as_ref()
        .and_then(|server| server_env_value(&server.client))
        .or_else(|| first_non_empty_env(&["NEURO_SAP_CLIENT", "SAP_CLIENT"]));
    let sap_language = server_override
        .as_ref()
        .and_then(|server| server_env_value(&server.language))
        .or_else(|| first_non_empty_env(&["NEURO_SAP_LANGUAGE", "SAP_LANGUAGE", "SAP_LANG"]));
    let (sap_insecure_tls, sap_insecure_tls_source) = if let Some(server) = &server_override {
        (!server.verify_tls, Some("server_store"))
    } else {
        (sap_insecure_tls, sap_insecure_tls_source)
    };

    let sap = SapConfig {
        url: sap_url,
        user: sap_user,
        password: sap_password,
        client: sap_client,
        language: sap_language,
        insecure_tls: sap_insecure_tls,
        insecure_tls_source: sap_insecure_tls_source,
        timeout_secs: sap_timeout_secs,
        timeout_source: sap_timeout_source,
        csrf_fetch_path: env_or_default(
            &["NEURO_ADT_CSRF_FETCH_PATH", "SAP_ADT_CSRF_FETCH_PATH"],
            DEFAULT_ADT_CSRF_FETCH_PATH,
        ),
        search_objects_path: env_or_default(
            &["NEURO_ADT_SEARCH_PATH", "SAP_ADT_SEARCH_PATH"],
            DEFAULT_ADT_SEARCH_PATH,
        ),
    };

    let (ws_timeout_secs, ws_timeout_source) = parse_env_u64_with_default(
        &["NEURO_WS_TIMEOUT_SECS", "SAP_WS_TIMEOUT_SECS"],
        DEFAULT_WS_TIMEOUT_SECS,
    )?;

    let mut ws_headers = collect_ws_headers_from_prefix("SAP_WS_HEADER_");
    ws_headers.extend(collect_ws_headers_from_prefix("NEURO_WS_HEADER_"));

    let ws = WsConfig {
        url: first_non_empty_env(&["NEURO_WS_URL", "SAP_WS_URL"]),
        timeout_secs: ws_timeout_secs,
        timeout_source: ws_timeout_source,
        headers: ws_headers,
    };

    let (read_only, read_only_source) =
        parse_env_bool_with_default(&["NEURO_SAFETY_READ_ONLY"], false)?;
    let (require_etag_for_updates, require_etag_for_updates_source) =
        parse_env_bool_with_default(&["NEURO_UPDATE_REQUIRE_ETAG"], false)?;

    let (blocked_source_patterns, blocked_source_patterns_source) =
        match parse_csv_env(&["NEURO_SAFETY_BLOCKED_PATTERNS"]) {
            Some((patterns, source)) => (patterns, Some(source)),
            None => (Vec::new(), None),
        };

    let (allowed_ws_domains, allowed_ws_domains_source) =
        match parse_csv_env(&["NEURO_SAFETY_ALLOWED_WS_DOMAINS"]) {
            Some((domains, source)) => (domains, Some(source)),
            None => (Vec::new(), None),
        };

    let safety = SafetyConfig {
        read_only,
        read_only_source,
        blocked_source_patterns,
        blocked_source_patterns_source,
        allowed_ws_domains,
        allowed_ws_domains_source,
        require_etag_for_updates,
        require_etag_for_updates_source,
    };

    Ok(RuntimeConfig {
        sap,
        ws,
        safety,
        server_selection,
    })
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn metadata_for(runtime: &NeuroRuntime, cached_runtime: bool) -> BTreeMap<String, Value> {
    let mut metadata = BTreeMap::new();
    metadata.insert("cachedRuntime".to_string(), json!(cached_runtime));
    metadata.insert(
        "serverSelectionSource".to_string(),
        json!(runtime.config.server_selection.source),
    );
    metadata.insert(
        "serverCacheKey".to_string(),
        json!(runtime.config.server_selection.cache_key.clone()),
    );
    metadata.insert(
        "serverId".to_string(),
        json!(runtime.config.server_selection.server_id.clone()),
    );
    metadata.insert(
        "serverName".to_string(),
        json!(runtime.config.server_selection.server_name.clone()),
    );
    metadata.insert("sapReady".to_string(), json!(runtime.config.sap.is_ready()));
    metadata.insert(
        "sapUrlSource".to_string(),
        json!(source(&runtime.config.sap.url)),
    );
    metadata.insert(
        "sapUserSource".to_string(),
        json!(source(&runtime.config.sap.user)),
    );
    metadata.insert(
        "sapPasswordSource".to_string(),
        json!(source(&runtime.config.sap.password)),
    );
    metadata.insert(
        "sapClient".to_string(),
        json!(value(&runtime.config.sap.client)),
    );
    metadata.insert(
        "sapClientSource".to_string(),
        json!(source(&runtime.config.sap.client)),
    );
    metadata.insert(
        "sapLanguage".to_string(),
        json!(value(&runtime.config.sap.language)),
    );
    metadata.insert(
        "sapLanguageSource".to_string(),
        json!(source(&runtime.config.sap.language)),
    );
    metadata.insert(
        "sapInsecureTls".to_string(),
        json!(runtime.config.sap.insecure_tls),
    );
    metadata.insert(
        "sapInsecureTlsSource".to_string(),
        json!(runtime.config.sap.insecure_tls_source.map(str::to_string)),
    );
    metadata.insert(
        "sapTimeoutSecs".to_string(),
        json!(runtime.config.sap.timeout_secs),
    );
    metadata.insert(
        "sapTimeoutSource".to_string(),
        json!(runtime.config.sap.timeout_source.map(str::to_string)),
    );
    metadata.insert(
        "adtCsrfFetchPath".to_string(),
        json!(runtime.config.sap.csrf_fetch_path.value.clone()),
    );
    metadata.insert(
        "adtCsrfFetchPathSource".to_string(),
        json!(runtime.config.sap.csrf_fetch_path.source),
    );
    metadata.insert(
        "adtSearchPath".to_string(),
        json!(runtime.config.sap.search_objects_path.value.clone()),
    );
    metadata.insert(
        "adtSearchPathSource".to_string(),
        json!(runtime.config.sap.search_objects_path.source),
    );
    metadata.insert(
        "wsConfigured".to_string(),
        json!(runtime.config.ws.url.is_some()),
    );
    metadata.insert(
        "wsUrlSource".to_string(),
        json!(source(&runtime.config.ws.url)),
    );
    metadata.insert(
        "wsTimeoutSecs".to_string(),
        json!(runtime.config.ws.timeout_secs),
    );
    metadata.insert(
        "wsTimeoutSource".to_string(),
        json!(runtime.config.ws.timeout_source.map(str::to_string)),
    );
    metadata.insert(
        "wsHeadersCount".to_string(),
        json!(runtime.config.ws.headers.len()),
    );
    metadata.insert(
        "safetyReadOnly".to_string(),
        json!(runtime.config.safety.read_only),
    );
    metadata.insert(
        "safetyReadOnlySource".to_string(),
        json!(runtime.config.safety.read_only_source.map(str::to_string)),
    );
    metadata.insert(
        "safetyBlockedPatterns".to_string(),
        json!(runtime.config.safety.blocked_source_patterns),
    );
    metadata.insert(
        "safetyBlockedPatternsSource".to_string(),
        json!(runtime
            .config
            .safety
            .blocked_source_patterns_source
            .map(str::to_string)),
    );
    metadata.insert(
        "safetyAllowedWsDomains".to_string(),
        json!(runtime.config.safety.allowed_ws_domains),
    );
    metadata.insert(
        "safetyAllowedWsDomainsSource".to_string(),
        json!(runtime
            .config
            .safety
            .allowed_ws_domains_source
            .map(str::to_string)),
    );
    metadata.insert(
        "safetyRequireEtagForUpdates".to_string(),
        json!(runtime.config.safety.require_etag_for_updates),
    );
    metadata.insert(
        "safetyRequireEtagForUpdatesSource".to_string(),
        json!(runtime
            .config
            .safety
            .require_etag_for_updates_source
            .map(str::to_string)),
    );
    metadata.insert(
        "sapAuthMode".to_string(),
        json!(
            if runtime.config.sap.user.is_some() && runtime.config.sap.password.is_some() {
                "basic"
            } else {
                "anonymous"
            }
        ),
    );
    metadata
}

fn source(entry: &Option<EnvValue>) -> Option<String> {
    entry.as_ref().map(|value| value.source.to_string())
}

fn value(entry: &Option<EnvValue>) -> Option<String> {
    entry.as_ref().map(|value| value.value.clone())
}

async fn success_response(
    runtime: Arc<NeuroRuntime>,
    cached_runtime: bool,
) -> RuntimeDiagnoseResponse {
    let mut response = runtime.engine.diagnose().await;

    let sap_component = if runtime.config.sap.is_ready() {
        RuntimeDiagnoseComponent {
            component: "sap_env_config".to_string(),
            status: DiagnoseStatus::Healthy,
            detail: "Required SAP runtime configuration is available".to_string(),
            latency_ms: None,
        }
    } else {
        RuntimeDiagnoseComponent {
            component: "sap_env_config".to_string(),
            status: DiagnoseStatus::Degraded,
            detail: "Missing one or more required SAP runtime settings".to_string(),
            latency_ms: None,
        }
    };

    let ws_env_component = if runtime.config.ws.url.is_some() {
        RuntimeDiagnoseComponent {
            component: "ws_env_config".to_string(),
            status: DiagnoseStatus::Healthy,
            detail: "WebSocket environment configuration detected".to_string(),
            latency_ms: None,
        }
    } else {
        RuntimeDiagnoseComponent {
            component: "ws_env_config".to_string(),
            status: DiagnoseStatus::Degraded,
            detail:
                "WebSocket environment URL was not configured (optional realtime features disabled)"
                    .to_string(),
            latency_ms: None,
        }
    };

    if sap_component.status > response.overall_status {
        response.overall_status = sap_component.status;
    }
    if ws_env_component.status > response.overall_status {
        response.overall_status = ws_env_component.status;
    }

    response.components.push(sap_component);
    response.components.push(ws_env_component);
    response
        .metadata
        .extend(metadata_for(runtime.as_ref(), cached_runtime));
    response
}

fn init_error_response(error: NeuroRuntimeError) -> RuntimeDiagnoseResponse {
    let mut metadata = BTreeMap::new();
    metadata.insert("cachedRuntime".to_string(), json!(false));
    metadata.insert("errorCode".to_string(), json!(error.code));
    metadata.insert("errorMessage".to_string(), json!(error.message.clone()));
    metadata.insert("errorDetails".to_string(), json!(error.details));

    RuntimeDiagnoseResponse {
        timestamp_epoch_secs: now_epoch_secs(),
        overall_status: DiagnoseStatus::Unavailable,
        components: vec![RuntimeDiagnoseComponent {
            component: "runtime_init".to_string(),
            status: DiagnoseStatus::Unavailable,
            detail: error.message,
            latency_ms: None,
        }],
        metadata,
    }
}

pub(crate) fn runtime_error(
    code: NeuroRuntimeErrorCode,
    message: String,
    details: Option<Value>,
) -> NeuroRuntimeError {
    NeuroRuntimeError {
        code,
        message,
        details,
    }
}

fn map_neuro_adt_error(error: NeuroAdtError) -> NeuroRuntimeError {
    runtime_error(error.runtime_code(), error.message, error.details)
}

fn map_runtime_error_to_adt(error: NeuroRuntimeError) -> NeuroAdtError {
    NeuroAdtError::from_runtime_error(error)
}

fn map_adt_error(error: AdtClientError) -> NeuroRuntimeError {
    match error {
        AdtClientError::MissingCsrfToken => runtime_error(
            NeuroRuntimeErrorCode::AdtCsrfError,
            "missing CSRF token from ADT response".to_string(),
            None,
        ),
        AdtClientError::UnexpectedStatus {
            operation,
            status,
            body,
        } => {
            let code = match status.as_u16() {
                401 | 403 => NeuroRuntimeErrorCode::AdtAuthError,
                _ => NeuroRuntimeErrorCode::AdtHttpError,
            };
            runtime_error(
                code,
                format!("ADT operation `{operation}` failed with status {status}"),
                Some(json!({
                    "operation": operation,
                    "statusCode": status.as_u16(),
                    "body": body,
                })),
            )
        }
        _ => runtime_error(NeuroRuntimeErrorCode::AdtHttpError, error.to_string(), None),
    }
}

fn map_ws_error(error: NeuroWsClientError) -> NeuroRuntimeError {
    match error {
        NeuroWsClientError::Timeout { timeout_secs } => runtime_error(
            NeuroRuntimeErrorCode::WsTimeout,
            format!("websocket request timed out after {timeout_secs} seconds"),
            Some(json!({ "timeoutSecs": timeout_secs })),
        ),
        other => runtime_error(
            NeuroRuntimeErrorCode::WsUnavailable,
            other.to_string(),
            None,
        ),
    }
}

fn map_engine_error(error: NeuroEngineError) -> NeuroRuntimeError {
    match error {
        NeuroEngineError::Adt(error) => map_adt_error(error),
        NeuroEngineError::Ws(error) => map_ws_error(error),
        NeuroEngineError::WsUnavailable => runtime_error(
            NeuroRuntimeErrorCode::WsUnavailable,
            "WebSocket client is not configured".to_string(),
            None,
        ),
        NeuroEngineError::SafetyViolation(message) => {
            runtime_error(NeuroRuntimeErrorCode::SafetyViolation, message, None)
        }
    }
}

fn map_mcp_error(error: NeuroMcpError) -> NeuroRuntimeError {
    match error {
        NeuroMcpError::UnknownTool(tool) => runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            format!("unsupported neuro MCP tool: {tool}"),
            None,
        ),
        NeuroMcpError::UnsupportedTool { tool } => runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            format!("neuro MCP tool is declared but not implemented: {tool}"),
            None,
        ),
        NeuroMcpError::InvalidArguments { tool, message } => runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            format!("invalid arguments for neuro MCP tool `{tool}`: {message}"),
            None,
        ),
        NeuroMcpError::Engine(error) => map_engine_error(error),
        NeuroMcpError::Serialize(error) => runtime_error(
            NeuroRuntimeErrorCode::Unknown,
            format!("failed to serialize neuro MCP response: {error}"),
            None,
        ),
    }
}

pub(crate) fn normalize_optional_server_id(server_id: Option<String>) -> Option<String> {
    server_id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(crate) async fn clear_runtime_cache(state: &AppState) {
    let _init_gate = state.neuro_runtime_init_gate.lock().await;
    let mut cache = state.neuro_runtime_cache.lock().await;
    cache.clear();
}

pub async fn get_or_init(
    state: &AppState,
    server_id: Option<&str>,
) -> Result<Arc<NeuroRuntime>, NeuroRuntimeError> {
    let started_at = Instant::now();
    let first_target = resolve_runtime_init_target(server_id)?;

    if let Some(runtime) = {
        let cache = state.neuro_runtime_cache.lock().await;
        cache.get(first_target.cache_key.as_str()).cloned()
    } {
        emit_neuro_runtime_init_telemetry("cache_hit", started_at.elapsed(), None);
        return Ok(runtime);
    }

    let _init_gate = state.neuro_runtime_init_gate.lock().await;
    let init_target = resolve_runtime_init_target(server_id)?;
    let cache_key = init_target.cache_key;
    let resolved_server_id = init_target.resolved_server_id;

    if let Some(runtime) = {
        let cache = state.neuro_runtime_cache.lock().await;
        cache.get(cache_key.as_str()).cloned()
    } {
        emit_neuro_runtime_init_telemetry("cache_hit_after_gate", started_at.elapsed(), None);
        return Ok(runtime);
    }

    let initialized = match NeuroRuntime::initialize(resolved_server_id.as_deref()).await {
        Ok(runtime) => Arc::new(runtime),
        Err(error) => {
            emit_neuro_runtime_init_telemetry("failed", started_at.elapsed(), Some(&error));
            return Err(error);
        }
    };

    let mut cache = state.neuro_runtime_cache.lock().await;
    cache.insert(cache_key, Arc::clone(&initialized));
    emit_neuro_runtime_init_telemetry("initialized", started_at.elapsed(), None);
    Ok(initialized)
}

pub async fn neuro_runtime_diagnose_impl(
    state: State<'_, AppState>,
) -> Result<RuntimeDiagnoseResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_runtime_diagnose", async {
        Ok(neuro_runtime_diagnose_for_app_state(state.inner()).await)
    })
    .await
}

pub async fn neuro_runtime_diagnose_for_app_state(state: &AppState) -> RuntimeDiagnoseResponse {
    if let Ok((_, selection)) = resolve_server_selection(None) {
        if let Some(runtime) = {
            let cache = state.neuro_runtime_cache.lock().await;
            cache.get(selection.cache_key.as_str()).cloned()
        } {
            return success_response(runtime, true).await;
        }
    }

    match get_or_init(state, None).await {
        Ok(runtime) => success_response(runtime, false).await,
        Err(error) => init_error_response(error),
    }
}

#[cfg(test)]
fn upsert_server(
    store: &mut AdtServerStore,
    request: AdtServerUpsertRequest,
) -> Result<StoredAdtServer, NeuroRuntimeError> {
    neuro_server_domain::upsert_server(
        store,
        AdtServerUpsertInput {
            id: request.id,
            name: request.name,
            base_url: request.base_url,
            client: request.client,
            language: request.language,
            username: request.username,
            password: request.password,
            verify_tls: request.verify_tls,
            active: request.active,
        },
    )
    .map_err(map_neuro_adt_error)
}

#[cfg(test)]
fn select_server(store: &mut AdtServerStore, server_id: &str) -> Result<(), NeuroRuntimeError> {
    neuro_server_domain::select_server(store, server_id).map_err(map_neuro_adt_error)
}

#[cfg(test)]
fn remove_server(store: &mut AdtServerStore, server_id: &str) -> bool {
    neuro_server_domain::remove_server(store, server_id)
}
fn resolve_explorer_state_server_id(
    requested_server_id: Option<String>,
) -> Result<String, NeuroRuntimeError> {
    let (_, selection) =
        resolve_server_selection(normalize_optional_server_id(requested_server_id).as_deref())?;
    Ok(selection
        .server_id
        .unwrap_or_else(|| ENV_DEFAULT_SERVER_ID.to_string()))
}

fn explorer_state_response(
    server_id: String,
    state: &StoredAdtExplorerState,
) -> AdtExplorerStateResponse {
    let selected_work_package = state.selected_work_package.clone();
    AdtExplorerStateResponse {
        server_id,
        selected_work_package: selected_work_package.clone(),
        working_package: selected_work_package,
        package_scope_roots: state.package_scope_roots.clone(),
        focused_object_uri: state.focused_object_uri.clone(),
        favorite_packages: state.favorite_packages.clone(),
        favorite_objects: state.favorite_objects.clone(),
    }
}

fn normalize_package_name(value: String) -> Option<String> {
    normalize_optional_field(Some(value))
}

fn normalize_namespace_name(value: String) -> Option<String> {
    normalize_optional_field(Some(value))
}

fn toggle_favorite_package_entry(
    state: &mut StoredAdtExplorerState,
    kind: AdtFavoritePackageKind,
    raw_name: String,
) -> Result<(), NeuroRuntimeError> {
    let Some(name) = normalize_optional_field(Some(raw_name)) else {
        return Err(runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            "favorite package name must not be empty".to_string(),
            None,
        ));
    };

    let index = state
        .favorite_packages
        .iter()
        .position(|entry| entry.kind == kind && entry.name.eq_ignore_ascii_case(name.as_str()));
    if let Some(index) = index {
        state.favorite_packages.remove(index);
    } else {
        state
            .favorite_packages
            .push(AdtFavoritePackage { kind, name });
    }
    normalize_explorer_state(state);
    Ok(())
}

fn toggle_favorite_object_entry(
    state: &mut StoredAdtExplorerState,
    entry: AdtFavoriteObject,
) -> Result<(), NeuroRuntimeError> {
    let normalized = normalize_favorite_object(entry).ok_or_else(|| {
        runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            "favorite object requires uri and name".to_string(),
            None,
        )
    })?;

    let index = state
        .favorite_objects
        .iter()
        .position(|existing| existing.uri.eq_ignore_ascii_case(normalized.uri.as_str()));
    if let Some(index) = index {
        state.favorite_objects.remove(index);
    } else {
        state.favorite_objects.push(normalized);
    }
    normalize_explorer_state(state);
    Ok(())
}

fn apply_explorer_state_patch(
    state: &mut StoredAdtExplorerState,
    request: &AdtExplorerStatePatchRequest,
) -> Result<(), NeuroRuntimeError> {
    if let Some(raw_namespace) = request.toggle_favorite_namespace.clone() {
        let Some(namespace) = normalize_namespace_name(raw_namespace) else {
            return Err(runtime_error(
                NeuroRuntimeErrorCode::InvalidArgument,
                "toggle_favorite_namespace must not be empty".to_string(),
                None,
            ));
        };
        toggle_favorite_package_entry(state, AdtFavoritePackageKind::Namespace, namespace)?;
    }

    if let Some(toggle) = request.toggle_favorite_package.clone() {
        match toggle {
            AdtFavoritePackageToggleRequest::Name(package_name) => {
                let Some(normalized_name) = normalize_package_name(package_name) else {
                    return Err(runtime_error(
                        NeuroRuntimeErrorCode::InvalidArgument,
                        "toggle_favorite_package must not be empty".to_string(),
                        None,
                    ));
                };
                toggle_favorite_package_entry(
                    state,
                    AdtFavoritePackageKind::Package,
                    normalized_name,
                )?;
            }
            AdtFavoritePackageToggleRequest::LegacyItem { name, kind } => {
                let Some(normalized_name) = normalize_optional_field(Some(name)) else {
                    return Err(runtime_error(
                        NeuroRuntimeErrorCode::InvalidArgument,
                        "toggle_favorite_package.name must not be empty".to_string(),
                        None,
                    ));
                };
                let resolved_kind = kind.unwrap_or(AdtFavoritePackageKind::Package);
                toggle_favorite_package_entry(state, resolved_kind, normalized_name)?;
            }
        }
    }

    if let Some(favorite_object) = request.toggle_favorite_object.clone() {
        toggle_favorite_object_entry(state, favorite_object)?;
    }

    if let Some(selected_work_package) = request.requested_work_package() {
        state.selected_work_package =
            selected_work_package.and_then(|value| normalize_optional_field(Some(value)));
        normalize_explorer_state(state);
    }

    if let Some(package_scope_roots) = request.package_scope_roots.clone() {
        state.package_scope_roots = normalize_package_scope_roots(package_scope_roots);
    }

    if let Some(Some(focused_object_uri)) = request.focused_object_uri.clone() {
        state.focused_object_uri = normalize_optional_field(Some(focused_object_uri));
        normalize_explorer_state(state);
    }

    Ok(())
}

fn object_matches_package_filter(
    object: &neuro_types::AdtObjectSummary,
    package_filter: Option<&str>,
) -> bool {
    match package_filter {
        Some(filter) => object
            .package
            .as_deref()
            .map(str::trim)
            .is_some_and(|package| package.eq_ignore_ascii_case(filter)),
        None => true,
    }
}

fn object_namespace(object: &neuro_types::AdtObjectSummary) -> Option<String> {
    extract_abap_namespace(object.name.as_str())
        .or_else(|| object.package.as_deref().and_then(extract_abap_namespace))
}

fn object_matches_namespace_filter(
    object: &neuro_types::AdtObjectSummary,
    namespace_filter: Option<&str>,
) -> bool {
    match namespace_filter {
        Some(filter) => object_namespace(object)
            .as_deref()
            .is_some_and(|namespace| namespace.eq_ignore_ascii_case(filter)),
        None => true,
    }
}

fn filter_objects_for_scope(
    mut objects: Vec<neuro_types::AdtObjectSummary>,
    scope: AdtListObjectsResponseScope,
    namespace_filter: Option<&str>,
    package_filter: Option<&str>,
    max_results: Option<u32>,
) -> Vec<neuro_types::AdtObjectSummary> {
    objects.retain(|object| {
        if !object_matches_package_filter(object, package_filter) {
            return false;
        }
        if !object_matches_namespace_filter(object, namespace_filter) {
            return false;
        }

        match scope {
            AdtListObjectsResponseScope::LocalObjects => object_namespace(object).is_none(),
            AdtListObjectsResponseScope::FavoritePackages
            | AdtListObjectsResponseScope::FavoriteObjects => true,
            AdtListObjectsResponseScope::SystemLibrary => object_namespace(object).is_some(),
        }
    });

    truncate_objects(objects, max_results)
}

fn search_query_for_scope(
    scope: AdtListObjectsResponseScope,
    namespace_filter: Option<&str>,
    package_filter: Option<&str>,
) -> String {
    match scope {
        AdtListObjectsResponseScope::LocalObjects => package_filter
            .unwrap_or(DEFAULT_PACKAGE_DISCOVERY_QUERY)
            .to_string(),
        AdtListObjectsResponseScope::FavoritePackages => namespace_filter
            .or(package_filter)
            .unwrap_or(DEFAULT_PACKAGE_DISCOVERY_QUERY)
            .to_string(),
        AdtListObjectsResponseScope::FavoriteObjects => DEFAULT_PACKAGE_DISCOVERY_QUERY.to_string(),
        AdtListObjectsResponseScope::SystemLibrary => namespace_filter
            .unwrap_or(DEFAULT_NAMESPACE_DISCOVERY_QUERY)
            .to_string(),
    }
}

fn truncate_objects(
    mut objects: Vec<neuro_types::AdtObjectSummary>,
    max_results: Option<u32>,
) -> Vec<neuro_types::AdtObjectSummary> {
    if let Some(limit) = max_results.and_then(|value| usize::try_from(value).ok()) {
        objects.truncate(limit);
    }
    objects
}

fn truncate_namespaces(
    mut namespaces: Vec<AdtNamespaceSummary>,
    max_results: Option<u32>,
) -> Vec<AdtNamespaceSummary> {
    if let Some(limit) = max_results.and_then(|value| usize::try_from(value).ok()) {
        namespaces.truncate(limit);
    }
    namespaces
}

fn resolve_favorite_package_filters(
    package_kind: Option<AdtFavoritePackageKind>,
    namespace_filter: Option<String>,
    package_filter: Option<String>,
) -> (Option<String>, Option<String>) {
    match package_kind {
        Some(AdtFavoritePackageKind::Namespace) => (namespace_filter.or(package_filter), None),
        Some(AdtFavoritePackageKind::Package) => (None, package_filter.or(namespace_filter)),
        None => (namespace_filter, package_filter),
    }
}

fn favorite_objects_to_summaries(
    state: &StoredAdtExplorerState,
    max_results: Option<u32>,
) -> Vec<neuro_types::AdtObjectSummary> {
    let objects = state
        .favorite_objects
        .iter()
        .map(|entry| neuro_types::AdtObjectSummary {
            uri: entry.uri.clone(),
            name: entry.name.clone(),
            object_type: entry.object_type.clone(),
            package: entry.package.clone(),
        })
        .collect();
    truncate_objects(objects, max_results)
}

fn namespace_summaries_from_objects(
    objects: Vec<neuro_types::AdtObjectSummary>,
    package_filter: Option<&str>,
) -> Vec<AdtNamespaceSummary> {
    let package_filter = package_filter.map(|entry| entry.to_ascii_uppercase());
    let mut namespaces = BTreeMap::<String, Option<String>>::new();

    for object in objects {
        let object_package = normalize_optional_field(object.package);
        if let Some(filter) = package_filter.as_ref() {
            let package_mismatch = object_package
                .as_ref()
                .map(|entry| entry.to_ascii_uppercase() != *filter)
                .unwrap_or(true);
            if package_mismatch {
                continue;
            }
        }

        let namespace = extract_abap_namespace(object.name.as_str())
            .or_else(|| object_package.as_deref().and_then(extract_abap_namespace));
        if let Some(namespace) = namespace {
            namespaces
                .entry(namespace)
                .or_insert(object_package.clone());
        }
    }

    namespaces
        .into_iter()
        .map(|(name, package_name)| AdtNamespaceSummary { name, package_name })
        .collect()
}

fn parse_abap_namespaces_from_table_raw(raw: &str) -> Vec<String> {
    fn is_namespace_char(ch: char) -> bool {
        ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'
    }

    let chars: Vec<char> = raw.chars().collect();
    let mut namespaces = HashSet::<String>::new();
    let mut index = 0usize;

    while index < chars.len() {
        if chars[index] != '/' {
            index += 1;
            continue;
        }

        let mut cursor = index + 1;
        while cursor < chars.len() && is_namespace_char(chars[cursor]) {
            cursor += 1;
        }

        if cursor > index + 1 && cursor < chars.len() && chars[cursor] == '/' {
            let inner = chars[index + 1..cursor].iter().collect::<String>();
            if inner.chars().any(|entry| entry.is_ascii_alphabetic()) {
                namespaces.insert(format!("/{inner}/"));
            }
            index = cursor + 1;
            continue;
        }

        index += 1;
    }

    let mut sorted = namespaces.into_iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.to_ascii_uppercase()
            .cmp(&right.to_ascii_uppercase())
            .then_with(|| left.cmp(right))
    });
    sorted
}

async fn list_namespaces_from_ddic_table(
    runtime: &NeuroRuntime,
    table_name: &str,
) -> Result<Vec<AdtNamespaceSummary>, NeuroRuntimeError> {
    let normalized_table_name = table_name.trim().to_ascii_uppercase();
    if normalized_table_name.is_empty() {
        return Ok(Vec::new());
    }

    let endpoint = format!(
        "/sap/bc/adt/datapreview/ddic?rowNumber={}&ddicEntityName={}",
        DEFAULT_DISCOVERY_MAX_RESULTS.max(1),
        normalized_table_name
    );
    let raw = runtime
        .engine
        .post_raw_text(endpoint.as_str(), None, None, Some("application/*"))
        .await
        .map_err(map_engine_error)?;
    Ok(parse_abap_namespaces_from_table_raw(raw.as_str())
        .into_iter()
        .map(|name| AdtNamespaceSummary {
            name,
            package_name: None,
        })
        .collect())
}

fn extract_abap_namespace(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    let mut slash_indices = trimmed.match_indices('/').map(|(index, _)| index);
    let _leading = slash_indices.next()?;
    let second = slash_indices.next()?;
    Some(trimmed[..=second].to_string())
}

pub async fn neuro_search_objects_impl(
    state: State<'_, AppState>,
    query: String,
    max_results: Option<u32>,
    server_id: Option<String>,
) -> Result<Vec<neuro_types::AdtObjectSummary>, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_search_objects", async {
        let runtime = get_or_init(
            state.inner(),
            normalize_optional_server_id(server_id).as_deref(),
        )
        .await?;
        runtime
            .engine
            .search(query.as_str(), max_results)
            .await
            .map_err(map_engine_error)
    })
    .await
}

pub async fn neuro_get_source_impl(
    state: State<'_, AppState>,
    object_uri: String,
    server_id: Option<String>,
) -> Result<neuro_types::AdtSourceResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_get_source", async {
        let runtime = get_or_init(
            state.inner(),
            normalize_optional_server_id(server_id).as_deref(),
        )
        .await?;
        runtime
            .engine
            .get_source(object_uri.as_str())
            .await
            .map_err(map_engine_error)
    })
    .await
}

pub async fn neuro_update_source_impl(
    state: State<'_, AppState>,
    request: AdtUpdateSourceCommandRequest,
) -> Result<neuro_types::AdtUpdateSourceResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_update_source", async {
        let server_id = normalize_optional_server_id(request.server_id.clone());
        let runtime = get_or_init(state.inner(), server_id.as_deref()).await?;
        runtime
            .engine
            .update_source(request.into_update_request())
            .await
            .map_err(map_engine_error)
    })
    .await
}

struct NeuroRuntimeNeuroAdtPort;

impl NeuroAdtPort for NeuroRuntimeNeuroAdtPort {
    fn load_server_store(&self) -> Result<AdtServerStore, NeuroAdtError> {
        load_server_store().map_err(map_runtime_error_to_adt)
    }

    fn save_server_store(&self, store: &AdtServerStore) -> Result<(), NeuroAdtError> {
        save_server_store(store).map_err(map_runtime_error_to_adt)
    }

    fn normalize_optional_server_id(&self, server_id: Option<String>) -> Option<String> {
        normalize_optional_server_id(server_id)
    }

    fn env_default_server_id(&self) -> &'static str {
        ENV_DEFAULT_SERVER_ID
    }

    fn clear_runtime_cache<'a>(&'a self, state: &'a AppState) -> NeuroAdtFuture<'a, ()> {
        Box::pin(async move {
            clear_runtime_cache(state).await;
        })
    }

    fn connect_server<'a>(
        &'a self,
        state: &'a AppState,
        server_id: Option<&'a str>,
    ) -> NeuroAdtFuture<'a, Result<AdtServerConnectivity, NeuroAdtError>> {
        Box::pin(async move {
            let runtime = get_or_init(state, server_id)
                .await
                .map_err(map_runtime_error_to_adt)?;
            let (connected, message) = runtime.adt_http_connectivity().await;
            Ok(AdtServerConnectivity {
                selected_server_id: runtime.selected_server_id(),
                connected,
                message,
            })
        })
    }
}

pub async fn neuro_adt_server_list_impl(
    _state: State<'_, AppState>,
) -> Result<AdtServerListResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_server_list", async {
        let port = NeuroRuntimeNeuroAdtPort;
        neuro_adt_use_cases::neuro_adt_server_list(&port).map_err(map_neuro_adt_error)
    })
    .await
}

pub async fn neuro_adt_server_upsert_impl(
    state: State<'_, AppState>,
    request: AdtServerUpsertRequest,
) -> Result<AdtServerRecord, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_server_upsert", async {
        let port = NeuroRuntimeNeuroAdtPort;
        neuro_adt_use_cases::neuro_adt_server_upsert(state.inner(), request, &port)
            .await
            .map_err(map_neuro_adt_error)
    })
    .await
}

pub async fn neuro_adt_server_remove_impl(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<AdtServerRemoveResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_server_remove", async {
        let port = NeuroRuntimeNeuroAdtPort;
        neuro_adt_use_cases::neuro_adt_server_remove(state.inner(), server_id, &port)
            .await
            .map_err(map_neuro_adt_error)
    })
    .await
}

pub async fn neuro_adt_server_select_impl(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<AdtServerSelectResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_server_select", async {
        let port = NeuroRuntimeNeuroAdtPort;
        neuro_adt_use_cases::neuro_adt_server_select(state.inner(), server_id, &port)
            .await
            .map_err(map_neuro_adt_error)
    })
    .await
}

pub async fn neuro_adt_server_connect_impl(
    state: State<'_, AppState>,
    server_id: Option<String>,
) -> Result<AdtServerConnectResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_server_connect", async {
        let port = NeuroRuntimeNeuroAdtPort;
        neuro_adt_use_cases::neuro_adt_server_connect(state.inner(), server_id, &port)
            .await
            .map_err(map_neuro_adt_error)
    })
    .await
}

pub async fn neuro_adt_list_packages_impl(
    state: State<'_, AppState>,
    server_id: Option<String>,
) -> Result<Vec<AdtPackageSummary>, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_list_packages", async {
        let normalized_server_id = normalize_optional_server_id(server_id);
        let runtime = get_or_init(state.inner(), normalized_server_id.as_deref()).await?;
        let objects = runtime
            .engine
            .search(
                DEFAULT_PACKAGE_DISCOVERY_QUERY,
                Some(DEFAULT_DISCOVERY_MAX_RESULTS),
            )
            .await
            .map_err(map_engine_error)?;

        let mut package_names = HashSet::<String>::new();
        for object in objects {
            if let Some(package) = normalize_optional_field(object.package) {
                package_names.insert(package);
            }
        }

        let mut packages = package_names
            .into_iter()
            .map(|name| AdtPackageSummary {
                name,
                description: None,
            })
            .collect::<Vec<_>>();
        packages.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(packages)
    })
    .await
}

pub async fn neuro_adt_list_namespaces_impl(
    state: State<'_, AppState>,
    package_name: Option<String>,
    server_id: Option<String>,
) -> Result<Vec<AdtNamespaceSummary>, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_list_namespaces", async {
        let normalized_server_id = normalize_optional_server_id(server_id);
        let normalized_package = normalize_optional_field(package_name);
        let query = normalized_package
            .as_deref()
            .unwrap_or(DEFAULT_NAMESPACE_DISCOVERY_QUERY);

        let runtime = get_or_init(state.inner(), normalized_server_id.as_deref()).await?;
        if normalized_package.is_none() {
            let mut seen = HashSet::<String>::new();
            let mut merged = Vec::<AdtNamespaceSummary>::new();
            for table_name in ["TRNSPACET", "TRNSPACE"] {
                if let Ok(entries) = list_namespaces_from_ddic_table(&runtime, table_name).await {
                    for entry in entries {
                        if seen.insert(entry.name.to_ascii_uppercase()) {
                            merged.push(entry);
                        }
                    }
                }
            }
            if !merged.is_empty() {
                merged.sort_by(|left, right| {
                    left.name
                        .to_ascii_uppercase()
                        .cmp(&right.name.to_ascii_uppercase())
                        .then_with(|| left.name.cmp(&right.name))
                });
                return Ok(merged);
            }
        }

        let objects = runtime
            .engine
            .search(query, Some(DEFAULT_DISCOVERY_MAX_RESULTS))
            .await
            .map_err(map_engine_error)?;
        Ok(namespace_summaries_from_objects(
            objects,
            normalized_package.as_deref(),
        ))
    })
    .await
}

pub async fn neuro_adt_list_package_inventory_impl(
    state: State<'_, AppState>,
    request: AdtPackageInventoryRequest,
) -> Result<AdtPackageInventoryResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_list_package_inventory", async {
        let include_subpackages = request.include_subpackages.unwrap_or(true);
        let include_objects = request.include_objects.unwrap_or(false);
        let max_packages = normalize_inventory_limit(
            request.max_packages,
            DEFAULT_DISCOVERY_MAX_RESULTS,
            "max_packages",
            MAX_PACKAGE_INVENTORY_MAX_PACKAGES,
        )?;
        let max_objects_per_package = normalize_inventory_limit(
            request.max_objects_per_package,
            DEFAULT_PACKAGE_INVENTORY_MAX_OBJECTS_PER_PACKAGE,
            "max_objects_per_package",
            MAX_PACKAGE_INVENTORY_MAX_OBJECTS_PER_PACKAGE,
        )?;
        let roots = normalize_inventory_roots(request.roots)?;

        let runtime = get_or_init(
            state.inner(),
            normalize_optional_server_id(request.server_id).as_deref(),
        )
        .await?;

        let root_discovery =
            discover_root_packages_for_inventory(&runtime, &roots, max_packages).await?;
        let discovered_packages = collect_inventory_packages(
            &runtime,
            root_discovery.packages,
            include_subpackages,
            max_packages,
        )
        .await?;

        let mut packages = Vec::<AdtPackageInventoryNode>::with_capacity(discovered_packages.len());
        let mut objects_by_package = if include_objects {
            Vec::<AdtPackageInventoryPackageObjects>::with_capacity(discovered_packages.len())
        } else {
            Vec::<AdtPackageInventoryPackageObjects>::new()
        };
        let mut packages_with_truncated_objects = 0u32;

        for package in discovered_packages {
            let package_name = package.name.clone();
            let object_count = if include_objects {
                let objects_result = list_source_objects_for_package_inventory(
                    &runtime,
                    package_name.as_str(),
                    max_objects_per_package,
                )
                .await?;
                if objects_result.is_truncated {
                    packages_with_truncated_objects =
                        packages_with_truncated_objects.saturating_add(1);
                }
                let count = u32::try_from(objects_result.objects.len()).unwrap_or(u32::MAX);
                objects_by_package.push(AdtPackageInventoryPackageObjects {
                    package_name: package_name.clone(),
                    objects: objects_result.objects,
                });
                count
            } else {
                0
            };

            packages.push(AdtPackageInventoryNode {
                name: package_name.clone(),
                parent_name: package.parent_name,
                depth: package.depth,
                is_root: package.is_root,
                object_count,
            });
        }

        let returned_packages = u32::try_from(packages.len()).unwrap_or(u32::MAX);
        let root_discovery_truncated = root_discovery.root_discovery_truncated;
        let max_packages_reached = root_discovery.max_packages_reached;
        let object_results_truncated = packages_with_truncated_objects > 0;
        let is_truncated =
            root_discovery_truncated || max_packages_reached || object_results_truncated;
        let metadata = AdtPackageInventoryMetadata {
            is_complete: !is_truncated,
            is_truncated,
            include_objects,
            max_packages_reached,
            root_discovery_truncated,
            object_results_truncated,
            max_packages,
            max_objects_per_package,
            returned_packages,
            packages_with_truncated_objects,
            roots: root_discovery.roots,
        };

        Ok(AdtPackageInventoryResponse {
            roots: roots.into_iter().map(|root| root.response_value).collect(),
            packages,
            objects_by_package,
            metadata: Some(metadata),
        })
    })
    .await
}

fn normalize_inventory_limit(
    value: Option<u32>,
    default_value: u32,
    field_name: &str,
    max_value: u32,
) -> Result<u32, NeuroRuntimeError> {
    match value {
        Some(0) => Err(runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            format!("{field_name} must be greater than zero"),
            None,
        )),
        Some(limit) if limit > max_value => Err(runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            format!("{field_name} must be less than or equal to {max_value}"),
            None,
        )),
        Some(limit) => Ok(limit),
        None => Ok(default_value),
    }
}

fn normalize_inventory_roots(
    roots: Vec<String>,
) -> Result<Vec<AdtPackageRootSpec>, NeuroRuntimeError> {
    let mut normalized = Vec::<AdtPackageRootSpec>::new();
    let mut seen = HashSet::<String>::new();

    for raw_root in roots {
        let Some(root) = parse_inventory_root(raw_root.as_str()) else {
            continue;
        };

        if seen.insert(root.dedupe_key.clone()) {
            normalized.push(root);
        }
    }

    if normalized.is_empty() {
        return Err(runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            "roots must contain at least one package, namespace, or pattern".to_string(),
            None,
        ));
    }

    Ok(normalized)
}

fn parse_inventory_root(raw_root: &str) -> Option<AdtPackageRootSpec> {
    let trimmed = raw_root.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (kind, query) = if let Some((prefix, raw_value)) = trimmed.split_once(':') {
        let normalized_prefix = prefix.trim().to_ascii_lowercase();
        if let Some(kind) = parse_inventory_root_kind(normalized_prefix.as_str()) {
            (kind, raw_value.trim())
        } else {
            (infer_inventory_root_kind(trimmed), trimmed)
        }
    } else {
        (infer_inventory_root_kind(trimmed), trimmed)
    };

    let query = match kind {
        AdtPackageRootKind::Package => normalize_inventory_package_name(query)?,
        AdtPackageRootKind::Namespace => normalize_inventory_namespace_name(query)?,
        AdtPackageRootKind::Pattern => normalize_inventory_pattern(query)?,
    };

    let response_value = query.clone();

    Some(AdtPackageRootSpec {
        kind,
        query: query.clone(),
        response_value,
        dedupe_key: format!(
            "{}:{}",
            inventory_root_kind_label(kind),
            query.to_ascii_uppercase()
        ),
    })
}

fn parse_inventory_root_kind(value: &str) -> Option<AdtPackageRootKind> {
    match value {
        "package" | "pkg" => Some(AdtPackageRootKind::Package),
        "namespace" | "ns" => Some(AdtPackageRootKind::Namespace),
        "pattern" | "query" => Some(AdtPackageRootKind::Pattern),
        _ => None,
    }
}

fn infer_inventory_root_kind(value: &str) -> AdtPackageRootKind {
    let trimmed = value.trim();
    if trimmed.contains('*') || trimmed.contains('?') {
        AdtPackageRootKind::Pattern
    } else if trimmed.starts_with('/') || trimmed.ends_with('/') {
        AdtPackageRootKind::Namespace
    } else {
        AdtPackageRootKind::Package
    }
}

fn inventory_root_kind_label(kind: AdtPackageRootKind) -> &'static str {
    match kind {
        AdtPackageRootKind::Package => "package",
        AdtPackageRootKind::Namespace => "namespace",
        AdtPackageRootKind::Pattern => "pattern",
    }
}

fn root_allows_subpackage_recursion(kind: AdtPackageRootKind) -> bool {
    kind == AdtPackageRootKind::Package
}

fn normalize_inventory_package_name(value: &str) -> Option<String> {
    normalize_optional_field(Some(value.to_ascii_uppercase()))
}

fn normalize_inventory_namespace_name(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.contains('*') || trimmed.contains('?') {
        return None;
    }

    let inner = trimmed.trim_matches('/');
    let normalized = normalize_optional_field(Some(inner.to_ascii_uppercase()))?;
    Some(format!("/{normalized}/"))
}

fn normalize_inventory_pattern(value: &str) -> Option<String> {
    normalize_optional_field(Some(value.to_ascii_uppercase()))
}

fn package_matches_inventory_root(package_name: &str, root: &AdtPackageRootSpec) -> bool {
    let normalized_package = package_name.trim().to_ascii_uppercase();
    if normalized_package.is_empty() {
        return false;
    }

    match root.kind {
        AdtPackageRootKind::Package => normalized_package.eq_ignore_ascii_case(root.query.as_str()),
        AdtPackageRootKind::Namespace => normalized_package.starts_with(root.query.as_str()),
        AdtPackageRootKind::Pattern => {
            wildcard_match(normalized_package.as_str(), root.query.as_str())
        }
    }
}

fn wildcard_match(value: &str, pattern: &str) -> bool {
    let value = value.as_bytes();
    let pattern = pattern.as_bytes();
    let mut value_index = 0usize;
    let mut pattern_index = 0usize;
    let mut last_star_index = None::<usize>;
    let mut last_star_value_index = 0usize;

    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == value[value_index])
        {
            value_index += 1;
            pattern_index += 1;
            continue;
        }

        if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            last_star_index = Some(pattern_index);
            pattern_index += 1;
            last_star_value_index = value_index;
            continue;
        }

        if let Some(star_index) = last_star_index {
            pattern_index = star_index + 1;
            last_star_value_index += 1;
            value_index = last_star_value_index;
            continue;
        }

        return false;
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

fn is_package_object_type(object_type: &str) -> bool {
    matches!(
        object_type.trim().to_ascii_uppercase().as_str(),
        "DEVC/K" | "DEVC" | "PACKAGE"
    )
}

fn is_source_object_type_for_inventory(object_type: &str) -> bool {
    matches!(
        object_type.trim().to_ascii_uppercase().as_str(),
        "PROG/P"
            | "PROG/I"
            | "CLAS/OC"
            | "INTF/OI"
            | "FUGR/FF"
            | "DDLS/DF"
            | "TABL/DT"
            | "STRU/DS"
            | "BDEF/BDO"
            | "SRVD/SRV"
            | "SRVB/SVB"
    )
}

fn package_name_from_search_result(object: neuro_types::AdtObjectSummary) -> Option<String> {
    normalize_optional_field(Some(object.name)).or_else(|| {
        object
            .package
            .and_then(|package| normalize_optional_field(Some(package)))
    })
}

fn cap_discovery_search_limit(requested_limit: u32) -> u32 {
    requested_limit.clamp(1, DEFAULT_DISCOVERY_MAX_RESULTS)
}

async fn discover_root_packages_for_inventory(
    runtime: &NeuroRuntime,
    roots: &[AdtPackageRootSpec],
    max_packages: u32,
) -> Result<AdtPackageInventoryRootDiscoverySummary, NeuroRuntimeError> {
    let mut discovered = Vec::<AdtPackageInventoryRootSeed>::new();
    let mut discovered_indexes = HashMap::<String, usize>::new();
    let mut roots_metadata = Vec::<AdtPackageInventoryRootMetadata>::with_capacity(roots.len());
    let max_packages = usize::try_from(max_packages).unwrap_or(usize::MAX);
    let mut max_packages_reached = false;
    let mut root_discovery_truncated = false;

    for (index, root) in roots.iter().enumerate() {
        if discovered.len() >= max_packages {
            max_packages_reached = true;
            root_discovery_truncated = true;
            roots_metadata.push(skipped_root_metadata(root));
            for skipped_root in roots.iter().skip(index + 1) {
                roots_metadata.push(skipped_root_metadata(skipped_root));
            }
            break;
        }

        let AdtPackageInventoryRootDiscovery {
            mut packages,
            queries_executed,
            result_limit_hit,
            mut is_complete,
        } = discover_packages_for_inventory_root(runtime, root).await?;

        let matched_packages = u32::try_from(packages.len()).unwrap_or(u32::MAX);
        let mut returned_packages = 0u32;

        for package_name in packages.drain(..) {
            let normalized = package_name.to_ascii_uppercase();
            if normalized.is_empty() {
                continue;
            }

            let recurse_subpackages = root_allows_subpackage_recursion(root.kind);
            if let Some(existing_index) = discovered_indexes.get(normalized.as_str()) {
                if recurse_subpackages {
                    discovered[*existing_index].recurse_subpackages = true;
                }
                continue;
            }

            let discovered_index = discovered.len();
            discovered_indexes.insert(normalized.clone(), discovered_index);
            discovered.push(AdtPackageInventoryRootSeed {
                name: normalized,
                recurse_subpackages,
            });
            returned_packages = returned_packages.saturating_add(1);
            if discovered.len() >= max_packages {
                max_packages_reached = true;
                root_discovery_truncated = true;
                is_complete = false;
                break;
            }
        }

        let root_metadata = AdtPackageInventoryRootMetadata {
            root: root.response_value.clone(),
            kind: inventory_root_kind_label(root.kind).to_string(),
            queries_executed,
            matched_packages,
            returned_packages,
            result_limit_hit,
            is_complete,
            skipped_due_to_max_packages: false,
        };
        root_discovery_truncated |= !root_metadata.is_complete;
        roots_metadata.push(root_metadata);

        if max_packages_reached {
            for skipped_root in roots.iter().skip(index + 1) {
                roots_metadata.push(skipped_root_metadata(skipped_root));
            }
            break;
        }
    }

    Ok(AdtPackageInventoryRootDiscoverySummary {
        packages: discovered,
        roots: roots_metadata,
        max_packages_reached,
        root_discovery_truncated,
    })
}

fn skipped_root_metadata(root: &AdtPackageRootSpec) -> AdtPackageInventoryRootMetadata {
    AdtPackageInventoryRootMetadata {
        root: root.response_value.clone(),
        kind: inventory_root_kind_label(root.kind).to_string(),
        queries_executed: 0,
        matched_packages: 0,
        returned_packages: 0,
        result_limit_hit: false,
        is_complete: false,
        skipped_due_to_max_packages: true,
    }
}

async fn discover_packages_for_inventory_root(
    runtime: &NeuroRuntime,
    root: &AdtPackageRootSpec,
) -> Result<AdtPackageInventoryRootDiscovery, NeuroRuntimeError> {
    if root.kind == AdtPackageRootKind::Package {
        return Ok(AdtPackageInventoryRootDiscovery {
            packages: vec![root.query.clone()],
            queries_executed: 0,
            result_limit_hit: false,
            is_complete: true,
        });
    }

    let search_limit = cap_discovery_search_limit(DEFAULT_DISCOVERY_MAX_RESULTS);
    let mut packages = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let mut queries_executed = 0u32;
    let mut result_limit_hit = false;

    let base_query = root_inventory_discovery_query(root);
    let (base_packages, base_limit_hit) =
        discover_packages_for_inventory_query(runtime, root, base_query.as_str(), search_limit)
            .await?;
    queries_executed = queries_executed.saturating_add(1);
    result_limit_hit |= base_limit_hit;
    for package_name in base_packages {
        if seen.insert(package_name.clone()) {
            packages.push(package_name);
        }
    }

    let mut chunk_budget_exhausted = false;
    if base_limit_hit {
        if let Some(chunk_prefix) = inventory_chunk_prefix_for_root(root) {
            for chunk_query in inventory_chunk_queries_from_prefix(chunk_prefix.as_str()) {
                if usize::try_from(queries_executed).unwrap_or(usize::MAX)
                    >= MAX_PACKAGE_INVENTORY_DISCOVERY_QUERIES_PER_ROOT
                {
                    chunk_budget_exhausted = true;
                    break;
                }

                let (chunk_packages, chunk_limit_hit) = discover_packages_for_inventory_query(
                    runtime,
                    root,
                    chunk_query.as_str(),
                    search_limit,
                )
                .await?;
                queries_executed = queries_executed.saturating_add(1);
                result_limit_hit |= chunk_limit_hit;

                for package_name in chunk_packages {
                    if seen.insert(package_name.clone()) {
                        packages.push(package_name);
                    }
                }
            }
        }
    }

    packages.sort_by(|left, right| {
        left.to_ascii_uppercase()
            .cmp(&right.to_ascii_uppercase())
            .then_with(|| left.cmp(right))
    });

    let is_complete = !result_limit_hit && !chunk_budget_exhausted;
    Ok(AdtPackageInventoryRootDiscovery {
        packages,
        queries_executed,
        result_limit_hit,
        is_complete,
    })
}

fn root_inventory_discovery_query(root: &AdtPackageRootSpec) -> String {
    match root.kind {
        AdtPackageRootKind::Namespace => format!("{}*", root.query),
        _ => root.query.clone(),
    }
}

fn inventory_chunk_prefix_for_root(root: &AdtPackageRootSpec) -> Option<String> {
    match root.kind {
        AdtPackageRootKind::Namespace => Some(root.query.clone()),
        AdtPackageRootKind::Pattern => inventory_chunk_prefix_for_pattern(root.query.as_str()),
        AdtPackageRootKind::Package => None,
    }
}

fn inventory_chunk_prefix_for_pattern(pattern: &str) -> Option<String> {
    if pattern.contains('?') {
        return None;
    }

    let wildcard_count = pattern.chars().filter(|ch| *ch == '*').count();
    if wildcard_count != 1 || !pattern.ends_with('*') {
        return None;
    }

    let prefix = pattern.trim_end_matches('*');
    if prefix.trim().is_empty() {
        return None;
    }

    Some(prefix.to_string())
}

fn inventory_chunk_queries_from_prefix(prefix: &str) -> Vec<String> {
    PACKAGE_INVENTORY_DISCOVERY_BUCKETS
        .chars()
        .map(|bucket| format!("{prefix}{bucket}*"))
        .collect()
}

async fn discover_packages_for_inventory_query(
    runtime: &NeuroRuntime,
    root: &AdtPackageRootSpec,
    query: &str,
    search_limit: u32,
) -> Result<(Vec<String>, bool), NeuroRuntimeError> {
    let objects = runtime
        .engine
        .search(query, Some(search_limit))
        .await
        .map_err(map_engine_error)?;
    let is_limit_hit = usize::try_from(search_limit)
        .ok()
        .is_some_and(|limit| objects.len() >= limit);

    let mut discovered = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();

    for object in objects {
        let Some(object_type) = object.object_type.as_deref() else {
            continue;
        };
        if !is_package_object_type(object_type) {
            continue;
        }

        let Some(package_name) = package_name_from_search_result(object) else {
            continue;
        };

        let normalized_package = package_name.to_ascii_uppercase();
        if !package_matches_inventory_root(normalized_package.as_str(), root) {
            continue;
        }

        if seen.insert(normalized_package.clone()) {
            discovered.push(normalized_package);
        }
    }

    discovered.sort_by(|left, right| {
        left.to_ascii_uppercase()
            .cmp(&right.to_ascii_uppercase())
            .then_with(|| left.cmp(right))
    });

    Ok((discovered, is_limit_hit))
}

async fn collect_inventory_packages(
    runtime: &NeuroRuntime,
    root_packages: Vec<AdtPackageInventoryRootSeed>,
    include_subpackages: bool,
    max_packages: u32,
) -> Result<Vec<AdtPackageInventoryVisit>, NeuroRuntimeError> {
    let max_packages = usize::try_from(max_packages).unwrap_or(usize::MAX);
    let mut queue = VecDeque::<AdtPackageInventoryVisit>::new();
    let mut visited = HashSet::<String>::new();

    for root_package in root_packages {
        let Some(normalized_root) = normalize_inventory_package_name(root_package.name.as_str())
        else {
            continue;
        };
        if visited.insert(normalized_root.clone()) {
            queue.push_back(AdtPackageInventoryVisit {
                name: normalized_root,
                parent_name: None,
                depth: 0,
                is_root: true,
                recurse_subpackages: root_package.recurse_subpackages,
            });
        }
    }

    let mut discovered = Vec::<AdtPackageInventoryVisit>::new();
    while let Some(current) = queue.pop_front() {
        if discovered.len() >= max_packages {
            break;
        }

        let current_name = current.name.clone();
        let current_depth = current.depth;
        let recurse_subpackages = current.recurse_subpackages;
        discovered.push(current);

        if !include_subpackages || !recurse_subpackages {
            continue;
        }

        let subpackages = fetch_subpackages_for_package(runtime, current_name.as_str()).await?;
        for subpackage in subpackages {
            if visited.insert(subpackage.clone()) {
                queue.push_back(AdtPackageInventoryVisit {
                    name: subpackage,
                    parent_name: Some(current_name.clone()),
                    depth: current_depth.saturating_add(1),
                    is_root: false,
                    recurse_subpackages,
                });
            }
        }
    }

    Ok(discovered)
}

async fn fetch_subpackages_for_package(
    runtime: &NeuroRuntime,
    package_name: &str,
) -> Result<Vec<String>, NeuroRuntimeError> {
    let endpoint = format!(
        "/sap/bc/adt/repository/nodestructure?parent_type=DEVC%2FK&parent_name={}&withShortDescriptions=true",
        encode_query_component(package_name)
    );
    let raw = runtime
        .engine
        .post_raw_text(endpoint.as_str(), None, None, Some("application/*"))
        .await
        .map_err(map_engine_error)?;
    Ok(parse_nodestructure_subpackages(raw.as_str(), package_name))
}

fn encode_query_component(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0F) as usize] as char);
        }
    }
    encoded
}

fn parse_nodestructure_subpackages(raw: &str, parent_package_name: &str) -> Vec<String> {
    let normalized_parent = parent_package_name.trim().to_ascii_uppercase();
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();

    for reference in parse_nodestructure_references(raw) {
        let Some(object_type) = reference.object_type.as_deref() else {
            continue;
        };
        if !is_package_object_type(object_type) {
            continue;
        }

        let Some(parent_name) = reference.parent_name.as_deref() else {
            continue;
        };
        if !parent_name.eq_ignore_ascii_case(normalized_parent.as_str()) {
            continue;
        }

        let child_name = reference.name.trim().to_ascii_uppercase();
        if child_name.is_empty() || child_name == normalized_parent {
            continue;
        }

        if seen.insert(child_name.clone()) {
            out.push(child_name);
        }
    }

    out.sort_by(|left, right| {
        left.to_ascii_uppercase()
            .cmp(&right.to_ascii_uppercase())
            .then_with(|| left.cmp(right))
    });
    out
}

fn parse_nodestructure_references(raw: &str) -> Vec<AdtNodeStructureRef> {
    let mut references = Vec::<AdtNodeStructureRef>::new();
    let mut cursor = 0usize;

    while let Some(tag_start_relative) = raw[cursor..].find('<') {
        let tag_start = cursor + tag_start_relative;
        let Some(tag_end_relative) = raw[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end_relative;
        let fragment = &raw[tag_start..=tag_end];
        cursor = tag_end.saturating_add(1);

        if !fragment.contains("objectReference") {
            continue;
        }

        let Some(name) = extract_first_xml_attribute(fragment, &["adtcore:name", "name"])
            .and_then(|value| normalize_optional_field(Some(value)))
        else {
            continue;
        };

        let object_type = extract_first_xml_attribute(fragment, &["adtcore:type", "type"])
            .and_then(|value| normalize_optional_field(Some(value)));

        let parent_name = extract_first_xml_attribute(
            fragment,
            &["adtcore:parentName", "parentName", "parent_name"],
        )
        .and_then(|value| normalize_optional_field(Some(value)))
        .or_else(|| {
            extract_first_xml_attribute(
                fragment,
                &["adtcore:parentUri", "parentUri", "parent_uri", "parent-uri"],
            )
            .and_then(|uri| package_name_from_uri(uri.as_str()))
        });

        references.push(AdtNodeStructureRef {
            name,
            object_type,
            parent_name,
        });
    }

    references
}

fn extract_first_xml_attribute(fragment: &str, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| extract_xml_attribute(fragment, name))
}

fn extract_xml_attribute(fragment: &str, name: &str) -> Option<String> {
    let mut search_start = 0usize;
    while let Some(relative_index) = fragment[search_start..].find(name) {
        let index = search_start + relative_index;
        search_start = index + name.len();

        let bytes = fragment.as_bytes();
        if index > 0 {
            let previous = bytes[index - 1] as char;
            if !(previous.is_ascii_whitespace() || previous == '<') {
                continue;
            }
        }

        let mut cursor = index + name.len();
        while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || (bytes[cursor] != b'"' && bytes[cursor] != b'\'') {
            continue;
        }

        let quote = bytes[cursor];
        cursor += 1;
        let value_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != quote {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            continue;
        }

        let value = &fragment[value_start..cursor];
        if value.trim().is_empty() {
            continue;
        }
        return Some(value.to_string());
    }

    None
}

fn package_name_from_uri(uri: &str) -> Option<String> {
    let path = uri.split('?').next().unwrap_or(uri);
    let last_segment = path
        .trim()
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap_or_default();
    let decoded_segment = decode_percent_component(last_segment);
    normalize_optional_field(Some(decoded_segment.to_ascii_uppercase()))
}

fn decode_percent_component(value: &str) -> String {
    fn hex_nibble(value: u8) -> Option<u8> {
        match value {
            b'0'..=b'9' => Some(value - b'0'),
            b'a'..=b'f' => Some(value - b'a' + 10),
            b'A'..=b'F' => Some(value - b'A' + 10),
            _ => None,
        }
    }

    let bytes = value.as_bytes();
    let mut decoded = Vec::<u8>::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = hex_nibble(bytes[index + 1]);
            let low = hex_nibble(bytes[index + 2]);
            if let (Some(high), Some(low)) = (high, low) {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(decoded.as_slice()).into_owned()
}

async fn list_source_objects_for_package_inventory(
    runtime: &NeuroRuntime,
    package_name: &str,
    max_objects_per_package: u32,
) -> Result<AdtPackageInventoryObjectsResult, NeuroRuntimeError> {
    let search_limit = cap_discovery_search_limit(DEFAULT_DISCOVERY_MAX_RESULTS);
    let raw_objects = runtime
        .engine
        .search(package_name, Some(search_limit))
        .await
        .map_err(map_engine_error)?;
    let search_limit_hit = usize::try_from(search_limit)
        .ok()
        .is_some_and(|limit| raw_objects.len() >= limit);

    let mut objects = raw_objects
        .into_iter()
        .filter(|object| {
            object_matches_package_filter(object, Some(package_name))
                && object
                    .object_type
                    .as_deref()
                    .is_some_and(is_source_object_type_for_inventory)
        })
        .collect::<Vec<_>>();

    objects.sort_by(|left, right| {
        left.name
            .to_ascii_uppercase()
            .cmp(&right.name.to_ascii_uppercase())
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.uri.cmp(&right.uri))
    });

    let mut is_truncated = search_limit_hit;
    if let Ok(limit) = usize::try_from(max_objects_per_package) {
        if objects.len() > limit {
            is_truncated = true;
        }
        objects.truncate(limit);
    }

    Ok(AdtPackageInventoryObjectsResult {
        objects,
        is_truncated,
    })
}

pub async fn neuro_adt_explorer_state_get_impl(
    _state: State<'_, AppState>,
    server_id: Option<String>,
) -> Result<AdtExplorerStateResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_explorer_state_get", async {
        let resolved_server_id = resolve_explorer_state_server_id(server_id)?;
        let mut store = load_server_store()?;
        normalize_server_store(&mut store);
        let state = store
            .explorer_state_by_server
            .get(resolved_server_id.as_str())
            .cloned()
            .unwrap_or_default();
        Ok(explorer_state_response(resolved_server_id, &state))
    })
    .await
}

pub async fn neuro_adt_explorer_state_patch_impl(
    _state: State<'_, AppState>,
    request: AdtExplorerStatePatchRequest,
) -> Result<AdtExplorerStateResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_explorer_state_patch", async {
        let resolved_server_id = resolve_explorer_state_server_id(request.server_id.clone())?;
        let mut store = load_server_store()?;
        normalize_server_store(&mut store);

        {
            let state = store
                .explorer_state_by_server
                .entry(resolved_server_id.clone())
                .or_default();
            apply_explorer_state_patch(state, &request)?;
        }

        normalize_server_store(&mut store);
        save_server_store(&store)?;

        let updated = store
            .explorer_state_by_server
            .get(resolved_server_id.as_str())
            .cloned()
            .unwrap_or_default();
        Ok(explorer_state_response(resolved_server_id, &updated))
    })
    .await
}

pub async fn neuro_adt_list_objects_impl(
    state: State<'_, AppState>,
    request: AdtListObjectsRequest,
) -> Result<AdtListObjectsResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_adt_list_objects", async {
        let AdtListObjectsRequest {
            scope,
            namespace,
            package_name,
            package_kind,
            max_results,
            server_id,
        } = request;
        let response_scope = scope.response_scope();
        let normalized_server_id = normalize_optional_server_id(server_id.clone());
        let normalized_max_results = max_results.filter(|value| *value > 0);

        if response_scope == AdtListObjectsResponseScope::FavoriteObjects {
            let resolved_server_id = resolve_explorer_state_server_id(server_id)?;
            let mut store = load_server_store()?;
            normalize_server_store(&mut store);
            let explorer_state = store
                .explorer_state_by_server
                .get(resolved_server_id.as_str())
                .cloned()
                .unwrap_or_default();
            return Ok(AdtListObjectsResponse {
                scope: response_scope,
                objects: favorite_objects_to_summaries(&explorer_state, normalized_max_results),
                namespaces: None,
            });
        }

        let mut normalized_namespace = normalize_optional_field(namespace);
        let mut normalized_package_name = normalize_optional_field(package_name);

        if response_scope == AdtListObjectsResponseScope::FavoritePackages {
            (normalized_namespace, normalized_package_name) = resolve_favorite_package_filters(
                package_kind,
                normalized_namespace,
                normalized_package_name,
            );
        }

        let query = search_query_for_scope(
            response_scope,
            normalized_namespace.as_deref(),
            normalized_package_name.as_deref(),
        );

        let runtime = get_or_init(state.inner(), normalized_server_id.as_deref()).await?;
        let objects = runtime
            .engine
            .search(query.as_str(), normalized_max_results)
            .await
            .map_err(map_engine_error)?;

        let filtered_objects = filter_objects_for_scope(
            objects,
            response_scope,
            normalized_namespace.as_deref(),
            normalized_package_name.as_deref(),
            normalized_max_results,
        );

        if response_scope == AdtListObjectsResponseScope::SystemLibrary
            && normalized_namespace.is_none()
            && !scope.is_legacy_namespace()
        {
            return Ok(AdtListObjectsResponse {
                scope: response_scope,
                objects: Vec::new(),
                namespaces: Some(truncate_namespaces(
                    namespace_summaries_from_objects(
                        filtered_objects,
                        normalized_package_name.as_deref(),
                    ),
                    normalized_max_results,
                )),
            });
        }

        Ok(AdtListObjectsResponse {
            scope: response_scope,
            objects: filtered_objects,
            namespaces: None,
        })
    })
    .await
}

pub async fn neuro_ws_request_impl(
    state: State<'_, AppState>,
    request: WsDomainRequest,
) -> Result<WsMessageEnvelope<Value>, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_ws_request", async {
        let runtime = get_or_init(state.inner(), None).await?;
        runtime
            .engine
            .send_domain_request(
                request.domain.as_str(),
                request.action.as_str(),
                request.payload,
            )
            .await
            .map_err(map_engine_error)
    })
    .await
}

pub async fn neuro_list_tools_impl(
    state: State<'_, AppState>,
) -> Result<Vec<NeuroToolSpec>, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_list_tools", async {
        let runtime = get_or_init(state.inner(), None).await?;
        Ok(NeuroMcpFacade::new(runtime.engine.clone()).list_tools())
    })
    .await
}

pub async fn neuro_invoke_tool_impl(
    state: State<'_, AppState>,
    tool_name: String,
    arguments: Value,
) -> Result<Value, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_invoke_tool", async {
        let runtime = get_or_init(state.inner(), None).await?;
        NeuroMcpFacade::new(runtime.engine.clone())
            .invoke(tool_name.as_str(), arguments)
            .await
            .map_err(map_mcp_error)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        super::shared_env_lock()
    }

    fn telemetry_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn telemetry_async_lock() -> &'static tokio::sync::Mutex<()> {
        static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "alicia_backend_neuro_runtime_{label}_{}_{}",
            std::process::id(),
            now
        ))
    }

    fn take_single_telemetry_event() -> Value {
        let mut events = drain_test_telemetry_events();
        assert_eq!(events.len(), 1, "expected exactly one telemetry event");
        serde_json::from_str(&events.remove(0)).expect("telemetry payload should be valid JSON")
    }

    fn with_env_overrides(pairs: &[(&str, Option<&str>)], test: impl FnOnce()) {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let previous: Vec<(String, Option<String>)> = pairs
            .iter()
            .map(|(key, _)| ((*key).to_string(), env::var(key).ok()))
            .collect();

        for (key, value) in pairs {
            match value {
                Some(next) => env::set_var(key, next),
                None => env::remove_var(key),
            }
        }

        test();

        for (key, value) in previous {
            match value {
                Some(previous_value) => env::set_var(&key, previous_value),
                None => env::remove_var(&key),
            }
        }
    }

    #[derive(Clone)]
    struct ConnectFailurePort {
        error: NeuroRuntimeError,
    }

    impl ConnectFailurePort {
        fn new(error: NeuroRuntimeError) -> Self {
            Self { error }
        }
    }

    impl NeuroAdtPort for ConnectFailurePort {
        fn load_server_store(&self) -> Result<AdtServerStore, NeuroAdtError> {
            Ok(AdtServerStore::default())
        }

        fn save_server_store(&self, _store: &AdtServerStore) -> Result<(), NeuroAdtError> {
            Ok(())
        }

        fn normalize_optional_server_id(&self, server_id: Option<String>) -> Option<String> {
            normalize_optional_server_id(server_id)
        }

        fn env_default_server_id(&self) -> &'static str {
            ENV_DEFAULT_SERVER_ID
        }

        fn clear_runtime_cache<'a>(&'a self, _state: &'a AppState) -> NeuroAdtFuture<'a, ()> {
            Box::pin(async move {})
        }

        fn connect_server<'a>(
            &'a self,
            _state: &'a AppState,
            _server_id: Option<&'a str>,
        ) -> NeuroAdtFuture<'a, Result<AdtServerConnectivity, NeuroAdtError>> {
            let error = self.error.clone();
            Box::pin(async move { Err(map_runtime_error_to_adt(error)) })
        }
    }
    fn sample_object(
        uri: &str,
        name: &str,
        object_type: Option<&str>,
        package: Option<&str>,
    ) -> neuro_types::AdtObjectSummary {
        neuro_types::AdtObjectSummary {
            uri: uri.to_string(),
            name: name.to_string(),
            object_type: object_type.map(str::to_string),
            package: package.map(str::to_string),
        }
    }

    #[test]
    fn normalize_inventory_roots_parses_kinds_and_deduplicates() {
        let roots = normalize_inventory_roots(vec![
            " zpkg ".to_string(),
            "package:ZPKG".to_string(),
            "namespace:/abap/".to_string(),
            "/ABAP/".to_string(),
            "pattern: zcl_* ".to_string(),
            "zcl_*".to_string(),
        ])
        .expect("roots should normalize");

        assert_eq!(roots.len(), 3);
        assert_eq!(roots[0].kind, AdtPackageRootKind::Package);
        assert_eq!(roots[0].query, "ZPKG");
        assert_eq!(roots[0].response_value, "ZPKG");

        assert_eq!(roots[1].kind, AdtPackageRootKind::Namespace);
        assert_eq!(roots[1].query, "/ABAP/");
        assert_eq!(roots[1].response_value, "/ABAP/");

        assert_eq!(roots[2].kind, AdtPackageRootKind::Pattern);
        assert_eq!(roots[2].query, "ZCL_*");
        assert_eq!(roots[2].response_value, "ZCL_*");
    }

    #[test]
    fn package_inventory_request_include_objects_supports_aliases_and_defaults() {
        let default_request: AdtPackageInventoryRequest = serde_json::from_value(json!({
            "roots": ["ZPKG"]
        }))
        .expect("default request should deserialize");
        assert_eq!(default_request.include_objects, None);

        let camel_case: AdtPackageInventoryRequest = serde_json::from_value(json!({
            "roots": ["ZPKG"],
            "includeObjects": true
        }))
        .expect("camelCase includeObjects should deserialize");
        assert_eq!(camel_case.include_objects, Some(true));

        let snake_case: AdtPackageInventoryRequest = serde_json::from_value(json!({
            "roots": ["ZPKG"],
            "include_objects": false
        }))
        .expect("snake_case include_objects should deserialize");
        assert_eq!(snake_case.include_objects, Some(false));
    }

    #[test]
    fn normalize_inventory_limit_rejects_values_above_maximum() {
        let error = normalize_inventory_limit(Some(5001), 250, "max_packages", 5000)
            .expect_err("value above max should fail");
        assert_eq!(error.code, NeuroRuntimeErrorCode::InvalidArgument);
        assert!(error.message.contains("max_packages"));
    }

    #[test]
    fn normalize_inventory_limit_accepts_boundary_value() {
        let value = normalize_inventory_limit(Some(1000), 250, "max_objects_per_package", 1000)
            .expect("boundary value should be accepted");
        assert_eq!(value, 1000);
    }

    #[test]
    fn normalize_inventory_roots_requires_at_least_one_valid_root() {
        let error = normalize_inventory_roots(vec![
            "".to_string(),
            "   ".to_string(),
            "namespace: ".to_string(),
        ])
        .expect_err("empty roots should fail");

        assert_eq!(error.code, NeuroRuntimeErrorCode::InvalidArgument);
        assert!(error.message.contains("roots"));
    }

    #[test]
    fn parse_nodestructure_refs_and_subpackages_filters_by_parent() {
        let payload = r#"
            <adtcore:objectReferences xmlns:adtcore="http://www.sap.com/adt/core">
                <adtcore:objectReference
                    adtcore:name="ZCHILD_ONE"
                    adtcore:type="DEVC/K"
                    adtcore:parentUri="/sap/bc/adt/packages/ZROOT" />
                <adtcore:objectReference
                    adtcore:name="zchild_two"
                    adtcore:type="DEVC/K"
                    parentName="ZROOT" />
                <adtcore:objectReference
                    adtcore:name="ZWRONG_PARENT"
                    adtcore:type="DEVC/K"
                    adtcore:parentName="ZOTHER" />
                <adtcore:objectReference
                    adtcore:name="ZLEGACY_PACKAGE_NAME"
                    adtcore:type="DEVC/K"
                    adtcore:packageName="ZROOT" />
                <adtcore:objectReference
                    adtcore:name="ZCL_ANY"
                    adtcore:type="CLAS/OC" />
            </adtcore:objectReferences>
        "#;

        let refs = parse_nodestructure_references(payload);
        assert_eq!(refs.len(), 5);
        assert_eq!(refs[0].name, "ZCHILD_ONE");
        assert_eq!(refs[0].object_type.as_deref(), Some("DEVC/K"));
        assert_eq!(refs[0].parent_name.as_deref(), Some("ZROOT"));
        assert_eq!(refs[3].name, "ZLEGACY_PACKAGE_NAME");
        assert_eq!(refs[3].parent_name, None);

        let subpackages = parse_nodestructure_subpackages(payload, "zroot");
        assert_eq!(
            subpackages,
            vec!["ZCHILD_ONE".to_string(), "ZCHILD_TWO".to_string()]
        );
    }

    #[test]
    fn parse_nodestructure_subpackages_supports_percent_encoded_parent_uri() {
        let payload = r#"
            <adtcore:objectReferences xmlns:adtcore="http://www.sap.com/adt/core">
                <adtcore:objectReference
                    adtcore:name="ZABC_CHILD"
                    adtcore:type="DEVC/K"
                    adtcore:parentUri="/sap/bc/adt/packages/%2FABC%2F" />
            </adtcore:objectReferences>
        "#;

        let subpackages = parse_nodestructure_subpackages(payload, "/abc/");
        assert_eq!(subpackages, vec!["ZABC_CHILD".to_string()]);
    }

    #[test]
    fn cap_discovery_search_limit_preserves_low_values_and_caps_high_values() {
        assert_eq!(cap_discovery_search_limit(7), 7);
        assert_eq!(
            cap_discovery_search_limit(DEFAULT_DISCOVERY_MAX_RESULTS + 1),
            DEFAULT_DISCOVERY_MAX_RESULTS
        );
    }

    #[test]
    fn inventory_chunk_prefix_for_pattern_requires_single_trailing_wildcard() {
        assert_eq!(
            inventory_chunk_prefix_for_pattern("ZPKG*"),
            Some("ZPKG".to_string())
        );
        assert_eq!(inventory_chunk_prefix_for_pattern("ZPKG"), None);
        assert_eq!(inventory_chunk_prefix_for_pattern("Z*PKG"), None);
        assert_eq!(inventory_chunk_prefix_for_pattern("ZPKG?*"), None);
        assert_eq!(inventory_chunk_prefix_for_pattern("*"), None);
    }

    #[test]
    fn root_inventory_discovery_query_adds_wildcard_for_namespace() {
        let namespace_root = AdtPackageRootSpec {
            kind: AdtPackageRootKind::Namespace,
            query: "/ABC/".to_string(),
            response_value: "/ABC/".to_string(),
            dedupe_key: "namespace:/ABC/".to_string(),
        };
        assert_eq!(root_inventory_discovery_query(&namespace_root), "/ABC/*");

        let pattern_root = AdtPackageRootSpec {
            kind: AdtPackageRootKind::Pattern,
            query: "ZPKG*".to_string(),
            response_value: "ZPKG*".to_string(),
            dedupe_key: "pattern:ZPKG*".to_string(),
        };
        assert_eq!(root_inventory_discovery_query(&pattern_root), "ZPKG*");
    }

    #[test]
    fn root_allows_subpackage_recursion_only_for_explicit_package_roots() {
        assert!(root_allows_subpackage_recursion(
            AdtPackageRootKind::Package
        ));
        assert!(!root_allows_subpackage_recursion(
            AdtPackageRootKind::Namespace
        ));
        assert!(!root_allows_subpackage_recursion(
            AdtPackageRootKind::Pattern
        ));
    }

    #[test]
    fn inventory_chunk_queries_from_prefix_uses_expected_bucket_order() {
        let queries = inventory_chunk_queries_from_prefix("/ABC/");
        assert_eq!(queries.first().map(String::as_str), Some("/ABC/A*"));
        assert_eq!(queries.last().map(String::as_str), Some("/ABC/$*"));
        assert_eq!(queries.len(), PACKAGE_INVENTORY_DISCOVERY_BUCKETS.len());
    }

    #[test]
    fn parse_env_bool_supports_common_values() {
        assert_eq!(parse_env_bool("true"), Some(true));
        assert_eq!(parse_env_bool("YES"), Some(true));
        assert_eq!(parse_env_bool("1"), Some(true));
        assert_eq!(parse_env_bool("off"), Some(false));
        assert_eq!(parse_env_bool("0"), Some(false));
        assert_eq!(parse_env_bool("maybe"), None);
    }

    #[test]
    fn default_csrf_fetch_path_matches_contract_and_canonical_value() {
        assert_eq!(DEFAULT_ADT_CSRF_FETCH_PATH, "/sap/bc/adt/core/discovery");
        let contract_default: AdtHttpConfig = serde_json::from_value(json!({
            "base_url": "https://contract.local"
        }))
        .expect("contract default should deserialize");
        assert_eq!(
            DEFAULT_ADT_CSRF_FETCH_PATH,
            contract_default.csrf_fetch_path.as_str()
        );
    }

    #[test]
    fn resolve_runtime_config_uses_default_csrf_path_when_unset() {
        let store_path = unique_temp_path("resolve_default_csrf")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[
                (
                    NEURO_ADT_SERVER_STORE_PATH_ENV,
                    Some(store_path_text.as_str()),
                ),
                ("NEURO_SAP_URL", Some("https://env.local")),
                ("NEURO_SAP_USER", Some("env-user")),
                ("NEURO_SAP_PASSWORD", Some("env-pass")),
                ("NEURO_ADT_CSRF_FETCH_PATH", None),
                ("SAP_ADT_CSRF_FETCH_PATH", None),
            ],
            || {
                let config = resolve_runtime_config(None).expect("runtime config should resolve");
                assert_eq!(
                    config.sap.csrf_fetch_path.value,
                    DEFAULT_ADT_CSRF_FETCH_PATH
                );
                assert_eq!(config.sap.csrf_fetch_path.source, "default");
            },
        );

        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn resolve_runtime_config_prefers_neuro_env_over_fallbacks() {
        let store_path = unique_temp_path("resolve_env_prefers")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[
                (
                    NEURO_ADT_SERVER_STORE_PATH_ENV,
                    Some(store_path_text.as_str()),
                ),
                ("NEURO_SAP_URL", Some("https://neuro.local")),
                ("SAP_URL", Some("https://legacy.local")),
                ("NEURO_SAP_USER", Some("neuro-user")),
                ("SAP_USER", Some("legacy-user")),
                ("NEURO_SAP_PASSWORD", Some("neuro-pass")),
                ("SAP_PASSWORD", Some("legacy-pass")),
                ("NEURO_ADT_CSRF_FETCH_PATH", Some("/custom/neuro/csrf")),
                ("SAP_ADT_CSRF_FETCH_PATH", Some("/custom/legacy/csrf")),
                ("NEURO_WS_URL", Some("wss://neuro.ws")),
                ("SAP_WS_URL", Some("wss://legacy.ws")),
                ("NEURO_SAFETY_READ_ONLY", Some("false")),
                ("NEURO_UPDATE_REQUIRE_ETAG", Some("true")),
            ],
            || {
                let config = resolve_runtime_config(None).expect("runtime config should resolve");
                assert_eq!(
                    config.sap.url.as_ref().map(|value| value.value.as_str()),
                    Some("https://neuro.local")
                );
                assert_eq!(
                    config.sap.user.as_ref().map(|value| value.value.as_str()),
                    Some("neuro-user")
                );
                assert_eq!(
                    config
                        .sap
                        .password
                        .as_ref()
                        .map(|value| value.value.as_str()),
                    Some("neuro-pass")
                );
                assert_eq!(
                    config.ws.url.as_ref().map(|value| value.value.as_str()),
                    Some("wss://neuro.ws")
                );
                assert_eq!(config.sap.csrf_fetch_path.value, "/custom/neuro/csrf");
                assert_eq!(
                    config.sap.csrf_fetch_path.source,
                    "NEURO_ADT_CSRF_FETCH_PATH"
                );
                assert!(!config.safety.read_only);
                assert!(config.safety.require_etag_for_updates);
            },
        );

        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn resolve_runtime_config_rejects_invalid_boolean_values() {
        let store_path = unique_temp_path("resolve_invalid_bool")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();
        with_env_overrides(
            &[
                (
                    NEURO_ADT_SERVER_STORE_PATH_ENV,
                    Some(store_path_text.as_str()),
                ),
                ("NEURO_SAP_INSECURE", Some("not-bool")),
            ],
            || {
                let error = resolve_runtime_config(None).expect_err("invalid boolean must fail");
                assert_eq!(error.code, NeuroRuntimeErrorCode::InvalidArgument);
                assert!(error.message.contains("NEURO_SAP_INSECURE"));
            },
        );

        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn server_store_upsert_select_remove_roundtrip() {
        let store_path = unique_temp_path("server_store_roundtrip")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                let mut store = AdtServerStore::default();

                let first = upsert_server(
                    &mut store,
                    AdtServerUpsertRequest {
                        id: "srv_a".to_string(),
                        name: "Server A".to_string(),
                        base_url: "https://srv-a.local".to_string(),
                        client: Some("100".to_string()),
                        language: Some("EN".to_string()),
                        username: Some("alice".to_string()),
                        password: Some("secret-a".to_string()),
                        verify_tls: Some(true),
                        active: None,
                    },
                )
                .expect("first upsert should succeed");
                assert_eq!(first.id, "srv_a");
                assert!(!first.active);

                let second = upsert_server(
                    &mut store,
                    AdtServerUpsertRequest {
                        id: "srv_b".to_string(),
                        name: "Server B".to_string(),
                        base_url: "https://srv-b.local".to_string(),
                        client: Some("200".to_string()),
                        language: Some("PT".to_string()),
                        username: Some("bob".to_string()),
                        password: Some("secret-b".to_string()),
                        verify_tls: Some(false),
                        active: Some(true),
                    },
                )
                .expect("second upsert should succeed");
                assert_eq!(second.id, "srv_b");
                assert!(second.active);

                save_server_store(&store).expect("store should persist");
                let persisted = load_server_store().expect("store should load");
                assert_eq!(persisted.servers.len(), 2);
                assert_eq!(selected_server_id(&persisted).as_deref(), Some("srv_b"));

                let mut reloaded = persisted.clone();
                select_server(&mut reloaded, "srv_a").expect("select should succeed");
                assert_eq!(selected_server_id(&reloaded).as_deref(), Some("srv_a"));

                let removed = remove_server(&mut reloaded, "srv_a");
                assert!(removed);
                assert_eq!(selected_server_id(&reloaded), None);
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn explorer_state_persists_isolated_by_server_id() {
        let store_path = unique_temp_path("explorer_isolation")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                let mut store = AdtServerStore::default();
                store.explorer_state_by_server.insert(
                    "srv_a".to_string(),
                    StoredAdtExplorerState {
                        favorite_packages: vec![AdtFavoritePackage {
                            kind: AdtFavoritePackageKind::Package,
                            name: "ZPKG_A".to_string(),
                        }],
                        favorite_objects: vec![AdtFavoriteObject {
                            uri: "adt://obj/a".to_string(),
                            name: "ZCL_A".to_string(),
                            object_type: Some("CLAS".to_string()),
                            package: Some("ZPKG_A".to_string()),
                        }],
                        selected_work_package: Some("ZPKG_A".to_string()),
                        package_scope_roots: vec!["ZPKG_A".to_string()],
                        focused_object_uri: Some("adt://obj/a".to_string()),
                    },
                );
                store.explorer_state_by_server.insert(
                    "srv_b".to_string(),
                    StoredAdtExplorerState {
                        favorite_packages: vec![AdtFavoritePackage {
                            kind: AdtFavoritePackageKind::Namespace,
                            name: "/ABC/".to_string(),
                        }],
                        favorite_objects: vec![AdtFavoriteObject {
                            uri: "adt://obj/b".to_string(),
                            name: "/ABC/CL_B".to_string(),
                            object_type: Some("CLAS".to_string()),
                            package: Some("/ABC/PKG".to_string()),
                        }],
                        selected_work_package: Some("/ABC/PKG".to_string()),
                        package_scope_roots: vec!["/ABC/PKG".to_string()],
                        focused_object_uri: Some("adt://obj/b".to_string()),
                    },
                );

                save_server_store(&store).expect("store should persist");
                let reloaded = load_server_store().expect("store should reload");

                let state_a = reloaded
                    .explorer_state_by_server
                    .get("srv_a")
                    .expect("srv_a state should persist");
                assert_eq!(state_a.selected_work_package.as_deref(), Some("ZPKG_A"));
                assert_eq!(state_a.focused_object_uri.as_deref(), Some("adt://obj/a"));
                assert_eq!(state_a.favorite_packages.len(), 1);
                assert_eq!(state_a.favorite_objects.len(), 1);
                assert_eq!(state_a.favorite_packages[0].name, "ZPKG_A");
                assert_eq!(state_a.package_scope_roots, vec!["ZPKG_A"]);

                let state_b = reloaded
                    .explorer_state_by_server
                    .get("srv_b")
                    .expect("srv_b state should persist");
                assert_eq!(state_b.selected_work_package.as_deref(), Some("/ABC/PKG"));
                assert_eq!(state_b.focused_object_uri.as_deref(), Some("adt://obj/b"));
                assert_eq!(state_b.favorite_packages.len(), 1);
                assert_eq!(state_b.favorite_objects.len(), 1);
                assert_eq!(state_b.favorite_packages[0].name, "/ABC/");
                assert_eq!(state_b.package_scope_roots, vec!["/ABC/PKG"]);
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn explorer_state_toggle_favorites_adds_and_removes_entries() {
        let mut state = StoredAdtExplorerState::default();
        let patch = AdtExplorerStatePatchRequest {
            toggle_favorite_namespace: Some("/ABC/".to_string()),
            toggle_favorite_package: Some(AdtFavoritePackageToggleRequest::Name(
                "ZPKG".to_string(),
            )),
            toggle_favorite_object: Some(AdtFavoriteObject {
                uri: "adt://obj".to_string(),
                name: "ZCL_OBJECT".to_string(),
                object_type: Some("CLAS".to_string()),
                package: Some("ZPKG".to_string()),
            }),
            ..AdtExplorerStatePatchRequest::default()
        };

        apply_explorer_state_patch(&mut state, &patch).expect("add patch should succeed");
        assert_eq!(state.favorite_packages.len(), 2);
        assert_eq!(state.favorite_objects.len(), 1);

        apply_explorer_state_patch(&mut state, &patch).expect("remove patch should succeed");
        assert!(state.favorite_packages.is_empty());
        assert!(state.favorite_objects.is_empty());
    }

    #[test]
    fn explorer_state_set_work_package_is_persisted() {
        let store_path = unique_temp_path("explorer_work_package")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                let mut store = AdtServerStore::default();
                let mut state = StoredAdtExplorerState::default();
                let patch = AdtExplorerStatePatchRequest {
                    set_work_package: Some(Some("Z_WORK".to_string())),
                    ..AdtExplorerStatePatchRequest::default()
                };
                apply_explorer_state_patch(&mut state, &patch)
                    .expect("set_work_package patch should succeed");
                store
                    .explorer_state_by_server
                    .insert("srv_work".to_string(), state);
                save_server_store(&store).expect("store should persist");

                let reloaded = load_server_store().expect("store should reload");
                let persisted = reloaded
                    .explorer_state_by_server
                    .get("srv_work")
                    .expect("srv_work state should persist");
                assert_eq!(persisted.selected_work_package.as_deref(), Some("Z_WORK"));
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn explorer_state_focused_object_uri_patch_is_normalized_and_persisted() {
        let mut state = StoredAdtExplorerState::default();
        let patch = AdtExplorerStatePatchRequest {
            focused_object_uri: Some(Some("  adt://obj/focus  ".to_string())),
            ..AdtExplorerStatePatchRequest::default()
        };
        apply_explorer_state_patch(&mut state, &patch).expect("focused object patch should work");
        assert_eq!(state.focused_object_uri.as_deref(), Some("adt://obj/focus"));

        let no_change_patch = AdtExplorerStatePatchRequest {
            focused_object_uri: Some(None),
            ..AdtExplorerStatePatchRequest::default()
        };
        apply_explorer_state_patch(&mut state, &no_change_patch)
            .expect("null focused object patch should be ignored");
        assert_eq!(state.focused_object_uri.as_deref(), Some("adt://obj/focus"));
    }

    #[test]
    fn explorer_state_package_scope_roots_patch_dedupes_and_trims() {
        let mut state = StoredAdtExplorerState::default();
        let patch = AdtExplorerStatePatchRequest {
            package_scope_roots: Some(vec![
                "  ZPKG_A  ".to_string(),
                "".to_string(),
                " zpkg_a ".to_string(),
                "  /ABC/PKG  ".to_string(),
            ]),
            ..AdtExplorerStatePatchRequest::default()
        };
        apply_explorer_state_patch(&mut state, &patch)
            .expect("package scope roots patch should work");
        assert_eq!(
            state.package_scope_roots,
            vec!["ZPKG_A".to_string(), "/ABC/PKG".to_string()]
        );
    }

    #[test]
    fn explorer_state_package_scope_roots_patch_clears_with_empty_list() {
        let mut state = StoredAdtExplorerState {
            package_scope_roots: vec!["ZPKG_A".to_string(), "/ABC/PKG".to_string()],
            ..StoredAdtExplorerState::default()
        };
        let patch = AdtExplorerStatePatchRequest {
            package_scope_roots: Some(vec![]),
            ..AdtExplorerStatePatchRequest::default()
        };
        apply_explorer_state_patch(&mut state, &patch).expect("clear patch should work");
        assert!(state.package_scope_roots.is_empty());
    }

    #[test]
    fn explorer_state_package_scope_roots_patch_is_persisted() {
        let store_path = unique_temp_path("explorer_scope_roots")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                let mut store = AdtServerStore::default();
                let mut state = StoredAdtExplorerState::default();
                let patch = AdtExplorerStatePatchRequest {
                    package_scope_roots: Some(vec![
                        "  ZPKG_A ".to_string(),
                        "zpkg_a".to_string(),
                        " /ABC/PKG ".to_string(),
                    ]),
                    ..AdtExplorerStatePatchRequest::default()
                };
                apply_explorer_state_patch(&mut state, &patch)
                    .expect("scope roots patch should work");
                store
                    .explorer_state_by_server
                    .insert("srv_scope".to_string(), state);
                save_server_store(&store).expect("store should persist");

                let reloaded = load_server_store().expect("store should reload");
                let persisted = reloaded
                    .explorer_state_by_server
                    .get("srv_scope")
                    .expect("srv_scope state should persist");
                assert_eq!(
                    persisted.package_scope_roots,
                    vec!["ZPKG_A".to_string(), "/ABC/PKG".to_string()]
                );
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn explorer_state_patch_request_accepts_package_scope_roots_aliases() {
        let snake_case: AdtExplorerStatePatchRequest = serde_json::from_value(json!({
            "package_scope_roots": ["ZPKG_A"]
        }))
        .expect("snake_case alias should deserialize");
        assert_eq!(
            snake_case.package_scope_roots,
            Some(vec!["ZPKG_A".to_string()])
        );

        let camel_case: AdtExplorerStatePatchRequest = serde_json::from_value(json!({
            "packageScopeRoots": ["/ABC/PKG"]
        }))
        .expect("camelCase alias should deserialize");
        assert_eq!(
            camel_case.package_scope_roots,
            Some(vec!["/ABC/PKG".to_string()])
        );
    }

    #[test]
    fn explorer_state_response_exposes_focused_object_uri() {
        let response = explorer_state_response(
            "srv_focus".to_string(),
            &StoredAdtExplorerState {
                focused_object_uri: Some("adt://obj/focus".to_string()),
                ..StoredAdtExplorerState::default()
            },
        );
        assert_eq!(response.server_id, "srv_focus");
        assert_eq!(
            response.focused_object_uri.as_deref(),
            Some("adt://obj/focus")
        );
    }

    #[test]
    fn explorer_state_response_exposes_package_scope_roots() {
        let response = explorer_state_response(
            "srv_scope".to_string(),
            &StoredAdtExplorerState {
                package_scope_roots: vec!["ZPKG_A".to_string(), "/ABC/PKG".to_string()],
                ..StoredAdtExplorerState::default()
            },
        );
        assert_eq!(response.server_id, "srv_scope");
        assert_eq!(
            response.package_scope_roots,
            vec!["ZPKG_A".to_string(), "/ABC/PKG".to_string()]
        );
    }

    #[test]
    fn list_objects_scope_supports_current_and_legacy_values() {
        let modern: AdtListObjectsRequest = serde_json::from_value(json!({
            "scope": "system_library"
        }))
        .expect("modern scope should deserialize");
        assert_eq!(
            modern.scope.response_scope(),
            AdtListObjectsResponseScope::SystemLibrary
        );

        let legacy: AdtListObjectsRequest = serde_json::from_value(json!({
            "scope": "namespace"
        }))
        .expect("legacy scope should deserialize");
        assert_eq!(
            legacy.scope.response_scope(),
            AdtListObjectsResponseScope::SystemLibrary
        );
    }

    #[test]
    fn list_objects_scope_filters_basically() {
        let objects = vec![
            sample_object("adt://1", "ZCL_LOCAL", Some("CLAS"), Some("ZPKG")),
            sample_object("adt://2", "/ABC/CL_NS", Some("CLAS"), Some("/ABC/PKG")),
            sample_object("adt://3", "ZCL_OTHER", Some("CLAS"), Some("ZOTHER")),
            sample_object("adt://4", "/XYZ/IF_IN_PKG", Some("INTF"), Some("ZPKG")),
        ];

        let local = filter_objects_for_scope(
            objects.clone(),
            AdtListObjectsResponseScope::LocalObjects,
            None,
            None,
            None,
        );
        assert_eq!(local.len(), 2);
        assert!(local.iter().any(|entry| entry.uri == "adt://1"));
        assert!(local.iter().any(|entry| entry.uri == "adt://3"));

        let namespace = filter_objects_for_scope(
            objects.clone(),
            AdtListObjectsResponseScope::SystemLibrary,
            Some("/ABC/"),
            None,
            None,
        );
        assert_eq!(namespace.len(), 1);
        assert_eq!(namespace[0].uri, "adt://2");

        let package = filter_objects_for_scope(
            objects,
            AdtListObjectsResponseScope::FavoritePackages,
            None,
            Some("zpkg"),
            Some(1),
        );
        assert_eq!(package.len(), 1);
        assert!(package[0].uri == "adt://1" || package[0].uri == "adt://4");
    }

    #[test]
    fn parse_abap_namespaces_from_table_raw_extracts_namespace_tokens() {
        let raw = r#"
<dpr:result>
  <dpr:value>/S4TAX/</dpr:value>
  <dpr:value>/1BEA/</dpr:value>
  <dpr:value>/sap/</dpr:value>
  <dpr:value>/AB_C/</dpr:value>
</dpr:result>
"#;
        let namespaces = parse_abap_namespaces_from_table_raw(raw);
        assert_eq!(namespaces, vec!["/1BEA/", "/AB_C/", "/S4TAX/"]);
    }

    #[test]
    fn parse_abap_namespaces_from_table_raw_ignores_uri_paths() {
        let raw = r#"
<dpr:meta href="http://www.sap.com/adt/datapreview/ddic"/>
<dpr:value>/SAP/BASIS</dpr:value>
<dpr:value>/CPD/MAIN/</dpr:value>
"#;
        let namespaces = parse_abap_namespaces_from_table_raw(raw);
        assert_eq!(namespaces, vec!["/CPD/", "/SAP/"]);
    }

    #[test]
    fn resolve_runtime_config_prefers_explicit_server_over_active_and_env() {
        let store_path = unique_temp_path("resolve_explicit")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[
                (
                    NEURO_ADT_SERVER_STORE_PATH_ENV,
                    Some(store_path_text.as_str()),
                ),
                ("NEURO_SAP_URL", Some("https://env.local")),
                ("NEURO_SAP_USER", Some("env-user")),
                ("NEURO_SAP_PASSWORD", Some("env-pass")),
                ("NEURO_ADT_CSRF_FETCH_PATH", Some("/custom/neuro/csrf")),
                ("SAP_ADT_CSRF_FETCH_PATH", Some("/custom/legacy/csrf")),
            ],
            || {
                let mut store = AdtServerStore::default();
                upsert_server(
                    &mut store,
                    AdtServerUpsertRequest {
                        id: "srv_a".to_string(),
                        name: "Server A".to_string(),
                        base_url: "https://srv-a.local".to_string(),
                        client: None,
                        language: None,
                        username: Some("alice".to_string()),
                        password: Some("secret-a".to_string()),
                        verify_tls: Some(true),
                        active: Some(false),
                    },
                )
                .expect("first upsert should succeed");
                upsert_server(
                    &mut store,
                    AdtServerUpsertRequest {
                        id: "srv_b".to_string(),
                        name: "Server B".to_string(),
                        base_url: "https://srv-b.local".to_string(),
                        client: None,
                        language: None,
                        username: Some("bob".to_string()),
                        password: Some("secret-b".to_string()),
                        verify_tls: Some(false),
                        active: Some(true),
                    },
                )
                .expect("second upsert should succeed");
                save_server_store(&store).expect("store should persist");

                let config = resolve_runtime_config(Some("srv_a"))
                    .expect("runtime config for explicit server should resolve");
                assert_eq!(
                    config.sap.url.as_ref().map(|entry| entry.value.as_str()),
                    Some("https://srv-a.local")
                );
                assert_eq!(
                    config.sap.url.as_ref().map(|entry| entry.source),
                    Some("server_store")
                );
                assert_eq!(
                    config.sap.user.as_ref().map(|entry| entry.value.as_str()),
                    Some("alice")
                );
                assert_eq!(
                    config
                        .sap
                        .password
                        .as_ref()
                        .map(|entry| entry.value.as_str()),
                    Some("secret-a")
                );
                assert_eq!(config.server_selection.server_id.as_deref(), Some("srv_a"));
                assert_eq!(config.server_selection.source, "command.server_id");
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn resolve_runtime_config_uses_active_server_then_env_fallback() {
        let store_path = unique_temp_path("resolve_active")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[
                (
                    NEURO_ADT_SERVER_STORE_PATH_ENV,
                    Some(store_path_text.as_str()),
                ),
                ("NEURO_SAP_URL", Some("https://env.local")),
                ("NEURO_SAP_USER", Some("env-user")),
                ("NEURO_SAP_PASSWORD", Some("env-pass")),
                ("NEURO_ADT_CSRF_FETCH_PATH", Some("/custom/neuro/csrf")),
                ("SAP_ADT_CSRF_FETCH_PATH", Some("/custom/legacy/csrf")),
            ],
            || {
                let mut store = AdtServerStore::default();
                upsert_server(
                    &mut store,
                    AdtServerUpsertRequest {
                        id: "srv_active".to_string(),
                        name: "Server Active".to_string(),
                        base_url: "https://active.local".to_string(),
                        client: None,
                        language: None,
                        username: Some("active-user".to_string()),
                        password: Some("active-pass".to_string()),
                        verify_tls: Some(true),
                        active: Some(true),
                    },
                )
                .expect("active upsert should succeed");
                save_server_store(&store).expect("store should persist");

                let active_config =
                    resolve_runtime_config(None).expect("active server should resolve");
                assert_eq!(
                    active_config
                        .sap
                        .url
                        .as_ref()
                        .map(|entry| entry.value.as_str()),
                    Some("https://active.local")
                );
                assert_eq!(
                    active_config.server_selection.server_id.as_deref(),
                    Some("srv_active")
                );
                assert_eq!(active_config.server_selection.source, "server_store.active");
                assert_eq!(
                    active_config.sap.csrf_fetch_path.value,
                    "/custom/neuro/csrf"
                );
                assert_eq!(
                    active_config.sap.csrf_fetch_path.source,
                    "NEURO_ADT_CSRF_FETCH_PATH"
                );

                let removed = fs::remove_file(&store_path);
                assert!(removed.is_ok(), "store file should be removable");

                let env_config = resolve_runtime_config(None).expect("env fallback should resolve");
                assert_eq!(
                    env_config
                        .sap
                        .url
                        .as_ref()
                        .map(|entry| entry.value.as_str()),
                    Some("https://env.local")
                );
                assert_eq!(env_config.server_selection.server_id, None);
                assert_eq!(env_config.server_selection.source, "env");
                assert_eq!(env_config.sap.csrf_fetch_path.value, "/custom/neuro/csrf");
                assert_eq!(
                    env_config.sap.csrf_fetch_path.source,
                    "NEURO_ADT_CSRF_FETCH_PATH"
                );
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn resolve_runtime_config_does_not_mix_env_credentials_for_server_override() {
        let store_path = unique_temp_path("resolve_no_credential_mix")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[
                (
                    NEURO_ADT_SERVER_STORE_PATH_ENV,
                    Some(store_path_text.as_str()),
                ),
                ("NEURO_SAP_USER", Some("env-user")),
                ("NEURO_SAP_PASSWORD", Some("env-pass")),
            ],
            || {
                let mut store = AdtServerStore::default();
                upsert_server(
                    &mut store,
                    AdtServerUpsertRequest {
                        id: "srv_a".to_string(),
                        name: "Server A".to_string(),
                        base_url: "https://srv-a.local".to_string(),
                        client: None,
                        language: None,
                        username: Some("store-user".to_string()),
                        password: None,
                        verify_tls: Some(true),
                        active: Some(true),
                    },
                )
                .expect("upsert should succeed");
                save_server_store(&store).expect("store should persist");

                let config = resolve_runtime_config(Some("srv_a"))
                    .expect("runtime config for explicit server should resolve");
                assert_eq!(
                    config.sap.user.as_ref().map(|entry| entry.value.as_str()),
                    Some("store-user")
                );
                assert!(config.sap.password.is_none());
                assert!(!config.sap.is_ready());
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn resolve_runtime_init_target_uses_same_server_id_and_cache_key() {
        let store_path = unique_temp_path("resolve_init_target")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                let mut store = AdtServerStore::default();
                upsert_server(
                    &mut store,
                    AdtServerUpsertRequest {
                        id: "srv_active".to_string(),
                        name: "Server Active".to_string(),
                        base_url: "https://active.local".to_string(),
                        client: None,
                        language: None,
                        username: Some("active-user".to_string()),
                        password: Some("active-pass".to_string()),
                        verify_tls: Some(true),
                        active: Some(true),
                    },
                )
                .expect("upsert should succeed");
                save_server_store(&store).expect("store should persist");

                let target = resolve_runtime_init_target(None).expect("target should resolve");
                assert_eq!(target.resolved_server_id.as_deref(), Some("srv_active"));
                assert_eq!(target.cache_key, "server:srv_active");

                let explicit_target = resolve_runtime_init_target(Some("srv_active"))
                    .expect("explicit target should resolve");
                assert_eq!(
                    explicit_target.resolved_server_id.as_deref(),
                    Some("srv_active")
                );
                assert_eq!(explicit_target.cache_key, "server:srv_active");
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[cfg(unix)]
    #[test]
    fn save_server_store_uses_restrictive_permissions() {
        let store_path = unique_temp_path("store_permissions")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                save_server_store(&AdtServerStore::default()).expect("store write should succeed");
                let metadata = fs::metadata(&store_path).expect("store metadata should exist");
                let mode = metadata.permissions().mode() & 0o777;
                assert_eq!(mode, 0o600);
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[tokio::test]
    async fn telemetry_reports_success_without_changing_result() {
        let _guard = telemetry_async_lock().lock().await;
        drain_test_telemetry_events();

        let result = run_with_neuro_command_telemetry("neuro_search_objects", async {
            Ok::<_, NeuroRuntimeError>(17u32)
        })
        .await
        .expect("operation should succeed");
        assert_eq!(result, 17);

        let event = take_single_telemetry_event();
        assert_eq!(
            event.get("event"),
            Some(&json!(NEURO_COMMAND_TELEMETRY_EVENT))
        );
        assert_eq!(event.get("command"), Some(&json!("neuro_search_objects")));
        assert_eq!(event.get("success"), Some(&json!(true)));
        assert!(event.get("latencyMs").and_then(Value::as_u64).is_some());
        assert!(event.get("errorCode").is_none());
        assert!(event.get("errorMessage").is_none());
    }

    #[tokio::test]
    async fn telemetry_reports_failure_and_preserves_error_details() {
        let _guard = telemetry_async_lock().lock().await;
        drain_test_telemetry_events();

        let expected = runtime_error(
            NeuroRuntimeErrorCode::WsUnavailable,
            "ws is unavailable".to_string(),
            None,
        );
        let result: Result<(), NeuroRuntimeError> =
            run_with_neuro_command_telemetry("neuro_ws_request", async { Err(expected) }).await;
        let error = result.expect_err("operation should fail");
        assert_eq!(error.code, NeuroRuntimeErrorCode::WsUnavailable);
        assert_eq!(error.message, "ws is unavailable");

        let event = take_single_telemetry_event();
        assert_eq!(
            event.get("event"),
            Some(&json!(NEURO_COMMAND_TELEMETRY_EVENT))
        );
        assert_eq!(event.get("command"), Some(&json!("neuro_ws_request")));
        assert_eq!(event.get("success"), Some(&json!(false)));
        assert_eq!(
            event.get("errorCode"),
            Some(&error_code_value(&NeuroRuntimeErrorCode::WsUnavailable))
        );
        assert_eq!(event.get("errorMessage"), Some(&json!("ws is unavailable")));
    }

    #[tokio::test]
    async fn connect_failure_preserves_runtime_error_code_from_get_or_init_mapping() {
        let expected_message = "auth failed while initializing runtime";
        let expected_details = json!({ "statusCode": 403 });
        let expected = runtime_error(
            NeuroRuntimeErrorCode::AdtAuthError,
            expected_message.to_string(),
            Some(expected_details.clone()),
        );
        let port = ConnectFailurePort::new(expected);
        let state = AppState::default();

        let result = neuro_adt_use_cases::neuro_adt_server_connect(&state, None, &port)
            .await
            .map_err(map_neuro_adt_error);

        let error = result.expect_err("connect should propagate failure");
        assert_eq!(error.code, NeuroRuntimeErrorCode::AdtAuthError);
        assert_eq!(error.message, expected_message);
        assert_eq!(error.details, Some(expected_details));
    }
    #[test]
    fn telemetry_latency_is_capped_to_u64_range() {
        let _guard = telemetry_lock().lock().expect("telemetry lock poisoned");
        drain_test_telemetry_events();

        let result: Result<(), NeuroRuntimeError> = Ok(());
        emit_neuro_command_telemetry(
            "neuro_runtime_diagnose",
            Duration::from_secs(u64::MAX),
            &result,
        );

        let event = take_single_telemetry_event();
        assert_eq!(event.get("latencyMs"), Some(&json!(u64::MAX)));
    }

    #[test]
    fn runtime_init_telemetry_emits_error_fields_on_failure() {
        let _guard = telemetry_lock().lock().expect("telemetry lock poisoned");
        drain_test_telemetry_events();

        let error = runtime_error(
            NeuroRuntimeErrorCode::RuntimeInitError,
            "missing SAP URL".to_string(),
            None,
        );
        emit_neuro_runtime_init_telemetry("failed", Duration::from_millis(9), Some(&error));

        let event = take_single_telemetry_event();
        assert_eq!(
            event.get("event"),
            Some(&json!(NEURO_RUNTIME_INIT_TELEMETRY_EVENT))
        );
        assert_eq!(event.get("status"), Some(&json!("failed")));
        assert_eq!(event.get("success"), Some(&json!(false)));
        assert_eq!(
            event.get("errorCode"),
            Some(&error_code_value(&NeuroRuntimeErrorCode::RuntimeInitError))
        );
    }

    #[test]
    fn runtime_init_telemetry_suppresses_cache_hit_by_default() {
        with_env_overrides(&[(NEURO_RUNTIME_INIT_TELEMETRY_VERBOSE_ENV, None)], || {
            let _guard = telemetry_lock().lock().expect("telemetry lock poisoned");
            drain_test_telemetry_events();

            emit_neuro_runtime_init_telemetry("cache_hit", Duration::from_millis(9), None);
            assert!(
                drain_test_telemetry_events().is_empty(),
                "cache-hit telemetry should be suppressed by default"
            );
        });
    }

    #[test]
    fn runtime_init_telemetry_emits_cache_hit_when_verbose_env_enabled() {
        with_env_overrides(
            &[(NEURO_RUNTIME_INIT_TELEMETRY_VERBOSE_ENV, Some("1"))],
            || {
                let _guard = telemetry_lock().lock().expect("telemetry lock poisoned");
                drain_test_telemetry_events();

                emit_neuro_runtime_init_telemetry(
                    "cache_hit_after_gate",
                    Duration::from_millis(9),
                    None,
                );

                let event = take_single_telemetry_event();
                assert_eq!(
                    event.get("event"),
                    Some(&json!(NEURO_RUNTIME_INIT_TELEMETRY_EVENT))
                );
                assert_eq!(event.get("status"), Some(&json!("cache_hit_after_gate")));
                assert_eq!(event.get("success"), Some(&json!(true)));
                assert!(event.get("errorCode").is_none());
                assert!(event.get("errorMessage").is_none());
            },
        );
    }

    #[test]
    fn clear_runtime_cache_waits_for_init_gate_before_clearing_cache() {
        let store_path = unique_temp_path("clear_cache_gate")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[
                (
                    NEURO_ADT_SERVER_STORE_PATH_ENV,
                    Some(store_path_text.as_str()),
                ),
                ("NEURO_SAP_URL", Some("https://example.local")),
                ("NEURO_SAP_USER", Some("tester")),
                ("NEURO_SAP_PASSWORD", Some("secret")),
            ],
            || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .build()
                    .expect("runtime should build");

                runtime.block_on(async {
                    let state = Arc::new(AppState::default());
                    let target = resolve_runtime_init_target(None)
                        .expect("runtime init target should resolve");

                    let _runtime = get_or_init(state.as_ref(), None)
                        .await
                        .expect("runtime init should succeed");
                    {
                        let cache = state.neuro_runtime_cache.lock().await;
                        assert!(
                            cache.contains_key(target.cache_key.as_str()),
                            "cache should contain initialized runtime before invalidation"
                        );
                    }

                    let gate_guard = state.neuro_runtime_init_gate.lock().await;
                    let clear_state = Arc::clone(&state);
                    let clear_task = tokio::spawn(async move {
                        clear_runtime_cache(clear_state.as_ref()).await;
                    });

                    tokio::time::sleep(Duration::from_millis(25)).await;
                    assert!(
                        !clear_task.is_finished(),
                        "clear_runtime_cache should wait for init gate"
                    );
                    {
                        let cache = state.neuro_runtime_cache.lock().await;
                        assert!(
                            cache.contains_key(target.cache_key.as_str()),
                            "cache should remain populated while clear is blocked by init gate"
                        );
                    }

                    drop(gate_guard);
                    clear_task.await.expect("clear task should complete");
                    let cache = state.neuro_runtime_cache.lock().await;
                    assert!(
                        cache.is_empty(),
                        "cache should be cleared after gate release"
                    );
                });
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
