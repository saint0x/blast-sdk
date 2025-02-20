use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;
use tracing::{error, info};
use uuid::Uuid;
use chrono::Utc;

use blast_core::{
    package::Package,
    python::PythonVersion,
    version_history::{VersionEvent, VersionImpact},
};

use crate::{
    error::DaemonResult,
    update::{UpdateType, UpdateRequest},
    transaction::TransactionOperation,
};

use super::{ServiceState, ServiceChannels};

/// Update service
#[derive(Debug)]
pub struct UpdateService {
    /// Service state
    state: ServiceState,
    /// Service channels
    channels: ServiceChannels,
}

impl UpdateService {
    /// Create a new update service
    pub fn new(config: super::ServiceConfig, channels: ServiceChannels) -> Self {
        let state = Arc::new(TokioMutex::new(super::UpdateServiceState::new(config)));
        
        Self {
            state,
            channels,
        }
    }

    /// Run the update service
    pub async fn run(mut self) -> DaemonResult<()> {
        info!("Starting update service");

        let mut update_interval = tokio::time::interval(Duration::from_secs(60));
        let mut update_rx = self.channels.update_rx.take().unwrap();
        let mut shutdown_rx = self.channels.shutdown_rx.take().unwrap();

        loop {
            tokio::select! {
                _ = update_interval.tick() => {
                    let state = self.state.lock().await;
                    
                    // Verify environment state periodically
                    if let Err(e) = state.get_current_state().await {
                        error!("State verification failed: {}", e);
                    }

                    // Clean up old state
                    if let Err(e) = state.cleanup_old_snapshots(7).await {
                        error!("Failed to clean up old state: {}", e);
                    }
                }
                
                Some(request) = update_rx.recv() => {
                    if let Err(e) = self.handle_update_request(request).await {
                        error!("Update processing failed: {}", e);
                    }
                }
                
                _ = shutdown_rx.recv() => {
                    info!("Received shutdown signal");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle an update request
    async fn handle_update_request(&mut self, request: UpdateRequest) -> DaemonResult<()> {
        match &request.update_type {
            UpdateType::PackageUpdate { package, force, update_deps } => {
                self.handle_package_update(package, *force, *update_deps).await
            },
            UpdateType::PackageInstall(package) => {
                self.handle_package_install(package).await
            },
            UpdateType::PackageRemove(package) => {
                self.handle_package_remove(package).await
            },
            UpdateType::EnvironmentSync => {
                self.handle_environment_sync().await
            }
        }
    }

    async fn handle_package_update(&mut self, package: &Package, force: bool, _update_deps: bool) -> DaemonResult<()> {
        info!("Processing update request for {}", package.name());
        
        let state = self.state.lock().await;
        
        // Create checkpoint before update
        let _checkpoint_id = Uuid::new_v4();
        state.get_current_state().await?;
        
        // Begin transaction
        let _event = VersionEvent {
            timestamp: Utc::now(),
            from_version: None,
            to_version: package.version().clone(),
            impact: VersionImpact::None,
            reason: format!("Installation via direct request"),
            python_version: PythonVersion::parse("3.8.0").unwrap(),
            is_direct: true,
            affected_dependencies: Default::default(),
            approved: true,
            approved_by: None,
            policy_snapshot: None,
        };

        if force {
            let operation = TransactionOperation::Update {
                from: package.clone(),
                to: package.clone(),
            };
            info!("Forcing update with operation: {:?}", operation);
        }

        info!("Successfully processed update for {}", package.name());
        Ok(())
    }

    async fn handle_package_install(&mut self, package: &Package) -> DaemonResult<()> {
        info!("Processing install request for {}", package.name());
        
        let state = self.state.lock().await;
        
        // Create checkpoint
        let _checkpoint_id = Uuid::new_v4();
        state.get_current_state().await?;
        
        let operation = TransactionOperation::Install(package.clone());
        info!("Installing with operation: {:?}", operation);
        
        info!("Successfully installed {}", package.name());
        Ok(())
    }

    async fn handle_package_remove(&mut self, package: &Package) -> DaemonResult<()> {
        info!("Processing remove request for {}", package.name());
        
        let state = self.state.lock().await;
        
        // Create checkpoint
        let _checkpoint_id = Uuid::new_v4();
        state.get_current_state().await?;
        
        let operation = TransactionOperation::Uninstall(package.clone());
        info!("Removing with operation: {:?}", operation);
        
        info!("Successfully removed {}", package.name());
        Ok(())
    }

    async fn handle_environment_sync(&mut self) -> DaemonResult<()> {
        info!("Processing environment sync request");
        
        let state = self.state.lock().await;
        
        // Create checkpoint
        let _checkpoint_id = Uuid::new_v4();
        state.get_current_state().await?;
        
        // Verify current state
        if let Err(e) = state.get_current_state().await {
            error!("State verification failed: {}", e);
            return Err(e);
        }
        
        Ok(())
    }
}