use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalCreateRequest {
    pub(crate) cwd: Option<String>,
    pub(crate) shell: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalCreateResponse {
    pub(crate) terminal_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalWriteRequest {
    pub(crate) terminal_id: u64,
    pub(crate) data: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalResizeRequest {
    pub(crate) terminal_id: u64,
    pub(crate) cols: u16,
    pub(crate) rows: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminalKillRequest {
    pub(crate) terminal_id: u64,
}
