use std::path::Path;
use std::sync::Arc;

use codex_core::CodexThread;
use codex_core::{find_thread_path_by_id_str, Cursor as RolloutCursor, RolloutRecorder};
use codex_protocol::protocol::SessionSource;

use crate::codex_native_runtime::NativeCodexRuntime;
use crate::domain::session_thread_review::thread_query;
use crate::interface::tauri::dto::{
    CodexThreadListResponse, CodexThreadReadResponse, CodexThreadSummary,
};

const MAX_PAGE_SIZE: usize = 100;

pub(crate) struct ThreadListQuery {
    pub cursor: Option<RolloutCursor>,
    pub requested_page_size: usize,
    pub sort_key: codex_core::ThreadSortKey,
    pub allowed_sources: Vec<SessionSource>,
    pub source_filter: Option<Vec<String>>,
    pub model_provider_filter: Option<Vec<String>>,
    pub archived: bool,
    pub cwd_filter: Option<String>,
}

pub(crate) async fn list_threads(
    runtime: Arc<NativeCodexRuntime>,
    session_cwd: &Path,
    mut query: ThreadListQuery,
) -> Result<CodexThreadListResponse, String> {
    let config =
        crate::session_turn_runtime::native_config_builder(runtime.codex_home.clone(), session_cwd)
            .harness_overrides(
                crate::session_turn_runtime::native_profile_harness_overrides(session_cwd),
            )
            .build()
            .await
            .map_err(|error| format!("failed to build native thread list config: {error}"))?;
    let fallback_provider = config.model_provider_id.clone();
    let default_provider = config.model_provider_id.as_str();

    let mut cursor_obj = query.cursor.take();
    let mut last_cursor = cursor_obj.clone();
    let mut remaining = query.requested_page_size;
    let mut data = Vec::with_capacity(query.requested_page_size);
    let mut next_cursor = None;

    while remaining > 0 {
        let page_size = remaining.min(MAX_PAGE_SIZE);
        let page = if query.archived {
            RolloutRecorder::list_archived_threads(
                &config,
                page_size,
                cursor_obj.as_ref(),
                query.sort_key,
                &query.allowed_sources,
                query.model_provider_filter.as_deref(),
                default_provider,
            )
            .await
            .map_err(|error| format!("failed to list archived threads: {error}"))?
        } else {
            RolloutRecorder::list_threads(
                &config,
                page_size,
                cursor_obj.as_ref(),
                query.sort_key,
                &query.allowed_sources,
                query.model_provider_filter.as_deref(),
                default_provider,
            )
            .await
            .map_err(|error| format!("failed to list threads: {error}"))?
        };

        for item in page.items {
            let Some(summary) = crate::session_turn_runtime::native_thread_summary_from_list_item(
                item,
                &fallback_provider,
            ) else {
                continue;
            };
            if query.source_filter.as_ref().is_some_and(|filter| {
                !thread_query::source_kind_matches_filter(&summary.source, filter)
            }) {
                continue;
            }
            if query
                .cwd_filter
                .as_ref()
                .is_some_and(|expected_cwd| summary.cwd != *expected_cwd)
            {
                continue;
            }
            data.push(summary);
            if data.len() == query.requested_page_size {
                break;
            }
        }

        remaining = query.requested_page_size.saturating_sub(data.len());
        let next_cursor_value = page.next_cursor;
        next_cursor = thread_query::serialize_thread_cursor(next_cursor_value.clone());

        if remaining == 0 {
            break;
        }

        match next_cursor_value {
            Some(cursor_value) => {
                if last_cursor.as_ref() == Some(&cursor_value) {
                    next_cursor = None;
                    break;
                }
                last_cursor = Some(cursor_value.clone());
                cursor_obj = Some(cursor_value);
            }
            None => break,
        }
    }

    Ok(CodexThreadListResponse { data, next_cursor })
}

pub(crate) async fn read_thread(
    runtime: Arc<NativeCodexRuntime>,
    loaded_thread: Option<Arc<CodexThread>>,
    session_cwd: &Path,
    thread_id: String,
    include_turns: bool,
) -> Result<CodexThreadReadResponse, String> {
    let config =
        crate::session_turn_runtime::native_config_builder(runtime.codex_home.clone(), session_cwd)
            .harness_overrides(
                crate::session_turn_runtime::native_profile_harness_overrides(session_cwd),
            )
            .build()
            .await
            .map_err(|error| format!("failed to build native thread read config: {error}"))?;
    let fallback_provider = config.model_provider_id.clone();

    let rollout_path = if let Some(path) = loaded_thread
        .as_ref()
        .and_then(|thread| thread.rollout_path())
    {
        Some(path)
    } else {
        find_thread_path_by_id_str(runtime.codex_home.as_path(), &thread_id)
            .await
            .map_err(|error| format!("failed to resolve thread path: {error}"))?
    };

    if let Some(rollout_path) = rollout_path {
        return Ok(CodexThreadReadResponse {
            thread: crate::session_turn_runtime::native_thread_summary_from_rollout_path(
                rollout_path.as_path(),
                fallback_provider.as_str(),
                include_turns,
                Some(thread_id.as_str()),
            )
            .await?,
        });
    }

    if let Some(thread) = loaded_thread {
        let snapshot = thread.config_snapshot().await;
        let now = crate::session_turn_runtime::native_now_epoch_seconds();
        return Ok(CodexThreadReadResponse {
            thread: CodexThreadSummary {
                id: thread_id.clone(),
                codex_thread_id: Some(thread_id.clone()),
                preview: String::new(),
                model_provider: snapshot.model_provider_id,
                created_at: now,
                updated_at: now,
                cwd: snapshot.cwd.to_string_lossy().to_string(),
                path: None,
                source: snapshot.session_source.to_string(),
                turn_count: 0,
                turns: Vec::new(),
            },
        });
    }

    Err(format!("thread not found: {thread_id}"))
}
