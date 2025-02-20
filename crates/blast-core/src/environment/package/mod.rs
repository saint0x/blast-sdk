use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use serde::{Deserialize, Serialize};
use crate::error::{BlastResult, BlastError};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use std::collections::HashSet;
use std::time::{SystemTime, Duration};
use crate::version::VersionConstraint;
use chrono::{Utc, TimeZone};

mod resolver;
mod installer;
mod interceptor;
mod state;
mod graph;
mod progress;
mod scheduler;

pub use resolver::DependencyResolver;
pub use installer::PackageInstaller;
pub use interceptor::PipInterceptor;
pub use state::{PackageState, PackageInfo};
pub use graph::{DependencyGraph, DependencyNode};
pub use progress::{ProgressTracker, InstallationProgress, InstallationStep};
pub use scheduler::{OperationScheduler, SchedulerConfig, OperationPriority, OperationType, OperationStatus, QueueStatistics};

/// Package version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// Version string (PEP 440 compliant)
    pub version: String,
    /// Release timestamp
    pub released: chrono::DateTime<chrono::Utc>,
    /// Python version requirements
    pub python_requires: Option<String>,
    /// Package dependencies
    pub dependencies: Vec<Dependency>,
}

/// Package dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Package name
    pub name: String,
    /// Version constraint
    pub version_constraint: String,
    /// Optional dependency
    pub optional: bool,
    /// Environment markers
    pub markers: Option<String>,
}

/// Package operation types
#[derive(Debug, Clone)]
pub enum PackageOperation {
    /// Install package
    Install {
        name: String,
        version: Option<Version>,
        dependencies: Vec<Dependency>,
    },
    /// Uninstall package
    Uninstall {
        name: String,
    },
    /// Update package
    Update {
        name: String,
        from_version: Version,
        to_version: Version,
    },
}

/// Package layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    /// Python version
    pub python_version: String,
    /// Environment path
    pub env_path: PathBuf,
    /// Package index URL
    pub index_url: String,
    /// Extra index URLs
    pub extra_index_urls: Vec<String>,
    /// Trusted hosts
    pub trusted_hosts: Vec<String>,
    /// Require hashes
    pub require_hashes: bool,
    /// Allow prereleases
    pub allow_prereleases: bool,
    /// Cache directory
    pub cache_dir: PathBuf,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            python_version: String::from("3.9"),
            env_path: PathBuf::from("/var/lib/blast/environments"),
            index_url: String::from("https://pypi.org/simple"),
            extra_index_urls: Vec::new(),
            trusted_hosts: vec![String::from("pypi.org")],
            require_hashes: true,
            allow_prereleases: false,
            cache_dir: PathBuf::from("/var/lib/blast/cache"),
            cache_ttl: 86400, // 24 hours
        }
    }
}

/// Package state change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChange {
    /// Package installed
    PackageInstalled {
        name: String,
        version: Version,
        timestamp: SystemTime,
    },
    /// Package uninstalled
    PackageUninstalled {
        name: String,
        timestamp: SystemTime,
    },
    /// Package updated
    PackageUpdated {
        name: String,
        from: Version,
        to: Version,
        timestamp: SystemTime,
    },
    /// State restored from persistence
    StateRestored {
        timestamp: SystemTime,
    },
}

/// Version conflict type
#[derive(Debug, Clone)]
pub struct VersionConflict {
    /// Package name
    pub package: String,
    /// Required version
    pub required: VersionConstraint,
    /// Installed version
    pub installed: Version,
    /// Requiring packages
    pub required_by: HashSet<String>,
}

/// Conflict resolution strategy
#[derive(Debug, Clone)]
pub enum ResolutionStrategy {
    /// Use newest version that satisfies most constraints
    UseNewest,
    /// Use version with most dependents
    UseMostDependents,
    /// Force specific version
    ForceVersion(String),
    /// Remove conflicting package
    Remove,
}

/// Resolution result
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// Resolved version
    pub version: Version,
    /// Affected packages
    pub affected_packages: HashSet<String>,
    /// Required actions
    pub actions: Vec<PackageOperation>,
}

/// Dependency change event
#[derive(Debug, Clone)]
pub enum DependencyChange {
    /// File added
    FileAdded(PathBuf),
    /// File modified
    FileModified(PathBuf),
    /// File removed
    FileRemoved(PathBuf),
}

/// Package layer implementation
pub struct PackageLayer {
    /// Configuration
    config: PackageConfig,
    /// Dependency resolver
    resolver: Arc<DependencyResolver>,
    /// Package installer
    installer: Arc<PackageInstaller>,
    /// Pip interceptor
    interceptor: Arc<PipInterceptor>,
    /// Package state
    state: Arc<RwLock<PackageState>>,
    /// State change broadcaster
    state_tx: broadcast::Sender<StateChange>,
    /// Dependency watcher
    watcher: Arc<RwLock<Option<RecommendedWatcher>>>,
    /// Last state save
    last_save: Arc<RwLock<SystemTime>>,
    /// Operation scheduler
    scheduler: Arc<OperationScheduler>,
}

impl PackageLayer {
    /// Create new package layer
    pub async fn new(config: PackageConfig) -> BlastResult<Self> {
        let state = Arc::new(RwLock::new(PackageState::default()));
        let resolver = Arc::new(DependencyResolver::new(config.clone()));
        let installer = Arc::new(PackageInstaller::new(config.clone()));
        let interceptor = Arc::new(PipInterceptor::new(config.clone()));
        
        // Create scheduler with default config
        let scheduler_config = SchedulerConfig::default();
        let scheduler = Arc::new(OperationScheduler::new(scheduler_config));

        Ok(Self {
            config,
            resolver,
            installer,
            interceptor,
            state,
            state_tx: broadcast::channel(16).0,
            watcher: Arc::new(RwLock::new(None)),
            last_save: Arc::new(RwLock::new(SystemTime::now())),
            scheduler,
        })
    }

    /// Get package configuration
    pub fn config(&self) -> &PackageConfig {
        &self.config
    }

    /// Queue package operation
    pub async fn queue_operation(&self, operation: PackageOperation) -> BlastResult<String> {
        // Determine operation priority based on type
        let priority = match &operation {
            PackageOperation::Install { .. } => OperationPriority::Normal,
            PackageOperation::Uninstall { .. } => OperationPriority::High, // Higher priority for uninstalls
            PackageOperation::Update { .. } => OperationPriority::Normal,
        };

        // Queue operation with scheduler
        let operation_id = self.scheduler.queue_operation(
            operation.clone(),
            Some(priority),
            Vec::new(), // No dependencies for now
        ).await?;

        // Subscribe to operation status updates
        let mut status_rx = self.scheduler.subscribe_status_updates();
        let state = Arc::clone(&self.state);
        let installer = Arc::clone(&self.installer);
        let operation_clone = operation.clone();
        let operation_id_clone = operation_id.clone();

        // Spawn task to handle operation execution
        tokio::spawn(async move {
            while let Ok((id, status)) = status_rx.recv().await {
                if id != operation_id_clone {
                    continue;
                }

                match status {
                    OperationStatus::Running { .. } => {
                        // Execute operation
                        match &operation_clone {
                            PackageOperation::Install { name, version, dependencies: _ } => {
                                let version_str = version.as_ref().map(|v| v.version.as_str()).unwrap_or("");
                                if let Err(e) = installer.install_package(name.as_str(), version_str).await {
                                    tracing::error!("Failed to install package {}: {}", name, e);
                                }
                            }
                            PackageOperation::Uninstall { name } => {
                                if let Err(e) = installer.uninstall_package(name.as_str()).await {
                                    tracing::error!("Failed to uninstall package {}: {}", name, e);
                                }
                            }
                            PackageOperation::Update { name, from_version: _, to_version } => {
                                if let Err(e) = installer.update_package(name.as_str(), &to_version.version).await {
                                    tracing::error!("Failed to update package {}: {}", name, e);
                                }
                            }
                        }

                        // Update state after successful operation
                        let mut state = state.write().await;
                        if let Err(e) = state.update_from_operation(&operation_clone).await {
                            tracing::error!("Failed to update state: {}", e);
                        }
                        break;
                    }
                    OperationStatus::Completed { .. } => {
                        // Update state after successful operation
                        let mut state = state.write().await;
                        if let Err(e) = state.update_from_operation(&operation_clone).await {
                            tracing::error!("Failed to update state: {}", e);
                        }
                        break;
                    }
                    OperationStatus::Failed { .. } | OperationStatus::TimedOut { .. } => {
                        // Log failure and break
                        tracing::error!("Operation {} failed or timed out", id);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(operation_id)
    }

    /// Get operation status
    pub async fn get_operation_status(&self, operation_id: &str) -> Option<OperationStatus> {
        self.scheduler.get_operation_status(operation_id).await
    }

    /// Get queue statistics
    pub async fn get_queue_stats(&self) -> QueueStatistics {
        self.scheduler.get_queue_stats().await
    }

    /// Cancel operation
    pub async fn cancel_operation(&self, operation_id: &str) -> BlastResult<()> {
        self.scheduler.cancel_operation(operation_id).await
    }

    /// Intercept pip operation
    pub async fn intercept_pip(&self, args: Vec<String>) -> BlastResult<()> {
        self.interceptor.handle_pip_command(args).await
    }

    /// Check package conflicts
    pub async fn check_conflicts(&self) -> BlastResult<Vec<String>> {
        let state = self.state.read().await;
        self.resolver.check_state_conflicts(&state).await
    }

    /// Get current package state
    pub async fn get_state(&self) -> BlastResult<PackageState> {
        Ok(self.state.read().await.clone())
    }

    /// Save current state to disk
    pub async fn save_state(&self) -> BlastResult<()> {
        let state = self.state.read().await;
        let state_path = self.config.env_path.join("package_state.json");
        
        // Create state directory if it doesn't exist
        if let Some(parent) = state_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Serialize state
        let json = serde_json::to_string_pretty(&*state)
            .map_err(|e| crate::error::BlastError::package(format!(
                "Failed to serialize package state: {}", e
            )))?;
        
        // Write atomically using a temporary file
        let temp_path = state_path.with_extension("tmp");
        tokio::fs::write(&temp_path, json).await?;
        tokio::fs::rename(temp_path, &state_path).await?;
        
        // Update last save time
        *self.last_save.write().await = SystemTime::now();
        
        // Broadcast state change
        let _ = self.state_tx.send(StateChange::StateRestored {
            timestamp: SystemTime::now(),
        });
        
        Ok(())
    }

    /// Load state from disk
    pub async fn load_state(&self) -> BlastResult<()> {
        let state_path = self.config.env_path.join("package_state.json");
        
        if state_path.exists() {
            // Read state file
            let json = tokio::fs::read_to_string(&state_path).await?;
            
            // Deserialize state
            let loaded_state: PackageState = serde_json::from_str(&json)
                .map_err(|e| crate::error::BlastError::package(format!(
                    "Failed to deserialize package state: {}", e
                )))?;
            
            // Update state
            *self.state.write().await = loaded_state;
            
            // Broadcast state change
            let _ = self.state_tx.send(StateChange::StateRestored {
                timestamp: SystemTime::now(),
            });
        }
        
        Ok(())
    }

    /// Start automatic state persistence
    async fn start_state_persistence(&self) -> BlastResult<()> {
        let save_interval = Duration::from_secs(300); // Save every 5 minutes
        let state = Arc::clone(&self.state);
        let last_save = Arc::clone(&self.last_save);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(save_interval);
            loop {
                interval.tick().await;
                
                // Check if state has changed since last save
                let state_guard = state.read().await;
                let last_save_time = *last_save.read().await;
                let last_save_datetime = Utc.timestamp_opt(
                    last_save_time.duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    0
                ).unwrap();
                
                if state_guard.last_modified() > last_save_datetime {
                    let state_path = config.env_path.join("package_state.json");
                    if let Err(e) = state_guard.save(&state_path).await {
                        tracing::error!("Failed to save package state: {}", e);
                    } else {
                        *last_save.write().await = SystemTime::now();
                    }
                }
            }
        });
        
        Ok(())
    }

    /// Subscribe to state changes
    pub fn subscribe_state_changes(&self) -> broadcast::Receiver<StateChange> {
        self.state_tx.subscribe()
    }

    /// Start dependency monitoring
    pub async fn start_dependency_monitoring(&self) -> BlastResult<()> {
        let site_packages = self.config.env_path.join("lib")
            .join(format!("python{}", self.config.python_version))
            .join("site-packages");

        // Create watcher
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let changes: Vec<DependencyChange> = match event.kind {
                    notify::EventKind::Create(_) => {
                        event.paths.into_iter().map(DependencyChange::FileAdded).collect()
                    }
                    notify::EventKind::Modify(_) => {
                        event.paths.into_iter().map(DependencyChange::FileModified).collect()
                    }
                    notify::EventKind::Remove(_) => {
                        event.paths.into_iter().map(DependencyChange::FileRemoved).collect()
                    }
                    _ => Vec::new(),
                };
                if !changes.is_empty() {
                    let _ = tx.blocking_send(changes);
                }
            }
        }).map_err(|e| BlastError::package(format!("Failed to create file watcher: {}", e)))?;

        // Start watching site-packages
        watcher.watch(&site_packages, RecursiveMode::Recursive)
            .map_err(|e| BlastError::package(format!("Failed to watch directory: {}", e)))?;
        *self.watcher.write().await = Some(watcher);

        // Handle dependency changes
        let state = Arc::clone(&self.state);
        let resolver = Arc::clone(&self.resolver);
        
        tokio::spawn(async move {
            while let Some(changes) = rx.recv().await {
                for change in changes {
                    match change {
                        DependencyChange::FileAdded(path) |
                        DependencyChange::FileModified(path) => {
                            if path.extension().map_or(false, |ext| ext == "dist-info") {
                                // Metadata changed, update dependency graph
                                if let Err(e) = Self::update_package_metadata(&state, &resolver, &path).await {
                                    tracing::error!("Failed to update package metadata: {}", e);
                                }
                            }
                        }
                        DependencyChange::FileRemoved(path) => {
                            if path.extension().map_or(false, |ext| ext == "dist-info") {
                                // Package removed, update state
                                if let Some(name) = path.file_stem() {
                                    if let Err(e) = state.write().await.remove_package(name.to_string_lossy().as_ref()).await {
                                        tracing::error!("Failed to remove package from state: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Resolve version conflicts
    pub async fn resolve_conflicts(&self, conflicts: Vec<VersionConflict>) -> BlastResult<Vec<ResolutionResult>> {
        let mut results = Vec::new();
        
        for conflict in conflicts {
            // Get all versions that could satisfy the requirements
            let versions = self.resolver.get_compatible_versions(
                &conflict.package,
                &conflict.required,
                &conflict.installed,
            ).await?;
            
            if versions.is_empty() {
                // No compatible version found, remove the package
                results.push(ResolutionResult {
                    version: conflict.installed.clone(),
                    affected_packages: conflict.required_by.clone(),
                    actions: vec![PackageOperation::Uninstall {
                        name: conflict.package.clone(),
                    }],
                });
                continue;
            }
            
            // Find the best version according to strategy
            let versions_clone = versions.clone();
            let best_version = versions.into_iter()
                .max_by(|a, b| {
                    // Prefer versions that satisfy more dependents
                    let a_satisfied = conflict.required_by.iter()
                        .filter(|&dep| {
                            self.resolver.is_version_compatible(dep, a).unwrap_or(false)
                        })
                        .count();
                    let b_satisfied = conflict.required_by.iter()
                        .filter(|&dep| {
                            self.resolver.is_version_compatible(dep, b).unwrap_or(false)
                        })
                        .count();
                    a_satisfied.cmp(&b_satisfied)
                })
                .unwrap_or_else(|| versions_clone[0].clone());
            
            // Create update operation
            results.push(ResolutionResult {
                version: best_version.clone(),
                affected_packages: conflict.required_by.clone(),
                actions: vec![PackageOperation::Update {
                    name: conflict.package.clone(),
                    from_version: conflict.installed.clone(),
                    to_version: best_version,
                }],
            });
        }
        
        Ok(results)
    }

    /// Update package metadata from dist-info
    async fn update_package_metadata(
        state: &Arc<RwLock<PackageState>>,
        resolver: &Arc<DependencyResolver>,
        path: &Path,
    ) -> BlastResult<()> {
        // Read metadata from dist-info
        let metadata = resolver.read_package_metadata(path).await?;
        
        // Update state with new metadata
        let mut state = state.write().await;
        state.update_package_metadata(&metadata).await?;
        
        Ok(())
    }

    /// Initialize package layer
    pub async fn initialize(&self) -> BlastResult<()> {
        // Load saved state
        self.load_state().await?;
        
        // Start state persistence
        self.start_state_persistence().await?;
        
        // Start dependency monitoring
        self.start_dependency_monitoring().await?;
        
        Ok(())
    }
} 