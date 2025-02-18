use std::collections::HashMap;

use crate::environment::Environment;
use crate::python::{PythonEnvironment, PythonVersion};
use crate::package::Package;
use crate::version::VersionConstraint;
use crate::manifest::Manifest;
use crate::error::{BlastError, BlastResult};
use crate::metadata::PackageMetadata;

/// Native Python environment wrapper
#[derive(Clone)]
pub struct NativeEnvironment {
    inner: PythonEnvironment,
}

impl NativeEnvironment {
    /// Create a new Python environment
    pub fn new(path: String, python_version: String) -> BlastResult<Self> {
        let version = PythonVersion::parse(&python_version)?;
        Ok(Self {
            inner: PythonEnvironment::new(path.into(), version),
        })
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
        self.inner.install_package(&package.inner).await
    }

    /// Uninstall a package from the environment
    pub async fn uninstall_package(&self, package: &NativePackage) -> BlastResult<()> {
        self.inner.uninstall_package(&package.inner).await
    }

    /// Get installed packages
    pub async fn get_packages(&self) -> BlastResult<Vec<NativePackage>> {
        let packages = self.inner.get_packages()?;
        Ok(packages.into_iter().map(|p| NativePackage { inner: p }).collect())
    }

    /// Execute Python code in the environment
    pub async fn execute_python(&self, code: &str) -> BlastResult<String> {
        let output = tokio::process::Command::new(&self.inner.interpreter_path())
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
        let python_path = self.inner.interpreter_path();
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
        let pip = self.inner.pip_executable();
        let output = tokio::process::Command::new(pip)
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
        Ok(Self {
            inner: Manifest::from_environment(&env.inner).await?
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
    let env = NativeEnvironment::new(path, version)?;
    env.inner.create().await?;
    Ok(env)
}

/// Get the active Python environment
pub fn get_active_environment() -> BlastResult<Option<NativeEnvironment>> {
    match PythonEnvironment::get_active()? {
        Some(env) => Ok(Some(NativeEnvironment { inner: env })),
        None => Ok(None),
    }
} 