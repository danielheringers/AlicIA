use serde_json::{json, Value};
#[cfg(feature = "native-codex-runtime")]
use std::fs::{FileTimes, OpenOptions};
#[cfg(feature = "native-codex-runtime")]
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(feature = "native-codex-runtime")]
use std::time::SystemTime;
use tauri::{AppHandle, Manager, State};

#[cfg(feature = "native-codex-runtime")]
use codex_core::config::{ConfigBuilder, ConfigOverrides};
#[cfg(feature = "native-codex-runtime")]
use codex_core::error::CodexErr;
#[cfg(feature = "native-codex-runtime")]
use codex_core::CodexThread;
#[cfg(feature = "native-codex-runtime")]
use codex_core::SteerInputError;
#[cfg(feature = "native-codex-runtime")]
use codex_core::{
    find_archived_thread_path_by_id_str, find_thread_path_by_id_str, rollout_date_parts,
    ARCHIVED_SESSIONS_SUBDIR, SESSIONS_SUBDIR,
};
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::config_types::SandboxMode as SandboxModeConfig;
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::config_types::WebSearchMode as WebSearchModeConfig;
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::protocol::{
    AskForApproval, EventMsg, Op, ReviewRequest, ReviewTarget, SandboxPolicy,
};
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::user_input::UserInput;
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::ThreadId;

use crate::application::session_thread_review::use_cases as session_thread_review_use_cases;
#[cfg(feature = "native-codex-runtime")]
use crate::codex_event_translator::NativeCodexEventTranslator;
#[cfg(feature = "native-codex-runtime")]
use crate::emit_codex_event;
#[cfg(feature = "native-codex-runtime")]
use crate::infrastructure::runtime_bridge::session_thread_shared;
#[cfg(feature = "native-codex-runtime")]
use crate::interface::tauri::dto::CodexThreadSummary;
use crate::interface::tauri::dto::{
    CodexApprovalRespondRequest, CodexInputItem, CodexReviewStartRequest, CodexReviewStartResponse,
    CodexThreadArchiveRequest, CodexThreadArchiveResponse, CodexThreadCloseRequest,
    CodexThreadCloseResponse, CodexThreadCompactStartRequest, CodexThreadCompactStartResponse,
    CodexThreadForkRequest, CodexThreadForkResponse, CodexThreadListRequest,
    CodexThreadListResponse, CodexThreadOpenResponse, CodexThreadReadRequest,
    CodexThreadReadResponse, CodexThreadRollbackRequest, CodexThreadRollbackResponse,
    CodexThreadUnarchiveRequest, CodexThreadUnarchiveResponse, CodexTurnInterruptRequest,
    CodexTurnInterruptResponse, CodexTurnRunRequest, CodexTurnRunResponse, CodexTurnSteerRequest,
    CodexTurnSteerResponse, CodexUserInputRespondRequest, CodexUserInputRespondResponse,
};
use crate::status_runtime::{fetch_rate_limits_for_status, format_non_tui_status};
use crate::{
    emit_lifecycle, emit_stderr, emit_stdout, lock_active_session, lock_runtime_config, AppState,
    RuntimeCodexConfig,
};

#[cfg(feature = "native-codex-runtime")]
fn normalize_runtime_thread_id(thread_id: Option<String>) -> Option<String> {
    thread_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(feature = "native-codex-runtime")]
fn runtime_approval_policy(policy: &str) -> AskForApproval {
    match policy.trim().to_ascii_lowercase().as_str() {
        "untrusted" => AskForApproval::UnlessTrusted,
        "on-failure" => AskForApproval::OnFailure,
        "never" => AskForApproval::Never,
        _ => AskForApproval::OnRequest,
    }
}

#[cfg(feature = "native-codex-runtime")]
fn runtime_sandbox_policy(policy: &str) -> SandboxPolicy {
    match policy.trim().to_ascii_lowercase().as_str() {
        "danger-full-access" => SandboxPolicy::DangerFullAccess,
        "workspace-write" => SandboxPolicy::new_workspace_write_policy(),
        _ => SandboxPolicy::new_read_only_policy(),
    }
}

#[cfg(feature = "native-codex-runtime")]
fn runtime_sandbox_mode(policy: &str) -> SandboxModeConfig {
    match policy.trim().to_ascii_lowercase().as_str() {
        "danger-full-access" => SandboxModeConfig::DangerFullAccess,
        "workspace-write" => SandboxModeConfig::WorkspaceWrite,
        _ => SandboxModeConfig::ReadOnly,
    }
}

#[cfg(feature = "native-codex-runtime")]
fn runtime_model_override(model: &str) -> Option<String> {
    let normalized = model.trim();
    if normalized.is_empty() || normalized.eq_ignore_ascii_case("default") {
        None
    } else {
        Some(normalized.to_string())
    }
}

#[cfg(feature = "native-codex-runtime")]
const ALICIA_NATIVE_INTERNAL_PROFILE: &str = session_thread_shared::ALICIA_NATIVE_INTERNAL_PROFILE;

#[cfg(feature = "native-codex-runtime")]
fn native_internal_profile_cli_overrides() -> Vec<(String, toml::Value)> {
    session_thread_shared::native_internal_profile_cli_overrides()
}

#[cfg(feature = "native-codex-runtime")]
fn runtime_profile_override(profile: &str) -> Option<String> {
    let normalized = profile.trim();
    if normalized.is_empty() {
        return None;
    }

    let lowered = normalized.to_ascii_lowercase();
    if matches!(
        lowered.as_str(),
        "read_only"
            | "read-only"
            | "read_write_with_approval"
            | "read-write-with-approval"
            | "full_access"
            | "full-access"
    ) {
        return None;
    }

    Some(normalized.to_string())
}

#[cfg(feature = "native-codex-runtime")]
fn runtime_profile_or_internal(profile: &str) -> String {
    runtime_profile_override(profile).unwrap_or_else(|| ALICIA_NATIVE_INTERNAL_PROFILE.to_string())
}

#[cfg(feature = "native-codex-runtime")]
fn native_profile_harness_overrides(cwd: &Path) -> ConfigOverrides {
    session_thread_shared::native_profile_harness_overrides(cwd)
}

#[cfg(feature = "native-codex-runtime")]
fn native_config_builder(codex_home: PathBuf, cwd: &Path) -> ConfigBuilder {
    session_thread_shared::native_config_builder(codex_home, cwd)
}

#[cfg(feature = "native-codex-runtime")]
fn runtime_config_harness_overrides(
    runtime_config: &RuntimeCodexConfig,
    cwd: &Path,
) -> ConfigOverrides {
    ConfigOverrides {
        model: runtime_model_override(runtime_config.model.as_str()),
        cwd: Some(cwd.to_path_buf()),
        approval_policy: Some(runtime_approval_policy(
            runtime_config.approval_policy.as_str(),
        )),
        sandbox_mode: Some(runtime_sandbox_mode(runtime_config.sandbox.as_str())),
        config_profile: Some(runtime_profile_or_internal(runtime_config.profile.as_str())),
        ..Default::default()
    }
}

#[cfg(feature = "native-codex-runtime")]
fn runtime_reasoning_effort(effort: &str) -> Option<ReasoningEffortConfig> {
    match effort.trim().to_ascii_lowercase().as_str() {
        "none" => Some(ReasoningEffortConfig::None),
        "minimal" => Some(ReasoningEffortConfig::Minimal),
        "low" => Some(ReasoningEffortConfig::Low),
        "medium" => Some(ReasoningEffortConfig::Medium),
        "high" => Some(ReasoningEffortConfig::High),
        "xhigh" => Some(ReasoningEffortConfig::XHigh),
        _ => None,
    }
}

#[cfg(feature = "native-codex-runtime")]
fn runtime_web_search_mode(mode: &str) -> Option<WebSearchModeConfig> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "disabled" => Some(WebSearchModeConfig::Disabled),
        "cached" => Some(WebSearchModeConfig::Cached),
        "live" => Some(WebSearchModeConfig::Live),
        _ => None,
    }
}

#[cfg(feature = "native-codex-runtime")]
fn apply_runtime_config_bootstrap_overrides(
    config: &mut codex_core::config::Config,
    runtime_config: &RuntimeCodexConfig,
) -> Result<(), String> {
    if let Some(reasoning_effort) = runtime_reasoning_effort(runtime_config.reasoning.as_str()) {
        config.model_reasoning_effort = Some(reasoning_effort);
    }
    if let Some(web_search_mode) = runtime_web_search_mode(runtime_config.web_search_mode.as_str())
    {
        config
            .web_search_mode
            .set(web_search_mode)
            .map_err(|error| {
                format!("failed to apply runtime web_search_mode override: {error}")
            })?;
    }
    Ok(())
}

#[cfg(feature = "native-codex-runtime")]
#[allow(dead_code)]
fn json_to_toml_value(value: Value) -> toml::Value {
    match value {
        Value::Null => toml::Value::String(String::new()),
        Value::Bool(flag) => toml::Value::Boolean(flag),
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                toml::Value::Integer(integer)
            } else if let Some(float) = number.as_f64() {
                toml::Value::Float(float)
            } else {
                toml::Value::String(number.to_string())
            }
        }
        Value::String(text) => toml::Value::String(text),
        Value::Array(entries) => {
            toml::Value::Array(entries.into_iter().map(json_to_toml_value).collect())
        }
        Value::Object(entries) => toml::Value::Table(
            entries
                .into_iter()
                .map(|(key, entry)| (key, json_to_toml_value(entry)))
                .collect(),
        ),
    }
}

#[cfg(feature = "native-codex-runtime")]
#[allow(dead_code)]
fn parse_runtime_cli_overrides(
    config: Option<Value>,
) -> Result<Vec<(String, toml::Value)>, String> {
    let Some(config) = config else {
        return Ok(Vec::new());
    };

    let Value::Object(entries) = config else {
        return Err("config must be a plain JSON object".to_string());
    };

    Ok(entries
        .into_iter()
        .map(|(key, value)| (key, json_to_toml_value(value)))
        .collect())
}
#[cfg(feature = "native-codex-runtime")]
fn translate_turn_input_items(items: Vec<CodexInputItem>) -> Result<Vec<UserInput>, String> {
    let mut translated = Vec::with_capacity(items.len());

    for item in items {
        let item_type = item.item_type.trim().to_ascii_lowercase();
        match item_type.as_str() {
            "text" => {
                let text = item.text.unwrap_or_default();
                translated.push(UserInput::Text {
                    text,
                    text_elements: Vec::new(),
                });
            }
            "local_image" | "localimage" => {
                let path = item
                    .path
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "local_image input item requires `path`".to_string())?;
                translated.push(UserInput::LocalImage {
                    path: std::path::PathBuf::from(path),
                });
            }
            "image" => {
                let image_url = item
                    .image_url
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "image input item requires `imageUrl`".to_string())?;
                translated.push(UserInput::Image { image_url });
            }
            "mention" => {
                let path = item
                    .path
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "mention input item requires `path`".to_string())?;
                let name = item
                    .name
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| path.clone());
                translated.push(UserInput::Mention { name, path });
            }
            "skill" => {
                let name = item
                    .name
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "skill input item requires `name`".to_string())?;
                if let Some(path) = item
                    .path
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                {
                    translated.push(UserInput::Skill {
                        name,
                        path: std::path::PathBuf::from(path),
                    });
                } else {
                    translated.push(UserInput::Text {
                        text: format!("[skill] {name}"),
                        text_elements: Vec::new(),
                    });
                }
            }
            other => return Err(format!("unsupported input item type: {other}")),
        }
    }

    Ok(translated)
}

#[cfg(feature = "native-codex-runtime")]
async fn resolve_native_thread(
    app: &AppHandle,
    session_id: u64,
    requested_thread_id: Option<String>,
    cwd: &std::path::Path,
    create_thread_runtime_config: Option<RuntimeCodexConfig>,
) -> Result<(String, Arc<CodexThread>, bool), String> {
    let requested_thread_id = normalize_runtime_thread_id(requested_thread_id);
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
            .or_else(|| normalize_runtime_thread_id(active.thread_id.clone()));

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

                let mut config_builder = native_config_builder(runtime.codex_home.clone(), cwd);
                if let Some(runtime_config) = bootstrap_runtime_config {
                    config_builder = config_builder
                        .harness_overrides(runtime_config_harness_overrides(runtime_config, cwd));
                } else {
                    config_builder =
                        config_builder.harness_overrides(native_profile_harness_overrides(cwd));
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

    let mut config_builder = native_config_builder(runtime.codex_home.clone(), cwd);
    if let Some(runtime_config) = bootstrap_runtime_config {
        config_builder =
            config_builder.harness_overrides(runtime_config_harness_overrides(runtime_config, cwd));
    } else {
        config_builder = config_builder.harness_overrides(native_profile_harness_overrides(cwd));
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
async fn load_native_thread_from_active_session(
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

            let config = native_config_builder(runtime.codex_home.clone(), session_cwd.as_path())
                .harness_overrides(native_profile_harness_overrides(session_cwd.as_path()))
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

#[cfg(feature = "native-codex-runtime")]
fn with_native_handles_mut<R>(
    app: &AppHandle,
    session_id: u64,
    f: impl FnOnce(&mut crate::NativeSessionHandles) -> R,
) -> Option<R> {
    let state = app.state::<AppState>();
    let mut guard = lock_active_session(state.inner()).ok()?;
    let active = guard.as_mut()?;
    if active.session_id != session_id {
        return None;
    }
    let crate::ActiveSessionTransport::Native(native) = &mut active.transport;
    Some(f(native))
}

#[cfg(feature = "native-codex-runtime")]
fn reinsert_pending_approval_entry(
    pending_approvals: &mut std::collections::HashMap<String, crate::NativePendingApproval>,
    action_id: &str,
    pending_approval: crate::NativePendingApproval,
) {
    pending_approvals.insert(action_id.to_string(), pending_approval);
}

#[cfg(feature = "native-codex-runtime")]
fn reinsert_pending_user_input_entry(
    pending_user_inputs: &mut std::collections::HashMap<String, crate::NativePendingUserInput>,
    action_id: &str,
    pending_user_input: crate::NativePendingUserInput,
) {
    pending_user_inputs.insert(action_id.to_string(), pending_user_input);
}

#[cfg(feature = "native-codex-runtime")]
fn validate_approval_decision_before_lookup(decision: &str) -> Result<(), String> {
    if decision.trim().is_empty() {
        return Err("decision is required".to_string());
    }
    Ok(())
}

#[cfg(feature = "native-codex-runtime")]
fn validate_user_input_decision_before_lookup(decision: &str) -> Result<(), String> {
    crate::domain::session_thread_review::interaction_policy::parse_user_input_decision(decision)
        .map(|_| ())
}

#[cfg(feature = "native-codex-runtime")]
fn reinsert_pending_approval_for_session(
    app: &AppHandle,
    session_id: u64,
    action_id: &str,
    pending_approval: crate::NativePendingApproval,
) {
    let _ = with_native_handles_mut(app, session_id, |native| {
        reinsert_pending_approval_entry(&mut native.pending_approvals, action_id, pending_approval)
    });
}

#[cfg(feature = "native-codex-runtime")]
fn reinsert_pending_user_input_for_session(
    app: &AppHandle,
    session_id: u64,
    action_id: &str,
    pending_user_input: crate::NativePendingUserInput,
) {
    let _ = with_native_handles_mut(app, session_id, |native| {
        reinsert_pending_user_input_entry(
            &mut native.pending_user_inputs,
            action_id,
            pending_user_input,
        )
    });
}

#[cfg(feature = "native-codex-runtime")]
fn map_native_steer_error(error: SteerInputError) -> String {
    match error {
        SteerInputError::NoActiveTurn(_) => "no active turn to steer".to_string(),
        SteerInputError::ExpectedTurnMismatch { expected, actual } => {
            format!("expected_turn_id mismatch: expected `{expected}`, active `{actual}`")
        }
        SteerInputError::EmptyInput => "input_items cannot be empty".to_string(),
    }
}
#[cfg(feature = "native-codex-runtime")]
fn turn_id_mismatch_error(expected: &str, actual: &str) -> String {
    format!("turn_id mismatch: expected `{expected}`, active `{actual}`")
}

#[cfg(feature = "native-codex-runtime")]
fn resolve_native_active_turn_for_thread(
    native: &crate::NativeSessionHandles,
    requested_thread_id: &str,
    thread: &Arc<CodexThread>,
) -> Option<(String, String)> {
    if let Some(turn_id) = native.active_turns.get(requested_thread_id) {
        return Some((requested_thread_id.to_string(), turn_id.clone()));
    }

    native
        .threads
        .iter()
        .find_map(|(known_thread_id, known_thread)| {
            if !Arc::ptr_eq(known_thread, thread) {
                return None;
            }
            native
                .active_turns
                .get(known_thread_id)
                .map(|turn_id| (known_thread_id.clone(), turn_id.clone()))
        })
}

#[cfg(feature = "native-codex-runtime")]
fn clear_native_pending_actions_for_threads(
    native: &mut crate::NativeSessionHandles,
    thread_ids: &[String],
) {
    if thread_ids.is_empty() {
        return;
    }
    let thread_id_set = thread_ids
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    native
        .pending_approvals
        .retain(|_, pending| !thread_id_set.contains(pending.thread_id.as_str()));
    native
        .pending_user_inputs
        .retain(|_, pending| !thread_id_set.contains(pending.thread_id.as_str()));
}

#[cfg(feature = "native-codex-runtime")]
fn parse_native_review_request(
    target: Option<Value>,
    delivery: Option<String>,
) -> Result<ReviewRequest, String> {
    let target = match target {
        Some(target_value) => serde_json::from_value::<ReviewTarget>(target_value)
            .map_err(|error| format!("target is invalid for native review request: {error}"))?,
        None => ReviewTarget::UncommittedChanges,
    };

    let user_facing_hint = delivery
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .map(|value| format!("delivery:{value}"));

    Ok(ReviewRequest {
        target,
        user_facing_hint,
    })
}

#[cfg(feature = "native-codex-runtime")]
fn infer_thread_id_from_rollout_path(path: &Path) -> Option<String> {
    session_thread_shared::infer_thread_id_from_rollout_path(path)
}

#[cfg(feature = "native-codex-runtime")]
async fn native_thread_summary_from_rollout_path(
    rollout_path: &Path,
    fallback_provider: &str,
    include_turns: bool,
    preferred_thread_id: Option<&str>,
) -> Result<CodexThreadSummary, String> {
    session_thread_shared::native_thread_summary_from_rollout_path(
        rollout_path,
        fallback_provider,
        include_turns,
        preferred_thread_id,
    )
    .await
}

fn unsupported_slash_command_message(command: &str) -> String {
    let normalized = command.trim();
    let display_command = if normalized.is_empty() {
        "/"
    } else {
        normalized
    };
    format!(
        "slash command `{display_command}` is not available in the current runtime. Supported command: /status"
    )
}
fn finish_session_turn(app: &AppHandle, session_id: u64, discovered_thread_id: Option<String>) {
    let state = app.state::<AppState>();
    let mut guard = match lock_active_session(state.inner()) {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let Some(active) = guard.as_mut() else {
        return;
    };

    if active.session_id != session_id {
        return;
    }

    active.busy = false;
    if let Some(thread_id) = discovered_thread_id {
        active.thread_id = Some(thread_id);
    }
}

fn is_active_session(app: &AppHandle, session_id: u64) -> bool {
    let state = app.state::<AppState>();
    let guard = match lock_active_session(state.inner()) {
        Ok(guard) => guard,
        Err(_) => return false,
    };

    guard
        .as_ref()
        .is_some_and(|active| active.session_id == session_id)
}

fn parse_slash_command(prompt: &str) -> Option<(&str, &str)> {
    let trimmed = prompt.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let command = parts.next()?;
    let args = parts.next().unwrap_or("").trim();
    Some((command, args))
}

#[cfg(feature = "native-codex-runtime")]
async fn schedule_turn_run_native(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexTurnRunRequest,
) -> Result<CodexTurnRunResponse, String> {
    let runtime_config = lock_runtime_config(state.inner())?.clone();
    let CodexTurnRunRequest {
        thread_id: requested_thread_id,
        input_items,
        output_schema,
    } = request;
    let output_schema_for_turn = output_schema.clone();

    let schedule_plan =
        session_thread_review_use_cases::plan_native_thread_schedule(&state, requested_thread_id)?;
    let session_thread_review_use_cases::NativeThreadSchedulePlan {
        reservation,
        requested_thread_id,
    } = schedule_plan;
    let response = reservation.turn_run_accepted_response();
    let session_thread_review_use_cases::NativeSessionSlotReservation {
        session_id,
        pid,
        cwd,
        ..
    } = reservation;

    let app_for_task = app.clone();
    let event_seq = Arc::clone(&state.next_event_seq);
    tauri::async_runtime::spawn(async move {
        let result: Result<String, String> = async {
            let requested = requested_thread_id;
            let (thread_id, thread, created_thread) = resolve_native_thread(
                &app_for_task,
                session_id,
                requested,
                &cwd,
                Some(runtime_config.clone()),
            )
            .await?;

            if created_thread {
                emit_codex_event(
                    &app_for_task,
                    session_id,
                    json!({
                        "type": "thread.started",
                        "thread_id": thread_id.clone(),
                    }),
                    &event_seq,
                );
            }

            let model = if runtime_config.model.trim().is_empty()
                || runtime_config.model.eq_ignore_ascii_case("default")
            {
                thread.config_snapshot().await.model
            } else {
                runtime_config.model.clone()
            };

            let user_items = translate_turn_input_items(input_items)?;

            thread
                .submit(Op::UserTurn {
                    items: user_items,
                    cwd: cwd.clone(),
                    approval_policy: runtime_approval_policy(&runtime_config.approval_policy),
                    sandbox_policy: runtime_sandbox_policy(&runtime_config.sandbox),
                    model,
                    effort: runtime_reasoning_effort(&runtime_config.reasoning),
                    summary: ReasoningSummaryConfig::default(),
                    final_output_json_schema: output_schema_for_turn,
                    collaboration_mode: None,
                    personality: None,
                })
                .await
                .map_err(|error| format!("failed to submit native turn: {error}"))?;

            let mut translator = NativeCodexEventTranslator::new(thread_id.clone());
            loop {
                let event = thread
                    .next_event()
                    .await
                    .map_err(|error| format!("native event stream failed: {error}"))?;
                let is_terminal = matches!(
                    event.msg,
                    EventMsg::TurnComplete(_) | EventMsg::TurnAborted(_)
                );

                let Some(translated_events) =
                    with_native_handles_mut(&app_for_task, session_id, |native| {
                        translator.translate_event(&event, native)
                    })
                else {
                    break;
                };

                for translated in translated_events {
                    emit_codex_event(&app_for_task, session_id, translated, &event_seq);
                }

                if is_terminal {
                    break;
                }
            }

            Ok(thread_id)
        }
        .await;

        match result {
            Ok(returned_thread_id) => {
                finish_session_turn(&app_for_task, session_id, Some(returned_thread_id));
            }
            Err(error) => {
                if is_active_session(&app_for_task, session_id) {
                    emit_lifecycle(
                        &app_for_task,
                        "error",
                        Some(session_id),
                        pid,
                        None,
                        Some(error),
                    );
                }
                finish_session_turn(&app_for_task, session_id, None);
            }
        }
    });

    Ok(response)
}

async fn schedule_turn_run(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexTurnRunRequest,
) -> Result<CodexTurnRunResponse, String> {
    schedule_turn_run_native(app, state, request).await
}

pub(crate) async fn codex_turn_run_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexTurnRunRequest,
) -> Result<CodexTurnRunResponse, String> {
    if request.input_items.is_empty() {
        return Err("input_items cannot be empty".to_string());
    }
    if request
        .output_schema
        .as_ref()
        .is_some_and(|schema| !schema.is_object())
    {
        return Err("output_schema must be a plain JSON object".to_string());
    }
    schedule_turn_run(app, state, request).await
}

#[cfg(feature = "native-codex-runtime")]
async fn schedule_review_start_native(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexReviewStartRequest,
) -> Result<CodexReviewStartResponse, String> {
    let runtime_config = lock_runtime_config(state.inner())?.clone();
    let CodexReviewStartRequest {
        thread_id: requested_thread_id,
        target,
        delivery,
    } = request;

    let schedule_plan =
        session_thread_review_use_cases::plan_native_thread_schedule(&state, requested_thread_id)?;
    let session_thread_review_use_cases::NativeThreadSchedulePlan {
        reservation,
        requested_thread_id,
    } = schedule_plan;
    let response = reservation.review_start_accepted_response();
    let session_thread_review_use_cases::NativeSessionSlotReservation {
        session_id,
        pid,
        cwd,
        ..
    } = reservation;

    let app_for_task = app.clone();
    let event_seq = Arc::clone(&state.next_event_seq);
    tauri::async_runtime::spawn(async move {
        let result: Result<String, String> = async {
            let requested = requested_thread_id;
            let (thread_id, thread, created_thread) = resolve_native_thread(
                &app_for_task,
                session_id,
                requested,
                &cwd,
                Some(runtime_config.clone()),
            )
            .await?;

            if created_thread {
                emit_codex_event(
                    &app_for_task,
                    session_id,
                    json!({
                        "type": "thread.started",
                        "thread_id": thread_id.clone(),
                    }),
                    &event_seq,
                );
            }

            let review_request = parse_native_review_request(target, delivery)?;
            thread
                .submit(Op::Review { review_request })
                .await
                .map_err(|error| format!("failed to submit native review: {error}"))?;

            let mut translator = NativeCodexEventTranslator::new(thread_id.clone());
            loop {
                let event = thread
                    .next_event()
                    .await
                    .map_err(|error| format!("native event stream failed: {error}"))?;
                let is_terminal = matches!(
                    event.msg,
                    EventMsg::TurnComplete(_) | EventMsg::TurnAborted(_)
                );

                let Some(translated_events) =
                    with_native_handles_mut(&app_for_task, session_id, |native| {
                        translator.translate_event(&event, native)
                    })
                else {
                    break;
                };

                for translated in translated_events {
                    emit_codex_event(&app_for_task, session_id, translated, &event_seq);
                }

                if is_terminal {
                    break;
                }
            }

            Ok(thread_id)
        }
        .await;

        match result {
            Ok(returned_thread_id) => {
                finish_session_turn(&app_for_task, session_id, Some(returned_thread_id));
            }
            Err(error) => {
                if is_active_session(&app_for_task, session_id) {
                    emit_lifecycle(
                        &app_for_task,
                        "error",
                        Some(session_id),
                        pid,
                        None,
                        Some(error),
                    );
                }
                finish_session_turn(&app_for_task, session_id, None);
            }
        }
    });

    Ok(response)
}

async fn schedule_review_start(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexReviewStartRequest,
) -> Result<CodexReviewStartResponse, String> {
    schedule_review_start_native(app, state, request).await
}

pub(crate) async fn codex_review_start_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexReviewStartRequest,
) -> Result<CodexReviewStartResponse, String> {
    session_thread_review_use_cases::validate_review_start_request(&request)?;

    schedule_review_start(app, state, request).await
}

pub(crate) async fn codex_thread_open_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    thread_id: Option<String>,
) -> Result<CodexThreadOpenResponse, String> {
    let runtime_config = lock_runtime_config(state.inner())?.clone();
    let (session_id, cwd) = {
        let guard = lock_active_session(state.inner())?;
        let active = guard
            .as_ref()
            .ok_or_else(|| "no active codex session".to_string())?;
        (active.session_id, active.cwd.clone())
    };

    let (opened_thread_id, _thread, created_thread) = resolve_native_thread(
        &app,
        session_id,
        normalize_runtime_thread_id(thread_id),
        &cwd,
        Some(runtime_config),
    )
    .await?;

    {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;
        if active.session_id != session_id {
            return Err("active codex session changed while opening thread".to_string());
        }
        active.thread_id = Some(opened_thread_id.clone());
    }

    if created_thread {
        emit_codex_event(
            &app,
            session_id,
            json!({
                "type": "thread.started",
                "thread_id": opened_thread_id.clone(),
            }),
            &state.next_event_seq,
        );
    }

    Ok(CodexThreadOpenResponse {
        thread_id: opened_thread_id,
    })
}

pub(crate) async fn codex_thread_close_impl(
    state: State<'_, AppState>,
    request: CodexThreadCloseRequest,
) -> Result<CodexThreadCloseResponse, String> {
    let thread_id = request.thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    let (runtime, removed_from_cache) = {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;

        let crate::ActiveSessionTransport::Native(native) = &mut active.transport;

        let removed_from_cache = native.threads.remove(&thread_id);
        let mut removed_thread_ids = vec![thread_id.clone()];
        native.active_turns.remove(&thread_id);
        if let Some(removed) = removed_from_cache.as_ref() {
            let alias_ids = native
                .threads
                .iter()
                .filter_map(|(candidate_id, candidate)| {
                    Arc::ptr_eq(candidate, removed).then_some(candidate_id.clone())
                })
                .collect::<Vec<_>>();
            for alias_id in alias_ids {
                native.threads.remove(alias_id.as_str());
                native.active_turns.remove(alias_id.as_str());
                removed_thread_ids.push(alias_id);
            }
        }
        clear_native_pending_actions_for_threads(native, &removed_thread_ids);
        let removed_cache_entry = removed_from_cache.is_some();

        if active.thread_id.as_deref().is_some_and(|active_id| {
            active_id == thread_id
                || (removed_cache_entry && !native.threads.contains_key(active_id))
        }) {
            active.thread_id = None;
        }

        (Arc::clone(&native.runtime), removed_from_cache)
    };

    let mut removed_thread = removed_from_cache;
    let removable_thread_id = ThreadId::from_string(&thread_id).ok().or_else(|| {
        removed_thread
            .as_ref()
            .and_then(|thread| thread.rollout_path())
            .as_deref()
            .and_then(infer_thread_id_from_rollout_path)
            .and_then(|value| ThreadId::from_string(&value).ok())
    });
    if let Some(removable_thread_id) = removable_thread_id {
        if let Some(thread) = runtime
            .thread_manager
            .remove_thread(&removable_thread_id)
            .await
        {
            removed_thread = Some(thread);
        }
    }

    let Some(thread) = removed_thread else {
        return Err(format!("thread not found: {thread_id}"));
    };
    let _ = thread.submit(Op::Shutdown).await;

    Ok(CodexThreadCloseResponse {
        thread_id,
        removed: true,
    })
}

pub(crate) async fn codex_thread_list_impl(
    state: State<'_, AppState>,
    request: CodexThreadListRequest,
) -> Result<CodexThreadListResponse, String> {
    session_thread_review_use_cases::codex_thread_list(state, request).await
}

pub(crate) async fn codex_thread_read_impl(
    state: State<'_, AppState>,
    request: CodexThreadReadRequest,
) -> Result<CodexThreadReadResponse, String> {
    session_thread_review_use_cases::codex_thread_read(state, request).await
}

pub(crate) async fn codex_thread_archive_impl(
    state: State<'_, AppState>,
    request: CodexThreadArchiveRequest,
) -> Result<CodexThreadArchiveResponse, String> {
    let thread_id = request.thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    let (runtime, loaded_thread) = {
        let guard = lock_active_session(state.inner())?;
        let active = guard
            .as_ref()
            .ok_or_else(|| "no active codex session".to_string())?;

        let crate::ActiveSessionTransport::Native(native) = &active.transport;

        (
            Arc::clone(&native.runtime),
            native.threads.get(&thread_id).cloned(),
        )
    };

    let parsed_thread_id = ThreadId::from_string(&thread_id).ok();
    let inferred_thread_id = loaded_thread
        .as_ref()
        .and_then(|thread| thread.rollout_path())
        .as_deref()
        .and_then(infer_thread_id_from_rollout_path);
    let resolved_thread_id = inferred_thread_id
        .as_deref()
        .and_then(|value| ThreadId::from_string(value).ok())
        .or(parsed_thread_id);

    let rollout_path = if let Some(path) = loaded_thread
        .as_ref()
        .and_then(|thread| thread.rollout_path())
    {
        path
    } else {
        let lookup_thread_id = inferred_thread_id
            .as_deref()
            .unwrap_or(thread_id.as_str())
            .to_string();
        find_thread_path_by_id_str(runtime.codex_home.as_path(), lookup_thread_id.as_str())
            .await
            .map_err(|error| format!("failed to locate thread id {lookup_thread_id}: {error}"))?
            .ok_or_else(|| format!("no rollout found for thread id {lookup_thread_id}"))?
    };

    let sessions_dir = runtime.codex_home.join(SESSIONS_SUBDIR);
    let canonical_sessions_dir = tokio::fs::canonicalize(&sessions_dir)
        .await
        .map_err(|error| {
            format!("failed to archive thread: unable to resolve sessions directory: {error}")
        })?;
    let canonical_rollout_path = tokio::fs::canonicalize(&rollout_path).await.map_err(|_| {
        format!(
            "rollout path `{}` must be in sessions directory",
            rollout_path.display()
        )
    })?;
    if !canonical_rollout_path.starts_with(&canonical_sessions_dir) {
        return Err(format!(
            "rollout path `{}` must be in sessions directory",
            rollout_path.display()
        ));
    }

    let canonical_thread_id = inferred_thread_id
        .clone()
        .or_else(|| resolved_thread_id.as_ref().map(ToString::to_string))
        .or_else(|| infer_thread_id_from_rollout_path(canonical_rollout_path.as_path()))
        .ok_or_else(|| {
            format!(
                "failed to infer canonical thread id from rollout `{}`",
                rollout_path.display()
            )
        })?;
    let required_suffix = format!("{canonical_thread_id}.jsonl");
    let file_name = canonical_rollout_path.file_name().ok_or_else(|| {
        format!(
            "rollout path `{}` missing file name",
            rollout_path.display()
        )
    })?;
    if !file_name
        .to_string_lossy()
        .ends_with(required_suffix.as_str())
    {
        return Err(format!(
            "rollout path `{}` does not match thread id {canonical_thread_id}",
            rollout_path.display(),
        ));
    }
    let file_name = file_name.to_owned();

    if let Some(resolved_thread_id) = resolved_thread_id {
        if let Some(thread) = runtime
            .thread_manager
            .remove_thread(&resolved_thread_id)
            .await
        {
            let _ = thread.submit(Op::Shutdown).await;
        }
    }

    {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;

        let crate::ActiveSessionTransport::Native(native) = &mut active.transport;
        let removed_from_cache = native.threads.remove(&thread_id);
        let mut removed_thread_ids = vec![thread_id.clone()];
        native.active_turns.remove(&thread_id);
        if let Some(removed) = removed_from_cache.as_ref() {
            let alias_ids = native
                .threads
                .iter()
                .filter_map(|(candidate_id, candidate)| {
                    Arc::ptr_eq(candidate, removed).then_some(candidate_id.clone())
                })
                .collect::<Vec<_>>();
            for alias_id in alias_ids {
                native.threads.remove(alias_id.as_str());
                native.active_turns.remove(alias_id.as_str());
                removed_thread_ids.push(alias_id);
            }
        }
        clear_native_pending_actions_for_threads(native, &removed_thread_ids);

        if active.thread_id.as_deref().is_some_and(|active_id| {
            active_id == thread_id
                || (removed_from_cache.is_some() && !native.threads.contains_key(active_id))
        }) {
            active.thread_id = None;
        }
    }

    let archive_dir = runtime.codex_home.join(ARCHIVED_SESSIONS_SUBDIR);
    tokio::fs::create_dir_all(&archive_dir)
        .await
        .map_err(|error| format!("failed to archive thread: {error}"))?;

    let archived_path = archive_dir.join(file_name);
    tokio::fs::rename(&canonical_rollout_path, &archived_path)
        .await
        .map_err(|error| format!("failed to archive thread: {error}"))?;

    Ok(CodexThreadArchiveResponse {
        id: thread_id.clone(),
        codex_thread_id: canonical_thread_id,
        archived: true,
    })
}

pub(crate) async fn codex_thread_unarchive_impl(
    state: State<'_, AppState>,
    request: CodexThreadUnarchiveRequest,
) -> Result<CodexThreadUnarchiveResponse, String> {
    let thread_id = request.thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    let (runtime, session_cwd) = {
        let guard = lock_active_session(state.inner())?;
        let active = guard
            .as_ref()
            .ok_or_else(|| "no active codex session".to_string())?;

        let crate::ActiveSessionTransport::Native(native) = &active.transport;

        (Arc::clone(&native.runtime), active.cwd.clone())
    };

    let config = native_config_builder(runtime.codex_home.clone(), session_cwd.as_path())
        .harness_overrides(native_profile_harness_overrides(session_cwd.as_path()))
        .build()
        .await
        .map_err(|error| format!("failed to build native thread unarchive config: {error}"))?;
    let fallback_provider = config.model_provider_id.clone();

    let archived_path =
        find_archived_thread_path_by_id_str(runtime.codex_home.as_path(), &thread_id)
            .await
            .map_err(|error| format!("failed to locate archived thread id {thread_id}: {error}"))?
            .ok_or_else(|| format!("no archived rollout found for thread id {thread_id}"))?;

    let rollout_path_display = archived_path.display().to_string();
    let archived_folder = runtime.codex_home.join(ARCHIVED_SESSIONS_SUBDIR);
    let canonical_archived_dir =
        tokio::fs::canonicalize(&archived_folder)
            .await
            .map_err(|error| {
                format!("failed to unarchive thread: unable to resolve archived directory: {error}")
            })?;
    let canonical_rollout_path = tokio::fs::canonicalize(&archived_path)
        .await
        .ok()
        .filter(|path| path.starts_with(&canonical_archived_dir))
        .ok_or_else(|| {
            format!("rollout path `{rollout_path_display}` must be in archived directory")
        })?;

    let required_suffix = format!("{thread_id}.jsonl");
    let Some(file_name) = canonical_rollout_path
        .file_name()
        .map(|name| name.to_owned())
    else {
        return Err(format!(
            "rollout path `{rollout_path_display}` missing file name"
        ));
    };
    if !file_name
        .to_string_lossy()
        .ends_with(required_suffix.as_str())
    {
        return Err(format!(
            "rollout path `{rollout_path_display}` does not match thread id {thread_id}"
        ));
    }

    let Some((year, month, day)) = rollout_date_parts(&file_name) else {
        return Err(format!(
            "rollout path `{rollout_path_display}` missing filename timestamp"
        ));
    };

    let sessions_folder = runtime.codex_home.join(SESSIONS_SUBDIR);
    let dest_dir = sessions_folder.join(year).join(month).join(day);
    tokio::fs::create_dir_all(&dest_dir)
        .await
        .map_err(|error| format!("failed to unarchive thread: {error}"))?;
    let restored_path = dest_dir.join(&file_name);
    tokio::fs::rename(&canonical_rollout_path, &restored_path)
        .await
        .map_err(|error| format!("failed to unarchive thread: {error}"))?;
    tokio::task::spawn_blocking({
        let restored_path = restored_path.clone();
        move || -> std::io::Result<()> {
            let times = FileTimes::new().set_modified(SystemTime::now());
            OpenOptions::new()
                .append(true)
                .open(&restored_path)?
                .set_times(times)?;
            Ok(())
        }
    })
    .await
    .map_err(|error| format!("failed to update unarchived thread timestamp: {error}"))?
    .map_err(|error| format!("failed to update unarchived thread timestamp: {error}"))?;

    let thread = native_thread_summary_from_rollout_path(
        restored_path.as_path(),
        fallback_provider.as_str(),
        false,
        Some(thread_id.as_str()),
    )
    .await?;

    Ok(CodexThreadUnarchiveResponse { thread })
}

pub(crate) async fn codex_thread_compact_start_impl(
    state: State<'_, AppState>,
    request: CodexThreadCompactStartRequest,
) -> Result<CodexThreadCompactStartResponse, String> {
    let thread_id = request.thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    let thread = load_native_thread_from_active_session(&state, &thread_id).await?;
    thread
        .submit(Op::Compact)
        .await
        .map_err(|error| format!("failed to start compaction: {error}"))?;

    Ok(CodexThreadCompactStartResponse {
        ok: true,
        thread_id: thread_id.clone(),
        codex_thread_id: thread_id,
    })
}

pub(crate) async fn codex_thread_rollback_impl(
    state: State<'_, AppState>,
    request: CodexThreadRollbackRequest,
) -> Result<CodexThreadRollbackResponse, String> {
    let thread_id = request.thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    if request.num_turns < 1 {
        return Err("num_turns must be greater than or equal to 1".to_string());
    }

    let (runtime, session_cwd) = {
        let guard = lock_active_session(state.inner())?;
        let active = guard
            .as_ref()
            .ok_or_else(|| "no active codex session".to_string())?;

        let crate::ActiveSessionTransport::Native(native) = &active.transport;

        (Arc::clone(&native.runtime), active.cwd.clone())
    };

    let config = native_config_builder(runtime.codex_home.clone(), session_cwd.as_path())
        .harness_overrides(native_profile_harness_overrides(session_cwd.as_path()))
        .build()
        .await
        .map_err(|error| format!("failed to build native thread rollback config: {error}"))?;
    let fallback_provider = config.model_provider_id.clone();

    let thread = load_native_thread_from_active_session(&state, &thread_id).await?;
    let rollout_path = if let Some(path) = thread.rollout_path() {
        path
    } else {
        find_thread_path_by_id_str(runtime.codex_home.as_path(), &thread_id)
            .await
            .map_err(|error| format!("failed to locate thread id {thread_id}: {error}"))?
            .ok_or_else(|| format!("no rollout found for thread id {thread_id}"))?
    };

    let initial_summary = native_thread_summary_from_rollout_path(
        rollout_path.as_path(),
        fallback_provider.as_str(),
        true,
        Some(thread_id.as_str()),
    )
    .await?;
    let target_turn_count = initial_summary
        .turn_count
        .saturating_sub(request.num_turns as usize);

    thread
        .submit(Op::ThreadRollback {
            num_turns: request.num_turns,
        })
        .await
        .map_err(|error| format!("failed to start rollback: {error}"))?;

    let timeout = std::time::Duration::from_secs(10);
    let start = std::time::Instant::now();
    loop {
        let summary = native_thread_summary_from_rollout_path(
            rollout_path.as_path(),
            fallback_provider.as_str(),
            true,
            Some(thread_id.as_str()),
        )
        .await?;

        if summary.turn_count <= target_turn_count
            || summary.turn_count < initial_summary.turn_count
        {
            return Ok(CodexThreadRollbackResponse { thread: summary });
        }

        if start.elapsed() >= timeout {
            return Err("timed out waiting for thread rollback confirmation".to_string());
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

pub(crate) async fn codex_thread_fork_impl(
    state: State<'_, AppState>,
    request: CodexThreadForkRequest,
) -> Result<CodexThreadForkResponse, String> {
    let CodexThreadForkRequest {
        thread_id,
        path,
        model,
        model_provider,
        cwd,
        approval_policy,
        sandbox,
        config,
        base_instructions,
        developer_instructions,
        persist_extended_history,
        new_thread_id,
    } = request;

    let thread_id = thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    let normalized_path = path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let normalized_model = model
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let normalized_model_provider = model_provider
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let normalized_cwd = cwd
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let normalized_approval_policy = approval_policy
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let normalized_sandbox = sandbox
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let normalized_base_instructions = base_instructions.map(|value| value.trim().to_string());
    let normalized_base_instructions =
        normalized_base_instructions.filter(|value| !value.is_empty());
    let normalized_developer_instructions =
        developer_instructions.map(|value| value.trim().to_string());
    let normalized_developer_instructions =
        normalized_developer_instructions.filter(|value| !value.is_empty());
    let persist_extended_history = persist_extended_history.unwrap_or(false);
    let normalized_new_thread_id = new_thread_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let config_entries = match config {
        Some(Value::Object(entries)) => Some(entries),
        Some(_) => return Err("config must be a plain JSON object".to_string()),
        None => None,
    };
    let config_cwd_override = config_entries
        .as_ref()
        .and_then(|entries| entries.get("cwd"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);

    let (runtime, session_cwd) = {
        let guard = lock_active_session(state.inner())?;
        let active = guard
            .as_ref()
            .ok_or_else(|| "no active codex session".to_string())?;

        let crate::ActiveSessionTransport::Native(native) = &active.transport;

        if let Some(alias) = normalized_new_thread_id.as_deref() {
            if native.threads.contains_key(alias) {
                return Err(format!("newThreadId `{alias}` is already in use"));
            }
        }

        (Arc::clone(&native.runtime), active.cwd.clone())
    };
    if let Some(alias) = normalized_new_thread_id.as_deref() {
        let alias_exists_on_disk = find_thread_path_by_id_str(runtime.codex_home.as_path(), alias)
            .await
            .map_err(|error| format!("failed to validate newThreadId `{alias}`: {error}"))?
            .is_some();
        if alias_exists_on_disk {
            return Err(format!("newThreadId `{alias}` is already in use"));
        }
    }

    let rollout_path = if let Some(path) = normalized_path {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path
        } else {
            runtime.codex_home.join(path)
        }
    } else {
        find_thread_path_by_id_str(runtime.codex_home.as_path(), &thread_id)
            .await
            .map_err(|error| format!("failed to locate thread id {thread_id}: {error}"))?
            .ok_or_else(|| format!("no rollout found for thread id {thread_id}"))?
    };

    let fork_cwd = normalized_cwd
        .map(PathBuf::from)
        .or(config_cwd_override)
        .unwrap_or_else(|| session_cwd.clone());
    let mut overrides = ConfigOverrides {
        cwd: Some(fork_cwd),
        ..Default::default()
    };
    overrides.model = normalized_model;
    overrides.model_provider = normalized_model_provider;
    overrides.approval_policy = normalized_approval_policy
        .as_deref()
        .map(runtime_approval_policy);
    overrides.sandbox_mode = normalized_sandbox.as_deref().map(runtime_sandbox_mode);
    overrides.base_instructions = normalized_base_instructions;
    overrides.developer_instructions = normalized_developer_instructions;
    if let Some(entries) = config_entries {
        if overrides.model.is_none() {
            let model_override = entries
                .get("model")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(model_override) = model_override {
                overrides.model = Some(model_override.to_string());
            }
        }

        if overrides.model_provider.is_none() {
            let model_provider = entries
                .get("modelProvider")
                .or_else(|| entries.get("model_provider"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(model_provider) = model_provider {
                overrides.model_provider = Some(model_provider.to_string());
            }
        }

        if overrides.approval_policy.is_none() {
            let approval_override = entries
                .get("approvalPolicy")
                .or_else(|| entries.get("approval_policy"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(approval_override) = approval_override {
                overrides.approval_policy = Some(runtime_approval_policy(approval_override));
            }
        }

        if overrides.sandbox_mode.is_none() {
            let sandbox_override = entries
                .get("sandbox")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(sandbox_override) = sandbox_override {
                overrides.sandbox_mode = Some(runtime_sandbox_mode(sandbox_override));
            }
        }

        if overrides.base_instructions.is_none() {
            let base_instructions_override = entries
                .get("baseInstructions")
                .or_else(|| entries.get("base_instructions"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(base_instructions_override) = base_instructions_override {
                overrides.base_instructions = Some(base_instructions_override.to_string());
            }
        }

        if overrides.developer_instructions.is_none() {
            let developer_instructions_override = entries
                .get("developerInstructions")
                .or_else(|| entries.get("developer_instructions"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(developer_instructions_override) = developer_instructions_override {
                overrides.developer_instructions =
                    Some(developer_instructions_override.to_string());
            }
        }
    }
    overrides
        .config_profile
        .get_or_insert_with(|| ALICIA_NATIVE_INTERNAL_PROFILE.to_string());

    let builder = ConfigBuilder::default()
        .codex_home(runtime.codex_home.clone())
        .fallback_cwd(Some(session_cwd.clone()))
        .cli_overrides(native_internal_profile_cli_overrides())
        .harness_overrides(overrides);

    let fork_config = builder
        .build()
        .await
        .map_err(|error| format!("failed to build native thread fork config: {error}"))?;
    let fallback_provider = fork_config.model_provider_id.clone();

    let new_thread = runtime
        .thread_manager
        .fork_thread(
            usize::MAX,
            fork_config,
            rollout_path.clone(),
            persist_extended_history,
        )
        .await
        .map_err(|error| match error {
            CodexErr::Io(_) | CodexErr::Json(_) => {
                format!(
                    "failed to load rollout `{}`: {error}",
                    rollout_path.display()
                )
            }
            CodexErr::InvalidRequest(message) => message,
            _ => format!("error forking thread: {error}"),
        })?;

    let resolved_thread_id = new_thread.thread_id.to_string();
    let local_thread_id = normalized_new_thread_id.unwrap_or_else(|| resolved_thread_id.clone());
    let forked_thread = Arc::clone(&new_thread.thread);
    let rollout_path = new_thread
        .session_configured
        .rollout_path
        .ok_or_else(|| format!("rollout path missing for thread {resolved_thread_id}"))?;

    let thread = native_thread_summary_from_rollout_path(
        rollout_path.as_path(),
        fallback_provider.as_str(),
        true,
        Some(local_thread_id.as_str()),
    )
    .await?;

    {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;

        let crate::ActiveSessionTransport::Native(native) = &mut active.transport;
        native
            .threads
            .insert(resolved_thread_id.clone(), Arc::clone(&forked_thread));
        if local_thread_id != resolved_thread_id {
            native
                .threads
                .insert(local_thread_id.clone(), forked_thread);
        }
        active.thread_id = Some(local_thread_id);
    }

    Ok(CodexThreadForkResponse { thread })
}

pub(crate) async fn codex_turn_steer_impl(
    state: State<'_, AppState>,
    request: CodexTurnSteerRequest,
) -> Result<CodexTurnSteerResponse, String> {
    let thread_id = request.thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    let expected_turn_id = request.expected_turn_id.trim().to_string();
    if expected_turn_id.is_empty() {
        return Err("expected_turn_id is required".to_string());
    }

    if request.input_items.is_empty() {
        return Err("input_items cannot be empty".to_string());
    }

    let thread = load_native_thread_from_active_session(&state, &thread_id).await?;
    let input_items = translate_turn_input_items(request.input_items)?;
    let turn_id = thread
        .steer_input(input_items, Some(expected_turn_id.as_str()))
        .await
        .map_err(map_native_steer_error)?;

    Ok(CodexTurnSteerResponse {
        thread_id: thread_id.clone(),
        codex_thread_id: thread_id,
        turn_id,
    })
}

pub(crate) async fn codex_turn_interrupt_impl(
    state: State<'_, AppState>,
    request: CodexTurnInterruptRequest,
) -> Result<CodexTurnInterruptResponse, String> {
    let thread_id = request.thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    let turn_id = request.turn_id.trim().to_string();
    if turn_id.is_empty() {
        return Err("turn_id is required".to_string());
    }

    let thread = load_native_thread_from_active_session(&state, &thread_id).await?;
    let (codex_thread_id, active_turn_id) = {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;
        let crate::ActiveSessionTransport::Native(native) = &mut active.transport;

        resolve_native_active_turn_for_thread(native, &thread_id, &thread)
            .ok_or_else(|| "no active turn to interrupt".to_string())?
    };
    if active_turn_id != turn_id {
        return Err(turn_id_mismatch_error(&turn_id, active_turn_id.as_str()));
    }

    thread
        .submit(Op::Interrupt)
        .await
        .map_err(|error| format!("failed to submit native turn interrupt: {error}"))?;

    Ok(CodexTurnInterruptResponse {
        ok: true,
        thread_id: thread_id.clone(),
        codex_thread_id,
        turn_id,
    })
}

pub(crate) async fn codex_approval_respond_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexApprovalRespondRequest,
) -> Result<(), String> {
    let action_id = request.action_id.trim();
    if action_id.is_empty() {
        return Err("action_id is required".to_string());
    }
    validate_approval_decision_before_lookup(&request.decision)?;

    let remember = request.remember.unwrap_or(false);

    let (session_id, thread, pending_approval, event_seq) = {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;

        let session_id = active.session_id;
        let crate::ActiveSessionTransport::Native(native) = &mut active.transport;

        let pending_approval = native
            .pending_approvals
            .remove(action_id)
            .ok_or_else(|| format!("approval action not found: {action_id}"))?;

        let thread = native.threads.get(&pending_approval.thread_id).cloned();
        let Some(thread) = thread else {
            reinsert_pending_approval_entry(
                &mut native.pending_approvals,
                action_id,
                pending_approval.clone(),
            );
            return Err(format!("thread not found: {}", pending_approval.thread_id));
        };

        (
            session_id,
            thread,
            pending_approval,
            Arc::clone(&state.next_event_seq),
        )
    };

    let pending_kind = match pending_approval.kind {
        crate::NativeApprovalKind::CommandExecution => {
            session_thread_review_use_cases::ApprovalPendingKind::CommandExecution
        }
        crate::NativeApprovalKind::FileChange => {
            session_thread_review_use_cases::ApprovalPendingKind::FileChange
        }
    };
    let approval_plan = match session_thread_review_use_cases::plan_approval_response(
        action_id,
        pending_kind,
        &pending_approval.call_id,
        &pending_approval.turn_id,
        &request.decision,
        remember,
        request.execpolicy_amendment,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            reinsert_pending_approval_for_session(
                &app,
                session_id,
                action_id,
                pending_approval.clone(),
            );
            return Err(error);
        }
    };

    if let Err(error) = thread.submit(approval_plan.op).await {
        reinsert_pending_approval_for_session(
            &app,
            session_id,
            action_id,
            pending_approval.clone(),
        );
        return Err(format!(
            "failed to submit native approval response: {error}"
        ));
    }

    emit_codex_event(&app, session_id, approval_plan.resolved_event, &event_seq);

    Ok(())
}
pub(crate) async fn codex_user_input_respond_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CodexUserInputRespondRequest,
) -> Result<CodexUserInputRespondResponse, String> {
    let action_id = request.action_id.trim();
    if action_id.is_empty() {
        return Err("action_id is required".to_string());
    }
    validate_user_input_decision_before_lookup(&request.decision)?;

    let (session_id, thread, pending_user_input, event_seq) = {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;

        let session_id = active.session_id;
        let crate::ActiveSessionTransport::Native(native) = &mut active.transport;

        let pending_user_input = native
            .pending_user_inputs
            .remove(action_id)
            .ok_or_else(|| format!("user input action not found: {action_id}"))?;

        let thread = native.threads.get(&pending_user_input.thread_id).cloned();
        let Some(thread) = thread else {
            reinsert_pending_user_input_entry(
                &mut native.pending_user_inputs,
                action_id,
                pending_user_input.clone(),
            );
            return Err(format!(
                "thread not found: {}",
                pending_user_input.thread_id
            ));
        };

        (
            session_id,
            thread,
            pending_user_input,
            Arc::clone(&state.next_event_seq),
        )
    };

    let response_plan = match session_thread_review_use_cases::plan_user_input_response(
        action_id,
        &pending_user_input.thread_id,
        &pending_user_input.turn_id,
        &pending_user_input.call_id,
        &request.decision,
        request.answers,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            reinsert_pending_user_input_for_session(
                &app,
                session_id,
                action_id,
                pending_user_input.clone(),
            );
            return Err(error);
        }
    };

    if let Err(error) = thread.submit(response_plan.op).await {
        reinsert_pending_user_input_for_session(
            &app,
            session_id,
            action_id,
            pending_user_input.clone(),
        );
        return Err(format!(
            "failed to submit native user_input response: {error}"
        ));
    }

    emit_codex_event(&app, session_id, response_plan.resolved_event, &event_seq);

    Ok(CodexUserInputRespondResponse {
        ok: true,
        action_id: action_id.to_string(),
        decision: response_plan.decision,
    })
}
pub(crate) async fn send_codex_input_impl(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
) -> Result<(), String> {
    let prompt = text.trim_end_matches(['\r', '\n']).to_string();
    if prompt.trim().is_empty() {
        return Err("cannot send empty input".to_string());
    }

    let runtime_config = lock_runtime_config(state.inner())?.clone();
    let slash_command = parse_slash_command(&prompt);

    let (session_id, pid, thread_id, cwd, binary, transport, slash_status_requested) = {
        let mut guard = lock_active_session(state.inner())?;
        let active = guard
            .as_mut()
            .ok_or_else(|| "no active codex session".to_string())?;

        if active.busy {
            return Err("codex session is still processing the previous turn".to_string());
        }

        if let Some((command, _args)) = slash_command {
            if command.eq_ignore_ascii_case("/status") {
                (
                    active.session_id,
                    active.pid,
                    active.thread_id.clone(),
                    active.cwd.clone(),
                    active.binary.clone(),
                    active.transport(),
                    true,
                )
            } else {
                emit_stderr(
                    &app,
                    active.session_id,
                    unsupported_slash_command_message(command),
                );
                return Ok(());
            }
        } else {
            (
                active.session_id,
                active.pid,
                active.thread_id.clone(),
                active.cwd.clone(),
                active.binary.clone(),
                active.transport(),
                false,
            )
        }
    };

    if slash_status_requested {
        let rate_limits = fetch_rate_limits_for_status(&binary, &cwd);
        let chunk = format_non_tui_status(
            session_id,
            pid,
            thread_id.as_deref(),
            &cwd,
            &runtime_config,
            transport,
            rate_limits.as_ref(),
        );
        emit_stdout(&app, session_id, chunk);
        return Ok(());
    }

    let request = CodexTurnRunRequest {
        thread_id,
        input_items: vec![CodexInputItem {
            item_type: "text".to_string(),
            text: Some(prompt),
            path: None,
            image_url: None,
            name: None,
        }],
        output_schema: None,
    };

    let _ = cwd;
    let _ = schedule_turn_run(app, state, request).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "native-codex-runtime")]
    use super::{
        reinsert_pending_approval_entry, reinsert_pending_user_input_entry, runtime_model_override,
        runtime_profile_or_internal, runtime_profile_override, runtime_web_search_mode,
        validate_approval_decision_before_lookup, validate_user_input_decision_before_lookup,
        ALICIA_NATIVE_INTERNAL_PROFILE,
    };
    #[cfg(feature = "native-codex-runtime")]
    use crate::application::session_thread_review::use_cases as session_thread_review_use_cases;
    #[cfg(feature = "native-codex-runtime")]
    use crate::{NativeApprovalKind, NativePendingApproval, NativePendingUserInput};
    #[cfg(feature = "native-codex-runtime")]
    use codex_protocol::config_types::WebSearchMode as WebSearchModeConfig;
    #[cfg(feature = "native-codex-runtime")]
    use serde_json::Value;
    #[cfg(feature = "native-codex-runtime")]
    use std::collections::HashMap;
    #[cfg(feature = "native-codex-runtime")]
    use std::path::PathBuf;

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_model_override_discards_default_value() {
        assert_eq!(runtime_model_override("default"), None);
        assert_eq!(runtime_model_override("  "), None);
        assert_eq!(
            runtime_model_override("gpt-5-codex"),
            Some("gpt-5-codex".to_string())
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_profile_override_ignores_internal_permission_profiles() {
        assert_eq!(runtime_profile_override(""), None);
        assert_eq!(runtime_profile_override("read_only"), None);
        assert_eq!(runtime_profile_override(" read_write_with_approval "), None);
        assert_eq!(runtime_profile_override("full_access"), None);
        assert_eq!(
            runtime_profile_override(" custom_profile "),
            Some("custom_profile".to_string())
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_profile_or_internal_uses_internal_profile_fallback() {
        assert_eq!(
            runtime_profile_or_internal("read_only"),
            ALICIA_NATIVE_INTERNAL_PROFILE.to_string()
        );
        assert_eq!(
            runtime_profile_or_internal("custom_profile"),
            "custom_profile".to_string()
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_web_search_mode_parses_supported_modes() {
        assert_eq!(
            runtime_web_search_mode("cached"),
            Some(WebSearchModeConfig::Cached)
        );
        assert_eq!(
            runtime_web_search_mode("live"),
            Some(WebSearchModeConfig::Live)
        );
        assert_eq!(
            runtime_web_search_mode("disabled"),
            Some(WebSearchModeConfig::Disabled)
        );
        assert_eq!(runtime_web_search_mode("invalid"), None);
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_schedule_contract_keeps_accepted_response_shapes() {
        let reservation = session_thread_review_use_cases::NativeSessionSlotReservation {
            session_id: 13,
            pid: Some(77),
            cwd: PathBuf::from("C:/runtime"),
            initial_thread_id: Some("thread-13".to_string()),
        };

        let turn_response = reservation.turn_run_accepted_response();
        assert!(turn_response.accepted);
        assert_eq!(turn_response.session_id, 13);
        assert_eq!(turn_response.thread_id, Some("thread-13".to_string()));

        let review_response = reservation.review_start_accepted_response();
        assert!(review_response.accepted);
        assert_eq!(review_response.session_id, 13);
        assert_eq!(review_response.thread_id, Some("thread-13".to_string()));
        assert_eq!(
            review_response.review_thread_id,
            Some("thread-13".to_string())
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_reinsert_pending_approval_smoke() {
        let mut pending = HashMap::new();
        let action_id = "approval-7";
        let value = NativePendingApproval {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            kind: NativeApprovalKind::CommandExecution,
        };

        reinsert_pending_approval_entry(&mut pending, action_id, value.clone());

        let inserted = pending
            .get(action_id)
            .expect("pending approval should be reinserted");
        assert_eq!(inserted.thread_id, value.thread_id);
        assert_eq!(inserted.turn_id, value.turn_id);
        assert_eq!(inserted.call_id, value.call_id);
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_reinsert_pending_user_input_smoke() {
        let mut pending = HashMap::new();
        let action_id = "user-input-7";
        let value = NativePendingUserInput {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
        };

        reinsert_pending_user_input_entry(&mut pending, action_id, value.clone());

        let inserted = pending
            .get(action_id)
            .expect("pending user input should be reinserted");
        assert_eq!(inserted.thread_id, value.thread_id);
        assert_eq!(inserted.turn_id, value.turn_id);
        assert_eq!(inserted.call_id, value.call_id);
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_approval_decision_validation_requires_non_empty_value() {
        assert_eq!(
            validate_approval_decision_before_lookup(" "),
            Err("decision is required".to_string())
        );
        assert_eq!(validate_approval_decision_before_lookup("accept"), Ok(()));
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_user_input_decision_validation_is_fail_fast() {
        assert_eq!(
            validate_user_input_decision_before_lookup(""),
            Err("decision is required".to_string())
        );
        assert_eq!(
            validate_user_input_decision_before_lookup("later"),
            Err("decision must be one of: submit, cancel".to_string())
        );
        assert_eq!(
            validate_user_input_decision_before_lookup(" Submit "),
            Ok(())
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn runtime_resolved_event_shapes_smoke() {
        let approval_plan = session_thread_review_use_cases::plan_approval_response(
            "approval-1",
            session_thread_review_use_cases::ApprovalPendingKind::CommandExecution,
            "call-1",
            "turn-1",
            "accept",
            false,
            None,
        )
        .expect("approval plan should be valid");

        assert_eq!(
            approval_plan
                .resolved_event
                .get("type")
                .and_then(Value::as_str),
            Some("approval.resolved")
        );
        assert_eq!(
            approval_plan
                .resolved_event
                .get("action_id")
                .and_then(Value::as_str),
            Some("approval-1")
        );
        assert_eq!(
            approval_plan
                .resolved_event
                .get("kind")
                .and_then(Value::as_str),
            Some("command_execution")
        );
        assert!(approval_plan
            .resolved_event
            .get("decision")
            .and_then(Value::as_str)
            .is_some());

        let user_input_plan = session_thread_review_use_cases::plan_user_input_response(
            "user-input-1",
            "thread-1",
            "turn-1",
            "call-1",
            "cancel",
            HashMap::new(),
        )
        .expect("user_input plan should be valid");

        assert_eq!(
            user_input_plan
                .resolved_event
                .get("type")
                .and_then(Value::as_str),
            Some("user_input.resolved")
        );
        assert_eq!(
            user_input_plan
                .resolved_event
                .get("outcome")
                .and_then(Value::as_str),
            Some("cancelled")
        );
        assert_eq!(
            user_input_plan
                .resolved_event
                .get("error")
                .and_then(Value::as_str),
            Some("user input cancelled by user")
        );
    }
}
