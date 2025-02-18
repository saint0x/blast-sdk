//! Daemon manifest management implementation

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

/// Daemon manifest manager
#[derive(Clone)]
pub struct DaemonManifestManager {
    manifest: Arc<RwLock<Manifest>>,
    manifest_path: PathBuf,
}

impl DaemonManifestManager {
    /// Create new manifest manager
    pub async fn new(manifest_path: PathBuf) -> BlastResult<Self> {
        let manifest = if manifest_path.exists() {
            Manifest::load(&manifest_path)?
        } else {
            // Create default manifest
            let env = blast_core::python::PythonEnvironment::new(
                manifest_path.parent().unwrap().to_path_buf(),
                blast_core::python::PythonVersion::default(),
            );
            Manifest::from_environment(&env)?
        };

        Ok(Self {
            manifest: Arc::new(RwLock::new(manifest)),
            manifest_path,
        })
    }

    /// Save manifest to disk
    async fn save_manifest(&self, manifest: &Manifest) -> BlastResult<()> {
        manifest.save(&self.manifest_path)
    }
}

#[async_trait]
impl ManifestManager for DaemonManifestManager {
    async fn get_manifest(&self) -> BlastResult<Manifest> {
        Ok(self.manifest.read().await.clone())
    }

    async fn update_manifest(&self, manifest: &Manifest) -> BlastResult<()> {
        *self.manifest.write().await = manifest.clone();
        self.save_manifest(manifest).await
    }

    async fn record_package_install(&self, package: &Package) -> BlastResult<()> {
        let mut manifest = self.manifest.write().await;
        manifest.record_package_install(
            package.name.clone(),
            package.version.to_string(),
        );
        self.save_manifest(&manifest).await
    }

    async fn record_package_removal(&self, package: &Package) -> BlastResult<()> {
        let mut manifest = self.get_manifest().await?;
        manifest.remove_package(package);
        self.save_manifest(&manifest).await
    }

    async fn record_env_var_change(&self, key: &str, value: &str) -> BlastResult<()> {
        let mut manifest = self.manifest.write().await;
        manifest.record_env_var_change(key.to_string(), value.to_string());
        self.save_manifest(&manifest).await
    }

    async fn record_system_dependency(&self, dependency: &SystemDependency) -> BlastResult<()> {
        let mut manifest = self.manifest.write().await;
        manifest.record_system_dependency(dependency.clone());
        self.save_manifest().await
    }

    async fn record_hook_addition(&self, hook_type: &str, command: &str) -> BlastResult<()> {
        let mut manifest = self.manifest.write().await;
        manifest.record_hook_addition(hook_type, command.to_string());
        self.save_manifest().await
    }

    async fn verify_manifest(&self) -> BlastResult<bool> {
        let manifest = self.manifest.read().await;
        manifest.metadata.verify()
    }
} 