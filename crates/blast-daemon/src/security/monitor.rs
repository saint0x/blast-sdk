use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use sysinfo::{System, SystemExt, ProcessExt};
use blast_core::{
    error::{BlastError, BlastResult},
    security::{ResourceLimits, ResourceUsage},
};

/// Resource monitor for tracking environment resource usage
pub struct ResourceMonitor {
    /// System information
    sys: Arc<Mutex<System>>,
    /// Process ID to monitor
    pid: u32,
    /// Resource limits
    limits: ResourceLimits,
    /// Usage history
    history: Arc<Mutex<Vec<ResourceUsage>>>,
    /// Start time
    start_time: Instant,
}

impl ResourceMonitor {
    /// Create new resource monitor
    pub fn new(pid: u32, limits: ResourceLimits) -> Self {
        Self {
            sys: Arc::new(Mutex::new(System::new_all())),
            pid,
            limits,
            history: Arc::new(Mutex::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    /// Get current resource usage
    pub async fn get_usage(&self) -> BlastResult<ResourceUsage> {
        let mut sys = self.sys.lock().await;
        sys.refresh_all();

        let process = sys.process(self.pid as i32)
            .ok_or_else(|| BlastError::ProcessNotFound(self.pid))?;

        let usage = ResourceUsage {
            timestamp: Instant::now(),
            memory_bytes: process.memory() * 1024, // KB to bytes
            cpu_percent: process.cpu_usage(),
            disk_bytes_read: process.disk_usage().read_bytes,
            disk_bytes_written: process.disk_usage().written_bytes,
            network_bytes_rx: 0, // TODO: Implement network monitoring
            network_bytes_tx: 0,
        };

        // Store in history
        self.history.lock().await.push(usage.clone());

        Ok(usage)
    }

    /// Check if resource usage exceeds limits
    pub async fn check_limits(&self) -> BlastResult<bool> {
        let usage = self.get_usage().await?;
        
        // Check memory limit
        if let Some(limit) = self.limits.memory_bytes {
            if usage.memory_bytes > limit {
                return Ok(false);
            }
        }

        // Check CPU limit
        if let Some(limit) = self.limits.cpu_percent {
            if usage.cpu_percent > limit {
                return Ok(false);
            }
        }

        // Check disk read limit
        if let Some(limit) = self.limits.disk_bytes_read {
            if usage.disk_bytes_read > limit {
                return Ok(false);
            }
        }

        // Check disk write limit
        if let Some(limit) = self.limits.disk_bytes_written {
            if usage.disk_bytes_written > limit {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Get resource usage history
    pub async fn get_history(&self) -> BlastResult<Vec<ResourceUsage>> {
        Ok(self.history.lock().await.clone())
    }

    /// Get average resource usage over time window
    pub async fn get_average_usage(&self, window: Duration) -> BlastResult<ResourceUsage> {
        let history = self.history.lock().await;
        let now = Instant::now();
        
        // Filter entries within time window
        let recent: Vec<_> = history.iter()
            .filter(|usage| now.duration_since(usage.timestamp) <= window)
            .collect();

        if recent.is_empty() {
            return Err(BlastError::NoDataAvailable);
        }

        // Calculate averages
        let avg_memory = recent.iter().map(|u| u.memory_bytes).sum::<u64>() / recent.len() as u64;
        let avg_cpu = recent.iter().map(|u| u.cpu_percent).sum::<f32>() / recent.len() as f32;
        let avg_disk_read = recent.iter().map(|u| u.disk_bytes_read).sum::<u64>() / recent.len() as u64;
        let avg_disk_write = recent.iter().map(|u| u.disk_bytes_written).sum::<u64>() / recent.len() as u64;

        Ok(ResourceUsage {
            timestamp: now,
            memory_bytes: avg_memory,
            cpu_percent: avg_cpu,
            disk_bytes_read: avg_disk_read,
            disk_bytes_written: avg_disk_write,
            network_bytes_rx: 0,
            network_bytes_tx: 0,
        })
    }

    /// Start background monitoring task
    pub async fn start_monitoring(&self, interval: Duration) -> BlastResult<()> {
        let sys = self.sys.clone();
        let history = self.history.clone();
        let pid = self.pid;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                interval.tick().await;
                
                let mut sys = sys.lock().await;
                sys.refresh_all();

                if let Some(process) = sys.process(pid as i32) {
                    let usage = ResourceUsage {
                        timestamp: Instant::now(),
                        memory_bytes: process.memory() * 1024,
                        cpu_percent: process.cpu_usage(),
                        disk_bytes_read: process.disk_usage().read_bytes,
                        disk_bytes_written: process.disk_usage().written_bytes,
                        network_bytes_rx: 0,
                        network_bytes_tx: 0,
                    };

                    history.lock().await.push(usage);
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[tokio::test]
    async fn test_resource_monitoring() {
        // Start a test process
        let output = Command::new("sleep")
            .arg("10")
            .spawn()
            .unwrap();
        
        let pid = output.id();

        let limits = ResourceLimits {
            memory_bytes: Some(1024 * 1024 * 100), // 100MB
            cpu_percent: Some(50.0),
            disk_bytes_read: Some(1024 * 1024), // 1MB
            disk_bytes_written: Some(1024 * 1024),
            ..Default::default()
        };

        let monitor = ResourceMonitor::new(pid, limits);

        // Test usage monitoring
        let usage = monitor.get_usage().await.unwrap();
        assert!(usage.memory_bytes > 0);
        assert!(usage.cpu_percent >= 0.0);

        // Test limit checking
        assert!(monitor.check_limits().await.unwrap());

        // Test history
        monitor.start_monitoring(Duration::from_secs(1)).await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        let history = monitor.get_history().await.unwrap();
        assert!(!history.is_empty());

        // Test average usage
        let avg = monitor.get_average_usage(Duration::from_secs(5)).await.unwrap();
        assert!(avg.memory_bytes > 0);
    }
} 