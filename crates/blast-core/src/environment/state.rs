use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::error::BlastResult;
use crate::state::SyncState;
use crate::python::PythonVersion;

/// Environment state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentState {
    /// Environment ID
    pub id: String,
    /// Environment name
    pub name: String,
    /// Environment path
    pub path: PathBuf,
    /// Python version
    pub python_version: PythonVersion,
    /// Creation time
    pub created_at: SystemTime,
    /// Last modified time
    pub modified_at: SystemTime,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Package state
    pub package_state: PackageState,
    /// Container state
    pub container_state: ContainerState,
    /// Resource state
    pub resource_state: ResourceState,
    /// Security state
    pub security_state: SecurityState,
    /// Sync state
    pub sync_state: SyncState,
}

impl EnvironmentState {
    /// Create new environment state
    pub fn new(
        id: String,
        name: String,
        path: PathBuf,
        python_version: PythonVersion,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name,
            path,
            python_version,
            created_at: now,
            modified_at: now,
            env_vars: HashMap::new(),
            package_state: PackageState::default(),
            container_state: ContainerState::default(),
            resource_state: ResourceState::default(),
            security_state: SecurityState::default(),
            sync_state: SyncState::default(),
        }
    }

    /// Update modified time
    pub fn touch(&mut self) {
        self.modified_at = SystemTime::now();
    }

    /// Add environment variable
    pub fn add_env_var(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
        self.touch();
    }

    /// Remove environment variable
    pub fn remove_env_var(&mut self, key: &str) -> Option<String> {
        let value = self.env_vars.remove(key);
        if value.is_some() {
            self.touch();
        }
        value
    }

    /// Get environment variable
    pub fn get_env_var(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }
}

/// Package state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageState {
    /// Installed packages
    pub installed: HashMap<String, PackageInfo>,
    /// Package requirements
    pub requirements: Vec<String>,
    /// Package constraints
    pub constraints: Vec<String>,
    /// Package sources
    pub sources: Vec<String>,
}

/// Package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Installation time
    pub installed_at: SystemTime,
    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Container state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerState {
    /// Container ID
    pub id: Option<String>,
    /// Container status
    pub status: ContainerStatus,
    /// Container PID
    pub pid: Option<u32>,
    /// Container network
    pub network: Option<NetworkState>,
    /// Container mounts
    pub mounts: Vec<MountState>,
}

/// Container status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerStatus {
    /// Container is created
    Created,
    /// Container is running
    Running,
    /// Container is paused
    Paused,
    /// Container is stopped
    Stopped,
    /// Container is deleted
    Deleted,
}

impl Default for ContainerStatus {
    fn default() -> Self {
        Self::Created
    }
}

/// Network state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkState {
    /// Network ID
    pub id: String,
    /// Network name
    pub name: String,
    /// Network type
    pub network_type: String,
    /// IP address
    pub ip_address: Option<String>,
    /// Gateway
    pub gateway: Option<String>,
    /// DNS servers
    pub dns: Vec<String>,
}

/// Mount state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountState {
    /// Mount source
    pub source: PathBuf,
    /// Mount target
    pub target: PathBuf,
    /// Mount type
    pub mount_type: String,
    /// Mount options
    pub options: Vec<String>,
}

/// Resource state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceState {
    /// CPU usage
    pub cpu_usage: f64,
    /// Memory usage
    pub memory_usage: u64,
    /// Disk usage
    pub disk_usage: u64,
    /// Network usage
    pub network_usage: NetworkUsage,
}

/// Network usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkUsage {
    /// Bytes received
    pub rx_bytes: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Packets transmitted
    pub tx_packets: u64,
}

/// Security state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityState {
    /// Current capabilities
    pub capabilities: Vec<String>,
    /// Seccomp status
    pub seccomp_enabled: bool,
    /// AppArmor profile
    pub apparmor_profile: Option<String>,
    /// SELinux context
    pub selinux_context: Option<String>,
}

/// State manager implementation
pub struct StateManager {
    /// Environment state
    state: RwLock<EnvironmentState>,
}

impl StateManager {
    /// Create new state manager
    pub fn new(state: EnvironmentState) -> Self {
        Self {
            state: RwLock::new(state),
        }
    }

    /// Get environment state
    pub async fn get_state(&self) -> BlastResult<EnvironmentState> {
        Ok(self.state.read().await.clone())
    }

    /// Update environment state
    pub async fn update_state<F>(&self, f: F) -> BlastResult<()>
    where
        F: FnOnce(&mut EnvironmentState) -> BlastResult<()>,
    {
        let mut state = self.state.write().await;
        f(&mut state)?;
        state.touch();
        Ok(())
    }

    /// Save environment state
    pub async fn save_state(&self, path: &PathBuf) -> BlastResult<()> {
        let state = self.state.read().await;
        let state_json = serde_json::to_string_pretty(&*state)?;
        tokio::fs::write(path, state_json).await?;
        Ok(())
    }

    /// Load environment state
    pub async fn load_state(path: &PathBuf) -> BlastResult<Self> {
        let state_json = tokio::fs::read_to_string(path).await?;
        let state = serde_json::from_str(&state_json)?;
        Ok(Self::new(state))
    }

    /// Update package state
    pub async fn update_package_state<F>(&self, f: F) -> BlastResult<()>
    where
        F: FnOnce(&mut PackageState) -> BlastResult<()>,
    {
        self.update_state(|state| {
            f(&mut state.package_state)?;
            Ok(())
        }).await
    }

    /// Update container state
    pub async fn update_container_state<F>(&self, f: F) -> BlastResult<()>
    where
        F: FnOnce(&mut ContainerState) -> BlastResult<()>,
    {
        self.update_state(|state| {
            f(&mut state.container_state)?;
            Ok(())
        }).await
    }

    /// Update resource state
    pub async fn update_resource_state<F>(&self, f: F) -> BlastResult<()>
    where
        F: FnOnce(&mut ResourceState) -> BlastResult<()>,
    {
        self.update_state(|state| {
            f(&mut state.resource_state)?;
            Ok(())
        }).await
    }

    /// Update security state
    pub async fn update_security_state<F>(&self, f: F) -> BlastResult<()>
    where
        F: FnOnce(&mut SecurityState) -> BlastResult<()>,
    {
        self.update_state(|state| {
            f(&mut state.security_state)?;
            Ok(())
        }).await
    }

    /// Update sync state
    pub async fn update_sync_state<F>(&self, f: F) -> BlastResult<()>
    where
        F: FnOnce(&mut SyncState) -> BlastResult<()>,
    {
        self.update_state(|state| {
            f(&mut state.sync_state)?;
            Ok(())
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_environment_state() {
        let state = EnvironmentState::new(
            "test-id".to_string(),
            "test-env".to_string(),
            PathBuf::from("/tmp/test-env"),
            PythonVersion::new(3, 9, Some(0)),
        );

        assert_eq!(state.id, "test-id");
        assert_eq!(state.name, "test-env");
        assert_eq!(state.python_version, PythonVersion::new(3, 9, Some(0)));
    }

    #[tokio::test]
    async fn test_state_manager() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.json");

        let state = EnvironmentState::new(
            "test-id".to_string(),
            "test-env".to_string(),
            PathBuf::from("/tmp/test-env"),
            PythonVersion::new(3, 9, Some(0)),
        );

        let manager = StateManager::new(state);

        // Test updating state
        manager.update_state(|state| {
            state.add_env_var("TEST_VAR".to_string(), "test_value".to_string());
            Ok(())
        }).await.unwrap();

        // Test saving state
        manager.save_state(&state_path).await.unwrap();

        // Test loading state
        let loaded_manager = StateManager::load_state(&state_path).await.unwrap();
        let loaded_state = loaded_manager.get_state().await.unwrap();

        assert_eq!(
            loaded_state.get_env_var("TEST_VAR").unwrap(),
            "test_value"
        );
    }

    #[tokio::test]
    async fn test_package_state() {
        let state = EnvironmentState::new(
            "test-id".to_string(),
            "test-env".to_string(),
            PathBuf::from("/tmp/test-env"),
            PythonVersion::new(3, 9, Some(0)),
        );

        let manager = StateManager::new(state);

        // Test updating package state
        manager.update_package_state(|state| {
            state.installed.insert(
                "test-package".to_string(),
                PackageInfo {
                    name: "test-package".to_string(),
                    version: "1.0.0".to_string(),
                    installed_at: SystemTime::now(),
                    dependencies: Vec::new(),
                },
            );
            Ok(())
        }).await.unwrap();

        let state = manager.get_state().await.unwrap();
        assert!(state.package_state.installed.contains_key("test-package"));
    }
} 