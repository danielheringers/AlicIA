#[cfg(feature = "native-codex-runtime")]
use codex_core::config::{ConfigBuilder, ConfigOverrides};
#[cfg(feature = "native-codex-runtime")]
use codex_core::mcp::auth::{oauth_login_support, McpOAuthLoginSupport};
#[cfg(feature = "native-codex-runtime")]
use codex_core::mcp::{collect_mcp_snapshot, group_tools_by_server};
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::protocol::{McpAuthStatus, McpServerRefreshConfig};
#[cfg(feature = "native-codex-runtime")]
use codex_rmcp_client::perform_oauth_login_return_url;
use serde_json::{json, Value};
#[cfg(feature = "native-codex-runtime")]
use std::collections::BTreeSet;
use std::env;
use std::path::PathBuf;
#[cfg(feature = "native-codex-runtime")]
use std::sync::Arc;
use tauri::State;
#[cfg(feature = "native-codex-runtime")]
use toml::map::Map as TomlMap;

use crate::mcp_runtime::McpServerListResponse;
use crate::{lock_active_session, AppState};

#[cfg(feature = "native-codex-runtime")]
pub(crate) type NativeReloadContext = (
    Arc<crate::codex_native_runtime::NativeCodexRuntime>,
    PathBuf,
);

#[cfg(not(feature = "native-codex-runtime"))]
pub(crate) type NativeReloadContext = ();

#[cfg(feature = "native-codex-runtime")]
pub(crate) type NativeBinaryCwdContext = (String, PathBuf);

#[cfg(not(feature = "native-codex-runtime"))]
pub(crate) type NativeBinaryCwdContext = ();

#[cfg(feature = "native-codex-runtime")]
fn native_reload_context_from_session(
    session: &crate::ActiveSession,
) -> Option<NativeReloadContext> {
    match &session.transport {
        crate::ActiveSessionTransport::Native(native) => {
            Some((Arc::clone(&native.runtime), session.cwd.clone()))
        }
    }
}

#[cfg(not(feature = "native-codex-runtime"))]
fn native_reload_context_from_session(
    _session: &crate::ActiveSession,
) -> Option<NativeReloadContext> {
    None
}

#[cfg(feature = "native-codex-runtime")]
fn native_binary_cwd_context_from_session(
    session: &crate::ActiveSession,
) -> Option<NativeBinaryCwdContext> {
    match &session.transport {
        crate::ActiveSessionTransport::Native(_) => {
            Some((session.binary.clone(), session.cwd.clone()))
        }
    }
}

#[cfg(not(feature = "native-codex-runtime"))]
fn native_binary_cwd_context_from_session(
    _session: &crate::ActiveSession,
) -> Option<NativeBinaryCwdContext> {
    None
}

pub(crate) fn active_native_reload_context(
    state: &State<'_, AppState>,
) -> Result<Option<NativeReloadContext>, String> {
    let active = lock_active_session(state.inner())?;
    Ok(active.as_ref().and_then(native_reload_context_from_session))
}

pub(crate) fn active_native_binary_cwd_context(
    state: &State<'_, AppState>,
) -> Result<Option<NativeBinaryCwdContext>, String> {
    let active = lock_active_session(state.inner())?;
    Ok(active
        .as_ref()
        .and_then(native_binary_cwd_context_from_session))
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn native_runtime_and_cwd_for_mcp(
    state: &State<'_, AppState>,
) -> Result<NativeReloadContext, String> {
    if let Some(context) = active_native_reload_context(state)? {
        return Ok(context);
    }

    let runtime = crate::codex_native_runtime::native_runtime_get_or_init(state.inner()).await?;
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    Ok((runtime, cwd))
}

#[cfg(not(feature = "native-codex-runtime"))]
pub(crate) async fn native_runtime_and_cwd_for_mcp(
    _state: &State<'_, AppState>,
) -> Result<NativeReloadContext, String> {
    Err("native runtime feature is disabled".to_string())
}

#[cfg(feature = "native-codex-runtime")]
const ALICIA_NATIVE_INTERNAL_PROFILE: &str = "__alicia_native_internal";

#[cfg(feature = "native-codex-runtime")]
fn native_internal_profile_cli_overrides() -> Vec<(String, toml::Value)> {
    vec![(
        format!("profiles.{ALICIA_NATIVE_INTERNAL_PROFILE}"),
        toml::Value::Table(TomlMap::new()),
    )]
}

#[cfg(feature = "native-codex-runtime")]
fn native_internal_profile_harness_overrides(cwd: PathBuf) -> ConfigOverrides {
    ConfigOverrides {
        cwd: Some(cwd),
        config_profile: Some(ALICIA_NATIVE_INTERNAL_PROFILE.to_string()),
        ..Default::default()
    }
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn build_native_config_for_cwd(
    runtime: &crate::codex_native_runtime::NativeCodexRuntime,
    cwd: PathBuf,
    context: &str,
) -> Result<codex_core::config::Config, String> {
    ConfigBuilder::default()
        .codex_home(runtime.codex_home.clone())
        .fallback_cwd(Some(cwd.clone()))
        .cli_overrides(native_internal_profile_cli_overrides())
        .harness_overrides(native_internal_profile_harness_overrides(cwd))
        .build()
        .await
        .map_err(|error| format!("failed to build {context} config: {error}"))
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn mcp_entry_from_config(
    config_entries: &serde_json::Map<String, Value>,
    name: &str,
) -> (String, Option<String>, bool, Option<String>) {
    let Some(entry) = config_entries.get(name).and_then(Value::as_object) else {
        return ("stdio".to_string(), None, true, None);
    };

    let enabled = entry
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let status_reason = entry
        .get("disabled_reason")
        .or_else(|| entry.get("disabledReason"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let transport_entry = entry.get("transport").and_then(Value::as_object);
    let transport_type = transport_entry
        .and_then(|transport| transport.get("type"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("stdio");

    let transport = match transport_type {
        "streamable_http" | "streamable-http" => "streamable-http".to_string(),
        "sse" => "sse".to_string(),
        _ => "stdio".to_string(),
    };

    let url = if transport == "streamable-http" {
        transport_entry
            .and_then(|transport| transport.get("url"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    } else {
        None
    };

    (transport, url, enabled, status_reason)
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn auth_status_label(status: McpAuthStatus) -> &'static str {
    match status {
        McpAuthStatus::Unsupported => "not_logged_in",
        McpAuthStatus::NotLoggedIn => "not_logged_in",
        McpAuthStatus::BearerToken => "bearer_token",
        McpAuthStatus::OAuth => "oauth",
    }
}

#[cfg(feature = "native-codex-runtime")]
fn build_native_mcp_server_list_response(
    config_servers: &serde_json::Map<String, Value>,
    auth_statuses: &std::collections::HashMap<String, McpAuthStatus>,
    resources: &std::collections::HashMap<String, Vec<codex_protocol::mcp::Resource>>,
    resource_templates: &std::collections::HashMap<
        String,
        Vec<codex_protocol::mcp::ResourceTemplate>,
    >,
    tools_by_server: &std::collections::HashMap<
        String,
        std::collections::HashMap<String, codex_protocol::mcp::Tool>,
    >,
    fallback_elapsed_ms: u64,
) -> McpServerListResponse {
    let mut server_names = BTreeSet::<String>::new();
    server_names.extend(config_servers.keys().cloned());
    server_names.extend(auth_statuses.keys().cloned());
    server_names.extend(resources.keys().cloned());
    server_names.extend(resource_templates.keys().cloned());
    server_names.extend(tools_by_server.keys().cloned());

    let mut data = Vec::<Value>::with_capacity(server_names.len());
    for name in server_names {
        let (transport, url, enabled, status_reason) = mcp_entry_from_config(config_servers, &name);
        let auth_status = auth_statuses
            .get(&name)
            .copied()
            .unwrap_or(McpAuthStatus::NotLoggedIn);

        let mut tools = tools_by_server
            .get(&name)
            .map(|tools| tools.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        tools.sort();
        tools.dedup();

        let is_connected = enabled
            && (auth_statuses.contains_key(&name)
                || resources.contains_key(&name)
                || resource_templates.contains_key(&name)
                || tools_by_server.contains_key(&name));

        data.push(json!({
            "name": name,
            "transport": transport,
            "status": if is_connected { "connected" } else { "disconnected" },
            "statusReason": status_reason,
            "authStatus": auth_status_label(auth_status),
            "tools": tools,
            "url": url,
        }));
    }

    let result = json!({
        "data": data,
        "total": data.len(),
    });

    crate::mcp_runtime::parse_mcp_server_list_runtime_result(&result, fallback_elapsed_ms)
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn collect_native_mcp_server_list(
    runtime: Arc<crate::codex_native_runtime::NativeCodexRuntime>,
    cwd: PathBuf,
    fallback_elapsed_ms: u64,
) -> Result<McpServerListResponse, String> {
    let config = build_native_config_for_cwd(runtime.as_ref(), cwd, "MCP list").await?;
    let snapshot = collect_mcp_snapshot(&config).await;
    let tools_by_server = group_tools_by_server(&snapshot.tools);

    let config_servers_value = serde_json::to_value(config.mcp_servers.get())
        .map_err(|error| format!("failed to serialize MCP server config: {error}"))?;
    let config_servers = config_servers_value
        .as_object()
        .cloned()
        .unwrap_or_default();

    Ok(build_native_mcp_server_list_response(
        &config_servers,
        &snapshot.auth_statuses,
        &snapshot.resources,
        &snapshot.resource_templates,
        &tools_by_server,
        fallback_elapsed_ms,
    ))
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn start_mcp_oauth_login(
    config: &codex_core::config::Config,
    name: &str,
    scopes: &[String],
    timeout_secs: Option<u64>,
) -> Result<String, String> {
    let server_config = config
        .mcp_servers
        .get()
        .get(name)
        .ok_or_else(|| format!("mcp server `{name}` is not configured"))?;

    let timeout_secs = timeout_secs
        .map(|value| i64::try_from(value).map_err(|_| "timeoutSecs is too large".to_string()))
        .transpose()?;

    let oauth_config = match oauth_login_support(&server_config.transport).await {
        McpOAuthLoginSupport::Supported(config) => config,
        McpOAuthLoginSupport::Unsupported => {
            return Err(format!("mcp server `{name}` does not support oauth login"));
        }
        McpOAuthLoginSupport::Unknown(error) => {
            return Err(format!(
                "failed to determine oauth login support for `{name}`: {error}"
            ));
        }
    };

    let login = perform_oauth_login_return_url(
        name,
        oauth_config.url.as_str(),
        config.mcp_oauth_credentials_store_mode,
        oauth_config.http_headers,
        oauth_config.env_http_headers,
        scopes,
        timeout_secs,
        config.mcp_oauth_callback_port,
    )
    .await
    .map_err(|error| format!("failed to start mcp oauth login for `{name}`: {error}"))?;

    let (authorization_url, completion) = login.into_parts();
    tauri::async_runtime::spawn(async move {
        let _ = completion.await;
    });

    Ok(authorization_url)
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn refresh_mcp_servers(
    runtime: &crate::codex_native_runtime::NativeCodexRuntime,
    cwd: PathBuf,
) -> Result<(), String> {
    let config = build_native_config_for_cwd(runtime, cwd, "MCP reload").await?;

    let mcp_servers = serde_json::to_value(config.mcp_servers.get())
        .map_err(|error| format!("failed to serialize MCP servers: {error}"))?;
    let mcp_oauth_credentials_store_mode =
        serde_json::to_value(config.mcp_oauth_credentials_store_mode).map_err(|error| {
            format!("failed to serialize MCP OAuth credentials store mode: {error}")
        })?;

    let refresh_config = McpServerRefreshConfig {
        mcp_servers,
        mcp_oauth_credentials_store_mode,
    };

    runtime
        .thread_manager
        .refresh_mcp_servers(refresh_config)
        .await;
    Ok(())
}
#[cfg(all(test, feature = "native-codex-runtime"))]
mod tests {
    use super::*;
    use codex_protocol::mcp::{Resource, ResourceTemplate, Tool};
    use codex_protocol::protocol::{McpAuthStatus, McpListToolsResponseEvent};
    use serde_json::json;
    use std::collections::HashMap;

    fn fixture_tool(name: &str) -> Tool {
        Tool {
            name: name.to_string(),
            title: None,
            description: None,
            input_schema: json!({ "type": "object" }),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    fn fixture_resource(name: &str) -> Resource {
        Resource {
            annotations: None,
            description: None,
            mime_type: None,
            name: name.to_string(),
            size: None,
            title: None,
            uri: format!("file:///tmp/{name}"),
            icons: None,
            meta: None,
        }
    }

    fn fixture_resource_template(name: &str) -> ResourceTemplate {
        ResourceTemplate {
            annotations: None,
            uri_template: format!("file:///tmp/{name}/{{id}}"),
            name: name.to_string(),
            title: None,
            description: None,
            mime_type: None,
        }
    }

    #[test]
    fn mcp_entry_from_config_defaults_when_missing() {
        let entries = serde_json::Map::new();
        let (transport, url, enabled, status_reason) = mcp_entry_from_config(&entries, "missing");

        assert_eq!(transport, "stdio");
        assert_eq!(url, None);
        assert!(enabled);
        assert_eq!(status_reason, None);
    }

    #[test]
    fn mcp_entry_from_config_reads_transport_and_disabled_reason() {
        let mut entries = serde_json::Map::new();
        entries.insert(
            "alpha".to_string(),
            json!({
                "enabled": false,
                "disabledReason": "  policy lock  ",
                "transport": {
                    "type": "streamable_http",
                    "url": "  https://alpha.example/mcp  "
                }
            }),
        );

        let (transport, url, enabled, status_reason) = mcp_entry_from_config(&entries, "alpha");

        assert_eq!(transport, "streamable-http");
        assert_eq!(url, Some("https://alpha.example/mcp".to_string()));
        assert!(!enabled);
        assert_eq!(status_reason, Some("policy lock".to_string()));
    }

    #[test]
    fn auth_status_label_maps_supported_values() {
        assert_eq!(
            auth_status_label(McpAuthStatus::Unsupported),
            "not_logged_in"
        );
        assert_eq!(
            auth_status_label(McpAuthStatus::NotLoggedIn),
            "not_logged_in"
        );
        assert_eq!(
            auth_status_label(McpAuthStatus::BearerToken),
            "bearer_token"
        );
        assert_eq!(auth_status_label(McpAuthStatus::OAuth), "oauth");
    }

    #[test]
    fn collect_native_mcp_server_list_builds_response_from_local_fixture() {
        let mut config_servers = serde_json::Map::new();
        config_servers.insert(
            "alpha".to_string(),
            json!({
                "enabled": true,
                "transport": {
                    "type": "streamable_http",
                    "url": "https://alpha.example/mcp"
                }
            }),
        );
        config_servers.insert(
            "beta".to_string(),
            json!({
                "enabled": false,
                "disabled_reason": "manually disabled"
            }),
        );

        let mut auth_statuses = HashMap::new();
        auth_statuses.insert("alpha".to_string(), McpAuthStatus::OAuth);
        auth_statuses.insert("gamma".to_string(), McpAuthStatus::BearerToken);

        let mut resources = HashMap::new();
        resources.insert("beta".to_string(), vec![fixture_resource("beta-resource")]);

        let mut resource_templates = HashMap::new();
        resource_templates.insert(
            "delta".to_string(),
            vec![fixture_resource_template("delta-template")],
        );

        let mut tools_by_server = HashMap::new();
        let mut epsilon_tools = HashMap::new();
        epsilon_tools.insert("z_tool".to_string(), fixture_tool("z_tool"));
        epsilon_tools.insert("a_tool".to_string(), fixture_tool("a_tool"));
        tools_by_server.insert("epsilon".to_string(), epsilon_tools);

        let response = build_native_mcp_server_list_response(
            &config_servers,
            &auth_statuses,
            &resources,
            &resource_templates,
            &tools_by_server,
            77,
        );

        assert_eq!(response.total, 5);
        assert_eq!(response.elapsed_ms, 77);

        let by_name = response
            .data
            .iter()
            .map(|entry| (entry.name.clone(), entry))
            .collect::<HashMap<_, _>>();

        let alpha = by_name.get("alpha").expect("alpha should exist");
        assert_eq!(alpha.transport, "streamable-http");
        assert_eq!(alpha.url, Some("https://alpha.example/mcp".to_string()));
        assert_eq!(alpha.status, "connected");
        assert_eq!(alpha.auth_status, "oauth");

        let beta = by_name.get("beta").expect("beta should exist");
        assert_eq!(beta.status, "disconnected");
        assert_eq!(beta.status_reason, Some("manually disabled".to_string()));

        let gamma = by_name.get("gamma").expect("gamma should exist");
        assert_eq!(gamma.transport, "stdio");
        assert_eq!(gamma.auth_status, "bearer_token");
        assert_eq!(gamma.status, "connected");

        let delta = by_name.get("delta").expect("delta should exist");
        assert_eq!(delta.auth_status, "not_logged_in");
        assert_eq!(delta.status, "connected");

        let epsilon = by_name.get("epsilon").expect("epsilon should exist");
        assert_eq!(
            epsilon.tools,
            vec!["a_tool".to_string(), "z_tool".to_string()]
        );
        assert_eq!(epsilon.status, "connected");
    }

    #[test]
    fn collect_native_mcp_server_list_uses_snapshot_tools_grouping() {
        let mut snapshot = McpListToolsResponseEvent {
            tools: HashMap::new(),
            resources: HashMap::new(),
            resource_templates: HashMap::new(),
            auth_statuses: HashMap::new(),
        };
        snapshot
            .tools
            .insert("mcp__omega__run".to_string(), fixture_tool("run"));

        let grouped = group_tools_by_server(&snapshot.tools);
        let response = build_native_mcp_server_list_response(
            &serde_json::Map::new(),
            &snapshot.auth_statuses,
            &snapshot.resources,
            &snapshot.resource_templates,
            &grouped,
            13,
        );

        assert_eq!(response.total, 1);
        assert_eq!(response.data[0].name, "omega");
        assert_eq!(response.data[0].tools, vec!["run".to_string()]);
        assert_eq!(response.data[0].status, "connected");
    }
}
