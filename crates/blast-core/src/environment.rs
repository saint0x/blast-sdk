use std::path::PathBuf;
use std::process::Command;
use std::collections::HashMap;
use async_trait::async_trait;
use crate::error::{BlastError, BlastResult};
use crate::package::Package;
use crate::version::VersionConstraint;
use crate::python::PythonVersion;
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

/// Core trait for environment management
#[async_trait]
pub trait Environment: Send + Sync + 'static {
    /// Create a new environment
    async fn create(&self) -> BlastResult<()>;

    /// Activate the environment
    async fn activate(&self) -> BlastResult<()>;

    /// Deactivate the environment
    async fn deactivate(&self) -> BlastResult<()>;

    /// Install a package
    async fn install_package(&self, package: &Package) -> BlastResult<()>;

    /// Uninstall a package
    async fn uninstall_package(&self, package: &Package) -> BlastResult<()>;

    /// Get installed packages
    async fn get_packages(&self) -> BlastResult<Vec<Package>>;

    /// Get environment path
    fn path(&self) -> &PathBuf;

    /// Get Python version
    fn python_version(&self) -> &PythonVersion;

    /// Set environment name
    fn set_name(&mut self, name: String);

    /// Get environment name
    fn name(&self) -> Option<&str>;
}

/// Python environment implementation
#[derive(Debug, Clone)]
pub struct PythonEnvironment {
    path: PathBuf,
    python_version: PythonVersion,
    name: Option<String>,
}

impl PythonEnvironment {
    /// Create a new Python environment
    pub fn new(path: PathBuf, python_version: PythonVersion) -> Self {
        Self {
            path,
            python_version,
            name: None,
        }
    }

    /// Get the pip executable path for this environment
    fn pip_executable(&self) -> PathBuf {
        #[cfg(unix)]
        {
            self.path.join("bin").join("pip")
        }
        #[cfg(windows)]
        {
            self.path.join("Scripts").join("pip.exe")
        }
    }
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
        // Execute pip freeze to get installed packages
        let output = Command::new(self.pip_executable())
            .arg("freeze")
            .output()
            .map_err(|e| BlastError::environment(format!(
                "Failed to execute pip freeze: {}", e
            )))?;

        if !output.status.success() {
            return Err(BlastError::environment(format!(
                "Failed to get installed packages: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Parse pip freeze output
        let packages = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() >= 2 {
                    let name = parts[0].trim().to_string();
                    let version = parts[1].trim().replace('=', "");
                    
                    // Create empty dependencies map and any version constraint
                    let dependencies = HashMap::new();
                    let python_version = VersionConstraint::any();
                    
                    Package::new(
                        name.clone(),
                        version.clone(),
                        create_package_metadata(
                            name,
                            version,
                            dependencies,
                            python_version.clone(),
                        ),
                        python_version
                    ).ok()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(packages)
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