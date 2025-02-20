use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::BlastResult;
use crate::environment::Environment;
use super::types::SyncOperation;

/// Sync manager implementation
pub struct SyncManager {
    /// Source environment
    source: Arc<Box<dyn Environment>>,
    /// Target environment
    target: Arc<Box<dyn Environment>>,
    /// Pending operations
    pending_ops: Arc<RwLock<Vec<SyncOperation>>>,
}

impl SyncManager {
    /// Create new sync manager
    pub fn new(
        source: Arc<Box<dyn Environment>>,
        target: Arc<Box<dyn Environment>>,
    ) -> Self {
        Self {
            source,
            target,
            pending_ops: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if environments are compatible
    pub async fn check_compatibility(&self) -> BlastResult<bool> {
        // Check Python versions
        let source_version = self.source.python_version();
        let target_version = self.target.python_version();

        // For now just check major version compatibility
        let source_major = source_version.split('.').next().unwrap_or("0");
        let target_major = target_version.split('.').next().unwrap_or("0");

        Ok(source_major == target_major)
    }

    /// Queue sync operation
    pub async fn queue_operation(&self, op: SyncOperation) -> BlastResult<()> {
        self.pending_ops.write().await.push(op);
        Ok(())
    }

    /// Apply sync operation
    pub async fn apply_operation(&self, op: SyncOperation) -> BlastResult<()> {
        match op {
            SyncOperation::AddPackage(package) => {
                self.target.install_package(
                    package.name().to_string(),
                    Some(package.version().to_string()),
                ).await?;
            }
            SyncOperation::RemovePackage(package) => {
                self.target.uninstall_package(package.name().to_string()).await?;
            }
            SyncOperation::UpdatePackage { name, from_version: _, to_version } => {
                self.target.update_package(name, to_version).await?;
            }
            SyncOperation::UpdateEnvVar { key: _, value: _ } => {
                // Environment variables are handled through the environment configuration
                // This would require extending the Environment trait
                // For now we'll just log that this operation isn't supported
                eprintln!("Environment variable updates not yet supported");
            }
            SyncOperation::UpdatePythonVersion(_) => {
                // Python version updates require recreating the environment
                // This would require extending the Environment trait
                // For now we'll just log that this operation isn't supported
                eprintln!("Python version updates not yet supported");
            }
        }
        Ok(())
    }

    /// Get source environment
    pub fn source(&self) -> Arc<Box<dyn Environment>> {
        Arc::clone(&self.source)
    }

    /// Get target environment
    pub fn target(&self) -> Arc<Box<dyn Environment>> {
        Arc::clone(&self.target)
    }

    /// Get pending operations
    pub async fn get_pending_ops(&self) -> BlastResult<Vec<SyncOperation>> {
        Ok(self.pending_ops.read().await.clone())
    }

    /// Clear pending operations
    pub async fn clear_pending_ops(&self) -> BlastResult<()> {
        self.pending_ops.write().await.clear();
        Ok(())
    }

    /// Sync environments
    pub async fn sync(&self) -> BlastResult<()> {
        // Check compatibility
        if !self.check_compatibility().await? {
            return Err(crate::error::BlastError::Sync(
                "Environments are not compatible".to_string()
            ));
        }

        // Get pending operations
        let ops = self.pending_ops.read().await.clone();

        // Apply operations
        for op in ops {
            self.apply_operation(op).await?;
        }

        // Clear pending operations
        self.clear_pending_ops().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::environment::{EnvironmentConfig, EnvironmentImpl, IsolationLevel, ResourceLimits, SecurityConfig};

    #[tokio::test]
    async fn test_sync_manager() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        // Create source environment
        let source_config = EnvironmentConfig {
            name: "source".to_string(),
            path: source_dir,
            python_version: "3.9.0".to_string(),
            isolation: IsolationLevel::Process,
            resource_limits: ResourceLimits::default(),
            security: SecurityConfig::default(),
        };
        let source = Arc::new(Box::new(EnvironmentImpl::new(source_config).await.unwrap()) as Box<dyn Environment>);

        // Create target environment
        let target_config = EnvironmentConfig {
            name: "target".to_string(),
            path: target_dir,
            python_version: "3.9.0".to_string(),
            isolation: IsolationLevel::Process,
            resource_limits: ResourceLimits::default(),
            security: SecurityConfig::default(),
        };
        let target = Arc::new(Box::new(EnvironmentImpl::new(target_config).await.unwrap()) as Box<dyn Environment>);

        // Create sync manager
        let manager = SyncManager::new(source, target);

        // Test compatibility check
        assert!(manager.check_compatibility().await.unwrap());

        // Test sync operation
        manager.sync().await.unwrap();
    }
} 