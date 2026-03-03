use tauri::{AppHandle, State};

use crate::interface::tauri::dto::{
    CodexApprovalRespondRequest, CodexReviewStartRequest, CodexReviewStartResponse,
    CodexThreadArchiveRequest, CodexThreadArchiveResponse, CodexThreadCloseRequest,
    CodexThreadCloseResponse, CodexThreadCompactStartRequest, CodexThreadCompactStartResponse,
    CodexThreadForkRequest, CodexThreadForkResponse, CodexThreadListRequest,
    CodexThreadListResponse, CodexThreadOpenResponse, CodexThreadReadRequest,
    CodexThreadReadResponse, CodexThreadRollbackRequest, CodexThreadRollbackResponse,
    CodexThreadUnarchiveRequest, CodexThreadUnarchiveResponse, CodexTurnInterruptRequest,
    CodexTurnInterruptResponse, CodexTurnRunRequest, CodexTurnRunResponse, CodexTurnSteerRequest,
    CodexTurnSteerResponse, CodexUserInputRespondRequest, CodexUserInputRespondResponse,
};
use crate::AppState;

#[tauri::command]
pub async fn codex_turn_run(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexTurnRunRequest,
) -> Result<CodexTurnRunResponse, String> {
    crate::session_runtime::codex_turn_run_impl(app, state, request).await
}

#[tauri::command]
pub async fn codex_thread_open(
    app: AppHandle,
    state: State<'_, AppState>,
    thread_id: Option<String>,
) -> Result<CodexThreadOpenResponse, String> {
    crate::session_runtime::codex_thread_open_impl(app, state, thread_id).await
}

#[tauri::command]
pub async fn codex_thread_close(
    state: State<'_, AppState>,
    request: CodexThreadCloseRequest,
) -> Result<CodexThreadCloseResponse, String> {
    crate::session_runtime::codex_thread_close_impl(state, request).await
}

#[tauri::command]
pub async fn codex_thread_list(
    state: State<'_, AppState>,
    request: Option<CodexThreadListRequest>,
) -> Result<CodexThreadListResponse, String> {
    crate::session_runtime::codex_thread_list_impl(state, request.unwrap_or_default()).await
}

#[tauri::command]
pub async fn codex_thread_read(
    state: State<'_, AppState>,
    request: CodexThreadReadRequest,
) -> Result<CodexThreadReadResponse, String> {
    crate::session_runtime::codex_thread_read_impl(state, request).await
}

#[tauri::command]
pub async fn codex_thread_archive(
    state: State<'_, AppState>,
    request: CodexThreadArchiveRequest,
) -> Result<CodexThreadArchiveResponse, String> {
    crate::session_runtime::codex_thread_archive_impl(state, request).await
}

#[tauri::command]
pub async fn codex_thread_unarchive(
    state: State<'_, AppState>,
    request: CodexThreadUnarchiveRequest,
) -> Result<CodexThreadUnarchiveResponse, String> {
    crate::session_runtime::codex_thread_unarchive_impl(state, request).await
}

#[tauri::command]
pub async fn codex_thread_compact_start(
    state: State<'_, AppState>,
    request: CodexThreadCompactStartRequest,
) -> Result<CodexThreadCompactStartResponse, String> {
    crate::session_runtime::codex_thread_compact_start_impl(state, request).await
}

#[tauri::command]
pub async fn codex_thread_rollback(
    state: State<'_, AppState>,
    request: CodexThreadRollbackRequest,
) -> Result<CodexThreadRollbackResponse, String> {
    crate::session_runtime::codex_thread_rollback_impl(state, request).await
}

#[tauri::command]
pub async fn codex_thread_fork(
    state: State<'_, AppState>,
    request: CodexThreadForkRequest,
) -> Result<CodexThreadForkResponse, String> {
    crate::session_runtime::codex_thread_fork_impl(state, request).await
}

#[tauri::command]
pub async fn codex_turn_steer(
    state: State<'_, AppState>,
    request: CodexTurnSteerRequest,
) -> Result<CodexTurnSteerResponse, String> {
    crate::session_runtime::codex_turn_steer_impl(state, request).await
}

#[tauri::command]
pub async fn codex_turn_interrupt(
    state: State<'_, AppState>,
    request: CodexTurnInterruptRequest,
) -> Result<CodexTurnInterruptResponse, String> {
    crate::session_runtime::codex_turn_interrupt_impl(state, request).await
}

#[tauri::command]
pub async fn codex_review_start(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexReviewStartRequest,
) -> Result<CodexReviewStartResponse, String> {
    crate::session_runtime::codex_review_start_impl(app, state, request).await
}

#[tauri::command]
pub async fn codex_approval_respond(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexApprovalRespondRequest,
) -> Result<(), String> {
    crate::session_runtime::codex_approval_respond_impl(app, state, request).await
}

#[tauri::command]
pub async fn codex_user_input_respond(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexUserInputRespondRequest,
) -> Result<CodexUserInputRespondResponse, String> {
    crate::session_runtime::codex_user_input_respond_impl(app, state, request).await
}

#[tauri::command]
pub async fn send_codex_input(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
) -> Result<(), String> {
    crate::session_runtime::send_codex_input_impl(app, state, text).await
}
