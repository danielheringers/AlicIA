use tauri::State;

use crate::account_runtime::{
    AccountLoginStartRequest, AccountLoginStartResponse, AccountLogoutResponse,
    AccountRateLimitsReadResponse, AccountReadRequest, AccountReadResponse, AppListRequest,
    AppListResponse,
};
use crate::mcp_runtime::{
    McpLoginRequest, McpLoginResponse, McpReloadResponse, McpServerListResponse,
    McpStartupWarmupResponse,
};
use crate::AppState;

#[tauri::command]
pub async fn codex_wait_for_mcp_startup(
    state: State<'_, AppState>,
) -> Result<McpStartupWarmupResponse, String> {
    crate::command_runtime::codex_wait_for_mcp_startup_impl(state).await
}

#[tauri::command]
pub async fn codex_app_list(
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
pub async fn codex_account_read(
    state: State<'_, AppState>,
    request: Option<AccountReadRequest>,
) -> Result<AccountReadResponse, String> {
    crate::command_runtime::codex_account_read_impl(state, request.unwrap_or_default()).await
}

#[tauri::command]
pub async fn codex_account_login_start(
    state: State<'_, AppState>,
    request: AccountLoginStartRequest,
) -> Result<AccountLoginStartResponse, String> {
    crate::command_runtime::codex_account_login_start_impl(state, request).await
}

#[tauri::command]
pub async fn codex_account_logout(
    state: State<'_, AppState>,
) -> Result<AccountLogoutResponse, String> {
    crate::command_runtime::codex_account_logout_impl(state).await
}

#[tauri::command]
pub async fn codex_account_rate_limits_read(
    state: State<'_, AppState>,
) -> Result<AccountRateLimitsReadResponse, String> {
    crate::command_runtime::codex_account_rate_limits_read_impl(state).await
}

#[tauri::command]
pub async fn codex_mcp_list(state: State<'_, AppState>) -> Result<McpServerListResponse, String> {
    crate::command_runtime::codex_mcp_list_impl(state).await
}

#[tauri::command]
pub async fn codex_mcp_login(
    state: State<'_, AppState>,
    request: McpLoginRequest,
) -> Result<McpLoginResponse, String> {
    crate::command_runtime::codex_mcp_login_impl(state, request).await
}

#[tauri::command]
pub async fn codex_mcp_reload(state: State<'_, AppState>) -> Result<McpReloadResponse, String> {
    crate::command_runtime::codex_mcp_reload_impl(state).await
}
