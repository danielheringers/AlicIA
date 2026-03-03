use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn is_backend_subproject_dir(cwd: &Path) -> bool {
    let Some(dir_name) = cwd.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    if !dir_name.eq_ignore_ascii_case("backend") {
        return false;
    }

    let Some(parent) = cwd.parent() else {
        return false;
    };

    parent.join("frontend").is_dir() && parent.join("backend").is_dir()
}

pub(crate) fn resolve_default_session_cwd_from_process_cwd(process_cwd: &Path) -> PathBuf {
    let normalized_cwd =
        fs::canonicalize(process_cwd).unwrap_or_else(|_| process_cwd.to_path_buf());

    if is_backend_subproject_dir(&normalized_cwd) {
        return normalized_cwd
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(normalized_cwd);
    }

    normalized_cwd
}

pub(crate) fn resolve_default_session_cwd_from_env() -> PathBuf {
    let process_cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    resolve_default_session_cwd_from_process_cwd(&process_cwd)
}

pub(crate) fn resolve_runtime_status_workspace(
    active_session_cwd: Option<&Path>,
    process_cwd: &Path,
) -> String {
    active_session_cwd
        .map(|cwd| cwd.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            resolve_default_session_cwd_from_process_cwd(process_cwd)
                .to_string_lossy()
                .to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::{resolve_default_session_cwd_from_process_cwd, resolve_runtime_status_workspace};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "alicia_backend_workspace_runtime_{label}_{}_{}",
            std::process::id(),
            now
        ))
    }

    #[test]
    fn default_cwd_uses_project_parent_when_running_inside_backend_subproject() {
        let temp_root = unique_temp_path("default_parent");
        let project_root = temp_root.join("alicia");
        let backend = project_root.join("backend");
        let frontend = project_root.join("frontend");

        fs::create_dir_all(&backend).expect("backend folder should be created");
        fs::create_dir_all(&frontend).expect("frontend folder should be created");

        let resolved = resolve_default_session_cwd_from_process_cwd(&backend);
        let expected = fs::canonicalize(&project_root).expect("project root should canonicalize");

        assert_eq!(resolved, expected);

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn default_cwd_keeps_backend_when_frontend_sibling_is_missing() {
        let temp_root = unique_temp_path("no_frontend");
        let project_root = temp_root.join("alicia");
        let backend = project_root.join("backend");

        fs::create_dir_all(&backend).expect("backend folder should be created");

        let resolved = resolve_default_session_cwd_from_process_cwd(&backend);
        let expected = fs::canonicalize(&backend).expect("backend should canonicalize");

        assert_eq!(resolved, expected);

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn runtime_status_workspace_prefers_active_session_workspace() {
        let active_workspace = PathBuf::from("C:/workspace/from-session");
        let process_cwd = PathBuf::from("C:/workspace/process");

        let workspace = resolve_runtime_status_workspace(
            Some(active_workspace.as_path()),
            process_cwd.as_path(),
        );

        assert_eq!(workspace, active_workspace.to_string_lossy().to_string());
    }

    #[test]
    fn runtime_status_workspace_falls_back_to_resolved_process_workspace() {
        let temp_root = unique_temp_path("status_fallback");
        let project_root = temp_root.join("alicia");
        let backend = project_root.join("backend");
        let frontend = project_root.join("frontend");

        fs::create_dir_all(&backend).expect("backend folder should be created");
        fs::create_dir_all(&frontend).expect("frontend folder should be created");

        let workspace = resolve_runtime_status_workspace(None, backend.as_path());
        let expected = fs::canonicalize(project_root)
            .expect("project root should canonicalize")
            .to_string_lossy()
            .to_string();

        assert_eq!(workspace, expected);

        let _ = fs::remove_dir_all(temp_root);
    }
}
