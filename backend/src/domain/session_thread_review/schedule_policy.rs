#[cfg(feature = "native-codex-runtime")]
pub(crate) fn normalize_scheduled_thread_id(thread_id: Option<String>) -> Option<String> {
    thread_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn resolve_scheduled_thread_id(
    requested_thread_id: Option<String>,
    fallback_thread_id: Option<String>,
) -> Option<String> {
    normalize_scheduled_thread_id(requested_thread_id.or(fallback_thread_id))
}

#[cfg(all(test, feature = "native-codex-runtime"))]
mod tests {
    use super::{normalize_scheduled_thread_id, resolve_scheduled_thread_id};

    #[test]
    fn normalize_scheduled_thread_id_trims_and_discards_empty_values() {
        assert_eq!(normalize_scheduled_thread_id(None), None);
        assert_eq!(normalize_scheduled_thread_id(Some("   ".to_string())), None);
        assert_eq!(
            normalize_scheduled_thread_id(Some(" thread-7 ".to_string())),
            Some("thread-7".to_string())
        );
    }

    #[test]
    fn resolve_scheduled_thread_id_prefers_requested_value() {
        let resolved = resolve_scheduled_thread_id(
            Some(" requested-thread ".to_string()),
            Some("fallback-thread".to_string()),
        );

        assert_eq!(resolved, Some("requested-thread".to_string()));
    }

    #[test]
    fn resolve_scheduled_thread_id_uses_fallback_only_when_requested_is_none() {
        assert_eq!(
            resolve_scheduled_thread_id(None, Some(" fallback-thread ".to_string())),
            Some("fallback-thread".to_string())
        );
    }

    #[test]
    fn resolve_scheduled_thread_id_keeps_legacy_behavior_for_blank_requested() {
        assert_eq!(
            resolve_scheduled_thread_id(
                Some("   ".to_string()),
                Some("fallback-thread".to_string())
            ),
            None
        );
    }
}
