use tauri::{AppHandle, State};

use crate::interface::tauri::dto::{StartCodexSessionConfig, StartCodexSessionResponse};
use crate::AppState;

#[tauri::command]
pub async fn start_codex_session(
    app: AppHandle,
    state: State<'_, AppState>,
    config: Option<StartCodexSessionConfig>,
) -> Result<StartCodexSessionResponse, String> {
    crate::session_runtime::start_codex_session_impl(app, state, config).await
}

#[tauri::command]
pub fn resize_codex_pty(state: State<'_, AppState>, _rows: u16, _cols: u16) -> Result<(), String> {
    crate::session_runtime::resize_codex_pty_impl(state, _rows, _cols)
}

#[tauri::command]
pub async fn stop_codex_session(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    crate::session_runtime::stop_codex_session_impl(app, state).await
}
