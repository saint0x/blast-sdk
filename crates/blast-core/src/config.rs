use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::{BlastError, BlastResult};
use crate::python::PythonVersion;
use crate::types::{CacheSettings, UpdateStrategy};

/// Configuration for a Blast environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastConfig {
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
    /// Required Python version
    pub python_version: PythonVersion,
    /// Update strategy
    pub update_strategy: UpdateStrategy,
    /// Cache settings
    pub cache_settings: CacheSettings,
    /// Project root directory
    pub project_root: PathBuf,
    /// Environment directory (relative to project root)
    pub env_dir: PathBuf,
    /// Dependencies configuration
    pub dependencies: DependenciesConfig,
    /// Development dependencies configuration
    pub dev_dependencies: Option<DependenciesConfig>,
}

impl BlastConfig {
    /// Create a new configuration
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        python_version: PythonVersion,
        project_root: PathBuf,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            python_version,
            update_strategy: UpdateStrategy::default(),
            cache_settings: CacheSettings::default(),
            project_root,
            env_dir: PathBuf::from(".venv"),
            dependencies: DependenciesConfig::default(),
            dev_dependencies: None,
        }
    }

    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> BlastResult<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    /// Save configuration to a TOML file
    pub fn save(&self) -> BlastResult<()> {
        let config_path = self.project_root.join("blast.toml");
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(config_path, contents)?;
        Ok(())
    }

    /// Get the absolute path to the environment directory
    pub fn env_path(&self) -> PathBuf {
        self.project_root.join(&self.env_dir)
    }

    /// Validate the configuration
    pub fn validate(&self) -> BlastResult<()> {
        if !self.env_dir.exists() {
            return Err(BlastError::config(format!(
                "Environment directory does not exist: {}",
                self.env_dir.display()
            )));
        }

        if self.env_dir.is_absolute() {
            return Err(BlastError::config(
                "Environment directory must be relative to project root"
            ));
        }

        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn python_version(&self) -> &PythonVersion {
        &self.python_version
    }

    pub fn root_dir(&self) -> &Path {
        &self.project_root
    }

    /// Convert config to TOML string
    pub fn to_toml(&self) -> BlastResult<String> {
        toml::to_string(self).map_err(|e| BlastError::Config(format!("Failed to serialize config: {}", e)))
    }

    /// Create config from TOML string
    pub fn from_toml(content: &str) -> BlastResult<Self> {
        toml::from_str(content).map_err(|e| BlastError::Config(format!("Failed to parse config: {}", e)))
    }
}

/// Configuration for dependencies
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependenciesConfig {
    /// Package dependencies with version constraints
    pub packages: Vec<DependencySpec>,
    /// Additional package indexes
    pub package_index: Option<Vec<String>>,
    /// Allow pre-releases
    pub allow_prereleases: bool,
}

/// Specification for a package dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySpec {
    /// Package name
    pub name: String,
    /// Version constraint
    pub version: String,
    /// Optional extras
    pub extras: Option<Vec<String>>,
    /// Optional package index
    pub index: Option<String>,
} 