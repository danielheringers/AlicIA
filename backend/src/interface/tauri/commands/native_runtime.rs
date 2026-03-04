use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn codex_native_runtime_diagnose(
    state: State<'_, AppState>,
) -> Result<crate::codex_native_runtime::NativeCodexRuntimeDiagnoseResponse, String> {
    crate::codex_native_runtime::codex_native_runtime_diagnose_impl(state).await
}
