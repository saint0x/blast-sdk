use std::path::PathBuf;
use tokio::process::Command;
use crate::error::BlastResult;
use crate::package::Package;
use crate::environment::Environment;
use super::PythonVersion;

/// Python environment implementation
#[derive(Debug, Clone)]
pub struct PythonEnvironment {
    /// Inner environment
    inner: EnvironmentImpl,
    /// Cached version string
    version_string: String,
}

/// Inner environment implementation
#[derive(Debug, Clone)]
struct EnvironmentImpl {
    /// Environment name
    name: String,
    /// Environment path
    path: PathBuf,
    /// Python version
    version: PythonVersion,
}

impl PythonEnvironment {
    /// Create new Python environment
    pub async fn new(
        name: String,
        path: PathBuf,
        python_version: PythonVersion,
    ) -> BlastResult<Self> {
        let inner = EnvironmentImpl {
            name,
            path,
            version: python_version.clone(),
        };
        
        Ok(Self { 
            inner,
            version_string: python_version.to_string(),
        })
    }

    /// Get installed packages
    pub async fn get_packages(&self) -> BlastResult<Vec<Package>> {
        let pip_path = self.inner.path.join("bin").join("pip");
        let output = Command::new(pip_path)
            .args(&["list", "--format=json"])
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let packages: Vec<Package> = serde_json::from_slice(&output.stdout)?;
        Ok(packages)
    }
}

#[async_trait::async_trait]
impl Environment for PythonEnvironment {
    async fn init(&self) -> BlastResult<()> {
        // Create standard directories
        let bin_dir = self.inner.path.join("bin");
        let lib_dir = self.inner.path.join("lib");
        let include_dir = self.inner.path.join("include");
        let site_packages_dir = lib_dir.join("python3").join("site-packages");

        for dir in [&bin_dir, &lib_dir, &include_dir, &site_packages_dir] {
            tokio::fs::create_dir_all(dir).await?;
        }

        Ok(())
    }

    async fn install_package(&self, name: String, version: Option<String>) -> BlastResult<()> {
        let mut cmd = Command::new(self.inner.path.join("bin").join("pip"));
        cmd.arg("install").arg(&name);
        if let Some(version) = version {
            cmd.arg(&format!("{}=={}", name, version));
        }
        let output = cmd.output().await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(crate::error::BlastError::Environment(
                format!("Failed to install package {}", name)
            ))
        }
    }

    async fn uninstall_package(&self, name: String) -> BlastResult<()> {
        let output = Command::new(self.inner.path.join("bin").join("pip"))
            .args(&["uninstall", "-y", &name])
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(crate::error::BlastError::Environment(
                format!("Failed to uninstall package {}", name)
            ))
        }
    }

    async fn update_package(&self, name: String, version: String) -> BlastResult<()> {
        let output = Command::new(self.inner.path.join("bin").join("pip"))
            .args(&["install", "-U", &format!("{}=={}", name, version)])
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(crate::error::BlastError::Environment(
                format!("Failed to update package {}", name)
            ))
        }
    }

    async fn check_package_conflicts(&self) -> BlastResult<Vec<String>> {
        let output = Command::new(self.inner.path.join("bin").join("pip"))
            .args(&["check"])
            .output()
            .await?;
        if output.status.success() {
            Ok(Vec::new())
        } else {
            Ok(String::from_utf8_lossy(&output.stderr)
                .lines()
                .map(|s| s.to_string())
                .collect())
        }
    }

    async fn intercept_pip(&self, args: Vec<String>) -> BlastResult<()> {
        let output = Command::new(self.inner.path.join("bin").join("pip"))
            .args(&args)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(crate::error::BlastError::Environment(
                "Pip command failed".to_string()
            ))
        }
    }

    async fn get_packages(&self) -> BlastResult<Vec<Package>> {
        self.get_packages().await
    }

    fn path(&self) -> &PathBuf {
        &self.inner.path
    }

    fn python_version(&self) -> &str {
        &self.version_string
    }

    fn name(&self) -> &str {
        if self.inner.name.is_empty() {
            "unnamed"
        } else {
            &self.inner.name
        }
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