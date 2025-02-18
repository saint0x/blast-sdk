//! Update service for processing package updates

use std::sync::Arc;
use std::time::Duration;
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc, Mutex as TokioMutex};
use tracing::{error, info};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

use blast_core::{
    package::Package,
    python::PythonVersion,
    state::EnvironmentState,
    version_history::{VersionEvent, VersionImpact},
    error::BlastResult,
};

use blast_core::version_control::{VersionManager, VersionPolicy};
use blast_resolver::{DependencyResolver, PyPIClient, Cache};

use crate::{
    DaemonError,
    DaemonResult,
    update::{UpdateType, UpdateRequest},
    transaction::{TransactionOperation, TransactionManager, TransactionContext, TransactionStatus},
    monitor::{MonitorEvent, EnvironmentUsage, PythonResourceMonitor},
    state::{StateManager, Checkpoint},
    metrics::MetricsManager,
    environment::EnvironmentManager,
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
    /// State manager
    state_manager: StateManager,
    /// Environment manager
    environment_manager: Arc<EnvironmentManager>,
}

// Manual Debug implementation to handle non-Debug DependencyResolver
impl std::fmt::Debug for UpdateServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateServiceState")
            .field("monitor", &self.monitor)
            .field("resolver", &"<DependencyResolver>")
            .field("transaction_manager", &self.transaction_manager)
            .field("version_manager", &"<VersionManager>")
            .field("state_manager", &"<StateManager>")
            .field("environment_manager", &"<EnvironmentManager>")
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
        let _metrics = Arc::new(MetricsManager::new(1000)); // Keep last 1000 operations
        
        Ok(Self {
            monitor: PythonResourceMonitor::new(
                env_path.clone(),
                cache_path,
                Default::default(),
            ),
            resolver: Arc::new(DependencyResolver::new(pypi_client, cache)),
            transaction_manager: TransactionManager::new(EnvironmentState::new(
                "default".to_string(),
                PythonVersion::parse("3.8.0").unwrap(),
                Default::default(),
                Default::default(),
            )),
            version_manager: VersionManager::new(VersionPolicy::default()),
            state_manager: StateManager::new(env_path.clone()),
            environment_manager: Arc::new(EnvironmentManager::new(env_path)),
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
    state: Arc<TokioMutex<UpdateServiceState>>,
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

                    // Verify environment state periodically
                    if let Err(e) = state.state_manager.verify_state().await {
                        error!("State verification failed: {}", e);
                    }

                    // Clean up old snapshots
                    if let Err(e) = state.state_manager.cleanup_old_snapshots(7).await {
                        error!("Failed to clean up old snapshots: {}", e);
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
        
        // Create checkpoint before update
        let checkpoint_id = Uuid::new_v4();
        state.state_manager.create_checkpoint(
            checkpoint_id,
            format!("Pre-update state for {}", package.name()),
            None,
        ).await?;
        
        // Begin transaction
        let mut ctx = state.transaction_manager.begin_transaction(format!("Update package {}", package.name())).await?;
        
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
            for dep in &dependencies {
                ctx.add_operation(TransactionOperation::Install(dep.clone()))?;
            }
        }
        
        // Commit transaction
        match state.transaction_manager.commit_transaction(ctx.id).await {
            Ok(_) => {
                // Update state
                let current_state = state.state_manager.get_current_state().await?;
                let event = VersionEvent {
                    timestamp: Utc::now(),
                    from_version: None,
                    to_version: package.version().clone(),
                    impact: VersionImpact::None,
                    reason: format!("Installation via direct request"),
                    python_version: current_state.python_version.clone(),
                    is_direct: true,
                    affected_dependencies: Default::default(),
                    approved: true,
                    approved_by: None,
                    policy_snapshot: None,
                };
                state.state_manager.add_package_with_event(package, event).await?;
                
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
                // Restore from checkpoint
                if let Err(restore_err) = state.state_manager.restore_checkpoint(&checkpoint_id.to_string()).await {
                    error!("Failed to restore from snapshot: {}", restore_err);
                }
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
        
        // Create checkpoint
        let checkpoint_id = Uuid::new_v4();
        state.state_manager.create_checkpoint(
            checkpoint_id,
            format!("Pre-install state for {}", package.name()),
            None,
        ).await?;
        
        let mut ctx = state.transaction_manager.begin_transaction(format!("Install package {}", package.name())).await?;
        ctx.add_operation(TransactionOperation::Install(package.clone()))?;
        
        match state.transaction_manager.commit_transaction(ctx.id).await {
            Ok(_) => {
                // Update state
                let current_state = state.state_manager.get_current_state().await?;
                let event = VersionEvent {
                    timestamp: Utc::now(),
                    from_version: None,
                    to_version: package.version().clone(),
                    impact: VersionImpact::None,
                    reason: format!("Installation via direct request"),
                    python_version: current_state.python_version.clone(),
                    is_direct: true,
                    affected_dependencies: Default::default(),
                    approved: true,
                    approved_by: None,
                    policy_snapshot: None,
                };
                state.state_manager.add_package_with_event(package, event).await?;
                
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
                // Restore from checkpoint
                if let Err(restore_err) = state.state_manager.restore_checkpoint(&checkpoint_id.to_string()).await {
                    error!("Failed to restore from checkpoint: {}", restore_err);
                }
                if let Err(e) = state.transaction_manager.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback installation: {}", e);
                }
                Err(e.into())
            }
        }
    }

    async fn handle_package_remove(&mut self, package: &Package) -> DaemonResult<()> {
        info!("Processing remove request for {}", package.name());
        
        let state = self.state.lock().await;
        
        // Create checkpoint
        let checkpoint_id = Uuid::new_v4();
        state.state_manager.create_checkpoint(
            checkpoint_id,
            format!("Pre-remove state for {}", package.name()),
            None,
        ).await?;
        
        let mut ctx = state.transaction_manager.begin_transaction(format!("Remove package {}", package.name())).await?;
        ctx.add_operation(TransactionOperation::Uninstall(package.clone()))?;
        
        match state.transaction_manager.commit_transaction(ctx.id).await {
            Ok(_) => {
                info!("Successfully removed {}", package.name());
                Ok(())
            }
            Err(e) => {
                error!("Failed to remove package: {}", e);
                // Restore from checkpoint
                if let Err(restore_err) = state.state_manager.restore_checkpoint(&checkpoint_id.to_string()).await {
                    error!("Failed to restore from checkpoint: {}", restore_err);
                }
                if let Err(e) = state.transaction_manager.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback removal: {}", e);
                }
                Err(e)
            }
        }
    }

    async fn handle_environment_sync(&mut self) -> DaemonResult<()> {
        info!("Processing environment sync request");
        
        let state = self.state.lock().await;
        
        // Create checkpoint
        let checkpoint_id = Uuid::new_v4();
        state.state_manager.create_checkpoint(
            checkpoint_id,
            "Pre-sync state".to_string(),
            None,
        ).await?;
        
        // Verify current state
        if let Err(e) = state.state_manager.verify_state().await {
            error!("State verification failed: {}", e);
            // Restore from checkpoint
            if let Err(restore_err) = state.state_manager.restore_checkpoint(&checkpoint_id.to_string()).await {
                error!("Failed to restore from checkpoint: {}", restore_err);
            }
            return Err(e.into());
        }
        
        Ok(())
    }
}

impl DaemonService {
    /// Create a new daemon service
    pub fn new(monitor_tx: mpsc::Sender<MonitorEvent>) -> DaemonResult<Self> {
        let env_path = PathBuf::from("environments/default");
        let cache_path = PathBuf::from("cache");
        
        let state = Arc::new(TokioMutex::new(UpdateServiceState::new(env_path, cache_path)?));
        
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
        
        // Create and start update service
        let env_path = PathBuf::from("environments/default");
        let cache_path = PathBuf::from("cache");
        
        let (service, state, update_tx, shutdown_tx) = UpdateService::new(env_path, cache_path)?;
        
        // Store shared state and channels
        self.state = state;
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
        let env_path = PathBuf::from("environments/default");
        let cache_path = PathBuf::from("cache");
        self.state = Arc::new(TokioMutex::new(UpdateServiceState::new(env_path, cache_path)?));
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
            tx.send(request).await.map_err(|_| DaemonError::Service("Update channel closed".to_string()))?;
            Ok(())
        } else {
            Err(DaemonError::Service("Service not started".to_string()))
        }
    }

    pub async fn cleanup_old_snapshots(&self, max_age_days: u64) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.state_manager.cleanup_old_snapshots(max_age_days).await?;
        Ok(())
    }

    pub async fn begin_transaction(&self, description: String) -> DaemonResult<TransactionContext> {
        let state = self.state.lock().await;
        state.transaction_manager.begin_transaction(description).await
    }

    pub async fn commit_transaction(&self, id: Uuid) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.transaction_manager.commit_transaction(id).await
    }

    pub async fn rollback_transaction(&self, id: Uuid) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.transaction_manager.rollback_transaction(id).await
    }

    pub async fn get_current_state(&self) -> DaemonResult<EnvironmentState> {
        let state = self.state.lock().await;
        state.state_manager.get_current_state().await.map_err(Into::into)
    }

    pub async fn list_checkpoints(&self) -> DaemonResult<Vec<Checkpoint>> {
        let state = self.state.lock().await;
        state.state_manager.list_checkpoints().await.map_err(Into::into)
    }

    pub async fn get_checkpoint(&self, id: &str) -> DaemonResult<Option<Checkpoint>> {
        let state = self.state.lock().await;
        state.state_manager.get_checkpoint(id).await.map_err(Into::into)
    }

    pub async fn create_environment(&self, name: String, python_version: PythonVersion) -> DaemonResult<()> {
        let state = self.state.lock().await;
        
        // Create environment using environment manager
        let _env = state.environment_manager.create_environment(&name, &python_version).await.map_err(|e: blast_core::error::BlastError| DaemonError::from(e))?;
        
        let env_state = EnvironmentState::new(
            name.clone(),
            python_version,
            HashMap::new(),
            HashMap::new(),
        );

        state.state_manager.add_environment(name, env_state).await.map_err(|e: blast_core::error::BlastError| DaemonError::from(e))?;
        Ok(())
    }

    pub async fn remove_environment(&self, name: &str) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.state_manager.remove_environment(name).await.map_err(|e: blast_core::error::BlastError| DaemonError::from(e))
    }

    pub async fn activate_environment(&self, name: String) -> DaemonResult<()> {
        let state = self.state.lock().await;
        let env_state = state.state_manager.get_current_state().await.map_err(|e: blast_core::error::BlastError| DaemonError::from(e))?;
        
        // Create environment using environment manager
        let env = state.environment_manager.create_environment(&name, &env_state.python_version).await.map_err(|e: blast_core::error::BlastError| DaemonError::from(e))?;
        
        state.state_manager.set_active_environment(
            name,
            env.path().to_path_buf(),
            env_state.python_version.clone(),
        ).await.map_err(|e: blast_core::error::BlastError| DaemonError::from(e))
    }

    pub async fn deactivate_environment(&self) -> DaemonResult<()> {
        let state = self.state.lock().await;
        state.state_manager.clear_active_environment().await.map_err(Into::into)
    }

    /// Check if there are any pending updates
    pub async fn has_pending_updates(&self) -> bool {
        let state = self.state.lock().await;
        
        // Get current state
        let current_state = match state.state_manager.get_current_state().await {
            Ok(state) => state,
            Err(_) => return false,
        };
        
        // Check current state against blast.toml config
        let config_path = state.environment_manager.as_ref().root_path().join("blast.toml");
        if let Ok(config) = blast_core::config::BlastConfig::from_file(&config_path) {
            // Check Python version mismatch
            if current_state.python_version != config.python_version {
                return true;
            }

            // Check package version mismatches
            for package in &config.dependencies.packages {
                if let Some(current_version) = current_state.packages.get(&package.name) {
                    if current_version.to_string() != package.version {
                        return true;
                    }
                } else {
                    return true;
                }
            }

            // Check for packages installed but not in config
            for name in current_state.packages.keys() {
                if !config.dependencies.packages.iter().any(|p| &p.name == name) {
                    return true;
                }
            }
        }

        // Check for pending transactions
        let transactions = state.transaction_manager.list_active_transactions().await;
        if let Ok(transactions) = transactions {
            if transactions.values().any(|t| matches!(t.status, TransactionStatus::Pending)) {
                return true;
            }
        }

        false
    }

    /// Process pending updates
    pub async fn process_updates(&mut self) -> BlastResult<()> {
        // Check for resource usage
        self.monitor_tx.send(MonitorEvent::ResourceCheck).await
            .map_err(|e| DaemonError::monitor(format!("Failed to send resource check: {}", e)))?;

        // Begin a new transaction for processing updates
        let transaction_id = {
            let state = self.state.lock().await;
            let ctx = state.transaction_manager.begin_transaction("Process pending updates".to_string()).await?;
            ctx.id
        };
        
        // Process the transaction
        self.process_transaction_by_id(transaction_id).await?;
        
        // Commit the transaction if successful
        let state = self.state.lock().await;
        state.transaction_manager.commit_transaction(transaction_id).await?;

        Ok(())
    }

    async fn process_transaction_by_id(&mut self, transaction_id: Uuid) -> BlastResult<()> {
        let transaction = {
            let state = self.state.lock().await;
            // Get the transaction details
            let mut ctx = state.transaction_manager.begin_transaction("Get transaction details".to_string()).await?;
            ctx.id = transaction_id;
            ctx
        };

        self.process_transaction(&transaction).await
    }

    async fn process_transaction(&mut self, transaction: &TransactionContext) -> BlastResult<()> {
        let _state = self.state.lock().await;
        for operation in &transaction.operations {
            match operation {
                TransactionOperation::Install(package) => {
                    // Handle package installation
                    info!("Processing install transaction for package: {:?}", package);
                }
                TransactionOperation::Uninstall(package) => {
                    // Handle package uninstallation
                    info!("Processing uninstall transaction for package: {:?}", package);
                }
                TransactionOperation::Update { from, to } => {
                    // Handle package update
                    info!("Processing update transaction from {:?} to {:?}", from, to);
                }
                TransactionOperation::AddEnvironment { name, state: env_state, .. } => {
                    // Handle environment addition
                    info!("Processing add environment: {} with state: {:?}", name, env_state);
                }
                TransactionOperation::RemoveEnvironment { name } => {
                    // Handle environment removal
                    info!("Processing remove environment: {}", name);
                }
            }
        }
        Ok(())
    }
} 