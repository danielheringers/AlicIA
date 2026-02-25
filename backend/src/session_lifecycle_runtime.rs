use std::env;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

use crate::{
    default_codex_binary, emit_lifecycle, lock_active_session, resolve_codex_launch,
    ActiveSessionTransport, AppState, StartCodexSessionConfig, StartCodexSessionResponse,
};

fn binary_for_launch(program: &str, args: &[String]) -> String {
    if program.eq_ignore_ascii_case("cmd") && args.len() >= 2 && args[0].eq_ignore_ascii_case("/c")
    {
        return args[1].clone();
    }

    if let Some(first_arg) = args.first() {
        let lowered = first_arg.to_ascii_lowercase();
        if lowered.ends_with(".js") || lowered.ends_with(".mjs") || lowered.ends_with(".cjs") {
            return first_arg.clone();
        }
    }

    program.to_string()
}

#[cfg(feature = "native-codex-runtime")]
async fn start_native_session(
    app: &AppHandle,
    state: &State<'_, AppState>,
    session_id: u64,
    cwd: PathBuf,
    binary: String,
) -> Result<StartCodexSessionResponse, String> {
    let runtime = crate::codex_native_runtime::native_runtime_get_or_init(state.inner()).await?;
    let pid = std::process::id();

    {
        let mut guard = lock_active_session(state.inner())?;
        *guard = Some(crate::ActiveSession {
            session_id,
            pid: Some(pid),
            binary,
            cwd,
            thread_id: None,
            busy: false,
            transport: ActiveSessionTransport::Native(crate::NativeSessionHandles {
                runtime,
                threads: std::collections::HashMap::new(),
                active_turns: std::collections::HashMap::new(),
                pending_approvals: std::collections::HashMap::new(),
                pending_user_inputs: std::collections::HashMap::new(),
                next_approval_id: 1,
                next_user_input_id: 1,
            }),
        });
    }

    emit_lifecycle(app, "started", Some(session_id), Some(pid), None, None);
    Ok(StartCodexSessionResponse { session_id, pid })
}

async fn start_codex_session_internal(
    app: AppHandle,
    state: State<'_, AppState>,
    config: Option<StartCodexSessionConfig>,
) -> Result<StartCodexSessionResponse, String> {
    let _start_gate = state.session_start_gate.lock().await;

    {
        let active_guard = lock_active_session(state.inner())?;
        if active_guard.is_some() {
            return Err("an active codex session is already running".to_string());
        }
    }

    let config = config.unwrap_or_default();
    if config.args.as_ref().is_some_and(|args| !args.is_empty()) {
        return Err("custom start args are not supported".to_string());
    }
    if config
        .env
        .as_ref()
        .is_some_and(|env_overrides| !env_overrides.is_empty())
    {
        return Err("custom session env is not supported by native runtime".to_string());
    }

    let configured_binary = config
        .binary
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(default_codex_binary);

    let (launch_program, launch_args) = resolve_codex_launch(&configured_binary, &[])?;
    let binary = binary_for_launch(&launch_program, &launch_args);

    let cwd = config
        .cwd
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    if !cwd.exists() {
        return Err(format!("session cwd does not exist: {}", cwd.display()));
    }

    let session_id = state.next_session_id.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "native-codex-runtime")]
    return start_native_session(&app, &state, session_id, cwd, binary).await;

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = (&app, &state, session_id, &cwd, &binary);
        Err(
            "native runtime feature is disabled in this build; enable `native-codex-runtime`"
                .to_string(),
        )
    }
}

pub(crate) async fn start_codex_session_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    config: Option<StartCodexSessionConfig>,
) -> Result<StartCodexSessionResponse, String> {
    start_codex_session_internal(app, state, config).await
}

pub(crate) fn resize_codex_pty_impl(
    state: State<'_, AppState>,
    _rows: u16,
    _cols: u16,
) -> Result<(), String> {
    let guard = lock_active_session(state.inner())?;
    if guard.is_none() {
        return Err("no active session".to_string());
    }

    Ok(())
}

pub(crate) async fn stop_codex_session_impl(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _start_gate = state.session_start_gate.lock().await;

    let active_session = {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_ref()
            .ok_or_else(|| "no active codex session".to_string())?;
        if active.busy {
            return Err("cannot stop session while a turn is still running".to_string());
        }
        guard
            .take()
            .ok_or_else(|| "no active codex session".to_string())?
    };

    let crate::ActiveSession {
        session_id,
        pid,
        binary,
        cwd,
        thread_id,
        busy,
        transport,
    } = active_session;

    #[cfg(not(feature = "native-codex-runtime"))]
    let _ = (&binary, &cwd, &thread_id, &busy);

    match transport {
        ActiveSessionTransport::Native(handles) => {
            if handles
                .runtime
                .thread_manager
                .remove_and_close_all_threads()
                .await
                .is_err()
            {
                let mut guard = lock_active_session(state.inner())?;
                *guard = Some(crate::ActiveSession {
                    session_id,
                    pid,
                    binary,
                    cwd,
                    thread_id,
                    busy,
                    transport: ActiveSessionTransport::Native(handles),
                });
                return Err("failed to stop native codex session".to_string());
            }
        }
    }

    emit_lifecycle(
        &app,
        "stopped",
        Some(session_id),
        pid,
        None,
        Some("stopped by request".to_string()),
    );

    Ok(())
}
