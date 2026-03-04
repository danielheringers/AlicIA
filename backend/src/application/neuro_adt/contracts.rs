use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct AdtServerUpsertRequest {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub verify_tls: Option<bool>,
    #[serde(default)]
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtServerRecord {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    pub verify_tls: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtServerListResponse {
    pub servers: Vec<AdtServerRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_server_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtServerSelectResponse {
    pub selected_server_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtServerRemoveResponse {
    pub removed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_server_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdtServerConnectResponse {
    pub server_id: String,
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
