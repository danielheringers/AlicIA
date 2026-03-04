use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexReasoningEffortOption {
    pub(crate) reasoning_effort: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexModel {
    pub(crate) id: String,
    pub(crate) model: String,
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) supported_reasoning_efforts: Vec<CodexReasoningEffortOption>,
    pub(crate) default_reasoning_effort: String,
    pub(crate) supports_personality: bool,
    pub(crate) is_default: bool,
    pub(crate) upgrade: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexModelListResponse {
    pub(crate) data: Vec<CodexModel>,
}
