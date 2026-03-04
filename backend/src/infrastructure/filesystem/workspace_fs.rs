use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use crate::{CodexWorkspaceListDirectoryEntry, CodexWorkspaceListDirectoryEntryKind};

pub(crate) fn workspace_relative_path_to_string(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();
    parts.join("/")
}

pub(crate) fn canonicalize_workspace_root(
    workspace_cwd: &Path,
    operation: &str,
) -> Result<PathBuf, String> {
    let metadata = fs::metadata(workspace_cwd).map_err(|error| {
        format!(
            "{operation} failed to inspect workspace cwd '{}': {error}",
            workspace_cwd.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "{operation} invalid workspace cwd '{}': path is not a directory",
            workspace_cwd.display()
        ));
    }

    fs::canonicalize(workspace_cwd).map_err(|error| {
        format!(
            "{operation} failed to canonicalize workspace cwd '{}': {error}",
            workspace_cwd.display()
        )
    })
}

pub(crate) fn ensure_path_within_workspace(
    workspace_root: &Path,
    candidate: &Path,
    operation: &str,
    context: &str,
) -> Result<(), String> {
    if candidate.starts_with(workspace_root) {
        return Ok(());
    }

    Err(format!(
        "{operation} rejected path traversal: {context} '{}' escapes workspace '{}'",
        candidate.display(),
        workspace_root.display()
    ))
}

pub(crate) fn ensure_secure_workspace_parent(
    workspace_root: &Path,
    relative_path: &Path,
    operation: &str,
) -> Result<PathBuf, String> {
    let mut parent_dir = workspace_root.to_path_buf();

    if let Some(relative_parent) = relative_path.parent() {
        for component in relative_parent.components() {
            let Component::Normal(name) = component else {
                return Err(format!(
                    "{operation} rejected unsafe path '{}': non-normal parent component",
                    relative_path.display()
                ));
            };

            let next = parent_dir.join(name);
            if next.exists() {
                let metadata = fs::symlink_metadata(&next).map_err(|error| {
                    format!(
                        "{operation} failed to inspect path '{}': {error}",
                        next.display()
                    )
                })?;

                if metadata.file_type().is_symlink() {
                    let canonical = fs::canonicalize(&next).map_err(|error| {
                        format!(
                            "{operation} failed to canonicalize symlink '{}': {error}",
                            next.display()
                        )
                    })?;
                    ensure_path_within_workspace(workspace_root, &canonical, operation, "parent")?;
                    if !canonical.is_dir() {
                        return Err(format!(
                            "{operation} rejected parent '{}': not a directory",
                            canonical.display()
                        ));
                    }
                    parent_dir = canonical;
                } else if metadata.is_dir() {
                    parent_dir = next;
                } else {
                    return Err(format!(
                        "{operation} rejected parent '{}': not a directory",
                        next.display()
                    ));
                }
            } else {
                fs::create_dir(&next).map_err(|error| {
                    format!(
                        "{operation} failed to create parent directory '{}': {error}",
                        next.display()
                    )
                })?;
                parent_dir = next;
            }
        }
    }

    ensure_path_within_workspace(workspace_root, &parent_dir, operation, "parent")?;
    Ok(parent_dir)
}

pub(crate) fn ensure_existing_workspace_directory_target(
    workspace_root: &Path,
    relative_path: &Path,
    target_path: &Path,
    operation: &str,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(target_path).map_err(|error| {
        format!(
            "{operation} failed to inspect existing path '{}': {error}",
            target_path.display()
        )
    })?;

    if metadata.file_type().is_symlink() {
        let canonical_target = fs::canonicalize(target_path).map_err(|error| {
            format!(
                "{operation} failed to canonicalize existing symlink '{}': {error}",
                target_path.display()
            )
        })?;
        ensure_path_within_workspace(workspace_root, &canonical_target, operation, "target")?;
        if canonical_target.is_dir() {
            return Ok(());
        }

        return Err(format!(
            "{operation} rejected path '{}': target exists and resolves to a non-directory",
            relative_path.display()
        ));
    }

    if metadata.is_dir() {
        let canonical_target = fs::canonicalize(target_path).map_err(|error| {
            format!(
                "{operation} failed to canonicalize existing directory '{}': {error}",
                target_path.display()
            )
        })?;
        ensure_path_within_workspace(workspace_root, &canonical_target, operation, "target")?;
        return Ok(());
    }

    Err(format!(
        "{operation} rejected path '{}': target exists and is not a directory",
        relative_path.display()
    ))
}

pub(crate) fn resolve_workspace_directory_for_list(
    workspace_root: &Path,
    workspace_cwd: &Path,
    relative_path: &Path,
    operation: &str,
) -> Result<PathBuf, String> {
    let target_path = if relative_path.as_os_str().is_empty() {
        workspace_cwd.to_path_buf()
    } else {
        workspace_cwd.join(relative_path)
    };

    let metadata = fs::symlink_metadata(&target_path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            format!(
                "{operation} directory '{}' does not exist in workspace '{}'",
                relative_path.display(),
                workspace_cwd.display()
            )
        } else {
            format!(
                "{operation} failed to inspect directory '{}': {error}",
                target_path.display()
            )
        }
    })?;

    let canonical_target = if metadata.file_type().is_symlink() {
        let canonical = fs::canonicalize(&target_path).map_err(|error| {
            format!(
                "{operation} failed to canonicalize symlink '{}': {error}",
                target_path.display()
            )
        })?;
        ensure_path_within_workspace(workspace_root, &canonical, operation, "target")?;
        if !canonical.is_dir() {
            return Err(format!(
                "{operation} rejected path '{}': target resolves to a non-directory",
                relative_path.display()
            ));
        }
        canonical
    } else if metadata.is_dir() {
        let canonical = fs::canonicalize(&target_path).map_err(|error| {
            format!(
                "{operation} failed to canonicalize directory '{}': {error}",
                target_path.display()
            )
        })?;
        ensure_path_within_workspace(workspace_root, &canonical, operation, "target")?;
        canonical
    } else {
        return Err(format!(
            "{operation} rejected path '{}': target is not a directory",
            relative_path.display()
        ));
    };

    Ok(canonical_target)
}

fn workspace_directory_has_children_within_workspace(
    workspace_root: &Path,
    directory_path: &Path,
    operation: &str,
) -> Result<bool, String> {
    let entries = fs::read_dir(directory_path).map_err(|error| {
        format!(
            "{operation} failed to read directory '{}': {error}",
            directory_path.display()
        )
    })?;

    let mut has_children = false;
    for child in entries {
        let child = child.map_err(|error| {
            format!(
                "{operation} failed to read directory entry in '{}': {error}",
                directory_path.display()
            )
        })?;
        has_children = true;

        let child_path = child.path();
        let metadata = fs::symlink_metadata(&child_path).map_err(|error| {
            format!(
                "{operation} failed to inspect path '{}': {error}",
                child_path.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            let canonical = fs::canonicalize(&child_path).map_err(|error| {
                format!(
                    "{operation} failed to canonicalize symlink '{}': {error}",
                    child_path.display()
                )
            })?;
            ensure_path_within_workspace(workspace_root, &canonical, operation, "entry")?;
        }
    }

    Ok(has_children)
}

fn classify_workspace_directory_entry(
    workspace_root: &Path,
    entry_path: &Path,
    entry_relative_path: &Path,
    operation: &str,
) -> Result<(CodexWorkspaceListDirectoryEntryKind, Option<bool>), String> {
    let metadata = fs::symlink_metadata(entry_path).map_err(|error| {
        format!(
            "{operation} failed to inspect path '{}': {error}",
            entry_path.display()
        )
    })?;

    if metadata.file_type().is_symlink() {
        let canonical_target = fs::canonicalize(entry_path).map_err(|error| {
            format!(
                "{operation} failed to canonicalize symlink '{}': {error}",
                entry_path.display()
            )
        })?;
        ensure_path_within_workspace(workspace_root, &canonical_target, operation, "entry")?;
        let resolved_metadata = fs::metadata(&canonical_target).map_err(|error| {
            format!(
                "{operation} failed to inspect resolved symlink '{}': {error}",
                canonical_target.display()
            )
        })?;
        if resolved_metadata.is_dir() {
            let has_children = workspace_directory_has_children_within_workspace(
                workspace_root,
                &canonical_target,
                operation,
            )?;
            return Ok((
                CodexWorkspaceListDirectoryEntryKind::Directory,
                Some(has_children),
            ));
        }
        if resolved_metadata.is_file() {
            return Ok((CodexWorkspaceListDirectoryEntryKind::File, None));
        }

        return Err(format!(
            "{operation} rejected path '{}': unsupported symlink target type",
            entry_relative_path.display()
        ));
    }

    if metadata.is_dir() {
        let has_children = workspace_directory_has_children_within_workspace(
            workspace_root,
            entry_path,
            operation,
        )?;
        return Ok((
            CodexWorkspaceListDirectoryEntryKind::Directory,
            Some(has_children),
        ));
    }

    if metadata.is_file() {
        return Ok((CodexWorkspaceListDirectoryEntryKind::File, None));
    }

    Err(format!(
        "{operation} rejected path '{}': unsupported entry type",
        entry_relative_path.display()
    ))
}

pub(crate) fn list_workspace_directory_entries_within_workspace(
    workspace_root: &Path,
    directory_path: &Path,
    requested_relative_path: &Path,
    operation: &str,
) -> Result<Vec<CodexWorkspaceListDirectoryEntry>, String> {
    let mut directories = Vec::<CodexWorkspaceListDirectoryEntry>::new();
    let mut files = Vec::<CodexWorkspaceListDirectoryEntry>::new();

    let entries = fs::read_dir(directory_path).map_err(|error| {
        format!(
            "{operation} failed to read directory '{}': {error}",
            directory_path.display()
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "{operation} failed to read directory entry in '{}': {error}",
                directory_path.display()
            )
        })?;
        let name_os = entry.file_name();
        let name = name_os.to_string_lossy().to_string();
        let entry_relative_path = if requested_relative_path.as_os_str().is_empty() {
            PathBuf::from(&name_os)
        } else {
            requested_relative_path.join(&name_os)
        };
        let entry_path = entry.path();
        let (kind, has_children) = classify_workspace_directory_entry(
            workspace_root,
            &entry_path,
            &entry_relative_path,
            operation,
        )?;
        let output = CodexWorkspaceListDirectoryEntry {
            name,
            path: workspace_relative_path_to_string(entry_relative_path.as_path()),
            kind,
            has_children,
        };

        match output.kind {
            CodexWorkspaceListDirectoryEntryKind::Directory => directories.push(output),
            CodexWorkspaceListDirectoryEntryKind::File => files.push(output),
        }
    }

    directories.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });
    files.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });
    directories.extend(files);

    Ok(directories)
}

#[cfg(test)]
mod tests {
    use super::workspace_directory_has_children_within_workspace;
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
            "alicia_backend_workspace_fs_{label}_{}_{}",
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
    fn has_children_rejects_outside_symlink_with_mixed_children_deterministically() {
        let temp_root = unique_temp_path("has_children_outside_symlink");
        let workspace = temp_root.join("workspace");
        let nested = workspace.join("nested");
        let outside = temp_root.join("outside");
        fs::create_dir_all(&nested).expect("nested directory should be created");
        fs::create_dir_all(&outside).expect("outside directory should be created");

        fs::write(nested.join("ok.txt"), "ok").expect("safe child should exist");
        let outside_target = outside.join("leak.txt");
        fs::write(&outside_target, "outside").expect("outside target should exist");

        let outside_link = nested.join("outside_link.txt");
        if let Err(error) = create_file_symlink(&outside_target, &outside_link) {
            if can_skip_symlink_test(&error) {
                let _ = fs::remove_dir_all(&temp_root);
                return;
            }
            panic!("failed to create symlink for test: {error}");
        }

        let workspace_root = fs::canonicalize(&workspace).expect("workspace should canonicalize");
        for _ in 0..3 {
            let error = workspace_directory_has_children_within_workspace(
                &workspace_root,
                &nested,
                "codex_workspace_list_directory",
            )
            .expect_err("outside symlink should always be rejected");
            assert!(error.contains("escapes workspace"));
        }

        let _ = fs::remove_dir_all(temp_root);
    }
}
