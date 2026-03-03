use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeCodexConfig {
    pub(crate) model: String,
    pub(crate) reasoning: String,
    pub(crate) approval_preset: String,
    pub(crate) approval_policy: String,
    pub(crate) sandbox: String,
    pub(crate) profile: String,
    pub(crate) web_search_mode: String,
}

impl Default for RuntimeCodexConfig {
    fn default() -> Self {
        Self {
            model: "default".to_string(),
            reasoning: "default".to_string(),
            approval_preset: "auto".to_string(),
            approval_policy: "on-request".to_string(),
            sandbox: "read-only".to_string(),
            profile: "read_write_with_approval".to_string(),
            web_search_mode: "cached".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeStatusResponse {
    pub(crate) session_id: Option<u64>,
    pub(crate) pid: Option<u32>,
    pub(crate) workspace: String,
    pub(crate) runtime_config: RuntimeCodexConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeCapabilitiesResponse {
    pub(crate) methods: HashMap<String, bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) contract: Option<RuntimeContractMetadata>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeContractMetadata {
    pub(crate) version: String,
}
