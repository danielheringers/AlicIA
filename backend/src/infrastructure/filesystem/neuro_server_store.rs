use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::neuro_adt::error::NeuroAdtError;
use crate::domain::neuro_adt::types::{
    AdtServerStore, DEFAULT_ADT_SERVER_STORE_RELATIVE_PATH, NEURO_ADT_SERVER_STORE_PATH_ENV,
};

pub(crate) fn resolve_codex_home() -> PathBuf {
    if let Some(path) = env::var_os("CODEX_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(path).join(".codex");
    }
    if let Some(path) = env::var_os("USERPROFILE").filter(|value| !value.is_empty()) {
        return PathBuf::from(path).join(".codex");
    }
    PathBuf::from(".codex")
}

pub(crate) fn resolve_server_store_path() -> PathBuf {
    if let Some(path) =
        env::var_os(NEURO_ADT_SERVER_STORE_PATH_ENV).filter(|value| !value.is_empty())
    {
        return PathBuf::from(path);
    }

    resolve_codex_home().join(DEFAULT_ADT_SERVER_STORE_RELATIVE_PATH)
}

pub(crate) fn load_server_store() -> Result<AdtServerStore, NeuroAdtError> {
    let path = resolve_server_store_path();
    if !path.exists() {
        return Ok(AdtServerStore::default());
    }

    let raw = fs::read_to_string(&path).map_err(|error| {
        NeuroAdtError::runtime_init_error(format!(
            "failed to read ADT server store at `{}`: {error}",
            path.display()
        ))
    })?;

    serde_json::from_str(&raw).map_err(|error| {
        NeuroAdtError::runtime_init_error(format!(
            "failed to parse ADT server store at `{}`: {error}",
            path.display()
        ))
    })
}

pub(crate) fn save_server_store(store: &AdtServerStore) -> Result<(), NeuroAdtError> {
    let path = resolve_server_store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            NeuroAdtError::runtime_init_error(format!(
                "failed to create ADT server store directory `{}`: {error}",
                parent.display()
            ))
        })?;
    }

    let payload = serde_json::to_string_pretty(store).map_err(|error| {
        NeuroAdtError::runtime_init_error(format!("failed to serialize ADT server store: {error}"))
    })?;

    write_server_store_payload(path.as_path(), payload.as_str()).map_err(|error| {
        NeuroAdtError::runtime_init_error(format!(
            "failed to write ADT server store at `{}`: {error}",
            path.display()
        ))
    })
}

#[cfg(unix)]
fn write_server_store_payload(path: &Path, payload: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(payload.as_bytes())?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn write_server_store_payload(path: &Path, payload: &str) -> std::io::Result<()> {
    // Windows ACL semantics do not map to Unix mode bits.
    fs::write(path, payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::neuro_adt::error::NeuroAdtErrorCode;
    use crate::domain::neuro_adt::types::StoredAdtServer;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        crate::domain::neuro_adt::types::shared_env_lock()
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "alicia_backend_neuro_server_store_fs_{label}_{}_{}",
            std::process::id(),
            now
        ))
    }

    fn with_env_overrides(pairs: &[(&str, Option<&str>)], test: impl FnOnce()) {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let previous: Vec<(String, Option<String>)> = pairs
            .iter()
            .map(|(key, _)| ((*key).to_string(), env::var(key).ok()))
            .collect();

        for (key, value) in pairs {
            match value {
                Some(next) => env::set_var(key, next),
                None => env::remove_var(key),
            }
        }

        test();

        for (key, value) in previous {
            match value {
                Some(previous_value) => env::set_var(&key, previous_value),
                None => env::remove_var(&key),
            }
        }
    }

    #[test]
    fn load_server_store_returns_default_when_file_is_missing() {
        let store_path = unique_temp_path("missing")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                let loaded = load_server_store().expect("missing store should return default");
                assert!(loaded.servers.is_empty());
                assert!(loaded.explorer_state_by_server.is_empty());
            },
        );
    }

    #[test]
    fn save_and_load_server_store_roundtrip() {
        let store_path = unique_temp_path("roundtrip")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                let mut store = AdtServerStore::default();
                store.servers.push(StoredAdtServer {
                    id: "srv_a".to_string(),
                    name: "Server A".to_string(),
                    base_url: "https://srv-a.local".to_string(),
                    client: Some("100".to_string()),
                    language: Some("EN".to_string()),
                    username: Some("alice".to_string()),
                    password: Some("secret".to_string()),
                    verify_tls: true,
                    active: true,
                });

                save_server_store(&store).expect("save should succeed");
                let loaded = load_server_store().expect("load should succeed");

                assert_eq!(loaded.servers.len(), 1);
                assert_eq!(loaded.servers[0].id, "srv_a");
                assert_eq!(loaded.servers[0].name, "Server A");
                assert_eq!(loaded.servers[0].client.as_deref(), Some("100"));
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn load_server_store_reports_parse_error() {
        let store_path = unique_temp_path("parse_error")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                if let Some(parent) = store_path.parent() {
                    fs::create_dir_all(parent).expect("parent directory should exist");
                }
                fs::write(&store_path, "{not-json")
                    .expect("invalid json payload should be written");

                let error = load_server_store().expect_err("invalid payload must fail");
                assert_eq!(error.code, NeuroAdtErrorCode::RuntimeInitError);
                assert!(error.message.contains("failed to parse ADT server store"));
                assert!(error
                    .message
                    .contains(store_path.to_string_lossy().as_ref()));
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[cfg(unix)]
    #[test]
    fn save_server_store_uses_restrictive_permissions() {
        let store_path = unique_temp_path("permissions")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
                Some(store_path_text.as_str()),
            )],
            || {
                save_server_store(&AdtServerStore::default()).expect("save should succeed");
                let metadata = fs::metadata(&store_path).expect("metadata should be readable");
                let mode = metadata.permissions().mode() & 0o777;
                assert_eq!(mode, 0o600);
            },
        );

        if store_path.exists() {
            let _ = fs::remove_file(&store_path);
        }
        if let Some(parent) = store_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
