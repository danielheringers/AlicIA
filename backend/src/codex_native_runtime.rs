#[cfg(feature = "native-codex-runtime")]
use codex_core::config::{ConfigBuilder, ConfigOverrides};
use serde::Serialize;
#[cfg(feature = "native-codex-runtime")]
use std::sync::Arc;
use tauri::State;
#[cfg(feature = "native-codex-runtime")]
use toml::map::Map as TomlMap;

use crate::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCodexRuntimeDiagnoseMetadata {
    pub feature_enabled: bool,
    pub cached_runtime: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex_home: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCodexRuntimeDiagnoseResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub metadata: NativeCodexRuntimeDiagnoseMetadata,
}

#[cfg(feature = "native-codex-runtime")]
pub struct NativeCodexRuntime {
    pub codex_home: std::path::PathBuf,
    pub auth_manager: Arc<codex_core::AuthManager>,
    pub thread_manager: Arc<codex_core::ThreadManager>,
    pub session_source: codex_protocol::protocol::SessionSource,
}

#[cfg(not(feature = "native-codex-runtime"))]
#[allow(dead_code)]
#[derive(Debug)]
pub struct NativeCodexRuntime;

#[cfg(feature = "native-codex-runtime")]
const ALICIA_NATIVE_INTERNAL_PROFILE: &str = "__alicia_native_internal";

#[cfg(feature = "native-codex-runtime")]
fn native_internal_profile_cli_overrides() -> Vec<(String, toml::Value)> {
    vec![(
        format!("profiles.{ALICIA_NATIVE_INTERNAL_PROFILE}"),
        toml::Value::Table(TomlMap::new()),
    )]
}

#[cfg(feature = "native-codex-runtime")]
fn native_internal_profile_harness_overrides() -> ConfigOverrides {
    ConfigOverrides {
        config_profile: Some(ALICIA_NATIVE_INTERNAL_PROFILE.to_string()),
        ..Default::default()
    }
}

#[cfg(feature = "native-codex-runtime")]
impl NativeCodexRuntime {
    pub async fn initialize() -> Result<Self, String> {
        let config = ConfigBuilder::default()
            .cli_overrides(native_internal_profile_cli_overrides())
            .harness_overrides(native_internal_profile_harness_overrides())
            .build()
            .await
            .map_err(|error| format!("failed to build codex runtime config: {error}"))?;

        let codex_home = config.codex_home.clone();
        let forced_workspace_id = config.forced_chatgpt_workspace_id.clone();
        let auth_manager = Arc::new(codex_core::AuthManager::new(
            codex_home.clone(),
            true,
            config.cli_auth_credentials_store_mode,
        ));
        auth_manager.set_forced_chatgpt_workspace_id(forced_workspace_id);

        let session_source = codex_protocol::protocol::SessionSource::VSCode;
        let thread_manager = Arc::new(codex_core::ThreadManager::new(
            codex_home.clone(),
            Arc::clone(&auth_manager),
            session_source.clone(),
        ));

        Ok(Self {
            codex_home,
            auth_manager,
            thread_manager,
            session_source,
        })
    }
}

#[cfg(feature = "native-codex-runtime")]
pub async fn native_runtime_get_or_init(
    state: &AppState,
) -> Result<Arc<NativeCodexRuntime>, String> {
    if let Some(runtime) = {
        let cache = state.native_codex_runtime.lock().await;
        cache.clone()
    } {
        return Ok(runtime);
    }

    let _init_gate = state.native_codex_runtime_init_gate.lock().await;

    if let Some(runtime) = {
        let cache = state.native_codex_runtime.lock().await;
        cache.clone()
    } {
        return Ok(runtime);
    }

    let initialized = NativeCodexRuntime::initialize()
        .await
        .map(Arc::new)
        .map_err(|error| {
            let message = format!("failed to initialize native runtime: {error}");
            eprintln!("[native-runtime] {message}");
            message
        })?;

    let mut cache = state.native_codex_runtime.lock().await;
    *cache = Some(Arc::clone(&initialized));
    Ok(initialized)
}

#[cfg(not(feature = "native-codex-runtime"))]
#[allow(dead_code)]
pub async fn native_runtime_get_or_init(_state: &AppState) -> Result<NativeCodexRuntime, String> {
    Err("native runtime feature is disabled".to_string())
}

#[cfg(not(feature = "native-codex-runtime"))]
fn disabled_response() -> NativeCodexRuntimeDiagnoseResponse {
    NativeCodexRuntimeDiagnoseResponse {
        ok: true,
        error: None,
        metadata: NativeCodexRuntimeDiagnoseMetadata {
            feature_enabled: false,
            cached_runtime: false,
            codex_home: None,
            session_source: None,
            auth_mode: None,
            thread_count: None,
        },
    }
}

#[cfg(feature = "native-codex-runtime")]
fn auth_mode_label(auth_mode: Option<codex_core::auth::AuthMode>) -> Option<String> {
    auth_mode.map(|mode| match mode {
        codex_core::auth::AuthMode::ApiKey => "api_key".to_string(),
        codex_core::auth::AuthMode::Chatgpt => "chatgpt".to_string(),
    })
}

#[cfg(feature = "native-codex-runtime")]
fn init_error_response(error: String) -> NativeCodexRuntimeDiagnoseResponse {
    NativeCodexRuntimeDiagnoseResponse {
        ok: false,
        error: Some(error),
        metadata: NativeCodexRuntimeDiagnoseMetadata {
            feature_enabled: true,
            cached_runtime: false,
            codex_home: None,
            session_source: None,
            auth_mode: None,
            thread_count: None,
        },
    }
}

#[cfg(feature = "native-codex-runtime")]
async fn success_response(
    runtime: Arc<NativeCodexRuntime>,
    cached_runtime: bool,
) -> NativeCodexRuntimeDiagnoseResponse {
    runtime.auth_manager.reload();
    let codex_home_configured = !runtime.codex_home.as_os_str().is_empty();
    let thread_count = runtime.thread_manager.list_thread_ids().await.len();
    NativeCodexRuntimeDiagnoseResponse {
        ok: true,
        error: None,
        metadata: NativeCodexRuntimeDiagnoseMetadata {
            feature_enabled: true,
            cached_runtime,
            codex_home: codex_home_configured.then(|| "configured".to_string()),
            session_source: Some(runtime.session_source.to_string()),
            auth_mode: auth_mode_label(runtime.auth_manager.auth_mode()),
            thread_count: Some(thread_count),
        },
    }
}

#[cfg(feature = "native-codex-runtime")]
pub async fn codex_native_runtime_diagnose_impl(
    state: State<'_, AppState>,
) -> Result<NativeCodexRuntimeDiagnoseResponse, String> {
    if let Some(runtime) = {
        let cache = state.inner().native_codex_runtime.lock().await;
        cache.clone()
    } {
        return Ok(success_response(runtime, true).await);
    }

    match native_runtime_get_or_init(state.inner()).await {
        Ok(runtime) => Ok(success_response(runtime, false).await),
        Err(error) => Ok(init_error_response(error)),
    }
}

#[cfg(not(feature = "native-codex-runtime"))]
pub async fn codex_native_runtime_diagnose_impl(
    _state: State<'_, AppState>,
) -> Result<NativeCodexRuntimeDiagnoseResponse, String> {
    Ok(disabled_response())
}
