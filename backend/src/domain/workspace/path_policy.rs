use std::path::{Component, Path, PathBuf};

pub(crate) fn normalize_workspace_relative_path(
    operation: &str,
    path: &str,
) -> Result<PathBuf, String> {
    if path.is_empty() {
        return Err(format!("{operation} rejected empty path"));
    }

    if path.contains('\0') {
        return Err(format!(
            "{operation} rejected unsafe path '{path}': contains NUL"
        ));
    }

    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Err(format!(
            "{operation} rejected unsafe path '{path}': absolute paths are not allowed"
        ));
    }

    if !candidate
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{operation} rejected unsafe path '{path}': non-normal components are not allowed"
        ));
    }

    Ok(candidate.to_path_buf())
}

pub(crate) fn normalize_workspace_list_relative_path(
    operation: &str,
    path: Option<&str>,
) -> Result<PathBuf, String> {
    match path {
        Some(value) if !value.is_empty() => normalize_workspace_relative_path(operation, value),
        _ => Ok(PathBuf::new()),
    }
}

pub(crate) fn normalize_workspace_entry_name(
    operation: &str,
    new_name: &str,
) -> Result<String, String> {
    if new_name.trim().is_empty() {
        return Err(format!("{operation} rejected empty newName"));
    }

    if new_name.contains('\0') {
        return Err(format!(
            "{operation} rejected unsafe newName '{new_name}': contains NUL"
        ));
    }

    if new_name == "." || new_name == ".." {
        return Err(format!(
            "{operation} rejected unsafe newName '{new_name}': '.' and '..' are not allowed"
        ));
    }

    if new_name.contains('/') || new_name.contains('\\') {
        return Err(format!(
            "{operation} rejected unsafe newName '{new_name}': path separators are not allowed"
        ));
    }

    let candidate = Path::new(new_name);
    if !candidate
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{operation} rejected unsafe newName '{new_name}': non-normal components are not allowed"
        ));
    }

    Ok(new_name.to_string())
}
