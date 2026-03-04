use std::collections::HashSet;

use neuro_types::{NeuroRuntimeError, NeuroRuntimeErrorCode};

use crate::neuro_runtime::{
    runtime_error, AdtServerListResponse, AdtServerRecord, AdtServerStore, AdtServerUpsertRequest,
    StoredAdtServer,
};

fn normalize_optional_field(value: Option<String>) -> Option<String> {
    value.and_then(|entry| {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(crate) fn normalize_required_field(
    field: &str,
    value: &str,
) -> Result<String, NeuroRuntimeError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            format!("{field} must not be empty"),
            None,
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_stored_server(server: StoredAdtServer) -> Option<StoredAdtServer> {
    let id = server.id.trim().to_string();
    let name = server.name.trim().to_string();
    let base_url = server.base_url.trim().to_string();
    if id.is_empty() || name.is_empty() || base_url.is_empty() {
        return None;
    }

    Some(StoredAdtServer {
        id,
        name,
        base_url,
        client: normalize_optional_field(server.client),
        language: normalize_optional_field(server.language),
        username: normalize_optional_field(server.username),
        password: normalize_optional_field(server.password),
        verify_tls: server.verify_tls,
        active: server.active,
    })
}

pub(crate) fn normalize_server_registry(store: &mut AdtServerStore) {
    let mut seen_ids = HashSet::<String>::new();
    let mut normalized = Vec::<StoredAdtServer>::new();
    let mut active_id: Option<String> = None;

    for server in std::mem::take(&mut store.servers) {
        let mut server = match normalize_stored_server(server) {
            Some(entry) => entry,
            None => continue,
        };

        if !seen_ids.insert(server.id.clone()) {
            continue;
        }

        if server.active {
            if active_id.is_none() {
                active_id = Some(server.id.clone());
            } else {
                server.active = false;
            }
        }

        normalized.push(server);
    }

    if let Some(active_id) = active_id {
        for server in &mut normalized {
            server.active = server.id == active_id;
        }
    }

    store.servers = normalized;
}

pub(crate) fn selected_server_id(store: &AdtServerStore) -> Option<String> {
    store
        .servers
        .iter()
        .find(|server| server.active)
        .map(|server| server.id.clone())
}

pub(crate) fn to_server_record(server: &StoredAdtServer) -> AdtServerRecord {
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

pub(crate) fn server_list_response(store: &AdtServerStore) -> AdtServerListResponse {
    AdtServerListResponse {
        servers: store.servers.iter().map(to_server_record).collect(),
        selected_server_id: selected_server_id(store),
    }
}

pub(crate) fn upsert_server(
    store: &mut AdtServerStore,
    request: AdtServerUpsertRequest,
) -> Result<StoredAdtServer, NeuroRuntimeError> {
    let id = normalize_required_field("id", request.id.as_str())?;
    let name = normalize_required_field("name", request.name.as_str())?;
    let base_url = normalize_required_field("base_url", request.base_url.as_str())?;

    let existing = store.servers.iter().find(|server| server.id == id).cloned();

    let new_password = match request.password {
        Some(password) => normalize_optional_field(Some(password)),
        None => existing.as_ref().and_then(|server| server.password.clone()),
    };

    let server = StoredAdtServer {
        id: id.clone(),
        name,
        base_url,
        client: normalize_optional_field(request.client),
        language: normalize_optional_field(request.language),
        username: normalize_optional_field(request.username),
        password: new_password,
        verify_tls: request.verify_tls.unwrap_or(
            existing
                .as_ref()
                .map(|server| server.verify_tls)
                .unwrap_or(true),
        ),
        active: request.active.unwrap_or(
            existing
                .as_ref()
                .map(|server| server.active)
                .unwrap_or(false),
        ),
    };

    if let Some(index) = store.servers.iter().position(|entry| entry.id == id) {
        store.servers[index] = server.clone();
    } else {
        store.servers.push(server.clone());
    }

    if request.active == Some(true) {
        for entry in &mut store.servers {
            entry.active = entry.id == id;
        }
    } else if request.active == Some(false) {
        for entry in &mut store.servers {
            if entry.id == id {
                entry.active = false;
            }
        }
    }

    normalize_server_registry(store);

    store
        .servers
        .iter()
        .find(|entry| entry.id == id)
        .cloned()
        .ok_or_else(|| {
            runtime_error(
                NeuroRuntimeErrorCode::RuntimeInitError,
                format!("failed to persist ADT server `{id}`"),
                None,
            )
        })
}

pub(crate) fn select_server(
    store: &mut AdtServerStore,
    server_id: &str,
) -> Result<(), NeuroRuntimeError> {
    let found = store.servers.iter().any(|server| server.id == server_id);
    if !found {
        return Err(runtime_error(
            NeuroRuntimeErrorCode::InvalidArgument,
            format!("ADT server `{server_id}` is not configured"),
            None,
        ));
    }

    for server in &mut store.servers {
        server.active = server.id == server_id;
    }

    Ok(())
}

pub(crate) fn remove_server(store: &mut AdtServerStore, server_id: &str) -> bool {
    let original_len = store.servers.len();
    store.servers.retain(|server| server.id != server_id);
    let removed = original_len != store.servers.len();
    if removed {
        store.explorer_state_by_server.remove(server_id);
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::neuro_runtime::StoredAdtExplorerState;

    fn sample_request(id: &str) -> AdtServerUpsertRequest {
        AdtServerUpsertRequest {
            id: id.to_string(),
            name: format!("Server {id}"),
            base_url: format!("https://{id}.local"),
            client: None,
            language: None,
            username: None,
            password: None,
            verify_tls: None,
            active: None,
        }
    }

    #[test]
    fn upsert_preserves_password_when_request_omits_it() {
        let mut store = AdtServerStore::default();
        upsert_server(
            &mut store,
            AdtServerUpsertRequest {
                password: Some("secret".to_string()),
                active: Some(true),
                ..sample_request("srv_a")
            },
        )
        .expect("initial upsert should work");

        let updated = upsert_server(
            &mut store,
            AdtServerUpsertRequest {
                name: "  Updated  ".to_string(),
                base_url: "  https://updated.local  ".to_string(),
                password: None,
                ..sample_request("srv_a")
            },
        )
        .expect("update should work");

        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.base_url, "https://updated.local");
        assert_eq!(updated.password.as_deref(), Some("secret"));
        assert!(updated.active);
    }

    #[test]
    fn normalize_server_registry_enforces_single_active_and_dedupes_ids() {
        let mut store = AdtServerStore {
            servers: vec![
                StoredAdtServer {
                    id: "srv_a".to_string(),
                    name: "A".to_string(),
                    base_url: "https://a.local".to_string(),
                    client: None,
                    language: None,
                    username: None,
                    password: None,
                    verify_tls: true,
                    active: true,
                },
                StoredAdtServer {
                    id: "srv_a".to_string(),
                    name: "A duplicate".to_string(),
                    base_url: "https://dup.local".to_string(),
                    client: None,
                    language: None,
                    username: None,
                    password: None,
                    verify_tls: true,
                    active: false,
                },
                StoredAdtServer {
                    id: "srv_b".to_string(),
                    name: "B".to_string(),
                    base_url: "https://b.local".to_string(),
                    client: None,
                    language: None,
                    username: None,
                    password: None,
                    verify_tls: true,
                    active: true,
                },
                StoredAdtServer {
                    id: "   ".to_string(),
                    name: "invalid".to_string(),
                    base_url: "https://invalid.local".to_string(),
                    client: None,
                    language: None,
                    username: None,
                    password: None,
                    verify_tls: true,
                    active: false,
                },
            ],
            ..AdtServerStore::default()
        };

        normalize_server_registry(&mut store);

        assert_eq!(store.servers.len(), 2);
        assert_eq!(store.servers[0].id, "srv_a");
        assert!(store.servers[0].active);
        assert_eq!(store.servers[1].id, "srv_b");
        assert!(!store.servers[1].active);
    }

    #[test]
    fn select_server_rejects_unknown_server() {
        let mut store = AdtServerStore::default();
        upsert_server(&mut store, sample_request("srv_a")).expect("upsert should work");

        let error = select_server(&mut store, "srv_missing").expect_err("must fail");
        assert_eq!(error.code, NeuroRuntimeErrorCode::InvalidArgument);
        assert!(error.message.contains("srv_missing"));
    }

    #[test]
    fn remove_server_prunes_explorer_state_for_removed_server() {
        let mut store = AdtServerStore::default();
        upsert_server(&mut store, sample_request("srv_a")).expect("upsert should work");
        upsert_server(&mut store, sample_request("srv_b")).expect("upsert should work");

        store
            .explorer_state_by_server
            .insert("srv_a".to_string(), StoredAdtExplorerState::default());
        store
            .explorer_state_by_server
            .insert("srv_b".to_string(), StoredAdtExplorerState::default());

        let removed = remove_server(&mut store, "srv_a");

        assert!(removed);
        assert!(store.servers.iter().all(|entry| entry.id != "srv_a"));
        assert!(!store.explorer_state_by_server.contains_key("srv_a"));
        assert!(store.explorer_state_by_server.contains_key("srv_b"));
    }
}
