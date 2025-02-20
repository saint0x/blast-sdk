#![allow(dead_code)]

mod config;
mod container;
mod network;
mod filesystem;

pub use config::*;
pub use container::*;
pub use network::*;
pub use filesystem::*;

use crate::error::BlastResult;

/// Enhanced isolation implementation
pub struct EnhancedIsolation {
    /// Isolation level
    level: IsolationLevel,
    /// Namespace configuration
    namespace_config: NamespaceConfig,
    /// CGroup configuration
    cgroup_config: CGroupConfig,
    /// Network policy
    network_policy: NetworkPolicy,
    /// Filesystem policy
    filesystem_policy: FilesystemPolicy,
}

impl EnhancedIsolation {
    /// Create new enhanced isolation
    pub async fn new(config: &IsolationConfig) -> BlastResult<Self> {
        Ok(Self {
            level: config.level,
            namespace_config: config.namespace_config.clone(),
            cgroup_config: config.cgroup_config.clone(),
            network_policy: config.network_policy.clone(),
            filesystem_policy: config.filesystem_policy.clone(),
        })
    }

    /// Get current isolation level
    pub fn level(&self) -> IsolationLevel {
        self.level
    }

    /// Get namespace configuration
    pub fn namespace_config(&self) -> &NamespaceConfig {
        &self.namespace_config
    }

    /// Get cgroup configuration
    pub fn cgroup_config(&self) -> &CGroupConfig {
        &self.cgroup_config
    }

    /// Get network policy
    pub fn network_policy(&self) -> &NetworkPolicy {
        &self.network_policy
    }

    /// Get filesystem policy
    pub fn filesystem_policy(&self) -> &FilesystemPolicy {
        &self.filesystem_policy
    }
} 