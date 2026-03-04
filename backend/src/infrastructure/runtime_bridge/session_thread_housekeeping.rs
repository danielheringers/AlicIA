#[cfg(feature = "native-codex-runtime")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "native-codex-runtime")]
use std::sync::Arc;

#[cfg(feature = "native-codex-runtime")]
use codex_core::CodexThread;

#[cfg(feature = "native-codex-runtime")]
fn remove_thread_and_aliases_from_cache<T>(
    threads: &mut HashMap<String, Arc<T>>,
    active_turns: &mut HashMap<String, String>,
    active_thread_id: &mut Option<String>,
    thread_id: &str,
) -> (Option<Arc<T>>, Vec<String>) {
    let removed_from_cache = threads.remove(thread_id);
    let mut removed_thread_ids = vec![thread_id.to_string()];
    active_turns.remove(thread_id);

    if let Some(removed_thread) = removed_from_cache.as_ref() {
        let alias_ids = threads
            .iter()
            .filter_map(|(candidate_id, candidate_thread)| {
                Arc::ptr_eq(candidate_thread, removed_thread).then_some(candidate_id.clone())
            })
            .collect::<Vec<_>>();
        for alias_id in alias_ids {
            threads.remove(alias_id.as_str());
            active_turns.remove(alias_id.as_str());
            removed_thread_ids.push(alias_id);
        }
    }

    let removed_cache_entry = removed_from_cache.is_some();
    if active_thread_id.as_deref().is_some_and(|active_id| {
        active_id == thread_id || (removed_cache_entry && !threads.contains_key(active_id))
    }) {
        *active_thread_id = None;
    }

    (removed_from_cache, removed_thread_ids)
}

#[cfg(feature = "native-codex-runtime")]
fn clear_pending_actions_for_threads<T>(
    pending_entries: &mut HashMap<String, T>,
    removed_thread_ids: &[String],
    thread_id_for_entry: impl Fn(&T) -> &str,
) {
    if removed_thread_ids.is_empty() {
        return;
    }

    let removed_thread_id_set = removed_thread_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();

    pending_entries
        .retain(|_, pending| !removed_thread_id_set.contains(thread_id_for_entry(pending)));
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn apply_native_thread_close_archive_housekeeping(
    native: &mut crate::NativeSessionHandles,
    active_thread_id: &mut Option<String>,
    thread_id: &str,
) -> Option<Arc<CodexThread>> {
    let (removed_from_cache, removed_thread_ids) = remove_thread_and_aliases_from_cache(
        &mut native.threads,
        &mut native.active_turns,
        active_thread_id,
        thread_id,
    );

    clear_pending_actions_for_threads(
        &mut native.pending_approvals,
        &removed_thread_ids,
        |pending| pending.thread_id.as_str(),
    );
    clear_pending_actions_for_threads(
        &mut native.pending_user_inputs,
        &removed_thread_ids,
        |pending| pending.thread_id.as_str(),
    );

    removed_from_cache
}

#[cfg(all(test, feature = "native-codex-runtime"))]
mod tests {
    use super::{clear_pending_actions_for_threads, remove_thread_and_aliases_from_cache};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    struct FakePendingAction {
        thread_id: String,
    }

    #[test]
    fn remove_thread_and_aliases_filters_aliases_with_same_thread_pointer() {
        let primary_thread = Arc::new(11_u8);
        let retained_thread = Arc::new(22_u8);

        let mut threads = HashMap::from([
            ("thread-main".to_string(), Arc::clone(&primary_thread)),
            ("thread-main-alias".to_string(), Arc::clone(&primary_thread)),
            ("thread-keep".to_string(), Arc::clone(&retained_thread)),
        ]);
        let mut active_turns = HashMap::from([
            ("thread-main".to_string(), "turn-main".to_string()),
            ("thread-main-alias".to_string(), "turn-alias".to_string()),
            ("thread-keep".to_string(), "turn-keep".to_string()),
        ]);
        let mut active_thread_id = Some("thread-main-alias".to_string());

        let (removed_from_cache, removed_thread_ids) = remove_thread_and_aliases_from_cache(
            &mut threads,
            &mut active_turns,
            &mut active_thread_id,
            "thread-main",
        );

        let removed = removed_from_cache.expect("expected target thread to be removed");
        assert!(Arc::ptr_eq(&removed, &primary_thread));
        assert_eq!(removed_thread_ids.len(), 2);
        assert!(removed_thread_ids.contains(&"thread-main".to_string()));
        assert!(removed_thread_ids.contains(&"thread-main-alias".to_string()));
        assert!(!threads.contains_key("thread-main"));
        assert!(!threads.contains_key("thread-main-alias"));
        assert!(threads.contains_key("thread-keep"));
        assert!(!active_turns.contains_key("thread-main"));
        assert!(!active_turns.contains_key("thread-main-alias"));
        assert!(active_turns.contains_key("thread-keep"));
    }

    #[test]
    fn clear_pending_actions_only_removes_entries_for_removed_thread_ids() {
        let mut approvals = HashMap::from([
            (
                "approval-1".to_string(),
                FakePendingAction {
                    thread_id: "thread-main".to_string(),
                },
            ),
            (
                "approval-2".to_string(),
                FakePendingAction {
                    thread_id: "thread-main-alias".to_string(),
                },
            ),
            (
                "approval-3".to_string(),
                FakePendingAction {
                    thread_id: "thread-keep".to_string(),
                },
            ),
        ]);
        let removed_thread_ids = vec!["thread-main".to_string(), "thread-main-alias".to_string()];

        clear_pending_actions_for_threads(&mut approvals, &removed_thread_ids, |pending| {
            pending.thread_id.as_str()
        });

        assert_eq!(approvals.len(), 1);
        assert!(approvals.contains_key("approval-3"));
    }

    #[test]
    fn remove_thread_and_aliases_preserves_active_thread_reset_rule() {
        let retained_thread = Arc::new(5_u8);

        let mut threads =
            HashMap::from([("thread-keep".to_string(), Arc::clone(&retained_thread))]);
        let mut active_turns = HashMap::new();

        let mut active_thread_id = Some("thread-missing".to_string());
        let (removed, removed_thread_ids) = remove_thread_and_aliases_from_cache(
            &mut threads,
            &mut active_turns,
            &mut active_thread_id,
            "thread-missing",
        );
        assert!(removed.is_none());
        assert_eq!(removed_thread_ids, vec!["thread-missing".to_string()]);
        assert_eq!(active_thread_id, None);

        let mut active_thread_id = Some("thread-keep".to_string());
        let (removed, _) = remove_thread_and_aliases_from_cache(
            &mut threads,
            &mut active_turns,
            &mut active_thread_id,
            "thread-other",
        );
        assert!(removed.is_none());
        assert_eq!(active_thread_id.as_deref(), Some("thread-keep"));
    }
}
