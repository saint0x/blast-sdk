pub mod manager;
pub mod operations;
pub mod validation;
pub mod conflicts;
pub mod types;

// Re-export main types
pub use manager::SyncManager;
pub use operations::{SyncOperation, SyncStatus, OperationStatus, UpgradeOperation};
pub use validation::{ValidationIssue, IssueSeverity, PerformanceImpact, SyncValidation};
pub use conflicts::{SyncConflict, ConflictType, ConflictResolution};
pub use types::{SyncChange, CacheSizeLimits, MergeStrategy}; 