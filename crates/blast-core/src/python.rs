use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;
use async_trait::async_trait;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use crate::error::{BlastError, BlastResult};
use crate::package::Package;
use crate::version::VersionConstraint;
use crate::environment::Environment;
use crate::metadata::PackageMetadata;

// Helper function to create package metadata from dependencies
fn create_package_metadata(
    name: String,
    version: String,
    dependencies: HashMap<String, VersionConstraint>,
    python_version: VersionConstraint,
) -> PackageMetadata {
    PackageMetadata::new(
        name,
        version,
        dependencies,
        python_version,
    )
}

/// Python version specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PythonVersion {
    major: u32,
    minor: u32,
    patch: Option<u32>,
}

impl Default for PythonVersion {
    fn default() -> Self {
        Self {
            major: 3,
            minor: 7,
            patch: None,
        }
    }
}

impl PythonVersion {
    /// Create a new Python version
    pub fn new(major: u32, minor: u32, patch: Option<u32>) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Get the major version
    pub fn major(&self) -> u32 {
        self.major
    }

    /// Get the minor version
    pub fn minor(&self) -> u32 {
        self.minor
    }

    /// Get the patch version
    pub fn patch(&self) -> Option<u32> {
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
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 {
            return Err(BlastError::Python(format!(
                "Invalid Python version format: {}",
                version
            )));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| BlastError::Python(format!("Invalid major version: {}", parts[0])))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| BlastError::Python(format!("Invalid minor version: {}", parts[1])))?;
        let patch = if parts.len() > 2 {
            Some(
                parts[2]
                    .parse()
                    .map_err(|_| BlastError::Python(format!("Invalid patch version: {}", parts[2])))?,
            )
        } else {
            None
        };

        Ok(Self {
            major,
            minor,
            patch,
        })
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

impl FromStr for PythonVersion {
    type Err = BlastError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
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

    /// Get the pip executable
    pub fn pip_executable(&self) -> PathBuf {
        self.interpreter_path().join("pip")
    }

    pub fn get_active() -> BlastResult<Option<Self>> {
        if let Ok(path) = std::env::var("BLAST_ENV_PATH") {
            let python_version = if let Ok(version) = std::env::var("BLAST_PYTHON_VERSION") {
                PythonVersion::parse(&version)?
            } else {
                PythonVersion::default()
            };

            Ok(Some(Self::new(PathBuf::from(path), python_version)))
        } else {
            Ok(None)
        }
    }

    /// Check if a package is installed
    pub async fn has_package(&self, package: &Package) -> BlastResult<bool> {
        let packages = self.get_packages()?;
        Ok(packages.iter().any(|p| p.name() == package.name() && p.version() == package.version()))
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

#[async_trait]
impl Environment for PythonEnvironment {
    async fn create(&self) -> BlastResult<()> {
        // Create virtual environment using the system Python
        let output = Command::new("python3")
            .arg("-m")
            .arg("venv")
            .arg(&self.path)
            .output()
            .map_err(|e| BlastError::environment(format!(
                "Failed to create virtual environment: {}", e
            )))?;

        if !output.status.success() {
            return Err(BlastError::environment(format!(
                "Failed to create virtual environment: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    async fn activate(&self) -> BlastResult<()> {
        // No need to actually activate - we'll use full paths to executables
        Ok(())
    }

    async fn deactivate(&self) -> BlastResult<()> {
        // No need to actually deactivate - we'll use full paths to executables
        Ok(())
    }

    async fn install_package(&self, package: &Package) -> BlastResult<()> {
        let pip = self.pip_executable();
        let package_spec = format!("{}=={}", package.name(), package.version());
        
        let output = Command::new(pip)
            .arg("install")
            .arg(&package_spec)
            .output()
            .map_err(|e| BlastError::environment(format!(
                "Failed to execute pip install: {}", e
            )))?;

        if !output.status.success() {
            return Err(BlastError::environment(format!(
                "Failed to install package {}: {}",
                package_spec,
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    async fn uninstall_package(&self, package: &Package) -> BlastResult<()> {
        let pip = self.pip_executable();
        
        let output = Command::new(pip)
            .arg("uninstall")
            .arg("--yes")
            .arg(package.name())
            .output()
            .map_err(|e| BlastError::environment(format!(
                "Failed to execute pip uninstall: {}", e
            )))?;

        if !output.status.success() {
            return Err(BlastError::environment(format!(
                "Failed to uninstall package {}: {}",
                package.name(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    async fn get_packages(&self) -> BlastResult<Vec<Package>> {
        let output = Command::new(&self.pip_executable())
            .args(&["list", "--format=json"])
            .output()?;

        if !output.status.success() {
            return Err(BlastError::CommandFailed(
                "Failed to list packages".to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let packages: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
        
        let result = packages.into_iter()
            .map(|pkg| {
                let name = pkg["name"].as_str().ok_or_else(|| 
                    BlastError::ParseError("Missing package name".to_string())
                )?.to_string();
                
                let version = pkg["version"].as_str().ok_or_else(|| 
                    BlastError::ParseError("Missing package version".to_string())
                )?.to_string();

                Package::new(
                    name.clone(),
                    version.clone(),
                    create_package_metadata(
                        name,
                        version,
                        HashMap::new(),
                        VersionConstraint::any(),
                    ),
                    VersionConstraint::any(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn python_version(&self) -> &PythonVersion {
        &self.python_version
    }

    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
} 