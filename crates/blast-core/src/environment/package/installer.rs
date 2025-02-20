use std::path::Path;
use tokio::process::Command;
use crate::error::BlastResult;
use super::{PackageConfig, DependencyGraph};

/// Package installer implementation
pub struct PackageInstaller {
    /// Configuration
    config: PackageConfig,
}

impl PackageInstaller {
    /// Create new package installer
    pub fn new(config: PackageConfig) -> Self {
        Self { config }
    }

    /// Install packages from dependency graph
    pub async fn install_packages(&self, graph: &DependencyGraph) -> BlastResult<()> {
        // Create installation plan
        let plan = self.create_installation_plan(graph);
        
        // Execute each step in plan
        for step in plan {
            match step {
                InstallationStep::Install { name, version } => {
                    self.install_package(&name, &version).await?;
                }
                InstallationStep::Update { name, from: _, to } => {
                    self.update_package(&name, &to).await?;
                }
                InstallationStep::Remove { name } => {
                    self.uninstall_package(&name).await?;
                }
            }
        }
        
        Ok(())
    }

    /// Update packages from dependency graph
    pub async fn update_packages(&self, graph: &DependencyGraph) -> BlastResult<()> {
        // Create update plan
        let plan = self.create_update_plan(graph);
        
        // Execute each step in plan
        for step in plan {
            match step {
                InstallationStep::Install { name, version } => {
                    self.install_package(&name, &version).await?;
                }
                InstallationStep::Update { name, from: _, to } => {
                    self.update_package(&name, &to).await?;
                }
                InstallationStep::Remove { name } => {
                    self.uninstall_package(&name).await?;
                }
            }
        }
        
        Ok(())
    }

    /// Install single package
    pub async fn install_package(&self, name: &str, version: &str) -> BlastResult<()> {
        // Prepare pip command
        let mut cmd = self.create_pip_command();
        
        cmd.arg("install")
            .arg("--no-deps") // Dependencies handled separately
            .arg("--no-cache-dir") // Use our own caching
            .arg(format!("{}=={}", name, version));
        
        // Add index URL if specified
        if !self.config.index_url.is_empty() {
            cmd.arg("--index-url").arg(&self.config.index_url);
        }
        
        // Add extra index URLs
        for url in &self.config.extra_index_urls {
            cmd.arg("--extra-index-url").arg(url);
        }
        
        // Add trusted hosts
        for host in &self.config.trusted_hosts {
            cmd.arg("--trusted-host").arg(host);
        }
        
        // Execute command
        let output = cmd.output().await?;
        
        if !output.status.success() {
            return Err(crate::error::BlastError::package(format!(
                "Failed to install package {}: {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        
        Ok(())
    }

    /// Update single package
    pub async fn update_package(&self, name: &str, to: &str) -> BlastResult<()> {
        // First uninstall old version
        self.uninstall_package(name).await?;
        
        // Then install new version
        self.install_package(name, to).await?;
        
        Ok(())
    }

    /// Uninstall single package
    pub async fn uninstall_package(&self, name: &str) -> BlastResult<()> {
        // Prepare pip command
        let mut cmd = self.create_pip_command();
        
        cmd.arg("uninstall")
            .arg("--yes") // Don't ask for confirmation
            .arg(name);
        
        // Execute command
        let output = cmd.output().await?;
        
        if !output.status.success() {
            return Err(crate::error::BlastError::package(format!(
                "Failed to uninstall package {}: {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        
        Ok(())
    }

    /// Create pip command with proper environment
    fn create_pip_command(&self) -> Command {
        let mut cmd = Command::new(self.get_pip_path());
        
        // Set environment variables
        cmd.env("PYTHONPATH", &self.config.env_path)
            .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
            .env("PIP_NO_WARN_SCRIPT_LOCATION", "1");
        
        if self.config.require_hashes {
            cmd.env("PIP_REQUIRE_HASHES", "1");
        }
        
        cmd
    }

    /// Get pip executable path
    fn get_pip_path(&self) -> &Path {
        // TODO: Make this configurable and platform-specific
        Path::new("/usr/local/bin/pip")
    }

    /// Create installation plan from dependency graph
    fn create_installation_plan(&self, graph: &DependencyGraph) -> Vec<InstallationStep> {
        let mut plan = Vec::new();
        
        // Add installation steps in dependency order
        for node in graph.nodes() {
            plan.push(InstallationStep::Install {
                name: node.name.clone(),
                version: node.version.clone(),
            });
        }
        
        plan
    }

    /// Create update plan from dependency graph
    fn create_update_plan(&self, graph: &DependencyGraph) -> Vec<InstallationStep> {
        let mut plan = Vec::new();
        
        // Add update steps in dependency order
        for node in graph.nodes() {
            if let Some(current_version) = node.current_version.as_ref() {
                plan.push(InstallationStep::Update {
                    name: node.name.clone(),
                    from: current_version.clone(),
                    to: node.version.clone(),
                });
            } else {
                plan.push(InstallationStep::Install {
                    name: node.name.clone(),
                    version: node.version.clone(),
                });
            }
        }
        
        plan
    }
}

/// Installation step types
#[derive(Debug)]
enum InstallationStep {
    /// Install new package
    Install {
        name: String,
        version: String,
    },
    /// Update existing package
    Update {
        name: String,
        from: String,
        to: String,
    },
    /// Remove package
    Remove {
        name: String,
    },
} 