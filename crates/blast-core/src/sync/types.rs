use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::package::Package;
use crate::python::PythonVersion;
use crate::version::Version;

/// Change to be applied during sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncChange {
    /// Install a new package
    InstallPackage(Package),
    /// Remove a package
    RemovePackage(Package),
    /// Update package version
    UpdatePackage {
        package: Package,
        from_version: Version,
        to_version: Version,
    },
    /// Update environment variables
    UpdateEnvVars(HashMap<String, String>),
    /// Update Python version
    UpdatePythonVersion {
        from_version: PythonVersion,
        to_version: PythonVersion,
    },
}

/// Strategy for merging environment changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Keep all changes from source
    KeepSource,
    /// Keep all changes from target
    KeepTarget,
    /// Prefer source changes but allow manual resolution
    PreferSource,
    /// Interactive merge with manual conflict resolution
    Interactive,
}

/// Cache size limits for sync operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSizeLimits {
    /// Maximum number of packages to cache
    pub max_packages: usize,
    /// Maximum size of package cache in bytes
    pub max_size: u64,
    /// Maximum age of cached items in seconds
    pub max_age: u64,
} 