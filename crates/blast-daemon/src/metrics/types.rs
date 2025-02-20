use std::time::Duration;

/// Package-level metrics
#[derive(Debug, Clone)]
pub struct PackageMetrics {
    pub install_time: Duration,
    pub size_bytes: u64,
    pub dependencies: usize,
}

/// Environment-level metrics
#[derive(Debug, Clone)]
pub struct EnvironmentMetrics {
    pub total_packages: usize,
    pub disk_usage_bytes: u64,
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
}

/// Metrics manager for collecting and aggregating metrics
pub struct MetricsManager {
    package_metrics: Vec<PackageMetrics>,
    environment_metrics: Vec<EnvironmentMetrics>,
}

impl MetricsManager {
    pub fn new() -> Self {
        Self {
            package_metrics: Vec::new(),
            environment_metrics: Vec::new(),
        }
    }

    pub fn add_package_metrics(&mut self, metrics: PackageMetrics) {
        self.package_metrics.push(metrics);
    }

    pub fn add_environment_metrics(&mut self, metrics: EnvironmentMetrics) {
        self.environment_metrics.push(metrics);
    }
} 