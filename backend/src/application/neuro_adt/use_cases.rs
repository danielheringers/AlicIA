use crate::domain::neuro_adt::server_store::{
    normalize_required_field, remove_server, select_server, selected_server_id,
    server_list_response, to_server_record, upsert_server,
};
use crate::neuro_runtime::{
    clear_runtime_cache, get_or_init, load_server_store, normalize_optional_server_id,
    save_server_store, AdtServerConnectResponse, AdtServerListResponse, AdtServerRecord,
    AdtServerRemoveResponse, AdtServerSelectResponse, AdtServerUpsertRequest,
    ENV_DEFAULT_SERVER_ID,
};
use crate::AppState;

pub(crate) fn neuro_adt_server_list(
) -> Result<AdtServerListResponse, neuro_types::NeuroRuntimeError> {
    let store = load_server_store()?;
    Ok(server_list_response(&store))
}

pub(crate) async fn neuro_adt_server_upsert(
    state: &AppState,
    request: AdtServerUpsertRequest,
) -> Result<AdtServerRecord, neuro_types::NeuroRuntimeError> {
    let mut store = load_server_store()?;
    let stored = upsert_server(&mut store, request)?;
    save_server_store(&store)?;
    clear_runtime_cache(state).await;
    Ok(to_server_record(&stored))
}

pub(crate) async fn neuro_adt_server_remove(
    state: &AppState,
    server_id: String,
) -> Result<AdtServerRemoveResponse, neuro_types::NeuroRuntimeError> {
    let mut store = load_server_store()?;
    let normalized_server_id = normalize_required_field("server_id", server_id.as_str())?;
    let removed = remove_server(&mut store, normalized_server_id.as_str());

    if removed {
        save_server_store(&store)?;
        clear_runtime_cache(state).await;
    }

    Ok(AdtServerRemoveResponse {
        removed,
        selected_server_id: selected_server_id(&store),
    })
}

pub(crate) async fn neuro_adt_server_select(
    state: &AppState,
    server_id: String,
) -> Result<AdtServerSelectResponse, neuro_types::NeuroRuntimeError> {
    let mut store = load_server_store()?;
    let normalized_server_id = normalize_required_field("server_id", server_id.as_str())?;
    select_server(&mut store, normalized_server_id.as_str())?;
    save_server_store(&store)?;
    clear_runtime_cache(state).await;

    Ok(AdtServerSelectResponse {
        selected_server_id: normalized_server_id,
    })
}

pub(crate) async fn neuro_adt_server_connect(
    state: &AppState,
    server_id: Option<String>,
) -> Result<AdtServerConnectResponse, neuro_types::NeuroRuntimeError> {
    let selected_server_id = normalize_optional_server_id(server_id);
    let runtime = get_or_init(state, selected_server_id.as_deref()).await?;
    let (connected, message) = runtime.adt_http_connectivity().await;

    let response_server_id = runtime
        .selected_server_id()
        .or(selected_server_id)
        .unwrap_or_else(|| ENV_DEFAULT_SERVER_ID.to_string());

    Ok(AdtServerConnectResponse {
        server_id: response_server_id,
        connected,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        crate::neuro_runtime::shared_env_lock()
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "alicia_backend_neuro_adt_use_cases_{label}_{}_{}",
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

    fn sample_request(id: &str) -> AdtServerUpsertRequest {
        AdtServerUpsertRequest {
            id: id.to_string(),
            name: format!("Server {id}"),
            base_url: format!("http://127.0.0.1:9/{id}"),
            client: Some("100".to_string()),
            language: Some("EN".to_string()),
            username: Some("tester".to_string()),
            password: Some("secret".to_string()),
            verify_tls: Some(true),
            active: None,
        }
    }

    #[test]
    fn server_use_cases_preserve_command_contract_for_list_select_remove() {
        let store_path = unique_temp_path("contract")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[(
                "NEURO_ADT_SERVER_STORE_PATH",
                Some(store_path_text.as_str()),
            )],
            || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .expect("runtime should build");
                runtime.block_on(async {
                    let state = Arc::new(AppState::default());

                    neuro_adt_server_upsert(
                        state.as_ref(),
                        AdtServerUpsertRequest {
                            active: Some(false),
                            ..sample_request("srv_a")
                        },
                    )
                    .await
                    .expect("first upsert should succeed");

                    neuro_adt_server_upsert(
                        state.as_ref(),
                        AdtServerUpsertRequest {
                            active: Some(true),
                            ..sample_request("srv_b")
                        },
                    )
                    .await
                    .expect("second upsert should succeed");

                    let list = neuro_adt_server_list().expect("list should succeed");
                    assert_eq!(list.servers.len(), 2);
                    assert_eq!(list.selected_server_id.as_deref(), Some("srv_b"));

                    let selected = neuro_adt_server_select(state.as_ref(), "srv_a".to_string())
                        .await
                        .expect("select should succeed");
                    assert_eq!(selected.selected_server_id, "srv_a");

                    let removed = neuro_adt_server_remove(state.as_ref(), "srv_a".to_string())
                        .await
                        .expect("remove should succeed");
                    assert!(removed.removed);
                    assert_eq!(removed.selected_server_id, None);
                });
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
    fn mutating_use_cases_invalidate_runtime_cache_after_change() {
        let store_path = unique_temp_path("cache_invalidation")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[
                (
                    "NEURO_ADT_SERVER_STORE_PATH",
                    Some(store_path_text.as_str()),
                ),
                ("NEURO_SAP_URL", Some("http://127.0.0.1:9")),
                ("NEURO_SAP_USER", Some("tester")),
                ("NEURO_SAP_PASSWORD", Some("secret")),
                ("NEURO_SAP_TIMEOUT_SECS", Some("1")),
            ],
            || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .expect("runtime should build");
                runtime.block_on(async {
                    let state = Arc::new(AppState::default());

                    let _ = get_or_init(state.as_ref(), None)
                        .await
                        .expect("runtime init should succeed");
                    {
                        let cache = state.neuro_runtime_cache.lock().await;
                        assert!(
                            !cache.is_empty(),
                            "cache should be populated before mutation"
                        );
                    }

                    neuro_adt_server_upsert(
                        state.as_ref(),
                        AdtServerUpsertRequest {
                            active: Some(true),
                            ..sample_request("srv_mut")
                        },
                    )
                    .await
                    .expect("upsert should succeed");
                    {
                        let cache = state.neuro_runtime_cache.lock().await;
                        assert!(cache.is_empty(), "upsert should clear runtime cache");
                    }

                    let _ = get_or_init(state.as_ref(), None)
                        .await
                        .expect("runtime init should succeed after upsert");
                    {
                        let cache = state.neuro_runtime_cache.lock().await;
                        assert!(!cache.is_empty(), "cache should be repopulated");
                    }

                    neuro_adt_server_select(state.as_ref(), "srv_mut".to_string())
                        .await
                        .expect("select should succeed");
                    {
                        let cache = state.neuro_runtime_cache.lock().await;
                        assert!(cache.is_empty(), "select should clear runtime cache");
                    }

                    let _ = get_or_init(state.as_ref(), None)
                        .await
                        .expect("runtime init should succeed after select");
                    {
                        let cache = state.neuro_runtime_cache.lock().await;
                        assert!(!cache.is_empty(), "cache should be repopulated");
                    }

                    let removed = neuro_adt_server_remove(state.as_ref(), "srv_mut".to_string())
                        .await
                        .expect("remove should succeed");
                    assert!(removed.removed, "remove should report mutation");
                    {
                        let cache = state.neuro_runtime_cache.lock().await;
                        assert!(cache.is_empty(), "remove should clear runtime cache");
                    }
                });
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
    fn connect_use_case_resolves_server_id_from_runtime_selection() {
        let store_path = unique_temp_path("connect")
            .join("alicia")
            .join("neuro")
            .join("adt_servers.json");
        let store_path_text = store_path.to_string_lossy().to_string();

        with_env_overrides(
            &[
                (
                    "NEURO_ADT_SERVER_STORE_PATH",
                    Some(store_path_text.as_str()),
                ),
                ("NEURO_SAP_URL", Some("http://127.0.0.1:9")),
                ("NEURO_SAP_USER", Some("tester")),
                ("NEURO_SAP_PASSWORD", Some("secret")),
                ("NEURO_SAP_TIMEOUT_SECS", Some("1")),
            ],
            || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .expect("runtime should build");
                runtime.block_on(async {
                    let state = Arc::new(AppState::default());

                    neuro_adt_server_upsert(
                        state.as_ref(),
                        AdtServerUpsertRequest {
                            active: Some(true),
                            ..sample_request("srv_connect")
                        },
                    )
                    .await
                    .expect("upsert should succeed");

                    let response = neuro_adt_server_connect(state.as_ref(), None)
                        .await
                        .expect("connect should not fail");

                    assert_eq!(response.server_id, "srv_connect");
                });
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
