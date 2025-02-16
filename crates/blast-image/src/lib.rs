//! Blast Image Management Library
//! 
//! This library provides functionality for managing Python environment images,
//! including creation, validation, and metadata management.

pub mod platform;
pub mod hooks;
pub mod validation;
pub mod packages;
pub mod layer;

pub use platform::{PlatformInfo, PlatformRequirements, GpuRequirements};
pub use hooks::{EnvironmentHooks, PathModifications};
pub use validation::{
    ImageValidator, ValidationResult, ValidationError, ValidationWarning,
    ValidationOptions, ValidationErrorCode, ValidationWarningCode,
};
pub use packages::{
    PackageConfig, PackageDependency, PackageIndex, IndexCredentials,
    DependencyTree,
};
pub use layer::{Layer, CompressionLevel, LayerMetadata};

// Re-export manifest types from blast-core
pub use blast_core::manifest::{
    Manifest, BlastMetadata, SystemDependency, ResourceRequirements,
    VenvConfig, LayerInfo, LayerType, CompressionType,
};

/// Error type for blast-image operations
#[derive(Debug, thiserror::Error)]
pub enum BlastError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Platform error: {0}")]
    Platform(String),
    
    #[error("Layer error: {0}")]
    Layer(String),

    #[error("Package error: {0}")]
    Package(String),
}

/// Result type for blast-image operations
pub type Result<T> = std::result::Result<T, BlastError>;

// Re-export commonly used types
pub use chrono;
pub use blake3;
pub use serde;
pub use url;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_COMPATIBLE_VERSION: &str = "0.1.0";

/// Check if two versions are compatible
pub fn is_compatible_version(version: &str) -> bool {
    // TODO: Implement proper version compatibility check
    version.starts_with("0.1.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        assert!(is_compatible_version("0.1.0"));
        assert!(is_compatible_version("0.1.1"));
        assert!(!is_compatible_version("0.2.0"));
    }
} 