use crate::application::neuro_adt::contracts::{
    AdtServerConnectResponse, AdtServerListResponse, AdtServerRecord, AdtServerRemoveResponse,
    AdtServerSelectResponse, AdtServerUpsertRequest,
};
use crate::application::neuro_adt::ports::NeuroAdtPort;
use crate::domain::neuro_adt::error::NeuroAdtError;
use crate::domain::neuro_adt::server_store::{
    normalize_required_field, remove_server, select_server, selected_server_id, upsert_server,
};
use crate::domain::neuro_adt::types::{AdtServerStore, AdtServerUpsertInput, StoredAdtServer};
use crate::AppState;

fn to_server_record(server: &StoredAdtServer) -> AdtServerRecord {
    AdtServerRecord {
        id: server.id.clone(),
        name: server.name.clone(),
        base_url: server.base_url.clone(),
        client: server.client.clone(),
        language: server.language.clone(),
        username: server.username.clone(),
        verify_tls: server.verify_tls,
        active: server.active,
    }
}

fn server_list_response(store: &AdtServerStore) -> AdtServerListResponse {
    AdtServerListResponse {
        servers: store.servers.iter().map(to_server_record).collect(),
        selected_server_id: selected_server_id(store),
    }
}

pub(crate) fn neuro_adt_server_list(
    port: &impl NeuroAdtPort,
) -> Result<AdtServerListResponse, NeuroAdtError> {
    let store = port.load_server_store()?;
    Ok(server_list_response(&store))
}

pub(crate) async fn neuro_adt_server_upsert(
    state: &AppState,
    request: AdtServerUpsertRequest,
    port: &impl NeuroAdtPort,
) -> Result<AdtServerRecord, NeuroAdtError> {
    let mut store = port.load_server_store()?;
    let stored = upsert_server(
        &mut store,
        AdtServerUpsertInput {
            id: request.id,
            name: request.name,
            base_url: request.base_url,
            client: request.client,
            language: request.language,
            username: request.username,
            password: request.password,
            verify_tls: request.verify_tls,
            active: request.active,
        },
    )?;
    port.save_server_store(&store)?;
    port.clear_runtime_cache(state).await;
    Ok(to_server_record(&stored))
}

pub(crate) async fn neuro_adt_server_remove(
    state: &AppState,
    server_id: String,
    port: &impl NeuroAdtPort,
) -> Result<AdtServerRemoveResponse, NeuroAdtError> {
    let mut store = port.load_server_store()?;
    let normalized_server_id = normalize_required_field("server_id", server_id.as_str())?;
    let removed = remove_server(&mut store, normalized_server_id.as_str());

    if removed {
        port.save_server_store(&store)?;
        port.clear_runtime_cache(state).await;
    }

    Ok(AdtServerRemoveResponse {
        removed,
        selected_server_id: selected_server_id(&store),
    })
}

pub(crate) async fn neuro_adt_server_select(
    state: &AppState,
    server_id: String,
    port: &impl NeuroAdtPort,
) -> Result<AdtServerSelectResponse, NeuroAdtError> {
    let mut store = port.load_server_store()?;
    let normalized_server_id = normalize_required_field("server_id", server_id.as_str())?;
    select_server(&mut store, normalized_server_id.as_str())?;
    port.save_server_store(&store)?;
    port.clear_runtime_cache(state).await;

    Ok(AdtServerSelectResponse {
        selected_server_id: normalized_server_id,
    })
}

pub(crate) async fn neuro_adt_server_connect(
    state: &AppState,
    server_id: Option<String>,
    port: &impl NeuroAdtPort,
) -> Result<AdtServerConnectResponse, NeuroAdtError> {
    let selected_server_id = port.normalize_optional_server_id(server_id);
    let connection = port
        .connect_server(state, selected_server_id.as_deref())
        .await?;

    let response_server_id = connection
        .selected_server_id
        .or(selected_server_id)
        .unwrap_or_else(|| port.env_default_server_id().to_string());

    Ok(AdtServerConnectResponse {
        server_id: response_server_id,
        connected: connection.connected,
        message: connection.message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::neuro_adt::ports::{AdtServerConnectivity, NeuroAdtFuture};
    use crate::domain::neuro_adt::types::{
        AdtServerStore, ENV_DEFAULT_SERVER_ID, NEURO_ADT_SERVER_STORE_PATH_ENV,
    };
    use crate::infrastructure::filesystem::neuro_server_store as neuro_server_store_fs;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestPort {
        clear_calls: Arc<AtomicUsize>,
    }

    impl TestPort {
        fn new() -> Self {
            Self {
                clear_calls: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn clear_calls(&self) -> usize {
            self.clear_calls.load(Ordering::SeqCst)
        }
    }

    impl NeuroAdtPort for TestPort {
        fn load_server_store(&self) -> Result<AdtServerStore, NeuroAdtError> {
            neuro_server_store_fs::load_server_store()
        }

        fn save_server_store(&self, store: &AdtServerStore) -> Result<(), NeuroAdtError> {
            neuro_server_store_fs::save_server_store(store)
        }

        fn normalize_optional_server_id(&self, server_id: Option<String>) -> Option<String> {
            server_id.and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
        }

        fn env_default_server_id(&self) -> &'static str {
            ENV_DEFAULT_SERVER_ID
        }

        fn clear_runtime_cache<'a>(&'a self, _state: &'a AppState) -> NeuroAdtFuture<'a, ()> {
            self.clear_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {})
        }

        fn connect_server<'a>(
            &'a self,
            _state: &'a AppState,
            server_id: Option<&'a str>,
        ) -> NeuroAdtFuture<'a, Result<AdtServerConnectivity, NeuroAdtError>> {
            Box::pin(async move {
                let selected_server_id = server_id
                    .and_then(|entry| {
                        let trimmed = entry.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    })
                    .or_else(|| {
                        self.load_server_store()
                            .ok()
                            .and_then(|store| selected_server_id(&store))
                    });

                Ok(AdtServerConnectivity {
                    selected_server_id,
                    connected: false,
                    message: Some("connection failed".to_string()),
                })
            })
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        crate::domain::neuro_adt::types::shared_env_lock()
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
                NEURO_ADT_SERVER_STORE_PATH_ENV,
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
                    let port = TestPort::new();

                    neuro_adt_server_upsert(
                        state.as_ref(),
                        AdtServerUpsertRequest {
                            active: Some(false),
                            ..sample_request("srv_a")
                        },
                        &port,
                    )
                    .await
                    .expect("first upsert should succeed");

                    neuro_adt_server_upsert(
                        state.as_ref(),
                        AdtServerUpsertRequest {
                            active: Some(true),
                            ..sample_request("srv_b")
                        },
                        &port,
                    )
                    .await
                    .expect("second upsert should succeed");

                    let list = neuro_adt_server_list(&port).expect("list should succeed");
                    assert_eq!(list.servers.len(), 2);
                    assert_eq!(list.selected_server_id.as_deref(), Some("srv_b"));

                    let selected =
                        neuro_adt_server_select(state.as_ref(), "srv_a".to_string(), &port)
                            .await
                            .expect("select should succeed");
                    assert_eq!(selected.selected_server_id, "srv_a");

                    let removed =
                        neuro_adt_server_remove(state.as_ref(), "srv_a".to_string(), &port)
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
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
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
                    let port = TestPort::new();

                    neuro_adt_server_upsert(
                        state.as_ref(),
                        AdtServerUpsertRequest {
                            active: Some(true),
                            ..sample_request("srv_mut")
                        },
                        &port,
                    )
                    .await
                    .expect("upsert should succeed");

                    neuro_adt_server_select(state.as_ref(), "srv_mut".to_string(), &port)
                        .await
                        .expect("select should succeed");

                    let removed =
                        neuro_adt_server_remove(state.as_ref(), "srv_mut".to_string(), &port)
                            .await
                            .expect("remove should succeed");
                    assert!(removed.removed, "remove should report mutation");
                    assert_eq!(port.clear_calls(), 3);
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
            &[(
                NEURO_ADT_SERVER_STORE_PATH_ENV,
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
                    let port = TestPort::new();

                    neuro_adt_server_upsert(
                        state.as_ref(),
                        AdtServerUpsertRequest {
                            active: Some(true),
                            ..sample_request("srv_connect")
                        },
                        &port,
                    )
                    .await
                    .expect("upsert should succeed");

                    let response = neuro_adt_server_connect(state.as_ref(), None, &port)
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

    #[test]
    fn connect_use_case_falls_back_to_env_default_server_id() {
        let store_path = unique_temp_path("connect_default")
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
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .expect("runtime should build");
                runtime.block_on(async {
                    let state = Arc::new(AppState::default());
                    let port = TestPort::new();

                    let response = neuro_adt_server_connect(state.as_ref(), None, &port)
                        .await
                        .expect("connect should not fail");

                    assert_eq!(response.server_id, ENV_DEFAULT_SERVER_ID);
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
