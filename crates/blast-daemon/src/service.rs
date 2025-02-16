//! Update service for processing package updates

use std::sync::Arc;
use std::time::Duration;
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, Mutex as TokioMutex};
use tracing::{error, info};

use blast_core::{
    package::Package,
    python::PythonVersion,
    state::EnvironmentState,
};

use blast_core::version_control::{VersionManager, VersionPolicy};
use blast_resolver::{DependencyResolver, PyPIClient, Cache};

use crate::{
    DaemonError,
    DaemonResult,
    update::{UpdateType, UpdateRequest},
    transaction::{TransactionOperation, TransactionManager},
    monitor::{MonitorEvent, EnvironmentUsage, PythonResourceMonitor},
};

/// Internal state for the update service
pub(crate) struct UpdateServiceState {
    /// Python resource monitor
    monitor: PythonResourceMonitor,
    /// Dependency resolver
    resolver: Arc<DependencyResolver>,
    /// Transaction manager
    transaction_manager: TransactionManager,
    /// Version manager
    version_manager: VersionManager,
}

// Manual Debug implementation to handle non-Debug DependencyResolver
impl std::fmt::Debug for UpdateServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateServiceState")
            .field("monitor", &self.monitor)
            .field("resolver", &"<DependencyResolver>")
            .field("transaction_manager", &self.transaction_manager)
            .field("version_manager", &"<VersionManager>")
            .finish()
    }
}

impl UpdateServiceState {
    /// Create a new instance with the given paths and default configuration
    fn new(env_path: PathBuf, cache_path: PathBuf) -> DaemonResult<Self> {
        // Initialize PyPI client with reasonable timeouts
        let pypi_client = PyPIClient::new(
            30, // connect timeout in seconds
            60, // request timeout in seconds
            false, // don't verify SSL (set to true in production)
        ).map_err(|e| DaemonError::Resolver(e.to_string()))?;
        
        let cache = Cache::new(cache_path.clone());
        
        // Create initial environment state
        let initial_state = EnvironmentState::new(
            "default".to_string(),
            PythonVersion::parse("3.8.0").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        Ok(Self {
            monitor: PythonResourceMonitor::new(
                env_path,
                cache_path,
                Default::default(),
            ),
            resolver: Arc::new(DependencyResolver::new(pypi_client, cache)),
            transaction_manager: TransactionManager::new(initial_state),
            version_manager: VersionManager::new(VersionPolicy::default()),
        })
    }
}

/// Service for managing Python environment updates
#[derive(Debug)]
pub struct UpdateService {
    /// Shared service state
    state: Arc<TokioMutex<UpdateServiceState>>,
    /// Channel for receiving update requests
    update_rx: mpsc::Receiver<UpdateRequest>,
    /// Channel for sending shutdown signals
    shutdown_rx: broadcast::Receiver<()>,
}

/// Daemon service for handling updates and monitoring
#[derive(Debug)]
pub struct DaemonService {
    /// Channel for sending monitor events
    monitor_tx: mpsc::Sender<MonitorEvent>,
    /// Update service state
    update_service_state: Option<Arc<TokioMutex<UpdateServiceState>>>,
    /// Channel for sending update requests
    update_tx: Option<mpsc::Sender<UpdateRequest>>,
    /// Channel for sending shutdown signals
    shutdown_tx: Option<broadcast::Sender<()>>,
}

impl UpdateService {
    /// Create a new update service
    pub(crate) fn new(env_path: PathBuf, cache_path: PathBuf) -> DaemonResult<(Self, Arc<TokioMutex<UpdateServiceState>>, mpsc::Sender<UpdateRequest>, broadcast::Sender<()>)> {
        let (update_tx, update_rx) = mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        
        let state = Arc::new(TokioMutex::new(UpdateServiceState::new(env_path, cache_path)?));

        Ok((Self {
            state: state.clone(),
            update_rx,
            shutdown_rx,
        }, state, update_tx, shutdown_tx))
    }

    /// Run the update service
    pub(crate) async fn run(mut self) -> DaemonResult<()> {
        info!("Starting update service");
        
        let mut update_interval = tokio::time::interval(Duration::from_secs(60));
        
        loop {
            tokio::select! {
                _ = update_interval.tick() => {
                    let mut state = self.state.lock().await;
                    if !state.monitor.check_limits() {
                        let usage = state.monitor.get_current_usage();
                        error!(
                            "Resource limits exceeded - Env Size: {} MB, Cache Size: {} MB",
                            usage.env_disk_usage.total_size / 1_048_576,
                            usage.cache_usage.total_size / 1_048_576
                        );
                        return Err(DaemonError::ResourceLimit(
                            "Python environment resource limits exceeded".to_string()
                        ));
                    }
                }
                
                Some(request) = self.update_rx.recv() => {
                    if let Err(e) = self.handle_update_request(request).await {
                        error!("Update processing failed: {}", e);
                    }
                }
                
                _ = self.shutdown_rx.recv() => {
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

    async fn handle_package_update(&mut self, package: &Package, force: bool, update_deps: bool) -> DaemonResult<()> {
        info!("Processing update request for {}", package.name());
        
        let mut state = self.state.lock().await;
        
        // Check version policy
        if !force {
            if let Some(history) = state.version_manager.get_history(package.name()) {
                if let Some(current_version) = &history.current_version {
                    if !state.version_manager.check_upgrade_allowed(package, package.version())? {
                        return Err(DaemonError::Version(format!(
                            "Upgrade from {} to {} is not allowed by version policy",
                            current_version,
                            package.version()
                        )));
                    }
                }
            }
        }

        // Begin transaction
        let mut ctx = state.transaction_manager.begin_transaction().await?;
        
        // Resolve dependencies
        let dependencies = state.resolver.resolve(package).await
            .map_err(|e| DaemonError::Resolver(format!("Failed to resolve dependencies: {}", e)))?;
        
        // Add operations
        let operation = if force {
            TransactionOperation::Update {
                from: package.clone(),
                to: package.clone(),
            }
        } else {
            TransactionOperation::Install(package.clone())
        };
        
        ctx.add_operation(operation)?;
        
        if update_deps {
            for dep in dependencies {
                ctx.add_operation(TransactionOperation::Install(dep))?;
            }
        }

        // Commit transaction
        match state.transaction_manager.commit_transaction(ctx.id).await {
            Ok(_) => {
                state.version_manager.add_installation(
                    package,
                    true,
                    &PythonVersion::parse("3.8").unwrap(),
                    "User requested update".to_string(),
                );
                info!("Successfully processed update for {}", package.name());
                Ok(())
            }
            Err(e) => {
                error!("Failed to commit transaction: {}", e);
                if let Err(e) = state.transaction_manager.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback transaction: {}", e);
                }
                Err(e)
            }
        }
    }

    async fn handle_package_install(&mut self, package: &Package) -> DaemonResult<()> {
        info!("Processing install request for {}", package.name());
        
        let mut state = self.state.lock().await;
        let mut ctx = state.transaction_manager.begin_transaction().await?;
        ctx.add_operation(TransactionOperation::Install(package.clone()))?;
        
        match state.transaction_manager.commit_transaction(ctx.id).await {
            Ok(_) => {
                state.version_manager.add_installation(
                    package,
                    true,
                    &PythonVersion::parse("3.8").unwrap(),
                    "User requested install".to_string(),
                );
                info!("Successfully installed {}", package.name());
                Ok(())
            }
            Err(e) => {
                error!("Failed to install package: {}", e);
                if let Err(e) = state.transaction_manager.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback installation: {}", e);
                }
                Err(e)
            }
        }
    }

    async fn handle_package_remove(&mut self, package: &Package) -> DaemonResult<()> {
        info!("Processing remove request for {}", package.name());
        
        let state = self.state.lock().await;
        let mut ctx = state.transaction_manager.begin_transaction().await?;
        ctx.add_operation(TransactionOperation::Uninstall(package.clone()))?;
        
        match state.transaction_manager.commit_transaction(ctx.id).await {
            Ok(_) => {
                info!("Successfully removed {}", package.name());
                Ok(())
            }
            Err(e) => {
                error!("Failed to remove package: {}", e);
                if let Err(e) = state.transaction_manager.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback removal: {}", e);
                }
                Err(e)
            }
        }
    }

    async fn handle_environment_sync(&mut self) -> DaemonResult<()> {
        info!("Processing environment sync request");
        Ok(())
    }
}

impl DaemonService {
    /// Create a new daemon service
    pub fn new(monitor_tx: mpsc::Sender<MonitorEvent>) -> Self {
        Self { 
            monitor_tx,
            update_service_state: None,
            update_tx: None,
            shutdown_tx: None,
        }
    }

    /// Start the daemon service
    pub async fn start(&mut self) -> DaemonResult<()> {
        info!("Starting daemon service");
        
        // Create and start update service
        let env_path = PathBuf::from("environments/default");
        let cache_path = PathBuf::from("cache");
        
        let (service, state, update_tx, shutdown_tx) = UpdateService::new(env_path, cache_path)?;
        
        // Store shared state and channels
        self.update_service_state = Some(state);
        self.update_tx = Some(update_tx);
        self.shutdown_tx = Some(shutdown_tx);
        
        // Spawn the service task
        let service_handle = tokio::spawn(async move {
            if let Err(e) = service.run().await {
                error!("Update service error: {}", e);
            }
        });
        
        // Monitor service handle
        tokio::spawn(async move {
            if let Err(e) = service_handle.await {
                error!("Update service task error: {}", e);
            }
        });
        
        Ok(())
    }

    /// Stop the daemon service
    pub async fn stop(&mut self) -> DaemonResult<()> {
        info!("Stopping daemon service");
        
        // Send shutdown signal
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        
        // Clear state
        self.update_service_state = None;
        self.update_tx = None;
        
        Ok(())
    }

    /// Notify about resource usage
    pub async fn notify_resource_usage(&self, usage: EnvironmentUsage) -> DaemonResult<()> {
        self.monitor_tx.send(MonitorEvent::ResourceUpdate(usage))
            .await
            .map_err(|e| DaemonError::Monitor(e.to_string()))?;
        Ok(())
    }

    /// Notify about package changes
    pub async fn notify_package_change(&self) -> DaemonResult<()> {
        self.monitor_tx.send(MonitorEvent::PackageChanged)
            .await
            .map_err(|e| DaemonError::Monitor(e.to_string()))?;
        Ok(())
    }

    /// Send an update request
    pub async fn send_update_request(&self, request: UpdateRequest) -> DaemonResult<()> {
        if let Some(tx) = &self.update_tx {
            tx.send(request)
                .await
                .map_err(|e| DaemonError::Service(format!("Failed to send update request: {}", e)))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blast_core::package::{Package, PackageId, Version, VersionConstraint};
    use tempfile::tempdir;
    use std::str::FromStr;
    use std::time::Duration;

    #[tokio::test]
    async fn test_update_service() {
        let dir = tempdir().unwrap();
        let (service, _state, _update_tx, _shutdown_tx) = UpdateService::new(
            dir.path().to_path_buf(),
            dir.path().to_path_buf(),
        ).expect("Failed to create update service");

        // Spawn service
        let handle = tokio::spawn(async move {
            if let Err(e) = service.run().await {
                error!("Service error: {}", e);
            }
        });

        // Wait a bit and let it shut down
        tokio::time::sleep(Duration::from_millis(100)).await;
        handle.abort();
    }

    #[tokio::test]
    async fn test_daemon_service() {
        let (monitor_tx, _) = mpsc::channel(100);
        let mut daemon = DaemonService::new(monitor_tx);

        // Start service
        daemon.start().await.expect("Failed to start daemon");

        // Create test package
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        // Send update request
        let request = UpdateRequest::new_install(package);
        daemon.send_update_request(request).await.expect("Failed to send update request");

        // Stop service
        daemon.stop().await.expect("Failed to stop daemon");
    }
} 