//! Blast Image Management Library
//! 
//! This library provides functionality for managing Python environment images,
//! including creation, validation, and metadata management.

pub mod platform;
pub mod hooks;
pub mod validation;
pub mod packages;
pub mod layer;
pub mod compression;
pub mod error;

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
pub use layer::{Layer, LayerType, LayerMetadata};
pub use compression::{
    CompressionType, CompressionLevel, CompressionStrategy,
    compression_ratio, create_strategy,
};
pub use error::{Error, Result};

// Re-export manifest types from blast-core
pub use blast_core::manifest::{
    Manifest, BlastMetadata, SystemDependency, ResourceRequirements,
    VenvConfig, LayerInfo,
};

// Re-export commonly used types
pub use chrono;
pub use blake3;
pub use serde;
pub use url;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_COMPATIBLE_VERSION: &str = "0.1.0";

use blast_core::Version;

/// Check if two versions are compatible
pub fn is_compatible_version(version: &str) -> bool {
    if let Ok(version) = Version::parse(version) {
        let version_str = version.to_string();
        let parts: Vec<&str> = version_str.split('.').collect();
        parts.get(0).map_or(false, |major| *major == "0") &&
        parts.get(1).map_or(false, |minor| *minor == "1")
    } else {
        false
    }
} 