use tauri::State;

use crate::interface::tauri::dto::{
    CodexWorkspaceCreateDirectoryRequest, CodexWorkspaceCreateDirectoryResponse,
    CodexWorkspaceListDirectoryRequest, CodexWorkspaceListDirectoryResponse,
    CodexWorkspaceReadFileRequest, CodexWorkspaceReadFileResponse,
    CodexWorkspaceRenameEntryRequest, CodexWorkspaceRenameEntryResponse,
    CodexWorkspaceWriteFileRequest, CodexWorkspaceWriteFileResponse,
    GitCommitApprovedReviewRequest, GitCommitApprovedReviewResponse, GitWorkspaceChangesRequest,
    GitWorkspaceChangesResponse, RunCodexCommandResponse,
};
use crate::{lock_active_session, AppState};

#[tauri::command]
pub fn run_codex_command(
    args: Vec<String>,
    cwd: Option<String>,
) -> Result<RunCodexCommandResponse, String> {
    crate::command_runtime::run_codex_command_impl(args, cwd)
}

#[tauri::command]
pub fn git_commit_approved_review(
    state: State<'_, AppState>,
    mut request: GitCommitApprovedReviewRequest,
) -> Result<GitCommitApprovedReviewResponse, String> {
    let active_cwd = {
        let active = lock_active_session(state.inner())?;
        active
            .as_ref()
            .map(|session| session.cwd.to_string_lossy().to_string())
    };

    let Some(cwd) = active_cwd else {
        return Err("git_commit_approved_review requires an active codex session".to_string());
    };

    request.cwd = Some(cwd);

    crate::command_runtime::git_commit_approved_review_impl(request)
}

#[tauri::command]
pub fn git_workspace_changes(
    state: State<'_, AppState>,
    request: Option<GitWorkspaceChangesRequest>,
) -> Result<GitWorkspaceChangesResponse, String> {
    {
        let active = lock_active_session(state.inner())?;
        if active.is_none() {
            return Err("git_workspace_changes requires an active codex session".to_string());
        }
    }

    crate::command_runtime::git_workspace_changes_impl(state, request.unwrap_or_default())
}

#[tauri::command]
pub fn codex_workspace_read_file(
    state: State<'_, AppState>,
    request: CodexWorkspaceReadFileRequest,
) -> Result<CodexWorkspaceReadFileResponse, String> {
    crate::command_runtime::codex_workspace_read_file_impl(state, request)
}

#[tauri::command]
pub fn codex_workspace_write_file(
    state: State<'_, AppState>,
    request: CodexWorkspaceWriteFileRequest,
) -> Result<CodexWorkspaceWriteFileResponse, String> {
    crate::command_runtime::codex_workspace_write_file_impl(state, request)
}

#[tauri::command]
pub fn codex_workspace_create_directory(
    state: State<'_, AppState>,
    request: CodexWorkspaceCreateDirectoryRequest,
) -> Result<CodexWorkspaceCreateDirectoryResponse, String> {
    crate::command_runtime::codex_workspace_create_directory_impl(state, request)
}

#[tauri::command]
pub fn codex_workspace_list_directory(
    state: State<'_, AppState>,
    request: Option<CodexWorkspaceListDirectoryRequest>,
) -> Result<CodexWorkspaceListDirectoryResponse, String> {
    crate::command_runtime::codex_workspace_list_directory_impl(state, request.unwrap_or_default())
}

#[tauri::command]
pub fn codex_workspace_rename_entry(
    state: State<'_, AppState>,
    request: CodexWorkspaceRenameEntryRequest,
) -> Result<CodexWorkspaceRenameEntryResponse, String> {
    crate::command_runtime::codex_workspace_rename_entry_impl(state, request)
}
