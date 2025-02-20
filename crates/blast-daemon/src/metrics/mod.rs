mod types;
pub use types::*;

use std::time::{Duration, Instant};

/// Performance metrics snapshot
#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    /// Average pip install time
    pub avg_pip_install_time: Duration,
    /// Average sync time
    pub avg_sync_time: Duration,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Timestamp of snapshot
    pub timestamp: Instant,
} 