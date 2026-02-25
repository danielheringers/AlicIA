#[cfg(feature = "native-codex-runtime")]
use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(feature = "native-codex-runtime")]
use serde_json::{json, Value};

#[cfg(feature = "native-codex-runtime")]
use codex_core::protocol::{
    AgentStatus, CollabAgentInteractionEndEvent, CollabAgentSpawnEndEvent, CollabCloseEndEvent,
    CollabResumeEndEvent, CollabWaitingEndEvent, Event, EventMsg, ExecCommandEndEvent,
    ExecCommandStatus, ExitedReviewModeEvent, FileChange as ProtocolFileChange, ItemCompletedEvent,
    ItemStartedEvent, McpToolCallEndEvent, PatchApplyEndEvent, PatchApplyStatus, ReviewRequest,
};

#[cfg(feature = "native-codex-runtime")]
use codex_protocol::items::{AgentMessageContent, TurnItem};

#[cfg(feature = "native-codex-runtime")]
use codex_protocol::models::{FunctionCallOutputPayload, ResponseItem};

#[cfg(feature = "native-codex-runtime")]
use codex_protocol::plan_tool::StepStatus;

#[cfg(feature = "native-codex-runtime")]
use codex_protocol::ThreadId;

#[cfg(feature = "native-codex-runtime")]
pub(crate) struct NativeCodexEventTranslator {
    thread_id: String,
    last_turn_error: Option<Value>,
    agent_buffers: HashMap<String, String>,
    pending_raw_collab_calls: HashMap<String, PendingRawCollabCall>,
    completed_collab_call_ids: BoundedStringSet,
}

#[cfg(feature = "native-codex-runtime")]
const REVIEW_FALLBACK_MESSAGE: &str = "Reviewer failed to output a response.";

#[cfg(feature = "native-codex-runtime")]
const COMPLETED_COLLAB_CALL_IDS_CAPACITY: usize = 128;

#[cfg(feature = "native-codex-runtime")]
#[derive(Debug, Clone)]
struct PendingRawCollabCall {
    tool: String,
    sender_thread_id: String,
    receiver_thread_ids: Vec<String>,
    prompt: String,
    parsed_arguments: Value,
}

#[cfg(feature = "native-codex-runtime")]
#[derive(Debug, Clone)]
struct BoundedStringSet {
    max_entries: usize,
    entries: HashSet<String>,
    order: VecDeque<String>,
}

#[cfg(feature = "native-codex-runtime")]
impl BoundedStringSet {
    fn with_capacity(max_entries: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            entries: HashSet::new(),
            order: VecDeque::new(),
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    fn contains(&self, value: &str) -> bool {
        self.entries.contains(value)
    }

    fn insert(&mut self, value: String) {
        if !self.entries.insert(value.clone()) {
            return;
        }
        self.order.push_back(value);
        while self.order.len() > self.max_entries {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
    }
}

#[cfg(feature = "native-codex-runtime")]
impl NativeCodexEventTranslator {
    pub(crate) fn new(thread_id: String) -> Self {
        Self {
            thread_id,
            last_turn_error: None,
            agent_buffers: HashMap::new(),
            pending_raw_collab_calls: HashMap::new(),
            completed_collab_call_ids: BoundedStringSet::with_capacity(
                COMPLETED_COLLAB_CALL_IDS_CAPACITY,
            ),
        }
    }

    fn clear_raw_collab_caches(&mut self) {
        self.pending_raw_collab_calls.clear();
        self.completed_collab_call_ids.clear();
    }

    fn mark_collab_call_completed(&mut self, call_id: &str) {
        self.pending_raw_collab_calls.remove(call_id);
        self.completed_collab_call_ids.insert(call_id.to_string());
    }

    fn translate_collab_agent_spawn_end(&mut self, event: &CollabAgentSpawnEndEvent) -> Vec<Value> {
        if self.completed_collab_call_ids.contains(&event.call_id) {
            return Vec::new();
        }
        self.mark_collab_call_completed(&event.call_id);
        map_collab_agent_spawn_end(event)
    }

    fn translate_collab_agent_interaction_end(
        &mut self,
        event: &CollabAgentInteractionEndEvent,
    ) -> Vec<Value> {
        if self.completed_collab_call_ids.contains(&event.call_id) {
            return Vec::new();
        }
        self.mark_collab_call_completed(&event.call_id);
        map_collab_agent_interaction_end(event)
    }

    fn translate_collab_waiting_end(&mut self, event: &CollabWaitingEndEvent) -> Vec<Value> {
        if self.completed_collab_call_ids.contains(&event.call_id) {
            return Vec::new();
        }
        self.mark_collab_call_completed(&event.call_id);
        map_collab_waiting_end(event)
    }

    fn translate_collab_close_end(&mut self, event: &CollabCloseEndEvent) -> Vec<Value> {
        if self.completed_collab_call_ids.contains(&event.call_id) {
            return Vec::new();
        }
        self.mark_collab_call_completed(&event.call_id);
        map_collab_close_end(event)
    }

    fn translate_collab_resume_end(&mut self, event: &CollabResumeEndEvent) -> Vec<Value> {
        if self.completed_collab_call_ids.contains(&event.call_id) {
            return Vec::new();
        }
        self.mark_collab_call_completed(&event.call_id);
        map_collab_resume_end(event)
    }

    fn translate_raw_response_item(&mut self, item: &ResponseItem) -> Vec<Value> {
        match item {
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => {
                if self.completed_collab_call_ids.contains(call_id) {
                    return Vec::new();
                }
                if let Some(pending) = pending_raw_collab_call(
                    self.thread_id.as_str(),
                    name.as_str(),
                    arguments.as_str(),
                ) {
                    self.pending_raw_collab_calls
                        .insert(call_id.clone(), pending);
                }
                Vec::new()
            }
            ResponseItem::FunctionCallOutput { call_id, output } => {
                if self.completed_collab_call_ids.contains(call_id) {
                    return Vec::new();
                }
                let pending = self
                    .pending_raw_collab_calls
                    .get(call_id)
                    .cloned()
                    .or_else(|| {
                        infer_pending_raw_collab_call_from_output(self.thread_id.as_str(), output)
                    });
                let Some(pending) = pending else {
                    return Vec::new();
                };
                let Some(mapped) = map_raw_collab_function_call_output(call_id, &pending, output)
                else {
                    return Vec::new();
                };
                if mapped_collab_status_is_terminal(&mapped) {
                    self.mark_collab_call_completed(call_id);
                }
                mapped
            }
            _ => Vec::new(),
        }
    }

    pub(crate) fn translate_event(
        &mut self,
        event: &Event,
        native: &mut crate::NativeSessionHandles,
    ) -> Vec<Value> {
        match &event.msg {
            EventMsg::TurnStarted(_) => {
                self.last_turn_error = None;
                self.agent_buffers.clear();
                self.clear_raw_collab_caches();
                native
                    .active_turns
                    .insert(self.thread_id.clone(), event.id.clone());
                vec![json!({
                    "type": "turn.started",
                    "thread_id": self.thread_id.clone(),
                    "turn_id": event.id.clone(),
                })]
            }
            EventMsg::TurnComplete(_) => {
                if native
                    .active_turns
                    .get(self.thread_id.as_str())
                    .is_some_and(|active_turn_id| active_turn_id == event.id.as_str())
                {
                    native.active_turns.remove(self.thread_id.as_str());
                }
                self.agent_buffers.clear();
                self.clear_raw_collab_caches();
                if let Some(error) = self.last_turn_error.take() {
                    vec![json!({
                        "type": "turn.failed",
                        "thread_id": self.thread_id.clone(),
                        "turn_id": event.id.clone(),
                        "error": error,
                    })]
                } else {
                    vec![json!({
                        "type": "turn.completed",
                        "thread_id": self.thread_id.clone(),
                        "turn_id": event.id.clone(),
                    })]
                }
            }
            EventMsg::TurnAborted(aborted) => {
                if native
                    .active_turns
                    .get(self.thread_id.as_str())
                    .is_some_and(|active_turn_id| active_turn_id == event.id.as_str())
                {
                    native.active_turns.remove(self.thread_id.as_str());
                }
                self.agent_buffers.clear();
                self.clear_raw_collab_caches();
                let reason = match aborted.reason {
                    codex_core::protocol::TurnAbortReason::Interrupted => "turn interrupted",
                    codex_core::protocol::TurnAbortReason::Replaced => "turn replaced",
                    codex_core::protocol::TurnAbortReason::ReviewEnded => "review ended",
                };
                vec![json!({
                    "type": "turn.failed",
                    "thread_id": self.thread_id.clone(),
                    "turn_id": event.id.clone(),
                    "error": {
                        "message": reason,
                    },
                })]
            }
            EventMsg::TokenCount(token_count) => token_count
                .info
                .as_ref()
                .map(|info| {
                    vec![json!({
                        "type": "thread.token_usage.updated",
                        "thread_id": self.thread_id.clone(),
                        "turn_id": event.id.clone(),
                        "token_usage": token_usage_to_legacy_json(info),
                    })]
                })
                .unwrap_or_default(),
            EventMsg::TurnDiff(diff) => vec![json!({
                "type": "turn.diff.updated",
                "thread_id": self.thread_id.clone(),
                "turn_id": event.id.clone(),
                "diff": diff.unified_diff.clone(),
            })],
            EventMsg::PlanUpdate(plan) => vec![json!({
                "type": "turn.plan.updated",
                "thread_id": self.thread_id.clone(),
                "turn_id": event.id.clone(),
                "explanation": plan.explanation.clone(),
                "plan": plan
                    .plan
                    .iter()
                    .map(|entry| {
                        json!({
                            "step": entry.step.clone(),
                            "status": legacy_plan_step_status(entry.status.clone()),
                        })
                    })
                    .collect::<Vec<_>>(),
            })],
            EventMsg::AgentMessageContentDelta(delta) => {
                let next_value = {
                    let previous = self.agent_buffers.entry(delta.item_id.clone()).or_default();
                    previous.push_str(&delta.delta);
                    previous.clone()
                };

                vec![json!({
                    "type": "item.updated",
                    "item": {
                        "type": "agent_message",
                        "id": delta.item_id.clone(),
                        "text": next_value,
                    },
                })]
            }
            EventMsg::ExecCommandEnd(exec_command_end) => map_exec_command_end(exec_command_end),
            EventMsg::McpToolCallEnd(mcp_tool_call_end) => map_mcp_tool_call_end(mcp_tool_call_end),
            EventMsg::PatchApplyEnd(patch_apply_end) => map_patch_apply_end(patch_apply_end),
            EventMsg::CollabAgentSpawnEnd(collab_spawn_end) => {
                self.translate_collab_agent_spawn_end(collab_spawn_end)
            }
            EventMsg::CollabAgentInteractionEnd(collab_interaction_end) => {
                self.translate_collab_agent_interaction_end(collab_interaction_end)
            }
            EventMsg::CollabWaitingEnd(collab_waiting_end) => {
                self.translate_collab_waiting_end(collab_waiting_end)
            }
            EventMsg::CollabCloseEnd(collab_close_end) => {
                self.translate_collab_close_end(collab_close_end)
            }
            EventMsg::CollabResumeEnd(collab_resume_end) => {
                self.translate_collab_resume_end(collab_resume_end)
            }
            EventMsg::RawResponseItem(raw_response_item) => {
                self.translate_raw_response_item(&raw_response_item.item)
            }
            EventMsg::EnteredReviewMode(review_request) => {
                map_entered_review_mode(&event.id, review_request)
            }
            EventMsg::ExitedReviewMode(review_event) => {
                map_exited_review_mode(&event.id, review_event)
            }
            EventMsg::ItemStarted(item) => map_item_started(item),
            EventMsg::ItemCompleted(item) => {
                let mapped = map_item_completed(item);
                if let Some(item_id) = mapped
                    .first()
                    .and_then(|value| value.get("item"))
                    .and_then(|item| item.get("id"))
                    .and_then(Value::as_str)
                {
                    self.agent_buffers.remove(item_id);
                }
                mapped
            }
            EventMsg::ExecApprovalRequest(request) => {
                let action_id = next_approval_action_id(native);
                native.pending_approvals.insert(
                    action_id.clone(),
                    crate::NativePendingApproval {
                        thread_id: self.thread_id.clone(),
                        turn_id: request.turn_id.clone(),
                        call_id: request.call_id.clone(),
                        kind: crate::NativeApprovalKind::CommandExecution,
                    },
                );

                let command = codex_core::parse_command::shlex_join(&request.command);
                let command_actions = serde_json::to_value(&request.parsed_cmd)
                    .unwrap_or_else(|_| Value::Array(Vec::new()));
                let proposed_execpolicy_amendment = request
                    .proposed_execpolicy_amendment
                    .as_ref()
                    .map(|value| value.command.clone())
                    .unwrap_or_default();

                vec![json!({
                    "type": "approval.requested",
                    "action_id": action_id,
                    "kind": "command_execution",
                    "thread_id": self.thread_id.clone(),
                    "turn_id": request.turn_id.clone(),
                    "item_id": request.call_id.clone(),
                    "reason": request.reason.clone().unwrap_or_default(),
                    "command": command,
                    "cwd": request.cwd.to_string_lossy().to_string(),
                    "command_actions": command_actions,
                    "proposed_execpolicy_amendment": proposed_execpolicy_amendment,
                    "grant_root": "",
                })]
            }
            EventMsg::ApplyPatchApprovalRequest(request) => {
                let action_id = next_approval_action_id(native);
                native.pending_approvals.insert(
                    action_id.clone(),
                    crate::NativePendingApproval {
                        thread_id: self.thread_id.clone(),
                        turn_id: request.turn_id.clone(),
                        call_id: request.call_id.clone(),
                        kind: crate::NativeApprovalKind::FileChange,
                    },
                );

                vec![json!({
                    "type": "approval.requested",
                    "action_id": action_id,
                    "kind": "file_change",
                    "thread_id": self.thread_id.clone(),
                    "turn_id": request.turn_id.clone(),
                    "item_id": request.call_id.clone(),
                    "reason": request.reason.clone().unwrap_or_default(),
                    "command": "",
                    "cwd": "",
                    "command_actions": [],
                    "proposed_execpolicy_amendment": [],
                    "grant_root": request
                        .grant_root
                        .as_ref()
                        .map(|path| path.to_string_lossy().to_string())
                        .unwrap_or_default(),
                })]
            }
            EventMsg::RequestUserInput(request) => {
                let action_id = next_user_input_action_id(native);
                native.pending_user_inputs.insert(
                    action_id.clone(),
                    crate::NativePendingUserInput {
                        thread_id: self.thread_id.clone(),
                        turn_id: request.turn_id.clone(),
                        call_id: request.call_id.clone(),
                    },
                );

                let questions = request
                    .questions
                    .iter()
                    .map(|question| {
                        json!({
                            "id": question.id.clone(),
                            "header": question.header.clone(),
                            "question": question.question.clone(),
                            "isOther": question.is_other,
                            "isSecret": question.is_secret,
                            "options": question.options.as_ref().map(|options| {
                                options
                                    .iter()
                                    .map(|option| {
                                        json!({
                                            "label": option.label.clone(),
                                            "description": option.description.clone(),
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            }),
                        })
                    })
                    .collect::<Vec<_>>();

                vec![json!({
                    "type": "user_input.requested",
                    "action_id": action_id,
                    "thread_id": self.thread_id.clone(),
                    "turn_id": request.turn_id.clone(),
                    "item_id": request.call_id.clone(),
                    "questions": questions,
                    "timeout_ms": 900000,
                })]
            }
            EventMsg::Error(error) => {
                if error.affects_turn_status() {
                    self.last_turn_error = Some(json!({
                        "message": error.message.clone(),
                        "codex_error_info": error.codex_error_info.clone(),
                    }));
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(feature = "native-codex-runtime")]
fn pending_raw_collab_call(
    sender_thread_id: &str,
    tool_name: &str,
    arguments: &str,
) -> Option<PendingRawCollabCall> {
    let tool = normalize_collab_tool_name(tool_name)?;
    let parsed_arguments: Value = serde_json::from_str(arguments).ok()?;
    let receiver_thread_ids = collab_receiver_thread_ids(tool.as_str(), &parsed_arguments);
    let prompt = if matches!(tool.as_str(), "spawn_agent" | "send_input") {
        collab_prompt_from_arguments(&parsed_arguments)
    } else {
        String::new()
    };

    Some(PendingRawCollabCall {
        tool,
        sender_thread_id: sender_thread_id.to_string(),
        receiver_thread_ids,
        prompt,
        parsed_arguments,
    })
}

#[cfg(feature = "native-codex-runtime")]
fn infer_pending_raw_collab_call_from_output(
    sender_thread_id: &str,
    output: &FunctionCallOutputPayload,
) -> Option<PendingRawCollabCall> {
    let parsed_output = parse_function_call_output_json(output)?;

    if read_text(
        parsed_output
            .get("agent_id")
            .or_else(|| parsed_output.get("agentId")),
    )
    .filter(|thread_id| is_valid_thread_id(thread_id))
    .is_some()
    {
        return Some(PendingRawCollabCall {
            tool: "spawn_agent".to_string(),
            sender_thread_id: sender_thread_id.to_string(),
            receiver_thread_ids: Vec::new(),
            prompt: String::new(),
            parsed_arguments: json!({}),
        });
    }

    let raw_statuses = parsed_output.get("status").and_then(Value::as_object)?;
    let mut receiver_thread_ids = raw_statuses
        .iter()
        .filter_map(|(thread_id, raw_status)| {
            if !is_valid_thread_id(thread_id) {
                return None;
            }
            parse_agent_status(raw_status).map(|_| thread_id.clone())
        })
        .collect::<Vec<_>>();
    receiver_thread_ids.sort();
    receiver_thread_ids.dedup();
    if receiver_thread_ids.is_empty() {
        return None;
    }

    Some(PendingRawCollabCall {
        tool: "wait".to_string(),
        sender_thread_id: sender_thread_id.to_string(),
        receiver_thread_ids,
        prompt: String::new(),
        parsed_arguments: json!({}),
    })
}

#[cfg(feature = "native-codex-runtime")]
fn is_valid_thread_id(candidate: &str) -> bool {
    ThreadId::from_string(candidate).is_ok()
}

#[cfg(feature = "native-codex-runtime")]
fn normalize_collab_tool_name(tool_name: &str) -> Option<String> {
    let normalized = normalize_tool_name_key(tool_name);
    let canonical = normalized.strip_prefix("collab_").unwrap_or(&normalized);

    if matches_collab_tool_alias(canonical, &["spawn_agent", "spawnagent"]) {
        return Some("spawn_agent".to_string());
    }
    if matches_collab_tool_alias(canonical, &["send_input", "sendinput"]) {
        return Some("send_input".to_string());
    }
    if matches_collab_tool_alias(canonical, &["resume_agent", "resumeagent"]) {
        return Some("resume_agent".to_string());
    }
    if matches_collab_tool_alias(canonical, &["wait", "wait_agents", "wait_agent"]) {
        return Some("wait".to_string());
    }
    if matches_collab_tool_alias(canonical, &["close_agent", "closeagent"]) {
        return Some("close_agent".to_string());
    }

    None
}

#[cfg(feature = "native-codex-runtime")]
fn matches_collab_tool_alias(normalized_tool_name: &str, aliases: &[&str]) -> bool {
    aliases.iter().any(|alias| {
        normalized_tool_name == *alias || normalized_tool_name.ends_with(&format!("_{alias}"))
    })
}

#[cfg(feature = "native-codex-runtime")]
fn normalize_tool_name_key(raw: &str) -> String {
    let mut normalized = String::with_capacity(raw.len());
    let mut wrote_separator = false;

    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            wrote_separator = false;
            continue;
        }
        if !wrote_separator {
            normalized.push('_');
            wrote_separator = true;
        }
    }

    normalized.trim_matches('_').to_string()
}

#[cfg(feature = "native-codex-runtime")]
fn collab_receiver_thread_ids(tool: &str, parsed_arguments: &Value) -> Vec<String> {
    match tool {
        "spawn_agent" => Vec::new(),
        "send_input" | "resume_agent" | "close_agent" => {
            read_text(parsed_arguments.get("id")).into_iter().collect()
        }
        "wait" => parsed_arguments
            .get("ids")
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(|entry| read_text(Some(entry)))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

#[cfg(feature = "native-codex-runtime")]
fn collab_prompt_from_arguments(parsed_arguments: &Value) -> String {
    if let Some(message) = read_text(parsed_arguments.get("message")) {
        return message;
    }

    parsed_arguments
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(collab_item_preview)
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default()
}

#[cfg(feature = "native-codex-runtime")]
fn collab_item_preview(item: &Value) -> Option<String> {
    let item_type = item
        .get("type")
        .and_then(Value::as_str)
        .map(normalize_tool_name_key)?;

    match item_type.as_str() {
        "text" => read_text(item.get("text")),
        "image" => Some("[image]".to_string()),
        "local_image" => read_text(item.get("path")).map(|path| format!("[local_image:{path}]")),
        "skill" => {
            let name = read_text(item.get("name"))?;
            let path = read_text(item.get("path"))?;
            Some(format!("[skill:${name}]({path})"))
        }
        "mention" => {
            let name = read_text(item.get("name"))?;
            let path = read_text(item.get("path"))?;
            Some(format!("[mention:{name}]({path})"))
        }
        _ => None,
    }
}

#[cfg(feature = "native-codex-runtime")]
fn map_raw_collab_function_call_output(
    call_id: &str,
    pending: &PendingRawCollabCall,
    output: &FunctionCallOutputPayload,
) -> Option<Vec<Value>> {
    let parsed_output = parse_function_call_output_json(output)?;
    let output_failed = output.success.is_some_and(|success| !success);
    let tool = pending.tool.as_str();

    match tool {
        "spawn_agent" => {
            let receiver_thread_id = read_text(
                parsed_output
                    .get("agent_id")
                    .or_else(|| parsed_output.get("agentId")),
            )?;
            let receiver_thread_ids = vec![receiver_thread_id.clone()];
            let running = AgentStatus::Running;
            let state_pairs = vec![(receiver_thread_id, &running)];
            Some(map_collab_tool_call_completed(
                call_id,
                tool,
                if output_failed { "failed" } else { "completed" },
                pending.sender_thread_id.clone(),
                receiver_thread_ids,
                pending.prompt.clone(),
                legacy_collab_agents_states_from_pairs(&state_pairs),
            ))
        }
        "send_input" => {
            let _submission_id = read_text(
                parsed_output
                    .get("submission_id")
                    .or_else(|| parsed_output.get("submissionId")),
            )?;
            let receiver_thread_ids = if pending.receiver_thread_ids.is_empty() {
                collab_receiver_thread_ids(tool, &pending.parsed_arguments)
            } else {
                pending.receiver_thread_ids.clone()
            };
            if receiver_thread_ids.is_empty() {
                return None;
            }
            let running = AgentStatus::Running;
            let state_pairs = receiver_thread_ids
                .iter()
                .cloned()
                .map(|thread_id| (thread_id, &running))
                .collect::<Vec<_>>();

            Some(map_collab_tool_call_completed(
                call_id,
                tool,
                if output_failed { "failed" } else { "completed" },
                pending.sender_thread_id.clone(),
                receiver_thread_ids,
                pending.prompt.clone(),
                legacy_collab_agents_states_from_pairs(&state_pairs),
            ))
        }
        "resume_agent" | "close_agent" => {
            let agent_status = parsed_output.get("status").and_then(parse_agent_status)?;
            let receiver_thread_ids = if pending.receiver_thread_ids.is_empty() {
                collab_receiver_thread_ids(tool, &pending.parsed_arguments)
            } else {
                pending.receiver_thread_ids.clone()
            };
            if receiver_thread_ids.is_empty() {
                return None;
            }
            let state_pairs = receiver_thread_ids
                .iter()
                .cloned()
                .map(|thread_id| (thread_id, &agent_status))
                .collect::<Vec<_>>();

            Some(map_collab_tool_call_completed(
                call_id,
                tool,
                if output_failed {
                    "failed"
                } else {
                    legacy_collab_status_from_agent_status(&agent_status)
                },
                pending.sender_thread_id.clone(),
                receiver_thread_ids,
                pending.prompt.clone(),
                legacy_collab_agents_states_from_pairs(&state_pairs),
            ))
        }
        "wait" => {
            let raw_statuses = parsed_output.get("status").and_then(Value::as_object)?;
            let mut parsed_statuses = raw_statuses
                .iter()
                .filter_map(|(thread_id, raw_status)| {
                    parse_agent_status(raw_status).map(|status| (thread_id.clone(), status))
                })
                .collect::<Vec<_>>();

            let mut receiver_thread_ids = if pending.receiver_thread_ids.is_empty() {
                raw_statuses.keys().cloned().collect::<Vec<_>>()
            } else {
                pending.receiver_thread_ids.clone()
            };
            receiver_thread_ids.sort();
            receiver_thread_ids.dedup();
            if receiver_thread_ids.is_empty() {
                return None;
            }

            let timed_out = parsed_output
                .get("timed_out")
                .or_else(|| parsed_output.get("timedOut"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            parsed_statuses.sort_by(|left, right| left.0.cmp(&right.0));
            let status = if output_failed
                || parsed_statuses.iter().any(|(_, state)| {
                    matches!(state, AgentStatus::Errored(_) | AgentStatus::NotFound)
                }) {
                "failed"
            } else if parsed_statuses.is_empty() || timed_out {
                "in_progress"
            } else {
                "completed"
            };
            let state_pairs = parsed_statuses
                .iter()
                .map(|(thread_id, state)| (thread_id.clone(), state))
                .collect::<Vec<_>>();

            Some(map_collab_tool_call_completed(
                call_id,
                tool,
                status,
                pending.sender_thread_id.clone(),
                receiver_thread_ids,
                pending.prompt.clone(),
                legacy_collab_agents_states_from_pairs(&state_pairs),
            ))
        }
        _ => None,
    }
}

#[cfg(feature = "native-codex-runtime")]
fn parse_function_call_output_json(output: &FunctionCallOutputPayload) -> Option<Value> {
    let output_text = output.body.to_text()?;
    serde_json::from_str(&output_text).ok()
}

#[cfg(feature = "native-codex-runtime")]
fn parse_agent_status(value: &Value) -> Option<AgentStatus> {
    serde_json::from_value(value.clone()).ok()
}

#[cfg(feature = "native-codex-runtime")]
fn mapped_collab_status_is_terminal(mapped_events: &[Value]) -> bool {
    mapped_events
        .first()
        .and_then(|event| event.get("item"))
        .and_then(|item| item.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|item_type| item_type == "collab_tool_call")
        && mapped_events
            .first()
            .and_then(|event| event.get("item"))
            .and_then(|item| item.get("status"))
            .and_then(Value::as_str)
            .is_some_and(|status| status != "in_progress")
}

#[cfg(feature = "native-codex-runtime")]
fn read_text(value: Option<&Value>) -> Option<String> {
    let text = value?.as_str()?.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

#[cfg(feature = "native-codex-runtime")]
fn next_approval_action_id(native: &mut crate::NativeSessionHandles) -> String {
    let value = native.next_approval_id;
    native.next_approval_id += 1;
    format!("approval-{value}")
}

#[cfg(feature = "native-codex-runtime")]
fn next_user_input_action_id(native: &mut crate::NativeSessionHandles) -> String {
    let value = native.next_user_input_id;
    native.next_user_input_id += 1;
    format!("user-input-{value}")
}

#[cfg(feature = "native-codex-runtime")]
fn legacy_plan_step_status(status: StepStatus) -> &'static str {
    match status {
        StepStatus::Pending => "pending",
        StepStatus::InProgress => "inProgress",
        StepStatus::Completed => "completed",
    }
}

#[cfg(feature = "native-codex-runtime")]
fn legacy_exec_command_status(status: &ExecCommandStatus) -> &'static str {
    match status {
        ExecCommandStatus::Completed => "completed",
        ExecCommandStatus::Failed => "failed",
        ExecCommandStatus::Declined => "declined",
    }
}

#[cfg(feature = "native-codex-runtime")]
fn legacy_patch_apply_status(status: &PatchApplyStatus) -> &'static str {
    match status {
        PatchApplyStatus::Completed => "completed",
        PatchApplyStatus::Failed => "failed",
        PatchApplyStatus::Declined => "declined",
    }
}

#[cfg(feature = "native-codex-runtime")]
fn map_item_started(item: &ItemStartedEvent) -> Vec<Value> {
    let Some(mapped_item) = turn_item_to_legacy(&item.item) else {
        return Vec::new();
    };

    vec![json!({
        "type": "item.started",
        "item": mapped_item,
    })]
}

#[cfg(feature = "native-codex-runtime")]
fn map_item_completed(item: &ItemCompletedEvent) -> Vec<Value> {
    let Some(mapped_item) = turn_item_to_legacy(&item.item) else {
        return Vec::new();
    };

    vec![json!({
        "type": "item.completed",
        "item": mapped_item,
    })]
}

#[cfg(feature = "native-codex-runtime")]
fn map_exec_command_end(event: &ExecCommandEndEvent) -> Vec<Value> {
    vec![json!({
        "type": "item.completed",
        "item": {
            "type": "command_execution",
            "id": event.call_id.clone(),
            "command": codex_core::parse_command::shlex_join(&event.command),
            "status": legacy_exec_command_status(&event.status),
            "aggregated_output": event.aggregated_output.clone(),
            "exit_code": event.exit_code,
        },
    })]
}

#[cfg(feature = "native-codex-runtime")]
fn map_mcp_tool_call_end(event: &McpToolCallEndEvent) -> Vec<Value> {
    let arguments = event
        .invocation
        .arguments
        .as_ref()
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or_else(|| json!({}));

    let (result, error) = match &event.result {
        Ok(value) => (
            json!({
                "content": value.content.clone(),
                "structuredContent": value.structured_content.clone(),
            }),
            Value::Null,
        ),
        Err(message) => (
            Value::Null,
            json!({
                "message": message.clone(),
            }),
        ),
    };

    vec![json!({
        "type": "item.completed",
        "item": {
            "type": "mcp_tool_call",
            "id": event.call_id.clone(),
            "server": event.invocation.server.clone(),
            "tool": event.invocation.tool.clone(),
            "status": if event.is_success() { "completed" } else { "failed" },
            "arguments": arguments,
            "result": result,
            "error": error,
        },
    })]
}

#[cfg(feature = "native-codex-runtime")]
fn map_patch_apply_end(event: &PatchApplyEndEvent) -> Vec<Value> {
    vec![json!({
        "type": "item.completed",
        "item": {
            "type": "file_change",
            "id": event.call_id.clone(),
            "status": legacy_patch_apply_status(&event.status),
            "changes": legacy_patch_changes(&event.changes),
        },
    })]
}

#[cfg(feature = "native-codex-runtime")]
fn map_collab_agent_spawn_end(event: &CollabAgentSpawnEndEvent) -> Vec<Value> {
    let has_receiver = event.new_thread_id.is_some();
    let status = match &event.status {
        AgentStatus::Errored(_) | AgentStatus::NotFound => "failed",
        _ if has_receiver => "completed",
        _ => "failed",
    };
    let (receiver_thread_ids, agents_states) = match &event.new_thread_id {
        Some(thread_id) => {
            let receiver_id = thread_id.to_string();
            let state_pairs = vec![(receiver_id.clone(), &event.status)];
            (
                vec![receiver_id],
                legacy_collab_agents_states_from_pairs(&state_pairs),
            )
        }
        None => (Vec::new(), Value::Object(serde_json::Map::new())),
    };

    let error = match (&event.new_thread_id, &event.status) {
        (None, AgentStatus::Errored(message)) => Some(json!({
            "message": message.clone(),
        })),
        (None, AgentStatus::NotFound) => Some(json!({
            "message": "agent not found",
        })),
        _ => None,
    };

    let mut mapped = map_collab_tool_call_completed(
        &event.call_id,
        "spawn_agent",
        status,
        event.sender_thread_id.to_string(),
        receiver_thread_ids,
        event.prompt.clone(),
        agents_states,
    );
    if let Some(error_payload) = error {
        if let Some(item) = mapped
            .first_mut()
            .and_then(|value| value.get_mut("item"))
            .and_then(Value::as_object_mut)
        {
            item.insert("error".to_string(), error_payload);
        }
    }
    mapped
}

#[cfg(feature = "native-codex-runtime")]
fn map_collab_agent_interaction_end(event: &CollabAgentInteractionEndEvent) -> Vec<Value> {
    let status = legacy_collab_status_from_agent_status(&event.status);
    let receiver_id = event.receiver_thread_id.to_string();
    let state_pairs = vec![(receiver_id.clone(), &event.status)];

    map_collab_tool_call_completed(
        &event.call_id,
        "send_input",
        status,
        event.sender_thread_id.to_string(),
        vec![receiver_id],
        event.prompt.clone(),
        legacy_collab_agents_states_from_pairs(&state_pairs),
    )
}

#[cfg(feature = "native-codex-runtime")]
fn map_collab_waiting_end(event: &CollabWaitingEndEvent) -> Vec<Value> {
    let mut state_pairs: Vec<(String, &AgentStatus)> = event
        .statuses
        .iter()
        .map(|(thread_id, status)| (thread_id.to_string(), status))
        .collect();
    state_pairs.sort_by(|left, right| left.0.cmp(&right.0));

    let status = if state_pairs
        .iter()
        .any(|(_, state)| matches!(state, AgentStatus::Errored(_) | AgentStatus::NotFound))
    {
        "failed"
    } else {
        "completed"
    };
    let receiver_thread_ids = state_pairs
        .iter()
        .map(|(thread_id, _)| thread_id.clone())
        .collect::<Vec<_>>();

    map_collab_tool_call_completed(
        &event.call_id,
        "wait",
        status,
        event.sender_thread_id.to_string(),
        receiver_thread_ids,
        String::new(),
        legacy_collab_agents_states_from_pairs(&state_pairs),
    )
}

#[cfg(feature = "native-codex-runtime")]
fn map_collab_close_end(event: &CollabCloseEndEvent) -> Vec<Value> {
    let status = legacy_collab_status_from_agent_status(&event.status);
    let receiver_id = event.receiver_thread_id.to_string();
    let state_pairs = vec![(receiver_id.clone(), &event.status)];

    map_collab_tool_call_completed(
        &event.call_id,
        "close_agent",
        status,
        event.sender_thread_id.to_string(),
        vec![receiver_id],
        String::new(),
        legacy_collab_agents_states_from_pairs(&state_pairs),
    )
}

#[cfg(feature = "native-codex-runtime")]
fn map_collab_resume_end(event: &CollabResumeEndEvent) -> Vec<Value> {
    let status = legacy_collab_status_from_agent_status(&event.status);
    let receiver_id = event.receiver_thread_id.to_string();
    let state_pairs = vec![(receiver_id.clone(), &event.status)];

    map_collab_tool_call_completed(
        &event.call_id,
        "resume_agent",
        status,
        event.sender_thread_id.to_string(),
        vec![receiver_id],
        String::new(),
        legacy_collab_agents_states_from_pairs(&state_pairs),
    )
}

#[cfg(feature = "native-codex-runtime")]
fn map_collab_tool_call_completed(
    call_id: &str,
    tool: &str,
    status: &str,
    sender_thread_id: String,
    receiver_thread_ids: Vec<String>,
    prompt: String,
    agents_states: Value,
) -> Vec<Value> {
    vec![json!({
        "type": "item.completed",
        "item": {
            "type": "collab_tool_call",
            "id": call_id,
            "tool": tool,
            "status": status,
            "sender_thread_id": sender_thread_id,
            "receiver_thread_ids": receiver_thread_ids,
            "prompt": prompt,
            "agents_states": agents_states,
        },
    })]
}

#[cfg(feature = "native-codex-runtime")]
fn legacy_collab_status_from_agent_status(status: &AgentStatus) -> &'static str {
    match status {
        AgentStatus::Errored(_) | AgentStatus::NotFound => "failed",
        _ => "completed",
    }
}

#[cfg(feature = "native-codex-runtime")]
fn legacy_collab_agents_states_from_pairs(pairs: &[(String, &AgentStatus)]) -> Value {
    let mut payload = serde_json::Map::new();
    for (thread_id, status) in pairs {
        payload.insert(thread_id.clone(), legacy_collab_agent_state(status));
    }
    Value::Object(payload)
}

#[cfg(feature = "native-codex-runtime")]
fn legacy_collab_agent_state(status: &AgentStatus) -> Value {
    match status {
        AgentStatus::PendingInit => json!({
            "status": "pendingInit",
            "message": Value::Null,
        }),
        AgentStatus::Running => json!({
            "status": "running",
            "message": Value::Null,
        }),
        AgentStatus::Completed(message) => json!({
            "status": "completed",
            "message": message,
        }),
        AgentStatus::Errored(message) => json!({
            "status": "errored",
            "message": message,
        }),
        AgentStatus::Shutdown => json!({
            "status": "shutdown",
            "message": Value::Null,
        }),
        AgentStatus::NotFound => json!({
            "status": "notFound",
            "message": Value::Null,
        }),
    }
}

#[cfg(feature = "native-codex-runtime")]
fn map_entered_review_mode(event_id: &str, review_request: &ReviewRequest) -> Vec<Value> {
    let review = review_request
        .user_facing_hint
        .clone()
        .unwrap_or_else(|| codex_core::review_prompts::user_facing_hint(&review_request.target));
    map_review_mode_item_events("entered_review_mode", event_id, review)
}

#[cfg(feature = "native-codex-runtime")]
fn map_exited_review_mode(event_id: &str, review_event: &ExitedReviewModeEvent) -> Vec<Value> {
    let review = review_event
        .review_output
        .as_ref()
        .map(codex_core::review_format::render_review_output_text)
        .unwrap_or_else(|| REVIEW_FALLBACK_MESSAGE.to_string());
    map_review_mode_item_events("exited_review_mode", event_id, review)
}

#[cfg(feature = "native-codex-runtime")]
fn map_review_mode_item_events(item_type: &str, item_id: &str, review: String) -> Vec<Value> {
    vec![
        json!({
            "type": "item.started",
            "item": {
                "type": item_type,
                "id": item_id,
                "review": review,
            },
        }),
        json!({
            "type": "item.completed",
            "item": {
                "type": item_type,
                "id": item_id,
                "review": review,
            },
        }),
    ]
}

#[cfg(feature = "native-codex-runtime")]
fn legacy_patch_changes(changes: &HashMap<std::path::PathBuf, ProtocolFileChange>) -> Vec<Value> {
    let mut converted: Vec<(String, Value)> = changes
        .iter()
        .map(|(path, change)| {
            let path_string = path.to_string_lossy().to_string();
            (
                path_string.clone(),
                json!({
                    "path": path_string,
                    "kind": legacy_patch_change_kind(change),
                    "diff": legacy_patch_change_diff(change),
                }),
            )
        })
        .collect();
    converted.sort_by(|left, right| left.0.cmp(&right.0));
    converted.into_iter().map(|(_, value)| value).collect()
}

#[cfg(feature = "native-codex-runtime")]
fn legacy_patch_change_kind(change: &ProtocolFileChange) -> Value {
    match change {
        ProtocolFileChange::Add { .. } => json!({ "type": "add" }),
        ProtocolFileChange::Delete { .. } => json!({ "type": "delete" }),
        ProtocolFileChange::Update { move_path, .. } => json!({
            "type": "update",
            "movePath": move_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
        }),
    }
}

#[cfg(feature = "native-codex-runtime")]
fn legacy_patch_change_diff(change: &ProtocolFileChange) -> String {
    match change {
        ProtocolFileChange::Add { content } => content.clone(),
        ProtocolFileChange::Delete { content } => content.clone(),
        ProtocolFileChange::Update {
            unified_diff,
            move_path,
        } => move_path
            .as_ref()
            .map(|path| format!("{unified_diff}\n\nMoved to: {}", path.display()))
            .unwrap_or_else(|| unified_diff.clone()),
    }
}

#[cfg(feature = "native-codex-runtime")]
fn turn_item_to_legacy(item: &TurnItem) -> Option<Value> {
    match item {
        TurnItem::AgentMessage(agent) => {
            let text = agent
                .content
                .iter()
                .map(|entry| match entry {
                    AgentMessageContent::Text { text } => text.clone(),
                })
                .collect::<String>();
            Some(json!({
                "type": "agent_message",
                "id": agent.id,
                "text": text,
            }))
        }
        TurnItem::Plan(plan) => Some(json!({
            "type": "reasoning",
            "id": plan.id,
            "text": plan.text,
        })),
        TurnItem::Reasoning(reasoning) => {
            let summary = reasoning.summary_text.join("\n");
            let content = reasoning.raw_content.join("\n");
            let text = match (summary.is_empty(), content.is_empty()) {
                (true, true) => String::new(),
                (false, true) => summary,
                (true, false) => content,
                (false, false) => format!("{summary}\n{content}"),
            };

            Some(json!({
                "type": "reasoning",
                "id": reasoning.id,
                "text": text,
            }))
        }
        TurnItem::WebSearch(search) => Some(json!({
            "type": "web_search",
            "id": search.id,
            "query": search.query,
            "action": search.action,
        })),
        TurnItem::UserMessage(user) => Some(json!({
            "type": "user_message",
            "id": user.id,
            "content": user.content,
        })),
        TurnItem::ContextCompaction(compaction) => Some(json!({
            "type": "context_compaction",
            "id": compaction.id,
        })),
    }
}

#[cfg(feature = "native-codex-runtime")]
fn token_usage_to_legacy_json(info: &codex_core::protocol::TokenUsageInfo) -> Value {
    json!({
        "total": token_usage_breakdown_to_json(&info.total_token_usage),
        "last": token_usage_breakdown_to_json(&info.last_token_usage),
        "model_context_window": info.model_context_window,
    })
}

#[cfg(feature = "native-codex-runtime")]
fn token_usage_breakdown_to_json(usage: &codex_core::protocol::TokenUsage) -> Value {
    json!({
        "total_tokens": usage.total_tokens,
        "input_tokens": usage.input_tokens,
        "cached_input_tokens": usage.cached_input_tokens,
        "output_tokens": usage.output_tokens,
        "reasoning_output_tokens": usage.reasoning_output_tokens,
    })
}

#[cfg(all(test, feature = "native-codex-runtime"))]
mod tests {
    use super::{
        legacy_plan_step_status, map_collab_agent_interaction_end, map_collab_agent_spawn_end,
        map_collab_close_end, map_collab_resume_end, map_collab_waiting_end,
        map_entered_review_mode, map_exec_command_end, map_exited_review_mode,
        map_mcp_tool_call_end, map_patch_apply_end, turn_item_to_legacy,
        NativeCodexEventTranslator,
    };
    use codex_core::protocol::{
        AgentStatus, CollabAgentInteractionEndEvent, CollabAgentSpawnEndEvent, CollabCloseEndEvent,
        CollabResumeEndEvent, CollabWaitingEndEvent, ExecCommandEndEvent, ExecCommandSource,
        ExecCommandStatus, FileChange, McpInvocation, McpToolCallEndEvent, PatchApplyEndEvent,
        PatchApplyStatus, ReviewOutputEvent, ReviewRequest, ReviewTarget,
    };
    use codex_protocol::items::{
        AgentMessageContent, AgentMessageItem, PlanItem, ReasoningItem, TurnItem,
    };
    use codex_protocol::mcp::CallToolResult;
    use codex_protocol::models::{FunctionCallOutputPayload, ResponseItem};
    use codex_protocol::plan_tool::StepStatus;
    use codex_protocol::ThreadId;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn maps_agent_message_item_to_legacy_payload() {
        let item = TurnItem::AgentMessage(AgentMessageItem {
            id: "item-1".to_string(),
            content: vec![AgentMessageContent::Text {
                text: "hello".to_string(),
            }],
            phase: None,
        });

        let mapped = turn_item_to_legacy(&item).expect("mapped item");
        assert_eq!(
            mapped.get("type").and_then(|value| value.as_str()),
            Some("agent_message")
        );
        assert_eq!(
            mapped.get("id").and_then(|value| value.as_str()),
            Some("item-1")
        );
        assert_eq!(
            mapped.get("text").and_then(|value| value.as_str()),
            Some("hello")
        );
    }

    #[test]
    fn maps_plan_item_to_reasoning_legacy_payload() {
        let item = TurnItem::Plan(PlanItem {
            id: "plan-1".to_string(),
            text: "Step A".to_string(),
        });

        let mapped = turn_item_to_legacy(&item).expect("mapped item");
        assert_eq!(
            mapped.get("type").and_then(|value| value.as_str()),
            Some("reasoning")
        );
        assert_eq!(
            mapped.get("text").and_then(|value| value.as_str()),
            Some("Step A")
        );
    }

    #[test]
    fn maps_reasoning_item_to_text_payload() {
        let item = TurnItem::Reasoning(ReasoningItem {
            id: "reason-1".to_string(),
            summary_text: vec!["summary".to_string()],
            raw_content: vec!["raw".to_string()],
        });

        let mapped = turn_item_to_legacy(&item).expect("mapped item");
        assert_eq!(
            mapped.get("type").and_then(|value| value.as_str()),
            Some("reasoning")
        );
        assert_eq!(
            mapped.get("text").and_then(|value| value.as_str()),
            Some("summary\nraw")
        );
    }
    #[test]
    fn maps_plan_step_status_to_bridge_contract() {
        assert_eq!(legacy_plan_step_status(StepStatus::Pending), "pending");
        assert_eq!(
            legacy_plan_step_status(StepStatus::InProgress),
            "inProgress"
        );
        assert_eq!(legacy_plan_step_status(StepStatus::Completed), "completed");
    }

    #[test]
    fn maps_exec_command_end_to_command_execution_item_completed() {
        let event = ExecCommandEndEvent {
            call_id: "exec-1".to_string(),
            process_id: None,
            turn_id: "turn-1".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            cwd: PathBuf::from("."),
            parsed_cmd: Vec::new(),
            source: ExecCommandSource::Agent,
            interaction_input: None,
            stdout: "hello\n".to_string(),
            stderr: String::new(),
            aggregated_output: "hello\n".to_string(),
            exit_code: 0,
            duration: Duration::from_millis(10),
            formatted_output: "hello\n".to_string(),
            status: ExecCommandStatus::Completed,
        };

        let mapped = map_exec_command_end(&event);
        assert_eq!(
            mapped,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "command_execution",
                    "id": "exec-1",
                    "command": "echo hello",
                    "status": "completed",
                    "aggregated_output": "hello\n",
                    "exit_code": 0,
                },
            })]
        );
    }

    #[test]
    fn maps_mcp_tool_call_end_to_mcp_tool_call_item_completed() {
        let event = McpToolCallEndEvent {
            call_id: "mcp-1".to_string(),
            invocation: McpInvocation {
                server: "filesystem".to_string(),
                tool: "read_file".to_string(),
                arguments: Some(serde_json::Value::Null),
            },
            duration: Duration::from_millis(12),
            result: Ok(CallToolResult {
                content: vec![json!({
                    "type": "text",
                    "text": "ok",
                })],
                structured_content: Some(json!({
                    "ok": true,
                })),
                is_error: Some(true),
                meta: None,
            }),
        };

        let mapped = map_mcp_tool_call_end(&event);
        assert_eq!(
            mapped,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "mcp_tool_call",
                    "id": "mcp-1",
                    "server": "filesystem",
                    "tool": "read_file",
                    "status": "failed",
                    "arguments": {},
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": "ok",
                            }
                        ],
                        "structuredContent": {
                            "ok": true,
                        },
                    },
                    "error": null,
                },
            })]
        );
    }

    #[test]
    fn maps_patch_apply_end_to_file_change_item_completed() {
        let mut changes = HashMap::new();
        changes.insert(
            PathBuf::from("b.txt"),
            FileChange::Update {
                unified_diff: "@@ -1 +1 @@\n-old\n+new".to_string(),
                move_path: Some(PathBuf::from("renamed.txt")),
            },
        );
        changes.insert(
            PathBuf::from("a.txt"),
            FileChange::Add {
                content: "new file".to_string(),
            },
        );
        let event = PatchApplyEndEvent {
            call_id: "patch-1".to_string(),
            turn_id: "turn-1".to_string(),
            stdout: String::new(),
            stderr: String::new(),
            success: false,
            changes,
            status: PatchApplyStatus::Declined,
        };

        let mapped = map_patch_apply_end(&event);
        assert_eq!(
            mapped,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "file_change",
                    "id": "patch-1",
                    "status": "declined",
                    "changes": [
                        {
                            "path": "a.txt",
                            "kind": {
                                "type": "add",
                            },
                            "diff": "new file",
                        },
                        {
                            "path": "b.txt",
                            "kind": {
                                "type": "update",
                                "movePath": "renamed.txt",
                            },
                            "diff": "@@ -1 +1 @@\n-old\n+new\n\nMoved to: renamed.txt",
                        }
                    ],
                },
            })]
        );
    }

    #[test]
    fn maps_entered_review_mode_to_started_and_completed_items() {
        let request = ReviewRequest {
            target: ReviewTarget::Custom {
                instructions: "check".to_string(),
            },
            user_facing_hint: Some("Reviewing changes".to_string()),
        };

        let mapped = map_entered_review_mode("turn-42", &request);
        assert_eq!(
            mapped,
            vec![
                json!({
                    "type": "item.started",
                    "item": {
                        "type": "entered_review_mode",
                        "id": "turn-42",
                        "review": "Reviewing changes",
                    },
                }),
                json!({
                    "type": "item.completed",
                    "item": {
                        "type": "entered_review_mode",
                        "id": "turn-42",
                        "review": "Reviewing changes",
                    },
                }),
            ]
        );
    }

    #[test]
    fn maps_exited_review_mode_to_started_and_completed_items() {
        let event = codex_core::protocol::ExitedReviewModeEvent {
            review_output: Some(ReviewOutputEvent {
                findings: Vec::new(),
                overall_correctness: "correct".to_string(),
                overall_explanation: "Looks good".to_string(),
                overall_confidence_score: 0.9,
            }),
        };

        let mapped = map_exited_review_mode("turn-43", &event);
        assert_eq!(
            mapped,
            vec![
                json!({
                    "type": "item.started",
                    "item": {
                        "type": "exited_review_mode",
                        "id": "turn-43",
                        "review": "Looks good",
                    },
                }),
                json!({
                    "type": "item.completed",
                    "item": {
                        "type": "exited_review_mode",
                        "id": "turn-43",
                        "review": "Looks good",
                    },
                }),
            ]
        );
    }

    fn thread_id(value: &str) -> ThreadId {
        ThreadId::from_string(value).expect("valid thread id")
    }

    #[test]
    fn maps_collab_agent_spawn_end_to_collab_tool_call_item_completed() {
        let sender_thread_id = thread_id("11111111-1111-1111-1111-111111111111");
        let receiver_thread_id = thread_id("22222222-2222-2222-2222-222222222222");
        let event = CollabAgentSpawnEndEvent {
            call_id: "collab-call-1".to_string(),
            sender_thread_id,
            new_thread_id: Some(receiver_thread_id),
            prompt: "Investigate".to_string(),
            status: AgentStatus::Running,
        };

        let mapped = map_collab_agent_spawn_end(&event);
        assert_eq!(
            mapped,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "collab-call-1",
                    "tool": "spawn_agent",
                    "status": "completed",
                    "sender_thread_id": "11111111-1111-1111-1111-111111111111",
                    "receiver_thread_ids": ["22222222-2222-2222-2222-222222222222"],
                    "prompt": "Investigate",
                    "agents_states": {
                        "22222222-2222-2222-2222-222222222222": {
                            "status": "running",
                            "message": null,
                        },
                    },
                },
            })]
        );
    }

    #[test]
    fn maps_collab_agent_interaction_end_to_collab_tool_call_item_completed() {
        let sender_thread_id = thread_id("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        let receiver_thread_id = thread_id("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
        let event = CollabAgentInteractionEndEvent {
            call_id: "collab-call-2".to_string(),
            sender_thread_id,
            receiver_thread_id,
            prompt: "Apply patch".to_string(),
            status: AgentStatus::Completed(Some("done".to_string())),
        };

        let mapped = map_collab_agent_interaction_end(&event);
        assert_eq!(
            mapped,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "collab-call-2",
                    "tool": "send_input",
                    "status": "completed",
                    "sender_thread_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                    "receiver_thread_ids": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"],
                    "prompt": "Apply patch",
                    "agents_states": {
                        "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                            "status": "completed",
                            "message": "done",
                        },
                    },
                },
            })]
        );
    }

    #[test]
    fn maps_collab_agent_spawn_end_without_receiver_to_failed_item_with_error() {
        let sender_thread_id = thread_id("15151515-1515-1515-1515-151515151515");
        let event = CollabAgentSpawnEndEvent {
            call_id: "collab-call-6".to_string(),
            sender_thread_id,
            new_thread_id: None,
            prompt: "Investigate".to_string(),
            status: AgentStatus::Errored("spawn failed".to_string()),
        };

        let mapped = map_collab_agent_spawn_end(&event);
        assert_eq!(
            mapped,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "collab-call-6",
                    "tool": "spawn_agent",
                    "status": "failed",
                    "sender_thread_id": "15151515-1515-1515-1515-151515151515",
                    "receiver_thread_ids": [],
                    "prompt": "Investigate",
                    "agents_states": {},
                    "error": {
                        "message": "spawn failed",
                    },
                },
            })]
        );
    }

    #[test]
    fn maps_collab_waiting_end_to_failed_collab_tool_call_when_any_agent_failed() {
        let sender_thread_id = thread_id("cccccccc-cccc-cccc-cccc-cccccccccccc");
        let receiver_thread_id_a = thread_id("dddddddd-dddd-dddd-dddd-dddddddddddd");
        let receiver_thread_id_b = thread_id("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee");
        let mut statuses = HashMap::new();
        statuses.insert(receiver_thread_id_b, AgentStatus::Running);
        statuses.insert(
            receiver_thread_id_a,
            AgentStatus::Errored("timed out".to_string()),
        );
        let event = CollabWaitingEndEvent {
            sender_thread_id,
            call_id: "collab-call-3".to_string(),
            statuses,
        };

        let mapped = map_collab_waiting_end(&event);
        assert_eq!(
            mapped,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "collab-call-3",
                    "tool": "wait",
                    "status": "failed",
                    "sender_thread_id": "cccccccc-cccc-cccc-cccc-cccccccccccc",
                    "receiver_thread_ids": [
                        "dddddddd-dddd-dddd-dddd-dddddddddddd",
                        "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee",
                    ],
                    "prompt": "",
                    "agents_states": {
                        "dddddddd-dddd-dddd-dddd-dddddddddddd": {
                            "status": "errored",
                            "message": "timed out",
                        },
                        "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee": {
                            "status": "running",
                            "message": null,
                        },
                    },
                },
            })]
        );
    }

    #[test]
    fn maps_collab_close_end_to_collab_tool_call_item_completed() {
        let sender_thread_id = thread_id("ffffffff-ffff-ffff-ffff-ffffffffffff");
        let receiver_thread_id = thread_id("12121212-1212-1212-1212-121212121212");
        let event = CollabCloseEndEvent {
            call_id: "collab-call-4".to_string(),
            sender_thread_id,
            receiver_thread_id,
            status: AgentStatus::Shutdown,
        };

        let mapped = map_collab_close_end(&event);
        assert_eq!(
            mapped,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "collab-call-4",
                    "tool": "close_agent",
                    "status": "completed",
                    "sender_thread_id": "ffffffff-ffff-ffff-ffff-ffffffffffff",
                    "receiver_thread_ids": ["12121212-1212-1212-1212-121212121212"],
                    "prompt": "",
                    "agents_states": {
                        "12121212-1212-1212-1212-121212121212": {
                            "status": "shutdown",
                            "message": null,
                        },
                    },
                },
            })]
        );
    }

    #[test]
    fn maps_collab_resume_end_to_collab_tool_call_item_completed() {
        let sender_thread_id = thread_id("13131313-1313-1313-1313-131313131313");
        let receiver_thread_id = thread_id("14141414-1414-1414-1414-141414141414");
        let event = CollabResumeEndEvent {
            call_id: "collab-call-5".to_string(),
            sender_thread_id,
            receiver_thread_id,
            status: AgentStatus::NotFound,
        };

        let mapped = map_collab_resume_end(&event);
        assert_eq!(
            mapped,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "collab-call-5",
                    "tool": "resume_agent",
                    "status": "failed",
                    "sender_thread_id": "13131313-1313-1313-1313-131313131313",
                    "receiver_thread_ids": ["14141414-1414-1414-1414-141414141414"],
                    "prompt": "",
                    "agents_states": {
                        "14141414-1414-1414-1414-141414141414": {
                            "status": "notFound",
                            "message": null,
                        },
                    },
                },
            })]
        );
    }
    #[test]
    fn maps_raw_collab_function_call_and_output_to_item_completed() {
        let mut translator =
            NativeCodexEventTranslator::new("11111111-1111-1111-1111-111111111111".to_string());

        let started = translator.translate_raw_response_item(&ResponseItem::FunctionCall {
            id: None,
            name: "spawn_agent".to_string(),
            arguments: r#"{"message":"Investigate"}"#.to_string(),
            call_id: "raw-collab-1".to_string(),
        });
        assert!(started.is_empty());

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-collab-1".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"agent_id":"22222222-2222-2222-2222-222222222222"}"#.to_string(),
            ),
        });

        assert_eq!(
            completed,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "raw-collab-1",
                    "tool": "spawn_agent",
                    "status": "completed",
                    "sender_thread_id": "11111111-1111-1111-1111-111111111111",
                    "receiver_thread_ids": ["22222222-2222-2222-2222-222222222222"],
                    "prompt": "Investigate",
                    "agents_states": {
                        "22222222-2222-2222-2222-222222222222": {
                            "status": "running",
                            "message": null,
                        }
                    }
                }
            })]
        );
    }

    #[test]
    fn infers_spawn_agent_from_raw_output_without_pending_cache() {
        let mut translator =
            NativeCodexEventTranslator::new("aaaaaaaa-1111-1111-1111-111111111111".to_string());

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-infer-1".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"agent_id":"bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"}"#.to_string(),
            ),
        });

        assert_eq!(
            completed,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "raw-infer-1",
                    "tool": "spawn_agent",
                    "status": "completed",
                    "sender_thread_id": "aaaaaaaa-1111-1111-1111-111111111111",
                    "receiver_thread_ids": ["bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"],
                    "prompt": "",
                    "agents_states": {
                        "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb": {
                            "status": "running",
                            "message": null,
                        }
                    }
                }
            })]
        );
    }

    #[test]
    fn infers_wait_from_raw_output_without_pending_cache_when_statuses_parse() {
        let mut translator =
            NativeCodexEventTranslator::new("cccccccc-1111-1111-1111-111111111111".to_string());

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-infer-2".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"status":{"dddddddd-dddd-dddd-dddd-dddddddddddd":"running"}}"#.to_string(),
            ),
        });

        assert_eq!(
            completed,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "raw-infer-2",
                    "tool": "wait",
                    "status": "completed",
                    "sender_thread_id": "cccccccc-1111-1111-1111-111111111111",
                    "receiver_thread_ids": ["dddddddd-dddd-dddd-dddd-dddddddddddd"],
                    "prompt": "",
                    "agents_states": {
                        "dddddddd-dddd-dddd-dddd-dddddddddddd": {
                            "status": "running",
                            "message": null,
                        }
                    }
                }
            })]
        );
    }

    #[test]
    fn does_not_infer_tool_from_submission_output_without_pending_cache() {
        let mut translator =
            NativeCodexEventTranslator::new("eeeeeeee-1111-1111-1111-111111111111".to_string());

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-infer-3".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"submission_id":"submission-1"}"#.to_string(),
            ),
        });

        assert!(completed.is_empty());
    }

    #[test]
    fn does_not_infer_wait_from_unparseable_statuses_without_pending_cache() {
        let mut translator =
            NativeCodexEventTranslator::new("ffffffff-1111-1111-1111-111111111111".to_string());

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-infer-4".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"status":{"dddddddd-dddd-dddd-dddd-dddddddddddd":"unknown_state"}}"#
                    .to_string(),
            ),
        });

        assert!(completed.is_empty());
    }

    #[test]
    fn does_not_infer_wait_from_timed_out_with_empty_status_without_pending_cache() {
        let mut translator =
            NativeCodexEventTranslator::new("12121212-1111-1111-1111-111111111111".to_string());

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-infer-5".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"status":{},"timed_out":true}"#.to_string(),
            ),
        });

        assert!(completed.is_empty());
    }

    #[test]
    fn does_not_infer_spawn_agent_from_invalid_agent_id_without_pending_cache() {
        let mut translator =
            NativeCodexEventTranslator::new("14141414-1111-1111-1111-111111111111".to_string());

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-infer-7".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"agent_id":"not-a-thread-id"}"#.to_string(),
            ),
        });

        assert!(completed.is_empty());
    }

    #[test]
    fn does_not_infer_wait_from_invalid_thread_ids_without_pending_cache() {
        let mut translator =
            NativeCodexEventTranslator::new("15151515-1111-1111-1111-111111111111".to_string());

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-infer-8".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"status":{"not-a-thread-id":"running"}}"#.to_string(),
            ),
        });

        assert!(completed.is_empty());
    }

    #[test]
    fn deduplicates_inferred_raw_output_by_call_id() {
        let mut translator =
            NativeCodexEventTranslator::new("13131313-1111-1111-1111-111111111111".to_string());
        let output = ResponseItem::FunctionCallOutput {
            call_id: "raw-infer-6".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"agent_id":"bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"}"#.to_string(),
            ),
        };

        let first = translator.translate_raw_response_item(&output);
        assert_eq!(first.len(), 1);
        let second = translator.translate_raw_response_item(&output);
        assert!(second.is_empty());
    }

    #[test]
    fn deduplicates_collab_end_after_terminal_raw_inference_for_same_call_id() {
        let mut translator =
            NativeCodexEventTranslator::new("18181818-1111-1111-1111-111111111111".to_string());
        let raw_output = ResponseItem::FunctionCallOutput {
            call_id: "raw-collab-dedupe-1".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"agent_id":"19191919-1919-1919-1919-191919191919"}"#.to_string(),
            ),
        };
        let raw_completed = translator.translate_raw_response_item(&raw_output);
        assert_eq!(raw_completed.len(), 1);

        let collab_event = CollabAgentSpawnEndEvent {
            call_id: "raw-collab-dedupe-1".to_string(),
            sender_thread_id: thread_id("18181818-1111-1111-1111-111111111111"),
            new_thread_id: Some(thread_id("19191919-1919-1919-1919-191919191919")),
            prompt: String::new(),
            status: AgentStatus::Running,
        };
        let collab_completed = translator.translate_collab_agent_spawn_end(&collab_event);
        assert!(collab_completed.is_empty());
    }

    #[test]
    fn deduplicates_terminal_raw_inference_after_collab_end_for_same_call_id() {
        let mut translator =
            NativeCodexEventTranslator::new("20202020-1111-1111-1111-111111111111".to_string());
        let collab_event = CollabAgentSpawnEndEvent {
            call_id: "raw-collab-dedupe-2".to_string(),
            sender_thread_id: thread_id("20202020-1111-1111-1111-111111111111"),
            new_thread_id: Some(thread_id("21212121-2121-2121-2121-212121212121")),
            prompt: String::new(),
            status: AgentStatus::Running,
        };
        let collab_completed = translator.translate_collab_agent_spawn_end(&collab_event);
        assert_eq!(collab_completed.len(), 1);

        let raw_output = ResponseItem::FunctionCallOutput {
            call_id: "raw-collab-dedupe-2".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"agent_id":"21212121-2121-2121-2121-212121212121"}"#.to_string(),
            ),
        };
        let raw_completed = translator.translate_raw_response_item(&raw_output);
        assert!(raw_completed.is_empty());
    }

    #[test]
    fn in_progress_raw_wait_does_not_block_follow_up_collab_waiting_end() {
        let mut translator =
            NativeCodexEventTranslator::new("16161616-1111-1111-1111-111111111111".to_string());

        let started = translator.translate_raw_response_item(&ResponseItem::FunctionCall {
            id: None,
            name: "wait".to_string(),
            arguments: r#"{"ids":["17171717-1717-1717-1717-171717171717"]}"#.to_string(),
            call_id: "raw-collab-7".to_string(),
        });
        assert!(started.is_empty());

        let raw_progress =
            translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
                call_id: "raw-collab-7".to_string(),
                output: FunctionCallOutputPayload::from_text(
                    r#"{"status":{},"timed_out":true}"#.to_string(),
                ),
            });
        assert_eq!(
            raw_progress
                .first()
                .and_then(|event| event.get("item"))
                .and_then(|item| item.get("status"))
                .and_then(Value::as_str),
            Some("in_progress")
        );

        let sender_thread_id = thread_id("16161616-1111-1111-1111-111111111111");
        let receiver_thread_id = thread_id("17171717-1717-1717-1717-171717171717");
        let mut statuses = HashMap::new();
        statuses.insert(receiver_thread_id, AgentStatus::Completed(None));
        let collab_event = CollabWaitingEndEvent {
            sender_thread_id,
            call_id: "raw-collab-7".to_string(),
            statuses,
        };

        let structured_completion = translator.translate_collab_waiting_end(&collab_event);
        assert_eq!(
            structured_completion
                .first()
                .and_then(|event| event.get("item"))
                .and_then(|item| item.get("status"))
                .and_then(Value::as_str),
            Some("completed")
        );
    }

    #[test]
    fn skips_raw_output_when_call_id_is_already_completed() {
        let mut translator =
            NativeCodexEventTranslator::new("33333333-3333-3333-3333-333333333333".to_string());

        let started = translator.translate_raw_response_item(&ResponseItem::FunctionCall {
            id: None,
            name: "send_input".to_string(),
            arguments: r#"{"id":"44444444-4444-4444-4444-444444444444","message":"Ping"}"#
                .to_string(),
            call_id: "raw-collab-2".to_string(),
        });
        assert!(started.is_empty());

        translator.mark_collab_call_completed("raw-collab-2");

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-collab-2".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"submission_id":"submission-1"}"#.to_string(),
            ),
        });

        assert!(completed.is_empty());
    }

    #[test]
    fn ignores_malformed_raw_collab_arguments_and_output_without_panicking() {
        let mut translator =
            NativeCodexEventTranslator::new("55555555-5555-5555-5555-555555555555".to_string());

        let malformed_arguments =
            translator.translate_raw_response_item(&ResponseItem::FunctionCall {
                id: None,
                name: "spawn_agent".to_string(),
                arguments: "{".to_string(),
                call_id: "raw-collab-3".to_string(),
            });
        assert!(malformed_arguments.is_empty());

        let missing_pending =
            translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
                call_id: "raw-collab-3".to_string(),
                output: FunctionCallOutputPayload::from_text("not-json".to_string()),
            });
        assert!(missing_pending.is_empty());

        let started = translator.translate_raw_response_item(&ResponseItem::FunctionCall {
            id: None,
            name: "send_input".to_string(),
            arguments: r#"{"id":"66666666-6666-6666-6666-666666666666","message":"Ping"}"#
                .to_string(),
            call_id: "raw-collab-4".to_string(),
        });
        assert!(started.is_empty());

        let malformed_output =
            translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
                call_id: "raw-collab-4".to_string(),
                output: FunctionCallOutputPayload::from_text("not-json".to_string()),
            });
        assert!(malformed_output.is_empty());
    }

    #[test]
    fn raw_wait_timeout_without_statuses_remains_in_progress() {
        let mut translator =
            NativeCodexEventTranslator::new("77777777-7777-7777-7777-777777777777".to_string());

        let started = translator.translate_raw_response_item(&ResponseItem::FunctionCall {
            id: None,
            name: "wait".to_string(),
            arguments: r#"{"ids":["88888888-8888-8888-8888-888888888888"]}"#.to_string(),
            call_id: "raw-collab-5".to_string(),
        });
        assert!(started.is_empty());

        let completed = translator.translate_raw_response_item(&ResponseItem::FunctionCallOutput {
            call_id: "raw-collab-5".to_string(),
            output: FunctionCallOutputPayload::from_text(
                r#"{"status":{},"timed_out":true}"#.to_string(),
            ),
        });

        assert_eq!(
            completed,
            vec![json!({
                "type": "item.completed",
                "item": {
                    "type": "collab_tool_call",
                    "id": "raw-collab-5",
                    "tool": "wait",
                    "status": "in_progress",
                    "sender_thread_id": "77777777-7777-7777-7777-777777777777",
                    "receiver_thread_ids": ["88888888-8888-8888-8888-888888888888"],
                    "prompt": "",
                    "agents_states": {},
                }
            })]
        );
    }

    #[test]
    fn collab_end_translation_is_deduplicated_by_call_id() {
        let mut translator =
            NativeCodexEventTranslator::new("99999999-9999-9999-9999-999999999999".to_string());
        let sender_thread_id = thread_id("99999999-9999-9999-9999-999999999999");
        let receiver_thread_id = thread_id("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        let event = CollabAgentSpawnEndEvent {
            call_id: "collab-dedupe-1".to_string(),
            sender_thread_id,
            new_thread_id: Some(receiver_thread_id),
            prompt: "Do it".to_string(),
            status: AgentStatus::Running,
        };

        let first = translator.translate_collab_agent_spawn_end(&event);
        assert_eq!(first.len(), 1);

        let second = translator.translate_collab_agent_spawn_end(&event);
        assert!(second.is_empty());
    }
}
