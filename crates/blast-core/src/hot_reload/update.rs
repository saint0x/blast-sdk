use crate::package::Package;
use tokio::time::Instant;

/// Hot reload update type
#[derive(Debug, Clone)]
pub enum HotReloadUpdateType {
    /// Package update
    Package(Package),
    /// Environment variable update
    EnvVar(String, String),
    /// Python version update
    PythonVersion(String),
}

/// Hot reload update status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotReloadUpdateStatus {
    /// Update is pending
    Pending,
    /// Update is in progress
    InProgress,
    /// Update completed successfully
    Completed,
    /// Update failed
    Failed(String),
}

/// Hot reload update
#[derive(Debug, Clone)]
pub struct HotReloadUpdate {
    /// Update timestamp
    pub timestamp: Instant,
    /// Update type
    pub update_type: HotReloadUpdateType,
    /// Update status
    pub status: HotReloadUpdateStatus,
} 