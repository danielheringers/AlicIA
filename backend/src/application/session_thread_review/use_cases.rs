use tauri::State;

use crate::domain::session_thread_review::{review_policy, thread_query};
use crate::infrastructure::runtime_bridge::session_thread_catalog::{self, ThreadListQuery};
use crate::interface::tauri::dto::{
    CodexReviewStartRequest, CodexThreadListRequest, CodexThreadListResponse,
    CodexThreadReadRequest, CodexThreadReadResponse,
};
use crate::{lock_active_session, AppState};

const DEFAULT_PAGE_SIZE: usize = 25;
const MAX_PAGE_SIZE: usize = 100;

fn requested_page_size(limit: Option<u32>) -> usize {
    limit
        .map(|value| usize::try_from(value).unwrap_or(usize::MAX))
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE)
}

pub(crate) fn validate_review_start_request(
    request: &CodexReviewStartRequest,
) -> Result<(), String> {
    review_policy::validate_review_start(request.target.as_ref(), request.delivery.as_deref())
}

pub(crate) async fn codex_thread_list(
    state: State<'_, AppState>,
    request: CodexThreadListRequest,
) -> Result<CodexThreadListResponse, String> {
    let CodexThreadListRequest {
        cursor,
        limit,
        sort_key,
        model_providers,
        source_kinds,
        archived,
        cwd,
    } = request;

    let (runtime, session_cwd) = {
        let guard = lock_active_session(state.inner())?;
        let active = guard
            .as_ref()
            .ok_or_else(|| "no active codex session".to_string())?;

        let crate::ActiveSessionTransport::Native(native) = &active.transport;

        (std::sync::Arc::clone(&native.runtime), active.cwd.clone())
    };

    let (allowed_sources, source_filter) = thread_query::parse_source_filters(source_kinds);
    let query = ThreadListQuery {
        cursor: thread_query::parse_thread_cursor(cursor)?,
        requested_page_size: requested_page_size(limit),
        sort_key: thread_query::parse_thread_sort_key(sort_key)?,
        allowed_sources,
        source_filter,
        model_provider_filter: thread_query::normalize_model_provider_filters(model_providers),
        archived: archived.unwrap_or(false),
        cwd_filter: cwd
            .map(|entry| entry.trim().to_string())
            .filter(|entry| !entry.is_empty()),
    };

    session_thread_catalog::list_threads(runtime, session_cwd.as_path(), query).await
}

pub(crate) async fn codex_thread_read(
    state: State<'_, AppState>,
    request: CodexThreadReadRequest,
) -> Result<CodexThreadReadResponse, String> {
    let thread_id = request.thread_id.trim().to_string();
    if thread_id.is_empty() {
        return Err("thread_id is required".to_string());
    }

    let include_turns = request.include_turns.unwrap_or(true);
    let (runtime, loaded_thread, session_cwd) = {
        let guard = lock_active_session(state.inner())?;
        let active = guard
            .as_ref()
            .ok_or_else(|| "no active codex session".to_string())?;

        let crate::ActiveSessionTransport::Native(native) = &active.transport;

        (
            std::sync::Arc::clone(&native.runtime),
            native.threads.get(&thread_id).cloned(),
            active.cwd.clone(),
        )
    };

    session_thread_catalog::read_thread(
        runtime,
        loaded_thread,
        session_cwd.as_path(),
        thread_id,
        include_turns,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::requested_page_size;

    #[test]
    fn requested_page_size_defaults_to_twenty_five() {
        assert_eq!(requested_page_size(None), 25);
    }

    #[test]
    fn requested_page_size_clamps_to_minimum_when_limit_is_zero() {
        assert_eq!(requested_page_size(Some(0)), 1);
    }

    #[test]
    fn requested_page_size_clamps_to_maximum_when_limit_is_large() {
        assert_eq!(requested_page_size(Some(999)), 100);
    }
}
