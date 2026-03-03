use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexInputItem {
    #[serde(rename = "type")]
    pub(crate) item_type: String,
    pub(crate) text: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) image_url: Option<String>,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexTurnRunRequest {
    pub(crate) thread_id: Option<String>,
    pub(crate) input_items: Vec<CodexInputItem>,
    pub(crate) output_schema: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexTurnRunResponse {
    pub(crate) accepted: bool,
    pub(crate) session_id: u64,
    pub(crate) thread_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadOpenResponse {
    pub(crate) thread_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadCloseRequest {
    pub(crate) thread_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadCloseResponse {
    pub(crate) thread_id: String,
    pub(crate) removed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadTurnHistoryMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadTurnSummary {
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) item_count: usize,
    #[serde(default)]
    pub(crate) messages: Vec<CodexThreadTurnHistoryMessage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadSummary {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) codex_thread_id: Option<String>,
    pub(crate) preview: String,
    pub(crate) model_provider: String,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    pub(crate) cwd: String,
    pub(crate) path: Option<String>,
    pub(crate) source: String,
    pub(crate) turn_count: usize,
    #[serde(default)]
    pub(crate) turns: Vec<CodexThreadTurnSummary>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadListRequest {
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<u32>,
    pub(crate) sort_key: Option<String>,
    pub(crate) model_providers: Option<Vec<String>>,
    pub(crate) source_kinds: Option<Vec<String>>,
    pub(crate) archived: Option<bool>,
    pub(crate) cwd: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadListResponse {
    pub(crate) data: Vec<CodexThreadSummary>,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadReadRequest {
    pub(crate) thread_id: String,
    pub(crate) include_turns: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadReadResponse {
    pub(crate) thread: CodexThreadSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadArchiveRequest {
    pub(crate) thread_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadArchiveResponse {
    pub(crate) id: String,
    pub(crate) codex_thread_id: String,
    pub(crate) archived: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadUnarchiveRequest {
    pub(crate) thread_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadUnarchiveResponse {
    pub(crate) thread: CodexThreadSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadCompactStartRequest {
    pub(crate) thread_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadCompactStartResponse {
    pub(crate) ok: bool,
    pub(crate) thread_id: String,
    pub(crate) codex_thread_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadRollbackRequest {
    pub(crate) thread_id: String,
    pub(crate) num_turns: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadRollbackResponse {
    pub(crate) thread: CodexThreadSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadForkRequest {
    pub(crate) thread_id: String,
    pub(crate) path: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) model_provider: Option<String>,
    pub(crate) cwd: Option<String>,
    pub(crate) approval_policy: Option<String>,
    pub(crate) sandbox: Option<String>,
    pub(crate) config: Option<Value>,
    pub(crate) base_instructions: Option<String>,
    pub(crate) developer_instructions: Option<String>,
    pub(crate) persist_extended_history: Option<bool>,
    pub(crate) new_thread_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexThreadForkResponse {
    pub(crate) thread: CodexThreadSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexTurnSteerRequest {
    pub(crate) thread_id: String,
    pub(crate) input_items: Vec<CodexInputItem>,
    pub(crate) expected_turn_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexTurnSteerResponse {
    pub(crate) thread_id: String,
    pub(crate) codex_thread_id: String,
    pub(crate) turn_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexTurnInterruptRequest {
    pub(crate) thread_id: String,
    pub(crate) turn_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexTurnInterruptResponse {
    pub(crate) ok: bool,
    pub(crate) thread_id: String,
    pub(crate) codex_thread_id: String,
    pub(crate) turn_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexReviewStartRequest {
    pub(crate) thread_id: Option<String>,
    pub(crate) target: Option<Value>,
    pub(crate) delivery: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexReviewStartResponse {
    pub(crate) accepted: bool,
    pub(crate) session_id: u64,
    pub(crate) thread_id: Option<String>,
    pub(crate) review_thread_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexApprovalRespondRequest {
    pub(crate) action_id: String,
    pub(crate) decision: String,
    pub(crate) remember: Option<bool>,
    pub(crate) execpolicy_amendment: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexUserInputRespondRequest {
    #[serde(alias = "action_id")]
    pub(crate) action_id: String,
    pub(crate) decision: String,
    #[serde(default)]
    pub(crate) answers: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexUserInputRespondResponse {
    pub(crate) ok: bool,
    pub(crate) action_id: String,
    pub(crate) decision: String,
}
