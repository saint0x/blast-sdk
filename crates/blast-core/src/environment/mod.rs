//! Environment module for managing Python environments.
//! Provides functionality for creating and managing isolated Python environments.

pub mod isolation;
pub mod resources;
pub mod security;
mod state;
pub mod package;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::error::{BlastResult, BlastError};
use crate::package::Package;
use crate::python::PythonVersion;
use crate::version::VersionConstraint;
use std::collections::HashMap;

// Import state types
use state::{
    EnvironmentState,
    StateManager,
    ContainerStatus,
};

// Import package types
pub use package::{
    PackageOperation,
    PackageLayer,
    PackageConfig,
    PackageState,
    DependencyGraph,
    Version,
    Dependency,
};

// Import resource types
pub use resources::{
    ResourceManager,
    ResourceLimits,
};

// Import security types
pub use security::{
    SecurityManager,
    SecurityConfig,
};

pub use isolation::{
    IsolationLevel,
    IsolationConfig,
    EnhancedIsolation,
    NetworkPolicy,
    FilesystemPolicy,
    ContainerRuntime,
    Container,
    ContainerConfig,
};

pub use state::{
    MountState,
    ResourceState,
    NetworkUsage,
    SecurityState,
};

/// Core trait for environment management
#[async_trait::async_trait]
pub trait Environment: Send + Sync {
    /// Initialize environment
    async fn init(&self) -> BlastResult<()>;

    /// Install package
    async fn install_package(&self, name: String, version: Option<String>) -> BlastResult<()>;

    /// Uninstall package
    async fn uninstall_package(&self, name: String) -> BlastResult<()>;

    /// Update package
    async fn update_package(&self, name: String, version: String) -> BlastResult<()>;

    /// Check package conflicts
    async fn check_package_conflicts(&self) -> BlastResult<Vec<String>>;

    /// Intercept pip operation
    async fn intercept_pip(&self, args: Vec<String>) -> BlastResult<()>;

    /// Get installed packages
    async fn get_packages(&self) -> BlastResult<Vec<Package>>;

    /// Get environment path
    fn path(&self) -> &PathBuf;

    /// Get Python version
    fn python_version(&self) -> &str;

    /// Get environment name
    fn name(&self) -> &str;
}

/// Environment configuration
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    /// Environment name
    pub name: String,
    /// Environment path
    pub path: PathBuf,
    /// Python version
    pub python_version: String,
    /// Isolation configuration
    pub isolation: IsolationLevel,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: PathBuf::new(),
            python_version: "3.9".to_string(),
            isolation: IsolationLevel::Process,
            resource_limits: ResourceLimits::default(),
            security: SecurityConfig::default(),
        }
    }
}

/// Environment implementation
#[derive(Clone)]
pub struct EnvironmentImpl {
    /// Environment configuration
    config: EnvironmentConfig,
    /// State manager
    state_manager: Arc<RwLock<StateManager>>,
    /// Resource manager
    resource_manager: Arc<ResourceManager>,
    /// Security manager
    security_manager: Arc<SecurityManager>,
    /// Package manager
    package_manager: Arc<PackageLayer>,
    /// Container runtime
    container: Arc<RwLock<Box<dyn ContainerRuntime + Send + Sync>>>,
}

#[async_trait::async_trait]
impl Environment for EnvironmentImpl {
    async fn init(&self) -> BlastResult<()> {
        // Initialize container
        self.container.write().await.initialize().await?;
        
        // Apply resource limits
        self.resource_manager.apply_limits().await?;
        
        // Apply security configuration
        self.security_manager.apply_config().await?;
        
        // Update container state
        self.state_manager.write().await.update_container_state(|state| {
            state.status = ContainerStatus::Running;
            Ok(())
        }).await?;
        
        Ok(())
    }

    async fn install_package(&self, name: String, version: Option<String>) -> BlastResult<()> {
        // Create package operation
        let op = PackageOperation::Install {
            name,
            version: version.map(|v| Version {
                version: v,
                released: chrono::Utc::now(),
                python_requires: None,
                dependencies: Vec::new(),
            }),
            dependencies: Vec::new(),
        };
        
        // Queue operation
        self.package_manager.queue_operation(op).await?;
        
        Ok(())
    }

    async fn uninstall_package(&self, name: String) -> BlastResult<()> {
        // Create package operation
        let op = PackageOperation::Uninstall { name };
        
        // Queue operation
        self.package_manager.queue_operation(op).await?;
        
        Ok(())
    }

    async fn update_package(&self, name: String, version: String) -> BlastResult<()> {
        let state = self.package_manager.get_state().await?;
        let current_version = state.get_package(&name)
            .ok_or_else(|| BlastError::package(
                format!("Package {} not installed", name)
            ))?;

        // Create package operation
        let op = PackageOperation::Update {
            name,
            from_version: current_version.version.clone(),
            to_version: Version {
                version,
                released: chrono::Utc::now(),
                python_requires: None,
                dependencies: Vec::new(),
            },
        };
        
        // Queue operation
        self.package_manager.queue_operation(op).await?;
        
        Ok(())
    }

    async fn check_package_conflicts(&self) -> BlastResult<Vec<String>> {
        self.package_manager.check_conflicts().await
    }

    async fn intercept_pip(&self, args: Vec<String>) -> BlastResult<()> {
        self.package_manager.intercept_pip(args).await
    }

    async fn get_packages(&self) -> BlastResult<Vec<Package>> {
        let state = self.package_manager.get_state().await?;
        let mut packages = Vec::new();
        
        for metadata in state.get_all_packages() {
            // Create dependencies map for PackageMetadata
            let deps = metadata.version.dependencies.iter()
                .map(|d| (d.name.clone(), VersionConstraint::parse(&d.version_constraint).unwrap_or_default()))
                .collect::<HashMap<String, VersionConstraint>>();

            // Create package metadata
            let pkg_metadata = crate::metadata::PackageMetadata::new(
                metadata.version.version.clone(),
                metadata.version.version.clone(),
                deps,
                metadata.version.dependencies.get(0)
                    .map(|d| VersionConstraint::parse(&d.version_constraint))
                    .transpose()?
                    .unwrap_or_default()
            );

            let pkg = Package::new(
                metadata.version.version.clone(),
                metadata.version.version.clone(),
                pkg_metadata,
                metadata.version.dependencies.get(0)
                    .map(|d| VersionConstraint::parse(&d.version_constraint))
                    .transpose()?
                    .unwrap_or_default()
            )?;
            packages.push(pkg);
        }
        
        Ok(packages)
    }

    fn path(&self) -> &PathBuf {
        &self.config.path
    }

    fn python_version(&self) -> &str {
        &self.config.python_version
    }

    fn name(&self) -> &str {
        &self.config.name
    }
}

impl EnvironmentImpl {
    /// Create new environment
    pub async fn new(config: EnvironmentConfig) -> BlastResult<Self> {
        // Create environment state
        let state = EnvironmentState::new(
            Uuid::new_v4().to_string(),
            config.name.clone(),
            config.path.clone(),
            PythonVersion::parse(&config.python_version)?,
        );
        
        // Create managers
        let state_manager = Arc::new(RwLock::new(StateManager::new(state)));
        let resource_manager = Arc::new(ResourceManager::new(config.resource_limits.clone()));
        let security_manager = Arc::new(SecurityManager::new(config.security.clone()));
        
        // Create package configuration
        let package_config = PackageConfig {
            python_version: config.python_version.clone(),
            env_path: config.path.clone(),
            ..PackageConfig::default()
        };
        
        let package_manager = Arc::new(PackageLayer::new(package_config).await?);
        
        // Create container configuration
        let container_config = ContainerConfig {
            network_policy: NetworkPolicy::default(),
            filesystem_policy: FilesystemPolicy::default(),
            root_dir: config.path.clone(),
            name: config.name.clone(),
            labels: Default::default(),
        };

        // Initialize container runtime
        let container = Box::new(Container::new(&container_config).await?) as Box<dyn ContainerRuntime + Send + Sync>;
        
        Ok(Self {
            config,
            state_manager,
            resource_manager,
            security_manager,
            package_manager,
            container: Arc::new(RwLock::new(container)),
        })
    }

    /// Get state manager
    pub fn state_manager(&self) -> Arc<RwLock<StateManager>> {
        Arc::clone(&self.state_manager)
    }

    /// Get resource manager
    pub fn resource_manager(&self) -> Arc<ResourceManager> {
        Arc::clone(&self.resource_manager)
    }

    /// Get security manager
    pub fn security_manager(&self) -> Arc<SecurityManager> {
        Arc::clone(&self.security_manager)
    }

    /// Get package manager
    pub fn package_manager(&self) -> Arc<PackageLayer> {
        Arc::clone(&self.package_manager)
    }

    /// Get container runtime
    pub fn container(&self) -> Arc<RwLock<Box<dyn ContainerRuntime + Send + Sync>>> {
        Arc::clone(&self.container)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_environment() -> EnvironmentImpl {
        let temp_dir = TempDir::new().unwrap();
        
        let config = EnvironmentConfig {
            name: "test-env".to_string(),
            path: temp_dir.path().to_path_buf(),
            python_version: "3.9".to_string(),
            isolation: IsolationLevel::Process,
            resource_limits: ResourceLimits::default(),
            security: SecurityConfig::default(),
        };
        
        EnvironmentImpl::new(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_environment_creation() {
        let temp_dir = TempDir::new().unwrap();
        
        let config = EnvironmentConfig {
            name: "test-env".to_string(),
            path: temp_dir.path().to_path_buf(),
            python_version: "3.9".to_string(),
            isolation: IsolationLevel::Process,
            resource_limits: ResourceLimits::default(),
            security: SecurityConfig::default(),
        };
        
        let env = EnvironmentImpl::new(config).await.unwrap();
        
        // Test initialization
        env.init().await.unwrap();
        
        // Test state
        assert_eq!(env.name(), "test-env");
        assert_eq!(env.python_version(), "3.9");
        
        // Test package management
        env.install_package("requests".to_string(), None).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let pkg_state = env.package_manager.get_state().await.unwrap();
        assert!(pkg_state.is_installed("requests"));
        
        // Test conflicts
        let conflicts = env.check_package_conflicts().await.unwrap();
        assert!(conflicts.is_empty());
    }

    #[tokio::test]
    async fn test_package_operations() {
        let env = create_test_environment().await;
        
        // Install package
        env.install_package("requests".to_string(), None).await.unwrap();
        
        // Verify package state
        let pkg_state = env.package_manager.get_state().await.unwrap();
        assert!(pkg_state.is_installed("requests"));
        
        // Uninstall package
        env.uninstall_package("requests".to_string()).await.unwrap();
        
        // Verify package removed
        let pkg_state = env.package_manager.get_state().await.unwrap();
        assert!(!pkg_state.is_installed("requests"));
    }
} 