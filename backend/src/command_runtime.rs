use std::collections::HashMap;
use std::env;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use tauri::State;

use crate::account_runtime::{
    AccountLoginStartRequest, AccountLoginStartResponse, AccountLogoutResponse,
    AccountRateLimitsReadResponse, AccountReadRequest, AccountReadResponse, AppListRequest,
    AppListResponse,
};
use crate::application::account_mcp::use_cases as account_mcp_use_cases;
use crate::application::workspace::use_cases as workspace_use_cases;
use crate::generated::runtime_contract::{RUNTIME_CONTRACT_VERSION, RUNTIME_METHOD_KEYS};
use crate::interface::tauri::dto::CodexModelListResponse;
use crate::mcp_runtime::{
    McpLoginRequest, McpLoginResponse, McpReloadResponse, McpServerListResponse,
    McpStartupWarmupResponse,
};
use crate::models_runtime::fetch_models_for_picker;
use crate::{
    default_codex_binary, lock_active_session, resolve_codex_launch, ActiveSessionTransport,
    AppState, CodexWorkspaceCreateDirectoryRequest, CodexWorkspaceCreateDirectoryResponse,
    CodexWorkspaceListDirectoryRequest, CodexWorkspaceListDirectoryResponse,
    CodexWorkspaceReadFileRequest, CodexWorkspaceReadFileResponse,
    CodexWorkspaceRenameEntryRequest, CodexWorkspaceRenameEntryResponse,
    CodexWorkspaceWriteFileRequest, CodexWorkspaceWriteFileResponse, GitCommandExecutionResult,
    GitCommitApprovedReviewRequest, GitCommitApprovedReviewResponse, GitWorkspaceChange,
    GitWorkspaceChangesRequest, GitWorkspaceChangesResponse, RunCodexCommandResponse,
    RuntimeCapabilitiesResponse, RuntimeContractMetadata,
};
fn disable_methods_for_native_transport(methods: &mut HashMap<String, bool>) {
    const NATIVE_UNSUPPORTED_METHODS: &[&str] = &["tool.call.dynamic"];

    for method in NATIVE_UNSUPPORTED_METHODS {
        methods.insert((*method).to_string(), false);
    }
}

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

#[cfg(test)]
fn list_workspace_directory_within_workspace(
    workspace_root: &Path,
    workspace_cwd: &Path,
    requested_path: Option<&str>,
    operation: &str,
) -> Result<CodexWorkspaceListDirectoryResponse, String> {
    workspace_use_cases::list_workspace_directory_within_workspace(
        workspace_root,
        workspace_cwd,
        requested_path,
        operation,
    )
}

#[cfg(test)]
fn create_workspace_directory_within_workspace(
    workspace_root: &Path,
    relative_path: &Path,
    operation: &str,
) -> Result<(), String> {
    workspace_use_cases::create_workspace_directory_within_workspace(
        workspace_root,
        relative_path,
        operation,
    )
}

#[cfg(test)]
fn rename_workspace_entry_within_workspace(
    workspace_root: &Path,
    workspace_cwd: &Path,
    requested_path: &str,
    new_name: &str,
    operation: &str,
) -> Result<String, String> {
    workspace_use_cases::rename_workspace_entry_within_workspace(
        workspace_root,
        workspace_cwd,
        requested_path,
        new_name,
        operation,
    )
}
pub(crate) fn codex_workspace_read_file_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceReadFileRequest,
) -> Result<CodexWorkspaceReadFileResponse, String> {
    workspace_use_cases::codex_workspace_read_file_impl(state, request)
}

pub(crate) fn codex_workspace_write_file_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceWriteFileRequest,
) -> Result<CodexWorkspaceWriteFileResponse, String> {
    workspace_use_cases::codex_workspace_write_file_impl(state, request)
}

pub(crate) fn codex_workspace_create_directory_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceCreateDirectoryRequest,
) -> Result<CodexWorkspaceCreateDirectoryResponse, String> {
    workspace_use_cases::codex_workspace_create_directory_impl(state, request)
}

pub(crate) fn codex_workspace_list_directory_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceListDirectoryRequest,
) -> Result<CodexWorkspaceListDirectoryResponse, String> {
    workspace_use_cases::codex_workspace_list_directory_impl(state, request)
}

pub(crate) fn codex_workspace_rename_entry_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceRenameEntryRequest,
) -> Result<CodexWorkspaceRenameEntryResponse, String> {
    workspace_use_cases::codex_workspace_rename_entry_impl(state, request)
}

pub(crate) fn pick_workspace_folder_impl() -> Option<String> {
    rfd::FileDialog::new()
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string())
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
    account_mcp_use_cases::codex_wait_for_mcp_startup_impl(state).await
}

pub(crate) async fn codex_app_list_impl(
    state: State<'_, AppState>,
    request: AppListRequest,
) -> Result<AppListResponse, String> {
    account_mcp_use_cases::codex_app_list_impl(state, request).await
}

pub(crate) async fn codex_account_read_impl(
    state: State<'_, AppState>,
    request: AccountReadRequest,
) -> Result<AccountReadResponse, String> {
    account_mcp_use_cases::codex_account_read_impl(state, request).await
}

pub(crate) async fn codex_account_login_start_impl(
    state: State<'_, AppState>,
    request: AccountLoginStartRequest,
) -> Result<AccountLoginStartResponse, String> {
    account_mcp_use_cases::codex_account_login_start_impl(state, request).await
}

pub(crate) async fn codex_account_logout_impl(
    state: State<'_, AppState>,
) -> Result<AccountLogoutResponse, String> {
    account_mcp_use_cases::codex_account_logout_impl(state).await
}

pub(crate) async fn codex_account_rate_limits_read_impl(
    state: State<'_, AppState>,
) -> Result<AccountRateLimitsReadResponse, String> {
    account_mcp_use_cases::codex_account_rate_limits_read_impl(state).await
}

pub(crate) async fn codex_mcp_list_impl(
    state: State<'_, AppState>,
) -> Result<McpServerListResponse, String> {
    account_mcp_use_cases::codex_mcp_list_impl(state).await
}

pub(crate) async fn codex_mcp_login_impl(
    state: State<'_, AppState>,
    request: McpLoginRequest,
) -> Result<McpLoginResponse, String> {
    account_mcp_use_cases::codex_mcp_login_impl(state, request).await
}

pub(crate) async fn codex_mcp_reload_impl(
    state: State<'_, AppState>,
) -> Result<McpReloadResponse, String> {
    account_mcp_use_cases::codex_mcp_reload_impl(state).await
}
#[cfg(test)]
mod tests {
    use super::{
        classify_git_status, create_workspace_directory_within_workspace,
        default_runtime_capabilities, disable_methods_for_native_transport,
        extract_capabilities_contract_version, list_workspace_directory_within_workspace,
        parse_git_status_porcelain, rename_workspace_entry_within_workspace,
    };
    use crate::CodexWorkspaceListDirectoryEntryKind;
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "alicia_backend_command_runtime_{label}_{}_{}",
            std::process::id(),
            now
        ))
    }

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

    #[test]
    fn runtime_capabilities_include_neuro_methods() {
        let methods = default_runtime_capabilities();
        for method in [
            "neuro.runtime.diagnose",
            "neuro.search.objects",
            "neuro.get.source",
            "neuro.update.source",
            "neuro.adt.server.list",
            "neuro.adt.server.upsert",
            "neuro.adt.server.remove",
            "neuro.adt.server.select",
            "neuro.adt.server.connect",
            "neuro.adt.list.packages",
            "neuro.adt.list.namespaces",
            "neuro.adt.explorer.state.get",
            "neuro.adt.explorer.state.patch",
            "neuro.adt.list.objects",
            "neuro.adt.list.package_inventory",
            "neuro.ws.request",
            "neuro.mcp.list_tools",
            "neuro.mcp.invoke",
        ] {
            assert_eq!(
                methods.get(method),
                Some(&true),
                "neuro capability should stay enabled: {method}"
            );
        }
    }

    #[test]
    fn runtime_capabilities_include_workspace_file_methods() {
        let methods = default_runtime_capabilities();
        for method in [
            "workspace.file.read",
            "workspace.file.write",
            "workspace.directory.create",
            "workspace.directory.list",
            "workspace.entry.rename",
        ] {
            assert_eq!(
                methods.get(method),
                Some(&true),
                "workspace capability should stay enabled: {method}"
            );
        }
    }

    #[test]
    fn workspace_directory_list_orders_directories_before_files() {
        let temp_root = unique_temp_path("list_ordering");
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(workspace.join("alpha_empty")).expect("alpha directory should exist");
        fs::create_dir_all(workspace.join("zeta_nested")).expect("zeta directory should exist");
        fs::write(workspace.join("zeta_nested/item.txt"), "x")
            .expect("nested file should be created");
        fs::write(workspace.join("Beta.txt"), "x").expect("file should be created");
        fs::write(workspace.join("omega.txt"), "x").expect("file should be created");

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        let response = list_workspace_directory_within_workspace(
            &workspace_root,
            &workspace_root,
            None,
            "codex_workspace_list_directory",
        )
        .expect("listing should succeed");

        assert_eq!(response.path, "");
        assert_eq!(response.entries.len(), 4);

        assert_eq!(response.entries[0].name, "alpha_empty");
        assert_eq!(response.entries[0].path, "alpha_empty");
        assert!(matches!(
            &response.entries[0].kind,
            CodexWorkspaceListDirectoryEntryKind::Directory
        ));
        assert_eq!(response.entries[0].has_children, Some(false));

        assert_eq!(response.entries[1].name, "zeta_nested");
        assert!(matches!(
            &response.entries[1].kind,
            CodexWorkspaceListDirectoryEntryKind::Directory
        ));
        assert_eq!(response.entries[1].has_children, Some(true));

        assert_eq!(response.entries[2].name, "Beta.txt");
        assert!(matches!(
            &response.entries[2].kind,
            CodexWorkspaceListDirectoryEntryKind::File
        ));
        assert_eq!(response.entries[2].has_children, None);

        assert_eq!(response.entries[3].name, "omega.txt");
        assert!(matches!(
            &response.entries[3].kind,
            CodexWorkspaceListDirectoryEntryKind::File
        ));
        assert_eq!(response.entries[3].has_children, None);

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn workspace_directory_list_rejects_non_normal_relative_path() {
        let temp_root = unique_temp_path("list_reject_non_normal");
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace should be created");

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        let error = list_workspace_directory_within_workspace(
            &workspace_root,
            &workspace_root,
            Some("../outside"),
            "codex_workspace_list_directory",
        )
        .expect_err("path traversal should be rejected");
        assert!(error.contains("non-normal components are not allowed"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn workspace_directory_list_returns_relative_paths_for_nested_directory() {
        let temp_root = unique_temp_path("list_nested");
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(workspace.join("nested/deeper"))
            .expect("nested directories should exist");
        fs::write(workspace.join("nested/file.txt"), "x").expect("file should be created");

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        let response = list_workspace_directory_within_workspace(
            &workspace_root,
            &workspace_root,
            Some("nested"),
            "codex_workspace_list_directory",
        )
        .expect("listing should succeed");

        assert_eq!(response.path, "nested");
        assert_eq!(response.entries.len(), 2);
        assert_eq!(response.entries[0].path, "nested/deeper");
        assert_eq!(response.entries[1].path, "nested/file.txt");

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn workspace_directory_create_is_idempotent_for_existing_directory() {
        let temp_root = unique_temp_path("mkdir_idempotent");
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace should be created");

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        let relative_path = Path::new("nested/child");
        create_workspace_directory_within_workspace(
            &workspace_root,
            relative_path,
            "codex_workspace_create_directory",
        )
        .expect("first create should succeed");
        create_workspace_directory_within_workspace(
            &workspace_root,
            relative_path,
            "codex_workspace_create_directory",
        )
        .expect("second create should be idempotent");

        assert!(workspace_root.join(relative_path).is_dir());
        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn workspace_directory_create_rejects_existing_file_target() {
        let temp_root = unique_temp_path("mkdir_existing_file");
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace should be created");
        fs::create_dir_all(workspace.join("nested")).expect("parent should be created");
        fs::write(workspace.join("nested/child"), "content").expect("file should be created");

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        let error = create_workspace_directory_within_workspace(
            &workspace_root,
            Path::new("nested/child"),
            "codex_workspace_create_directory",
        )
        .expect_err("existing file target should fail");
        assert!(error.contains("target exists and is not a directory"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn workspace_entry_rename_succeeds_for_file_and_directory() {
        let temp_root = unique_temp_path("rename_success");
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(workspace.join("nested")).expect("nested directory should be created");
        fs::write(workspace.join("nested/old.txt"), "content").expect("file should be created");
        fs::create_dir_all(workspace.join("nested/old_dir"))
            .expect("old directory should be created");

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");

        let renamed_file = rename_workspace_entry_within_workspace(
            &workspace_root,
            &workspace_root,
            "nested/old.txt",
            "new.txt",
            "codex_workspace_rename_entry",
        )
        .expect("file rename should succeed");
        assert_eq!(renamed_file, "nested/new.txt");
        assert!(workspace_root.join("nested/new.txt").exists());
        assert!(!workspace_root.join("nested/old.txt").exists());

        let renamed_directory = rename_workspace_entry_within_workspace(
            &workspace_root,
            &workspace_root,
            "nested/old_dir",
            "new_dir",
            "codex_workspace_rename_entry",
        )
        .expect("directory rename should succeed");
        assert_eq!(renamed_directory, "nested/new_dir");
        assert!(workspace_root.join("nested/new_dir").is_dir());
        assert!(!workspace_root.join("nested/old_dir").exists());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn workspace_entry_rename_rejects_invalid_new_name() {
        let temp_root = unique_temp_path("rename_invalid_name");
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace should be created");
        fs::write(workspace.join("old.txt"), "content").expect("file should be created");

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        let error = rename_workspace_entry_within_workspace(
            &workspace_root,
            &workspace_root,
            "old.txt",
            "..",
            "codex_workspace_rename_entry",
        )
        .expect_err("invalid newName should fail");
        assert!(error.contains("'.' and '..' are not allowed"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn workspace_entry_rename_rejects_existing_destination() {
        let temp_root = unique_temp_path("rename_overwrite");
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(workspace.join("nested")).expect("nested directory should be created");
        fs::write(workspace.join("nested/source.txt"), "source").expect("source should exist");
        fs::write(workspace.join("nested/target.txt"), "target")
            .expect("target should already exist");

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        let error = rename_workspace_entry_within_workspace(
            &workspace_root,
            &workspace_root,
            "nested/source.txt",
            "target.txt",
            "codex_workspace_rename_entry",
        )
        .expect_err("destination overwrite should fail");
        assert!(error.contains("destination"));
        assert!(error.contains("already exists"));

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn workspace_entry_rename_rejects_path_traversal() {
        let temp_root = unique_temp_path("rename_traversal");
        let workspace = temp_root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace should be created");

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        let error = rename_workspace_entry_within_workspace(
            &workspace_root,
            &workspace_root,
            "../outside.txt",
            "renamed.txt",
            "codex_workspace_rename_entry",
        )
        .expect_err("path traversal should fail");
        assert!(error.contains("non-normal components are not allowed"));

        let _ = fs::remove_dir_all(temp_root);
    }
}
