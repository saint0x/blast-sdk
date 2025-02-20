use serde::{Deserialize, Serialize};
use super::{NetworkPolicy, FilesystemPolicy};

/// Isolation levels supported by the environment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Basic process isolation
    Process,
    /// Linux namespace isolation
    Namespace,
    /// Full container isolation
    Container,
    /// Lightweight VM (future)
    VM,
}

/// Configuration for Linux namespaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceConfig {
    /// Mount namespace
    pub mnt: bool,
    /// Process ID namespace
    pub pid: bool,
    /// Network namespace
    pub net: bool,
    /// IPC namespace
    pub ipc: bool,
    /// UTS namespace
    pub uts: bool,
    /// User namespace
    pub user: bool,
    /// CGroup namespace
    pub cgroup: bool,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            mnt: true,
            pid: true,
            net: true,
            ipc: true,
            uts: true,
            user: true,
            cgroup: true,
        }
    }
}

/// I/O limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoLimits {
    /// Read bandwidth limit in bytes per second
    pub read_bps_limit: Option<u64>,
    /// Write bandwidth limit in bytes per second
    pub write_bps_limit: Option<u64>,
}

/// CGroup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CGroupConfig {
    /// List of enabled controllers
    pub controllers: Vec<String>,
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
    /// Process limit
    pub process_limit: Option<u32>,
    /// CPU quota in microseconds
    pub cpu_quota: Option<u64>,
    /// CPU period in microseconds
    pub cpu_period: Option<u64>,
    /// CPU shares (relative weight)
    pub cpu_shares: Option<u64>,
    /// I/O weight (1-10000)
    pub io_weight: Option<u32>,
    /// I/O limits configuration
    pub io_limits: Option<IoLimits>,
}

impl Default for CGroupConfig {
    fn default() -> Self {
        Self {
            controllers: vec![
                "memory".to_string(),
                "cpu".to_string(),
                "io".to_string(),
                "pids".to_string(),
            ],
            memory_limit: None,
            process_limit: None,
            cpu_quota: None,
            cpu_period: None,
            cpu_shares: None,
            io_weight: None,
            io_limits: None,
        }
    }
}

/// Isolation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationConfig {
    /// Isolation level
    pub level: IsolationLevel,
    /// Namespace configuration
    pub namespace_config: NamespaceConfig,
    /// CGroup configuration
    pub cgroup_config: CGroupConfig,
    /// Network policy
    pub network_policy: NetworkPolicy,
    /// Filesystem policy
    pub filesystem_policy: FilesystemPolicy,
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            level: IsolationLevel::Process,
            namespace_config: NamespaceConfig::default(),
            cgroup_config: CGroupConfig::default(),
            network_policy: NetworkPolicy::default(),
            filesystem_policy: FilesystemPolicy::default(),
        }
    }
} 