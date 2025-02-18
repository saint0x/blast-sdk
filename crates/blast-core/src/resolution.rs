use serde::{Deserialize, Serialize};

/// Metrics from dependency resolution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolutionMetrics {
    /// Number of packages resolved
    pub package_count: usize,
    /// Time taken for resolution (ms)
    pub resolution_time_ms: u64,
    /// Number of network requests
    pub network_requests: usize,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of version conflicts resolved
    pub conflicts_resolved: usize,
} 