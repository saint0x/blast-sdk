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
        // Validate project root exists
        if !self.project_root.exists() {
            return Err(BlastError::config(format!(
                "Project root does not exist: {}",
                self.project_root.display()
            )));
        }

        // Validate environment directory is relative
        if self.env_dir.is_absolute() {
            return Err(BlastError::config("Environment directory must be relative"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_config_serialization() {
        let config = BlastConfig::new(
            "test-project",
            "1.0.0",
            PythonVersion::from_str("3.8").unwrap(),
            PathBuf::from("/tmp/test-project"),
        );

        let toml = config.to_toml().unwrap();
        let parsed = BlastConfig::from_toml(&toml).unwrap();

        assert_eq!(config.name, parsed.name);
        assert_eq!(config.version, parsed.version);
        assert_eq!(config.python_version, parsed.python_version);
    }

    #[test]
    fn test_config_validation() {
        let config = BlastConfig::new(
            "test-project",
            "1.0.0",
            PythonVersion::from_str("3.8").unwrap(),
            PathBuf::from("/nonexistent/path"),
        );

        assert!(config.validate().is_err());

        let config = BlastConfig {
            env_dir: PathBuf::from("/absolute/path"),
            ..config
        };

        assert!(config.validate().is_err());
    }
} 