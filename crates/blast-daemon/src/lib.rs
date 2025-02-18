//! Background service for the Blast Python environment manager.
//! 
//! This crate provides a daemon service that monitors Python environments
//! and handles real-time dependency updates.

use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex as TokioMutex, RwLock, oneshot};
use tracing::{error, info};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use blast_core::{
    python::{PythonVersion, PythonEnvironment},
    security::SecurityPolicy,
    package::Package,
    version::VersionConstraint,
    metadata::PackageMetadata,
    environment::Environment,
    error::BlastResult,
    state::EnvironmentState,
};

use crate::monitor::MonitorEvent;
use crate::transaction::{TransactionOperation, TransactionManager, TransactionContext};
use crate::error::DaemonResult;

pub mod error;
pub mod state;
pub mod metrics;
pub mod service;
pub mod monitor;
pub mod transaction;
pub mod update;
pub mod environment;

// Re-export commonly used types
pub use error::DaemonError;
pub use state::{StateManager, Checkpoint};
pub use metrics::MetricsManager;
pub use service::DaemonService;
pub use monitor::PythonResourceMonitor;
pub use environment::EnvironmentManager;

// Internal module re-exports
pub use monitor::{
    PythonResourceLimits,
    EnvironmentUsage,
    EnvDiskUsage,
    CacheUsage,
};
pub use blast_image::validation;
pub use state::*;
pub use metrics::{
    PackageMetrics,
    EnvironmentMetrics,
};

// Local imports with full paths to avoid conflicts
use crate::update::UpdateManager;

/// Daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Maximum number of pending updates
    pub max_pending_updates: usize,
    /// Maximum age of state snapshots in days
    pub max_snapshot_age_days: u64,
    /// Environment path
    pub env_path: PathBuf,
    /// Cache path
    pub cache_path: PathBuf,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            max_pending_updates: 100,
            max_snapshot_age_days: 7,
            env_path: PathBuf::from("environments/default"),
            cache_path: PathBuf::from("cache"),
        }
    }
}

/// Daemon state
#[derive(Debug)]
pub struct Daemon {
    /// Configuration for the daemon
    config: DaemonConfig,
    /// Channel for sending monitor events
    monitor_tx: mpsc::Sender<MonitorEvent>,
    /// Update manager for environment updates
    update_manager: Arc<TokioMutex<UpdateManager>>,
    /// State manager
    state_manager: Arc<RwLock<StateManager>>,
    /// Environment activation state
    activation_state: Arc<RwLock<ActivationState>>,
    /// Environment manager
    environment_manager: Arc<EnvironmentManager>,
    /// Service instance
    service: Arc<TokioMutex<DaemonService>>,
    /// Transaction manager
    transaction_manager: Arc<TokioMutex<TransactionManager>>,
}

#[derive(Debug, Clone)]
pub struct ActivationState {
    /// Currently active environment name
    active_env_name: Option<String>,
    /// Path to active environment
    active_env_path: Option<PathBuf>,
    /// Python version of active environment
    active_python_version: Option<PythonVersion>,
    /// Activation timestamp
    activated_at: Option<SystemTime>,
}

impl ActivationState {
    pub fn new() -> Self {
        Self {
            active_env_name: None,
            active_env_path: None,
            active_python_version: None,
            activated_at: None,
        }
    }
}

impl Daemon {
    /// Create a new daemon instance
    pub async fn new(config: DaemonConfig) -> BlastResult<Self> {
        // Create channels
        let (monitor_tx, monitor_rx) = mpsc::channel(config.max_pending_updates);

        // Create managers
        let state_manager = Arc::new(RwLock::new(StateManager::new(config.env_path.clone())));
        
        // Create initial environment state for transaction manager
        let initial_state = blast_core::state::EnvironmentState::new(
            "default".to_string(),
            PythonVersion::default(),
            HashMap::new(),
            Default::default(),
        );
        let transaction_manager = Arc::new(TokioMutex::new(TransactionManager::new(initial_state)));
        let activation_state = Arc::new(RwLock::new(ActivationState::new()));
        
        // Create environment manager
        let environment_manager = Arc::new(EnvironmentManager::new(config.env_path.clone()));

        // Create update manager
        let update_manager = Arc::new(TokioMutex::new(UpdateManager::new(
            config.env_path.clone(),
            config.cache_path.clone(),
            monitor_rx
        )));

        // Create service instance with cloned monitor_tx
        let service = Arc::new(TokioMutex::new(DaemonService::new(monitor_tx.clone())?));

        Ok(Self {
            config,
            monitor_tx,
            update_manager,
            state_manager,
            activation_state,
            environment_manager,
            service,
            transaction_manager,
        })
    }

    /// Get the daemon configuration
    pub fn config(&self) -> &DaemonConfig {
        &self.config
    }

    /// Get the state manager
    pub fn state_manager(&self) -> Arc<RwLock<StateManager>> {
        self.state_manager.clone()
    }

    pub async fn list_environments(&self) -> DaemonResult<Vec<DaemonEnvironment>> {
        let state_manager = self.state_manager.read().await;
        let current_state = state_manager.get_current_state().await?;
        
        if current_state.name() != "default" {
            Ok(vec![DaemonEnvironment {
                name: current_state.name().to_string(),
                python_version: current_state.python_version.to_string(),
                path: self.config.env_path.clone(),
                last_accessed: SystemTime::now(),
                active: current_state.is_active(),
            }])
        } else {
            Ok(vec![])
        }
    }

    /// Create a new Python environment
    pub async fn create_environment(&self, policy: &SecurityPolicy) -> DaemonResult<PythonEnvironment> {
        info!("Creating new environment with policy: {:?}", policy);
        
        // Create environment using environment manager
        let env = self.environment_manager.create_environment(
            "default",
            &policy.python_version,
        ).await?;

        // Create basic directory structure
        for dir in ["bin", "lib", "include"] {
            tokio::fs::create_dir_all(env.path().join(dir)).await.map_err(|e| 
                DaemonError::environment(format!("Failed to create {} directory: {}", dir, e))
            )?;
        }

        // Create site-packages directory
        tokio::fs::create_dir_all(env.path().join("lib").join("python3").join("site-packages"))
            .await
            .map_err(|e| DaemonError::environment(format!(
                "Failed to create site-packages directory: {}", e
            )))?;

        // Create initial environment state
        let env_state = blast_core::state::EnvironmentState::new(
            "default".to_string(),
            policy.python_version.clone(),
            HashMap::new(),
            HashMap::new(),
        );

        // Add environment with transaction
        self.add_environment("default".to_string(), env_state).await?;

        info!("Environment created successfully at {}", env.path().display());
        Ok(env)
    }

    pub async fn destroy_environment(&self, env: &PythonEnvironment) -> DaemonResult<()> {
        info!("Destroying environment at {}", env.path().display());
        
        // Stop monitoring
        self.stop_monitoring(env).await?;

        // Remove environment with transaction
        if let Some(name) = env.path().file_name().and_then(|n| n.to_str()) {
            self.remove_environment_with_transaction(name).await?;
        }

        // Remove environment directory
        tokio::fs::remove_dir_all(env.path()).await?;

        info!("Environment destroyed successfully");
        Ok(())
    }

    pub async fn stop_monitoring(&self, env: &PythonEnvironment) -> DaemonResult<()> {
        info!("Stopping environment monitoring");
        
        // Send stop monitoring event
        self.monitor_tx.send(MonitorEvent::StopMonitoring {
            env_path: env.path().to_path_buf(),
        }).await.map_err(|e| DaemonError::monitor(format!("Failed to send stop monitoring event: {}", e)))?;

        info!("Environment monitoring stopped");
        Ok(())
    }

    pub async fn clean_environment(&self, env: &PythonEnvironment) -> DaemonResult<()> {
        info!("Cleaning environment");
        
        // Get all installed packages
        let packages = env.get_packages()?;
        
        // Remove each package with transaction
        for package in packages {
            if let Err(e) = self.remove_package(&package).await {
                error!("Failed to remove package {}: {}", package.name(), e);
            }
        }

        info!("Environment cleaned successfully");
        Ok(())
    }

    pub async fn reinitialize_environment(&self, env: &PythonEnvironment) -> DaemonResult<()> {
        info!("Reinitializing environment");
        
        // Create fresh virtual environment
        env.create().await?;

        // Set up basic configuration
        let python_path = env.path().join("bin").join("python");
        std::fs::write(
            env.path().join("pyvenv.cfg"),
            format!(
                "home = {}\nversion = {}\ninclude-system-site-packages = false\n",
                python_path.display(),
                env.python_version()
            ),
        )?;

        info!("Environment reinitialized successfully");
        Ok(())
    }

    pub async fn restore_essential_packages(&self, _env: &PythonEnvironment) -> DaemonResult<()> {
        info!("Restoring essential packages");
        
        // Install pip and setuptools with transactions
        let essential_packages = vec![
            Package::new(
                "pip".to_string(),
                "latest".to_string(),
                PackageMetadata::new(
                    "pip".to_string(),
                    "latest".to_string(),
                    HashMap::new(),
                    VersionConstraint::any(),
                ),
                VersionConstraint::any(),
            )?,
            Package::new(
                "setuptools".to_string(),
                "latest".to_string(),
                PackageMetadata::new(
                    "setuptools".to_string(),
                    "latest".to_string(),
                    HashMap::new(),
                    VersionConstraint::any(),
                ),
                VersionConstraint::any(),
            )?,
        ];

        for package in essential_packages {
            self.install_package(&package).await?;
        }

        info!("Essential packages restored successfully");
        Ok(())
    }

    /// Run the daemon in the background
    pub async fn run(self) -> DaemonResult<()> {
        info!("Starting daemon service");
        
        // Create a channel for startup completion
        let (startup_tx, startup_rx) = oneshot::channel();
        let startup_signal = Arc::new(TokioMutex::new(Some(startup_tx)));

        // Take ownership of the update manager
        let update_manager = Arc::clone(&self.update_manager);
        
        // Spawn the update manager in a separate task
        let update_task = {
            let startup_signal = Arc::clone(&startup_signal);
            tokio::spawn(async move {
                // Get the update manager lock inside the task
                let mut update_manager = update_manager.lock().await;
                
                // Signal successful startup immediately after getting the lock
                if let Some(tx) = startup_signal.lock().await.take() {
                    let _ = tx.send(());
                }
                
                // Run the update manager
                update_manager.run().await
            })
        };

        // Wait for startup completion or timeout
        tokio::select! {
            _ = startup_rx => {
                info!("Daemon startup completed successfully");
                
                // Keep running until update task completes or error occurs
                match update_task.await {
                    Ok(Ok(_)) => info!("Update manager completed normally"),
                    Ok(Err(e)) => {
                        error!("Update manager error: {}", e);
                        return Err(e);
                    }
                    Err(e) => {
                        error!("Update manager task failed: {}", e);
                        return Err(DaemonError::service(format!("Update manager task failed: {}", e)));
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                error!("Daemon startup timed out after 5 seconds");
                return Err(DaemonError::service("Daemon startup timed out".to_string()));
            }
        }

        Ok(())
    }

    /// Register an environment as active and start monitoring
    pub async fn register_active_environment(&self, _env_name: String) -> DaemonResult<()> {
        let state_manager = self.state_manager.clone();
        
        // Update active status in state
        {
            let state_manager = state_manager.write().await;
            let mut current_state = state_manager.get_current_state().await?;
            current_state.set_active(true);
            state_manager.update_current_state(current_state).await?;
        }

        // Start monitoring task
        let monitor_tx = self.monitor_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = monitor_tx.send(MonitorEvent::ResourceCheck).await {
                    error!("Failed to send resource check event: {}", e);
                    break;
                }
            }
        });

        Ok(())
    }

    /// Deactivate the current environment
    pub async fn deactivate_environment(&self) -> DaemonResult<()> {
        info!("Deactivating environment");
        
        // Clear activation state
        {
            let mut activation_state = self.activation_state.write().await;
            activation_state.active_env_name = None;
            activation_state.active_env_path = None;
            activation_state.active_python_version = None;
            activation_state.activated_at = None;
        }

        // Update state manager
        {
            let state_manager = self.state_manager.write().await;
            state_manager.clear_active_environment().await?;
        }

        info!("Environment deactivated successfully");
        Ok(())
    }

    /// Verify daemon has necessary permissions and access
    pub async fn verify_access(&self) -> DaemonResult<()> {
        // Verify environment path access
        let env_path = &self.config.env_path;
        if !env_path.exists() {
            std::fs::create_dir_all(env_path).map_err(|e| {
                DaemonError::Access(format!("Failed to create environment directory: {}", e))
            })?;
        }

        // Verify cache path access
        let cache_path = &self.config.cache_path;
        if !cache_path.exists() {
            std::fs::create_dir_all(cache_path).map_err(|e| {
                DaemonError::Access(format!("Failed to create cache directory: {}", e))
            })?;
        }

        // Verify state file access
        let state_manager = self.state_manager.read().await;
        state_manager.verify().await.map_err(|e| {
            DaemonError::Access(format!("Failed to verify state access: {}", e))
        })?;

        // Verify we can write to the environment directory
        let test_file = env_path.join(".blast_write_test");
        tokio::fs::write(&test_file, b"test").await.map_err(|e| {
            DaemonError::Access(format!("Failed to write to environment directory: {}", e))
        })?;
        tokio::fs::remove_file(&test_file).await.map_err(|e| {
            DaemonError::Access(format!("Failed to clean up test file: {}", e))
        })?;

        // Verify we can write to the cache directory
        let test_file = cache_path.join(".blast_write_test");
        tokio::fs::write(&test_file, b"test").await.map_err(|e| {
            DaemonError::Access(format!("Failed to write to cache directory: {}", e))
        })?;
        tokio::fs::remove_file(&test_file).await.map_err(|e| {
            DaemonError::Access(format!("Failed to clean up test file: {}", e))
        })?;

        // Verify monitor channel
        if self.monitor_tx.is_closed() {
            return Err(DaemonError::Access("Monitor channel is closed".to_string()));
        }

        Ok(())
    }

    /// Activate an environment
    pub async fn activate_environment(&self, env_name: &str, python_version: PythonVersion) -> DaemonResult<()> {
        info!("Activating environment: {}", env_name);
        
        let env_path = self.config.env_path.join(env_name);
        
        // Ensure environment exists and is properly initialized
        if !env_path.exists() {
            // Create environment using environment manager
            self.environment_manager.create_environment(
                env_name,
                &python_version,
            ).await?;
        }

        // Update activation state
        {
            let mut activation_state = self.activation_state.write().await;
            activation_state.active_env_name = Some(env_name.to_string());
            activation_state.active_env_path = Some(env_path.clone());
            activation_state.active_python_version = Some(python_version.clone());
            activation_state.activated_at = Some(SystemTime::now());
        }

        // Update state manager
        {
            let state_manager = self.state_manager.write().await;
            state_manager.set_active_environment(
                env_name.to_string(),
                env_path,
                python_version,
            ).await?;
        }

        // Start monitoring
        self.monitor_tx.send(MonitorEvent::ResourceCheck).await.map_err(|e| {
            DaemonError::monitor(format!("Failed to send resource check event: {}", e))
        })?;

        info!("Environment activated successfully");
        Ok(())
    }

    /// Get current activation state
    pub async fn get_activation_state(&self) -> DaemonResult<ActivationState> {
        Ok(self.activation_state.read().await.clone())
    }

    /// Start the daemon in the background
    pub async fn start_background(&self) -> BlastResult<()> {
        // Clone necessary components for the background task
        let service = self.service.clone();
        let config = self.config.clone();
        
        // Spawn the background task
        tokio::spawn(async move {
            let mut last_update = std::time::Instant::now();
            
            loop {
                let mut service = service.lock().await;
                
                // Only process updates every few seconds to avoid unnecessary transactions
                if last_update.elapsed() >= Duration::from_secs(5) {
                    // Check if there are any pending updates before creating a transaction
                    if service.has_pending_updates().await {
                        if let Err(e) = service.process_updates().await {
                            // Only log as error if it's not a "Transaction not found" error
                            if !e.to_string().contains("Transaction not found") {
                                error!("Error processing updates: {}", e);
                            }
                        }
                    }
                    last_update = std::time::Instant::now();
                }
                
                // Sleep for a short duration to prevent busy-waiting
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        // Create socket directory if it doesn't exist
        let socket_dir = std::path::Path::new("/tmp/blast");
        if !socket_dir.exists() {
            std::fs::create_dir_all(socket_dir)?;
        }

        // Create socket file
        let socket_path = socket_dir.join(format!("{}.sock", config.env_path.file_name().unwrap_or_default().to_string_lossy()));
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }
        std::fs::write(&socket_path, "")?;

        Ok(())
    }

    /// Check if there are any pending updates
    pub async fn has_pending_updates(&self) -> bool {
        let service = self.service.lock().await;
        service.has_pending_updates().await
    }

    /// Begin a new transaction
    pub async fn begin_transaction(&self, description: String) -> DaemonResult<TransactionContext> {
        let transaction_manager = self.transaction_manager.lock().await;
        transaction_manager.begin_transaction(description).await
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, id: uuid::Uuid) -> DaemonResult<()> {
        let transaction_manager = self.transaction_manager.lock().await;
        transaction_manager.commit_transaction(id).await
    }

    /// Rollback a transaction
    pub async fn rollback_transaction(&self, id: uuid::Uuid) -> DaemonResult<()> {
        let transaction_manager = self.transaction_manager.lock().await;
        transaction_manager.rollback_transaction(id).await
    }

    /// Install a package with transaction handling
    pub async fn install_package(&self, package: &Package) -> DaemonResult<()> {
        info!("Installing package {} with transaction", package.name());
        
        // Create checkpoint before installation
        let checkpoint_id = uuid::Uuid::new_v4();
        self.state_manager.write().await.create_checkpoint(
            checkpoint_id,
            format!("Pre-install state for {}", package.name()),
            None,
        ).await?;

        // Begin transaction
        let mut ctx = self.begin_transaction(format!("Install package {}", package.name())).await?;
        ctx.add_operation(TransactionOperation::Install(package.clone()))?;

        // Attempt to commit
        match self.commit_transaction(ctx.id).await {
            Ok(_) => {
                info!("Successfully installed {}", package.name());
                Ok(())
            }
            Err(e) => {
                error!("Failed to install package: {}", e);
                // Restore from checkpoint
                if let Err(restore_err) = self.state_manager.write().await.restore_checkpoint(&checkpoint_id.to_string()).await {
                    error!("Failed to restore from checkpoint: {}", restore_err);
                }
                if let Err(e) = self.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback installation: {}", e);
                }
                Err(e)
            }
        }
    }

    /// Update a package with transaction handling
    pub async fn update_package(&self, package: &Package, force: bool) -> DaemonResult<()> {
        info!("Updating package {} with transaction", package.name());
        
        // Create checkpoint before update
        let checkpoint_id = uuid::Uuid::new_v4();
        self.state_manager.write().await.create_checkpoint(
            checkpoint_id,
            format!("Pre-update state for {}", package.name()),
            None,
        ).await?;

        // Begin transaction
        let mut ctx = self.begin_transaction(format!("Update package {}", package.name())).await?;
        
        // Add update operation
        let operation = if force {
            TransactionOperation::Update {
                from: package.clone(),
                to: package.clone(),
            }
        } else {
            TransactionOperation::Install(package.clone())
        };
        ctx.add_operation(operation)?;

        // Attempt to commit
        match self.commit_transaction(ctx.id).await {
            Ok(_) => {
                info!("Successfully updated {}", package.name());
                Ok(())
            }
            Err(e) => {
                error!("Failed to update package: {}", e);
                // Restore from checkpoint
                if let Err(restore_err) = self.state_manager.write().await.restore_checkpoint(&checkpoint_id.to_string()).await {
                    error!("Failed to restore from checkpoint: {}", restore_err);
                }
                if let Err(e) = self.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback update: {}", e);
                }
                Err(e)
            }
        }
    }

    /// Remove a package with transaction handling
    pub async fn remove_package(&self, package: &Package) -> DaemonResult<()> {
        info!("Removing package {} with transaction", package.name());
        
        // Create checkpoint before removal
        let checkpoint_id = uuid::Uuid::new_v4();
        self.state_manager.write().await.create_checkpoint(
            checkpoint_id,
            format!("Pre-remove state for {}", package.name()),
            None,
        ).await?;

        // Begin transaction
        let mut ctx = self.begin_transaction(format!("Remove package {}", package.name())).await?;
        ctx.add_operation(TransactionOperation::Uninstall(package.clone()))?;

        // Attempt to commit
        match self.commit_transaction(ctx.id).await {
            Ok(_) => {
                info!("Successfully removed {}", package.name());
                Ok(())
            }
            Err(e) => {
                error!("Failed to remove package: {}", e);
                // Restore from checkpoint
                if let Err(restore_err) = self.state_manager.write().await.restore_checkpoint(&checkpoint_id.to_string()).await {
                    error!("Failed to restore from checkpoint: {}", restore_err);
                }
                if let Err(e) = self.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback removal: {}", e);
                }
                Err(e)
            }
        }
    }

    /// Add an environment with transaction handling
    pub async fn add_environment(&self, name: String, state: EnvironmentState) -> DaemonResult<()> {
        info!("Adding environment {} with transaction", name);
        
        // Create checkpoint
        let checkpoint_id = uuid::Uuid::new_v4();
        self.state_manager.write().await.create_checkpoint(
            checkpoint_id,
            format!("Pre-add environment state for {}", name),
            None,
        ).await?;

        // Begin transaction
        let mut ctx = self.begin_transaction(format!("Add environment {}", name)).await?;
        ctx.add_operation(TransactionOperation::AddEnvironment {
            name: name.clone(),
            state: state.clone(),
        })?;

        // Attempt to commit
        match self.commit_transaction(ctx.id).await {
            Ok(_) => {
                info!("Successfully added environment {}", name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to add environment: {}", e);
                // Restore from checkpoint
                if let Err(restore_err) = self.state_manager.write().await.restore_checkpoint(&checkpoint_id.to_string()).await {
                    error!("Failed to restore from checkpoint: {}", restore_err);
                }
                if let Err(e) = self.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback environment addition: {}", e);
                }
                Err(e)
            }
        }
    }

    /// Remove an environment with transaction handling
    pub async fn remove_environment_with_transaction(&self, name: &str) -> DaemonResult<()> {
        info!("Removing environment {} with transaction", name);
        
        // Create checkpoint
        let checkpoint_id = uuid::Uuid::new_v4();
        self.state_manager.write().await.create_checkpoint(
            checkpoint_id,
            format!("Pre-remove environment state for {}", name),
            None,
        ).await?;

        // Begin transaction
        let mut ctx = self.begin_transaction(format!("Remove environment {}", name)).await?;
        ctx.add_operation(TransactionOperation::RemoveEnvironment {
            name: name.to_string(),
        })?;

        // Attempt to commit
        match self.commit_transaction(ctx.id).await {
            Ok(_) => {
                info!("Successfully removed environment {}", name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to remove environment: {}", e);
                // Restore from checkpoint
                if let Err(restore_err) = self.state_manager.write().await.restore_checkpoint(&checkpoint_id.to_string()).await {
                    error!("Failed to restore from checkpoint: {}", restore_err);
                }
                if let Err(e) = self.rollback_transaction(ctx.id).await {
                    error!("Failed to rollback environment removal: {}", e);
                }
                Err(e)
            }
        }
    }
}

/// Performance metrics snapshot
#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    /// Average pip install time
    pub avg_pip_install_time: Duration,
    /// Average sync time
    pub avg_sync_time: Duration,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Timestamp of snapshot
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct DaemonEnvironment {
    pub name: String,
    pub python_version: String,
    pub path: PathBuf,
    pub last_accessed: SystemTime,
    pub active: bool,
}

impl From<Box<dyn Environment>> for DaemonEnvironment {
    fn from(env: Box<dyn Environment>) -> Self {
        Self {
            name: env.name().unwrap_or("unnamed").to_string(),
            python_version: env.python_version().to_string(),
            path: env.path().to_path_buf(),
            last_accessed: SystemTime::now(),
            active: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnvironmentImage {
    pub name: String,
    pub python_version: String,
    pub created: chrono::DateTime<chrono::Utc>,
}
