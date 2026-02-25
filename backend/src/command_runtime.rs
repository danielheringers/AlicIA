#[cfg(feature = "native-codex-runtime")]
use codex_core::config::{ConfigBuilder, ConfigOverrides};
#[cfg(feature = "native-codex-runtime")]
use codex_core::mcp::auth::{oauth_login_support, McpOAuthLoginSupport};
#[cfg(feature = "native-codex-runtime")]
use codex_core::mcp::{collect_mcp_snapshot, group_tools_by_server};
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::protocol::{McpAuthStatus, McpServerRefreshConfig};
#[cfg(feature = "native-codex-runtime")]
use codex_rmcp_client::perform_oauth_login_return_url;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
#[cfg(feature = "native-codex-runtime")]
use std::time::Duration;
use std::time::Instant;
use tauri::State;
#[cfg(feature = "native-codex-runtime")]
use toml::map::Map as TomlMap;

use crate::account_runtime::{
    parse_account_login_start_runtime_result, parse_account_logout_runtime_result,
    parse_account_rate_limits_runtime_result, parse_account_read_runtime_result,
    parse_app_list_runtime_result, AccountLoginStartRequest, AccountLoginStartResponse,
    AccountLogoutResponse, AccountRateLimitsReadResponse, AccountReadRequest, AccountReadResponse,
    AppListRequest, AppListResponse,
};
#[cfg(feature = "native-codex-runtime")]
use crate::app_server_runtime::request_app_server_method;
use crate::mcp_runtime::{
    parse_mcp_server_list_runtime_result, McpLoginRequest, McpLoginResponse, McpReloadResponse,
    McpServerListResponse, McpStartupWarmupResponse,
};
use crate::models_runtime::fetch_models_for_picker;
use crate::{
    default_codex_binary, lock_active_session, resolve_codex_launch, ActiveSessionTransport,
    AppState, CodexModelListResponse, GitCommandExecutionResult, GitCommitApprovedReviewRequest,
    GitCommitApprovedReviewResponse, GitWorkspaceChange, GitWorkspaceChangesRequest,
    GitWorkspaceChangesResponse, RunCodexCommandResponse, RuntimeCapabilitiesResponse,
    RuntimeContractMetadata,
};

#[cfg(feature = "native-codex-runtime")]
type NativeReloadContext = (
    Arc<crate::codex_native_runtime::NativeCodexRuntime>,
    std::path::PathBuf,
);

#[cfg(feature = "native-codex-runtime")]
fn native_reload_context_from_session(
    session: &crate::ActiveSession,
) -> Option<NativeReloadContext> {
    match &session.transport {
        ActiveSessionTransport::Native(native) => {
            Some((Arc::clone(&native.runtime), session.cwd.clone()))
        }
    }
}

#[cfg(not(feature = "native-codex-runtime"))]
fn native_reload_context_from_session(_session: &crate::ActiveSession) -> Option<()> {
    None
}

#[cfg(feature = "native-codex-runtime")]
type NativeBinaryCwdContext = (String, std::path::PathBuf);

#[cfg(feature = "native-codex-runtime")]
fn native_binary_cwd_context_from_session(
    session: &crate::ActiveSession,
) -> Option<NativeBinaryCwdContext> {
    match &session.transport {
        ActiveSessionTransport::Native(_) => Some((session.binary.clone(), session.cwd.clone())),
    }
}

#[cfg(not(feature = "native-codex-runtime"))]
fn native_binary_cwd_context_from_session(_session: &crate::ActiveSession) -> Option<()> {
    None
}

#[cfg(feature = "native-codex-runtime")]
async fn native_runtime_and_cwd_for_mcp(
    state: &State<'_, AppState>,
) -> Result<
    (
        Arc<crate::codex_native_runtime::NativeCodexRuntime>,
        PathBuf,
    ),
    String,
> {
    let active_context = {
        let active = lock_active_session(state.inner())?;
        active.as_ref().and_then(native_reload_context_from_session)
    };

    if let Some(context) = active_context {
        return Ok(context);
    }

    let runtime = crate::codex_native_runtime::native_runtime_get_or_init(state.inner()).await?;
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    Ok((runtime, cwd))
}

#[cfg(feature = "native-codex-runtime")]
const ALICIA_NATIVE_INTERNAL_PROFILE: &str = "__alicia_native_internal";

#[cfg(feature = "native-codex-runtime")]
fn native_internal_profile_cli_overrides() -> Vec<(String, toml::Value)> {
    vec![(
        format!("profiles.{ALICIA_NATIVE_INTERNAL_PROFILE}"),
        toml::Value::Table(TomlMap::new()),
    )]
}

#[cfg(feature = "native-codex-runtime")]
fn native_internal_profile_harness_overrides(cwd: PathBuf) -> ConfigOverrides {
    ConfigOverrides {
        cwd: Some(cwd),
        config_profile: Some(ALICIA_NATIVE_INTERNAL_PROFILE.to_string()),
        ..Default::default()
    }
}

#[cfg(feature = "native-codex-runtime")]
async fn build_native_config_for_cwd(
    runtime: &crate::codex_native_runtime::NativeCodexRuntime,
    cwd: PathBuf,
    context: &str,
) -> Result<codex_core::config::Config, String> {
    ConfigBuilder::default()
        .codex_home(runtime.codex_home.clone())
        .fallback_cwd(Some(cwd.clone()))
        .cli_overrides(native_internal_profile_cli_overrides())
        .harness_overrides(native_internal_profile_harness_overrides(cwd))
        .build()
        .await
        .map_err(|error| format!("failed to build {context} config: {error}"))
}

#[cfg(feature = "native-codex-runtime")]
fn mcp_entry_from_config(
    config_entries: &serde_json::Map<String, Value>,
    name: &str,
) -> (String, Option<String>, bool, Option<String>) {
    let Some(entry) = config_entries.get(name).and_then(Value::as_object) else {
        return ("stdio".to_string(), None, true, None);
    };

    let enabled = entry
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let status_reason = entry
        .get("disabled_reason")
        .or_else(|| entry.get("disabledReason"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let transport_entry = entry.get("transport").and_then(Value::as_object);
    let transport_type = transport_entry
        .and_then(|transport| transport.get("type"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("stdio");

    let transport = match transport_type {
        "streamable_http" | "streamable-http" => "streamable-http".to_string(),
        "sse" => "sse".to_string(),
        _ => "stdio".to_string(),
    };

    let url = if transport == "streamable-http" {
        transport_entry
            .and_then(|transport| transport.get("url"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    } else {
        None
    };

    (transport, url, enabled, status_reason)
}

#[cfg(feature = "native-codex-runtime")]
fn auth_status_label(status: McpAuthStatus) -> &'static str {
    match status {
        McpAuthStatus::Unsupported => "not_logged_in",
        McpAuthStatus::NotLoggedIn => "not_logged_in",
        McpAuthStatus::BearerToken => "bearer_token",
        McpAuthStatus::OAuth => "oauth",
    }
}

#[cfg(feature = "native-codex-runtime")]
async fn collect_native_mcp_server_list(
    runtime: Arc<crate::codex_native_runtime::NativeCodexRuntime>,
    cwd: PathBuf,
    fallback_elapsed_ms: u64,
) -> Result<McpServerListResponse, String> {
    let config = build_native_config_for_cwd(runtime.as_ref(), cwd, "MCP list").await?;
    let snapshot = collect_mcp_snapshot(&config).await;
    let tools_by_server = group_tools_by_server(&snapshot.tools);

    let config_servers_value = serde_json::to_value(config.mcp_servers.get())
        .map_err(|error| format!("failed to serialize MCP server config: {error}"))?;
    let config_servers = config_servers_value
        .as_object()
        .cloned()
        .unwrap_or_default();

    let mut server_names = BTreeSet::<String>::new();
    server_names.extend(config_servers.keys().cloned());
    server_names.extend(snapshot.auth_statuses.keys().cloned());
    server_names.extend(snapshot.resources.keys().cloned());
    server_names.extend(snapshot.resource_templates.keys().cloned());
    server_names.extend(tools_by_server.keys().cloned());

    let mut data = Vec::<Value>::with_capacity(server_names.len());
    for name in server_names {
        let (transport, url, enabled, status_reason) =
            mcp_entry_from_config(&config_servers, &name);
        let auth_status = snapshot
            .auth_statuses
            .get(&name)
            .copied()
            .unwrap_or(McpAuthStatus::NotLoggedIn);

        let mut tools = tools_by_server
            .get(&name)
            .map(|tools| tools.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        tools.sort();
        tools.dedup();

        let is_connected = enabled
            && (snapshot.auth_statuses.contains_key(&name)
                || snapshot.resources.contains_key(&name)
                || snapshot.resource_templates.contains_key(&name)
                || tools_by_server.contains_key(&name));

        data.push(json!({
            "name": name,
            "transport": transport,
            "status": if is_connected { "connected" } else { "disconnected" },
            "statusReason": status_reason,
            "authStatus": auth_status_label(auth_status),
            "tools": tools,
            "url": url,
        }));
    }

    let result = json!({
        "data": data,
        "total": data.len(),
    });
    Ok(parse_mcp_server_list_runtime_result(
        &result,
        fallback_elapsed_ms,
    ))
}

fn disable_methods_for_native_transport(methods: &mut HashMap<String, bool>) {
    const NATIVE_UNSUPPORTED_METHODS: &[&str] = &["tool.call.dynamic"];

    for method in NATIVE_UNSUPPORTED_METHODS {
        methods.insert((*method).to_string(), false);
    }
}

const RUNTIME_CONTRACT_VERSION: &str = "alicia.runtime.capabilities.v1";

const RUNTIME_METHOD_KEYS: &[&str] = &[
    "thread.open",
    "thread.close",
    "thread.list",
    "thread.read",
    "thread.archive",
    "thread.unarchive",
    "thread.compact.start",
    "thread.rollback",
    "thread.fork",
    "turn.run",
    "review.start",
    "turn.steer",
    "turn.interrupt",
    "approval.respond",
    "user_input.respond",
    "tool.call.dynamic",
    "mcp.warmup",
    "mcp.list",
    "mcp.login",
    "mcp.reload",
    "app.list",
    "account.read",
    "account.login.start",
    "account.logout",
    "account.rate_limits.read",
    "account.rateLimits.read",
    "config.get",
    "config.set",
];

fn default_runtime_capabilities() -> HashMap<String, bool> {
    let mut methods: HashMap<String, bool> = RUNTIME_METHOD_KEYS
        .iter()
        .map(|method| ((*method).to_string(), true))
        .collect();
    methods.insert("tool.call.dynamic".to_string(), false);
    methods
}

#[cfg(test)]
fn extract_capabilities_contract_version(result: &serde_json::Value) -> Option<String> {
    for key in [
        "runtimeContractVersion",
        "runtime_contract_version",
        "contractVersion",
    ] {
        if let Some(version) = result.get(key) {
            if let Some(text) = version.as_str() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
            if let Some(number) = version.as_u64() {
                return Some(number.to_string());
            }
        }
    }

    if let Some(contract) = result.get("contract").and_then(|value| value.as_object()) {
        if let Some(version) = contract.get("version").and_then(|value| value.as_str()) {
            let trimmed = version.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

fn is_unsupported_method_message(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("unsupported method") || normalized.contains("method not found")
}

fn is_unsupported_method_error_for(error: &str, methods: &[&str]) -> bool {
    if !is_unsupported_method_message(error) {
        return false;
    }

    let normalized = error.to_ascii_lowercase();
    methods.iter().any(|method| {
        let dotted = method.to_ascii_lowercase();
        let slashed = dotted.replace('.', "/");
        normalized.contains(&dotted) || normalized.contains(&slashed)
    })
}
pub(crate) fn run_codex_command_impl(
    args: Vec<String>,
    cwd: Option<String>,
) -> Result<RunCodexCommandResponse, String> {
    if args.is_empty() {
        return Err("run_codex_command requires at least one argument".to_string());
    }

    let binary = default_codex_binary();
    let cwd_path = cwd.map(PathBuf::from);
    run_codex_command_with_context(&binary, args, cwd_path.as_deref())
}

fn run_codex_command_with_context(
    binary: &str,
    args: Vec<String>,
    cwd: Option<&Path>,
) -> Result<RunCodexCommandResponse, String> {
    let (program, resolved_args) = resolve_codex_launch(binary, &args)?;

    let mut command = Command::new(program);
    command.args(resolved_args);

    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let output = command.output().map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            format!("failed to run codex command: executable not found ({error})")
        } else {
            format!("failed to run codex command: {error}")
        }
    })?;

    Ok(RunCodexCommandResponse {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        status: output.status.code().unwrap_or(-1),
        success: output.status.success(),
    })
}

fn run_git_command_impl(
    mut command: Command,
    operation: &str,
) -> Result<GitCommandExecutionResult, String> {
    let output = command.output().map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            format!("failed to run git {operation}: git executable not found in PATH ({error})")
        } else {
            format!("failed to run git {operation}: {error}")
        }
    })?;

    Ok(GitCommandExecutionResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        status: output.status.code().unwrap_or(-1),
        success: output.status.success(),
    })
}

fn is_safe_git_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }

    if path.contains('\0') {
        return false;
    }

    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return false;
    }

    candidate
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
}

fn to_literal_pathspec(path: &str) -> String {
    format!(":(literal){path}")
}

pub(crate) fn git_commit_approved_review_impl(
    request: GitCommitApprovedReviewRequest,
) -> Result<GitCommitApprovedReviewResponse, String> {
    let mut paths: Vec<String> = Vec::new();
    for entry in request.paths {
        let normalized = entry.trim().to_string();
        if normalized.is_empty() {
            continue;
        }
        if !is_safe_git_path(&normalized) {
            return Err(format!(
                "git_commit_approved_review rejected unsafe path: {normalized}"
            ));
        }
        paths.push(normalized);
    }

    if paths.is_empty() {
        return Err("git_commit_approved_review requires at least one non-empty path".to_string());
    }

    let literal_pathspecs: Vec<String> =
        paths.iter().map(|path| to_literal_pathspec(path)).collect();

    let message = request.message.trim().to_string();
    if message.is_empty() {
        return Err("git_commit_approved_review requires a non-empty message".to_string());
    }

    let cwd = request
        .cwd
        .as_deref()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(|| {
            env::current_dir().map_err(|error| {
                format!("failed to resolve current directory for git commit: {error}")
            })
        })?;
    if !cwd.is_absolute() {
        return Err("git_commit_approved_review requires an absolute cwd".to_string());
    }

    let mut git_add = Command::new("git");
    git_add
        .current_dir(&cwd)
        .arg("add")
        .arg("-A")
        .arg("--")
        .args(literal_pathspecs.iter());
    let add = run_git_command_impl(git_add, "add")?;

    let commit = if add.success {
        let mut git_commit = Command::new("git");
        git_commit
            .current_dir(&cwd)
            .arg("commit")
            .arg("-m")
            .arg(&message)
            .arg("--")
            .args(literal_pathspecs.iter());
        run_git_command_impl(git_commit, "commit")?
    } else {
        GitCommandExecutionResult {
            stdout: String::new(),
            stderr: "git commit skipped because git add failed".to_string(),
            status: -1,
            success: false,
        }
    };

    Ok(GitCommitApprovedReviewResponse {
        success: add.success && commit.success,
        add,
        commit,
    })
}

fn resolve_git_workspace_cwd(
    state: State<'_, AppState>,
    request: GitWorkspaceChangesRequest,
) -> Result<PathBuf, String> {
    let active_cwd = {
        let active = lock_active_session(state.inner())?;
        active.as_ref().map(|session| session.cwd.clone())
    };

    if let Some(cwd) = active_cwd {
        return Ok(cwd);
    }

    if let Some(cwd) = request
        .cwd
        .as_deref()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        return Ok(PathBuf::from(cwd));
    }

    env::current_dir().map_err(|error| {
        format!("failed to resolve current directory for git workspace changes: {error}")
    })
}

fn validate_workspace_cwd(cwd: &Path) -> Result<(), String> {
    if !cwd.exists() {
        return Err(format!(
            "git_workspace_changes invalid cwd '{}': path does not exist",
            cwd.display()
        ));
    }

    if !cwd.is_dir() {
        return Err(format!(
            "git_workspace_changes invalid cwd '{}': path is not a directory",
            cwd.display()
        ));
    }

    Ok(())
}

fn git_result_details(result: &GitCommandExecutionResult) -> String {
    let stderr = result.stderr.trim();
    if !stderr.is_empty() {
        return stderr.to_string();
    }

    let stdout = result.stdout.trim();
    if !stdout.is_empty() {
        return stdout.to_string();
    }

    format!("exit status {}", result.status)
}

fn ensure_git_repository(cwd: &Path) -> Result<(), String> {
    let mut git_rev_parse = Command::new("git");
    git_rev_parse
        .current_dir(cwd)
        .arg("rev-parse")
        .arg("--is-inside-work-tree");
    let rev_parse = run_git_command_impl(git_rev_parse, "rev-parse")?;

    if !rev_parse.success || rev_parse.stdout.trim() != "true" {
        return Err(format!(
            "git_workspace_changes requires a git repository at '{}': {}",
            cwd.display(),
            git_result_details(&rev_parse)
        ));
    }

    Ok(())
}

fn run_git_status_porcelain(cwd: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .arg("status")
        .arg("--porcelain=v1")
        .arg("-z")
        .output()
        .map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                format!("failed to run git status: git executable not found in PATH ({error})")
            } else {
                format!("failed to run git status: {error}")
            }
        })?;

    if !output.status.success() {
        let result = GitCommandExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            status: output.status.code().unwrap_or(-1),
            success: false,
        };

        return Err(format!(
            "failed to collect git workspace changes at '{}': {}",
            cwd.display(),
            git_result_details(&result)
        ));
    }

    Ok(output.stdout)
}

fn classify_git_status(code: &str) -> &'static str {
    if code == "??" {
        return "untracked";
    }

    // Porcelain v1 conflict matrix.
    // Source: git-status short format (XY status for unmerged paths).
    if matches!(code, "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU") {
        return "unmerged";
    }

    let mut chars = code.chars();
    let index = chars.next().unwrap_or(' ');
    let worktree = chars.next().unwrap_or(' ');

    if index == 'U' || worktree == 'U' {
        return "unmerged";
    }
    if index == 'R' || worktree == 'R' {
        return "renamed";
    }
    if index == 'C' || worktree == 'C' {
        return "copied";
    }
    if index == 'D' || worktree == 'D' {
        return "deleted";
    }
    if index == 'A' || worktree == 'A' {
        return "added";
    }
    if index == 'M' || worktree == 'M' || index == 'T' || worktree == 'T' {
        return "modified";
    }

    "unknown"
}

fn parse_status_path(path: &[u8]) -> String {
    String::from_utf8_lossy(path).to_string()
}

fn parse_git_status_porcelain(output: &[u8]) -> Result<Vec<GitWorkspaceChange>, String> {
    let mut files = Vec::new();
    let mut cursor = 0usize;

    while cursor < output.len() {
        let Some(relative_end) = output[cursor..].iter().position(|byte| *byte == b'\0') else {
            return Err("malformed git status output: missing NUL terminator".to_string());
        };

        let entry_end = cursor + relative_end;
        let entry = &output[cursor..entry_end];
        cursor = entry_end + 1;

        if entry.is_empty() {
            continue;
        }

        if entry.len() < 3 {
            return Err(format!(
                "malformed git status entry: '{}'",
                String::from_utf8_lossy(entry)
            ));
        }

        let code = std::str::from_utf8(&entry[0..2]).map_err(|_| {
            format!(
                "malformed git status code in entry '{}'",
                String::from_utf8_lossy(entry)
            )
        })?;
        if code == "!!" {
            continue;
        }

        if entry[2] != b' ' {
            return Err(format!(
                "malformed git status entry: '{}'",
                String::from_utf8_lossy(entry)
            ));
        }

        let primary_path = &entry[3..];
        if primary_path.is_empty() {
            return Err(format!(
                "malformed git status entry (missing path): '{}'",
                String::from_utf8_lossy(entry)
            ));
        }

        let mut from_path = None;
        let path = parse_status_path(primary_path);

        if code.contains('R') || code.contains('C') {
            let Some(relative_source_end) = output[cursor..].iter().position(|byte| *byte == b'\0')
            else {
                return Err(
                    "malformed git status output: missing rename/copy source path".to_string(),
                );
            };
            let source_end = cursor + relative_source_end;
            let source_path = &output[cursor..source_end];
            cursor = source_end + 1;

            if source_path.is_empty() {
                return Err(
                    "malformed git status output: empty rename/copy source path".to_string()
                );
            }

            from_path = Some(parse_status_path(source_path));
        }

        files.push(GitWorkspaceChange {
            path,
            status: classify_git_status(code).to_string(),
            code: code.to_string(),
            from_path,
        });
    }

    Ok(files)
}

pub(crate) fn git_workspace_changes_impl(
    state: State<'_, AppState>,
    request: GitWorkspaceChangesRequest,
) -> Result<GitWorkspaceChangesResponse, String> {
    let cwd = resolve_git_workspace_cwd(state, request)?;
    validate_workspace_cwd(&cwd)?;
    ensure_git_repository(&cwd)?;

    let status_output = run_git_status_porcelain(&cwd)?;
    let files = parse_git_status_porcelain(&status_output)?;

    Ok(GitWorkspaceChangesResponse {
        cwd: cwd.to_string_lossy().to_string(),
        total: files.len(),
        files,
    })
}

pub(crate) fn codex_models_list_impl(
    state: State<'_, AppState>,
) -> Result<CodexModelListResponse, String> {
    let (binary, cwd) = {
        let active = lock_active_session(state.inner())?;
        if let Some(session) = active.as_ref() {
            (session.binary.clone(), session.cwd.clone())
        } else {
            (
                default_codex_binary(),
                env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            )
        }
    };

    let models = fetch_models_for_picker(&binary, &cwd)?;
    Ok(CodexModelListResponse { data: models })
}

pub(crate) async fn codex_runtime_capabilities_impl(
    state: State<'_, AppState>,
) -> Result<RuntimeCapabilitiesResponse, String> {
    let mut methods = default_runtime_capabilities();
    let native_transport_active = {
        let active = lock_active_session(state.inner())?;
        active
            .as_ref()
            .is_some_and(|session| matches!(session.transport, ActiveSessionTransport::Native(_)))
    };
    if native_transport_active {
        disable_methods_for_native_transport(&mut methods);
    }

    Ok(RuntimeCapabilitiesResponse {
        methods,
        contract: Some(RuntimeContractMetadata {
            version: RUNTIME_CONTRACT_VERSION.to_string(),
        }),
    })
}

pub(crate) async fn codex_wait_for_mcp_startup_impl(
    state: State<'_, AppState>,
) -> Result<McpStartupWarmupResponse, String> {
    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let (runtime, cwd) = native_runtime_and_cwd_for_mcp(&state).await?;
        let list = collect_native_mcp_server_list(runtime, cwd, 0).await?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let ready_servers = list
            .data
            .into_iter()
            .filter(|entry| entry.status == "connected")
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        let total_ready = ready_servers.len();

        Ok(McpStartupWarmupResponse {
            ready_servers,
            total_ready,
            elapsed_ms,
        })
    }

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = state;
        Err("mcp warmup requires native runtime support in this build".to_string())
    }
}

pub(crate) async fn codex_app_list_impl(
    state: State<'_, AppState>,
    request: AppListRequest,
) -> Result<AppListResponse, String> {
    let native_binary_cwd_context = {
        let active = lock_active_session(state.inner())?;
        if let Some(session) = active.as_ref() {
            native_binary_cwd_context_from_session(session)
        } else {
            None
        }
    };

    let mut payload = serde_json::Map::new();
    if let Some(cursor) = request
        .cursor
        .as_deref()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        payload.insert("cursor".to_string(), json!(cursor));
    }
    if let Some(limit) = request.limit {
        payload.insert("limit".to_string(), json!(limit));
    }
    if let Some(thread_id) = request
        .thread_id
        .as_deref()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        payload.insert("threadId".to_string(), json!(thread_id));
    }
    if request.force_refetch {
        payload.insert("forceRefetch".to_string(), json!(true));
    }

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("app list requires an active codex session".to_string());
    };
    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = (native_binary_cwd_context, payload);
        return Err("app list requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let result = request_app_server_method(
            &binary,
            &cwd,
            "app/list",
            serde_json::Value::Object(payload),
            Duration::from_secs(90),
        );
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;

        match result {
            Ok(result) => Ok(parse_app_list_runtime_result(&result, elapsed_ms)),
            Err(error) => {
                if is_unsupported_method_error_for(&error, &["app.list", "app/list"]) {
                    Ok(AppListResponse {
                        data: Vec::new(),
                        next_cursor: None,
                        total: 0,
                        elapsed_ms,
                    })
                } else {
                    Err(error)
                }
            }
        }
    }
}

pub(crate) async fn codex_account_read_impl(
    state: State<'_, AppState>,
    request: AccountReadRequest,
) -> Result<AccountReadResponse, String> {
    let native_binary_cwd_context = {
        let active = lock_active_session(state.inner())?;
        if let Some(session) = active.as_ref() {
            native_binary_cwd_context_from_session(session)
        } else {
            None
        }
    };

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("account read requires an active codex session".to_string());
    };
    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = (native_binary_cwd_context, request);
        return Err("account read requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let result = request_app_server_method(
            &binary,
            &cwd,
            "account/read",
            json!({
                "refreshToken": request.refresh_token,
            }),
            Duration::from_secs(90),
        )?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(parse_account_read_runtime_result(&result, elapsed_ms))
    }
}

pub(crate) async fn codex_account_login_start_impl(
    state: State<'_, AppState>,
    request: AccountLoginStartRequest,
) -> Result<AccountLoginStartResponse, String> {
    let native_binary_cwd_context = {
        let active = lock_active_session(state.inner())?;
        if let Some(session) = active.as_ref() {
            native_binary_cwd_context_from_session(session)
        } else {
            None
        }
    };

    let login_type = request.login_type.trim();
    if login_type.is_empty() {
        return Err("type is required".to_string());
    }

    let mut payload = serde_json::Map::new();
    if login_type.eq_ignore_ascii_case("chatgpt") {
        payload.insert("type".to_string(), json!("chatgpt"));
        payload.insert("authMode".to_string(), json!("chatgpt"));
    } else if login_type.eq_ignore_ascii_case("apikey")
        || login_type.eq_ignore_ascii_case("api_key")
        || login_type.eq_ignore_ascii_case("apiKey")
    {
        let api_key = request
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .ok_or_else(|| "apiKey is required for type=apiKey".to_string())?;
        payload.insert("type".to_string(), json!("apiKey"));
        payload.insert("authMode".to_string(), json!("api_key"));
        payload.insert("apiKey".to_string(), json!(api_key));
    } else {
        return Err("unsupported account login type".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("account login requires an active codex session".to_string());
    };
    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = (native_binary_cwd_context, payload);
        return Err("account login requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        if login_type.eq_ignore_ascii_case("chatgpt") {
            let result = run_codex_command_with_context(
                &binary,
                vec!["login".to_string()],
                Some(cwd.as_path()),
            )?;
            if !result.success {
                let details = if result.stderr.trim().is_empty() {
                    result.stdout.trim().to_string()
                } else {
                    result.stderr.trim().to_string()
                };
                return Err(format!("codex login failed: {details}"));
            }

            let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
            return Ok(AccountLoginStartResponse {
                login_type: "chatgpt".to_string(),
                login_id: None,
                auth_url: None,
                started: true,
                elapsed_ms,
            });
        }

        let result = request_app_server_method(
            &binary,
            &cwd,
            "account/login/start",
            serde_json::Value::Object(payload),
            Duration::from_secs(90),
        )?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(parse_account_login_start_runtime_result(
            &result, elapsed_ms,
        ))
    }
}

pub(crate) async fn codex_account_logout_impl(
    state: State<'_, AppState>,
) -> Result<AccountLogoutResponse, String> {
    let native_binary_cwd_context = {
        let active = lock_active_session(state.inner())?;
        if let Some(session) = active.as_ref() {
            native_binary_cwd_context_from_session(session)
        } else {
            None
        }
    };

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("account logout requires an active codex session".to_string());
    };
    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = native_binary_cwd_context;
        return Err("account logout requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let result = request_app_server_method(
            &binary,
            &cwd,
            "account/logout",
            serde_json::Value::Null,
            Duration::from_secs(90),
        )?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(parse_account_logout_runtime_result(&result, elapsed_ms))
    }
}

pub(crate) async fn codex_account_rate_limits_read_impl(
    state: State<'_, AppState>,
) -> Result<AccountRateLimitsReadResponse, String> {
    let native_binary_cwd_context = {
        let active = lock_active_session(state.inner())?;
        if let Some(session) = active.as_ref() {
            native_binary_cwd_context_from_session(session)
        } else {
            None
        }
    };

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("account rate-limits requires an active codex session".to_string());
    };
    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = native_binary_cwd_context;
        return Err(
            "account rate-limits requires native runtime support in this build".to_string(),
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let result = request_app_server_method(
            &binary,
            &cwd,
            "account/rateLimits/read",
            json!({}),
            Duration::from_secs(90),
        )?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(parse_account_rate_limits_runtime_result(
            &result, elapsed_ms,
        ))
    }
}

pub(crate) async fn codex_mcp_list_impl(
    state: State<'_, AppState>,
) -> Result<McpServerListResponse, String> {
    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let (runtime, cwd) = native_runtime_and_cwd_for_mcp(&state).await?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        collect_native_mcp_server_list(runtime, cwd, elapsed_ms).await
    }

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = state;
        Err("mcp list requires native runtime support in this build".to_string())
    }
}

pub(crate) async fn codex_mcp_login_impl(
    state: State<'_, AppState>,
    request: McpLoginRequest,
) -> Result<McpLoginResponse, String> {
    let name = request.name.trim().to_string();
    if name.is_empty() {
        return Err("name is required".to_string());
    }

    let scopes: Vec<String> = request
        .scopes
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect();

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let (runtime, cwd) = native_runtime_and_cwd_for_mcp(&state).await?;
        let config = build_native_config_for_cwd(runtime.as_ref(), cwd, "MCP login").await?;

        let server_config = config
            .mcp_servers
            .get()
            .get(&name)
            .ok_or_else(|| format!("mcp server `{name}` is not configured"))?;
        let timeout_secs = request
            .timeout_secs
            .map(|value| i64::try_from(value).map_err(|_| "timeoutSecs is too large".to_string()))
            .transpose()?;

        let oauth_config = match oauth_login_support(&server_config.transport).await {
            McpOAuthLoginSupport::Supported(config) => config,
            McpOAuthLoginSupport::Unsupported => {
                return Err(format!("mcp server `{name}` does not support oauth login"));
            }
            McpOAuthLoginSupport::Unknown(error) => {
                return Err(format!(
                    "failed to determine oauth login support for `{name}`: {error}"
                ));
            }
        };

        let login = perform_oauth_login_return_url(
            &name,
            oauth_config.url.as_str(),
            config.mcp_oauth_credentials_store_mode,
            oauth_config.http_headers,
            oauth_config.env_http_headers,
            &scopes,
            timeout_secs,
            config.mcp_oauth_callback_port,
        )
        .await
        .map_err(|error| format!("failed to start mcp oauth login for `{name}`: {error}"))?;

        let (authorization_url, completion) = login.into_parts();
        tauri::async_runtime::spawn(async move {
            let _ = completion.await;
        });

        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(McpLoginResponse {
            name,
            authorization_url: Some(authorization_url),
            started: true,
            elapsed_ms,
        })
    }

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = (state, scopes);
        Err("mcp login requires native runtime support in this build".to_string())
    }
}

pub(crate) async fn codex_mcp_reload_impl(
    state: State<'_, AppState>,
) -> Result<McpReloadResponse, String> {
    let native_reload_context = {
        let active = lock_active_session(state.inner())?;
        if let Some(session) = active.as_ref() {
            native_reload_context_from_session(session)
        } else {
            None
        }
    };

    #[cfg(feature = "native-codex-runtime")]
    let Some((runtime, cwd)) = native_reload_context
    else {
        return Err("mcp reload requires an active codex session".to_string());
    };
    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = native_reload_context;
        return Err("mcp reload requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let config = build_native_config_for_cwd(runtime.as_ref(), cwd, "MCP reload").await?;

        let mcp_servers = serde_json::to_value(config.mcp_servers.get())
            .map_err(|error| format!("failed to serialize MCP servers: {error}"))?;
        let mcp_oauth_credentials_store_mode =
            serde_json::to_value(config.mcp_oauth_credentials_store_mode).map_err(|error| {
                format!("failed to serialize MCP OAuth credentials store mode: {error}")
            })?;

        let refresh_config = McpServerRefreshConfig {
            mcp_servers,
            mcp_oauth_credentials_store_mode,
        };
        runtime
            .thread_manager
            .refresh_mcp_servers(refresh_config)
            .await;

        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(McpReloadResponse {
            reloaded: true,
            elapsed_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_git_status, default_runtime_capabilities, disable_methods_for_native_transport,
        extract_capabilities_contract_version, parse_git_status_porcelain,
    };
    use serde_json::json;

    #[test]
    fn parse_untracked_entry() {
        let parsed =
            parse_git_status_porcelain(b"?? src/new_file.rs\0").expect("entry should parse");
        let entry = &parsed[0];

        assert_eq!(entry.path, "src/new_file.rs");
        assert_eq!(entry.status, "untracked");
        assert_eq!(entry.code, "??");
        assert!(entry.from_path.is_none());
    }

    #[test]
    fn parse_renamed_entry_with_origin() {
        let parsed = parse_git_status_porcelain(b"R  src/new_name.rs\0src/old_name.rs\0")
            .expect("entry should parse");
        let entry = &parsed[0];

        assert_eq!(entry.path, "src/new_name.rs");
        assert_eq!(entry.status, "renamed");
        assert_eq!(entry.code, "R ");
        assert_eq!(entry.from_path.as_deref(), Some("src/old_name.rs"));
    }

    #[test]
    fn parse_unmerged_entry() {
        let parsed =
            parse_git_status_porcelain(b"UU src/conflict.rs\0").expect("entry should parse");
        let entry = &parsed[0];

        assert_eq!(entry.path, "src/conflict.rs");
        assert_eq!(entry.status, "unmerged");
        assert_eq!(entry.code, "UU");
    }

    #[test]
    fn parse_status_output_with_multiple_entries() {
        let parsed = parse_git_status_porcelain(b"M  src/main.rs\0?? src/new.rs\0D  src/old.rs\0")
            .expect("output should parse");

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].status, "modified");
        assert_eq!(parsed[1].status, "untracked");
        assert_eq!(parsed[2].status, "deleted");
    }

    #[test]
    fn parse_malformed_entry_errors() {
        let error = parse_git_status_porcelain(b"X\0")
            .expect_err("malformed output should return an error");
        assert!(error.contains("malformed git status entry"));
    }

    #[test]
    fn parse_paths_with_spaces() {
        let parsed = parse_git_status_porcelain(b"M  src/folder with spaces/file name.rs\0")
            .expect("output should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path, "src/folder with spaces/file name.rs");
    }

    #[test]
    fn classify_common_statuses() {
        assert_eq!(classify_git_status("M "), "modified");
        assert_eq!(classify_git_status("A "), "added");
        assert_eq!(classify_git_status("D "), "deleted");
        assert_eq!(classify_git_status("C "), "copied");
        assert_eq!(classify_git_status("DD"), "unmerged");
        assert_eq!(classify_git_status("AA"), "unmerged");
        assert_eq!(classify_git_status("UU"), "unmerged");
    }
    #[test]
    fn extract_contract_version_from_contract_version_field() {
        let result = json!({
            "contractVersion": "runtime-contract-v3",
        });

        let version = extract_capabilities_contract_version(&result);
        assert_eq!(version.as_deref(), Some("runtime-contract-v3"));
    }

    #[test]
    fn extract_contract_version_from_runtime_contract_version_field() {
        let result = json!({
            "runtimeContractVersion": 7,
        });

        let version = extract_capabilities_contract_version(&result);
        assert_eq!(version.as_deref(), Some("7"));
    }

    #[test]
    fn extract_contract_version_from_nested_contract_object() {
        let result = json!({
            "contract": {
                "version": "runtime-contract-v4",
            }
        });

        let version = extract_capabilities_contract_version(&result);
        assert_eq!(version.as_deref(), Some("runtime-contract-v4"));
    }

    #[test]
    fn extract_contract_version_returns_none_when_missing() {
        let result = json!({
            "methods": {
                "thread.open": true,
            }
        });

        let version = extract_capabilities_contract_version(&result);
        assert!(version.is_none());
    }

    #[test]
    fn native_transport_keeps_supported_methods_enabled() {
        let mut methods = default_runtime_capabilities();

        disable_methods_for_native_transport(&mut methods);

        for method in [
            "mcp.warmup",
            "mcp.list",
            "mcp.login",
            "mcp.reload",
            "app.list",
            "account.read",
            "account.login.start",
            "account.logout",
            "account.rate_limits.read",
            "account.rateLimits.read",
        ] {
            assert_eq!(
                methods.get(method),
                Some(&true),
                "method should stay enabled: {method}"
            );
        }
    }

    #[test]
    fn native_transport_still_disables_unsupported_methods() {
        let mut methods = default_runtime_capabilities();

        disable_methods_for_native_transport(&mut methods);

        assert_eq!(methods.get("tool.call.dynamic"), Some(&false));
    }
}
