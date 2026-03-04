use serde_json::{json, Value};
#[cfg(feature = "native-codex-runtime")]
use std::time::Duration;
use std::time::Instant;
use tauri::State;

use crate::account_runtime::{
    parse_account_login_start_runtime_result, parse_account_logout_runtime_result,
    parse_account_rate_limits_runtime_result, parse_account_read_runtime_result,
    parse_app_list_runtime_result, AccountLoginStartRequest, AccountLoginStartResponse,
    AccountLogoutResponse, AccountRateLimitsReadResponse, AccountReadRequest, AccountReadResponse,
    AppListRequest, AppListResponse,
};
use crate::domain::account_mcp::validation as account_mcp_validation;
use crate::infrastructure::runtime_bridge::{app_server as app_server_bridge, mcp_native};
use crate::mcp_runtime::{
    McpLoginRequest, McpLoginResponse, McpReloadResponse, McpServerListResponse,
    McpStartupWarmupResponse,
};
use crate::AppState;

pub(crate) async fn codex_wait_for_mcp_startup_impl(
    state: State<'_, AppState>,
) -> Result<McpStartupWarmupResponse, String> {
    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let (runtime, cwd) = mcp_native::native_runtime_and_cwd_for_mcp(&state).await?;
        let list = mcp_native::collect_native_mcp_server_list(runtime, cwd, 0).await?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let ready_servers = list
            .data
            .into_iter()
            .filter(|entry| entry.status == "connected")
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        let total_ready = ready_servers.len();

        Ok(McpStartupWarmupResponse {
            ready_servers,
            total_ready,
            elapsed_ms,
        })
    }

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = state;
        Err("mcp warmup requires native runtime support in this build".to_string())
    }
}

fn app_list_response_from_runtime_result(
    result: Result<Value, String>,
    elapsed_ms: u64,
) -> Result<AppListResponse, String> {
    match result {
        Ok(result) => Ok(parse_app_list_runtime_result(&result, elapsed_ms)),
        Err(error) => {
            if account_mcp_validation::is_unsupported_method_error_for(
                &error,
                &["app.list", "app/list"],
            ) {
                Ok(AppListResponse {
                    data: Vec::new(),
                    next_cursor: None,
                    total: 0,
                    elapsed_ms,
                })
            } else {
                Err(error)
            }
        }
    }
}

pub(crate) async fn codex_app_list_impl(
    state: State<'_, AppState>,
    request: AppListRequest,
) -> Result<AppListResponse, String> {
    let native_binary_cwd_context = mcp_native::active_native_binary_cwd_context(&state)?;
    let payload = account_mcp_validation::build_app_list_payload(&request);

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("app list requires an active codex session".to_string());
    };

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = (native_binary_cwd_context, payload);
        return Err("app list requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let result = app_server_bridge::request_app_server_method(
            &binary,
            &cwd,
            "app/list",
            Value::Object(payload),
            Duration::from_secs(90),
        );
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;

        app_list_response_from_runtime_result(result, elapsed_ms)
    }
}

pub(crate) async fn codex_account_read_impl(
    state: State<'_, AppState>,
    request: AccountReadRequest,
) -> Result<AccountReadResponse, String> {
    let native_binary_cwd_context = mcp_native::active_native_binary_cwd_context(&state)?;

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("account read requires an active codex session".to_string());
    };

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = (native_binary_cwd_context, request);
        return Err("account read requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let result = app_server_bridge::request_app_server_method(
            &binary,
            &cwd,
            "account/read",
            json!({
                "refreshToken": request.refresh_token,
            }),
            Duration::from_secs(90),
        )?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(parse_account_read_runtime_result(&result, elapsed_ms))
    }
}

pub(crate) async fn codex_account_login_start_impl(
    state: State<'_, AppState>,
    request: AccountLoginStartRequest,
) -> Result<AccountLoginStartResponse, String> {
    let native_binary_cwd_context = mcp_native::active_native_binary_cwd_context(&state)?;

    let login_start = account_mcp_validation::validate_account_login_start_request(request)?;
    let login_type = login_start.login_type;
    let is_chatgpt = login_start.is_chatgpt;
    let payload = login_start.payload;

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("account login requires an active codex session".to_string());
    };

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = (native_binary_cwd_context, payload);
        return Err("account login requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();

        if is_chatgpt {
            let result = app_server_bridge::run_codex_command_with_context(
                &binary,
                vec!["login".to_string()],
                Some(cwd.as_path()),
            )?;

            if !result.success {
                let details = if result.stderr.trim().is_empty() {
                    result.stdout.trim().to_string()
                } else {
                    result.stderr.trim().to_string()
                };
                return Err(format!("codex login failed: {details}"));
            }

            let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
            return Ok(AccountLoginStartResponse {
                login_type,
                login_id: None,
                auth_url: None,
                started: true,
                elapsed_ms,
            });
        }

        let result = app_server_bridge::request_app_server_method(
            &binary,
            &cwd,
            "account/login/start",
            Value::Object(payload),
            Duration::from_secs(90),
        )?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(parse_account_login_start_runtime_result(
            &result, elapsed_ms,
        ))
    }
}

pub(crate) async fn codex_account_logout_impl(
    state: State<'_, AppState>,
) -> Result<AccountLogoutResponse, String> {
    let native_binary_cwd_context = mcp_native::active_native_binary_cwd_context(&state)?;

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("account logout requires an active codex session".to_string());
    };

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = native_binary_cwd_context;
        return Err("account logout requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let result = app_server_bridge::request_app_server_method(
            &binary,
            &cwd,
            "account/logout",
            Value::Null,
            Duration::from_secs(90),
        )?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(parse_account_logout_runtime_result(&result, elapsed_ms))
    }
}

pub(crate) async fn codex_account_rate_limits_read_impl(
    state: State<'_, AppState>,
) -> Result<AccountRateLimitsReadResponse, String> {
    let native_binary_cwd_context = mcp_native::active_native_binary_cwd_context(&state)?;

    #[cfg(feature = "native-codex-runtime")]
    let Some((binary, cwd)) = native_binary_cwd_context
    else {
        return Err("account rate-limits requires an active codex session".to_string());
    };

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = native_binary_cwd_context;
        return Err(
            "account rate-limits requires native runtime support in this build".to_string(),
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let result = app_server_bridge::request_app_server_method(
            &binary,
            &cwd,
            "account/rateLimits/read",
            json!({}),
            Duration::from_secs(90),
        )?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(parse_account_rate_limits_runtime_result(
            &result, elapsed_ms,
        ))
    }
}

pub(crate) async fn codex_mcp_list_impl(
    state: State<'_, AppState>,
) -> Result<McpServerListResponse, String> {
    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let (runtime, cwd) = mcp_native::native_runtime_and_cwd_for_mcp(&state).await?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        mcp_native::collect_native_mcp_server_list(runtime, cwd, elapsed_ms).await
    }

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = state;
        Err("mcp list requires native runtime support in this build".to_string())
    }
}

pub(crate) async fn codex_mcp_login_impl(
    state: State<'_, AppState>,
    request: McpLoginRequest,
) -> Result<McpLoginResponse, String> {
    let login = account_mcp_validation::normalize_mcp_login_request(request)?;

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        let (runtime, cwd) = mcp_native::native_runtime_and_cwd_for_mcp(&state).await?;
        let config =
            mcp_native::build_native_config_for_cwd(runtime.as_ref(), cwd, "MCP login").await?;

        let authorization_url = mcp_native::start_mcp_oauth_login(
            &config,
            &login.name,
            &login.scopes,
            login.timeout_secs,
        )
        .await?;

        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Ok(McpLoginResponse {
            name: login.name,
            authorization_url: Some(authorization_url),
            started: true,
            elapsed_ms,
        })
    }

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = (state, login);
        Err("mcp login requires native runtime support in this build".to_string())
    }
}

pub(crate) async fn codex_mcp_reload_impl(
    state: State<'_, AppState>,
) -> Result<McpReloadResponse, String> {
    let native_reload_context = mcp_native::active_native_reload_context(&state)?;

    #[cfg(feature = "native-codex-runtime")]
    let Some((runtime, cwd)) = native_reload_context
    else {
        return Err("mcp reload requires an active codex session".to_string());
    };

    #[cfg(not(feature = "native-codex-runtime"))]
    {
        let _ = native_reload_context;
        return Err("mcp reload requires native runtime support in this build".to_string());
    }

    #[cfg(feature = "native-codex-runtime")]
    {
        let started_at = Instant::now();
        mcp_native::refresh_mcp_servers(runtime.as_ref(), cwd).await?;
        let elapsed_ms = started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;

        Ok(McpReloadResponse {
            reloaded: true,
            elapsed_ms,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_list_returns_empty_payload_when_method_is_unsupported() {
        let response = app_list_response_from_runtime_result(
            Err("json-rpc error: unsupported method app/list".to_string()),
            41,
        )
        .expect("unsupported method should return empty app list");

        assert!(response.data.is_empty());
        assert_eq!(response.next_cursor, None);
        assert_eq!(response.total, 0);
        assert_eq!(response.elapsed_ms, 41);
    }
}
