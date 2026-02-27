use neuro_adt_core::AdtClientError;
use neuro_adt_ws::NeuroWsClientError;
use neuro_engine::NeuroEngineError;
use neuro_mcp::{NeuroMcpError, NeuroMcpFacade, NeuroToolSpec};
use neuro_types::{
    AdtAuth, AdtHttpConfig, AdtHttpEndpoints, DiagnoseStatus, NeuroEngineConfig, NeuroRuntimeError,
    NeuroRuntimeErrorCode, RuntimeDiagnoseComponent, RuntimeDiagnoseResponse, SafetyPolicy,
    WsClientConfig, WsDomainRequest, WsMessageEnvelope,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::future::Future;
use std::sync::Arc;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::AppState;

const DEFAULT_ADT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_WS_TIMEOUT_SECS: u64 = 15;
const DEFAULT_ADT_CSRF_FETCH_PATH: &str = "/sap/bc/adt";
const DEFAULT_ADT_SEARCH_PATH: &str =
    "/sap/bc/adt/repository/informationsystem/search?operation=quickSearch";
const NEURO_COMMAND_TELEMETRY_EVENT: &str = "neuro.command";
const NEURO_RUNTIME_INIT_TELEMETRY_EVENT: &str = "neuro.runtime_init";

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
}

pub struct NeuroRuntime {
    config: RuntimeConfig,
    engine: Arc<neuro_engine::NeuroEngine>,
}

impl NeuroRuntime {
    async fn initialize() -> Result<Self, NeuroRuntimeError> {
        let config = resolve_runtime_config()?;
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

fn resolve_runtime_config() -> Result<RuntimeConfig, NeuroRuntimeError> {
    let (sap_insecure_tls, sap_insecure_tls_source) =
        parse_env_bool_with_default(&["NEURO_SAP_INSECURE", "SAP_INSECURE"], false)?;
    let (sap_timeout_secs, sap_timeout_source) = parse_env_u64_with_default(
        &["NEURO_SAP_TIMEOUT_SECS", "SAP_TIMEOUT_SECS"],
        DEFAULT_ADT_TIMEOUT_SECS,
    )?;

    let sap = SapConfig {
        url: first_non_empty_env(&["NEURO_SAP_URL", "SAP_URL"]),
        user: first_non_empty_env(&["NEURO_SAP_USER", "SAP_USER", "SAP_USERNAME"]),
        password: first_non_empty_env(&["NEURO_SAP_PASSWORD", "SAP_PASSWORD", "SAP_PASS"]),
        client: first_non_empty_env(&["NEURO_SAP_CLIENT", "SAP_CLIENT"]),
        language: first_non_empty_env(&["NEURO_SAP_LANGUAGE", "SAP_LANGUAGE", "SAP_LANG"]),
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

    Ok(RuntimeConfig { sap, ws, safety })
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
            detail: "Required SAP runtime environment variables are configured".to_string(),
            latency_ms: None,
        }
    } else {
        RuntimeDiagnoseComponent {
            component: "sap_env_config".to_string(),
            status: DiagnoseStatus::Degraded,
            detail: "Missing one or more required SAP runtime environment variables".to_string(),
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

fn runtime_error(
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

pub async fn get_or_init(state: &AppState) -> Result<Arc<NeuroRuntime>, NeuroRuntimeError> {
    let started_at = Instant::now();

    if let Some(runtime) = {
        let cache = state.neuro_runtime.lock().await;
        cache.clone()
    } {
        emit_neuro_runtime_init_telemetry("cache_hit", started_at.elapsed(), None);
        return Ok(runtime);
    }

    let _init_gate = state.neuro_runtime_init_gate.lock().await;

    if let Some(runtime) = {
        let cache = state.neuro_runtime.lock().await;
        cache.clone()
    } {
        emit_neuro_runtime_init_telemetry("cache_hit_after_gate", started_at.elapsed(), None);
        return Ok(runtime);
    }

    let initialized = match NeuroRuntime::initialize().await {
        Ok(runtime) => Arc::new(runtime),
        Err(error) => {
            emit_neuro_runtime_init_telemetry("failed", started_at.elapsed(), Some(&error));
            return Err(error);
        }
    };

    let mut cache = state.neuro_runtime.lock().await;
    *cache = Some(Arc::clone(&initialized));
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
    if let Some(runtime) = {
        let cache = state.neuro_runtime.lock().await;
        cache.clone()
    } {
        return success_response(runtime, true).await;
    }

    match get_or_init(state).await {
        Ok(runtime) => success_response(runtime, false).await,
        Err(error) => init_error_response(error),
    }
}

pub async fn neuro_search_objects_impl(
    state: State<'_, AppState>,
    query: String,
    max_results: Option<u32>,
) -> Result<Vec<neuro_types::AdtObjectSummary>, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_search_objects", async {
        let runtime = get_or_init(state.inner()).await?;
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
) -> Result<neuro_types::AdtSourceResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_get_source", async {
        let runtime = get_or_init(state.inner()).await?;
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
    request: neuro_types::AdtUpdateSourceRequest,
) -> Result<neuro_types::AdtUpdateSourceResponse, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_update_source", async {
        let runtime = get_or_init(state.inner()).await?;
        runtime
            .engine
            .update_source(request)
            .await
            .map_err(map_engine_error)
    })
    .await
}

pub async fn neuro_ws_request_impl(
    state: State<'_, AppState>,
    request: WsDomainRequest,
) -> Result<WsMessageEnvelope<Value>, NeuroRuntimeError> {
    run_with_neuro_command_telemetry("neuro_ws_request", async {
        let runtime = get_or_init(state.inner()).await?;
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
        let runtime = get_or_init(state.inner()).await?;
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
        let runtime = get_or_init(state.inner()).await?;
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
    use std::time::Duration;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn telemetry_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn take_single_telemetry_event() -> Value {
        let mut events = drain_test_telemetry_events();
        assert_eq!(events.len(), 1, "expected exactly one telemetry event");
        serde_json::from_str(&events.remove(0)).expect("telemetry payload should be valid JSON")
    }

    fn with_env_overrides(pairs: &[(&str, Option<&str>)], test: impl FnOnce()) {
        let _guard = env_lock().lock().expect("env lock poisoned");

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
    fn resolve_runtime_config_prefers_neuro_env_over_fallbacks() {
        with_env_overrides(
            &[
                ("NEURO_SAP_URL", Some("https://neuro.local")),
                ("SAP_URL", Some("https://legacy.local")),
                ("NEURO_SAP_USER", Some("neuro-user")),
                ("SAP_USER", Some("legacy-user")),
                ("NEURO_SAP_PASSWORD", Some("neuro-pass")),
                ("SAP_PASSWORD", Some("legacy-pass")),
                ("NEURO_WS_URL", Some("wss://neuro.ws")),
                ("SAP_WS_URL", Some("wss://legacy.ws")),
                ("NEURO_SAFETY_READ_ONLY", Some("false")),
                ("NEURO_UPDATE_REQUIRE_ETAG", Some("true")),
            ],
            || {
                let config = resolve_runtime_config().expect("runtime config should resolve");
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
                assert!(!config.safety.read_only);
                assert!(config.safety.require_etag_for_updates);
            },
        );
    }

    #[test]
    fn resolve_runtime_config_rejects_invalid_boolean_values() {
        with_env_overrides(&[("NEURO_SAP_INSECURE", Some("not-bool"))], || {
            let error = resolve_runtime_config().expect_err("invalid boolean must fail");
            assert_eq!(error.code, NeuroRuntimeErrorCode::InvalidArgument);
            assert!(error.message.contains("NEURO_SAP_INSECURE"));
        });
    }

    #[tokio::test]
    async fn telemetry_reports_success_without_changing_result() {
        let _guard = telemetry_lock().lock().expect("telemetry lock poisoned");
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
        let _guard = telemetry_lock().lock().expect("telemetry lock poisoned");
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
}
