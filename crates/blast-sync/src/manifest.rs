//! Sync manifest management implementation

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

use blast_core::{
    BlastResult,
    ManifestManager,
    Package,
    SystemDependency,
    Manifest,
};

/// Sync manifest manager
#[derive(Clone)]
pub struct SyncManifestManager {
    manifest: Arc<RwLock<Manifest>>,
    manifest_path: PathBuf,
    daemon_client: Arc<blast_daemon::Client>,
}

impl SyncManifestManager {
    /// Create new manifest manager
    pub async fn new(manifest_path: PathBuf, daemon_client: Arc<blast_daemon::Client>) -> BlastResult<Self> {
        // Load manifest from daemon
        let manifest = daemon_client.get_manifest().await?;

        Ok(Self {
            manifest: Arc::new(RwLock::new(manifest)),
            manifest_path,
            daemon_client,
        })
    }

    /// Sync manifest with daemon
    pub async fn sync_with_daemon(&self) -> BlastResult<()> {
        let daemon_manifest = self.daemon_client.get_manifest().await?;
        *self.manifest.write().await = daemon_manifest;
        Ok(())
    }
}

#[async_trait]
impl ManifestManager for SyncManifestManager {
    async fn get_manifest(&self) -> BlastResult<Manifest> {
        Ok(self.manifest.read().await.clone())
    }

    async fn update_manifest(&self, manifest: &Manifest) -> BlastResult<()> {
        // Update local copy
        *self.manifest.write().await = manifest.clone();
        
        // Sync with daemon
        self.daemon_client.update_manifest(manifest).await
    }

    async fn record_package_install(&self, package: &Package) -> BlastResult<()> {
        // Update local copy
        let mut manifest = self.manifest.write().await;
        manifest.record_package_install(
            package.name.clone(),
            package.version.to_string(),
        );
        
        // Sync with daemon
        self.daemon_client.record_package_install(package).await
    }

    async fn record_package_removal(&self, package: &Package) -> BlastResult<()> {
        // Update local copy
        let mut manifest = self.manifest.write().await;
        manifest.record_package_removal(&package.name);
        
        // Sync with daemon
        self.daemon_client.record_package_removal(package).await
    }

    async fn record_env_var_change(&self, key: &str, value: &str) -> BlastResult<()> {
        // Update local copy
        let mut manifest = self.manifest.write().await;
        manifest.record_env_var_change(key.to_string(), value.to_string());
        
        // Sync with daemon
        self.daemon_client.record_env_var_change(key, value).await
    }

    async fn record_system_dependency(&self, dependency: &SystemDependency) -> BlastResult<()> {
        // Update local copy
        let mut manifest = self.manifest.write().await;
        manifest.record_system_dependency(dependency.clone());
        
        // Sync with daemon
        self.daemon_client.record_system_dependency(dependency).await
    }

    async fn record_hook_addition(&self, hook_type: &str, command: &str) -> BlastResult<()> {
        // Update local copy
        let mut manifest = self.manifest.write().await;
        manifest.record_hook_addition(hook_type, command.to_string());
        
        // Sync with daemon
        self.daemon_client.record_hook_addition(hook_type, command).await
    }

    async fn verify_manifest(&self) -> BlastResult<bool> {
        let manifest = self.manifest.read().await;
        let local_valid = manifest.metadata.verify()?;
        
        // Also verify against daemon
        let daemon_manifest = self.daemon_client.get_manifest().await?;
        let daemon_valid = daemon_manifest.metadata.verify()?;
        
        Ok(local_valid && daemon_valid && manifest.metadata.content_hash == daemon_manifest.metadata.content_hash)
    }
} 