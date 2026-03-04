use crate::account_runtime::{AccountLoginStartRequest, AppListRequest};
use crate::mcp_runtime::McpLoginRequest;
use serde_json::{json, Map, Value};

pub(crate) struct AccountLoginStartInput {
    pub login_type: String,
    pub payload: Map<String, Value>,
    pub is_chatgpt: bool,
}

pub(crate) struct McpLoginInput {
    pub name: String,
    pub scopes: Vec<String>,
    pub timeout_secs: Option<u64>,
}

pub(crate) fn build_app_list_payload(request: &AppListRequest) -> Map<String, Value> {
    let mut payload = Map::new();

    if let Some(cursor) = request
        .cursor
        .as_deref()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        payload.insert("cursor".to_string(), json!(cursor));
    }

    if let Some(limit) = request.limit {
        payload.insert("limit".to_string(), json!(limit));
    }

    if let Some(thread_id) = request
        .thread_id
        .as_deref()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        payload.insert("threadId".to_string(), json!(thread_id));
    }

    if request.force_refetch {
        payload.insert("forceRefetch".to_string(), json!(true));
    }

    payload
}

pub(crate) fn validate_account_login_start_request(
    request: AccountLoginStartRequest,
) -> Result<AccountLoginStartInput, String> {
    let login_type = request.login_type.trim();
    if login_type.is_empty() {
        return Err("type is required".to_string());
    }

    let mut payload = Map::new();
    if login_type.eq_ignore_ascii_case("chatgpt") {
        payload.insert("type".to_string(), json!("chatgpt"));
        payload.insert("authMode".to_string(), json!("chatgpt"));
        return Ok(AccountLoginStartInput {
            login_type: "chatgpt".to_string(),
            payload,
            is_chatgpt: true,
        });
    }

    if login_type.eq_ignore_ascii_case("apikey")
        || login_type.eq_ignore_ascii_case("api_key")
        || login_type.eq_ignore_ascii_case("apiKey")
    {
        let api_key = request
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .ok_or_else(|| "apiKey is required for type=apiKey".to_string())?;

        payload.insert("type".to_string(), json!("apiKey"));
        payload.insert("authMode".to_string(), json!("api_key"));
        payload.insert("apiKey".to_string(), json!(api_key));

        return Ok(AccountLoginStartInput {
            login_type: "apiKey".to_string(),
            payload,
            is_chatgpt: false,
        });
    }

    Err("unsupported account login type".to_string())
}

pub(crate) fn normalize_mcp_login_request(
    request: McpLoginRequest,
) -> Result<McpLoginInput, String> {
    let name = request.name.trim().to_string();
    if name.is_empty() {
        return Err("name is required".to_string());
    }

    let scopes = request
        .scopes
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();

    Ok(McpLoginInput {
        name,
        scopes,
        timeout_secs: request.timeout_secs,
    })
}

fn is_unsupported_method_message(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("unsupported method") || normalized.contains("method not found")
}

pub(crate) fn is_unsupported_method_error_for(error: &str, methods: &[&str]) -> bool {
    if !is_unsupported_method_message(error) {
        return false;
    }

    let normalized = error.to_ascii_lowercase();
    methods.iter().any(|method| {
        let dotted = method.to_ascii_lowercase();
        let slashed = dotted.replace('.', "/");
        normalized.contains(&dotted) || normalized.contains(&slashed)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_app_list_payload_includes_supported_fields() {
        let request = AppListRequest {
            cursor: Some("  cursor-123  ".to_string()),
            limit: Some(25),
            thread_id: Some("  thread-42 ".to_string()),
            force_refetch: true,
        };

        let payload = build_app_list_payload(&request);
        assert_eq!(payload.get("cursor"), Some(&json!("cursor-123")));
        assert_eq!(payload.get("limit"), Some(&json!(25)));
        assert_eq!(payload.get("threadId"), Some(&json!("thread-42")));
        assert_eq!(payload.get("forceRefetch"), Some(&json!(true)));
        assert_eq!(payload.len(), 4);
    }

    #[test]
    fn build_app_list_payload_ignores_empty_values() {
        let request = AppListRequest {
            cursor: Some("   ".to_string()),
            limit: None,
            thread_id: Some("".to_string()),
            force_refetch: false,
        };

        let payload = build_app_list_payload(&request);
        assert!(payload.is_empty());
    }

    #[test]
    fn validate_account_login_start_request_supports_chatgpt() {
        let request = AccountLoginStartRequest {
            login_type: "  ChatGPT  ".to_string(),
            api_key: Some("ignored".to_string()),
        };

        let parsed = validate_account_login_start_request(request)
            .expect("chatgpt login type should be accepted");
        assert_eq!(parsed.login_type, "chatgpt");
        assert!(parsed.is_chatgpt);
        assert_eq!(parsed.payload.get("type"), Some(&json!("chatgpt")));
        assert_eq!(parsed.payload.get("authMode"), Some(&json!("chatgpt")));
    }

    #[test]
    fn validate_account_login_start_request_supports_api_key() {
        let request = AccountLoginStartRequest {
            login_type: " api_key ".to_string(),
            api_key: Some("  sk-test-123  ".to_string()),
        };

        let parsed = validate_account_login_start_request(request)
            .expect("api key login type should be accepted");
        assert_eq!(parsed.login_type, "apiKey");
        assert!(!parsed.is_chatgpt);
        assert_eq!(parsed.payload.get("type"), Some(&json!("apiKey")));
        assert_eq!(parsed.payload.get("authMode"), Some(&json!("api_key")));
        assert_eq!(parsed.payload.get("apiKey"), Some(&json!("sk-test-123")));
    }

    #[test]
    fn validate_account_login_start_request_rejects_missing_api_key() {
        let request = AccountLoginStartRequest {
            login_type: "apiKey".to_string(),
            api_key: Some("   ".to_string()),
        };

        let error = validate_account_login_start_request(request)
            .err()
            .expect("api key login should require api_key");
        assert_eq!(error, "apiKey is required for type=apiKey");
    }

    #[test]
    fn is_unsupported_method_error_for_accepts_dotted_and_slashed_names() {
        assert!(is_unsupported_method_error_for(
            "JSON-RPC error: unsupported method app.list",
            &["app.list"]
        ));
        assert!(is_unsupported_method_error_for(
            "method not found: app/list",
            &["app.list"]
        ));
    }

    #[test]
    fn is_unsupported_method_error_for_rejects_other_errors() {
        assert!(!is_unsupported_method_error_for(
            "request failed: timeout",
            &["app.list"]
        ));
        assert!(!is_unsupported_method_error_for(
            "unsupported method account.read",
            &["app.list", "app/list"]
        ));
    }
}
