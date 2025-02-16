//! Image manifest handling
//! 
//! This module provides functionality for managing image manifests,
//! including metadata, layer information, and environment configuration.

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use blast_core::error::{BlastError, BlastResult};
use blast_core::python::{PythonEnvironment, PythonVersion};
use blast_core::security::SecurityPolicy;

use crate::layer::{Layer, LayerType, CompressionType};
use crate::platform::PlatformRequirements;
use crate::hooks::EnvironmentHooks;
use crate::packages::PackageIndex;

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
    pub platform: PlatformRequirements,
    /// Security policy
    #[serde(skip)]
    pub security_policy: SecurityPolicy,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Python package index URLs
    pub package_indexes: Vec<PackageIndex>,
    /// Environment hooks (pre/post activation)
    pub hooks: EnvironmentHooks,
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

impl Manifest {
    /// Create a new manifest from a Python environment
    pub fn from_environment(env: &PythonEnvironment) -> BlastResult<Self> {
        let metadata = BlastMetadata::new(
            env.name().unwrap_or("unnamed").to_string(),
            env.python_version().clone(),
            SecurityPolicy::default(),
        );

        Ok(Self {
            metadata,
            packages: Vec::new(),
            format_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Save manifest to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> BlastResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| BlastError::serialization(e.to_string()))?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load manifest from a file
    pub fn load<P: AsRef<Path>>(path: P) -> BlastResult<Self> {
        let content = fs::read_to_string(path)?;
        toml::from_str(&content)
            .map_err(|e| BlastError::serialization(e.to_string()))
    }

    /// Convert manifest to a Python environment
    pub fn to_environment(&self, target_path: &PathBuf) -> BlastResult<PythonEnvironment> {
        let env = PythonEnvironment::new(
            target_path.clone(),
            self.metadata.python_version.clone(),
        );

        // TODO: Apply environment configuration from manifest
        Ok(env)
    }

    /// Add a layer to the manifest
    pub fn add_layer(&mut self, layer: &Layer) {
        self.metadata.layers.push(LayerInfo {
            id: layer.name.clone(),
            layer_type: LayerType::Base, // TODO: Determine layer type
            size: layer.metadata.original_size,
            compression: CompressionType::Zstd,
            hash: layer.metadata.hash.clone(),
            created_at: layer.metadata.created_at,
        });
    }

    /// Add environment variables
    pub fn add_env_vars(&mut self, vars: HashMap<String, String>) {
        self.metadata.env_vars.extend(vars);
    }

    /// Add tags
    pub fn add_tags(&mut self, tags: Vec<String>) {
        self.metadata.tags.extend(tags);
    }

    /// Add custom metadata
    pub fn add_custom_metadata(&mut self, metadata: HashMap<String, String>) {
        self.metadata.custom.extend(metadata);
    }

    /// Get total size of all layers
    pub fn total_size(&self) -> u64 {
        self.metadata.layers.iter().map(|l| l.size).sum()
    }

    /// Record a package installation
    pub fn record_package_install(&mut self, package_name: String, version: String) {
        self.packages.push(format!("{}=={}", package_name, version));
        self.metadata.dependencies.insert(package_name, version);
        self.metadata.modified_at = Utc::now();
        self.metadata.update_hash();
    }

    /// Record package removal
    pub fn record_package_removal(&mut self, package_name: &str) {
        self.packages.retain(|p| !p.starts_with(&format!("{}==", package_name)));
        self.metadata.dependencies.remove(package_name);
        self.metadata.modified_at = Utc::now();
        self.metadata.update_hash();
    }

    /// Record environment variable change
    pub fn record_env_var_change(&mut self, key: String, value: String) {
        self.metadata.env_vars.insert(key, value);
        self.metadata.modified_at = Utc::now();
        self.metadata.update_hash();
    }

    /// Record system dependency
    pub fn record_system_dependency(&mut self, dependency: SystemDependency) {
        self.metadata.system_deps.push(dependency);
        self.metadata.modified_at = Utc::now();
        self.metadata.update_hash();
    }

    /// Update platform requirements
    pub fn update_platform_requirements(&mut self, requirements: PlatformRequirements) {
        self.metadata.platform = requirements;
        self.metadata.modified_at = Utc::now();
        self.metadata.update_hash();
    }

    /// Record hook addition
    pub fn record_hook_addition(&mut self, hook_type: &str, command: String) {
        match hook_type {
            "pre-activate" => self.metadata.hooks.add_pre_activate(command),
            "post-activate" => self.metadata.hooks.add_post_activate(command),
            "pre-deactivate" => self.metadata.hooks.add_pre_deactivate(command),
            "post-deactivate" => self.metadata.hooks.add_post_deactivate(command),
            _ => return,
        }
        self.metadata.modified_at = Utc::now();
        self.metadata.update_hash();
    }

    /// Get package version
    pub fn get_package_version(&self, package_name: &str) -> Option<&String> {
        self.metadata.dependencies.get(package_name)
    }

    /// Check if package is installed
    pub fn has_package(&self, package_name: &str) -> bool {
        self.metadata.dependencies.contains_key(package_name)
    }

    /// Get all installed packages with versions
    pub fn get_installed_packages(&self) -> &Vec<String> {
        &self.packages
    }

    /// Get direct dependencies only
    pub fn get_direct_dependencies(&self) -> HashMap<String, String> {
        self.metadata.dependencies.clone()
    }

    /// Get transitive dependencies
    pub fn get_transitive_dependencies(&self) -> HashMap<String, String> {
        self.metadata.transitive_deps.clone()
    }

    /// Record transitive dependency
    pub fn record_transitive_dependency(&mut self, package_name: String, version: String) {
        self.metadata.transitive_deps.insert(package_name, version);
        self.metadata.modified_at = Utc::now();
        self.metadata.update_hash();
    }
}

impl BlastMetadata {
    /// Create new metadata
    pub fn new(name: String, python_version: PythonVersion, security_policy: SecurityPolicy) -> Self {
        Self {
            name,
            version: "0.1.0".to_string(),
            created_at: Utc::now(),
            modified_at: Utc::now(),
            python_version,
            author: None,
            description: None,
            license: None,
            env_vars: HashMap::new(),
            tags: Vec::new(),
            dependencies: HashMap::new(),
            transitive_deps: HashMap::new(),
            system_deps: Vec::new(),
            platform: PlatformRequirements::default(),
            security_policy,
            resources: ResourceRequirements::default(),
            package_indexes: Vec::new(),
            hooks: EnvironmentHooks::default(),
            venv_config: VenvConfig::default(),
            content_hash: String::new(),
            custom: HashMap::new(),
            layers: Vec::new(),
        }
    }

    /// Save metadata to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> BlastResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| BlastError::serialization(e.to_string()))?;
        fs::write(path.as_ref().join("blast.toml"), content)?;
        Ok(())
    }

    /// Load metadata from a file
    pub fn load<P: AsRef<Path>>(path: P) -> BlastResult<Self> {
        let content = fs::read_to_string(path.as_ref().join("blast.toml"))?;
        toml::from_str(&content)
            .map_err(|e| BlastError::serialization(e.to_string()))
    }

    /// Update content hash
    pub fn update_hash(&mut self) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.name.as_bytes());
        hasher.update(self.version.as_bytes());
        hasher.update(self.python_version.to_string().as_bytes());
        
        for layer in &self.layers {
            hasher.update(layer.hash.as_bytes());
        }

        self.content_hash = hasher.finalize().to_hex().to_string();
    }

    /// Verify metadata integrity
    pub fn verify(&self) -> BlastResult<bool> {
        let mut temp = self.clone();
        temp.update_hash();
        Ok(temp.content_hash == self.content_hash)
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            max_memory: 1024 * 1024 * 1024 * 2, // 2GB
            max_disk: 1024 * 1024 * 1024 * 10,  // 10GB
            cpu_limit: None,
            network_limit: None,
            max_processes: 32,
            temp_storage: 1024 * 1024 * 512,    // 512MB
        }
    }
}

impl Default for VenvConfig {
    fn default() -> Self {
        Self {
            python_path: PathBuf::from("bin/python"),
            site_packages: PathBuf::from("lib/python3/site-packages"),
            system_site_packages: false,
            prompt: "(blast)".to_string(),
            python_path_additions: Vec::new(),
            symlinks: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_manifest_creation() {
        let temp = tempdir().unwrap();
        let env = PythonEnvironment::new(
            temp.path().to_path_buf(),
            PythonVersion::parse("3.8").unwrap(),
        );

        let manifest = Manifest::from_environment(&env).unwrap();
        assert_eq!(manifest.metadata.name, "unnamed");
        assert_eq!(manifest.metadata.python_version, "3.8");
    }

    #[test]
    fn test_manifest_serialization() {
        let temp = tempdir().unwrap();
        let env = PythonEnvironment::new(
            temp.path().to_path_buf(),
            PythonVersion::parse("3.8").unwrap(),
        );

        let manifest = Manifest::from_environment(&env).unwrap();
        
        // Save manifest
        let manifest_path = temp.path().join("manifest.toml");
        manifest.save(&manifest_path).unwrap();
        
        // Load manifest
        let loaded = Manifest::load(&manifest_path).unwrap();
        
        assert_eq!(loaded.metadata.name, manifest.metadata.name);
        assert_eq!(loaded.metadata.python_version, manifest.metadata.python_version);
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = BlastMetadata::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            SecurityPolicy::default(),
        );

        let temp = tempdir().unwrap();
        metadata.save(temp.path()).unwrap();
        
        let loaded = BlastMetadata::load(temp.path()).unwrap();
        assert_eq!(loaded.name, metadata.name);
        assert_eq!(loaded.python_version, metadata.python_version);
    }

    #[test]
    fn test_content_hash() {
        let mut metadata = BlastMetadata::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            SecurityPolicy::default(),
        );

        metadata.update_hash();
        let original_hash = metadata.content_hash.clone();
        
        // Modify metadata
        metadata.name = "modified".to_string();
        metadata.update_hash();
        
        assert_ne!(metadata.content_hash, original_hash);
    }

    #[test]
    fn test_package_tracking() {
        let temp = tempdir().unwrap();
        let env = PythonEnvironment::new(
            temp.path().to_path_buf(),
            PythonVersion::parse("3.8").unwrap(),
        );

        let mut manifest = Manifest::from_environment(&env).unwrap();
        
        // Test package installation
        manifest.record_package_install("numpy".to_string(), "1.21.0".to_string());
        assert!(manifest.has_package("numpy"));
        assert_eq!(manifest.get_package_version("numpy"), Some(&"1.21.0".to_string()));

        // Test package removal
        manifest.record_package_removal("numpy");
        assert!(!manifest.has_package("numpy"));
    }

    #[test]
    fn test_environment_tracking() {
        let temp = tempdir().unwrap();
        let env = PythonEnvironment::new(
            temp.path().to_path_buf(),
            PythonVersion::parse("3.8").unwrap(),
        );

        let mut manifest = Manifest::from_environment(&env).unwrap();
        
        // Test env var tracking
        manifest.record_env_var_change("TEST_VAR".to_string(), "test_value".to_string());
        assert_eq!(manifest.metadata.env_vars.get("TEST_VAR"), Some(&"test_value".to_string()));

        // Test hook tracking
        manifest.record_hook_addition("pre-activate", "echo 'activating'".to_string());
        assert!(manifest.metadata.hooks.pre_activate.contains(&"echo 'activating'".to_string()));
    }
} 