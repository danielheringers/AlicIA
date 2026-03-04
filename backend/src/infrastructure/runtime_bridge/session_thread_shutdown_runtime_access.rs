#[cfg(feature = "native-codex-runtime")]
use std::path::Path;
#[cfg(feature = "native-codex-runtime")]
use std::sync::Arc;

#[cfg(feature = "native-codex-runtime")]
use codex_core::CodexThread;
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::protocol::Op;
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::ThreadId;

#[cfg(feature = "native-codex-runtime")]
use crate::codex_native_runtime::NativeCodexRuntime;
#[cfg(feature = "native-codex-runtime")]
use crate::infrastructure::runtime_bridge::session_thread_shared;

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn resolve_removable_thread_id(
    thread_id: &str,
    fallback_rollout_path: Option<&Path>,
) -> Option<ThreadId> {
    ThreadId::from_string(thread_id).ok().or_else(|| {
        fallback_rollout_path
            .and_then(session_thread_shared::infer_thread_id_from_rollout_path)
            .and_then(|value| ThreadId::from_string(&value).ok())
    })
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn remove_and_shutdown_thread_best_effort(
    runtime: &Arc<NativeCodexRuntime>,
    removable_thread_id: Option<ThreadId>,
    fallback_thread: Option<Arc<CodexThread>>,
) -> Option<Arc<CodexThread>> {
    let mut removed_thread = fallback_thread;
    if let Some(removable_thread_id) = removable_thread_id {
        if let Some(thread) = runtime
            .thread_manager
            .remove_thread(&removable_thread_id)
            .await
        {
            removed_thread = Some(thread);
        }
    }

    if let Some(thread) = removed_thread.as_ref() {
        let _ = thread.submit(Op::Shutdown).await;
    }

    removed_thread
}

#[cfg(all(test, feature = "native-codex-runtime"))]
mod tests {
    use super::resolve_removable_thread_id;
    use std::path::Path;

    #[test]
    fn resolve_removable_thread_id_uses_direct_uuid_when_available() {
        let thread_id = "123e4567-e89b-12d3-a456-426614174000";

        let resolved = resolve_removable_thread_id(thread_id, None);

        assert_eq!(
            resolved.as_ref().map(ToString::to_string).as_deref(),
            Some(thread_id)
        );
    }

    #[test]
    fn resolve_removable_thread_id_falls_back_to_rollout_path_when_needed() {
        let fallback_path =
            Path::new("sessions/2026/03/04/rollout-67e55044-10b1-426f-9247-bb680e5fe0c8.jsonl");

        let resolved = resolve_removable_thread_id("thread-local-alias", Some(fallback_path));

        assert_eq!(
            resolved.as_ref().map(ToString::to_string).as_deref(),
            Some("67e55044-10b1-426f-9247-bb680e5fe0c8")
        );
    }

    #[test]
    fn resolve_removable_thread_id_returns_none_without_valid_id_or_fallback() {
        let resolved = resolve_removable_thread_id("thread-local-alias", None);

        assert!(resolved.is_none());
    }
}
