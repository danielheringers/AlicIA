use std::future::Future;
use std::pin::Pin;

use crate::domain::neuro_adt::error::NeuroAdtError;
use crate::domain::neuro_adt::types::AdtServerStore;
use crate::AppState;

pub(crate) type NeuroAdtFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone)]
pub(crate) struct AdtServerConnectivity {
    pub(crate) selected_server_id: Option<String>,
    pub(crate) connected: bool,
    pub(crate) message: Option<String>,
}

pub(crate) trait NeuroAdtPort: Send + Sync {
    fn load_server_store(&self) -> Result<AdtServerStore, NeuroAdtError>;
    fn save_server_store(&self, store: &AdtServerStore) -> Result<(), NeuroAdtError>;
    fn normalize_optional_server_id(&self, server_id: Option<String>) -> Option<String>;
    fn env_default_server_id(&self) -> &'static str;

    fn clear_runtime_cache<'a>(&'a self, state: &'a AppState) -> NeuroAdtFuture<'a, ()>;

    fn connect_server<'a>(
        &'a self,
        state: &'a AppState,
        server_id: Option<&'a str>,
    ) -> NeuroAdtFuture<'a, Result<AdtServerConnectivity, NeuroAdtError>>;
}
