//! Update request types and handling

use std::time::Duration;
use blast_core::package::Package;
use tokio::sync::mpsc;
use tracing::{info, warn};
use crate::{DaemonError, DaemonResult};
use crate::monitor::{MonitorEvent, PythonResourceMonitor, PythonResourceLimits};
use crate::metrics::MetricsManager;
use uuid;
use std::path::PathBuf;
use std::collections::HashMap;
use std::time::Instant;
use std::sync::Arc;

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
    /// Batched file changes
    pending_changes: HashMap<PathBuf, Instant>,
    /// Metrics manager
    metrics: Arc<MetricsManager>,
}

impl std::fmt::Debug for UpdateManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateManager")
            .field("monitor", &self.monitor)
            .field("env_path", &self.env_path)
            .field("cache_path", &self.cache_path)
            .field("pending_changes", &self.pending_changes)
            .field("metrics", &self.metrics)
            // Skip monitor_rx since it doesn't implement Debug
            .finish()
    }
}

impl UpdateManager {
    /// Create a new update manager
    pub fn new(env_path: PathBuf, cache_path: PathBuf, monitor_rx: mpsc::Receiver<MonitorEvent>) -> Self {
        Self {
            monitor: PythonResourceMonitor::new(
                env_path.clone(),
                cache_path.clone(),
                PythonResourceLimits::default(),
            ),
            env_path,
            cache_path,
            monitor_rx,
            pending_changes: HashMap::new(),
            metrics: Arc::new(MetricsManager::new(1000)), // Keep last 1000 operations
        }
    }

    /// Start processing updates
    pub async fn run(&mut self) -> DaemonResult<()> {
        info!("Starting update manager");
        
        // Use different intervals for different operations
        let mut file_batch_interval = tokio::time::interval(Duration::from_millis(250));
        let mut resource_check_interval = tokio::time::interval(Duration::from_secs(5));
        
        loop {
            tokio::select! {
                _ = file_batch_interval.tick() => {
                    // Process batched file changes
                    self.process_pending_changes().await?;
                }
                
                _ = resource_check_interval.tick() => {
                    // Check resource usage with caching
                    let usage = self.monitor.get_current_usage();
                    
                    // Log resource usage
                    info!(
                        "Resource update - Env Size: {} MB, Cache Size: {} MB, Packages: {}",
                        usage.env_disk_usage.total_size / 1_048_576,
                        usage.cache_usage.total_size / 1_048_576,
                        usage.cache_usage.package_count
                    );

                    // Check resource limits
                    if !self.monitor.check_limits() {
                        warn!("Resource limits exceeded! Throttling operations");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
                
                Some(event) = self.monitor_rx.recv() => {
                    match event {
                        MonitorEvent::FileChanged(path) => {
                            // Batch file changes instead of processing immediately
                            self.pending_changes.insert(path, Instant::now());
                        }
                        MonitorEvent::PackageChanged => {
                            info!("Processing package change");
                            self.handle_package_change().await?;
                        }
                        MonitorEvent::ResourceUpdate(usage) => {
                            info!(
                                "External resource update - Env Size: {} MB, Cache Size: {} MB",
                                usage.env_disk_usage.total_size / 1_048_576,
                                usage.cache_usage.total_size / 1_048_576
                            );
                        }
                    }
                }
                
                else => break,
            }
        }

        Ok(())
    }

    /// Process batched file changes
    async fn process_pending_changes(&mut self) -> DaemonResult<()> {
        let now = Instant::now();
        let mut to_process = Vec::new();

        // Filter changes older than 250ms to ensure we have a complete batch
        self.pending_changes.retain(|path, time| {
            if now.duration_since(*time) >= Duration::from_millis(250) {
                to_process.push(path.clone());
                false
            } else {
                true
            }
        });

        if !to_process.is_empty() {
            info!("Processing {} batched file changes", to_process.len());
            for path in to_process {
                self.handle_file_change(&path).await?;
            }
        }

        Ok(())
    }

    /// Handle Python file changes
    async fn handle_file_change(&self, path: &std::path::Path) -> DaemonResult<()> {
        // Check if the file is within our environment path
        if !path.starts_with(&self.env_path) {
            return Ok(());
        }

        info!(
            "Processing Python file change: {} ({})",
            path.display(),
            if path.is_file() { "file" } else { "directory" }
        );

        Ok(())
    }

    /// Handle package changes
    async fn handle_package_change(&mut self) -> DaemonResult<()> {
        info!("Processing package change event");
        
        let start = Instant::now();
        let operation_id = uuid::Uuid::new_v4();
        
        // Get initial state
        let initial_usage = self.monitor.get_current_usage();
        
        // Check environment state
        self.check_environment_state()?;
        
        // Check and cleanup cache if needed
        self.check_cache_state()?;
        
        // Get final state
        let final_usage = self.monitor.get_current_usage();
        
        // Calculate metrics
        let pip_duration = start.elapsed();
        let sync_duration = Duration::from_millis(100); // This would come from actual sync
        let dependency_count = final_usage.cache_usage.package_count - initial_usage.cache_usage.package_count;
        let cache_hits = self.calculate_cache_hits(&initial_usage, &final_usage);
        
        // Record metrics
        self.metrics.record_package_install(
            operation_id,
            "unknown".to_string(), // We'd get this from pip output
            "unknown".to_string(),
            pip_duration,
            sync_duration,
            dependency_count,
            cache_hits,
            final_usage.env_disk_usage.total_size,
        ).await;
        
        // Update environment metrics
        self.metrics.update_environment_metrics(
            "default".to_string(), // We'd get this from context
            final_usage.cache_usage.package_count,
            final_usage.env_disk_usage.total_size,
            final_usage.cache_usage.total_size,
            sync_duration,
        ).await;
        
        Ok(())
    }

    /// Calculate cache hits between two usage snapshots
    fn calculate_cache_hits(
        &self,
        initial: &crate::monitor::EnvironmentUsage,
        final_state: &crate::monitor::EnvironmentUsage,
    ) -> usize {
        // This is a simplified calculation
        // In reality, we'd track actual cache hits during dependency resolution
        let new_packages = final_state.cache_usage.package_count - initial.cache_usage.package_count;
        let cache_size_diff = final_state.cache_usage.total_size - initial.cache_usage.total_size;
        
        if cache_size_diff == 0 && new_packages > 0 {
            new_packages // All were cache hits
        } else {
            0 // Conservative estimate
        }
    }

    /// Get metrics manager
    pub fn metrics(&self) -> Arc<MetricsManager> {
        self.metrics.clone()
    }

    /// Check environment state
    fn check_environment_state(&self) -> DaemonResult<()> {
        let site_packages = self.env_path.join("lib").join("python3").join("site-packages");
        
        if !site_packages.exists() {
            return Err(DaemonError::Service(
                format!("Site-packages directory not found: {}", site_packages.display())
            ));
        }

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

    /// Check cache state and cleanup if needed
    fn check_cache_state(&mut self) -> DaemonResult<()> {
        // Ensure cache directory exists
        if !self.cache_path.exists() {
            std::fs::create_dir_all(&self.cache_path)?;
        }

        // Get current cache usage
        let usage = self.monitor.get_current_usage();
        let total_size = usage.cache_usage.total_size;
        let limit = self.monitor.get_limits().max_cache_size;

        // If cache size exceeds limit, clean up old entries
        if total_size > limit {
            self.cleanup_cache(total_size, limit)?;
        }

        Ok(())
    }

    /// Clean up old cache entries
    fn cleanup_cache(&mut self, current_size: u64, limit: u64) -> DaemonResult<()> {
        let mut entries: Vec<_> = std::fs::read_dir(&self.cache_path)?
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_file())
            .collect();

        // Sort by modification time (oldest first)
        entries.sort_by(|a, b| {
            let a_time = a.metadata().ok().and_then(|m| m.modified().ok());
            let b_time = b.metadata().ok().and_then(|m| m.modified().ok());
            a_time.cmp(&b_time)
        });

        let mut remaining_size = current_size;
        let target_size = limit * 9 / 10; // Aim to reduce to 90% of limit

        // Remove oldest files until we're under target size
        for entry in entries {
            if remaining_size <= target_size {
                break;
            }

            if let Ok(metadata) = entry.metadata() {
                let file_size = metadata.len();
                if let Err(e) = std::fs::remove_file(entry.path()) {
                    warn!("Failed to remove cache file {}: {}", entry.path().display(), e);
                    continue;
                }
                remaining_size = remaining_size.saturating_sub(file_size);
                info!("Removed cache file: {} ({} bytes)", entry.path().display(), file_size);
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

#[cfg(test)]
mod tests {
    use super::*;
    use blast_core::package::{PackageId, Version, VersionConstraint};
    use std::collections::HashMap;
    use tokio::sync::mpsc;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_update_manager() {
        let (tx, rx) = mpsc::channel(100);
        let mut manager = UpdateManager::new(PathBuf::from("test_env"), PathBuf::from("test_cache"), rx);

        // Send test events
        tx.send(MonitorEvent::FileChanged(PathBuf::from("test.py"))).await.unwrap();
        tx.send(MonitorEvent::PackageChanged).await.unwrap();

        // Run manager for a short time
        tokio::spawn(async move {
            manager.run().await.unwrap();
        });

        // Wait a bit for processing
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    #[test]
    fn test_update_request() {
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        // Test update request
        let request = UpdateRequest::new_update(package.clone(), true, true);
        match request.update_type {
            UpdateType::PackageUpdate { force, update_deps, .. } => {
                assert!(force);
                assert!(update_deps);
            }
            _ => panic!("Wrong update type"),
        }

        // Test install request
        let request = UpdateRequest::new_install(package.clone());
        match request.update_type {
            UpdateType::PackageInstall(_) => (),
            _ => panic!("Wrong update type"),
        }

        // Test remove request
        let request = UpdateRequest::new_remove(package);
        match request.update_type {
            UpdateType::PackageRemove(_) => (),
            _ => panic!("Wrong update type"),
        }

        // Test sync request
        let request = UpdateRequest::new_sync();
        match request.update_type {
            UpdateType::EnvironmentSync => (),
            _ => panic!("Wrong update type"),
        }
    }
} 