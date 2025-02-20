use std::path::PathBuf;
use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::error::BlastResult;
use super::{
    NamespaceConfig, CGroupConfig, NetworkPolicy, FilesystemPolicy,
    NetworkState, FilesystemState,
};

/// Container runtime trait
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Create namespaces
    async fn create_namespaces(&self, config: &NamespaceConfig) -> BlastResult<()>;
    
    /// Setup cgroups
    async fn setup_cgroups(&self, config: &CGroupConfig) -> BlastResult<()>;
    
    /// Configure network
    async fn configure_network(&self, policy: &NetworkPolicy) -> BlastResult<()>;
    
    /// Initialize container
    async fn initialize(&self) -> BlastResult<()>;
    
    /// Setup filesystem
    async fn setup_filesystem(&self, policy: &FilesystemPolicy) -> BlastResult<()>;
    
    /// Get container state
    async fn get_state(&self) -> BlastResult<ContainerState>;
    
    /// Cleanup container
    async fn cleanup(&self) -> BlastResult<()>;
}

/// Container implementation
pub struct Container {
    /// Container ID
    id: String,
    /// Root directory
    root_dir: PathBuf,
    /// Network state
    network: Arc<RwLock<NetworkState>>,
    /// Filesystem state
    filesystem: Arc<RwLock<FilesystemState>>,
    /// Container state
    state: Arc<RwLock<ContainerState>>,
}

impl Container {
    /// Create new container
    pub async fn new(config: &ContainerConfig) -> BlastResult<Self> {
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            root_dir: config.root_dir.clone(),
            network: Arc::new(RwLock::new(NetworkState::new(config.network_policy.clone()))),
            filesystem: Arc::new(RwLock::new(FilesystemState::new(config.filesystem_policy.clone()))),
            state: Arc::new(RwLock::new(ContainerState::default())),
        })
    }

    /// Get container ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get container root directory
    pub fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    /// Get container runtime state
    pub fn runtime_state(&self) -> &ContainerState {
        &self.state.read().await
    }
}

#[async_trait]
impl ContainerRuntime for Container {
    async fn create_namespaces(&self, config: &NamespaceConfig) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use nix::sched::{CloneFlags, unshare};
            
            let mut flags = CloneFlags::empty();
            
            if config.mnt {
                flags.insert(CloneFlags::CLONE_NEWNS);
            }
            if config.pid {
                flags.insert(CloneFlags::CLONE_NEWPID);
            }
            if config.net {
                flags.insert(CloneFlags::CLONE_NEWNET);
            }
            if config.ipc {
                flags.insert(CloneFlags::CLONE_NEWIPC);
            }
            if config.uts {
                flags.insert(CloneFlags::CLONE_NEWUTS);
            }
            if config.user {
                flags.insert(CloneFlags::CLONE_NEWUSER);
            }
            if config.cgroup {
                flags.insert(CloneFlags::CLONE_NEWCGROUP);
            }
            
            unshare(flags)?;
            
            // Update state
            let mut state = self.state.write().await;
            state.namespaces_created = true;
        }
        
        Ok(())
    }
    
    async fn setup_cgroups(&self, config: &CGroupConfig) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            use std::path::Path;
            
            // Create cgroup directory
            let cgroup_path = Path::new("/sys/fs/cgroup/blast").join(&self.id);
            fs::create_dir_all(&cgroup_path)?;
            
            // Set up controllers
            for controller in &config.controllers {
                let controller_path = cgroup_path.join(controller);
                fs::create_dir_all(&controller_path)?;
                
                // Write basic configs
                fs::write(
                    controller_path.join("tasks"),
                    std::process::id().to_string(),
                )?;
            }
            
            // Update state
            let mut state = self.state.write().await;
            state.cgroups_configured = true;
        }
        
        Ok(())
    }
    
    async fn configure_network(&self, policy: &NetworkPolicy) -> BlastResult<()> {
        let mut network = self.network.write().await;
        
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            
            // Create network namespace
            Command::new("ip")
                .args(&["netns", "add", &self.id])
                .status()?;
            
            // Configure interface
            if let Some(ref config) = policy.interface_config {
                // Create veth pair
                Command::new("ip")
                    .args(&["link", "add", &config.name, "type", "veth", "peer", "name", &format!("{}_h", config.name)])
                    .status()?;
                
                // Move interface to namespace
                Command::new("ip")
                    .args(&["link", "set", &config.name, "netns", &self.id])
                    .status()?;
                
                // Configure IP if specified
                if let Some(ref ip) = config.ip_address {
                    Command::new("ip")
                        .args(&["netns", "exec", &self.id, "ip", "addr", "add", ip, "dev", &config.name])
                        .status()?;
                }
                
                // Set interface up
                Command::new("ip")
                    .args(&["netns", "exec", &self.id, "ip", "link", "set", &config.name, "up"])
                    .status()?;
            }
            
            // Update state
            let mut state = self.state.write().await;
            state.network_configured = true;
        }
        
        Ok(())
    }
    
    async fn setup_filesystem(&self, policy: &FilesystemPolicy) -> BlastResult<()> {
        let mut fs = self.filesystem.write().await;
        
        // Create root directory
        std::fs::create_dir_all(&policy.root_dir)?;
        
        // Set up mount points
        for (mount_point, config) in &policy.mount_points {
            fs.mount(mount_point, config.clone()).await?;
        }
        
        // Create temporary directory
        std::fs::create_dir_all(&policy.tmp_dir)?;
        
        // Update state
        let mut state = self.state.write().await;
        state.filesystem_configured = true;
        
        Ok(())
    }
    
    async fn initialize(&self) -> BlastResult<()> {
        // Update state
        let mut state = self.state.write().await;
        state.initialized = true;
        
        Ok(())
    }
    
    async fn get_state(&self) -> BlastResult<ContainerState> {
        Ok(self.state.read().await.clone())
    }
    
    async fn cleanup(&self) -> BlastResult<()> {
        // Cleanup network namespace
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            Command::new("ip")
                .args(&["netns", "del", &self.id])
                .status()?;
        }
        
        // Cleanup mounts
        let fs = self.filesystem.read().await;
        for mount_point in fs.policy.mount_points.keys() {
            fs.unmount(mount_point).await?;
        }
        
        // Cleanup cgroups
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let cgroup_path = std::path::Path::new("/sys/fs/cgroup/blast").join(&self.id);
            if cgroup_path.exists() {
                fs::remove_dir_all(cgroup_path)?;
            }
        }
        
        // Update state
        let mut state = self.state.write().await;
        state.cleaned_up = true;
        
        Ok(())
    }
}

/// Container configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Root directory
    pub root_dir: PathBuf,
    /// Container name
    pub name: String,
    /// Container labels
    pub labels: HashMap<String, String>,
    /// Network policy
    pub network_policy: NetworkPolicy,
    /// Filesystem policy
    pub filesystem_policy: FilesystemPolicy,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            root_dir: PathBuf::from("/var/lib/blast/containers"),
            name: String::new(),
            labels: HashMap::new(),
            network_policy: NetworkPolicy::default(),
            filesystem_policy: FilesystemPolicy::default(),
        }
    }
}

/// Container state
#[derive(Debug, Clone, Default)]
pub struct ContainerState {
    /// Process ID
    pub pid: Option<u32>,
    /// Namespaces created
    pub namespaces_created: bool,
    /// CGroups configured
    pub cgroups_configured: bool,
    /// Network configured
    pub network_configured: bool,
    /// Filesystem configured
    pub filesystem_configured: bool,
    /// Container initialized
    pub initialized: bool,
    /// Container cleaned up
    pub cleaned_up: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_container_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = ContainerConfig {
            root_dir: temp_dir.path().to_path_buf(),
            name: "test-container".to_string(),
            labels: HashMap::new(),
            network_policy: NetworkPolicy::default(),
            filesystem_policy: FilesystemPolicy::default(),
        };

        let container = Container::new(&config).await.unwrap();
        assert!(!container.id.is_empty());
    }
} 