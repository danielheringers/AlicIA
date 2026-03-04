use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Manager};
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
use crate::interface::tauri::commands::{
    account_mcp::{
        codex_account_login_start, codex_account_logout, codex_account_rate_limits_read,
        codex_account_read, codex_app_list, codex_mcp_list, codex_mcp_login, codex_mcp_reload,
        codex_wait_for_mcp_startup,
    },
    models::codex_models_list,
    native_runtime::codex_native_runtime_diagnose,
    neuro::{
        neuro_adt_explorer_state_get, neuro_adt_explorer_state_patch, neuro_adt_list_namespaces,
        neuro_adt_list_objects, neuro_adt_list_package_inventory, neuro_adt_list_packages,
        neuro_adt_server_connect, neuro_adt_server_list, neuro_adt_server_remove,
        neuro_adt_server_select, neuro_adt_server_upsert, neuro_get_source, neuro_invoke_tool,
        neuro_list_tools, neuro_runtime_diagnose, neuro_search_objects, neuro_update_source,
        neuro_ws_request,
    },
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
