use std::path::PathBuf;
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

/// CGroup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CGroupConfig {
    /// CGroup version (v1 or v2)
    pub version: u8,
    /// CGroup path
    pub path: PathBuf,
    /// Controller configuration
    pub controllers: Vec<String>,
}

impl Default for CGroupConfig {
    fn default() -> Self {
        Self {
            version: 2,
            path: PathBuf::from("/sys/fs/cgroup/blast"),
            controllers: vec![
                "cpu".to_string(),
                "memory".to_string(),
                "io".to_string(),
                "pids".to_string(),
            ],
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