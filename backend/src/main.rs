use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex as AsyncMutex;

mod account_runtime;
mod app_server_runtime;
mod codex_event_translator;
mod codex_native_runtime;
mod command_runtime;
mod config_runtime;
mod events_runtime;
mod generated;
mod interface;
mod launch_runtime;
mod mcp_runtime;
mod models_runtime;
mod neuro_runtime;
mod session_lifecycle_runtime;
mod session_runtime;
mod session_turn_runtime;
mod status_runtime;
mod terminal_runtime;
mod workspace_runtime;
use crate::account_runtime::{
    AccountLoginStartRequest, AccountLoginStartResponse, AccountLogoutResponse,
    AccountRateLimitsReadResponse, AccountReadRequest, AccountReadResponse, AppListRequest,
    AppListResponse,
};
use crate::interface::tauri::commands::{
    runtime_config::{
        codex_config_get, codex_config_set, codex_runtime_capabilities, codex_runtime_status,
        load_codex_default_config, update_codex_config,
    },
    session_lifecycle::{resize_codex_pty, start_codex_session, stop_codex_session},
    session_turn::{
        codex_approval_respond, codex_review_start, codex_thread_archive, codex_thread_close,
        codex_thread_compact_start, codex_thread_fork, codex_thread_list, codex_thread_open,
        codex_thread_read, codex_thread_rollback, codex_thread_unarchive, codex_turn_interrupt,
        codex_turn_run, codex_turn_steer, codex_user_input_respond, send_codex_input,
    },
    terminal::{terminal_create, terminal_kill, terminal_resize, terminal_write},
    utility::{codex_help_snapshot, pick_image_file, pick_mention_file, pick_workspace_folder},
    workspace_git::{
        codex_workspace_create_directory, codex_workspace_list_directory,
        codex_workspace_read_file, codex_workspace_rename_entry, codex_workspace_write_file,
        git_commit_approved_review, git_workspace_changes, run_codex_command,
    },
};
use crate::mcp_runtime::{
    McpLoginRequest, McpLoginResponse, McpReloadResponse, McpServerListResponse,
    McpStartupWarmupResponse,
};
#[cfg(feature = "native-codex-runtime")]
use codex_core::CodexThread;

pub(crate) use crate::events_runtime::{
    emit_codex_event, emit_lifecycle, emit_stderr, emit_stdout, emit_terminal_data,
    emit_terminal_exit,
};
pub(crate) use crate::interface::tauri::dto::{
    CodexWorkspaceCreateDirectoryRequest, CodexWorkspaceCreateDirectoryResponse,
    CodexWorkspaceListDirectoryEntry, CodexWorkspaceListDirectoryEntryKind,
    CodexWorkspaceListDirectoryRequest, CodexWorkspaceListDirectoryResponse,
    CodexWorkspaceReadFileRequest, CodexWorkspaceReadFileResponse,
    CodexWorkspaceRenameEntryRequest, CodexWorkspaceRenameEntryResponse,
    CodexWorkspaceWriteFileRequest, CodexWorkspaceWriteFileResponse, GitCommandExecutionResult,
    GitCommitApprovedReviewRequest, GitCommitApprovedReviewResponse, GitWorkspaceChange,
    GitWorkspaceChangesRequest, GitWorkspaceChangesResponse, RunCodexCommandResponse,
    RuntimeCapabilitiesResponse, RuntimeCodexConfig, RuntimeContractMetadata,
    StartCodexSessionConfig, StartCodexSessionResponse, TerminalCreateRequest,
    TerminalCreateResponse, TerminalKillRequest, TerminalResizeRequest, TerminalWriteRequest,
};
pub(crate) use crate::launch_runtime::{
    default_codex_binary, resolve_binary_path, resolve_codex_launch,
};

fn parse_env_bool_flag(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

fn neuro_startup_probe_enabled() -> bool {
    match env::var("NEURO_LOG_STARTUP_DIAGNOSE") {
        Ok(raw) => parse_env_bool_flag(&raw).unwrap_or(true),
        Err(_) => true,
    }
}

fn diagnose_status_label(status: neuro_types::DiagnoseStatus) -> &'static str {
    match status {
        neuro_types::DiagnoseStatus::Healthy => "healthy",
        neuro_types::DiagnoseStatus::Degraded => "degraded",
        neuro_types::DiagnoseStatus::Unavailable => "unavailable",
    }
}

fn spawn_neuro_startup_probe(app_handle: AppHandle) {
    if !neuro_startup_probe_enabled() {
        eprintln!("[neuro-startup] startup diagnose probe disabled by NEURO_LOG_STARTUP_DIAGNOSE");
        return;
    }

    tauri::async_runtime::spawn(async move {
        eprintln!("[neuro-startup] running startup ADT diagnose probe");

        let state = app_handle.state::<AppState>();
        let report =
            crate::neuro_runtime::neuro_runtime_diagnose_for_app_state(state.inner()).await;

        eprintln!(
            "[neuro-startup] overall_status={}",
            diagnose_status_label(report.overall_status)
        );

        if let Some(component) = report
            .components
            .iter()
            .find(|component| component.component == "adt_http")
        {
            eprintln!(
                "[neuro-startup] adt_http status={} detail={}",
                diagnose_status_label(component.status),
                component.detail
            );
        } else {
            eprintln!("[neuro-startup] adt_http component was not reported by diagnose");
        }

        if let Some(component) = report
            .components
            .iter()
            .filter(|component| component.status != neuro_types::DiagnoseStatus::Healthy)
            .max_by_key(|component| component.status)
        {
            eprintln!(
                "[neuro-startup] critical_component={} status={} detail={}",
                component.component,
                diagnose_status_label(component.status),
                component.detail
            );
        }
    });
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CodexReasoningEffortOption {
    reasoning_effort: String,
    description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CodexModel {
    id: String,
    model: String,
    display_name: String,
    description: String,
    supported_reasoning_efforts: Vec<CodexReasoningEffortOption>,
    default_reasoning_effort: String,
    supports_personality: bool,
    is_default: bool,
    upgrade: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CodexModelListResponse {
    data: Vec<CodexModel>,
}

struct TerminalSession {
    terminal_id: u64,
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionTransport {
    Native,
}

#[cfg(feature = "native-codex-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
enum NativeApprovalKind {
    CommandExecution,
    FileChange,
}

#[cfg(feature = "native-codex-runtime")]
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct NativePendingApproval {
    thread_id: String,
    turn_id: String,
    call_id: String,
    kind: NativeApprovalKind,
}

#[cfg(feature = "native-codex-runtime")]
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct NativePendingUserInput {
    thread_id: String,
    turn_id: String,
    call_id: String,
}

#[cfg(feature = "native-codex-runtime")]
#[allow(dead_code)]
struct NativeSessionHandles {
    runtime: Arc<codex_native_runtime::NativeCodexRuntime>,
    threads: HashMap<String, Arc<CodexThread>>,
    active_turns: HashMap<String, String>,
    pending_approvals: HashMap<String, NativePendingApproval>,
    pending_user_inputs: HashMap<String, NativePendingUserInput>,
    next_approval_id: u64,
    next_user_input_id: u64,
}

#[allow(clippy::large_enum_variant)]
enum ActiveSessionTransport {
    Native(NativeSessionHandles),
}

struct ActiveSession {
    session_id: u64,
    pid: Option<u32>,
    binary: String,
    cwd: PathBuf,
    thread_id: Option<String>,
    busy: bool,
    transport: ActiveSessionTransport,
}

impl ActiveSession {
    fn transport(&self) -> SessionTransport {
        let _ = &self.transport;
        SessionTransport::Native
    }
}

struct AppState {
    active_session: Mutex<Option<ActiveSession>>,
    session_start_gate: AsyncMutex<()>,
    next_session_id: AtomicU64,
    runtime_config: Mutex<RuntimeCodexConfig>,
    next_event_seq: Arc<AtomicU64>,
    next_terminal_id: AtomicU64,
    terminals: Mutex<HashMap<u64, TerminalSession>>,
    #[cfg(feature = "native-codex-runtime")]
    native_codex_runtime: AsyncMutex<Option<Arc<codex_native_runtime::NativeCodexRuntime>>>,
    #[cfg(feature = "native-codex-runtime")]
    native_codex_runtime_init_gate: AsyncMutex<()>,
    neuro_runtime_cache: AsyncMutex<HashMap<String, Arc<neuro_runtime::NeuroRuntime>>>,
    neuro_runtime_init_gate: AsyncMutex<()>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_session: Mutex::new(None),
            session_start_gate: AsyncMutex::new(()),
            next_session_id: AtomicU64::new(1),
            runtime_config: Mutex::new(RuntimeCodexConfig::default()),
            next_event_seq: Arc::new(AtomicU64::new(1)),
            next_terminal_id: AtomicU64::new(1),
            terminals: Mutex::new(HashMap::new()),
            #[cfg(feature = "native-codex-runtime")]
            native_codex_runtime: AsyncMutex::new(None),
            #[cfg(feature = "native-codex-runtime")]
            native_codex_runtime_init_gate: AsyncMutex::new(()),
            neuro_runtime_cache: AsyncMutex::new(HashMap::new()),
            neuro_runtime_init_gate: AsyncMutex::new(()),
        }
    }
}
fn lock_active_session(state: &AppState) -> Result<MutexGuard<'_, Option<ActiveSession>>, String> {
    state
        .active_session
        .lock()
        .map_err(|_| "active session lock poisoned".to_string())
}

fn lock_runtime_config(state: &AppState) -> Result<MutexGuard<'_, RuntimeCodexConfig>, String> {
    state
        .runtime_config
        .lock()
        .map_err(|_| "runtime config lock poisoned".to_string())
}

#[tauri::command]
async fn codex_native_runtime_diagnose(
    state: State<'_, AppState>,
) -> Result<codex_native_runtime::NativeCodexRuntimeDiagnoseResponse, String> {
    crate::codex_native_runtime::codex_native_runtime_diagnose_impl(state).await
}

#[tauri::command]
async fn neuro_runtime_diagnose(
    state: State<'_, AppState>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_types::RuntimeDiagnoseResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_runtime_diagnose_impl(state).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_search_objects(
    state: State<'_, AppState>,
    query: String,
    max_results: Option<u32>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<Vec<neuro_types::AdtObjectSummary>>, String> {
    Ok(
        match crate::neuro_runtime::neuro_search_objects_impl(state, query, max_results, server_id)
            .await
        {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_get_source(
    state: State<'_, AppState>,
    object_uri: String,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_types::AdtSourceResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_get_source_impl(state, object_uri, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_update_source(
    state: State<'_, AppState>,
    request: neuro_runtime::AdtUpdateSourceCommandRequest,
) -> Result<neuro_types::NeuroCommandResponse<neuro_types::AdtUpdateSourceResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_update_source_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_server_list(
    state: State<'_, AppState>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerListResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_list_impl(state).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_server_upsert(
    state: State<'_, AppState>,
    request: neuro_runtime::AdtServerUpsertRequest,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerRecord>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_upsert_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_server_remove(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerRemoveResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_remove_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_server_select(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerSelectResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_select_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_server_connect(
    state: State<'_, AppState>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerConnectResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_connect_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_list_packages(
    state: State<'_, AppState>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<Vec<neuro_runtime::AdtPackageSummary>>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_list_packages_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_list_namespaces(
    state: State<'_, AppState>,
    package_name: Option<String>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<Vec<neuro_runtime::AdtNamespaceSummary>>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_list_namespaces_impl(state, package_name, server_id)
            .await
        {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_explorer_state_get(
    state: State<'_, AppState>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtExplorerStateResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_explorer_state_get_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_explorer_state_patch(
    state: State<'_, AppState>,
    mut request: neuro_runtime::AdtExplorerStatePatchRequest,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtExplorerStateResponse>, String> {
    if request.server_id.is_none() {
        request.server_id = server_id;
    }

    Ok(
        match crate::neuro_runtime::neuro_adt_explorer_state_patch_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_list_objects(
    state: State<'_, AppState>,
    request: neuro_runtime::AdtListObjectsRequest,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtListObjectsResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_list_objects_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_adt_list_package_inventory(
    state: State<'_, AppState>,
    request: neuro_runtime::AdtPackageInventoryRequest,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtPackageInventoryResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_list_package_inventory_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_ws_request(
    state: State<'_, AppState>,
    request: neuro_types::WsDomainRequest,
) -> Result<
    neuro_types::NeuroCommandResponse<neuro_types::WsMessageEnvelope<serde_json::Value>>,
    String,
> {
    Ok(
        match crate::neuro_runtime::neuro_ws_request_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_list_tools(
    state: State<'_, AppState>,
) -> Result<neuro_types::NeuroCommandResponse<Vec<neuro_mcp::NeuroToolSpec>>, String> {
    Ok(
        match crate::neuro_runtime::neuro_list_tools_impl(state).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
async fn neuro_invoke_tool(
    state: State<'_, AppState>,
    tool_name: String,
    arguments: serde_json::Value,
) -> Result<neuro_types::NeuroCommandResponse<serde_json::Value>, String> {
    Ok(
        match crate::neuro_runtime::neuro_invoke_tool_impl(state, tool_name, arguments).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
fn codex_models_list(state: State<'_, AppState>) -> Result<CodexModelListResponse, String> {
    crate::command_runtime::codex_models_list_impl(state)
}
#[tauri::command]
async fn codex_wait_for_mcp_startup(
    state: State<'_, AppState>,
) -> Result<McpStartupWarmupResponse, String> {
    crate::command_runtime::codex_wait_for_mcp_startup_impl(state).await
}
#[tauri::command]
async fn codex_app_list(
    state: State<'_, AppState>,
    request: Option<AppListRequest>,
) -> Result<AppListResponse, String> {
    crate::command_runtime::codex_app_list_impl(
        state,
        request.unwrap_or(AppListRequest {
            cursor: None,
            limit: None,
            thread_id: None,
            force_refetch: false,
        }),
    )
    .await
}

#[tauri::command]
async fn codex_account_read(
    state: State<'_, AppState>,
    request: Option<AccountReadRequest>,
) -> Result<AccountReadResponse, String> {
    crate::command_runtime::codex_account_read_impl(state, request.unwrap_or_default()).await
}

#[tauri::command]
async fn codex_account_login_start(
    state: State<'_, AppState>,
    request: AccountLoginStartRequest,
) -> Result<AccountLoginStartResponse, String> {
    crate::command_runtime::codex_account_login_start_impl(state, request).await
}

#[tauri::command]
async fn codex_account_logout(state: State<'_, AppState>) -> Result<AccountLogoutResponse, String> {
    crate::command_runtime::codex_account_logout_impl(state).await
}

#[tauri::command]
async fn codex_account_rate_limits_read(
    state: State<'_, AppState>,
) -> Result<AccountRateLimitsReadResponse, String> {
    crate::command_runtime::codex_account_rate_limits_read_impl(state).await
}
#[tauri::command]
async fn codex_mcp_list(state: State<'_, AppState>) -> Result<McpServerListResponse, String> {
    crate::command_runtime::codex_mcp_list_impl(state).await
}

#[tauri::command]
async fn codex_mcp_login(
    state: State<'_, AppState>,
    request: McpLoginRequest,
) -> Result<McpLoginResponse, String> {
    crate::command_runtime::codex_mcp_login_impl(state, request).await
}

#[tauri::command]
async fn codex_mcp_reload(state: State<'_, AppState>) -> Result<McpReloadResponse, String> {
    crate::command_runtime::codex_mcp_reload_impl(state).await
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            spawn_neuro_startup_probe(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_codex_session,
            codex_turn_run,
            codex_thread_open,
            codex_thread_close,
            codex_thread_list,
            codex_thread_read,
            codex_thread_archive,
            codex_thread_unarchive,
            codex_thread_compact_start,
            codex_thread_rollback,
            codex_thread_fork,
            codex_review_start,
            codex_turn_steer,
            codex_turn_interrupt,
            codex_approval_respond,
            codex_user_input_respond,
            update_codex_config,
            codex_config_get,
            codex_config_set,
            codex_runtime_status,
            codex_runtime_capabilities,
            codex_native_runtime_diagnose,
            neuro_runtime_diagnose,
            neuro_search_objects,
            neuro_get_source,
            neuro_update_source,
            neuro_adt_server_list,
            neuro_adt_server_upsert,
            neuro_adt_server_remove,
            neuro_adt_server_select,
            neuro_adt_server_connect,
            neuro_adt_list_packages,
            neuro_adt_list_namespaces,
            neuro_adt_explorer_state_get,
            neuro_adt_explorer_state_patch,
            neuro_adt_list_objects,
            neuro_adt_list_package_inventory,
            neuro_ws_request,
            neuro_list_tools,
            neuro_invoke_tool,
            load_codex_default_config,
            send_codex_input,
            stop_codex_session,
            resize_codex_pty,
            terminal_create,
            terminal_write,
            terminal_resize,
            terminal_kill,
            run_codex_command,
            git_commit_approved_review,
            git_workspace_changes,
            codex_workspace_read_file,
            codex_workspace_write_file,
            codex_workspace_create_directory,
            codex_workspace_list_directory,
            codex_workspace_rename_entry,
            codex_models_list,
            codex_app_list,
            codex_account_read,
            codex_account_login_start,
            codex_account_logout,
            codex_account_rate_limits_read,
            codex_mcp_list,
            codex_mcp_login,
            codex_mcp_reload,
            codex_wait_for_mcp_startup,
            pick_workspace_folder,
            pick_image_file,
            pick_mention_file,
            codex_help_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
