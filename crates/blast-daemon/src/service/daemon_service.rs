use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use blast_core::{
    python::PythonEnvironment,
    security::SecurityPolicy,
    environment::isolation::{SecurityManager, IsolationManager},
};

use crate::error::DaemonResult;
use crate::state::StateManager;

/// Daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Maximum number of pending updates
    pub max_pending_updates: usize,
    /// Maximum age of snapshots in days
    pub max_snapshot_age_days: u64,
    /// Environment path
    pub env_path: PathBuf,
    /// Cache path
    pub cache_path: PathBuf,
}

/// Daemon service
#[derive(Debug, Clone)]
pub struct Daemon {
    /// State manager
    state_manager: Arc<RwLock<StateManager>>,
    /// Security manager
    security_manager: Option<Arc<RwLock<SecurityManager>>>,
    /// Configuration
    config: DaemonConfig,
}

impl Daemon {
    /// Create a new daemon
    pub async fn new(config: DaemonConfig) -> DaemonResult<Self> {
        let state_manager = Arc::new(RwLock::new(StateManager::new(config.env_path.clone())));
        
        // Create security manager
        let security_manager = SecurityManager::new(
            config.env_path.clone(),
            IsolationManager::default(),
        );
        
        Ok(Self {
            state_manager,
            security_manager: Some(Arc::new(RwLock::new(security_manager))),
            config,
        })
    }

    /// Get state manager
    pub fn state_manager(&self) -> Arc<RwLock<StateManager>> {
        self.state_manager.clone()
    }
    
    /// Get security manager
    pub fn security_manager(&self) -> Option<Arc<RwLock<SecurityManager>>> {
        self.security_manager.clone()
    }

    /// Verify daemon access and permissions
    pub async fn verify_access(&self) -> DaemonResult<()> {
        // Verify state manager access
        let state_manager = self.state_manager.read().await;
        state_manager.verify().await?;

        // Verify environment path exists and is writable
        if !self.config.env_path.exists() {
            tokio::fs::create_dir_all(&self.config.env_path).await?;
        }

        // Verify cache path exists and is writable
        if !self.config.cache_path.exists() {
            tokio::fs::create_dir_all(&self.config.cache_path).await?;
        }

        Ok(())
    }

    /// Create environment
    pub async fn create_environment(&self, security_policy: &SecurityPolicy) -> DaemonResult<PythonEnvironment> {
        let env = PythonEnvironment::new(
            "default".to_string(),
            self.config.env_path.clone(),
            security_policy.python_version.clone(),
        ).await?;

        // Initialize security if available
        if let Some(security_manager) = &self.security_manager {
            let mut security_manager = security_manager.write().await;
            security_manager.register_environment(&env, security_policy).await?;
        }

        Ok(env)
    }

    /// Start background service
    pub async fn start_background(&self) -> DaemonResult<()> {
        // Start security manager if available
        if let Some(security_manager) = &self.security_manager {
            let security_manager = security_manager.read().await;
            security_manager.start_monitoring().await?;
        }
        
        Ok(())
    }
} 