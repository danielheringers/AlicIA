#[cfg(feature = "native-codex-runtime")]
use std::path::Path;
#[cfg(feature = "native-codex-runtime")]
use std::sync::Arc;

#[cfg(feature = "native-codex-runtime")]
use codex_core::config::{Config, ConfigOverrides};
#[cfg(feature = "native-codex-runtime")]
use codex_core::error::CodexErr;
#[cfg(feature = "native-codex-runtime")]
use codex_core::{find_thread_path_by_id_str, CodexThread};
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::ThreadId;
#[cfg(feature = "native-codex-runtime")]
use tauri::{AppHandle, Manager, State};

#[cfg(feature = "native-codex-runtime")]
use crate::infrastructure::runtime_bridge::session_thread_shared;
#[cfg(feature = "native-codex-runtime")]
use crate::{lock_active_session, AppState, RuntimeCodexConfig};

#[cfg(feature = "native-codex-runtime")]
pub(crate) type RuntimeConfigHarnessOverridesFn = fn(&RuntimeCodexConfig, &Path) -> ConfigOverrides;

#[cfg(feature = "native-codex-runtime")]
pub(crate) type ApplyRuntimeConfigBootstrapOverridesFn =
    fn(&mut Config, &RuntimeCodexConfig) -> Result<(), String>;

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn normalize_thread_id(thread_id: Option<String>) -> Option<String> {
    thread_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn resolve_or_create_thread_for_session(
    app: &AppHandle,
    session_id: u64,
    requested_thread_id: Option<String>,
    cwd: &Path,
    create_thread_runtime_config: Option<RuntimeCodexConfig>,
    runtime_config_harness_overrides: RuntimeConfigHarnessOverridesFn,
    apply_runtime_config_bootstrap_overrides: ApplyRuntimeConfigBootstrapOverridesFn,
) -> Result<(String, Arc<CodexThread>, bool), String> {
    let requested_thread_id = normalize_thread_id(requested_thread_id);
    let bootstrap_runtime_config = create_thread_runtime_config.as_ref();
    let (runtime, known_thread_id, known_thread) = {
        let state = app.state::<AppState>();
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;
        if active.session_id != session_id {
            return Err("active codex session changed while resolving thread".to_string());
        }

        let known_thread_id = requested_thread_id
            .clone()
            .or_else(|| normalize_thread_id(active.thread_id.clone()));

        let crate::ActiveSessionTransport::Native(native) = &mut active.transport;

        let known_thread = known_thread_id
            .as_ref()
            .and_then(|thread_id| native.threads.get(thread_id).cloned());

        (Arc::clone(&native.runtime), known_thread_id, known_thread)
    };

    if let Some(thread) = known_thread {
        if let Some(thread_id) = known_thread_id {
            return Ok((thread_id, thread, false));
        }
    }

    if let Some(thread_id) = known_thread_id {
        let parsed = ThreadId::from_string(&thread_id)
            .map_err(|error| format!("invalid thread id `{thread_id}`: {error}"))?;
        let (resolved_thread_id, thread) = match runtime.thread_manager.get_thread(parsed).await {
            Ok(thread) => (thread_id.clone(), thread),
            Err(CodexErr::ThreadNotFound(_)) => {
                let rollout_path =
                    find_thread_path_by_id_str(runtime.codex_home.as_path(), &thread_id)
                        .await
                        .map_err(|error| {
                            format!("failed to locate thread id `{thread_id}`: {error}")
                        })?
                        .ok_or_else(|| format!("no rollout found for thread id `{thread_id}`"))?;

                let mut config_builder =
                    session_thread_shared::native_config_builder(runtime.codex_home.clone(), cwd);
                if let Some(runtime_config) = bootstrap_runtime_config {
                    config_builder = config_builder
                        .harness_overrides(runtime_config_harness_overrides(runtime_config, cwd));
                } else {
                    config_builder = config_builder.harness_overrides(
                        session_thread_shared::native_profile_harness_overrides(cwd),
                    );
                }
                let mut config = config_builder
                    .build()
                    .await
                    .map_err(|error| format!("failed to build native thread config: {error}"))?;
                if let Some(runtime_config) = bootstrap_runtime_config {
                    apply_runtime_config_bootstrap_overrides(&mut config, runtime_config)?;
                }
                let resumed = runtime
                    .thread_manager
                    .resume_thread_from_rollout(
                        config,
                        rollout_path,
                        Arc::clone(&runtime.auth_manager),
                    )
                    .await
                    .map_err(|error| format!("failed to load thread `{thread_id}`: {error}"))?;

                (resumed.thread_id.to_string(), resumed.thread)
            }
            Err(error) => {
                return Err(format!("failed to load thread `{thread_id}`: {error}"));
            }
        };

        let state = app.state::<AppState>();
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;
        if active.session_id != session_id {
            return Err("active codex session changed while loading native thread".to_string());
        }
        let crate::ActiveSessionTransport::Native(native) = &mut active.transport;
        native
            .threads
            .insert(resolved_thread_id.clone(), Arc::clone(&thread));
        if resolved_thread_id != thread_id {
            native
                .threads
                .insert(thread_id.clone(), Arc::clone(&thread));
        }
        return Ok((resolved_thread_id, thread, false));
    }

    let mut config_builder =
        session_thread_shared::native_config_builder(runtime.codex_home.clone(), cwd);
    if let Some(runtime_config) = bootstrap_runtime_config {
        config_builder =
            config_builder.harness_overrides(runtime_config_harness_overrides(runtime_config, cwd));
    } else {
        config_builder = config_builder
            .harness_overrides(session_thread_shared::native_profile_harness_overrides(cwd));
    }
    let mut config = config_builder
        .build()
        .await
        .map_err(|error| format!("failed to build native thread config: {error}"))?;
    if let Some(runtime_config) = bootstrap_runtime_config {
        apply_runtime_config_bootstrap_overrides(&mut config, runtime_config)?;
    }
    let created = runtime
        .thread_manager
        .start_thread(config)
        .await
        .map_err(|error| format!("failed to start native thread: {error}"))?;
    let thread_id = created.thread_id.to_string();
    let thread = Arc::clone(&created.thread);

    let state = app.state::<AppState>();
    let mut guard = lock_active_session(state.inner())?;
    let active = guard
        .as_mut()
        .ok_or_else(|| "no active codex session".to_string())?;
    if active.session_id != session_id {
        return Err("active codex session changed while creating native thread".to_string());
    }
    let crate::ActiveSessionTransport::Native(native) = &mut active.transport;
    native
        .threads
        .insert(thread_id.clone(), Arc::clone(&thread));

    Ok((thread_id, thread, true))
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn load_thread_from_active_session(
    state: &State<'_, AppState>,
    thread_id: &str,
) -> Result<Arc<CodexThread>, String> {
    let normalized_thread_id = thread_id.trim();
    if normalized_thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    let (session_id, runtime, known_thread, session_cwd) = {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;
        let session_id = active.session_id;

        let crate::ActiveSessionTransport::Native(native) = &mut active.transport;

        (
            session_id,
            Arc::clone(&native.runtime),
            native.threads.get(normalized_thread_id).cloned(),
            active.cwd.clone(),
        )
    };

    if let Some(thread) = known_thread {
        return Ok(thread);
    }

    let parsed_thread_id = ThreadId::from_string(normalized_thread_id)
        .map_err(|error| format!("invalid thread id `{normalized_thread_id}`: {error}"))?;
    let (resolved_thread_id, thread) = match runtime
        .thread_manager
        .get_thread(parsed_thread_id)
        .await
    {
        Ok(thread) => (normalized_thread_id.to_string(), thread),
        Err(CodexErr::ThreadNotFound(_)) => {
            let rollout_path =
                find_thread_path_by_id_str(runtime.codex_home.as_path(), normalized_thread_id)
                    .await
                    .map_err(|error| {
                        format!("failed to locate thread id `{normalized_thread_id}`: {error}")
                    })?
                    .ok_or_else(|| {
                        format!("no rollout found for thread id `{normalized_thread_id}`")
                    })?;

            let config = session_thread_shared::native_config_builder(
                runtime.codex_home.clone(),
                session_cwd.as_path(),
            )
            .harness_overrides(session_thread_shared::native_profile_harness_overrides(
                session_cwd.as_path(),
            ))
            .build()
            .await
            .map_err(|error| format!("failed to build native thread config: {error}"))?;
            let resumed = runtime
                .thread_manager
                .resume_thread_from_rollout(config, rollout_path, Arc::clone(&runtime.auth_manager))
                .await
                .map_err(|error| {
                    format!("failed to load thread `{normalized_thread_id}`: {error}")
                })?;

            (resumed.thread_id.to_string(), resumed.thread)
        }
        Err(error) => {
            return Err(format!(
                "failed to load thread `{normalized_thread_id}`: {error}"
            ));
        }
    };

    let mut guard = lock_active_session(state.inner())?;
    let active = guard
        .as_mut()
        .ok_or_else(|| "no active codex session".to_string())?;
    if active.session_id != session_id {
        return Err("active codex session changed while loading native thread".to_string());
    }

    let crate::ActiveSessionTransport::Native(native) = &mut active.transport;
    native
        .threads
        .insert(resolved_thread_id.clone(), Arc::clone(&thread));
    if resolved_thread_id != normalized_thread_id {
        native
            .threads
            .insert(normalized_thread_id.to_string(), Arc::clone(&thread));
    }

    Ok(thread)
}
