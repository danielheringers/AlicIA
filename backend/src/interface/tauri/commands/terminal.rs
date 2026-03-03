use tauri::{AppHandle, State};

use crate::interface::tauri::dto::{
    TerminalCreateRequest, TerminalCreateResponse, TerminalKillRequest, TerminalResizeRequest,
    TerminalWriteRequest,
};
use crate::AppState;

#[tauri::command]
pub fn terminal_create(
    app: AppHandle,
    state: State<'_, AppState>,
    request: Option<TerminalCreateRequest>,
) -> Result<TerminalCreateResponse, String> {
    crate::terminal_runtime::terminal_create_impl(app, state, request)
}

#[tauri::command]
pub fn terminal_write(
    state: State<'_, AppState>,
    request: TerminalWriteRequest,
) -> Result<(), String> {
    crate::terminal_runtime::terminal_write_impl(state, request)
}

#[tauri::command]
pub fn terminal_resize(
    state: State<'_, AppState>,
    request: TerminalResizeRequest,
) -> Result<(), String> {
    crate::terminal_runtime::terminal_resize_impl(state, request)
}

#[tauri::command]
pub fn terminal_kill(
    app: AppHandle,
    state: State<'_, AppState>,
    request: TerminalKillRequest,
) -> Result<(), String> {
    crate::terminal_runtime::terminal_kill_impl(app, state, request)
}
