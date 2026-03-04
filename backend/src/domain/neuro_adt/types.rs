use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const ENV_DEFAULT_SERVER_ID: &str = "env_default";
pub(crate) const DEFAULT_ADT_SERVER_STORE_RELATIVE_PATH: &str = "alicia/neuro/adt_servers.json";
pub(crate) const NEURO_ADT_SERVER_STORE_PATH_ENV: &str = "NEURO_ADT_SERVER_STORE_PATH";

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdtFavoritePackageKind {
    Namespace,
    Package,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdtFavoritePackage {
    pub kind: AdtFavoritePackageKind,
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdtFavoriteObject {
    pub uri: String,
    pub name: String,
    #[serde(default)]
    pub object_type: Option<String>,
    #[serde(default)]
    pub package: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct AdtServerStore {
    #[serde(default)]
    pub(crate) servers: Vec<StoredAdtServer>,
    #[serde(default)]
    pub(crate) explorer_state_by_server: BTreeMap<String, StoredAdtExplorerState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredAdtServer {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) client: Option<String>,
    #[serde(default)]
    pub(crate) language: Option<String>,
    #[serde(default)]
    pub(crate) username: Option<String>,
    #[serde(default)]
    pub(crate) password: Option<String>,
    #[serde(default = "default_verify_tls")]
    pub(crate) verify_tls: bool,
    #[serde(default)]
    pub(crate) active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct StoredAdtExplorerState {
    #[serde(default)]
    pub(crate) favorite_packages: Vec<AdtFavoritePackage>,
    #[serde(default)]
    pub(crate) favorite_objects: Vec<AdtFavoriteObject>,
    #[serde(default)]
    pub(crate) selected_work_package: Option<String>,
    #[serde(default)]
    pub(crate) package_scope_roots: Vec<String>,
    #[serde(default)]
    pub(crate) focused_object_uri: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AdtServerUpsertInput {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) base_url: String,
    pub(crate) client: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) verify_tls: Option<bool>,
    pub(crate) active: Option<bool>,
}

pub(crate) fn default_verify_tls() -> bool {
    true
}

#[cfg(test)]
pub(crate) fn shared_env_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}
