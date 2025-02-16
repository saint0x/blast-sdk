use std::path::{Path, PathBuf};
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::error::{BlastError, BlastResult};
use crate::package::Package;

/// Python version specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PythonVersion {
    major: u8,
    minor: u8,
    patch: Option<u8>,
}

impl PythonVersion {
    /// Create a new Python version
    pub fn new(major: u8, minor: u8, patch: Option<u8>) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Get the major version
    pub fn major(&self) -> u8 {
        self.major
    }

    /// Get the minor version
    pub fn minor(&self) -> u8 {
        self.minor
    }

    /// Get the patch version
    pub fn patch(&self) -> Option<u8> {
        self.patch
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self.patch {
            Some(patch) => format!("{}.{}.{}", self.major, self.minor, patch),
            None => format!("{}.{}", self.major, self.minor),
        }
    }

    /// Check if this version is compatible with another version
    pub fn is_compatible_with(&self, other: &PythonVersion) -> bool {
        self.major == other.major && self.minor <= other.minor
    }

    pub fn parse(version: &str) -> BlastResult<Self> {
        Self::from_str(version)
    }
}

impl FromStr for PythonVersion {
    type Err = BlastError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        match parts.len() {
            2 => Ok(Self::new(
                parts[0].parse().map_err(|_| BlastError::version("Invalid major version"))?,
                parts[1].parse().map_err(|_| BlastError::version("Invalid minor version"))?,
                None,
            )),
            3 => Ok(Self::new(
                parts[0].parse().map_err(|_| BlastError::version("Invalid major version"))?,
                parts[1].parse().map_err(|_| BlastError::version("Invalid minor version"))?,
                Some(parts[2].parse().map_err(|_| BlastError::version("Invalid patch version"))?),
            )),
            _ => Err(BlastError::version("Invalid Python version format")),
        }
    }
}

impl std::fmt::Display for PythonVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.patch {
            Some(patch) => write!(f, "{}.{}.{}", self.major, self.minor, patch),
            None => write!(f, "{}.{}", self.major, self.minor),
        }
    }
}

impl PartialOrd for PythonVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PythonVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => match (self.patch, other.patch) {
                    (None, None) => Ordering::Equal,
                    (None, Some(_)) => Ordering::Less,
                    (Some(_), None) => Ordering::Greater,
                    (Some(a), Some(b)) => a.cmp(&b),
                },
                ord => ord,
            },
            ord => ord,
        }
    }
}

/// Python environment state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonEnvironment {
    /// Path to the environment
    pub path: PathBuf,
    /// Python version
    pub python_version: PythonVersion,
    /// Installed packages
    pub packages: Vec<Package>,
    /// Environment creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last update time
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Environment metadata
    pub metadata: EnvironmentMetadata,
    /// Environment name
    pub name: Option<String>,
    /// Environment version
    pub version: Option<String>,
}

impl PythonEnvironment {
    /// Create a new Python environment
    pub fn new(path: PathBuf, python_version: PythonVersion) -> Self {
        let now = chrono::Utc::now();
        Self {
            path,
            python_version,
            packages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: EnvironmentMetadata::default(),
            name: None,
            version: None,
        }
    }

    /// Get the environment path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the Python version
    pub fn python_version(&self) -> &PythonVersion {
        &self.python_version
    }

    /// Get the environment name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the environment version
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Set the environment name
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Set the environment version
    pub fn set_version(&mut self, version: String) {
        self.version = Some(version);
    }

    /// Get the Python interpreter path
    pub fn interpreter_path(&self) -> PathBuf {
        #[cfg(windows)]
        let python_exe = "python.exe";
        #[cfg(not(windows))]
        let python_exe = "python";

        self.path.join("bin").join(python_exe)
    }

    /// Check if the environment exists
    pub fn exists(&self) -> bool {
        self.interpreter_path().exists()
    }

    /// Get the packages in the environment
    pub fn get_packages(&self) -> BlastResult<Vec<Package>> {
        Ok(self.packages.clone())
    }

    /// Add a package to the environment
    pub fn add_package(&mut self, package: Package) {
        self.packages.push(package);
        self.updated_at = chrono::Utc::now();
    }

    /// Remove a package from the environment
    pub fn remove_package(&mut self, package: &Package) {
        self.packages.retain(|p| p.name() != package.name());
        self.updated_at = chrono::Utc::now();
    }

    /// Get a package by name
    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.packages.iter().find(|p| p.name() == name)
    }

    /// Update the Python version
    pub fn update_python_version(&mut self, version: &str) -> BlastResult<()> {
        let new_version = PythonVersion::parse(version)?;
        self.python_version = new_version;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Update an environment variable
    pub fn update_env_var(&mut self, key: &str, value: &str) {
        let env_vars = match self.metadata.extra.as_object_mut() {
            Some(obj) => {
                if !obj.contains_key("env_vars") {
                    obj.insert("env_vars".to_string(), serde_json::Value::Object(serde_json::Map::new()));
                }
                obj.get_mut("env_vars").unwrap().as_object_mut().unwrap()
            }
            None => {
                let mut map = serde_json::Map::new();
                map.insert("env_vars".to_string(), serde_json::Value::Object(serde_json::Map::new()));
                self.metadata.extra = serde_json::Value::Object(map);
                self.metadata.extra.as_object_mut().unwrap().get_mut("env_vars").unwrap().as_object_mut().unwrap()
            }
        };

        env_vars.insert(key.to_string(), serde_json::Value::String(value.to_string()));
        self.updated_at = chrono::Utc::now();
    }

    /// Get an environment variable
    pub fn get_env_var(&self, key: &str) -> Option<String> {
        self.metadata.extra
            .as_object()
            .and_then(|obj| obj.get("env_vars"))
            .and_then(|env_vars| env_vars.as_object())
            .and_then(|env_vars| env_vars.get(key))
            .and_then(|value| value.as_str())
            .map(|s| s.to_string())
    }

    /// Get all environment variables
    pub fn get_env_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        if let Some(obj) = self.metadata.extra.as_object() {
            if let Some(env_vars) = obj.get("env_vars").and_then(|v| v.as_object()) {
                for (key, value) in env_vars {
                    if let Some(value_str) = value.as_str() {
                        vars.insert(key.clone(), value_str.to_string());
                    }
                }
            }
        }
        vars
    }
}

/// Metadata for Python environments
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentMetadata {
    /// Environment name
    pub name: Option<String>,
    /// Environment description
    pub description: Option<String>,
    /// Custom tags
    pub tags: Vec<String>,
    /// Additional metadata
    pub extra: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_version_parsing() {
        assert!(PythonVersion::parse("3.8").is_ok());
        assert!(PythonVersion::parse("3.8.0").is_ok());
        assert!(PythonVersion::parse("3").is_err());
        assert!(PythonVersion::parse("invalid").is_err());
    }

    #[test]
    fn test_python_version_compatibility() {
        let v1 = PythonVersion::from_str("3.8").unwrap();
        let v2 = PythonVersion::from_str("3.9").unwrap();
        let v3 = PythonVersion::from_str("3.7").unwrap();
        let v4 = PythonVersion::from_str("2.7").unwrap();

        assert!(v1.is_compatible_with(&v2));
        assert!(!v2.is_compatible_with(&v1));
        assert!(v1.is_compatible_with(&v3));
        assert!(!v1.is_compatible_with(&v4));
    }

    #[test]
    fn test_environment_management() {
        let mut env = PythonEnvironment::new(
            PathBuf::from("/tmp/test-env"),
            PythonVersion::from_str("3.8").unwrap(),
        );

        let package = Package::new(
            crate::package::PackageId::new(
                "test-package",
                crate::package::Version::parse("1.0.0").unwrap(),
            ),
            Default::default(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        env.add_package(package.clone());
        assert_eq!(env.packages.len(), 1);

        env.remove_package(&package);
        assert_eq!(env.packages.len(), 0);
    }
} 