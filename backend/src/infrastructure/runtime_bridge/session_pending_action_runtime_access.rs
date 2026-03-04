#[cfg(feature = "native-codex-runtime")]
use std::collections::HashMap;
#[cfg(feature = "native-codex-runtime")]
use std::sync::Arc;

#[cfg(feature = "native-codex-runtime")]
use codex_core::CodexThread;
#[cfg(feature = "native-codex-runtime")]
use tauri::{AppHandle, State};

#[cfg(feature = "native-codex-runtime")]
use crate::infrastructure::runtime_bridge::session_turn_event_pipeline;
#[cfg(feature = "native-codex-runtime")]
use crate::{lock_active_session, AppState};

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn reinsert_pending_approval_entry(
    pending_approvals: &mut HashMap<String, crate::NativePendingApproval>,
    action_id: &str,
    pending_approval: crate::NativePendingApproval,
) {
    pending_approvals.insert(action_id.to_string(), pending_approval);
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn reinsert_pending_user_input_entry(
    pending_user_inputs: &mut HashMap<String, crate::NativePendingUserInput>,
    action_id: &str,
    pending_user_input: crate::NativePendingUserInput,
) {
    pending_user_inputs.insert(action_id.to_string(), pending_user_input);
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn reinsert_pending_approval_for_session(
    app: &AppHandle,
    session_id: u64,
    action_id: &str,
    pending_approval: crate::NativePendingApproval,
) {
    let _ = session_turn_event_pipeline::with_native_handles_mut(app, session_id, |native| {
        reinsert_pending_approval_entry(&mut native.pending_approvals, action_id, pending_approval)
    });
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn reinsert_pending_user_input_for_session(
    app: &AppHandle,
    session_id: u64,
    action_id: &str,
    pending_user_input: crate::NativePendingUserInput,
) {
    let _ = session_turn_event_pipeline::with_native_handles_mut(app, session_id, |native| {
        reinsert_pending_user_input_entry(
            &mut native.pending_user_inputs,
            action_id,
            pending_user_input,
        )
    });
}

#[cfg(feature = "native-codex-runtime")]
fn remove_pending_approval_from_maps<T: Clone>(
    pending_approvals: &mut HashMap<String, crate::NativePendingApproval>,
    threads: &HashMap<String, T>,
    action_id: &str,
) -> Result<(T, crate::NativePendingApproval), String> {
    let pending_approval = pending_approvals
        .remove(action_id)
        .ok_or_else(|| format!("approval action not found: {action_id}"))?;

    let thread = threads.get(&pending_approval.thread_id).cloned();
    let Some(thread) = thread else {
        reinsert_pending_approval_entry(pending_approvals, action_id, pending_approval.clone());
        return Err(format!("thread not found: {}", pending_approval.thread_id));
    };

    Ok((thread, pending_approval))
}

#[cfg(feature = "native-codex-runtime")]
fn remove_pending_user_input_from_maps<T: Clone>(
    pending_user_inputs: &mut HashMap<String, crate::NativePendingUserInput>,
    threads: &HashMap<String, T>,
    action_id: &str,
) -> Result<(T, crate::NativePendingUserInput), String> {
    let pending_user_input = pending_user_inputs
        .remove(action_id)
        .ok_or_else(|| format!("user input action not found: {action_id}"))?;

    let thread = threads.get(&pending_user_input.thread_id).cloned();
    let Some(thread) = thread else {
        reinsert_pending_user_input_entry(
            pending_user_inputs,
            action_id,
            pending_user_input.clone(),
        );
        return Err(format!(
            "thread not found: {}",
            pending_user_input.thread_id
        ));
    };

    Ok((thread, pending_user_input))
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn remove_pending_approval_and_lookup_thread(
    state: &State<'_, AppState>,
    action_id: &str,
) -> Result<(u64, Arc<CodexThread>, crate::NativePendingApproval), String> {
    let mut guard = lock_active_session(state.inner())?;
    let active = guard
        .as_mut()
        .ok_or_else(|| "no active codex session".to_string())?;

    let session_id = active.session_id;
    let crate::ActiveSessionTransport::Native(native) = &mut active.transport;

    let (thread, pending_approval) = remove_pending_approval_from_maps(
        &mut native.pending_approvals,
        &native.threads,
        action_id,
    )?;

    Ok((session_id, thread, pending_approval))
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn remove_pending_user_input_and_lookup_thread(
    state: &State<'_, AppState>,
    action_id: &str,
) -> Result<(u64, Arc<CodexThread>, crate::NativePendingUserInput), String> {
    let mut guard = lock_active_session(state.inner())?;
    let active = guard
        .as_mut()
        .ok_or_else(|| "no active codex session".to_string())?;

    let session_id = active.session_id;
    let crate::ActiveSessionTransport::Native(native) = &mut active.transport;

    let (thread, pending_user_input) = remove_pending_user_input_from_maps(
        &mut native.pending_user_inputs,
        &native.threads,
        action_id,
    )?;

    Ok((session_id, thread, pending_user_input))
}

#[cfg(all(test, feature = "native-codex-runtime"))]
mod tests {
    use super::{
        remove_pending_approval_and_lookup_thread, remove_pending_approval_from_maps,
        remove_pending_user_input_and_lookup_thread, remove_pending_user_input_from_maps,
    };
    use crate::{AppState, NativeApprovalKind, NativePendingApproval, NativePendingUserInput};
    use std::collections::HashMap;
    use tauri::Manager;

    fn pending_approval(thread_id: &str) -> NativePendingApproval {
        NativePendingApproval {
            thread_id: thread_id.to_string(),
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            kind: NativeApprovalKind::CommandExecution,
        }
    }

    fn pending_user_input(thread_id: &str) -> NativePendingUserInput {
        NativePendingUserInput {
            thread_id: thread_id.to_string(),
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
        }
    }

    #[test]
    fn remove_pending_approval_from_maps_returns_thread_and_entry() {
        let mut pending = HashMap::new();
        pending.insert("approval-1".to_string(), pending_approval("thread-1"));

        let mut threads = HashMap::new();
        threads.insert("thread-1".to_string(), "thread-handle".to_string());

        let (thread, removed) =
            remove_pending_approval_from_maps(&mut pending, &threads, "approval-1")
                .expect("remove should succeed");

        assert_eq!(thread, "thread-handle");
        assert_eq!(removed.thread_id, "thread-1");
        assert!(!pending.contains_key("approval-1"));
    }

    #[test]
    fn remove_pending_approval_from_maps_reinserts_when_thread_missing() {
        let mut pending = HashMap::new();
        pending.insert("approval-1".to_string(), pending_approval("thread-missing"));

        let threads: HashMap<String, String> = HashMap::new();

        let err = remove_pending_approval_from_maps(&mut pending, &threads, "approval-1")
            .expect_err("missing thread should fail");

        assert_eq!(err, "thread not found: thread-missing");
        assert!(pending.contains_key("approval-1"));
    }

    #[test]
    fn remove_pending_approval_from_maps_returns_error_for_missing_action() {
        let mut pending: HashMap<String, NativePendingApproval> = HashMap::new();
        let threads: HashMap<String, String> = HashMap::new();

        let err = remove_pending_approval_from_maps(&mut pending, &threads, "approval-404")
            .expect_err("missing action should fail");

        assert_eq!(err, "approval action not found: approval-404");
    }

    #[test]
    fn remove_pending_user_input_from_maps_returns_thread_and_entry() {
        let mut pending = HashMap::new();
        pending.insert("user-input-1".to_string(), pending_user_input("thread-1"));

        let mut threads = HashMap::new();
        threads.insert("thread-1".to_string(), "thread-handle".to_string());

        let (thread, removed) =
            remove_pending_user_input_from_maps(&mut pending, &threads, "user-input-1")
                .expect("remove should succeed");

        assert_eq!(thread, "thread-handle");
        assert_eq!(removed.thread_id, "thread-1");
        assert!(!pending.contains_key("user-input-1"));
    }

    #[test]
    fn remove_pending_user_input_from_maps_reinserts_when_thread_missing() {
        let mut pending = HashMap::new();
        pending.insert(
            "user-input-1".to_string(),
            pending_user_input("thread-missing"),
        );

        let threads: HashMap<String, String> = HashMap::new();

        let err = remove_pending_user_input_from_maps(&mut pending, &threads, "user-input-1")
            .expect_err("missing thread should fail");

        assert_eq!(err, "thread not found: thread-missing");
        assert!(pending.contains_key("user-input-1"));
    }

    #[test]
    fn remove_pending_user_input_from_maps_returns_error_for_missing_action() {
        let mut pending: HashMap<String, NativePendingUserInput> = HashMap::new();
        let threads: HashMap<String, String> = HashMap::new();

        let err = remove_pending_user_input_from_maps(&mut pending, &threads, "user-input-404")
            .expect_err("missing action should fail");

        assert_eq!(err, "user input action not found: user-input-404");
    }

    #[test]
    fn remove_pending_approval_and_lookup_thread_errors_when_session_missing() {
        let app = tauri::Builder::default()
            .any_thread()
            .manage(AppState::default())
            .build(tauri::generate_context!())
            .expect("test app should build");

        let state = app.handle().state::<AppState>();
        let err = match remove_pending_approval_and_lookup_thread(&state, "approval-1") {
            Ok(_) => panic!("missing session should fail"),
            Err(err) => err,
        };

        assert_eq!(err, "no active codex session");
    }

    #[test]
    fn remove_pending_user_input_and_lookup_thread_errors_when_session_missing() {
        let app = tauri::Builder::default()
            .any_thread()
            .manage(AppState::default())
            .build(tauri::generate_context!())
            .expect("test app should build");

        let state = app.handle().state::<AppState>();
        let err = match remove_pending_user_input_and_lookup_thread(&state, "user-input-1") {
            Ok(_) => panic!("missing session should fail"),
            Err(err) => err,
        };

        assert_eq!(err, "no active codex session");
    }
}
