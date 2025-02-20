use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use crate::error::BlastResult;

mod resolver;
mod installer;
mod interceptor;
mod state;
mod graph;

pub use resolver::DependencyResolver;
pub use installer::PackageInstaller;
pub use interceptor::PipInterceptor;
pub use state::{PackageState, PackageMetadata};
pub use graph::{DependencyGraph, DependencyNode};

/// Package version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// Version string (PEP 440 compliant)
    pub version: String,
    /// Release timestamp
    pub released: chrono::DateTime<chrono::Utc>,
    /// Python version requirements
    pub python_requires: Option<String>,
    /// Package dependencies
    pub dependencies: Vec<Dependency>,
}

/// Package dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Package name
    pub name: String,
    /// Version constraint
    pub version_constraint: String,
    /// Optional dependency
    pub optional: bool,
    /// Environment markers
    pub markers: Option<String>,
}

/// Package operation types
#[derive(Debug, Clone)]
pub enum PackageOperation {
    /// Install package
    Install {
        name: String,
        version: Option<Version>,
        dependencies: Vec<Dependency>,
    },
    /// Uninstall package
    Uninstall {
        name: String,
    },
    /// Update package
    Update {
        name: String,
        from_version: Version,
        to_version: Version,
    },
}

/// Package layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    /// Python version
    pub python_version: String,
    /// Environment path
    pub env_path: PathBuf,
    /// Package index URL
    pub index_url: String,
    /// Extra index URLs
    pub extra_index_urls: Vec<String>,
    /// Trusted hosts
    pub trusted_hosts: Vec<String>,
    /// Require hashes
    pub require_hashes: bool,
    /// Allow prereleases
    pub allow_prereleases: bool,
    /// Cache directory
    pub cache_dir: PathBuf,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            python_version: String::from("3.9"),
            env_path: PathBuf::from("/var/lib/blast/environments"),
            index_url: String::from("https://pypi.org/simple"),
            extra_index_urls: Vec::new(),
            trusted_hosts: vec![String::from("pypi.org")],
            require_hashes: true,
            allow_prereleases: false,
            cache_dir: PathBuf::from("/var/lib/blast/cache"),
            cache_ttl: 86400, // 24 hours
        }
    }
}

/// Package layer implementation
pub struct PackageLayer {
    /// Configuration
    config: PackageConfig,
    /// Dependency resolver
    resolver: Arc<DependencyResolver>,
    /// Package installer
    installer: Arc<PackageInstaller>,
    /// Pip interceptor
    interceptor: Arc<PipInterceptor>,
    /// Package state
    state: Arc<RwLock<PackageState>>,
}

impl PackageLayer {
    /// Create new package layer
    pub async fn new(config: PackageConfig) -> BlastResult<Self> {
        let state = Arc::new(RwLock::new(PackageState::new()));
        let resolver = Arc::new(DependencyResolver::new(config.clone()));
        let installer = Arc::new(PackageInstaller::new(config.clone()));
        let interceptor = Arc::new(PipInterceptor::new(config.clone()));

        Ok(Self {
            config,
            resolver,
            installer,
            interceptor,
            state,
        })
    }

    /// Queue package operation
    pub async fn queue_operation(&self, operation: PackageOperation) -> BlastResult<()> {
        match operation {
            PackageOperation::Install { name, version, dependencies } => {
                // Resolve dependencies
                let graph = self.resolver.resolve_dependencies(&name, version.as_ref(), &dependencies).await?;
                
                // Install packages
                self.installer.install_packages(&graph).await?;
                
                // Update state
                let mut state = self.state.write().await;
                state.update_from_graph(&graph).await?;
            }
            PackageOperation::Uninstall { name } => {
                // Remove package
                self.installer.uninstall_package(&name).await?;
                
                // Update state
                let mut state = self.state.write().await;
                state.remove_package(&name).await?;
            }
            PackageOperation::Update { name, from_version, to_version } => {
                // Resolve new dependencies
                let graph = self.resolver.resolve_version_update(&name, &from_version, &to_version).await?;
                
                // Update packages
                self.installer.update_packages(&graph).await?;
                
                // Update state
                let mut state = self.state.write().await;
                state.update_from_graph(&graph).await?;
            }
        }
        
        Ok(())
    }

    /// Intercept pip operation
    pub async fn intercept_pip(&self, args: Vec<String>) -> BlastResult<()> {
        self.interceptor.handle_pip_command(args).await
    }

    /// Check package conflicts
    pub async fn check_conflicts(&self) -> BlastResult<Vec<String>> {
        let state = self.state.read().await;
        self.resolver.check_state_conflicts(&state).await
    }

    /// Get current package state
    pub async fn get_state(&self) -> BlastResult<PackageState> {
        Ok(self.state.read().await.clone())
    }
} 