use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalKind {
    CommandExecution,
    FileChange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalDecision {
    Approved,
    ApprovedForSession,
    Denied,
    Abort,
    ApprovedExecpolicyAmendment {
        proposed_execpolicy_amendment: Vec<String>,
    },
}

pub(crate) fn normalize_execpolicy_amendment(
    execpolicy_amendment: Option<Vec<String>>,
) -> Vec<String> {
    execpolicy_amendment
        .unwrap_or_default()
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>()
}

pub(crate) fn resolve_approval_decision(
    kind: ApprovalKind,
    decision: &str,
    remember: bool,
    execpolicy_amendment: &[String],
) -> Result<ApprovalDecision, String> {
    let normalized = decision.trim();
    if normalized.is_empty() {
        return Err("decision is required".to_string());
    }

    let mut decision_key = normalized.to_ascii_lowercase();
    if remember && decision_key == "accept" {
        decision_key = "acceptforsession".to_string();
    }

    match decision_key.as_str() {
        "accept" => Ok(ApprovalDecision::Approved),
        "acceptforsession" => Ok(ApprovalDecision::ApprovedForSession),
        "decline" => Ok(ApprovalDecision::Denied),
        "cancel" => Ok(ApprovalDecision::Abort),
        "acceptwithexecpolicyamendment" => {
            if !matches!(kind, ApprovalKind::CommandExecution) {
                return Err(
                    "acceptWithExecpolicyAmendment is only supported for command_execution approvals"
                        .to_string(),
                );
            }
            if execpolicy_amendment.is_empty() {
                return Err(
                    "acceptWithExecpolicyAmendment requires execpolicyAmendment".to_string()
                );
            }
            Ok(ApprovalDecision::ApprovedExecpolicyAmendment {
                proposed_execpolicy_amendment: execpolicy_amendment.to_vec(),
            })
        }
        _ => Err(format!("unsupported approval decision: {normalized}")),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UserInputDecision {
    Submit,
    Cancel,
}

impl UserInputDecision {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Submit => "submit",
            Self::Cancel => "cancel",
        }
    }

    pub(crate) fn outcome(self) -> &'static str {
        match self {
            Self::Submit => "submitted",
            Self::Cancel => "cancelled",
        }
    }
}

pub(crate) fn parse_user_input_decision(decision: &str) -> Result<UserInputDecision, String> {
    let normalized = decision.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("decision is required".to_string());
    }
    match normalized.as_str() {
        "submit" => Ok(UserInputDecision::Submit),
        "cancel" => Ok(UserInputDecision::Cancel),
        _ => Err("decision must be one of: submit, cancel".to_string()),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UserInputAnswer {
    pub(crate) answers: Vec<String>,
}

pub(crate) fn normalize_user_input_answers(
    answers: HashMap<String, Value>,
) -> HashMap<String, UserInputAnswer> {
    fn as_answer_list(value: Value) -> Vec<String> {
        let raw_answers = match value {
            Value::Object(mut object) => object
                .remove("answers")
                .or_else(|| object.remove("value"))
                .unwrap_or(Value::Null),
            other => other,
        };

        match raw_answers {
            Value::Array(entries) => entries
                .into_iter()
                .map(|entry| match entry {
                    Value::String(text) => text.trim().to_string(),
                    other => other.to_string(),
                })
                .filter(|entry| !entry.is_empty())
                .collect(),
            Value::String(text) => {
                let trimmed = text.trim().to_string();
                if trimmed.is_empty() {
                    Vec::new()
                } else {
                    vec![trimmed]
                }
            }
            Value::Null => Vec::new(),
            other => vec![other.to_string()],
        }
    }

    let mut normalized = HashMap::new();
    for (question_id, value) in answers {
        let question_id = question_id.trim().to_string();
        if question_id.is_empty() {
            continue;
        }
        normalized.insert(
            question_id,
            UserInputAnswer {
                answers: as_answer_list(value),
            },
        );
    }
    normalized
}

pub(crate) fn resolve_user_input_response_id(
    turn_id: &str,
    call_id: &str,
) -> Result<String, String> {
    let response_id = if turn_id.trim().is_empty() {
        call_id.to_string()
    } else {
        turn_id.to_string()
    };
    if response_id.trim().is_empty() {
        return Err("missing turn identifier for user_input response".to_string());
    }
    Ok(response_id)
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_execpolicy_amendment, normalize_user_input_answers, parse_user_input_decision,
        resolve_approval_decision, resolve_user_input_response_id, ApprovalDecision, ApprovalKind,
        UserInputAnswer, UserInputDecision,
    };
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn resolve_approval_decision_matrix() {
        let matrix = vec![
            (
                ApprovalKind::CommandExecution,
                "accept",
                false,
                vec![],
                Ok(ApprovalDecision::Approved),
            ),
            (
                ApprovalKind::CommandExecution,
                "accept",
                true,
                vec![],
                Ok(ApprovalDecision::ApprovedForSession),
            ),
            (
                ApprovalKind::CommandExecution,
                "decline",
                false,
                vec![],
                Ok(ApprovalDecision::Denied),
            ),
            (
                ApprovalKind::CommandExecution,
                "cancel",
                false,
                vec![],
                Ok(ApprovalDecision::Abort),
            ),
            (
                ApprovalKind::CommandExecution,
                "acceptWithExecpolicyAmendment",
                false,
                vec!["echo ok".to_string()],
                Ok(ApprovalDecision::ApprovedExecpolicyAmendment {
                    proposed_execpolicy_amendment: vec!["echo ok".to_string()],
                }),
            ),
        ];

        for (kind, decision, remember, amendment, expected) in matrix {
            let result = resolve_approval_decision(kind, decision, remember, &amendment);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn resolve_approval_decision_rejects_execpolicy_amendment_for_file_change() {
        let result = resolve_approval_decision(
            ApprovalKind::FileChange,
            "acceptWithExecpolicyAmendment",
            false,
            &["echo nope".to_string()],
        );
        assert_eq!(
            result,
            Err(
                "acceptWithExecpolicyAmendment is only supported for command_execution approvals"
                    .to_string()
            )
        );
    }

    #[test]
    fn resolve_approval_decision_requires_execpolicy_amendment_entries() {
        let result = resolve_approval_decision(
            ApprovalKind::CommandExecution,
            "acceptWithExecpolicyAmendment",
            false,
            &[],
        );
        assert_eq!(
            result,
            Err("acceptWithExecpolicyAmendment requires execpolicyAmendment".to_string())
        );
    }

    #[test]
    fn normalize_execpolicy_amendment_discards_empty_entries() {
        let result = normalize_execpolicy_amendment(Some(vec![
            "  git status  ".to_string(),
            " ".to_string(),
            "\n".to_string(),
        ]));
        assert_eq!(result, vec!["git status".to_string()]);
    }

    #[test]
    fn parse_user_input_decision_accepts_submit_and_cancel() {
        assert_eq!(
            parse_user_input_decision(" Submit "),
            Ok(UserInputDecision::Submit)
        );
        assert_eq!(
            parse_user_input_decision("cancel"),
            Ok(UserInputDecision::Cancel)
        );
    }

    #[test]
    fn parse_user_input_decision_rejects_invalid_values() {
        assert_eq!(
            parse_user_input_decision(""),
            Err("decision is required".to_string())
        );
        assert_eq!(
            parse_user_input_decision("later"),
            Err("decision must be one of: submit, cancel".to_string())
        );
    }

    #[test]
    fn normalize_user_input_answers_handles_mixed_payloads() {
        let mut raw_answers = HashMap::new();
        raw_answers.insert("question-1".to_string(), json!([" yes ", "", 7]));
        raw_answers.insert(
            "question-2".to_string(),
            json!({ "answers": [" first ", " ", "second"] }),
        );
        raw_answers.insert("question-3".to_string(), json!({ "value": " maybe " }));
        raw_answers.insert("question-4".to_string(), json!("   "));
        raw_answers.insert("  ".to_string(), json!("ignored"));

        let normalized = normalize_user_input_answers(raw_answers);

        assert_eq!(
            normalized.get("question-1"),
            Some(&UserInputAnswer {
                answers: vec!["yes".to_string(), "7".to_string()],
            })
        );
        assert_eq!(
            normalized.get("question-2"),
            Some(&UserInputAnswer {
                answers: vec!["first".to_string(), "second".to_string()],
            })
        );
        assert_eq!(
            normalized.get("question-3"),
            Some(&UserInputAnswer {
                answers: vec!["maybe".to_string()],
            })
        );
        assert_eq!(
            normalized.get("question-4"),
            Some(&UserInputAnswer { answers: vec![] })
        );
        assert!(!normalized.contains_key(""));
        assert_eq!(normalized.len(), 4);
    }

    #[test]
    fn resolve_user_input_response_id_uses_turn_then_call() {
        assert_eq!(
            resolve_user_input_response_id("turn-1", "call-1"),
            Ok("turn-1".to_string())
        );
        assert_eq!(
            resolve_user_input_response_id("  ", "call-1"),
            Ok("call-1".to_string())
        );
    }

    #[test]
    fn resolve_user_input_response_id_requires_non_empty_turn_or_call() {
        let result = resolve_user_input_response_id(" ", " ");
        assert_eq!(
            result,
            Err("missing turn identifier for user_input response".to_string())
        );
    }
}
