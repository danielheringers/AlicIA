use tauri::State;

#[cfg(feature = "native-codex-runtime")]
use codex_protocol::protocol::{
    ExecPolicyAmendment, Op, ReviewDecision, ReviewRequest, ReviewTarget,
};
#[cfg(feature = "native-codex-runtime")]
use codex_protocol::request_user_input::{RequestUserInputAnswer, RequestUserInputResponse};
#[cfg(feature = "native-codex-runtime")]
use serde_json::{json, Value};
#[cfg(feature = "native-codex-runtime")]
use std::collections::HashMap;
#[cfg(feature = "native-codex-runtime")]
use std::path::PathBuf;

use crate::domain::session_thread_review::{
    interaction_policy, review_policy, schedule_policy, thread_query,
};
use crate::infrastructure::runtime_bridge::session_thread_catalog::{self, ThreadListQuery};
use crate::interface::tauri::dto::{
    CodexReviewStartRequest, CodexReviewStartResponse, CodexThreadListRequest,
    CodexThreadListResponse, CodexThreadReadRequest, CodexThreadReadResponse, CodexTurnRunResponse,
};
use crate::{lock_active_session, AppState};

const DEFAULT_PAGE_SIZE: usize = 25;
const MAX_PAGE_SIZE: usize = 100;

#[cfg(feature = "native-codex-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalPendingKind {
    CommandExecution,
    FileChange,
}

#[cfg(feature = "native-codex-runtime")]
#[derive(Debug)]
pub(crate) struct ApprovalResponsePlan {
    pub(crate) op: Op,
    pub(crate) resolved_event: Value,
}

#[cfg(feature = "native-codex-runtime")]
#[derive(Debug)]
pub(crate) struct UserInputResponsePlan {
    pub(crate) op: Op,
    pub(crate) resolved_event: Value,
    pub(crate) decision: String,
}

#[cfg(feature = "native-codex-runtime")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeSessionSlotReservation {
    pub(crate) session_id: u64,
    pub(crate) pid: Option<u32>,
    pub(crate) cwd: PathBuf,
    pub(crate) initial_thread_id: Option<String>,
}

#[cfg(feature = "native-codex-runtime")]
impl NativeSessionSlotReservation {
    pub(crate) fn turn_run_accepted_response(&self) -> CodexTurnRunResponse {
        CodexTurnRunResponse {
            accepted: true,
            session_id: self.session_id,
            thread_id: self.initial_thread_id.clone(),
        }
    }

    pub(crate) fn review_start_accepted_response(&self) -> CodexReviewStartResponse {
        CodexReviewStartResponse {
            accepted: true,
            session_id: self.session_id,
            thread_id: self.initial_thread_id.clone(),
            review_thread_id: self.initial_thread_id.clone(),
        }
    }
}

#[cfg(feature = "native-codex-runtime")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeThreadSchedulePlan {
    pub(crate) reservation: NativeSessionSlotReservation,
    pub(crate) requested_thread_id: Option<String>,
}

#[cfg(feature = "native-codex-runtime")]
fn reserve_native_session_slot(
    state: &State<'_, AppState>,
) -> Result<NativeSessionSlotReservation, String> {
    let mut guard = lock_active_session(state.inner())?;
    let active = guard
        .as_mut()
        .ok_or_else(|| "no active codex session".to_string())?;

    if active.busy {
        return Err("codex session is still processing the previous turn".to_string());
    }

    active.busy = true;

    Ok(NativeSessionSlotReservation {
        session_id: active.session_id,
        pid: active.pid,
        cwd: active.cwd.clone(),
        initial_thread_id: active.thread_id.clone(),
    })
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn plan_native_thread_schedule(
    state: &State<'_, AppState>,
    requested_thread_id: Option<String>,
) -> Result<NativeThreadSchedulePlan, String> {
    let reservation = reserve_native_session_slot(state)?;
    let requested_thread_id = schedule_policy::resolve_scheduled_thread_id(
        requested_thread_id,
        reservation.initial_thread_id.clone(),
    );

    Ok(NativeThreadSchedulePlan {
        reservation,
        requested_thread_id,
    })
}

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

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn plan_native_review_start_op(
    target: Option<Value>,
    delivery: Option<String>,
) -> Result<Op, String> {
    let review_request = parse_native_review_request(target, delivery)?;
    Ok(Op::Review { review_request })
}

#[cfg(feature = "native-codex-runtime")]
fn parse_native_review_request(
    target: Option<Value>,
    delivery: Option<String>,
) -> Result<ReviewRequest, String> {
    let target = match target {
        Some(target_value) => serde_json::from_value::<ReviewTarget>(target_value)
            .map_err(|error| format!("target is invalid for native review request: {error}"))?,
        None => ReviewTarget::UncommittedChanges,
    };

    let user_facing_hint = delivery
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .map(|value| format!("delivery:{value}"));

    Ok(ReviewRequest {
        target,
        user_facing_hint,
    })
}

#[cfg(feature = "native-codex-runtime")]
fn approval_kind_label(kind: ApprovalPendingKind) -> &'static str {
    match kind {
        ApprovalPendingKind::CommandExecution => "command_execution",
        ApprovalPendingKind::FileChange => "file_change",
    }
}

#[cfg(feature = "native-codex-runtime")]
fn to_domain_approval_kind(kind: ApprovalPendingKind) -> interaction_policy::ApprovalKind {
    match kind {
        ApprovalPendingKind::CommandExecution => interaction_policy::ApprovalKind::CommandExecution,
        ApprovalPendingKind::FileChange => interaction_policy::ApprovalKind::FileChange,
    }
}

#[cfg(feature = "native-codex-runtime")]
fn to_review_decision(decision: interaction_policy::ApprovalDecision) -> ReviewDecision {
    match decision {
        interaction_policy::ApprovalDecision::Approved => ReviewDecision::Approved,
        interaction_policy::ApprovalDecision::ApprovedForSession => {
            ReviewDecision::ApprovedForSession
        }
        interaction_policy::ApprovalDecision::Denied => ReviewDecision::Denied,
        interaction_policy::ApprovalDecision::Abort => ReviewDecision::Abort,
        interaction_policy::ApprovalDecision::ApprovedExecpolicyAmendment {
            proposed_execpolicy_amendment,
        } => ReviewDecision::ApprovedExecpolicyAmendment {
            proposed_execpolicy_amendment: ExecPolicyAmendment::new(proposed_execpolicy_amendment),
        },
    }
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn plan_approval_response(
    action_id: &str,
    pending_kind: ApprovalPendingKind,
    pending_call_id: &str,
    pending_turn_id: &str,
    decision: &str,
    remember: bool,
    execpolicy_amendment: Option<Vec<String>>,
) -> Result<ApprovalResponsePlan, String> {
    let normalized_execpolicy_amendment =
        interaction_policy::normalize_execpolicy_amendment(execpolicy_amendment);
    let domain_decision = interaction_policy::resolve_approval_decision(
        to_domain_approval_kind(pending_kind),
        decision,
        remember,
        &normalized_execpolicy_amendment,
    )?;
    let review_decision = to_review_decision(domain_decision);
    let decision_label = review_decision.to_opaque_string().to_string();

    let op = match pending_kind {
        ApprovalPendingKind::CommandExecution => Op::ExecApproval {
            id: pending_call_id.to_string(),
            turn_id: Some(pending_turn_id.to_string()).filter(|value| !value.trim().is_empty()),
            decision: review_decision,
        },
        ApprovalPendingKind::FileChange => Op::PatchApproval {
            id: pending_call_id.to_string(),
            decision: review_decision,
        },
    };

    Ok(ApprovalResponsePlan {
        op,
        resolved_event: json!({
            "type": "approval.resolved",
            "action_id": action_id,
            "kind": approval_kind_label(pending_kind),
            "decision": decision_label,
        }),
    })
}

#[cfg(feature = "native-codex-runtime")]
fn build_user_input_resolved_payload(
    action_id: &str,
    pending_thread_id: &str,
    pending_turn_id: &str,
    pending_call_id: &str,
    outcome: &str,
) -> Value {
    let mut resolved_payload = serde_json::Map::new();
    resolved_payload.insert(
        "type".to_string(),
        Value::String("user_input.resolved".to_string()),
    );
    resolved_payload.insert(
        "action_id".to_string(),
        Value::String(action_id.to_string()),
    );
    resolved_payload.insert(
        "thread_id".to_string(),
        Value::String(pending_thread_id.to_string()),
    );
    resolved_payload.insert(
        "turn_id".to_string(),
        Value::String(pending_turn_id.to_string()),
    );
    resolved_payload.insert(
        "item_id".to_string(),
        Value::String(pending_call_id.to_string()),
    );
    resolved_payload.insert("outcome".to_string(), Value::String(outcome.to_string()));
    if outcome == "cancelled" {
        resolved_payload.insert(
            "error".to_string(),
            Value::String("user input cancelled by user".to_string()),
        );
    }
    Value::Object(resolved_payload)
}

#[cfg(feature = "native-codex-runtime")]
fn to_protocol_answers(
    answers: HashMap<String, interaction_policy::UserInputAnswer>,
) -> HashMap<String, RequestUserInputAnswer> {
    answers
        .into_iter()
        .map(|(question_id, answer)| {
            (
                question_id,
                RequestUserInputAnswer {
                    answers: answer.answers,
                },
            )
        })
        .collect()
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn plan_user_input_response(
    action_id: &str,
    pending_thread_id: &str,
    pending_turn_id: &str,
    pending_call_id: &str,
    decision: &str,
    answers: HashMap<String, Value>,
) -> Result<UserInputResponsePlan, String> {
    let user_decision = interaction_policy::parse_user_input_decision(decision)?;
    let response_id =
        interaction_policy::resolve_user_input_response_id(pending_turn_id, pending_call_id)?;

    let protocol_answers = match user_decision {
        interaction_policy::UserInputDecision::Submit => {
            to_protocol_answers(interaction_policy::normalize_user_input_answers(answers))
        }
        interaction_policy::UserInputDecision::Cancel => HashMap::new(),
    };

    Ok(UserInputResponsePlan {
        op: Op::UserInputAnswer {
            id: response_id,
            response: RequestUserInputResponse {
                answers: protocol_answers,
            },
        },
        resolved_event: build_user_input_resolved_payload(
            action_id,
            pending_thread_id,
            pending_turn_id,
            pending_call_id,
            user_decision.outcome(),
        ),
        decision: user_decision.as_str().to_string(),
    })
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

    #[cfg(feature = "native-codex-runtime")]
    use super::{
        plan_approval_response, plan_native_review_start_op, plan_user_input_response,
        validate_review_start_request, ApprovalPendingKind, NativeSessionSlotReservation,
    };
    #[cfg(feature = "native-codex-runtime")]
    use crate::interface::tauri::dto::CodexReviewStartRequest;
    #[cfg(feature = "native-codex-runtime")]
    use codex_protocol::protocol::{Op, ReviewTarget};
    #[cfg(feature = "native-codex-runtime")]
    use serde_json::{json, Value};
    #[cfg(feature = "native-codex-runtime")]
    use std::collections::HashMap;
    #[cfg(feature = "native-codex-runtime")]
    use std::path::PathBuf;

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

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn native_session_slot_reservation_turn_run_accepted_shape() {
        let reservation = NativeSessionSlotReservation {
            session_id: 7,
            pid: Some(42),
            cwd: PathBuf::from("C:/workspace"),
            initial_thread_id: Some("thread-1".to_string()),
        };

        let response = reservation.turn_run_accepted_response();

        assert!(response.accepted);
        assert_eq!(response.session_id, 7);
        assert_eq!(response.thread_id, Some("thread-1".to_string()));
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn native_session_slot_reservation_review_start_accepted_shape() {
        let reservation = NativeSessionSlotReservation {
            session_id: 9,
            pid: None,
            cwd: PathBuf::from("C:/workspace"),
            initial_thread_id: Some("thread-review".to_string()),
        };

        let response = reservation.review_start_accepted_response();

        assert!(response.accepted);
        assert_eq!(response.session_id, 9);
        assert_eq!(response.thread_id, Some("thread-review".to_string()));
        assert_eq!(response.review_thread_id, Some("thread-review".to_string()));
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn plan_native_review_start_op_falls_back_to_uncommitted_changes() {
        let op = plan_native_review_start_op(None, Some(" INLINE ".to_string()))
            .expect("review plan should be valid");

        match op {
            Op::Review { review_request } => {
                assert_eq!(review_request.target, ReviewTarget::UncommittedChanges);
                assert_eq!(
                    review_request.user_facing_hint.as_deref(),
                    Some("delivery:inline")
                );
            }
            other => panic!("expected Op::Review, got {other:?}"),
        }
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn plan_native_review_start_op_rejects_invalid_target() {
        let invalid_target = json!("invalid");
        let expected_error = format!(
            "target is invalid for native review request: {}",
            serde_json::from_value::<ReviewTarget>(invalid_target.clone())
                .expect_err("target should be invalid")
        );

        let result = plan_native_review_start_op(Some(invalid_target), None);
        assert_eq!(result, Err(expected_error));
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn validate_review_start_request_applies_delivery_rules() {
        let valid = CodexReviewStartRequest {
            thread_id: None,
            target: None,
            delivery: Some(" detached ".to_string()),
        };
        assert_eq!(validate_review_start_request(&valid), Ok(()));

        let invalid = CodexReviewStartRequest {
            thread_id: None,
            target: None,
            delivery: Some("mail".to_string()),
        };
        assert_eq!(
            validate_review_start_request(&invalid),
            Err("delivery must be `inline` or `detached`".to_string())
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn plan_approval_response_builds_exec_approval_with_resolved_event() {
        let plan = plan_approval_response(
            "approval-1",
            ApprovalPendingKind::CommandExecution,
            "call-1",
            "turn-1",
            "acceptWithExecpolicyAmendment",
            false,
            Some(vec!["  echo hello  ".to_string(), "".to_string()]),
        )
        .expect("approval plan should be valid");

        let decision_label = match &plan.op {
            Op::ExecApproval {
                id,
                turn_id,
                decision,
            } => {
                assert_eq!(id, "call-1");
                assert_eq!(turn_id.as_deref(), Some("turn-1"));
                decision.to_opaque_string().to_string()
            }
            other => panic!("expected Op::ExecApproval, got {other:?}"),
        };

        assert_eq!(
            plan.resolved_event.get("type").and_then(Value::as_str),
            Some("approval.resolved")
        );
        assert_eq!(
            plan.resolved_event.get("action_id").and_then(Value::as_str),
            Some("approval-1")
        );
        assert_eq!(
            plan.resolved_event.get("kind").and_then(Value::as_str),
            Some("command_execution")
        );
        assert_eq!(
            plan.resolved_event.get("decision").and_then(Value::as_str),
            Some(decision_label.as_str())
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn plan_user_input_response_builds_submit_plan_and_normalizes_answers() {
        let mut answers = HashMap::new();
        answers.insert("question-1".to_string(), json!([" yes ", " "]));

        let plan = plan_user_input_response(
            "user-input-1",
            "thread-1",
            "  ",
            "call-1",
            "submit",
            answers,
        )
        .expect("user input submit plan should be valid");

        assert_eq!(plan.decision, "submit");

        match &plan.op {
            Op::UserInputAnswer { id, response } => {
                assert_eq!(id, "call-1");
                assert_eq!(
                    response
                        .answers
                        .get("question-1")
                        .map(|answer| answer.answers.clone()),
                    Some(vec!["yes".to_string()])
                );
            }
            other => panic!("expected Op::UserInputAnswer, got {other:?}"),
        }

        assert_eq!(
            plan.resolved_event.get("type").and_then(Value::as_str),
            Some("user_input.resolved")
        );
        assert_eq!(
            plan.resolved_event.get("outcome").and_then(Value::as_str),
            Some("submitted")
        );
        assert!(plan.resolved_event.get("error").is_none());
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn plan_user_input_response_rejects_missing_response_identifier() {
        let result = plan_user_input_response(
            "user-input-1",
            "thread-1",
            " ",
            " ",
            "submit",
            HashMap::new(),
        );

        assert_eq!(
            result.err(),
            Some("missing turn identifier for user_input response".to_string())
        );
    }

    #[cfg(feature = "native-codex-runtime")]
    #[test]
    fn plan_user_input_response_rejects_invalid_decision() {
        let result = plan_user_input_response(
            "user-input-1",
            "thread-1",
            "turn-1",
            "call-1",
            "later",
            HashMap::new(),
        );

        assert_eq!(
            result.err(),
            Some("decision must be one of: submit, cancel".to_string())
        );
    }
}
