use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use blast_core::python::PythonVersion;
use crate::error::DaemonResult;
use super::{State, StateManagement};

/// State manager checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint ID
    pub id: String,
    /// Checkpoint description
    pub description: String,
    /// Checkpoint timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Checkpoint state
    pub state: State,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// State manager
#[derive(Debug)]
pub struct StateManager {
    /// Root path
    root_path: PathBuf,
    /// Current state
    current_state: Arc<RwLock<State>>,
    /// Checkpoints
    checkpoints: Arc<RwLock<HashMap<String, Checkpoint>>>,
}

impl StateManager {
    /// Create new state manager
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            current_state: Arc::new(RwLock::new(State::default())),
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get root path
    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    /// Verify state manager access
    pub async fn verify(&self) -> DaemonResult<()> {
        // Verify state file exists and is writable
        let state_file = self.root_path.join("state.json");
        if !state_file.exists() {
            tokio::fs::write(&state_file, "{}").await?;
        }

        // Verify checkpoints directory exists and is writable
        let checkpoints_dir = self.root_path.join("checkpoints");
        if !checkpoints_dir.exists() {
            tokio::fs::create_dir_all(&checkpoints_dir).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl StateManagement for StateManager {
    async fn get_current_state(&self) -> DaemonResult<State> {
        Ok(self.current_state.read().await.clone())
    }

    async fn update_current_state(&self, state: State) -> DaemonResult<()> {
        *self.current_state.write().await = state;
        Ok(())
    }

    async fn create_checkpoint(&self, id: uuid::Uuid, description: String, metadata: Option<serde_json::Value>) -> DaemonResult<()> {
        let checkpoint = Checkpoint {
            id: id.to_string(),
            description,
            timestamp: chrono::Utc::now(),
            state: self.current_state.read().await.clone(),
            metadata,
        };

        self.checkpoints.write().await.insert(checkpoint.id.clone(), checkpoint);
        Ok(())
    }

    async fn restore_checkpoint(&self, id: &str) -> DaemonResult<()> {
        if let Some(checkpoint) = self.checkpoints.read().await.get(id) {
            *self.current_state.write().await = checkpoint.state.clone();
            Ok(())
        } else {
            Err(crate::error::DaemonError::state(format!("Checkpoint {} not found", id)))
        }
    }

    async fn set_active_environment(&self, name: String, path: PathBuf, python_version: PythonVersion) -> DaemonResult<()> {
        let mut state = self.current_state.write().await;
        state.active_env_name = Some(name);
        state.active_env_path = Some(path);
        state.active_python_version = Some(python_version);
        Ok(())
    }

    async fn clear_active_environment(&self) -> DaemonResult<()> {
        let mut state = self.current_state.write().await;
        state.active_env_name = None;
        state.active_env_path = None;
        state.active_python_version = None;
        Ok(())
    }

    async fn save(&self) -> DaemonResult<()> {
        let state_file = self.root_path.join("state.json");
        let state = self.current_state.read().await;
        let state_json = serde_json::to_string_pretty(&*state)?;
        tokio::fs::write(&state_file, state_json).await?;
        Ok(())
    }

    async fn load(&self) -> DaemonResult<()> {
        let state_file = self.root_path.join("state.json");
        if state_file.exists() {
            let state_json = tokio::fs::read_to_string(&state_file).await?;
            let state = serde_json::from_str(&state_json)?;
            *self.current_state.write().await = state;
        }
        Ok(())
    }
} 