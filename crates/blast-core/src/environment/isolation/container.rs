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
    ConnectionInfo, Protocol, ConnectionState,
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
    /// Get container runtime state
    pub fn runtime_state(&self) -> Arc<RwLock<ContainerState>> {
        Arc::clone(&self.state)
    }

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

    /// Get current resource usage
    pub async fn get_resource_usage(&self) -> BlastResult<ResourceUsage> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            use std::path::Path;
            
            let cgroup_path = Path::new("/sys/fs/cgroup/blast").join(&self.id);
            
            // Read memory stats
            let memory_current = fs::read_to_string(
                cgroup_path.join("memory").join("memory.current")
            )?.parse::<u64>()?;
            
            let memory_peak = fs::read_to_string(
                cgroup_path.join("memory").join("memory.peak")
            )?.parse::<u64>()?;
            
            // Read CPU stats
            let cpu_usage = fs::read_to_string(
                cgroup_path.join("cpu").join("cpu.stat")
            )?;
            let cpu_stats: HashMap<String, u64> = cpu_usage
                .lines()
                .filter_map(|line| {
                    let mut parts = line.split_whitespace();
                    match (parts.next(), parts.next()) {
                        (Some(key), Some(value)) => {
                            value.parse().ok().map(|v| (key.to_string(), v))
                        }
                        _ => None
                    }
                })
                .collect();
            
            // Read I/O stats
            let io_stats = fs::read_to_string(
                cgroup_path.join("io").join("io.stat")
            )?;
            let io_usage: HashMap<String, u64> = io_stats
                .lines()
                .filter_map(|line| {
                    let mut parts = line.split_whitespace();
                    Some((
                        parts.next()?.to_string(),
                        parts.next()?.parse().ok()?
                    ))
                })
                .collect();
            
            // Read process count
            let pids_current = fs::read_to_string(
                cgroup_path.join("pids").join("pids.current")
            )?.parse::<u32>()?;
            
            Ok(ResourceUsage {
                memory_current,
                memory_peak,
                cpu_usage: cpu_stats,
                io_usage,
                process_count: pids_current,
                timestamp: std::time::SystemTime::now(),
            })
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Ok(ResourceUsage::default())
        }
    }
}

#[async_trait]
impl ContainerRuntime for Container {
    async fn create_namespaces(&self, _config: &NamespaceConfig) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use nix::sched::{CloneFlags, unshare};
            
            let mut flags = CloneFlags::empty();
            
            if _config.mnt {
                flags.insert(CloneFlags::CLONE_NEWNS);
            }
            if _config.pid {
                flags.insert(CloneFlags::CLONE_NEWPID);
            }
            if _config.net {
                flags.insert(CloneFlags::CLONE_NEWNET);
            }
            if _config.ipc {
                flags.insert(CloneFlags::CLONE_NEWIPC);
            }
            if _config.uts {
                flags.insert(CloneFlags::CLONE_NEWUTS);
            }
            if _config.user {
                flags.insert(CloneFlags::CLONE_NEWUSER);
            }
            if _config.cgroup {
                flags.insert(CloneFlags::CLONE_NEWCGROUP);
            }
            
            unshare(flags)?;
            
            // Update state
            let mut state = self.state.write().await;
            state.namespaces_created = true;
        }
        
        Ok(())
    }
    
    async fn setup_cgroups(&self, _config: &CGroupConfig) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            use std::path::Path;
            
            // Create cgroup directory
            let cgroup_path = Path::new("/sys/fs/cgroup/blast").join(&self.id);
            fs::create_dir_all(&cgroup_path)?;
            
            // Set up controllers
            for controller in &_config.controllers {
                let controller_path = cgroup_path.join(controller);
                fs::create_dir_all(&controller_path)?;
                
                // Write basic configs
                fs::write(
                    controller_path.join("tasks"),
                    std::process::id().to_string(),
                )?;

                // Apply specific controller limits
                match controller.as_str() {
                    "memory" => {
                        // Default to 512MB if no limit specified
                        let memory_limit = _config.memory_limit
                            .unwrap_or(512 * 1024 * 1024); // 512MB in bytes
                        fs::write(
                            controller_path.join("memory.limit_in_bytes"),
                            memory_limit.to_string(),
                        )?;
                        
                        // Configure memory swappiness
                        fs::write(
                            controller_path.join("memory.swappiness"),
                            "0", // Disable swapping by default
                        )?;
                        
                        // Set memory soft limit (90% of hard limit)
                        let soft_limit = ((memory_limit as f64) * 0.9) as u64;
                        fs::write(
                            controller_path.join("memory.soft_limit_in_bytes"),
                            soft_limit.to_string(),
                        )?;
                    }
                    "cpu" => {
                        // Configure CPU quota (default to 100% of one CPU core)
                        let cpu_quota = _config.cpu_quota.unwrap_or(100_000);
                        let cpu_period = _config.cpu_period.unwrap_or(100_000);
                        
                        fs::write(
                            controller_path.join("cpu.cfs_quota_us"),
                            cpu_quota.to_string(),
                        )?;
                        
                        fs::write(
                            controller_path.join("cpu.cfs_period_us"),
                            cpu_period.to_string(),
                        )?;
                        
                        // Set CPU shares for fair scheduling
                        let cpu_shares = _config.cpu_shares.unwrap_or(1024);
                        fs::write(
                            controller_path.join("cpu.shares"),
                            cpu_shares.to_string(),
                        )?;
                    }
                    "io" => {
                        // Configure I/O weight (default to 100)
                        let io_weight = _config.io_weight.unwrap_or(100);
                        fs::write(
                            controller_path.join("io.weight"),
                            io_weight.to_string(),
                        )?;
                        
                        // Set I/O limits if specified
                        if let Some(limits) = &_config.io_limits {
                            // Read limits
                            if let Some(bps) = limits.read_bps_limit {
                                fs::write(
                                    controller_path.join("io.max"),
                                    format!("rbps={}", bps),
                                )?;
                            }
                            // Write limits
                            if let Some(bps) = limits.write_bps_limit {
                                fs::write(
                                    controller_path.join("io.max"),
                                    format!("wbps={}", bps),
                                )?;
                            }
                        }
                    }
                    "pids" => {
                        // Set process limit (default or configured)
                        let pids_limit = _config.process_limit.unwrap_or(100);
                        fs::write(
                            controller_path.join("pids.max"),
                            pids_limit.to_string(),
                        )?;
                    }
                    _ => {}
                }
            }
            
            // Update state
            let mut state = self.state.write().await;
            state.cgroups_configured = true;
            
            tracing::info!("CGroups configured for container {} with controllers: {:?}", 
                self.id, _config.controllers);
        }
        
        Ok(())
    }
    
    async fn configure_network(&self, policy: &NetworkPolicy) -> BlastResult<()> {
        // Validate network policy
        if !policy.allow_outbound && !policy.allow_inbound {
            tracing::info!("Network access completely disabled for container {}", self.id);
        }

        // Get network state lock and initialize
        let network = self.network.write().await;
        
        // Track initial network state
        network.track_connection(ConnectionInfo {
            source: format!("container:{}", self.id),
            destination: "host".to_string(),
            protocol: Protocol::TCP,
            state: ConnectionState::New,
            bytes_sent: 0,
            bytes_received: 0,
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
        }).await?;
        
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            
            // Create network namespace with error handling
            if let Err(e) = Command::new("ip")
                .args(&["netns", "add", &self.id])
                .status()
            {
                tracing::error!("Failed to create network namespace: {}", e);
                return Err(crate::error::BlastError::runtime(format!(
                    "Failed to create network namespace: {}", e
                )));
            }
            
            // Configure interface if specified
            if let Some(ref config) = policy.interface_config {
                // Create veth pair with error handling
                if let Err(e) = Command::new("ip")
                    .args(&["link", "add", &config.name, "type", "veth", "peer", 
                           "name", &format!("{}_h", config.name)])
                    .status()
                {
                    // Cleanup on failure
                    let _ = Command::new("ip").args(&["netns", "del", &self.id]).status();
                    return Err(crate::error::BlastError::runtime(format!(
                        "Failed to create veth pair: {}", e
                    )));
                }
                
                // Move interface to namespace
                if let Err(e) = Command::new("ip")
                    .args(&["link", "set", &config.name, "netns", &self.id])
                    .status()
                {
                    // Cleanup on failure
                    let _ = Command::new("ip").args(&["link", "del", &config.name]).status();
                    let _ = Command::new("ip").args(&["netns", "del", &self.id]).status();
                    return Err(crate::error::BlastError::runtime(format!(
                        "Failed to move interface to namespace: {}", e
                    )));
                }
                
                // Configure IP if specified
                if let Some(ref ip) = config.ip_address {
                    if let Err(e) = Command::new("ip")
                        .args(&["netns", "exec", &self.id, "ip", "addr", "add", ip, "dev", &config.name])
                        .status()
                    {
                        // Cleanup on failure
                        let _ = Command::new("ip").args(&["netns", "del", &self.id]).status();
                        return Err(crate::error::BlastError::runtime(format!(
                            "Failed to configure IP address: {}", e
                        )));
                    }
                }
                
                // Set interface up
                if let Err(e) = Command::new("ip")
                    .args(&["netns", "exec", &self.id, "ip", "link", "set", &config.name, "up"])
                    .status()
                {
                    // Cleanup on failure
                    let _ = Command::new("ip").args(&["netns", "del", &self.id]).status();
                    return Err(crate::error::BlastError::runtime(format!(
                        "Failed to bring up interface: {}", e
                    )));
                }

                // Configure bandwidth limits if specified
                if let Some(limit) = policy.bandwidth_limit {
                    if let Err(e) = Command::new("tc")
                        .args(&["qdisc", "add", "dev", &config.name, "root", "tbf", 
                               "rate", &format!("{}bit", limit * 8),
                               "latency", "50ms", "burst", "1540"])
                        .status()
                    {
                        tracing::warn!("Failed to set bandwidth limit: {}", e);
                    }
                }

                // Update bandwidth tracking
                network.update_bandwidth(0, 0).await?;
            }
            
            // Update state
            let mut state = self.state.write().await;
            state.network_configured = true;
            
            tracing::info!("Network configured for container {} with policy: {:?}", self.id, policy);
        }
        
        Ok(())
    }
    
    async fn setup_filesystem(&self, policy: &FilesystemPolicy) -> BlastResult<()> {
        let fs = self.filesystem.write().await;
        
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
        let mount_points = fs.get_mount_points().await?;
        for mount_point in mount_points {
            fs.unmount(&mount_point).await?;
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

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// Current memory usage in bytes
    pub memory_current: u64,
    /// Peak memory usage in bytes
    pub memory_peak: u64,
    /// CPU usage statistics
    pub cpu_usage: HashMap<String, u64>,
    /// I/O usage statistics
    pub io_usage: HashMap<String, u64>,
    /// Current process count
    pub process_count: u32,
    /// Timestamp of the measurement
    pub timestamp: std::time::SystemTime,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            memory_current: 0,
            memory_peak: 0,
            cpu_usage: HashMap::new(),
            io_usage: HashMap::new(),
            process_count: 0,
            timestamp: std::time::SystemTime::now(),
        }
    }
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