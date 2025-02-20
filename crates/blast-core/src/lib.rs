//! Core types and traits for the Blast Python environment manager.
//!
//! This crate provides the fundamental types, traits, and utilities that are used
//! throughout the Blast ecosystem.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod config;
pub mod error;
pub mod environment;
pub mod package;
pub mod python;
pub mod types;
pub mod utils;
pub mod version_control;
pub mod state;
pub mod sync;
pub mod version_history;
pub mod security;
pub mod manifest;
pub mod bindings;
pub mod version;
pub mod metadata;
pub mod layer;
pub mod resolution;
pub mod logging;
pub mod diagnostics;
pub mod ui;
pub mod debug;
pub mod hot_reload;
pub mod shell_scripts;

// Re-export commonly used types
pub use crate::config::BlastConfig;
pub use crate::error::{BlastError, BlastResult};
pub use crate::package::{Package, PackageId};
pub use crate::version::{Version, VersionConstraint};
pub use crate::python::{PythonEnvironment, PythonVersion};
pub use crate::types::{CacheSettings, UpdateStrategy};
pub use crate::version_control::{VersionManager, VersionPolicy, UpgradeStrategy};
pub use crate::version_history::{VersionHistory, VersionEvent, VersionImpact};
pub use crate::state::{EnvironmentState, StateCheckpoint, StateDiff, StateVerification};
pub use crate::sync::{SyncManager, SyncOperation, SyncChange, SyncConflict, ConflictResolution, SyncStatus};
pub use crate::manifest::{
    Manifest, BlastMetadata, SystemDependency, ResourceRequirements,
    VenvConfig, LayerInfo, LayerType, CompressionType,
};
pub use crate::security::SecurityPolicy;
pub use crate::bindings::{NativeEnvironment, NativePackage, NativeManifest};
pub use crate::metadata::{
    PackageMetadata, BuildMetadata, DistributionMetadata,
    DistributionType,
};
pub use crate::shell_scripts::ActivationScripts;

/// Core trait for environment management
#[async_trait]
pub trait EnvironmentManager: Send + Sync + 'static {
    /// Create a new Python environment
    async fn create_environment(&self, config: &BlastConfig) -> BlastResult<PythonEnvironment>;

    /// Update an existing environment
    async fn update_environment(&self, env: &PythonEnvironment) -> BlastResult<()>;

    /// Activate an environment
    async fn activate_environment(&self, env: &PythonEnvironment) -> BlastResult<()>;

    /// Deactivate an environment
    async fn deactivate_environment(&self, env: &PythonEnvironment) -> BlastResult<()>;
}

/// Core trait for dependency resolution
#[async_trait]
pub trait DependencyResolver: Send + Sync + 'static {
    /// Resolve dependencies for a package
    async fn resolve_dependencies(&self, package: &Package) -> BlastResult<Vec<Package>>;

    /// Check for updates to a package
    async fn check_updates(&self, package: &Package) -> BlastResult<Option<Package>>;

    /// Get package metadata from PyPI
    async fn get_package_metadata(&self, package_id: &PackageId) -> BlastResult<Package>;
}

/// Core trait for caching
#[async_trait]
pub trait Cache: Send + Sync + 'static {
    /// Store package in cache
    async fn store_package(&self, package: &Package, data: Vec<u8>) -> BlastResult<()>;

    /// Retrieve package from cache
    async fn get_package(&self, package: &Package) -> BlastResult<Option<Vec<u8>>>;

    /// Check if package exists in cache
    async fn has_package(&self, package: &Package) -> BlastResult<bool>;

    /// Clear cache
    async fn clear(&self) -> BlastResult<()>;
}

/// Core trait for environment snapshots
#[async_trait]
pub trait SnapshotManager: Send + Sync + 'static {
    /// Create a snapshot of an environment
    async fn create_snapshot(&self, env: &PythonEnvironment) -> BlastResult<PathBuf>;

    /// Restore an environment from a snapshot
    async fn restore_snapshot(&self, snapshot_path: &PathBuf) -> BlastResult<PythonEnvironment>;

    /// List available snapshots
    async fn list_snapshots(&self) -> BlastResult<Vec<PathBuf>>;
}

/// Core trait for monitoring Python imports
#[async_trait]
pub trait ImportMonitor: Send + Sync + 'static {
    /// Start monitoring imports
    async fn start_monitoring(&self) -> BlastResult<()>;

    /// Stop monitoring imports
    async fn stop_monitoring(&self) -> BlastResult<()>;

    /// Get current import statistics
    async fn get_statistics(&self) -> BlastResult<ImportStatistics>;
}

/// Statistics for Python imports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatistics {
    /// Total number of imports
    pub total_imports: usize,
    /// Number of cached imports
    pub cached_imports: usize,
    /// Number of imports that triggered updates
    pub update_triggers: usize,
    /// Average import time in milliseconds
    pub average_import_time_ms: f64,
}

/// Core trait for manifest management
#[async_trait]
pub trait ManifestManager: Send + Sync + 'static {
    /// Get current manifest
    async fn get_manifest(&self) -> BlastResult<Manifest>;
    
    /// Update manifest
    async fn update_manifest(&self, manifest: &Manifest) -> BlastResult<()>;
    
    /// Record package installation
    async fn record_package_install(&self, package: &Package) -> BlastResult<()>;
    
    /// Record package removal
    async fn record_package_removal(&self, package: &Package) -> BlastResult<()>;
    
    /// Record environment variable change
    async fn record_env_var_change(&self, key: &str, value: &str) -> BlastResult<()>;
    
    /// Record system dependency
    async fn record_system_dependency(&self, dependency: &SystemDependency) -> BlastResult<()>;
    
    /// Record hook addition
    async fn record_hook_addition(&self, hook_type: &str, command: &str) -> BlastResult<()>;
    
    /// Verify manifest integrity
    async fn verify_manifest(&self) -> BlastResult<bool>;
}

/// Initialize the library
pub fn init() {
    // Set up logging if not already configured
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();
}
