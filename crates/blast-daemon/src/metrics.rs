use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// Performance metrics for package operations
#[derive(Debug, Clone)]
pub struct PackageMetrics {
    /// Time taken for pip installation
    pub pip_install_duration: Duration,
    /// Time taken for environment synchronization
    pub sync_duration: Duration,
    /// Package name
    pub package_name: String,
    /// Package version
    pub package_version: String,
    /// Number of dependencies
    pub dependency_count: usize,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Memory usage during operation
    pub memory_usage: u64,
    /// Timestamp of operation
    pub timestamp: Instant,
}

/// Metrics for environment operations
#[derive(Debug, Clone)]
pub struct EnvironmentMetrics {
    /// Total number of packages
    pub total_packages: usize,
    /// Environment size
    pub env_size: u64,
    /// Cache size
    pub cache_size: u64,
    /// Average sync duration
    pub avg_sync_duration: Duration,
    /// Last update timestamp
    pub last_update: Instant,
}

/// Performance metrics manager
#[derive(Debug)]
pub struct MetricsManager {
    /// Package operation metrics
    package_metrics: Arc<RwLock<HashMap<Uuid, PackageMetrics>>>,
    /// Environment metrics
    environment_metrics: Arc<RwLock<HashMap<String, EnvironmentMetrics>>>,
    /// Rolling average window (in operations)
    metrics_window: usize,
}

impl MetricsManager {
    /// Create a new metrics manager
    pub fn new(metrics_window: usize) -> Self {
        Self {
            package_metrics: Arc::new(RwLock::new(HashMap::new())),
            environment_metrics: Arc::new(RwLock::new(HashMap::new())),
            metrics_window,
        }
    }

    /// Record package installation metrics
    pub async fn record_package_install(
        &self,
        operation_id: Uuid,
        package_name: String,
        package_version: String,
        pip_duration: Duration,
        sync_duration: Duration,
        dependency_count: usize,
        cache_hits: usize,
        memory_usage: u64,
    ) {
        let metrics = PackageMetrics {
            pip_install_duration: pip_duration,
            sync_duration,
            package_name,
            package_version,
            dependency_count,
            cache_hit_rate: if dependency_count > 0 {
                cache_hits as f32 / dependency_count as f32
            } else {
                0.0
            },
            memory_usage,
            timestamp: Instant::now(),
        };

        let mut package_metrics = self.package_metrics.write().await;
        package_metrics.insert(operation_id, metrics.clone());

        // Maintain rolling window
        if package_metrics.len() > self.metrics_window {
            let oldest = package_metrics.iter()
                .min_by_key(|(_, m)| m.timestamp)
                .map(|(k, _)| *k);
            if let Some(key) = oldest {
                package_metrics.remove(&key);
            }
        }

        info!(
            "Package install metrics - Package: {} v{}, Pip: {:?}, Sync: {:?}, Deps: {}, Cache hit rate: {:.2}",
            metrics.package_name,
            metrics.package_version,
            metrics.pip_install_duration,
            metrics.sync_duration,
            metrics.dependency_count,
            metrics.cache_hit_rate
        );
    }

    /// Update environment metrics
    pub async fn update_environment_metrics(
        &self,
        env_name: String,
        total_packages: usize,
        env_size: u64,
        cache_size: u64,
        sync_duration: Duration,
    ) {
        let mut env_metrics = self.environment_metrics.write().await;
        
        let metrics = env_metrics
            .entry(env_name.clone())
            .or_insert_with(|| EnvironmentMetrics {
                total_packages: 0,
                env_size: 0,
                cache_size: 0,
                avg_sync_duration: Duration::from_secs(0),
                last_update: Instant::now(),
            });

        // Update rolling average for sync duration
        metrics.avg_sync_duration = Duration::from_nanos(
            ((metrics.avg_sync_duration.as_nanos() as f64 * 0.9) +
             (sync_duration.as_nanos() as f64 * 0.1)) as u64
        );

        metrics.total_packages = total_packages;
        metrics.env_size = env_size;
        metrics.cache_size = cache_size;
        metrics.last_update = Instant::now();

        info!(
            "Environment metrics - Env: {}, Packages: {}, Size: {} MB, Cache: {} MB, Avg sync: {:?}",
            env_name,
            metrics.total_packages,
            metrics.env_size / 1_048_576,
            metrics.cache_size / 1_048_576,
            metrics.avg_sync_duration
        );
    }

    /// Get package metrics for an operation
    pub async fn get_package_metrics(&self, operation_id: &Uuid) -> Option<PackageMetrics> {
        self.package_metrics.read().await.get(operation_id).cloned()
    }

    /// Get environment metrics
    pub async fn get_environment_metrics(&self, env_name: &str) -> Option<EnvironmentMetrics> {
        self.environment_metrics.read().await.get(env_name).cloned()
    }

    /// Get average package installation times
    pub async fn get_average_install_times(&self) -> (Duration, Duration) {
        let metrics = self.package_metrics.read().await;
        
        if metrics.is_empty() {
            return (Duration::from_secs(0), Duration::from_secs(0));
        }

        let total: (Duration, Duration) = metrics.values()
            .fold((Duration::from_secs(0), Duration::from_secs(0)), |acc, m| {
                (acc.0 + m.pip_install_duration, acc.1 + m.sync_duration)
            });

        let count = metrics.len() as u32;
        (
            Duration::from_nanos(total.0.as_nanos() as u64 / count as u64),
            Duration::from_nanos(total.1.as_nanos() as u64 / count as u64)
        )
    }

    /// Get overall cache hit rate
    pub async fn get_cache_hit_rate(&self) -> f32 {
        let metrics = self.package_metrics.read().await;
        let total_hits: usize = metrics.values()
            .map(|m| (m.cache_hit_rate * m.dependency_count as f32) as usize)
            .sum();
        let total_deps: usize = metrics.values()
            .map(|m| m.dependency_count)
            .sum();
        
        if total_deps > 0 {
            total_hits as f32 / total_deps as f32
        } else {
            0.0
        }
    }
} 