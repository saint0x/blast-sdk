//! Background service for the Blast Python environment manager.
//! 
//! This crate provides a daemon service that monitors Python environments
//! and handles real-time dependency updates.

pub mod error;
pub mod state;
pub mod metrics;
pub mod service;
pub mod monitor;
pub mod transaction;
pub mod update;
pub mod environment;
pub mod activation;
pub mod validation;

// Re-export commonly used types
pub use error::DaemonError;
pub use state::{StateManager, Checkpoint};
pub use metrics::{MetricsManager, PerformanceSnapshot};
pub use service::{DaemonService, Daemon, DaemonConfig};
pub use monitor::PythonResourceMonitor;
pub use environment::{EnvManager, DaemonEnvironment, EnvironmentImage};
pub use activation::ActivationState;
pub use validation::{DependencyValidator, ValidationIssue, ValidationResult, ValidationMetrics};

// Internal module re-exports
pub use monitor::{
    PythonResourceLimits,
    EnvironmentUsage,
    EnvDiskUsage,
    CacheUsage,
};
pub use state::*;
pub use metrics::{
    PackageMetrics,
    EnvironmentMetrics,
}; 