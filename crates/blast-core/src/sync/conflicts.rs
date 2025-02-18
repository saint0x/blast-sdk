use serde::{Deserialize, Serialize};
use crate::version::Version;

/// Conflict during sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Description of the conflict
    pub description: String,
    /// Possible resolutions
    pub resolutions: Vec<ConflictResolution>,
    /// Selected resolution
    pub selected_resolution: Option<ConflictResolution>,
}

/// Type of sync conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    /// Version mismatch between environments
    VersionMismatch,
    /// Package exists in both environments with different versions
    PackageVersionConflict,
    /// Package dependencies are incompatible
    DependencyConflict,
    /// Environment variables conflict
    EnvVarConflict,
    /// Python version incompatibility
    PythonVersionConflict,
}

/// Resolution for a conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Use source version
    UseSource,
    /// Use target version
    UseTarget,
    /// Use specific version
    UseVersion(Version),
    /// Skip this change
    Skip,
    /// Merge changes
    Merge,
} 