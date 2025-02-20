use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::python::PythonVersion;
use super::types::SyncChange;
use super::conflicts::SyncConflict;
use super::validation::SyncValidation;
use crate::package::Package;

/// Type of sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncOperationType {
    /// Add package
    AddPackage(Package),
    /// Remove package
    RemovePackage(Package),
    /// Update package
    UpdatePackage {
        name: String,
        from_version: String,
        to_version: String,
    },
    /// Update environment variable
    UpdateEnvVar {
        key: String,
        value: String,
    },
    /// Update Python version
    UpdatePythonVersion(String),
}

/// Sync operation between environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOperation {
    /// Operation ID
    pub id: String,
    /// Source environment
    pub source: String,
    /// Target environment
    pub target: String,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Operation status
    pub status: SyncStatus,
    /// Operation type
    pub operation_type: SyncOperationType,
    /// Package changes
    pub changes: Vec<SyncChange>,
    /// Conflicts that need resolution
    pub conflicts: Vec<SyncConflict>,
    /// Validation results
    pub validation: SyncValidation,
}

/// Status of sync operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Sync is being planned
    Planning,
    /// Sync is in progress
    InProgress,
    /// Sync completed successfully
    Completed,
    /// Sync failed
    Failed(String),
    /// Sync was cancelled
    Cancelled,
}

/// Package upgrade operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeOperation {
    /// Package being upgraded
    pub package: String,
    /// Current version
    pub from_version: PythonVersion,
    /// Target version
    pub to_version: PythonVersion,
    /// Operation ID
    pub id: Uuid,
    /// Operation status
    pub status: OperationStatus,
    /// Start timestamp
    pub started_at: u64,
    /// Completion timestamp
    pub completed_at: Option<u64>,
}

/// Operation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Operation is pending
    Pending,
    /// Operation is in progress
    InProgress,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed(String),
}

impl UpgradeOperation {
    #[allow(dead_code)]
    pub fn new(package: String, from_version: PythonVersion, to_version: PythonVersion) -> Self {
        Self {
            package,
            from_version,
            to_version,
            id: Uuid::new_v4(),
            status: OperationStatus::Pending,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            completed_at: None,
        }
    }

    #[allow(dead_code)]
    pub fn complete(&mut self, status: OperationStatus) {
        self.status = status;
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }
} 