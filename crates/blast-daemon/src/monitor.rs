//! Environment monitoring functionality for Python environments

use std::path::PathBuf;
use std::time::{Instant, Duration};
use walkdir::WalkDir;

/// Monitor events for Python environment changes
#[derive(Debug)]
pub enum MonitorEvent {
    /// Resource usage check
    ResourceCheck,
    /// Package change detected
    PackageChanged,
    /// Stop monitoring for environment
    StopMonitoring {
        env_path: PathBuf,
    },
    /// Python file change event
    FileChanged(PathBuf),
    /// Environment resource usage update
    ResourceUpdate(EnvironmentUsage),
}

/// Environment resource usage information
#[derive(Debug, Clone)]
pub struct EnvironmentUsage {
    /// Environment disk usage in bytes
    pub env_disk_usage: EnvDiskUsage,
    /// Package cache usage
    pub cache_usage: CacheUsage,
    /// Timestamp of the measurement
    pub timestamp: Instant,
}

/// Environment disk usage information
#[derive(Debug, Clone, Default)]
pub struct EnvDiskUsage {
    /// Total size of the environment in bytes
    pub total_size: u64,
    /// Size of installed packages in bytes
    pub packages_size: u64,
    /// Size of Python standard library in bytes
    pub stdlib_size: u64,
}

/// Package cache usage information
#[derive(Debug, Clone, Default)]
pub struct CacheUsage {
    /// Total size of cached packages in bytes
    pub total_size: u64,
    /// Number of cached packages
    pub package_count: usize,
    /// Cache location
    pub cache_path: PathBuf,
}

/// Resource limits for Python environment
#[derive(Debug, Clone)]
pub struct PythonResourceLimits {
    /// Maximum environment size in bytes
    pub max_env_size: u64,
    /// Maximum cache size in bytes
    pub max_cache_size: u64,
}

impl Default for PythonResourceLimits {
    fn default() -> Self {
        Self {
            max_env_size: 1024 * 1024 * 1024 * 5, // 5GB
            max_cache_size: 1024 * 1024 * 1024 * 2, // 2GB
        }
    }
}

/// Monitor for Python environment resources
#[derive(Debug, Clone)]
pub struct PythonResourceMonitor {
    /// Resource limits
    limits: PythonResourceLimits,
    /// Environment path
    env_path: PathBuf,
    /// Cache path
    cache_path: PathBuf,
    /// Last update time
    last_update: Instant,
    /// Last environment usage
    last_env_usage: EnvDiskUsage,
    /// Last cache usage
    last_cache_usage: CacheUsage,
}

impl PythonResourceMonitor {
    /// Create a new Python resource monitor
    pub fn new(env_path: PathBuf, cache_path: PathBuf, limits: PythonResourceLimits) -> Self {
        Self {
            limits,
            env_path,
            cache_path,
            last_update: Instant::now(),
            last_env_usage: EnvDiskUsage::default(),
            last_cache_usage: CacheUsage::default(),
        }
    }

    /// Check if it's time for a resource update
    pub fn should_update(&self) -> bool {
        self.last_update.elapsed() >= Duration::from_secs(60)
    }

    /// Get current environment usage with optimized calculations
    pub fn get_current_usage(&mut self) -> EnvironmentUsage {
        // Only update if enough time has passed
        if !self.should_update() {
            return EnvironmentUsage {
                env_disk_usage: self.last_env_usage.clone(),
                cache_usage: self.last_cache_usage.clone(),
                timestamp: self.last_update,
            };
        }

        let usage = EnvironmentUsage {
            env_disk_usage: self.calculate_env_disk_usage(),
            cache_usage: self.calculate_cache_usage(),
            timestamp: Instant::now(),
        };

        // Cache the results
        self.last_env_usage = usage.env_disk_usage.clone();
        self.last_cache_usage = usage.cache_usage.clone();
        self.last_update = usage.timestamp;

        usage
    }

    /// Calculate environment disk usage with optimizations
    fn calculate_env_disk_usage(&self) -> EnvDiskUsage {
        let mut usage = EnvDiskUsage::default();
        
        // Calculate site-packages size
        let site_packages = self.env_path.join("lib").join("python3").join("site-packages");
        if site_packages.exists() {
            usage.packages_size = self.calculate_directory_size(&site_packages);
        }
        
        // Calculate stdlib size
        let stdlib = self.env_path.join("lib").join("python3");
        if stdlib.exists() {
            usage.stdlib_size = self.calculate_directory_size(&stdlib);
        }
        
        usage.total_size = usage.packages_size + usage.stdlib_size;
        usage
    }

    /// Calculate cache usage with optimizations
    fn calculate_cache_usage(&self) -> CacheUsage {
        let mut usage = CacheUsage::default();
        usage.cache_path = self.cache_path.clone();

        if !self.cache_path.exists() {
            return usage;
        }

        // Use walkdir for more efficient directory traversal
        for entry in WalkDir::new(&self.cache_path)
            .min_depth(1)
            .max_depth(2)  // Only go one level deep for package counting
            .into_iter()
            .filter_entry(|e| !self.should_skip_path(e.path()))
            .filter_map(|e| e.ok())
        {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.is_file() {
                usage.total_size += metadata.len();
            }

            // Count top-level directories as packages
            if metadata.is_dir() && entry.depth() == 1 {
                usage.package_count += 1;
            }
        }

        usage
    }

    /// Check if a path should be skipped during traversal
    fn should_skip_path(&self, path: &std::path::Path) -> bool {
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return true,
        };

        // Skip common unnecessary directories and files
        file_name.starts_with('.') || 
        file_name == "__pycache__" ||
        file_name == "*.pyc" ||
        file_name == "*.pyo" ||
        file_name == "*.pyd"
    }

    /// Calculate directory size efficiently
    fn calculate_directory_size(&self, path: &PathBuf) -> u64 {
        WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| !self.should_skip_path(e.path()))
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .filter(|m| m.is_file())
            .map(|m| m.len())
            .sum()
    }

    /// Check resource limits
    pub fn check_limits(&mut self) -> bool {
        let usage = self.get_current_usage();
        
        usage.env_disk_usage.total_size <= self.limits.max_env_size &&
        usage.cache_usage.total_size <= self.limits.max_cache_size
    }

    /// Update resource limits
    pub fn update_limits(&mut self, new_limits: PythonResourceLimits) {
        self.limits = new_limits;
    }

    /// Get current resource limits
    pub fn get_limits(&self) -> &PythonResourceLimits {
        &self.limits
    }
} 