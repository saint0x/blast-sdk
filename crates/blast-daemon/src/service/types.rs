use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc, Mutex as TokioMutex};
use blast_core::EnvironmentManager as CoreEnvironmentManager;

use crate::{
    monitor::MonitorEvent,
    update::UpdateRequest,
};

/// Common service state type
pub type ServiceState = Arc<TokioMutex<super::UpdateServiceState>>;

/// Debug wrapper for EnvironmentManager trait object
pub struct DebugEnvironmentManager(pub Arc<dyn CoreEnvironmentManager + Send + Sync>);

impl Clone for DebugEnvironmentManager {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl std::fmt::Debug for DebugEnvironmentManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvironmentManager").finish()
    }
}

/// Service channels
#[derive(Debug)]
pub struct ServiceChannels {
    /// Channel for sending monitor events
    pub monitor_tx: mpsc::Sender<MonitorEvent>,
    /// Channel for receiving monitor events
    pub monitor_rx: mpsc::Receiver<MonitorEvent>,
    /// Channel for sending update requests
    pub update_tx: Option<mpsc::Sender<UpdateRequest>>,
    /// Channel for receiving update requests
    pub update_rx: Option<mpsc::Receiver<UpdateRequest>>,
    /// Channel for sending shutdown signals
    pub shutdown_tx: Option<broadcast::Sender<()>>,
    /// Channel for receiving shutdown signals
    pub shutdown_rx: Option<broadcast::Receiver<()>>,
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Environment path
    pub env_path: PathBuf,
    /// Cache path
    pub cache_path: PathBuf,
    /// Environment manager
    pub environment_manager: DebugEnvironmentManager,
} 