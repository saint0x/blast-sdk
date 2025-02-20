use std::path::PathBuf;
use std::sync::Arc;
use std::fmt::Debug;

use super::{
    PythonVersion, PythonEnvironment, EnvironmentState,
    EnvironmentManager, DaemonResult, StateManagement,
};

use blast_core::config::{BlastConfig, DependenciesConfig};
use crate::state::StateManager;
use crate::monitor::MonitorEvent;

/// Debug wrapper for EnvironmentManager trait object
#[derive(Clone)]
struct DebugEnvironmentManager(Arc<dyn EnvironmentManager + Send + Sync>);

impl Debug for DebugEnvironmentManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvironmentManager").finish()
    }
}

/// Update service state
#[derive(Debug)]
pub struct UpdateServiceState {
    /// Environment path
    env_path: PathBuf,
    /// Cache path
    #[allow(dead_code)]
    cache_path: PathBuf,
    /// State manager
    state_manager: Arc<StateManager>,
    /// Environment manager
    #[allow(dead_code)]
    environment_manager: DebugEnvironmentManager,
}

impl UpdateServiceState {
    /// Create a new update service state
    pub fn new(config: super::ServiceConfig) -> Self {
        let state_manager = Arc::new(StateManager::new(config.env_path.clone()));
        
        Self {
            env_path: config.env_path,
            cache_path: config.cache_path,
            state_manager,
            environment_manager: DebugEnvironmentManager(config.environment_manager.0),
        }
    }

    /// Get the current state
    pub async fn get_current_state(&self) -> DaemonResult<EnvironmentState> {
        let state = self.state_manager
            .get_current_state()
            .await?;

        Ok(EnvironmentState::new(
            state.active_env_name.unwrap_or_default(),
            "default".to_string(),
            self.env_path.clone(),
            state.active_python_version.unwrap_or_else(|| PythonVersion::parse("3.8.0").unwrap()),
        ))
    }

    /// List checkpoints
    pub async fn list_checkpoints(&self, _state: &EnvironmentState) -> DaemonResult<Vec<String>> {
        // For now, just return empty list since StateManager doesn't have list method
        Ok(Vec::new())
    }

    /// Get checkpoint
    pub async fn get_checkpoint(&self, _state: &EnvironmentState, checkpoint_id: &str) -> DaemonResult<EnvironmentState> {
        // First restore the checkpoint
        self.state_manager
            .restore_checkpoint(checkpoint_id)
            .await?;
        
        // Then get the current state which will have the restored checkpoint data
        let state = self.state_manager
            .get_current_state()
            .await?;

        Ok(EnvironmentState::new(
            state.active_env_name.unwrap_or_default(),
            "default".to_string(),
            self.env_path.clone(),
            state.active_python_version.unwrap_or_else(|| PythonVersion::parse("3.8.0").unwrap()),
        ))
    }

    /// Create environment
    pub async fn create_environment(&self, python_version: PythonVersion) -> DaemonResult<()> {
        let config = BlastConfig {
            name: "default".to_string(),
            version: "0.1.0".to_string(),
            python_version,
            project_root: self.env_path.clone(),
            env_dir: self.env_path.join("envs"),
            cache_settings: Default::default(),
            update_strategy: Default::default(),
            dependencies: DependenciesConfig::default(),
            dev_dependencies: None,
        };

        self.environment_manager.0
            .create_environment(&config)
            .await?;

        Ok(())
    }

    /// Remove environment
    pub async fn remove_environment(&self) -> DaemonResult<()> {
        let env = PythonEnvironment::new(
            "default".to_string(),
            self.env_path.clone(),
            PythonVersion::parse("3.8.0").unwrap(),
        ).await?;

        self.environment_manager.0
            .deactivate_environment(&env)
            .await?;

        // Since there's no remove method, we'll just deactivate for now
        Ok(())
    }

    /// Activate environment
    pub async fn activate_environment(&self) -> DaemonResult<()> {
        let env = PythonEnvironment::new(
            "default".to_string(),
            self.env_path.clone(),
            PythonVersion::parse("3.8.0").unwrap(),
        ).await?;

        self.environment_manager.0
            .activate_environment(&env)
            .await?;

        Ok(())
    }

    /// Deactivate environment
    pub async fn deactivate_environment(&self) -> DaemonResult<()> {
        let env = PythonEnvironment::new(
            "default".to_string(),
            self.env_path.clone(),
            PythonVersion::parse("3.8.0").unwrap(),
        ).await?;

        self.environment_manager.0
            .deactivate_environment(&env)
            .await?;

        // Since there's no remove method, we'll just deactivate for now
        Ok(())
    }

    /// Cleanup old snapshots
    pub async fn cleanup_old_snapshots(&self, _max_age_days: u64) -> DaemonResult<()> {
        // StateManager doesn't have cleanup method yet
        Ok(())
    }

    /// Handle package update
    pub async fn handle_package_update(&self, _event: MonitorEvent) -> DaemonResult<()> {
        // Implementation details...
        Ok(())
    }
} 