use tauri::State;

use crate::interface::tauri::dto::CodexModelListResponse;
use crate::AppState;

#[tauri::command]
pub fn codex_models_list(state: State<'_, AppState>) -> Result<CodexModelListResponse, String> {
    crate::command_runtime::codex_models_list_impl(state)
}
