mod manager;
mod types;
mod operations;

pub use manager::SyncManager;
pub use types::SyncOperation;
pub use operations::SyncStatus;

pub mod validation;
pub mod conflicts;
pub use validation::{ValidationIssue, IssueSeverity, PerformanceImpact, SyncValidation};
pub use conflicts::{SyncConflict, ConflictType, ConflictResolution};
pub use types::{SyncChange, CacheSizeLimits, MergeStrategy}; 