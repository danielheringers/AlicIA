use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStartupWarmupResponse {
    pub ready_servers: Vec<String>,
    pub total_ready: usize,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerListEntry {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub status: String,
    pub status_reason: Option<String>,
    pub auth_status: String,
    pub tools: Vec<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerListResponse {
    pub data: Vec<McpServerListEntry>,
    pub total: usize,
    pub elapsed_ms: u64,
}

#[allow(dead_code)]
pub fn parse_mcp_startup_warmup_runtime_result(
    result: &Value,
    fallback_elapsed_ms: u64,
) -> McpStartupWarmupResponse {
    let mut ready_servers = result
        .get("readyServers")
        .or_else(|| result.get("ready_servers"))
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ready_servers.sort();
    ready_servers.dedup();

    let total_ready = result
        .get("totalReady")
        .or_else(|| result.get("total_ready"))
        .and_then(|value| {
            value
                .as_u64()
                .and_then(|count| usize::try_from(count).ok())
                .or_else(|| value.as_i64().and_then(|count| usize::try_from(count).ok()))
                .or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw| raw.trim().parse::<usize>().ok())
                })
        })
        .unwrap_or(ready_servers.len());

    let elapsed_ms = result
        .get("elapsedMs")
        .or_else(|| result.get("elapsed_ms"))
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|millis| u64::try_from(millis).ok()))
                .or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw| raw.trim().parse::<u64>().ok())
                })
        })
        .unwrap_or(fallback_elapsed_ms);

    McpStartupWarmupResponse {
        ready_servers,
        total_ready,
        elapsed_ms,
    }
}

fn normalize_mcp_server_id_base(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            normalized.push(ch);
        } else {
            normalized.push('-');
        }
    }

    let mut compact = String::with_capacity(normalized.len());
    let mut previous_dash = false;
    for ch in normalized.chars() {
        if ch == '-' {
            if !previous_dash {
                compact.push(ch);
                previous_dash = true;
            }
        } else {
            compact.push(ch);
            previous_dash = false;
        }
    }

    let compact = compact.trim_matches('-');
    if compact.is_empty() {
        "server".to_string()
    } else {
        compact.to_string()
    }
}

fn make_unique_mcp_server_id(base_id: &str, seen_ids: &mut HashMap<String, usize>) -> String {
    let count = seen_ids.get(base_id).copied().unwrap_or(0);
    seen_ids.insert(base_id.to_string(), count + 1);
    if count == 0 {
        base_id.to_string()
    } else {
        format!("{base_id}-{}", count + 1)
    }
}

#[allow(dead_code)]
pub fn parse_mcp_server_list_runtime_result(
    result: &Value,
    fallback_elapsed_ms: u64,
) -> McpServerListResponse {
    let mut seen_ids = HashMap::<String, usize>::new();
    let mut data = result
        .get("data")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_object)
                .filter_map(|entry| {
                    let name = entry
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(str::to_string)?;

                    let base_id = entry
                        .get("id")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|id| !id.is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| normalize_mcp_server_id_base(&name));
                    let id = make_unique_mcp_server_id(&base_id, &mut seen_ids);

                    let transport = entry
                        .get("transport")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| match value {
                            "stdio" | "sse" | "streamable-http" => value.to_string(),
                            _ => "stdio".to_string(),
                        })
                        .unwrap_or_else(|| "stdio".to_string());

                    let status = entry
                        .get("status")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| match value {
                            "connected" | "disconnected" | "error" | "connecting" => {
                                value.to_string()
                            }
                            _ => "connected".to_string(),
                        })
                        .unwrap_or_else(|| "connected".to_string());

                    let auth_status = entry
                        .get("authStatus")
                        .or_else(|| entry.get("auth_status"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| match value {
                            "not_logged_in" | "bearer_token" | "oauth" => value.to_string(),
                            "unsupported" => "not_logged_in".to_string(),
                            "notLoggedIn" => "not_logged_in".to_string(),
                            "bearerToken" => "bearer_token".to_string(),
                            "oAuth" => "oauth".to_string(),
                            _ => "not_logged_in".to_string(),
                        })
                        .unwrap_or_else(|| "not_logged_in".to_string());

                    let status_reason = entry
                        .get("statusReason")
                        .or_else(|| entry.get("status_reason"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string);

                    let mut tools = match entry.get("tools") {
                        Some(Value::Array(tools)) => tools
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::trim)
                            .filter(|tool| !tool.is_empty())
                            .map(str::to_string)
                            .collect::<Vec<_>>(),
                        Some(Value::Object(tools)) => tools
                            .iter()
                            .filter_map(|(key, tool)| {
                                tool.get("name")
                                    .and_then(Value::as_str)
                                    .map(str::trim)
                                    .filter(|name| !name.is_empty())
                                    .map(str::to_string)
                                    .or_else(|| {
                                        let normalized_key = key.trim();
                                        if normalized_key.is_empty() {
                                            None
                                        } else {
                                            Some(normalized_key.to_string())
                                        }
                                    })
                            })
                            .collect::<Vec<_>>(),
                        _ => Vec::new(),
                    };
                    tools.sort();
                    tools.dedup();

                    let url = entry
                        .get("url")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string);

                    Some(McpServerListEntry {
                        id,
                        name,
                        transport,
                        status,
                        status_reason,
                        auth_status,
                        tools,
                        url,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    data.sort_by(|a, b| a.name.cmp(&b.name));

    let total = result
        .get("total")
        .and_then(|value| {
            value
                .as_u64()
                .and_then(|count| usize::try_from(count).ok())
                .or_else(|| value.as_i64().and_then(|count| usize::try_from(count).ok()))
                .or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw| raw.trim().parse::<usize>().ok())
                })
        })
        .unwrap_or(data.len());

    let elapsed_ms = result
        .get("elapsedMs")
        .or_else(|| result.get("elapsed_ms"))
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|millis| u64::try_from(millis).ok()))
                .or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw| raw.trim().parse::<u64>().ok())
                })
        })
        .unwrap_or(fallback_elapsed_ms);

    McpServerListResponse {
        data,
        total,
        elapsed_ms,
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLoginRequest {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLoginResponse {
    pub name: String,
    pub authorization_url: Option<String>,
    pub started: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpReloadResponse {
    pub reloaded: bool,
    pub elapsed_ms: u64,
}

#[allow(dead_code)]
pub fn parse_mcp_login_runtime_result(
    result: &Value,
    fallback_elapsed_ms: u64,
) -> McpLoginResponse {
    let name = result
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "server".to_string());

    let authorization_url = result
        .get("authorizationUrl")
        .or_else(|| result.get("authorization_url"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let started = result
        .get("started")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let elapsed_ms = result
        .get("elapsedMs")
        .or_else(|| result.get("elapsed_ms"))
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|millis| u64::try_from(millis).ok()))
                .or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw| raw.trim().parse::<u64>().ok())
                })
        })
        .unwrap_or(fallback_elapsed_ms);

    McpLoginResponse {
        name,
        authorization_url,
        started,
        elapsed_ms,
    }
}

#[allow(dead_code)]
pub fn parse_mcp_reload_runtime_result(
    result: &Value,
    fallback_elapsed_ms: u64,
) -> McpReloadResponse {
    let reloaded = result
        .get("reloaded")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let elapsed_ms = result
        .get("elapsedMs")
        .or_else(|| result.get("elapsed_ms"))
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|millis| u64::try_from(millis).ok()))
                .or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw| raw.trim().parse::<u64>().ok())
                })
        })
        .unwrap_or(fallback_elapsed_ms);

    McpReloadResponse {
        reloaded,
        elapsed_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_mcp_server_list_runtime_result;
    use serde_json::json;

    #[test]
    fn parse_mcp_server_list_supports_tools_object() {
        let result = json!({
            "data": [
                {
                    "name": "playwright",
                    "authStatus": "oAuth",
                    "tools": {
                        "browser_navigate": { "name": "browser.navigate" },
                        "browser_click": {}
                    }
                }
            ]
        });

        let parsed = parse_mcp_server_list_runtime_result(&result, 42);
        assert_eq!(parsed.data.len(), 1);
        assert_eq!(parsed.data[0].name, "playwright");
        assert_eq!(parsed.data[0].auth_status, "oauth");
        assert_eq!(
            parsed.data[0].tools,
            vec!["browser.navigate".to_string(), "browser_click".to_string()]
        );
        assert_eq!(parsed.elapsed_ms, 42);
    }

    #[test]
    fn parse_mcp_server_list_supports_legacy_tools_array() {
        let result = json!({
            "data": [
                {
                    "name": "legacy",
                    "tools": ["tool_a", "tool_b", "tool_a"]
                }
            ],
            "elapsedMs": 12
        });

        let parsed = parse_mcp_server_list_runtime_result(&result, 5);
        assert_eq!(parsed.data.len(), 1);
        assert_eq!(
            parsed.data[0].tools,
            vec!["tool_a".to_string(), "tool_b".to_string()]
        );
        assert_eq!(parsed.elapsed_ms, 12);
    }

    #[test]
    fn parse_mcp_server_list_normalizes_unsupported_auth_status() {
        let result = json!({
            "data": [
                {
                    "name": "legacy-auth",
                    "authStatus": "unsupported"
                }
            ]
        });

        let parsed = parse_mcp_server_list_runtime_result(&result, 9);
        assert_eq!(parsed.data.len(), 1);
        assert_eq!(parsed.data[0].auth_status, "not_logged_in");
    }
}
