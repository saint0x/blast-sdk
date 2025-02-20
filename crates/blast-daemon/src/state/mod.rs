mod manager;
mod types;

pub use manager::*;
pub use types::*;

use std::path::PathBuf;
use blast_core::python::PythonVersion;
use crate::error::DaemonResult;

/// State management trait
#[async_trait::async_trait]
pub trait StateManagement {
    /// Get current state
    async fn get_current_state(&self) -> DaemonResult<types::State>;
    
    /// Update current state
    async fn update_current_state(&self, state: types::State) -> DaemonResult<()>;
    
    /// Create checkpoint
    async fn create_checkpoint(&self, id: uuid::Uuid, description: String, metadata: Option<serde_json::Value>) -> DaemonResult<()>;
    
    /// Restore checkpoint
    async fn restore_checkpoint(&self, id: &str) -> DaemonResult<()>;
    
    /// Set active environment
    async fn set_active_environment(&self, name: String, path: PathBuf, python_version: PythonVersion) -> DaemonResult<()>;
    
    /// Clear active environment
    async fn clear_active_environment(&self) -> DaemonResult<()>;

    /// Save state to disk
    async fn save(&self) -> DaemonResult<()>;

    /// Load state from disk
    async fn load(&self) -> DaemonResult<()>;
} 