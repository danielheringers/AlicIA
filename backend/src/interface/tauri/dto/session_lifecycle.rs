use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexSessionConfig {
    pub(crate) binary: Option<String>,
    pub(crate) args: Option<Vec<String>>,
    pub(crate) cwd: Option<String>,
    pub(crate) env: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexSessionResponse {
    pub(crate) session_id: u64,
    pub(crate) pid: u32,
}
