//! Update request types and handling

use blast_core::{
    package::Package,
    python::PythonVersion,
};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, debug, error};
use std::path::PathBuf;
use std::sync::Arc;
use crate::service::{DaemonResult, DaemonError};
use crate::monitor::{MonitorEvent, PythonResourceMonitor, PythonResourceLimits};
use crate::state::{StateManager, StateManagement, State};

/// Update manager for Python environments
pub struct UpdateManager {
    /// Python resource monitor
    monitor: PythonResourceMonitor,
    /// Environment path
    env_path: PathBuf,
    /// Cache path
    cache_path: PathBuf,
    /// Channel for receiving monitor events
    monitor_rx: mpsc::Receiver<MonitorEvent>,
    /// State manager
    state_manager: Arc<RwLock<StateManager>>,
}

impl std::fmt::Debug for UpdateManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateManager")
            .field("monitor", &self.monitor)
            .field("env_path", &self.env_path)
            .field("cache_path", &self.cache_path)
            .field("state_manager", &self.state_manager)
            // Skip monitor_rx since it doesn't implement Debug
            .finish()
    }
}

impl UpdateManager {
    /// Create a new update manager
    pub fn new(env_path: PathBuf, cache_path: PathBuf, monitor_rx: mpsc::Receiver<MonitorEvent>) -> Self {
        debug!("Creating new UpdateManager");
        debug!("Environment path: {}", env_path.display());
        debug!("Cache path: {}", cache_path.display());
        
        Self {
            monitor: PythonResourceMonitor::new(
                env_path.clone(),
                cache_path.clone(),
                PythonResourceLimits::default(),
            ),
            env_path: env_path.clone(),
            cache_path,
            monitor_rx,
            state_manager: Arc::new(RwLock::new(StateManager::new(env_path))),
        }
    }

    /// Start processing updates
    pub async fn run(&mut self) -> DaemonResult<()> {
        info!("Starting update manager");

        // Verify paths exist
        if !self.env_path.exists() {
            debug!("Creating environment directory: {}", self.env_path.display());
            tokio::fs::create_dir_all(&self.env_path).await.map_err(|e| {
                error!("Failed to create environment directory: {}", e);
                DaemonError::environment(format!("Failed to create environment directory: {}", e))
            })?;
        }

        if !self.cache_path.exists() {
            debug!("Creating cache directory: {}", self.cache_path.display());
            tokio::fs::create_dir_all(&self.cache_path).await.map_err(|e| {
                error!("Failed to create cache directory: {}", e);
                DaemonError::environment(format!("Failed to create cache directory: {}", e))
            })?;
        }

        // Perform initial resource check
        self.handle_resource_check().await?;

        // Main event loop
        while let Some(event) = self.monitor_rx.recv().await {
            match event {
                MonitorEvent::ResourceCheck => {
                    if let Err(e) = self.handle_resource_check().await {
                        error!("Failed to handle resource check: {}", e);
                    }
                }
                MonitorEvent::StopMonitoring { env_path } => {
                    info!("Stopping monitoring for environment: {}", env_path.display());
                    break;
                }
                MonitorEvent::PackageChanged => {
                    if let Err(e) = self.sync_environment_state().await {
                        error!("Failed to sync environment state: {}", e);
                    }
                }
                MonitorEvent::FileChanged(path) => {
                    if path.starts_with(&self.env_path) && 
                       path.starts_with(self.env_path.join("lib").join("python3").join("site-packages")) {
                        if let Err(e) = self.sync_environment_state().await {
                            error!("Failed to sync environment state: {}", e);
                        }
                    }
                }
                MonitorEvent::ResourceUpdate(usage) => {
                    info!(
                        "Resource update - Env Size: {} MB, Cache Size: {} MB, Packages: {}",
                        usage.env_disk_usage.total_size / 1_048_576,
                        usage.cache_usage.total_size / 1_048_576,
                        usage.cache_usage.package_count
                    );
                }
            }
        }

        info!("Update manager shutting down");
        Ok(())
    }

    /// Sync environment state with daemon
    async fn sync_environment_state(&self) -> DaemonResult<()> {
        // Create new state
        let state = State {
            active_env_name: Some("default".to_string()),
            active_python_version: Some(PythonVersion::parse("3.8.0").unwrap()),
            ..State::default()
        };

        // Update state manager
        let state_manager = self.state_manager.clone();
        let state_guard = state_manager.write().await;
        state_guard.update_current_state(state).await?;

        Ok(())
    }

    /// Check environment state
    async fn handle_resource_check(&self) -> DaemonResult<()> {
        // Check environment structure
        let required_dirs = ["bin", "lib", "include"];
        for dir in required_dirs {
            let path = self.env_path.join(dir);
            if !path.exists() {
                return Err(DaemonError::Service(
                    format!("Required directory not found: {}", path.display())
                ));
            }
        }

        Ok(())
    }
}

/// Update request types
#[derive(Debug)]
pub enum UpdateType {
    /// Package installation
    PackageInstall(Package),
    /// Package removal
    PackageRemove(Package),
    /// Environment sync
    EnvironmentSync,
    /// Package update
    PackageUpdate {
        package: Package,
        force: bool,
        update_deps: bool,
    }
}

/// Update request structure
#[derive(Debug)]
pub struct UpdateRequest {
    /// Request ID
    pub id: String,
    /// Update type
    pub update_type: UpdateType,
}

impl UpdateRequest {
    /// Create a new package update request
    pub fn new_update(package: Package, force: bool, update_deps: bool) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            update_type: UpdateType::PackageUpdate {
                package,
                force,
                update_deps,
            },
        }
    }

    /// Create a new package install request
    pub fn new_install(package: Package) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            update_type: UpdateType::PackageInstall(package),
        }
    }

    /// Create a new package remove request
    pub fn new_remove(package: Package) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            update_type: UpdateType::PackageRemove(package),
        }
    }

    /// Create a new environment sync request
    pub fn new_sync() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            update_type: UpdateType::EnvironmentSync,
        }
    }
} 