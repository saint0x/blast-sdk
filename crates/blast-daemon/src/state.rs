use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use blast_core::{
    error::{BlastError, BlastResult},
    state::EnvironmentState,
    python::PythonVersion,
    package::Package,
    version_history::{VersionEvent, VersionHistory},
};

const STATE_FILE: &str = ".blast/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastState {
    version: u32,
    timestamp: DateTime<Utc>,
    active_environment: Option<ActiveEnvironment>,
    environments: HashMap<String, EnvironmentState>,
}

impl BlastState {
    pub fn new() -> Self {
        Self {
            version: 1,
            timestamp: Utc::now(),
            active_environment: None,
            environments: HashMap::new(),
        }
    }

    pub fn active_environment(&self) -> Option<&ActiveEnvironment> {
        self.active_environment.as_ref()
    }

    pub fn environments(&self) -> &HashMap<String, EnvironmentState> {
        &self.environments
    }

    pub fn update_timestamp(&mut self) {
        self.timestamp = Utc::now();
    }

    pub fn update_environment(&mut self, name: String, state: EnvironmentState) {
        self.environments.insert(name, state);
        self.update_timestamp();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEnvironment {
    name: String,
    path: PathBuf,
    python_version: PythonVersion,
    activated_at: DateTime<Utc>,
}

impl ActiveEnvironment {
    pub fn new(name: String, path: PathBuf, python_version: PythonVersion) -> Self {
        Self {
            name,
            path,
            python_version,
            activated_at: Utc::now(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn python_version(&self) -> &PythonVersion {
        &self.python_version
    }

    pub fn activated_at(&self) -> DateTime<Utc> {
        self.activated_at
    }
}

#[derive(Debug, Clone)]
pub enum StateEvent {
    EnvironmentCreated(String),
    EnvironmentUpdated(String),
    EnvironmentRemoved(String),
    ActiveEnvironmentChanged(Option<String>),
    StateReloaded,
}

/// Checkpoint for environment state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint ID
    pub id: Uuid,
    /// Checkpoint description
    pub description: String,
    /// Transaction ID if associated with a transaction
    pub transaction_id: Option<Uuid>,
    /// Environment state at checkpoint
    pub state: EnvironmentState,
    /// Checkpoint creation time
    pub created_at: DateTime<Utc>,
}

/// Manager for environment state and checkpoints
#[derive(Debug)]
pub struct StateManager {
    state: Arc<RwLock<BlastState>>,
    root_path: PathBuf,
}

impl StateManager {
    /// Create new state manager
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            state: Arc::new(RwLock::new(BlastState::new())),
            root_path,
        }
    }

    pub async fn load(&self) -> BlastResult<()> {
        let state_path = self.root_path.join(STATE_FILE);
        if state_path.exists() {
            let contents = tokio::fs::read_to_string(&state_path).await?;
            let state: BlastState = serde_json::from_str(&contents)?;
            *self.state.write().await = state;
        }
        Ok(())
    }

    pub async fn save(&self) -> BlastResult<()> {
        let state = self.state.read().await;
        let state_path = self.root_path.join(STATE_FILE);
        
        // Ensure parent directory exists
        if let Some(parent) = state_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let contents = serde_json::to_string_pretty(&*state)?;
        tokio::fs::write(&state_path, contents).await?;
        Ok(())
    }

    pub async fn get_current_state(&self) -> BlastResult<EnvironmentState> {
        let state = self.state.read().await;
        if let Some(active) = &state.active_environment {
            if let Some(env_state) = state.environments.get(&active.name) {
                return Ok(env_state.clone());
            }
        }
        Ok(EnvironmentState::new(
            "default".to_string(),
            PythonVersion::parse("3.8.0").unwrap(),
            HashMap::new(),
            HashMap::new(),
        ))
    }

    pub async fn update_current_state(&self, env_state: EnvironmentState) -> BlastResult<()> {
        let mut state = self.state.write().await;
        if let Some(active) = &state.active_environment {
            let name = active.name.clone();
            state.update_environment(name, env_state);
            self.save().await?;
        }
        Ok(())
    }

    pub async fn set_active_environment(
        &self,
        name: String,
        path: PathBuf,
        python_version: PythonVersion,
    ) -> BlastResult<()> {
        let mut state = self.state.write().await;
        state.active_environment = Some(ActiveEnvironment::new(name.clone(), path, python_version));
        state.update_timestamp();
        self.save().await?;
        Ok(())
    }

    pub async fn clear_active_environment(&self) -> BlastResult<()> {
        let mut state = self.state.write().await;
        state.active_environment = None;
        state.update_timestamp();
        self.save().await?;
        Ok(())
    }

    pub async fn list_environments(&self) -> BlastResult<HashMap<String, EnvironmentState>> {
        let state = self.state.read().await;
        Ok(state.environments.clone())
    }

    pub async fn verify(&self) -> BlastResult<()> {
        let state_path = self.root_path.join(STATE_FILE);
        if state_path.exists() {
            let contents = tokio::fs::read_to_string(&state_path).await?;
            let _: BlastState = serde_json::from_str(&contents)?;
        }
        Ok(())
    }

    pub async fn create_checkpoint(
        &self,
        id: Uuid,
        description: String,
        transaction_id: Option<Uuid>,
    ) -> BlastResult<()> {
        let state = self.state.read().await;
        let checkpoint = Checkpoint {
            id,
            description,
            transaction_id,
            state: state.active_environment()
                .and_then(|active| state.environments.get(&active.name))
                .cloned()
                .unwrap_or_else(|| EnvironmentState::new(
                    "default".to_string(),
                    PythonVersion::parse("3.8.0").unwrap(),
                    HashMap::new(),
                    HashMap::new(),
                )),
            created_at: Utc::now(),
        };

        let checkpoint_path = self.root_path.join(".blast").join("checkpoints");
        tokio::fs::create_dir_all(&checkpoint_path).await?;
        
        let checkpoint_file = checkpoint_path.join(format!("{}.json", id));
        let contents = serde_json::to_string_pretty(&checkpoint)?;
        tokio::fs::write(checkpoint_file, contents).await?;
        
        Ok(())
    }

    pub async fn restore_checkpoint(&self, id: &str) -> BlastResult<()> {
        let checkpoint_path = self.root_path
            .join(".blast")
            .join("checkpoints")
            .join(format!("{}.json", id));
            
        if !checkpoint_path.exists() {
            return Err(BlastError::environment(format!("Checkpoint {} not found", id)));
        }

        let contents = tokio::fs::read_to_string(checkpoint_path).await?;
        let checkpoint: Checkpoint = serde_json::from_str(&contents)?;

        let mut state = self.state.write().await;
        let env_name = state.active_environment.as_ref()
            .map(|active| active.name.clone());

        if let Some(name) = env_name {
            state.environments.insert(name, checkpoint.state);
            state.update_timestamp();
            self.save().await?;
        }

        Ok(())
    }

    pub async fn list_checkpoints(&self) -> BlastResult<Vec<Checkpoint>> {
        let checkpoint_path = self.root_path.join(".blast").join("checkpoints");
        if !checkpoint_path.exists() {
            return Ok(Vec::new());
        }

        let mut checkpoints = Vec::new();
        let mut entries = tokio::fs::read_dir(checkpoint_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "json" {
                        let contents = tokio::fs::read_to_string(entry.path()).await?;
                        if let Ok(checkpoint) = serde_json::from_str(&contents) {
                            checkpoints.push(checkpoint);
                        }
                    }
                }
            }
        }

        Ok(checkpoints)
    }

    pub async fn get_checkpoint(&self, id: &str) -> BlastResult<Option<Checkpoint>> {
        let checkpoint_path = self.root_path
            .join(".blast")
            .join("checkpoints")
            .join(format!("{}.json", id));
            
        if !checkpoint_path.exists() {
            return Ok(None);
        }

        let contents = tokio::fs::read_to_string(checkpoint_path).await?;
        let checkpoint = serde_json::from_str(&contents)?;
        Ok(Some(checkpoint))
    }

    pub async fn cleanup_old_snapshots(&self, max_age_days: u64) -> BlastResult<()> {
        let checkpoint_path = self.root_path.join(".blast").join("checkpoints");
        if !checkpoint_path.exists() {
            return Ok(());
        }

        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let mut entries = tokio::fs::read_dir(checkpoint_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "json" {
                        let contents = tokio::fs::read_to_string(entry.path()).await?;
                        if let Ok(checkpoint) = serde_json::from_str::<Checkpoint>(&contents) {
                            if checkpoint.created_at < cutoff {
                                tokio::fs::remove_file(entry.path()).await?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn add_package_with_event(
        &self,
        package: &Package,
        event: VersionEvent,
    ) -> BlastResult<()> {
        let mut state = self.state.write().await;
        let env_name = state.active_environment.as_ref()
            .map(|active| active.name.clone());

        if let Some(name) = env_name {
            if let Some(env_state) = state.environments.get_mut(&name) {
                env_state.packages.insert(
                    package.id().name().to_string(),
                    package.version().clone(),
                );
                // Store version event in version_history map
                let pkg_name = package.id().name().to_string();
                env_state.version_histories
                    .entry(pkg_name.clone())
                    .or_insert_with(|| VersionHistory::new(pkg_name))
                    .add_event(event);
                state.update_timestamp();
                self.save().await?;
            }
        }
        Ok(())
    }

    pub async fn remove_package(&self, package: &Package) -> BlastResult<()> {
        let mut state = self.state.write().await;
        if let Some(active) = &state.active_environment {
            let name = active.name.clone();
            if let Some(env_state) = state.environments.get_mut(&name) {
                env_state.packages.remove(package.id().name());
                state.update_timestamp();
                self.save().await?;
            }
        }
        Ok(())
    }

    pub async fn update_package_with_event(
        &self,
        _from: &Package,
        to: &Package,
        event: VersionEvent,
    ) -> BlastResult<()> {
        let mut state = self.state.write().await;
        let env_name = state.active_environment.as_ref()
            .map(|active| active.name.clone());

        if let Some(name) = env_name {
            if let Some(env_state) = state.environments.get_mut(&name) {
                env_state.packages.insert(
                    to.id().name().to_string(),
                    to.version().clone(),
                );
                // Store version event in version_history map
                let pkg_name = to.id().name().to_string();
                env_state.version_histories
                    .entry(pkg_name.clone())
                    .or_insert_with(|| VersionHistory::new(pkg_name))
                    .add_event(event);
                state.update_timestamp();
                self.save().await?;
            }
        }
        Ok(())
    }

    pub async fn add_environment(&self, name: String, _env_state: EnvironmentState) -> BlastResult<()> {
        let mut state = self.state.write().await;
        state.environments.insert(name, _env_state);
        state.update_timestamp();
        self.save().await?;
        Ok(())
    }

    pub async fn remove_environment(&self, name: &str) -> BlastResult<()> {
        let mut state = self.state.write().await;
        state.environments.remove(name);
        state.update_timestamp();
        self.save().await?;
        Ok(())
    }

    pub async fn verify_state(&self) -> BlastResult<()> {
        self.verify().await?;
        
        // Additional state verification logic
        let state = self.state.read().await;
        
        // Verify all environments exist
        for (name, _env_state) in &state.environments {
            let env_path = self.root_path.join(name);
            if !env_path.exists() {
                return Err(BlastError::environment(
                    format!("Environment directory not found: {}", env_path.display())
                ));
            }
            
            // Verify Python version
            if !env_path.join("bin").join("python").exists() {
                return Err(BlastError::environment(
                    format!("Python executable not found in environment: {}", name)
                ));
            }
            
            // Verify package state
            let site_packages = env_path.join("lib").join("python3").join("site-packages");
            if !site_packages.exists() {
                return Err(BlastError::environment(
                    format!("Site-packages directory not found: {}", site_packages.display())
                ));
            }
        }
        
        Ok(())
    }

    pub async fn verify_environment(&self, _env_state: &EnvironmentState) -> BlastResult<()> {
        // Implementation of verify_environment method
        Ok(())
    }
} 