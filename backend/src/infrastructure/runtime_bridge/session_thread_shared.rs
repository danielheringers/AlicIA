#[cfg(feature = "native-codex-runtime")]
use std::path::{Path, PathBuf};
#[cfg(feature = "native-codex-runtime")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "native-codex-runtime")]
use codex_app_server_protocol::build_turns_from_rollout_items;
#[cfg(feature = "native-codex-runtime")]
use codex_app_server_protocol::{
    ThreadItem as ApiThreadItem, Turn as ApiTurn, UserInput as ApiUserInput,
};
#[cfg(feature = "native-codex-runtime")]
use codex_core::config::{ConfigBuilder, ConfigOverrides};
#[cfg(feature = "native-codex-runtime")]
use codex_core::{read_session_meta_line, RolloutRecorder, ThreadItem as NativeThreadItem};
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::protocol::{InitialHistory, SessionSource};
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::ThreadId;
#[cfg(feature = "native-codex-runtime")]
use toml::map::Map as TomlMap;

#[cfg(feature = "native-codex-runtime")]
use crate::interface::tauri::dto::{
    CodexThreadSummary, CodexThreadTurnHistoryMessage, CodexThreadTurnSummary,
};

#[cfg(feature = "native-codex-runtime")]
pub(crate) const ALICIA_NATIVE_INTERNAL_PROFILE: &str = "__alicia_native_internal";

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn native_internal_profile_cli_overrides() -> Vec<(String, toml::Value)> {
    vec![(
        format!("profiles.{ALICIA_NATIVE_INTERNAL_PROFILE}"),
        toml::Value::Table(TomlMap::new()),
    )]
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn native_profile_harness_overrides(cwd: &Path) -> ConfigOverrides {
    ConfigOverrides {
        cwd: Some(cwd.to_path_buf()),
        config_profile: Some(ALICIA_NATIVE_INTERNAL_PROFILE.to_string()),
        ..Default::default()
    }
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn native_config_builder(codex_home: PathBuf, cwd: &Path) -> ConfigBuilder {
    ConfigBuilder::default()
        .codex_home(codex_home)
        .fallback_cwd(Some(cwd.to_path_buf()))
        .cli_overrides(native_internal_profile_cli_overrides())
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn native_now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or(0)
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn native_path_epoch_seconds(path: &Path) -> i64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or_else(native_now_epoch_seconds)
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn infer_thread_id_from_rollout_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    if stem.len() < 36 {
        return None;
    }
    let candidate = &stem[stem.len() - 36..];
    ThreadId::from_string(candidate).ok()?;
    Some(candidate.to_string())
}

#[cfg(feature = "native-codex-runtime")]
fn native_status_to_string(value: &impl serde::Serialize, fallback: &str) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(|entry| entry.to_string()))
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(feature = "native-codex-runtime")]
fn native_user_input_to_text(input: &ApiUserInput) -> Option<String> {
    match input {
        ApiUserInput::Text { text, .. } => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        ApiUserInput::Image { url } => Some(format!("[image] {url}")),
        ApiUserInput::LocalImage { path } => {
            Some(format!("[local_image] {}", path.to_string_lossy()))
        }
        ApiUserInput::Skill { name, .. } => Some(format!("[skill] {name}")),
        ApiUserInput::Mention { name, path } => Some(format!("[mention] {name} ({path})")),
    }
}

#[cfg(feature = "native-codex-runtime")]
fn native_thread_item_to_history_message(
    item: &ApiThreadItem,
) -> Option<CodexThreadTurnHistoryMessage> {
    match item {
        ApiThreadItem::UserMessage { content, .. } => {
            let text = content
                .iter()
                .filter_map(native_user_input_to_text)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();

            (!text.is_empty()).then_some(CodexThreadTurnHistoryMessage {
                role: "user".to_string(),
                content: text,
            })
        }
        ApiThreadItem::AgentMessage { text, .. } => {
            let text = text.trim().to_string();
            (!text.is_empty()).then_some(CodexThreadTurnHistoryMessage {
                role: "agent".to_string(),
                content: text,
            })
        }
        ApiThreadItem::Plan { text, .. } => Some(CodexThreadTurnHistoryMessage {
            role: "system".to_string(),
            content: if text.trim().is_empty() {
                "[plan]".to_string()
            } else {
                format!("[plan] {}", text.trim())
            },
        }),
        ApiThreadItem::Reasoning {
            summary, content, ..
        } => {
            let body = [summary.join("\n"), content.join("\n")]
                .into_iter()
                .filter(|entry| !entry.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();

            Some(CodexThreadTurnHistoryMessage {
                role: "system".to_string(),
                content: if body.is_empty() {
                    "[reasoning]".to_string()
                } else {
                    format!("[reasoning]\n{body}")
                },
            })
        }
        ApiThreadItem::CommandExecution {
            command,
            status,
            aggregated_output,
            ..
        } => {
            let status = native_status_to_string(status, "unknown");
            let output = aggregated_output
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(|value| format!("\n{value}"))
                .unwrap_or_default();

            Some(CodexThreadTurnHistoryMessage {
                role: "system".to_string(),
                content: format!("[command:{status}] {command}{output}"),
            })
        }
        ApiThreadItem::FileChange { status, .. } => Some(CodexThreadTurnHistoryMessage {
            role: "system".to_string(),
            content: format!(
                "[file_change:{}]",
                native_status_to_string(status, "unknown")
            ),
        }),
        ApiThreadItem::McpToolCall { tool, status, .. } => Some(CodexThreadTurnHistoryMessage {
            role: "system".to_string(),
            content: format!(
                "[mcp:{}] {tool}",
                native_status_to_string(status, "unknown")
            ),
        }),
        ApiThreadItem::CollabAgentToolCall { tool, status, .. } => {
            Some(CodexThreadTurnHistoryMessage {
                role: "system".to_string(),
                content: format!(
                    "[collab:{}] {}",
                    native_status_to_string(status, "unknown"),
                    native_status_to_string(tool, "collab")
                ),
            })
        }
        ApiThreadItem::WebSearch { query, .. } => Some(CodexThreadTurnHistoryMessage {
            role: "system".to_string(),
            content: format!("[web_search] {query}"),
        }),
        ApiThreadItem::ImageView { path, .. } => Some(CodexThreadTurnHistoryMessage {
            role: "system".to_string(),
            content: format!("[image_view] {path}"),
        }),
        ApiThreadItem::EnteredReviewMode { review, .. } => Some(CodexThreadTurnHistoryMessage {
            role: "system".to_string(),
            content: if review.trim().is_empty() {
                "[review] started".to_string()
            } else {
                format!("[review] started: {}", review.trim())
            },
        }),
        ApiThreadItem::ExitedReviewMode { review, .. } => Some(CodexThreadTurnHistoryMessage {
            role: "system".to_string(),
            content: if review.trim().is_empty() {
                "[review] completed".to_string()
            } else {
                format!("[review] completed\n{}", review.trim())
            },
        }),
        ApiThreadItem::ContextCompaction { .. } => Some(CodexThreadTurnHistoryMessage {
            role: "system".to_string(),
            content: "[context_compaction]".to_string(),
        }),
    }
}

#[cfg(feature = "native-codex-runtime")]
fn native_turn_summary_from_api_turn(turn: ApiTurn) -> CodexThreadTurnSummary {
    let mut messages = Vec::new();
    for item in &turn.items {
        let Some(message) = native_thread_item_to_history_message(item) else {
            continue;
        };
        let should_skip =
            messages
                .last()
                .is_some_and(|previous: &CodexThreadTurnHistoryMessage| {
                    previous.role == message.role && previous.content == message.content
                });
        if !should_skip {
            messages.push(message);
        }
    }

    CodexThreadTurnSummary {
        id: turn.id,
        status: native_status_to_string(&turn.status, "unknown"),
        item_count: turn.items.len(),
        messages,
    }
}

#[cfg(feature = "native-codex-runtime")]
fn native_preview_from_turns(turns: &[CodexThreadTurnSummary]) -> String {
    turns
        .iter()
        .flat_map(|turn| turn.messages.iter())
        .find(|message| message.role == "user" && !message.content.trim().is_empty())
        .map(|message| message.content.clone())
        .unwrap_or_default()
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn native_thread_summary_from_list_item(
    item: NativeThreadItem,
    fallback_provider: &str,
) -> Option<CodexThreadSummary> {
    let thread_id = item
        .thread_id
        .map(|thread_id| thread_id.to_string())
        .or_else(|| infer_thread_id_from_rollout_path(item.path.as_path()))?;

    let updated_at = native_path_epoch_seconds(item.path.as_path());
    let created_at = updated_at;

    Some(CodexThreadSummary {
        id: thread_id.clone(),
        codex_thread_id: Some(thread_id),
        preview: item.first_user_message.unwrap_or_default(),
        model_provider: item
            .model_provider
            .unwrap_or_else(|| fallback_provider.to_string()),
        created_at,
        updated_at,
        cwd: item.cwd.unwrap_or_default().to_string_lossy().to_string(),
        path: Some(item.path.to_string_lossy().to_string()),
        source: item.source.unwrap_or(SessionSource::Unknown).to_string(),
        turn_count: 0,
        turns: Vec::new(),
    })
}

#[cfg(feature = "native-codex-runtime")]
async fn native_turn_summaries_from_rollout_path(
    rollout_path: &Path,
) -> Result<Vec<CodexThreadTurnSummary>, String> {
    let history = RolloutRecorder::get_rollout_history(rollout_path)
        .await
        .map_err(|error| format!("failed to read rollout history: {error}"))?;
    let items = match history {
        InitialHistory::New => Vec::new(),
        InitialHistory::Forked(items) => items,
        InitialHistory::Resumed(resumed) => resumed.history,
    };

    let turns = build_turns_from_rollout_items(&items);
    Ok(turns
        .into_iter()
        .map(native_turn_summary_from_api_turn)
        .collect())
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn native_thread_summary_from_rollout_path(
    rollout_path: &Path,
    fallback_provider: &str,
    include_turns: bool,
    preferred_thread_id: Option<&str>,
) -> Result<CodexThreadSummary, String> {
    let session_meta = read_session_meta_line(rollout_path)
        .await
        .map_err(|error| format!("failed to read thread metadata: {error}"))?;
    let full_turns = native_turn_summaries_from_rollout_path(rollout_path).await?;
    let turn_count = full_turns.len();
    let preview = native_preview_from_turns(&full_turns);
    let turns = if include_turns {
        full_turns
    } else {
        Vec::new()
    };
    let timestamp = native_path_epoch_seconds(rollout_path);
    let thread_id = preferred_thread_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| session_meta.meta.id.to_string());

    Ok(CodexThreadSummary {
        id: thread_id.clone(),
        codex_thread_id: Some(thread_id),
        preview,
        model_provider: session_meta
            .meta
            .model_provider
            .unwrap_or_else(|| fallback_provider.to_string()),
        created_at: timestamp,
        updated_at: timestamp,
        cwd: session_meta.meta.cwd.to_string_lossy().to_string(),
        path: Some(rollout_path.to_string_lossy().to_string()),
        source: session_meta.meta.source.to_string(),
        turn_count,
        turns,
    })
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn native_thread_summary_without_rollout_path(
    thread_id: &str,
    model_provider: String,
    cwd: &Path,
    source: String,
) -> CodexThreadSummary {
    let now = native_now_epoch_seconds();
    let thread_id = thread_id.trim().to_string();

    CodexThreadSummary {
        id: thread_id.clone(),
        codex_thread_id: Some(thread_id),
        preview: String::new(),
        model_provider,
        created_at: now,
        updated_at: now,
        cwd: cwd.to_string_lossy().to_string(),
        path: None,
        source,
        turn_count: 0,
        turns: Vec::new(),
    }
}

#[cfg(all(test, feature = "native-codex-runtime"))]
mod tests {
    use super::native_thread_summary_without_rollout_path;
    use std::path::Path;

    #[test]
    fn native_thread_summary_without_rollout_path_builds_fallback_summary() {
        let summary = native_thread_summary_without_rollout_path(
            "thread-123",
            "openai".to_string(),
            Path::new("workspace"),
            "cli".to_string(),
        );

        assert_eq!(summary.id, "thread-123");
        assert_eq!(summary.codex_thread_id.as_deref(), Some("thread-123"));
        assert_eq!(summary.model_provider, "openai");
        assert_eq!(summary.cwd, "workspace");
        assert_eq!(summary.source, "cli");
        assert_eq!(summary.path, None);
        assert_eq!(summary.turn_count, 0);
        assert!(summary.turns.is_empty());
        assert_eq!(summary.created_at, summary.updated_at);
    }
}
