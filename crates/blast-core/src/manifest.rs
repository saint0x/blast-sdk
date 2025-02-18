/// Blast environment manifest module
/// Contains types and functions for managing Blast environment manifests
/// Provides functionality for serializing and deserializing environment configurations
/// Core manifest types and traits for managing environment manifests

use std::path::PathBuf;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::python::PythonVersion;
use crate::security::SecurityPolicy;
use crate::environment::Environment;
use crate::error::{BlastError, BlastResult};
use crate::package::Package;

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
    /// Environment name
    pub name: Option<String>,
    /// Environment version
    pub version: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
    /// Python version with exact patch level
    pub python_version: PythonVersion,
    /// Author information
    pub author: Option<String>,
    /// Environment description
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

/// Manifest for a Blast environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Metadata about the environment
    pub metadata: BlastMetadata,
    /// Installed packages with versions
    pub packages: Vec<Package>,
    /// Manifest format version
    pub format_version: String,
    /// Python version
    pub python_version: PythonVersion,
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

impl Default for Manifest {
    fn default() -> Self {
        Self {
            packages: Vec::new(),
            metadata: BlastMetadata {
                name: None,
                version: "0.1.0".to_string(),
                description: None,
                created_at: Utc::now(),
                modified_at: Utc::now(),
                python_version: PythonVersion::new(3, 8, None),
                author: None,
                license: None,
                env_vars: HashMap::new(),
                tags: Vec::new(),
                dependencies: HashMap::new(),
                transitive_deps: HashMap::new(),
                system_deps: Vec::new(),
                platform_requirements: PlatformRequirements::default(),
                security_policy: SecurityPolicy::default(),
                resources: ResourceRequirements::default(),
                env_hooks: HashMap::new(),
                venv_config: VenvConfig::default(),
                content_hash: String::new(),
                custom: HashMap::new(),
                layers: Vec::new(),
            },
            format_version: "1.0.0".to_string(),
            python_version: PythonVersion::new(3, 8, None),
        }
    }
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            metadata: BlastMetadata {
                name: None,
                version: "0.1.0".to_string(),
                created_at: Utc::now(),
                modified_at: Utc::now(),
                python_version: PythonVersion::new(3, 8, None),
                author: None,
                description: None,
                license: None,
                env_vars: HashMap::new(),
                tags: Vec::new(),
                dependencies: HashMap::new(),
                transitive_deps: HashMap::new(),
                system_deps: Vec::new(),
                platform_requirements: PlatformRequirements::default(),
                security_policy: SecurityPolicy::default(),
                resources: ResourceRequirements::default(),
                env_hooks: HashMap::new(),
                venv_config: VenvConfig::default(),
                content_hash: String::new(),
                custom: HashMap::new(),
                layers: Vec::new(),
            },
            format_version: "1.0.0".to_string(),
            python_version: PythonVersion::new(3, 8, None),
        }
    }

    pub async fn from_environment<E: Environment>(env: &E) -> BlastResult<Self> {
        let packages = env.get_packages().await?;
        Ok(Self {
            packages,
            metadata: BlastMetadata {
                name: env.name().map(ToString::to_string),
                version: "0.1.0".to_string(),
                created_at: Utc::now(),
                modified_at: Utc::now(),
                python_version: env.python_version().clone(),
                author: None,
                description: None,
                license: None,
                env_vars: HashMap::new(),
                tags: Vec::new(),
                dependencies: HashMap::new(),
                transitive_deps: HashMap::new(),
                system_deps: Vec::new(),
                platform_requirements: PlatformRequirements::default(),
                security_policy: SecurityPolicy::default(),
                resources: ResourceRequirements::default(),
                env_hooks: HashMap::new(),
                venv_config: VenvConfig::default(),
                content_hash: String::new(),
                custom: HashMap::new(),
                layers: Vec::new(),
            },
            format_version: "1.0.0".to_string(),
            python_version: env.python_version().clone(),
        })
    }

    pub async fn save(&self, path: PathBuf) -> BlastResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| BlastError::Manifest(e.to_string()))?;
        fs::write(&path, content)
            .await
            .map_err(|e| BlastError::Io(e.to_string()))?;
        Ok(())
    }

    pub async fn load(path: PathBuf) -> BlastResult<Self> {
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| BlastError::Io(e.to_string()))?;
        serde_json::from_str(&content)
            .map_err(|e| BlastError::Manifest(e.to_string()))
    }

    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    pub fn add_package(&mut self, package: Package) {
        self.packages.push(package);
    }

    pub fn remove_package(&mut self, name: &str) {
        self.packages.retain(|p| p.name() != name);
    }
}

impl<T: Environment> From<&T> for Manifest {
    fn from(env: &T) -> Self {
        Self {
            metadata: BlastMetadata {
                name: env.name().map(ToString::to_string),
                version: "0.1.0".to_string(),
                description: None,
                created_at: Utc::now(),
                modified_at: Utc::now(),
                python_version: env.python_version().clone(),
                author: None,
                license: None,
                env_vars: HashMap::new(),
                tags: Vec::new(),
                dependencies: HashMap::new(),
                transitive_deps: HashMap::new(),
                system_deps: Vec::new(),
                platform_requirements: PlatformRequirements::default(),
                security_policy: SecurityPolicy::default(),
                resources: ResourceRequirements::default(),
                env_hooks: HashMap::new(),
                venv_config: VenvConfig::default(),
                content_hash: String::new(),
                custom: HashMap::new(),
                layers: Vec::new(),
            },
            packages: Vec::new(),
            format_version: "1.0.0".to_string(),
            python_version: env.python_version().clone(),
        }
    }
} 