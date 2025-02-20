use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::{
    error::BlastResult,
    package::Package,
};
use super::{PythonVersion, EnvironmentMetadata};

/// Python environment state
pub struct PythonEnvironmentState {
    /// Path to the environment
    pub path: PathBuf,
    /// Python version
    pub python_version: PythonVersion,
    /// Installed packages
    pub packages: Vec<Package>,
    /// Environment creation time
    pub created_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
    /// Environment metadata
    pub metadata: EnvironmentMetadata,
    /// Environment name
    pub name: Option<String>,
    /// Environment version
    pub version: Option<String>,
}

impl PythonEnvironmentState {
    /// Create new environment state
    pub fn new(path: PathBuf, python_version: PythonVersion) -> Self {
        Self {
            path,
            python_version,
            packages: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: EnvironmentMetadata::default(),
            name: None,
            version: None,
        }
    }

    /// Get environment path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get Python version
    pub fn python_version(&self) -> &PythonVersion {
        &self.python_version
    }

    /// Get environment name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get environment version
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Set environment name
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
        self.updated_at = Utc::now();
    }

    /// Set environment version
    pub fn set_version(&mut self, version: String) {
        self.version = Some(version);
        self.updated_at = Utc::now();
    }

    /// Get Python interpreter path
    pub fn interpreter_path(&self) -> PathBuf {
        self.path.join("bin").join(format!(
            "python{}",
            self.python_version
        ))
    }

    /// Check if environment exists
    pub fn exists(&self) -> bool {
        self.path.exists() && self.interpreter_path().exists()
    }

    /// Get installed packages
    pub fn get_packages(&self) -> BlastResult<Vec<Package>> {
        Ok(self.packages.clone())
    }

    /// Add package to environment
    pub fn add_package(&mut self, package: Package) {
        if !self.packages.iter().any(|p| p.name() == package.name()) {
            self.packages.push(package);
            self.updated_at = Utc::now();
        }
    }

    /// Remove package from environment
    pub fn remove_package(&mut self, package: &Package) {
        self.packages.retain(|p| p.name() != package.name());
        self.updated_at = Utc::now();
    }

    /// Get package by name
    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.packages.iter().find(|p| p.name() == name)
    }

    /// Update Python version
    pub fn update_python_version(&mut self, version: &str) -> BlastResult<()> {
        self.python_version = PythonVersion::parse(version)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update environment variable
    pub fn update_env_var(&mut self, key: &str, value: &str) {
        // First check if env_vars exists and create if not
        if !self.metadata.extra.get("env_vars").is_some() {
            let obj = serde_json::Map::new();
            self.metadata.extra.as_object_mut().unwrap()
                .insert("env_vars".to_string(), serde_json::Value::Object(obj));
        }

        // Now we can safely get and modify env_vars
        if let Some(env_vars) = self.metadata.extra.get_mut("env_vars").and_then(|v| v.as_object_mut()) {
            env_vars.insert(
                key.to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }

        self.updated_at = Utc::now();
    }

    /// Get environment variable
    pub fn get_env_var(&self, key: &str) -> Option<String> {
        self.metadata.extra.get("env_vars")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get all environment variables
    pub fn get_env_vars(&self) -> HashMap<String, String> {
        self.metadata.extra.get("env_vars")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        v.as_str().map(|s| (k.clone(), s.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get pip executable path
    pub fn pip_executable(&self) -> PathBuf {
        self.path.join("bin").join("pip")
    }

    /// Get active environment state
    pub fn get_active() -> BlastResult<Option<Self>> {
        // TODO: Implement active environment detection
        Ok(None)
    }

    /// Check if package is installed
    pub async fn has_package(&self, package: &Package) -> BlastResult<bool> {
        Ok(self.packages.iter().any(|p| p.name() == package.name()))
    }
} 