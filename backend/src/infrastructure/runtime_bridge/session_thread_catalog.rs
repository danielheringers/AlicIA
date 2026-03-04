use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use codex_core::CodexThread;
use codex_core::{
    find_thread_path_by_id_str, Cursor as RolloutCursor, RolloutRecorder, ThreadsPage,
};
use codex_protocol::protocol::SessionSource;

use crate::codex_native_runtime::NativeCodexRuntime;
use crate::domain::session_thread_review::thread_query;
use crate::infrastructure::runtime_bridge::session_thread_shared;
use crate::interface::tauri::dto::{CodexThreadListResponse, CodexThreadReadResponse};

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

async fn list_threads_with_loader<F, Fut>(
    mut query: ThreadListQuery,
    fallback_provider: &str,
    mut load_page: F,
) -> Result<CodexThreadListResponse, String>
where
    F: FnMut(
        usize,
        Option<RolloutCursor>,
        codex_core::ThreadSortKey,
        Vec<SessionSource>,
        Option<Vec<String>>,
    ) -> Fut,
    Fut: Future<Output = Result<ThreadsPage, String>>,
{
    let mut cursor_obj = query.cursor.take();
    let mut last_cursor = cursor_obj.clone();
    let mut remaining = query.requested_page_size;
    let mut data = Vec::with_capacity(query.requested_page_size);
    let mut next_cursor = None;

    while remaining > 0 {
        let page_size = remaining.min(MAX_PAGE_SIZE);
        let page = load_page(
            page_size,
            cursor_obj.clone(),
            query.sort_key,
            query.allowed_sources.clone(),
            query.model_provider_filter.clone(),
        )
        .await?;

        for item in page.items {
            let Some(summary) = session_thread_shared::native_thread_summary_from_list_item(
                item,
                fallback_provider,
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

pub(crate) async fn list_threads(
    runtime: Arc<NativeCodexRuntime>,
    session_cwd: &Path,
    query: ThreadListQuery,
) -> Result<CodexThreadListResponse, String> {
    let config =
        session_thread_shared::native_config_builder(runtime.codex_home.clone(), session_cwd)
            .harness_overrides(session_thread_shared::native_profile_harness_overrides(
                session_cwd,
            ))
            .build()
            .await
            .map_err(|error| format!("failed to build native thread list config: {error}"))?;
    let fallback_provider = config.model_provider_id.clone();
    let default_provider = config.model_provider_id.clone();
    let archived = query.archived;

    list_threads_with_loader(
        query,
        fallback_provider.as_str(),
        |page_size, cursor, sort_key, allowed_sources, model_provider_filter| {
            let config = &config;
            let default_provider = default_provider.as_str();

            async move {
                let cursor_ref = cursor.as_ref();
                let model_provider_filter_ref = model_provider_filter.as_deref();

                if archived {
                    RolloutRecorder::list_archived_threads(
                        config,
                        page_size,
                        cursor_ref,
                        sort_key,
                        &allowed_sources,
                        model_provider_filter_ref,
                        default_provider,
                    )
                    .await
                    .map_err(|error| format!("failed to list archived threads: {error}"))
                } else {
                    RolloutRecorder::list_threads(
                        config,
                        page_size,
                        cursor_ref,
                        sort_key,
                        &allowed_sources,
                        model_provider_filter_ref,
                        default_provider,
                    )
                    .await
                    .map_err(|error| format!("failed to list threads: {error}"))
                }
            }
        },
    )
    .await
}

pub(crate) async fn read_thread(
    runtime: Arc<NativeCodexRuntime>,
    loaded_thread: Option<Arc<CodexThread>>,
    session_cwd: &Path,
    thread_id: String,
    include_turns: bool,
) -> Result<CodexThreadReadResponse, String> {
    let config =
        session_thread_shared::native_config_builder(runtime.codex_home.clone(), session_cwd)
            .harness_overrides(session_thread_shared::native_profile_harness_overrides(
                session_cwd,
            ))
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
            thread: session_thread_shared::native_thread_summary_from_rollout_path(
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
        return Ok(CodexThreadReadResponse {
            thread: session_thread_shared::native_thread_summary_without_rollout_path(
                thread_id.as_str(),
                snapshot.model_provider_id,
                snapshot.cwd.as_path(),
                snapshot.session_source.to_string(),
            ),
        });
    }

    Err(format!("thread not found: {thread_id}"))
}

#[cfg(test)]
mod tests {
    use super::{list_threads_with_loader, ThreadListQuery};
    use codex_core::{parse_cursor, ThreadItem, ThreadSortKey, ThreadsPage};
    use codex_protocol::protocol::SessionSource;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn base_query(requested_page_size: usize) -> ThreadListQuery {
        ThreadListQuery {
            cursor: None,
            requested_page_size,
            sort_key: ThreadSortKey::CreatedAt,
            allowed_sources: vec![SessionSource::Cli, SessionSource::VSCode],
            source_filter: None,
            model_provider_filter: None,
            archived: false,
            cwd_filter: None,
        }
    }

    fn parse_test_cursor(token: &str) -> codex_core::Cursor {
        parse_cursor(token).expect("cursor should parse")
    }

    fn make_item(
        path: &str,
        source: SessionSource,
        cwd: &str,
        model_provider: Option<&str>,
        preview: &str,
    ) -> ThreadItem {
        ThreadItem {
            path: PathBuf::from(path),
            thread_id: None,
            first_user_message: Some(preview.to_string()),
            cwd: Some(PathBuf::from(cwd)),
            git_branch: None,
            git_sha: None,
            git_origin_url: None,
            source: Some(source),
            model_provider: model_provider.map(|value| value.to_string()),
            cli_version: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn list_threads_with_loader_paginates_and_stops_on_repeated_cursor() {
        let repeated_cursor =
            parse_test_cursor("2026-01-01T00:00:00Z|11111111-1111-1111-1111-111111111111");
        let page_one = ThreadsPage {
            items: vec![make_item(
                "sessions/thread-123e4567-e89b-12d3-a456-426614174001.jsonl",
                SessionSource::Cli,
                "/workspace",
                Some("openai"),
                "first",
            )],
            next_cursor: Some(repeated_cursor.clone()),
            num_scanned_files: 1,
            reached_scan_cap: false,
        };
        let page_two = ThreadsPage {
            items: vec![make_item(
                "sessions/thread-123e4567-e89b-12d3-a456-426614174002.jsonl",
                SessionSource::Cli,
                "/workspace",
                Some("openai"),
                "second",
            )],
            next_cursor: Some(repeated_cursor),
            num_scanned_files: 1,
            reached_scan_cap: false,
        };

        let mut pages = VecDeque::from([page_one, page_two]);
        let mut expected_page_sizes = VecDeque::from([5usize, 4usize]);
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_for_loader = Arc::clone(&call_count);

        let response = list_threads_with_loader(
            base_query(5),
            "fallback-provider",
            move |page_size, _cursor, sort_key, allowed_sources, model_provider_filter| {
                call_count_for_loader.fetch_add(1, Ordering::Relaxed);
                assert_eq!(sort_key, ThreadSortKey::CreatedAt);
                assert_eq!(
                    allowed_sources,
                    vec![SessionSource::Cli, SessionSource::VSCode]
                );
                assert!(model_provider_filter.is_none());
                assert_eq!(expected_page_sizes.pop_front(), Some(page_size));
                let page = pages.pop_front().expect("page should exist");
                async move { Ok(page) }
            },
        )
        .await
        .expect("list should succeed");

        assert_eq!(call_count.load(Ordering::Relaxed), 2);
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.next_cursor, None);
    }

    #[tokio::test]
    async fn list_threads_with_loader_applies_filters_and_forwards_model_provider_filter() {
        let mut query = base_query(10);
        query.source_filter = Some(vec!["cli".to_string()]);
        query.cwd_filter = Some("/workspace".to_string());
        query.model_provider_filter = Some(vec!["openai".to_string()]);

        let page = ThreadsPage {
            items: vec![
                make_item(
                    "sessions/thread-123e4567-e89b-12d3-a456-426614174010.jsonl",
                    SessionSource::Cli,
                    "/workspace",
                    None,
                    "kept",
                ),
                make_item(
                    "sessions/thread-123e4567-e89b-12d3-a456-426614174011.jsonl",
                    SessionSource::VSCode,
                    "/workspace",
                    Some("openai"),
                    "drop-source",
                ),
                make_item(
                    "sessions/thread-123e4567-e89b-12d3-a456-426614174012.jsonl",
                    SessionSource::Cli,
                    "/other",
                    Some("openai"),
                    "drop-cwd",
                ),
            ],
            next_cursor: None,
            num_scanned_files: 3,
            reached_scan_cap: false,
        };

        let mut pages = VecDeque::from([page]);
        let model_filter_seen = Arc::new(AtomicUsize::new(0));
        let model_filter_seen_for_loader = Arc::clone(&model_filter_seen);

        let response = list_threads_with_loader(
            query,
            "fallback-provider",
            move |_page_size, _cursor, _sort_key, _allowed_sources, model_provider_filter| {
                let normalized = model_provider_filter.unwrap_or_default();
                if normalized == vec!["openai".to_string()] {
                    model_filter_seen_for_loader.fetch_add(1, Ordering::Relaxed);
                }
                let page = pages.pop_front().expect("page should exist");
                async move { Ok(page) }
            },
        )
        .await
        .expect("list should succeed");

        assert_eq!(model_filter_seen.load(Ordering::Relaxed), 1);
        assert_eq!(response.next_cursor, None);
        assert_eq!(response.data.len(), 1);

        let thread = response.data.first().expect("expected one thread");
        assert_eq!(thread.preview, "kept");
        assert_eq!(thread.source, "cli");
        assert_eq!(thread.cwd, "/workspace");
        assert_eq!(thread.model_provider, "fallback-provider");
    }
}
