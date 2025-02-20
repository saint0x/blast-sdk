use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tracing::info;
use blast_core::python::PythonVersion;
use blast_core::EnvironmentManager as CoreEnvironmentManager;

use crate::{
    error::{DaemonError, DaemonResult},
    monitor::{MonitorEvent, EnvironmentUsage},
    update::UpdateRequest,
    state::{Checkpoint, State},
    environment::EnvManager,
};

use super::{ServiceConfig, UpdateServiceState, EnvironmentState, DebugEnvironmentManager};

/// Daemon service for handling updates and monitoring
#[derive(Debug)]
pub struct DaemonService {
    /// Channel for sending monitor events
    monitor_tx: mpsc::Sender<MonitorEvent>,
    /// Update service state
    state: Arc<TokioMutex<UpdateServiceState>>,
    /// Channel for sending update requests
    update_tx: Option<mpsc::Sender<UpdateRequest>>,
    /// Channel for sending shutdown signals
    shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,
}

impl DaemonService {
    /// Create a new daemon service
    pub fn new(monitor_tx: mpsc::Sender<MonitorEvent>) -> DaemonResult<Self> {
        let env_path = PathBuf::from("environments/default");
        let cache_path = PathBuf::from("cache");
        
        let state = Arc::new(TokioMutex::new(UpdateServiceState::new(ServiceConfig {
            env_path: env_path.clone(),
            cache_path,
            environment_manager: DebugEnvironmentManager(Arc::new(EnvManager::new(env_path)) as Arc<dyn CoreEnvironmentManager + Send + Sync>),
        })));
        
        Ok(Self {
            monitor_tx,
            state,
            update_tx: None,
            shutdown_tx: None,
        })
    }

    /// Start the daemon service
    pub async fn start(&mut self) -> DaemonResult<()> {
        info!("Starting daemon service");
        
        // Create channels for update service
        let (update_tx, _) = mpsc::channel(100);
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        
        self.update_tx = Some(update_tx);
        self.shutdown_tx = Some(shutdown_tx);
        
        Ok(())
    }

    /// Stop the daemon service
    pub async fn stop(&mut self) -> DaemonResult<()> {
        info!("Stopping daemon service");
        
        // Send shutdown signal
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        
        // Clear channels
        self.update_tx = None;
        
        Ok(())
    }

    /// Notify about resource usage
    pub async fn notify_resource_usage(&self, usage: EnvironmentUsage) -> DaemonResult<()> {
        self.monitor_tx.send(MonitorEvent::ResourceUpdate(usage))
            .await
            .map_err(|e| DaemonError::monitor(format!("Failed to send resource update: {}", e)))?;
        Ok(())
    }

    /// Notify about package changes
    pub async fn notify_package_change(&self) -> DaemonResult<()> {
        self.monitor_tx.send(MonitorEvent::PackageChanged)
            .await
            .map_err(|e| DaemonError::monitor(format!("Failed to send package change: {}", e)))?;
        Ok(())
    }

    /// Send an update request
    pub async fn send_update_request(&self, request: UpdateRequest) -> DaemonResult<()> {
        if let Some(tx) = &self.update_tx {
            tx.send(request)
                .await
                .map_err(|_| DaemonError::service("Update channel closed".to_string()))?;
            Ok(())
        } else {
            Err(DaemonError::service("Service not started".to_string()))
        }
    }

    pub async fn cleanup_old_snapshots(&self, max_age_days: u64) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.cleanup_old_snapshots(max_age_days).await
    }

    pub async fn get_current_state(&self) -> DaemonResult<EnvironmentState> {
        let state = self.state.lock().await;
        state.get_current_state().await
    }

    pub async fn list_checkpoints(&self) -> DaemonResult<Vec<Checkpoint>> {
        let state = self.state.lock().await;
        let current_state = state.get_current_state().await?;
        let checkpoints = state.list_checkpoints(&current_state).await?;
        Ok(checkpoints.into_iter().map(|id| Checkpoint {
            id: id.clone(),
            description: format!("Checkpoint {}", id),
            timestamp: chrono::Utc::now(),
            state: State::default(), // Initialize with default state since we don't have the actual state
            metadata: None,
        }).collect())
    }

    pub async fn get_checkpoint(&self, id: &str) -> DaemonResult<Option<Checkpoint>> {
        let state = self.state.lock().await;
        let current_state = state.get_current_state().await?;
        if let Ok(_checkpoint_state) = state.get_checkpoint(&current_state, id).await {
            Ok(Some(Checkpoint {
                id: id.to_string(),
                description: format!("Checkpoint {}", id),
                timestamp: chrono::Utc::now(),
                state: State::default(), // Initialize with default state since we don't have the actual state
                metadata: None,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn create_environment(&self, _name: String, python_version: PythonVersion) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.create_environment(python_version).await
    }

    pub async fn remove_environment(&self, _name: &str) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.remove_environment().await
    }

    pub async fn activate_environment(&self, _name: String) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.activate_environment().await
    }

    pub async fn deactivate_environment(&self) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.deactivate_environment().await
    }
} 