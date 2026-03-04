#[cfg(feature = "native-codex-runtime")]
use std::sync::atomic::AtomicU64;
#[cfg(feature = "native-codex-runtime")]
use std::sync::Arc;

#[cfg(feature = "native-codex-runtime")]
use codex_core::CodexThread;
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::protocol::EventMsg;
#[cfg(feature = "native-codex-runtime")]
use tauri::{AppHandle, Manager};

#[cfg(feature = "native-codex-runtime")]
use crate::codex_event_translator::NativeCodexEventTranslator;
#[cfg(feature = "native-codex-runtime")]
use crate::{emit_codex_event, emit_lifecycle, lock_active_session, AppState};

#[cfg(feature = "native-codex-runtime")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LifecycleErrorPayload {
    pub status: &'static str,
    pub session_id: Option<u64>,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
}

#[cfg(feature = "native-codex-runtime")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionTurnFinalizationPlan {
    pub discovered_thread_id: Option<String>,
    pub lifecycle_error: Option<LifecycleErrorPayload>,
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn with_native_handles_mut<R>(
    app: &AppHandle,
    session_id: u64,
    f: impl FnOnce(&mut crate::NativeSessionHandles) -> R,
) -> Option<R> {
    let state = app.state::<AppState>();
    let mut guard = lock_active_session(state.inner()).ok()?;
    let active = guard.as_mut()?;
    if active.session_id != session_id {
        return None;
    }
    let crate::ActiveSessionTransport::Native(native) = &mut active.transport;
    Some(f(native))
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn finish_session_turn(
    app: &AppHandle,
    session_id: u64,
    discovered_thread_id: Option<String>,
) {
    let state = app.state::<AppState>();
    let mut guard = match lock_active_session(state.inner()) {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let Some(active) = guard.as_mut() else {
        return;
    };

    if active.session_id != session_id {
        return;
    }

    active.busy = false;
    if let Some(thread_id) = discovered_thread_id {
        active.thread_id = Some(thread_id);
    }
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn is_active_session(app: &AppHandle, session_id: u64) -> bool {
    let state = app.state::<AppState>();
    let guard = match lock_active_session(state.inner()) {
        Ok(guard) => guard,
        Err(_) => return false,
    };

    guard
        .as_ref()
        .is_some_and(|active| active.session_id == session_id)
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn plan_session_turn_finalization(
    session_id: u64,
    pid: Option<u32>,
    result: Result<String, String>,
    is_session_active: bool,
) -> SessionTurnFinalizationPlan {
    match result {
        Ok(returned_thread_id) => SessionTurnFinalizationPlan {
            discovered_thread_id: Some(returned_thread_id),
            lifecycle_error: None,
        },
        Err(error) => SessionTurnFinalizationPlan {
            discovered_thread_id: None,
            lifecycle_error: is_session_active.then_some(LifecycleErrorPayload {
                status: "error",
                session_id: Some(session_id),
                pid,
                exit_code: None,
                message: Some(error),
            }),
        },
    }
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn finalize_session_turn_pipeline(
    app: &AppHandle,
    session_id: u64,
    pid: Option<u32>,
    result: Result<String, String>,
) {
    let finalization =
        plan_session_turn_finalization(session_id, pid, result, is_active_session(app, session_id));

    if let Some(payload) = finalization.lifecycle_error {
        emit_lifecycle(
            app,
            payload.status,
            payload.session_id,
            payload.pid,
            payload.exit_code,
            payload.message,
        );
    }

    finish_session_turn(app, session_id, finalization.discovered_thread_id);
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) async fn drive_session_turn_event_pipeline(
    app: &AppHandle,
    session_id: u64,
    thread_id: &str,
    thread: &Arc<CodexThread>,
    event_seq: &Arc<AtomicU64>,
) -> Result<(), String> {
    let mut translator = NativeCodexEventTranslator::new(thread_id.to_string());
    loop {
        let event = thread
            .next_event()
            .await
            .map_err(|error| format!("native event stream failed: {error}"))?;
        let is_terminal = matches!(
            event.msg,
            EventMsg::TurnComplete(_) | EventMsg::TurnAborted(_)
        );

        let Some(translated_events) = with_native_handles_mut(app, session_id, |native| {
            translator.translate_event(&event, native)
        }) else {
            break;
        };

        for translated in translated_events {
            emit_codex_event(app, session_id, translated, event_seq);
        }

        if is_terminal {
            break;
        }
    }

    Ok(())
}

#[cfg(all(test, feature = "native-codex-runtime"))]
mod tests {
    use super::{
        plan_session_turn_finalization, LifecycleErrorPayload, SessionTurnFinalizationPlan,
    };

    #[test]
    fn lifecycle_error_payload_emits_only_when_session_is_active() {
        let inactive_plan =
            plan_session_turn_finalization(41, Some(902), Err("stream failed".to_string()), false);
        assert_eq!(
            inactive_plan,
            SessionTurnFinalizationPlan {
                discovered_thread_id: None,
                lifecycle_error: None,
            }
        );

        let active_plan =
            plan_session_turn_finalization(41, Some(902), Err("stream failed".to_string()), true);
        assert_eq!(
            active_plan,
            SessionTurnFinalizationPlan {
                discovered_thread_id: None,
                lifecycle_error: Some(LifecycleErrorPayload {
                    status: "error",
                    session_id: Some(41),
                    pid: Some(902),
                    exit_code: None,
                    message: Some("stream failed".to_string()),
                }),
            }
        );
    }

    #[test]
    fn common_finalization_plan_preserves_success_and_error_shapes() {
        let success_plan =
            plan_session_turn_finalization(9, Some(77), Ok("thread-success".to_string()), true);
        assert_eq!(
            success_plan,
            SessionTurnFinalizationPlan {
                discovered_thread_id: Some("thread-success".to_string()),
                lifecycle_error: None,
            }
        );

        let error_plan =
            plan_session_turn_finalization(9, None, Err("submit failed".to_string()), true);
        assert_eq!(
            error_plan,
            SessionTurnFinalizationPlan {
                discovered_thread_id: None,
                lifecycle_error: Some(LifecycleErrorPayload {
                    status: "error",
                    session_id: Some(9),
                    pid: None,
                    exit_code: None,
                    message: Some("submit failed".to_string()),
                }),
            }
        );
    }
}
