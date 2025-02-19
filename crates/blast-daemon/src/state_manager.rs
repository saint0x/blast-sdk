use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::fs;
use serde_json;

use blast_core::{
    error::BlastResult,
    package::Package,
    state::{EnvironmentState, StateVerification},
    python::PythonVersion,
};

use crate::{
    DaemonError,
    DaemonResult,
    metrics::MetricsManager,
};

const STATE_FILE: &str = "daemon_state.json";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistentState {
    current_state: EnvironmentState,
    history: Vec<StateSnapshot>,
}

/// State manager for the daemon
#[derive(Debug)]
pub struct StateManager {
    /// Current environment state
    state: Arc<RwLock<EnvironmentState>>,
    /// State history for rollbacks
    history: Arc<RwLock<Vec<StateSnapshot>>>,
    /// Metrics manager
    metrics: Arc<MetricsManager>,
    /// Environment path
    env_path: PathBuf,
    /// State file path
    state_file: PathBuf,
}

/// Snapshot of daemon state
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    /// Snapshot ID
    pub id: Uuid,
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
    /// Environment state
    pub state: EnvironmentState,
    /// Verification result
    pub verification: Option<StateVerification>,
    /// Description
    pub description: String,
}

impl StateManager {
    /// Create a new state manager
    pub fn new(env_path: PathBuf, metrics: Arc<MetricsManager>) -> Self {
        let state_file = env_path.join(STATE_FILE);
        let initial_state = EnvironmentState::new(
            "default".to_string(),
            PythonVersion::parse("3.8.0").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let instance = Self {
            state: Arc::new(RwLock::new(initial_state)),
            history: Arc::new(RwLock::new(Vec::new())),
            metrics,
            env_path: env_path.clone(),
            state_file,
        };

        // Try to load existing state
        tokio::spawn(async move {
            if let Err(e) = instance.load_state().await {
                if std::env::var("BLAST_SCRIPT_OUTPUT").is_err() {
                    eprintln!("Failed to load state: {}", e);
                }
            }
        });

        instance
    }

    /// Load state from disk
    async fn load_state(&self) -> DaemonResult<()> {
        if self.state_file.exists() {
            let contents = fs::read_to_string(&self.state_file).await
                .map_err(|e| DaemonError::State(format!("Failed to read state file: {}", e)))?;
            
            let persistent_state: PersistentState = serde_json::from_str(&contents)
                .map_err(|e| DaemonError::State(format!("Failed to parse state file: {}", e)))?;
            
            *self.state.write().await = persistent_state.current_state;
            *self.history.write().await = persistent_state.history;
        }
        Ok(())
    }

    /// Save state to disk
    async fn save_state(&self) -> DaemonResult<()> {
        let persistent_state = PersistentState {
            current_state: self.state.read().await.clone(),
            history: self.history.read().await.clone(),
        };

        let contents = serde_json::to_string_pretty(&persistent_state)
            .map_err(|e| DaemonError::State(format!("Failed to serialize state: {}", e)))?;
        
        fs::write(&self.state_file, contents).await
            .map_err(|e| DaemonError::State(format!("Failed to write state file: {}", e)))?;
        
        Ok(())
    }

    /// Update current state
    pub async fn update_current_state(&self, new_state: EnvironmentState) -> DaemonResult<()> {
        *self.state.write().await = new_state;
        self.save_state().await
    }

    /// Get current state
    pub async fn get_current_state(&self) -> DaemonResult<EnvironmentState> {
        Ok(self.state.read().await.clone())
    }

    /// Create a state snapshot
    pub async fn create_snapshot(&self, description: String) -> DaemonResult<Uuid> {
        let state = self.state.read().await.clone();
        let snapshot = StateSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            state,
            verification: None,
            description,
        };

        self.history.write().await.push(snapshot.clone());
        Ok(snapshot.id)
    }

    /// Get a state snapshot
    pub async fn get_snapshot(&self, id: Uuid) -> DaemonResult<Option<StateSnapshot>> {
        Ok(self.history.read().await.iter()
            .find(|s| s.id == id)
            .cloned())
    }

    /// Restore from a snapshot
    pub async fn restore_snapshot(&self, id: Uuid) -> DaemonResult<()> {
        let snapshot = self.get_snapshot(id).await?
            .ok_or_else(|| DaemonError::State(format!("Snapshot {} not found", id)))?;

        // Create a new snapshot of current state before restoring
        self.create_snapshot("Pre-restore state".to_string()).await?;

        // Restore state
        *self.state.write().await = snapshot.state;

        // Record metrics
        self.metrics.record_state_restore(id).await;

        Ok(())
    }

    /// Update environment state
    pub async fn update_state(&self, packages: &[Package]) -> DaemonResult<()> {
        let mut state = self.state.write().await;
        
        // Create snapshot before update
        let snapshot_id = self.create_snapshot("Pre-update state".to_string()).await?;

        // Update packages
        for package in packages {
            state.packages.insert(
                package.name().to_string(),
                package.version().clone(),
            );
        }

        // Record metrics
        self.metrics.record_state_update(snapshot_id, packages.len()).await;

        Ok(())
    }

    /// Verify current state
    pub async fn verify_state(&self) -> DaemonResult<StateVerification> {
        let state = self.state.read().await;
        let mut verification = StateVerification::default();

        // Verify environment structure
        if !self.env_path.exists() {
            verification.add_error(
                "Environment directory not found".to_string(),
                None,
            );
        }

        // Verify Python installation
        let python_path = self.env_path.join("bin").join("python");
        if !python_path.exists() {
            verification.add_error(
                "Python executable not found".to_string(),
                None,
            );
        }

        // Verify package installations
        for (name, version) in &state.packages {
            let site_packages = self.env_path
                .join("lib")
                .join("python3")
                .join("site-packages")
                .join(name);

            if !site_packages.exists() {
                verification.add_warning(
                    format!("Package {} installation not found", name),
                    Some(format!("Version: {}", version)),
                );
            }
        }

        Ok(verification)
    }

    /// Clean up old snapshots
    pub async fn cleanup_snapshots(&self, max_age: chrono::Duration) -> DaemonResult<usize> {
        let now = Utc::now();
        let mut history = self.history.write().await;
        let initial_len = history.len();

        history.retain(|snapshot| {
            now.signed_duration_since(snapshot.timestamp) <= max_age
        });

        Ok(initial_len - history.len())
    }
} 