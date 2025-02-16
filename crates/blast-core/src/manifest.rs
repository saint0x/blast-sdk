//! Core manifest types and traits
//! 
//! This module provides the fundamental types for managing environment manifests.

use std::path::PathBuf;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::python::PythonVersion;
use crate::security::SecurityPolicy;

/// Layer type for image layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerType {
    /// Base Python installation
    Base,
    /// Package installations
    Packages,
    /// Custom files
    Custom,
    /// Configuration
    Config,
}

/// Compression type for layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// Zstandard compression
    Zstd,
    /// GZIP compression
    Gzip,
}

/// Comprehensive metadata for Blast environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastMetadata {
    /// Image name
    pub name: String,
    /// Image version
    pub version: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
    /// Python version with exact patch level
    pub python_version: PythonVersion,
    /// Author information
    pub author: Option<String>,
    /// Description
    pub description: Option<String>,
    /// License
    pub license: Option<String>,
    /// Environment variables required
    pub env_vars: HashMap<String, String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Direct dependencies with exact versions
    pub dependencies: HashMap<String, String>,
    /// All transitive dependencies with versions
    pub transitive_deps: HashMap<String, String>,
    /// System packages required (apt, brew, etc)
    pub system_deps: Vec<SystemDependency>,
    /// Platform requirements
    pub platform_requirements: PlatformRequirements,
    /// Security policy
    #[serde(skip)]
    pub security_policy: SecurityPolicy,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Environment hooks (pre/post activation)
    pub env_hooks: HashMap<String, Vec<String>>,
    /// Virtual environment configuration
    pub venv_config: VenvConfig,
    /// Content hash for integrity verification
    pub content_hash: String,
    /// Custom metadata
    pub custom: HashMap<String, String>,
    /// Image layers information
    pub layers: Vec<LayerInfo>,
}

/// System dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDependency {
    /// Package name
    pub name: String,
    /// Version constraint
    pub version: String,
    /// Package manager (apt, brew, etc)
    pub package_manager: String,
    /// Installation commands if custom
    pub install_commands: Option<Vec<String>>,
}

/// Platform requirements for the environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformRequirements {
    /// Operating system requirements
    pub os: Vec<String>,
    /// CPU architecture
    pub arch: Vec<String>,
    /// Minimum CPU cores
    pub min_cores: u32,
    /// Minimum memory in bytes
    pub min_memory: u64,
    /// Minimum disk space in bytes
    pub min_disk_space: u64,
    /// Required system features
    pub required_features: Vec<String>,
}

/// Resource requirements for the environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// Maximum memory usage
    pub max_memory: u64,
    /// Maximum disk usage
    pub max_disk: u64,
    /// CPU usage limits
    pub cpu_limit: Option<f64>,
    /// Network bandwidth limits
    pub network_limit: Option<u64>,
    /// Maximum number of processes
    pub max_processes: u32,
    /// Temporary storage requirements
    pub temp_storage: u64,
}

/// Virtual environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenvConfig {
    /// Python executable path
    pub python_path: PathBuf,
    /// Site-packages directory
    pub site_packages: PathBuf,
    /// Include system site-packages
    pub system_site_packages: bool,
    /// Prompt prefix
    pub prompt: String,
    /// Additional paths to add to PYTHONPATH
    pub python_path_additions: Vec<PathBuf>,
    /// Symlinks to create
    pub symlinks: HashMap<PathBuf, PathBuf>,
}

/// Layer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerInfo {
    /// Layer ID
    pub id: String,
    /// Layer type (base, packages, etc.)
    pub layer_type: LayerType,
    /// Layer size in bytes
    pub size: u64,
    /// Layer compression type
    pub compression: CompressionType,
    /// Layer hash for integrity verification
    pub hash: String,
    /// Layer creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Image manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Manifest metadata
    pub metadata: BlastMetadata,
    /// Installed packages with versions
    pub packages: Vec<String>,
    /// Manifest format version
    pub format_version: String,
}

impl Default for PlatformRequirements {
    fn default() -> Self {
        Self {
            os: vec!["linux".to_string(), "darwin".to_string()],
            arch: vec!["x86_64".to_string(), "aarch64".to_string()],
            min_cores: 1,
            min_memory: 1024 * 1024 * 1024, // 1GB
            min_disk_space: 5 * 1024 * 1024 * 1024, // 5GB
            required_features: Vec::new(),
        }
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            max_memory: 4 * 1024 * 1024 * 1024, // 4GB
            max_disk: 10 * 1024 * 1024 * 1024,  // 10GB
            cpu_limit: None,
            network_limit: None,
            max_processes: 32,
            temp_storage: 1024 * 1024 * 1024,   // 1GB
        }
    }
}

impl Default for VenvConfig {
    fn default() -> Self {
        Self {
            python_path: PathBuf::from("bin/python"),
            site_packages: PathBuf::from("lib/python/site-packages"),
            system_site_packages: false,
            prompt: "(.venv)".to_string(),
            python_path_additions: Vec::new(),
            symlinks: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_platform_requirements_default() {
        let requirements = PlatformRequirements::default();
        assert!(requirements.os.contains(&"linux".to_string()));
        assert!(requirements.arch.contains(&"x86_64".to_string()));
        assert_eq!(requirements.min_cores, 1);
    }

    #[test]
    fn test_resource_requirements_default() {
        let requirements = ResourceRequirements::default();
        assert!(requirements.max_memory > 0);
        assert!(requirements.max_disk > 0);
        assert_eq!(requirements.max_processes, 32);
    }

    #[test]
    fn test_venv_config_default() {
        let config = VenvConfig::default();
        assert!(!config.system_site_packages);
        assert_eq!(config.prompt, "(.venv)");
    }
} 