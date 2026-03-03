use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunCodexCommandResponse {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) status: i32,
    pub(crate) success: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitCommitApprovedReviewRequest {
    pub(crate) paths: Vec<String>,
    pub(crate) message: String,
    pub(crate) cwd: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitCommandExecutionResult {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) status: i32,
    pub(crate) success: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitCommitApprovedReviewResponse {
    pub(crate) success: bool,
    pub(crate) add: GitCommandExecutionResult,
    pub(crate) commit: GitCommandExecutionResult,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitWorkspaceChangesRequest {
    pub(crate) cwd: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitWorkspaceChange {
    pub(crate) path: String,
    pub(crate) status: String,
    pub(crate) code: String,
    pub(crate) from_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitWorkspaceChangesResponse {
    pub(crate) cwd: String,
    pub(crate) total: usize,
    pub(crate) files: Vec<GitWorkspaceChange>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceReadFileRequest {
    pub(crate) path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceReadFileResponse {
    pub(crate) path: String,
    pub(crate) content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceWriteFileRequest {
    pub(crate) path: String,
    pub(crate) content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceWriteFileResponse {
    pub(crate) path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceCreateDirectoryRequest {
    pub(crate) path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceCreateDirectoryResponse {
    pub(crate) path: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceListDirectoryRequest {
    pub(crate) path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CodexWorkspaceListDirectoryEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceListDirectoryEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) kind: CodexWorkspaceListDirectoryEntryKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) has_children: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceListDirectoryResponse {
    pub(crate) cwd: String,
    pub(crate) path: String,
    pub(crate) entries: Vec<CodexWorkspaceListDirectoryEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceRenameEntryRequest {
    pub(crate) path: String,
    pub(crate) new_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexWorkspaceRenameEntryResponse {
    pub(crate) path: String,
    pub(crate) new_path: String,
}
