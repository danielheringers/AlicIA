use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use tauri::State;

use crate::domain::workspace::path_policy::{
    normalize_workspace_entry_name, normalize_workspace_list_relative_path,
    normalize_workspace_relative_path,
};
use crate::infrastructure::filesystem::workspace_fs::{
    canonicalize_workspace_root, ensure_existing_workspace_directory_target,
    ensure_path_within_workspace, ensure_secure_workspace_parent,
    list_workspace_directory_entries_within_workspace, resolve_workspace_directory_for_list,
    workspace_relative_path_to_string,
};
use crate::{
    lock_active_session, AppState, CodexWorkspaceCreateDirectoryRequest,
    CodexWorkspaceCreateDirectoryResponse, CodexWorkspaceListDirectoryRequest,
    CodexWorkspaceListDirectoryResponse, CodexWorkspaceReadFileRequest,
    CodexWorkspaceReadFileResponse, CodexWorkspaceRenameEntryRequest,
    CodexWorkspaceRenameEntryResponse, CodexWorkspaceWriteFileRequest,
    CodexWorkspaceWriteFileResponse,
};

const WORKSPACE_READ_OPERATION: &str = "codex_workspace_read_file";
const WORKSPACE_WRITE_OPERATION: &str = "codex_workspace_write_file";
const WORKSPACE_CREATE_DIRECTORY_OPERATION: &str = "codex_workspace_create_directory";
const WORKSPACE_LIST_DIRECTORY_OPERATION: &str = "codex_workspace_list_directory";
const WORKSPACE_RENAME_ENTRY_OPERATION: &str = "codex_workspace_rename_entry";

fn active_workspace_cwd(state: &State<'_, AppState>, operation: &str) -> Result<PathBuf, String> {
    let active = lock_active_session(state.inner())?;
    let Some(session) = active.as_ref() else {
        return Err(format!("{operation} requires an active codex session"));
    };

    Ok(session.cwd.clone())
}

pub(crate) fn list_workspace_directory_within_workspace(
    workspace_root: &Path,
    workspace_cwd: &Path,
    requested_path: Option<&str>,
    operation: &str,
) -> Result<CodexWorkspaceListDirectoryResponse, String> {
    let relative_path = normalize_workspace_list_relative_path(operation, requested_path)?;
    let canonical_target = resolve_workspace_directory_for_list(
        workspace_root,
        workspace_cwd,
        &relative_path,
        operation,
    )?;
    let entries = list_workspace_directory_entries_within_workspace(
        workspace_root,
        &canonical_target,
        &relative_path,
        operation,
    )?;

    Ok(CodexWorkspaceListDirectoryResponse {
        cwd: workspace_cwd.to_string_lossy().to_string(),
        path: workspace_relative_path_to_string(relative_path.as_path()),
        entries,
    })
}

pub(crate) fn create_workspace_directory_within_workspace(
    workspace_root: &Path,
    relative_path: &Path,
    operation: &str,
) -> Result<(), String> {
    let directory_name = relative_path.file_name().ok_or_else(|| {
        format!(
            "{operation} rejected unsafe path '{}': missing directory name",
            relative_path.display()
        )
    })?;

    let parent_dir = ensure_secure_workspace_parent(workspace_root, relative_path, operation)?;
    let target_path = parent_dir.join(directory_name);

    if target_path.exists() {
        return ensure_existing_workspace_directory_target(
            workspace_root,
            relative_path,
            &target_path,
            operation,
        );
    }

    match fs::create_dir(&target_path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return ensure_existing_workspace_directory_target(
                workspace_root,
                relative_path,
                &target_path,
                operation,
            );
        }
        Err(error) => {
            return Err(format!(
                "{operation} failed to create directory '{}': {error}",
                relative_path.display()
            ));
        }
    }

    let canonical_target = fs::canonicalize(&target_path).map_err(|error| {
        format!(
            "{operation} failed to canonicalize created directory '{}': {error}",
            target_path.display()
        )
    })?;
    ensure_path_within_workspace(workspace_root, &canonical_target, operation, "target")?;

    if !canonical_target.is_dir() {
        return Err(format!(
            "{operation} rejected path '{}': created target is not a directory",
            relative_path.display()
        ));
    }

    Ok(())
}

pub(crate) fn rename_workspace_entry_within_workspace(
    workspace_root: &Path,
    workspace_cwd: &Path,
    requested_path: &str,
    new_name: &str,
    operation: &str,
) -> Result<String, String> {
    let relative_path = normalize_workspace_relative_path(operation, requested_path)?;
    let normalized_new_name = normalize_workspace_entry_name(operation, new_name)?;
    let source_name = relative_path.file_name().ok_or_else(|| {
        format!(
            "{operation} rejected unsafe path '{}': missing source name",
            relative_path.display()
        )
    })?;

    let source_parent_path = relative_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| workspace_cwd.join(path))
        .unwrap_or_else(|| workspace_cwd.to_path_buf());

    let canonical_source_parent = fs::canonicalize(&source_parent_path).map_err(|error| {
        format!(
            "{operation} failed to canonicalize source parent '{}': {error}",
            source_parent_path.display()
        )
    })?;
    ensure_path_within_workspace(
        workspace_root,
        &canonical_source_parent,
        operation,
        "parent",
    )?;
    if !canonical_source_parent.is_dir() {
        return Err(format!(
            "{operation} rejected source parent '{}': not a directory",
            source_parent_path.display()
        ));
    }

    let source_path = canonical_source_parent.join(source_name);
    fs::symlink_metadata(&source_path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            format!(
                "{operation} source '{}' does not exist in workspace '{}'",
                relative_path.display(),
                workspace_cwd.display()
            )
        } else {
            format!(
                "{operation} failed to inspect source '{}': {error}",
                source_path.display()
            )
        }
    })?;

    let canonical_source_target = fs::canonicalize(&source_path).map_err(|error| {
        format!(
            "{operation} failed to canonicalize source '{}': {error}",
            source_path.display()
        )
    })?;
    ensure_path_within_workspace(
        workspace_root,
        &canonical_source_target,
        operation,
        "source target",
    )?;

    let destination_path = canonical_source_parent.join(&normalized_new_name);
    ensure_path_within_workspace(workspace_root, &destination_path, operation, "target path")?;
    match fs::symlink_metadata(&destination_path) {
        Ok(_) => {
            return Err(format!(
                "{operation} rejected rename '{}': destination '{}' already exists",
                relative_path.display(),
                destination_path.display()
            ));
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "{operation} failed to inspect destination '{}': {error}",
                destination_path.display()
            ));
        }
    }

    fs::rename(&source_path, &destination_path).map_err(|error| {
        format!(
            "{operation} failed to rename '{}' to '{}': {error}",
            source_path.display(),
            destination_path.display()
        )
    })?;

    let canonical_destination = fs::canonicalize(&destination_path).map_err(|error| {
        format!(
            "{operation} failed to canonicalize renamed entry '{}': {error}",
            destination_path.display()
        )
    })?;
    ensure_path_within_workspace(workspace_root, &canonical_destination, operation, "target")?;

    let renamed_relative_path = relative_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.join(&normalized_new_name))
        .unwrap_or_else(|| PathBuf::from(&normalized_new_name));

    Ok(workspace_relative_path_to_string(
        renamed_relative_path.as_path(),
    ))
}

pub(crate) fn codex_workspace_read_file_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceReadFileRequest,
) -> Result<CodexWorkspaceReadFileResponse, String> {
    let operation = WORKSPACE_READ_OPERATION;
    let requested_path = request.path;
    let relative_path = normalize_workspace_relative_path(operation, &requested_path)?;

    let workspace_cwd = active_workspace_cwd(&state, operation)?;
    let workspace_root = canonicalize_workspace_root(&workspace_cwd, operation)?;
    let target_path = workspace_cwd.join(&relative_path);

    let metadata = fs::metadata(&target_path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            format!(
                "{operation} file '{}' does not exist in workspace '{}'",
                relative_path.display(),
                workspace_cwd.display()
            )
        } else {
            format!(
                "{operation} failed to inspect file '{}': {error}",
                target_path.display()
            )
        }
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "{operation} rejected path '{}': target is not a file",
            relative_path.display()
        ));
    }

    let canonical_target = fs::canonicalize(&target_path).map_err(|error| {
        format!(
            "{operation} failed to canonicalize file '{}': {error}",
            target_path.display()
        )
    })?;
    ensure_path_within_workspace(&workspace_root, &canonical_target, operation, "target")?;

    let content = fs::read_to_string(&canonical_target).map_err(|error| {
        format!(
            "{operation} failed to read file '{}': {error}",
            relative_path.display()
        )
    })?;

    Ok(CodexWorkspaceReadFileResponse {
        path: requested_path,
        content,
    })
}

fn resolve_workspace_write_target_path(
    workspace_root: &Path,
    relative_path: &Path,
    operation: &str,
) -> Result<PathBuf, String> {
    let file_name = relative_path.file_name().ok_or_else(|| {
        format!(
            "{operation} rejected unsafe path '{}': missing file name",
            relative_path.display()
        )
    })?;
    let parent_dir = ensure_secure_workspace_parent(workspace_root, relative_path, operation)?;
    let target_path = parent_dir.join(file_name);

    match fs::symlink_metadata(&target_path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                let canonical_target = fs::canonicalize(&target_path).map_err(|error| {
                    if error.kind() == ErrorKind::NotFound {
                        format!(
                            "{operation} rejected path '{}': dangling symlink target is not allowed",
                            relative_path.display()
                        )
                    } else {
                        format!(
                            "{operation} failed to canonicalize existing symlink '{}': {error}",
                            target_path.display()
                        )
                    }
                })?;
                ensure_path_within_workspace(
                    workspace_root,
                    &canonical_target,
                    operation,
                    "target",
                )?;
                if canonical_target.is_dir() {
                    return Err(format!(
                        "{operation} rejected path '{}': target resolves to a directory",
                        relative_path.display()
                    ));
                }
            } else if metadata.is_dir() {
                return Err(format!(
                    "{operation} rejected path '{}': target is a directory",
                    relative_path.display()
                ));
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "{operation} failed to inspect existing path '{}': {error}",
                target_path.display()
            ));
        }
    }

    Ok(target_path)
}
pub(crate) fn codex_workspace_write_file_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceWriteFileRequest,
) -> Result<CodexWorkspaceWriteFileResponse, String> {
    let operation = WORKSPACE_WRITE_OPERATION;
    let requested_path = request.path;
    let relative_path = normalize_workspace_relative_path(operation, &requested_path)?;

    let workspace_cwd = active_workspace_cwd(&state, operation)?;
    let workspace_root = canonicalize_workspace_root(&workspace_cwd, operation)?;

    let target_path =
        resolve_workspace_write_target_path(&workspace_root, &relative_path, operation)?;

    fs::write(&target_path, request.content).map_err(|error| {
        format!(
            "{operation} failed to write file '{}': {error}",
            relative_path.display()
        )
    })?;

    Ok(CodexWorkspaceWriteFileResponse {
        path: requested_path,
    })
}

pub(crate) fn codex_workspace_create_directory_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceCreateDirectoryRequest,
) -> Result<CodexWorkspaceCreateDirectoryResponse, String> {
    let operation = WORKSPACE_CREATE_DIRECTORY_OPERATION;
    let requested_path = request.path;
    let relative_path = normalize_workspace_relative_path(operation, &requested_path)?;

    let workspace_cwd = active_workspace_cwd(&state, operation)?;
    let workspace_root = canonicalize_workspace_root(&workspace_cwd, operation)?;

    create_workspace_directory_within_workspace(&workspace_root, &relative_path, operation)?;

    Ok(CodexWorkspaceCreateDirectoryResponse {
        path: requested_path,
    })
}

pub(crate) fn codex_workspace_list_directory_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceListDirectoryRequest,
) -> Result<CodexWorkspaceListDirectoryResponse, String> {
    let operation = WORKSPACE_LIST_DIRECTORY_OPERATION;
    let workspace_cwd = active_workspace_cwd(&state, operation)?;
    let workspace_root = canonicalize_workspace_root(&workspace_cwd, operation)?;

    list_workspace_directory_within_workspace(
        &workspace_root,
        &workspace_cwd,
        request.path.as_deref(),
        operation,
    )
}

pub(crate) fn codex_workspace_rename_entry_impl(
    state: State<'_, AppState>,
    request: CodexWorkspaceRenameEntryRequest,
) -> Result<CodexWorkspaceRenameEntryResponse, String> {
    let operation = WORKSPACE_RENAME_ENTRY_OPERATION;
    let requested_path = request.path;
    let new_path = {
        let workspace_cwd = active_workspace_cwd(&state, operation)?;
        let workspace_root = canonicalize_workspace_root(&workspace_cwd, operation)?;
        rename_workspace_entry_within_workspace(
            &workspace_root,
            &workspace_cwd,
            &requested_path,
            &request.new_name,
            operation,
        )?
    };

    Ok(CodexWorkspaceRenameEntryResponse {
        path: requested_path,
        new_path,
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_workspace_write_target_path;
    use std::fs;
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "alicia_backend_workspace_use_cases_{label}_{}_{}",
            std::process::id(),
            now
        ))
    }

    fn create_file_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link)
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, link)
        }
    }

    fn can_skip_symlink_test(error: &std::io::Error) -> bool {
        error.kind() == ErrorKind::PermissionDenied || error.raw_os_error() == Some(1314)
    }

    #[test]
    fn workspace_write_rejects_dangling_symlink_target() {
        let temp_root = unique_temp_path("write_dangling_symlink");
        let workspace = temp_root.join("workspace");
        let outside = temp_root.join("outside");
        fs::create_dir_all(&workspace).expect("workspace should be created");
        fs::create_dir_all(&outside).expect("outside directory should be created");

        let symlink_path = workspace.join("danger.txt");
        let dangling_target = outside.join("escape.txt");
        if let Err(error) = create_file_symlink(&dangling_target, &symlink_path) {
            if can_skip_symlink_test(&error) {
                let _ = fs::remove_dir_all(&temp_root);
                return;
            }
            panic!("failed to create symlink for test: {error}");
        }

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        let error = resolve_workspace_write_target_path(
            &workspace_root,
            Path::new("danger.txt"),
            "codex_workspace_write_file",
        )
        .expect_err("dangling symlink should be rejected");
        assert!(error.contains("dangling symlink target is not allowed"));

        let _ = fs::remove_dir_all(temp_root);
    }
}
