use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct StatusRateLimitWindow {
    pub(crate) used_percent: f64,
    pub(crate) window_minutes: Option<i64>,
    pub(crate) resets_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub(crate) struct StatusRateLimitSnapshot {
    pub(crate) limit_id: Option<String>,
    pub(crate) limit_name: Option<String>,
    pub(crate) primary: Option<StatusRateLimitWindow>,
    pub(crate) secondary: Option<StatusRateLimitWindow>,
}

fn parse_rate_limit_window(value: &Value) -> Option<StatusRateLimitWindow> {
    let object = value.as_object()?;
    let used_percent = object
        .get("usedPercent")
        .and_then(Value::as_f64)
        .or_else(|| object.get("used_percent").and_then(Value::as_f64))?;
    let window_minutes = object
        .get("windowDurationMins")
        .and_then(Value::as_i64)
        .or_else(|| object.get("window_minutes").and_then(Value::as_i64));
    let resets_at = object
        .get("resetsAt")
        .and_then(Value::as_i64)
        .or_else(|| object.get("resets_at").and_then(Value::as_i64));

    Some(StatusRateLimitWindow {
        used_percent,
        window_minutes,
        resets_at,
    })
}

fn parse_rate_limit_snapshot(value: &Value) -> Option<StatusRateLimitSnapshot> {
    let object = value.as_object()?;
    let limit_id = object
        .get("limitId")
        .and_then(Value::as_str)
        .or_else(|| object.get("limit_id").and_then(Value::as_str))
        .map(|value| value.to_string());
    let limit_name = object
        .get("limitName")
        .and_then(Value::as_str)
        .or_else(|| object.get("limit_name").and_then(Value::as_str))
        .map(|value| value.to_string());
    let primary = object.get("primary").and_then(parse_rate_limit_window);
    let secondary = object.get("secondary").and_then(parse_rate_limit_window);

    if primary.is_none() && secondary.is_none() {
        return None;
    }

    Some(StatusRateLimitSnapshot {
        limit_id,
        limit_name,
        primary,
        secondary,
    })
}

fn pick_rate_limit_snapshot(result: &Value) -> Option<StatusRateLimitSnapshot> {
    let object = result.as_object()?;

    if let Some(snapshot) = object.get("rateLimits").and_then(parse_rate_limit_snapshot) {
        return Some(snapshot);
    }

    let by_limit_id = object.get("rateLimitsByLimitId")?.as_object()?;

    let mut first_snapshot: Option<StatusRateLimitSnapshot> = None;
    for (key, value) in by_limit_id {
        if let Some(mut snapshot) = parse_rate_limit_snapshot(value) {
            if snapshot.limit_id.is_none() {
                snapshot.limit_id = Some(key.clone());
            }
            if snapshot
                .limit_id
                .as_ref()
                .map(|id| id.starts_with("codex"))
                .unwrap_or(false)
            {
                return Some(snapshot);
            }
            if first_snapshot.is_none() {
                first_snapshot = Some(snapshot);
            }
        }
    }

    first_snapshot
}

pub(crate) fn extract_rate_limits_from_app_server_message(
    message: &Value,
) -> Option<StatusRateLimitSnapshot> {
    if message
        .get("method")
        .and_then(Value::as_str)
        .is_some_and(|method| method == "account/rateLimits/updated")
    {
        return message
            .get("params")
            .and_then(|params| params.get("rateLimits"))
            .and_then(parse_rate_limit_snapshot);
    }

    if message
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(|id| id == "alicia-rate-limits")
    {
        return message.get("result").and_then(pick_rate_limit_snapshot);
    }

    None
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_account_rate_limits_updated_message() {
        let message = json!({
            "method": "account/rateLimits/updated",
            "params": {
                "rateLimits": {
                    "limitId": "codex-pro",
                    "limitName": "Codex Pro",
                    "primary": {
                        "usedPercent": 35.0,
                        "windowDurationMins": 300,
                        "resetsAt": 1_700_000_000
                    },
                    "secondary": {
                        "used_percent": 50.0,
                        "window_minutes": 10080,
                        "resets_at": 1_700_060_000
                    }
                }
            }
        });

        let snapshot = extract_rate_limits_from_app_server_message(&message)
            .expect("expected snapshot from update notification");

        assert_eq!(snapshot.limit_id.as_deref(), Some("codex-pro"));
        assert_eq!(snapshot.limit_name.as_deref(), Some("Codex Pro"));

        let primary = snapshot.primary.expect("expected primary window");
        assert_eq!(primary.used_percent, 35.0);
        assert_eq!(primary.window_minutes, Some(300));
        assert_eq!(primary.resets_at, Some(1_700_000_000));

        let secondary = snapshot.secondary.expect("expected secondary window");
        assert_eq!(secondary.used_percent, 50.0);
        assert_eq!(secondary.window_minutes, Some(10080));
        assert_eq!(secondary.resets_at, Some(1_700_060_000));
    }

    #[test]
    fn parses_alicia_rate_limits_response_and_prefers_codex_limit() {
        let message = json!({
            "id": "alicia-rate-limits",
            "result": {
                "rateLimitsByLimitId": {
                    "acme-basic": {
                        "limitName": "Acme Basic",
                        "primary": {
                            "usedPercent": 15.0,
                            "windowDurationMins": 60,
                            "resetsAt": 1_700_000_000
                        }
                    },
                    "codex-plus": {
                        "limit_name": "Codex Plus",
                        "primary": {
                            "used_percent": 20.0,
                            "window_minutes": 300,
                            "resets_at": 1_700_010_000
                        }
                    }
                }
            }
        });

        let snapshot = extract_rate_limits_from_app_server_message(&message)
            .expect("expected snapshot from alicia-rate-limits result");

        assert_eq!(snapshot.limit_id.as_deref(), Some("codex-plus"));
        assert_eq!(snapshot.limit_name.as_deref(), Some("Codex Plus"));

        let primary = snapshot.primary.expect("expected primary window");
        assert_eq!(primary.used_percent, 20.0);
        assert_eq!(primary.window_minutes, Some(300));
        assert_eq!(primary.resets_at, Some(1_700_010_000));
        assert!(snapshot.secondary.is_none());
    }

    #[test]
    fn parses_alicia_rate_limits_response_falls_back_to_first_valid_snapshot() {
        let message = json!({
            "id": "alicia-rate-limits",
            "result": {
                "rateLimitsByLimitId": {
                    "a-basic": {
                        "limitName": "A Basic",
                        "primary": {
                            "usedPercent": 12.0,
                            "windowDurationMins": 60,
                            "resetsAt": 1_700_000_000
                        }
                    },
                    "z-pro": {
                        "limitName": "Z Pro",
                        "primary": {
                            "usedPercent": 55.0,
                            "windowDurationMins": 120,
                            "resetsAt": 1_700_020_000
                        }
                    }
                }
            }
        });

        let snapshot = extract_rate_limits_from_app_server_message(&message)
            .expect("expected fallback snapshot from first valid non-codex limit");

        assert_eq!(snapshot.limit_id.as_deref(), Some("a-basic"));
        assert_eq!(snapshot.limit_name.as_deref(), Some("A Basic"));

        let primary = snapshot.primary.expect("expected primary window");
        assert_eq!(primary.used_percent, 12.0);
        assert_eq!(primary.window_minutes, Some(60));
        assert_eq!(primary.resets_at, Some(1_700_000_000));
    }

    #[test]
    fn returns_none_for_incompatible_rate_limit_messages() {
        let invalid_result_payload = json!({
            "id": "alicia-rate-limits",
            "result": {
                "rateLimitsByLimitId": {
                    "codex-pro": {
                        "limitName": "Codex Pro"
                    }
                }
            }
        });

        assert!(extract_rate_limits_from_app_server_message(&invalid_result_payload).is_none());

        let invalid_update_payload = json!({
            "method": "account/rateLimits/updated",
            "params": {
                "rateLimits": "not-an-object"
            }
        });

        assert!(extract_rate_limits_from_app_server_message(&invalid_update_payload).is_none());
    }
}
