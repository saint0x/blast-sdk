use std::collections::HashMap;
use std::path::PathBuf;

use crate::environment::{Environment, EnvironmentImpl};
use crate::python::{PythonEnvironment, PythonVersion};
use crate::package::Package;
use crate::version::VersionConstraint;
use crate::manifest::Manifest;
use crate::error::{BlastError, BlastResult};
use crate::metadata::PackageMetadata;

/// Python environment binding
pub struct PythonEnvironmentBinding {
    /// Inner environment
    inner: Box<dyn Environment>,
}

impl PythonEnvironmentBinding {
    /// Create new Python environment binding
    pub async fn new(
        name: String,
        path: PathBuf,
        python_version: PythonVersion,
    ) -> BlastResult<Self> {
        let config = crate::environment::EnvironmentConfig {
            name,
            path,
            python_version: python_version.to_string(),
            isolation: crate::environment::IsolationLevel::Process,
            resource_limits: crate::environment::ResourceLimits::default(),
            security: crate::environment::SecurityConfig::default(),
        };

        let inner = Box::new(EnvironmentImpl::new(config).await?) as Box<dyn Environment>;
        Ok(Self { inner })
    }

    /// Initialize environment
    pub async fn init(&self) -> BlastResult<()> {
        self.inner.init().await
    }

    /// Install package
    pub async fn install_package(&self, name: String, version: Option<String>) -> BlastResult<()> {
        self.inner.install_package(name, version).await
    }

    /// Uninstall package
    pub async fn uninstall_package(&self, name: String) -> BlastResult<()> {
        self.inner.uninstall_package(name).await
    }

    /// Update package
    pub async fn update_package(&self, name: String, version: String) -> BlastResult<()> {
        self.inner.update_package(name, version).await
    }

    /// Check package conflicts
    pub async fn check_package_conflicts(&self) -> BlastResult<Vec<String>> {
        self.inner.check_package_conflicts().await
    }

    /// Intercept pip operation
    pub async fn intercept_pip(&self, args: Vec<String>) -> BlastResult<()> {
        self.inner.intercept_pip(args).await
    }

    /// Get environment path
    pub fn path(&self) -> &PathBuf {
        self.inner.path()
    }

    /// Get Python version
    pub fn python_version(&self) -> &str {
        self.inner.python_version()
    }

    /// Get environment name
    pub fn name(&self) -> &str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_python_environment_binding() {
        let temp_dir = TempDir::new().unwrap();
        let version = PythonVersion::new(3, 9, Some(0));

        let env = PythonEnvironmentBinding::new(
            "test-env".to_string(),
            temp_dir.path().to_path_buf(),
            version,
        ).await.unwrap();

        // Test initialization
        env.init().await.unwrap();

        // Test package management
        env.install_package("requests".to_string(), None).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Test conflicts
        let conflicts = env.check_package_conflicts().await.unwrap();
        assert!(conflicts.is_empty());
    }
}

/// Native Python environment wrapper
#[derive(Clone)]
pub struct NativeEnvironment {
    inner: PythonEnvironment,
}

impl NativeEnvironment {
    /// Create a new Python environment
    pub async fn new(path: String, python_version: String) -> BlastResult<Self> {
        let version = PythonVersion::parse(&python_version)?;
        let env = PythonEnvironment::new(
            "default".to_string(),
            PathBuf::from(path),
            version,
        ).await?;
        Ok(Self { inner: env })
    }

    /// Get the environment path
    pub fn path(&self) -> String {
        self.inner.path().display().to_string()
    }

    /// Get the Python version
    pub fn python_version(&self) -> String {
        self.inner.python_version().to_string()
    }

    /// Install a package in the environment
    pub async fn install_package(&self, package: &NativePackage) -> BlastResult<()> {
        self.inner.install_package(
            package.inner.name().to_string(),
            Some(package.inner.version().to_string()),
        ).await
    }

    /// Uninstall a package from the environment
    pub async fn uninstall_package(&self, package: &NativePackage) -> BlastResult<()> {
        self.inner.uninstall_package(package.inner.name().to_string()).await
    }

    /// Get installed packages
    pub async fn get_packages(&self) -> BlastResult<Vec<NativePackage>> {
        let packages = self.inner.get_packages().await?;
        Ok(packages.into_iter().map(|p| NativePackage { inner: p }).collect())
    }

    /// Execute Python code in the environment
    pub async fn execute_python(&self, code: &str) -> BlastResult<String> {
        let python_path = self.inner.path().join("bin").join("python");
        let output = tokio::process::Command::new(python_path)
            .arg("-c")
            .arg(code)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(BlastError::python(format!(
                "Python execution failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Run a Python script file
    pub async fn run_script(&self, script_path: &str) -> BlastResult<String> {
        let python_path = self.inner.path().join("bin").join("python");
        let output = tokio::process::Command::new(python_path)
            .arg(script_path)
            .output()
            .await
            .map_err(|e| BlastError::python(format!("Failed to run script: {}", e)))?;

        if !output.status.success() {
            return Err(BlastError::python(format!(
                "Script execution failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Install dependencies from requirements.txt
    pub async fn install_requirements(&self, requirements_path: &str) -> BlastResult<()> {
        let pip_path = self.inner.path().join("bin").join("pip");
        let output = tokio::process::Command::new(pip_path)
            .arg("install")
            .arg("-r")
            .arg(requirements_path)
            .output()
            .await
            .map_err(|e| BlastError::python(format!("Failed to install requirements: {}", e)))?;

        if !output.status.success() {
            return Err(BlastError::python(format!(
                "Failed to install requirements: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }
}

/// Native package wrapper
#[derive(Clone)]
pub struct NativePackage {
    inner: Package,
}

impl NativePackage {
    /// Create a new package
    pub fn new(name: String, version: String, dependencies: Option<HashMap<String, String>>) -> BlastResult<Self> {
        let deps = dependencies.unwrap_or_default()
            .into_iter()
            .map(|(name, ver)| {
                VersionConstraint::parse(&ver)
                    .map(|constraint| (name, constraint))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let python_version = VersionConstraint::any();
        
        Ok(Self {
            inner: Package::new(
                name.clone(),
                version.clone(),
                PackageMetadata::new(
                    name,
                    version,
                    deps,
                    python_version.clone(),
                ),
                python_version,
            )?
        })
    }

    /// Get package name
    pub fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Get package version
    pub fn version(&self) -> String {
        self.inner.version().to_string()
    }

    /// Get package dependencies
    pub fn dependencies(&self) -> HashMap<String, String> {
        self.inner.all_dependencies(&[])
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect()
    }
}

/// Native manifest wrapper
pub struct NativeManifest {
    inner: Manifest,
}

impl NativeManifest {
    /// Create manifest from environment
    pub async fn from_environment(env: &NativeEnvironment) -> BlastResult<Self> {
        let name = env.inner.name().to_string();
        let version = "0.1.0".to_string();
        let python_version = env.inner.python_version().to_string();
        let packages = env.inner.get_packages().await?;
        
        Ok(Self {
            inner: Manifest::new(
                name,
                version,
                python_version,
                packages,
                Vec::new(),
            )
        })
    }

    /// Save manifest to file
    pub async fn save(&self, path: String) -> BlastResult<()> {
        self.inner.save(path.into()).await
    }

    /// Load manifest from file
    pub async fn load(path: String) -> BlastResult<Self> {
        Ok(Self {
            inner: Manifest::load(path.into()).await?
        })
    }

    /// Get packages in manifest
    pub fn packages(&self) -> Vec<NativePackage> {
        self.inner.packages()
            .iter()
            .map(|p| NativePackage { inner: p.clone() })
            .collect()
    }

    /// Add package to manifest
    pub fn add_package(&mut self, package: &NativePackage) {
        self.inner.add_package(package.inner.clone());
    }

    /// Remove package from manifest
    pub fn remove_package(&mut self, name: String) {
        self.inner.remove_package(&name);
    }
}

/// Create a new Python environment
pub async fn create_environment(path: String, python_version: Option<String>) -> BlastResult<NativeEnvironment> {
    let version = python_version.unwrap_or_else(|| "3.8".to_string());
    let env = NativeEnvironment::new(path, version).await?;
    env.inner.init().await?;
    Ok(env)
} 