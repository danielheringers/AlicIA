#[cfg(feature = "native-codex-runtime")]
use codex_core::{
    parse_cursor, Cursor as RolloutCursor, ThreadSortKey as CoreThreadSortKey,
    INTERACTIVE_SESSION_SOURCES,
};
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::protocol::SessionSource;

#[cfg(feature = "native-codex-runtime")]
fn normalize_source_kind_key(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .collect::<String>()
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn parse_source_filters(
    source_kinds: Option<Vec<String>>,
) -> (Vec<SessionSource>, Option<Vec<String>>) {
    fn default_native_sources() -> Vec<SessionSource> {
        let mut sources = INTERACTIVE_SESSION_SOURCES.to_vec();
        if !sources
            .iter()
            .any(|source| matches!(source, SessionSource::Unknown))
        {
            sources.push(SessionSource::Unknown);
        }
        sources
    }

    let Some(source_kinds) = source_kinds else {
        return (default_native_sources(), None);
    };

    let normalized = source_kinds
        .into_iter()
        .map(|entry| normalize_source_kind_key(&entry))
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        return (default_native_sources(), None);
    }

    let requires_post_filter = normalized
        .iter()
        .any(|kind| !matches!(kind.as_str(), "cli" | "vscode"));

    if requires_post_filter {
        (Vec::new(), Some(normalized))
    } else {
        let allowed_sources = normalized
            .iter()
            .filter_map(|kind| match kind.as_str() {
                "cli" => Some(SessionSource::Cli),
                "vscode" => Some(SessionSource::VSCode),
                _ => None,
            })
            .collect::<Vec<_>>();
        (allowed_sources, Some(normalized))
    }
}

pub(crate) fn normalize_model_provider_filters(
    model_providers: Option<Vec<String>>,
) -> Option<Vec<String>> {
    let entries = model_providers?;

    let normalized = entries
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn source_kind_matches_filter(source: &str, source_filters: &[String]) -> bool {
    let source_key = normalize_source_kind_key(source);
    source_filters.iter().any(|kind| match kind.as_str() {
        "cli" => source_key == "cli",
        "vscode" => source_key == "vscode",
        "exec" => source_key == "exec",
        "appserver" | "mcp" => source_key == "mcp",
        "subagent" => source_key.starts_with("subagent"),
        "subagentreview" => source_key == "subagentreview",
        "subagentcompact" => source_key == "subagentcompact",
        "subagentthreadspawn" => source_key.starts_with("subagentthreadspawn"),
        "subagentother" => {
            source_key.starts_with("subagent")
                && source_key != "subagentreview"
                && source_key != "subagentcompact"
                && !source_key.starts_with("subagentthreadspawn")
        }
        "unknown" => source_key == "unknown",
        _ => false,
    })
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn parse_thread_sort_key(sort_key: Option<String>) -> Result<CoreThreadSortKey, String> {
    let Some(raw) = sort_key else {
        return Ok(CoreThreadSortKey::CreatedAt);
    };
    let normalized = normalize_source_kind_key(&raw);
    if normalized.is_empty() || normalized == "createdat" {
        return Ok(CoreThreadSortKey::CreatedAt);
    }
    if normalized == "updatedat" {
        return Ok(CoreThreadSortKey::UpdatedAt);
    }

    Err("sort_key must be one of: created_at, updated_at".to_string())
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn parse_thread_cursor(cursor: Option<String>) -> Result<Option<RolloutCursor>, String> {
    let Some(cursor) = cursor
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    parse_cursor(&cursor)
        .map(Some)
        .ok_or_else(|| format!("invalid cursor: {cursor}"))
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn serialize_thread_cursor(cursor: Option<RolloutCursor>) -> Option<String> {
    cursor.and_then(|value| {
        serde_json::to_value(value)
            .ok()
            .and_then(|value| value.as_str().map(|entry| entry.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::normalize_model_provider_filters;
    #[cfg(feature = "native-codex-runtime")]
    use super::parse_source_filters;
    #[cfg(feature = "native-codex-runtime")]
    use codex_protocol::protocol::SessionSource;

    #[test]
    fn normalize_model_provider_filters_none_keeps_filter_disabled() {
        let result = normalize_model_provider_filters(None);
        assert_eq!(result, None);
    }

    #[test]
    fn normalize_model_provider_filters_discards_empty_entries() {
        let result =
            normalize_model_provider_filters(Some(vec!["  ".to_string(), "\n".to_string()]));
        assert_eq!(result, None);
    }

    #[test]
    fn normalize_model_provider_filters_keeps_non_empty_entries() {
        let result = normalize_model_provider_filters(Some(vec![
            " openai ".to_string(),
            "anthropic".to_string(),
        ]));
        assert_eq!(
            result,
            Some(vec!["openai".to_string(), "anthropic".to_string()])
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn parse_source_filters_default_keeps_unknown_compatibility() {
        let (allowed, post_filter) = parse_source_filters(None);
        assert!(allowed.contains(&SessionSource::Cli));
        assert!(allowed.contains(&SessionSource::VSCode));
        assert!(allowed.contains(&SessionSource::Unknown));
        assert_eq!(post_filter, None);
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn parse_source_filters_maps_unknown_with_post_filter() {
        let (allowed, post_filter) = parse_source_filters(Some(vec!["unknown".to_string()]));
        assert!(allowed.is_empty());
        assert_eq!(post_filter, Some(vec!["unknown".to_string()]));
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn parse_source_filters_unknown_and_vscode_keep_post_filter() {
        let (allowed, post_filter) =
            parse_source_filters(Some(vec!["unknown".to_string(), "vscode".to_string()]));
        assert!(allowed.is_empty());
        assert_eq!(
            post_filter,
            Some(vec!["unknown".to_string(), "vscode".to_string()])
        );
    }
}
