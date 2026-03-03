use std::env;
use std::path::PathBuf;
use tauri::State;

use crate::config_runtime::{load_runtime_config_from_codex, normalize_runtime_config};
use crate::interface::tauri::dto::{
    RuntimeCapabilitiesResponse, RuntimeCodexConfig, RuntimeStatusResponse,
};
use crate::{lock_active_session, lock_runtime_config, AppState};

#[tauri::command]
pub fn codex_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatusResponse, String> {
    let (session_id, pid, session_workspace) = {
        let active = lock_active_session(state.inner())?;
        (
            active.as_ref().map(|session| session.session_id),
            active.as_ref().and_then(|session| session.pid),
            active.as_ref().map(|session| session.cwd.clone()),
        )
    };

    let runtime_config = lock_runtime_config(state.inner())?.clone();
    let process_cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let workspace = crate::workspace_runtime::resolve_runtime_status_workspace(
        session_workspace.as_deref(),
        process_cwd.as_path(),
    );

    Ok(RuntimeStatusResponse {
        session_id,
        pid,
        workspace,
        runtime_config,
    })
}

#[tauri::command]
pub async fn codex_runtime_capabilities(
    state: State<'_, AppState>,
) -> Result<RuntimeCapabilitiesResponse, String> {
    crate::command_runtime::codex_runtime_capabilities_impl(state).await
}

#[tauri::command]
pub async fn load_codex_default_config(
    state: State<'_, AppState>,
) -> Result<RuntimeCodexConfig, String> {
    let loaded = load_runtime_config_from_codex().await?;
    let mut runtime = lock_runtime_config(state.inner())?;
    *runtime = loaded.clone();
    Ok(loaded)
}

#[tauri::command]
pub fn update_codex_config(
    state: State<'_, AppState>,
    config: RuntimeCodexConfig,
) -> Result<RuntimeCodexConfig, String> {
    let mut runtime = lock_runtime_config(state.inner())?;
    *runtime = normalize_runtime_config(config);
    Ok(runtime.clone())
}

#[tauri::command]
pub fn codex_config_get(state: State<'_, AppState>) -> Result<RuntimeCodexConfig, String> {
    Ok(lock_runtime_config(state.inner())?.clone())
}

#[tauri::command]
pub fn codex_config_set(
    state: State<'_, AppState>,
    patch: RuntimeCodexConfig,
) -> Result<RuntimeCodexConfig, String> {
    let mut runtime = lock_runtime_config(state.inner())?;
    *runtime = normalize_runtime_config(patch);
    Ok(runtime.clone())
}
