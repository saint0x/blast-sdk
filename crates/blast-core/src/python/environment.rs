use std::path::PathBuf;
use std::process::Command;
use std::collections::HashMap;
use async_trait::async_trait;
use crate::{
    error::{BlastError, BlastResult},
    package::Package,
    environment::{
        Environment,
        EnvironmentConfig,
        EnvironmentImpl,
        IsolationLevel,
        ResourceLimits,
        SecurityConfig,
    },
    metadata::PackageMetadata,
    version::VersionConstraint,
};
use super::PythonVersion;

/// Python environment implementation
#[derive(Clone)]
pub struct PythonEnvironment {
    /// Inner environment
    inner: EnvironmentImpl,
}

impl PythonEnvironment {
    /// Create new Python environment
    pub async fn new(
        name: String,
        path: PathBuf,
        python_version: PythonVersion,
    ) -> BlastResult<Self> {
        let config = EnvironmentConfig {
            name,
            path,
            python_version: python_version.to_string(),
            isolation: IsolationLevel::Process,
            resource_limits: ResourceLimits::default(),
            security: SecurityConfig::default(),
        };

        let inner = EnvironmentImpl::new(config).await?;
        Ok(Self { inner })
    }

    /// Get installed packages
    pub async fn get_packages(&self) -> BlastResult<Vec<Package>> {
        let pip_path = self.inner.path().join("bin").join("pip");
        let output = Command::new(pip_path)
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
                    PackageMetadata::new(
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
}

#[async_trait]
impl Environment for PythonEnvironment {
    async fn init(&self) -> BlastResult<()> {
        self.inner.init().await
    }

    async fn install_package(&self, name: String, version: Option<String>) -> BlastResult<()> {
        self.inner.install_package(name, version).await
    }

    async fn uninstall_package(&self, name: String) -> BlastResult<()> {
        self.inner.uninstall_package(name).await
    }

    async fn update_package(&self, name: String, version: String) -> BlastResult<()> {
        self.inner.update_package(name, version).await
    }

    async fn check_package_conflicts(&self) -> BlastResult<Vec<String>> {
        self.inner.check_package_conflicts().await
    }

    async fn intercept_pip(&self, args: Vec<String>) -> BlastResult<()> {
        self.inner.intercept_pip(args).await
    }

    async fn get_packages(&self) -> BlastResult<Vec<Package>> {
        self.inner.get_packages().await
    }

    fn path(&self) -> &PathBuf {
        self.inner.path()
    }

    fn python_version(&self) -> &str {
        self.inner.python_version()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_python_environment() {
        let temp_dir = TempDir::new().unwrap();
        let version = PythonVersion::new(3, 9, Some(0));
        
        let env = PythonEnvironment::new(
            "test-env".to_string(),
            temp_dir.path().to_path_buf(),
            version,
        ).await.unwrap();

        assert_eq!(env.name(), "test-env");
        assert_eq!(env.python_version(), "3.9.0");
        assert_eq!(env.path(), temp_dir.path());
    }
} 