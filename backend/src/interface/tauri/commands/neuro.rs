use tauri::State;

use crate::{neuro_runtime, AppState};

#[tauri::command]
pub async fn neuro_runtime_diagnose(
    state: State<'_, AppState>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_types::RuntimeDiagnoseResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_runtime_diagnose_impl(state).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_search_objects(
    state: State<'_, AppState>,
    query: String,
    max_results: Option<u32>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<Vec<neuro_types::AdtObjectSummary>>, String> {
    Ok(
        match crate::neuro_runtime::neuro_search_objects_impl(state, query, max_results, server_id)
            .await
        {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_get_source(
    state: State<'_, AppState>,
    object_uri: String,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_types::AdtSourceResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_get_source_impl(state, object_uri, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_update_source(
    state: State<'_, AppState>,
    request: neuro_runtime::AdtUpdateSourceCommandRequest,
) -> Result<neuro_types::NeuroCommandResponse<neuro_types::AdtUpdateSourceResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_update_source_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_server_list(
    state: State<'_, AppState>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerListResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_list_impl(state).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_server_upsert(
    state: State<'_, AppState>,
    request: neuro_runtime::AdtServerUpsertRequest,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerRecord>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_upsert_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_server_remove(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerRemoveResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_remove_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_server_select(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerSelectResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_select_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_server_connect(
    state: State<'_, AppState>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtServerConnectResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_server_connect_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_list_packages(
    state: State<'_, AppState>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<Vec<neuro_runtime::AdtPackageSummary>>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_list_packages_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_list_namespaces(
    state: State<'_, AppState>,
    package_name: Option<String>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<Vec<neuro_runtime::AdtNamespaceSummary>>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_list_namespaces_impl(state, package_name, server_id)
            .await
        {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_explorer_state_get(
    state: State<'_, AppState>,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtExplorerStateResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_explorer_state_get_impl(state, server_id).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_explorer_state_patch(
    state: State<'_, AppState>,
    mut request: neuro_runtime::AdtExplorerStatePatchRequest,
    server_id: Option<String>,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtExplorerStateResponse>, String> {
    if request.server_id.is_none() {
        request.server_id = server_id;
    }

    Ok(
        match crate::neuro_runtime::neuro_adt_explorer_state_patch_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_list_objects(
    state: State<'_, AppState>,
    request: neuro_runtime::AdtListObjectsRequest,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtListObjectsResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_list_objects_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_adt_list_package_inventory(
    state: State<'_, AppState>,
    request: neuro_runtime::AdtPackageInventoryRequest,
) -> Result<neuro_types::NeuroCommandResponse<neuro_runtime::AdtPackageInventoryResponse>, String> {
    Ok(
        match crate::neuro_runtime::neuro_adt_list_package_inventory_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_ws_request(
    state: State<'_, AppState>,
    request: neuro_types::WsDomainRequest,
) -> Result<
    neuro_types::NeuroCommandResponse<neuro_types::WsMessageEnvelope<serde_json::Value>>,
    String,
> {
    Ok(
        match crate::neuro_runtime::neuro_ws_request_impl(state, request).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_list_tools(
    state: State<'_, AppState>,
) -> Result<neuro_types::NeuroCommandResponse<Vec<neuro_mcp::NeuroToolSpec>>, String> {
    Ok(
        match crate::neuro_runtime::neuro_list_tools_impl(state).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}

#[tauri::command]
pub async fn neuro_invoke_tool(
    state: State<'_, AppState>,
    tool_name: String,
    arguments: serde_json::Value,
) -> Result<neuro_types::NeuroCommandResponse<serde_json::Value>, String> {
    Ok(
        match crate::neuro_runtime::neuro_invoke_tool_impl(state, tool_name, arguments).await {
            Ok(data) => neuro_types::NeuroCommandResponse::success(data),
            Err(error) => neuro_types::NeuroCommandResponse::failure(error),
        },
    )
}
