use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::RuntimeCodexConfig;

use super::rate_limit_snapshot::{StatusRateLimitSnapshot, StatusRateLimitWindow};

fn format_limit_window_label(window_minutes: Option<i64>) -> String {
    match window_minutes.unwrap_or(0) {
        300 => "5h".to_string(),
        10080 => "week".to_string(),
        value if value > 0 => format!("{value}m"),
        _ => "window".to_string(),
    }
}

fn format_limit_reset_eta(resets_at: Option<i64>) -> String {
    let Some(target_epoch) = resets_at else {
        return "n/a".to_string();
    };

    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64);

    let Some(now_epoch) = now_epoch else {
        return "n/a".to_string();
    };

    let seconds_remaining = (target_epoch - now_epoch).max(0);
    if seconds_remaining == 0 {
        return "now".to_string();
    }

    let hours = seconds_remaining / 3600;
    let minutes = (seconds_remaining % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn format_rate_limit_window_status(window: &StatusRateLimitWindow) -> String {
    let used = window.used_percent.clamp(0.0, 100.0);
    let remaining = (100.0 - used).clamp(0.0, 100.0);
    let reset_eta = format_limit_reset_eta(window.resets_at);

    format!(
        "{:.0}% remaining ({:.0}% used), resets in {reset_eta}",
        remaining, used
    )
}

pub(crate) fn format_non_tui_status(
    session_id: u64,
    pid: Option<u32>,
    thread_id: Option<&str>,
    cwd: &Path,
    runtime: &RuntimeCodexConfig,
    transport: crate::SessionTransport,
    rate_limits: Option<&StatusRateLimitSnapshot>,
) -> String {
    let pid_display = pid
        .map(|value| value.to_string())
        .unwrap_or_else(|| "n/a".to_string());
    let thread_display = thread_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("n/a");

    let mode_line = match transport {
        crate::SessionTransport::Native => "mode: native-runtime",
    };

    let mut lines = vec![
        "/status".to_string(),
        mode_line.to_string(),
        format!("session: #{session_id} (pid {pid_display})"),
        format!("thread: {thread_display}"),
        format!("workspace: {}", cwd.display()),
        format!("model: {}", runtime.model),
        format!("reasoning: {}", runtime.reasoning),
        format!("approval: {}", runtime.approval_policy),
        format!("sandbox: {}", runtime.sandbox),
        format!("web search: {}", runtime.web_search_mode),
    ];

    if let Some(snapshot) = rate_limits {
        if let Some(limit_id) = snapshot.limit_id.as_deref() {
            lines.push(format!("limit id: {limit_id}"));
        }
        if let Some(limit_name) = snapshot.limit_name.as_deref() {
            lines.push(format!("limit name: {limit_name}"));
        }
        if let Some(primary) = snapshot.primary.as_ref() {
            lines.push(format!(
                "remaining {}: {}",
                format_limit_window_label(primary.window_minutes),
                format_rate_limit_window_status(primary)
            ));
        }
        if let Some(secondary) = snapshot.secondary.as_ref() {
            lines.push(format!(
                "remaining {}: {}",
                format_limit_window_label(secondary.window_minutes),
                format_rate_limit_window_status(secondary)
            ));
        }
        if snapshot.primary.is_none() && snapshot.secondary.is_none() {
            lines.push("rate limits: unavailable".to_string());
        }
    } else {
        lines.push("rate limits: unavailable".to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn runtime_config_fixture() -> RuntimeCodexConfig {
        RuntimeCodexConfig {
            model: "gpt-5".to_string(),
            reasoning: "high".to_string(),
            approval_preset: "manual".to_string(),
            approval_policy: "never".to_string(),
            sandbox: "workspace-write".to_string(),
            profile: "read_write_with_approval".to_string(),
            web_search_mode: "enabled".to_string(),
        }
    }

    #[test]
    fn formats_status_contract_with_unavailable_rate_limits_golden() {
        let runtime = runtime_config_fixture();
        let status = format_non_tui_status(
            42,
            Some(1234),
            None,
            Path::new("workspace-dir"),
            &runtime,
            crate::SessionTransport::Native,
            None,
        );

        let expected = [
            "/status",
            "mode: native-runtime",
            "session: #42 (pid 1234)",
            "thread: n/a",
            "workspace: workspace-dir",
            "model: gpt-5",
            "reasoning: high",
            "approval: never",
            "sandbox: workspace-write",
            "web search: enabled",
            "rate limits: unavailable",
        ]
        .join("\n");

        assert_eq!(status, expected);
    }

    #[test]
    fn formats_status_with_rate_limit_windows_golden() {
        let runtime = runtime_config_fixture();
        let snapshot = StatusRateLimitSnapshot {
            limit_id: Some("codex-pro".to_string()),
            limit_name: Some("Codex Pro".to_string()),
            primary: Some(StatusRateLimitWindow {
                used_percent: 25.0,
                window_minutes: Some(300),
                resets_at: None,
            }),
            secondary: Some(StatusRateLimitWindow {
                used_percent: 60.0,
                window_minutes: Some(10080),
                resets_at: None,
            }),
        };
        let status = format_non_tui_status(
            7,
            Some(4321),
            Some("thread-123"),
            Path::new("workspace-dir"),
            &runtime,
            crate::SessionTransport::Native,
            Some(&snapshot),
        );

        let expected = [
            "/status",
            "mode: native-runtime",
            "session: #7 (pid 4321)",
            "thread: thread-123",
            "workspace: workspace-dir",
            "model: gpt-5",
            "reasoning: high",
            "approval: never",
            "sandbox: workspace-write",
            "web search: enabled",
            "limit id: codex-pro",
            "limit name: Codex Pro",
            "remaining 5h: 75% remaining (25% used), resets in n/a",
            "remaining week: 40% remaining (60% used), resets in n/a",
        ]
        .join("\n");

        assert_eq!(status, expected);
    }

    #[test]
    fn formats_rate_limit_window_status_with_reset_eta() {
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after UNIX_EPOCH in tests")
            .as_secs() as i64;
        let window = StatusRateLimitWindow {
            used_percent: 20.0,
            window_minutes: Some(300),
            resets_at: Some(now_epoch + 24 * 60 * 60),
        };

        let formatted = format_rate_limit_window_status(&window);
        assert!(formatted.starts_with("80% remaining (20% used), resets in "));

        let eta = formatted
            .split("resets in ")
            .nth(1)
            .expect("formatted output should include reset ETA");
        assert!(!eta.is_empty());
        assert_ne!(eta, "n/a");
        assert_ne!(eta, "now");
        assert!(eta.ends_with('m'));
    }
    #[test]
    fn formats_unavailable_when_snapshot_has_no_windows() {
        let runtime = runtime_config_fixture();
        let snapshot = StatusRateLimitSnapshot {
            limit_id: Some("codex-lite".to_string()),
            limit_name: Some("Codex Lite".to_string()),
            primary: None,
            secondary: None,
        };

        let status = format_non_tui_status(
            9,
            None,
            Some(""),
            Path::new("workspace-dir"),
            &runtime,
            crate::SessionTransport::Native,
            Some(&snapshot),
        );

        let expected = [
            "/status",
            "mode: native-runtime",
            "session: #9 (pid n/a)",
            "thread: n/a",
            "workspace: workspace-dir",
            "model: gpt-5",
            "reasoning: high",
            "approval: never",
            "sandbox: workspace-write",
            "web search: enabled",
            "limit id: codex-lite",
            "limit name: Codex Lite",
            "rate limits: unavailable",
        ]
        .join("\n");

        assert_eq!(status, expected);
    }
}
