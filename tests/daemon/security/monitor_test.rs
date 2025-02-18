use std::{process::Command, time::Duration, Instant};
use blast_core::security::ResourceLimits;
use blast_daemon::security::monitor::{ResourceMonitor, ResourceUsage};

mod basic_monitoring {
    use super::*;

    #[tokio::test]
    async fn test_monitor_creation() {
        let limits = ResourceLimits::default();
        let monitor = ResourceMonitor::new(std::process::id(), limits);
        assert!(monitor.get_usage().await.is_ok());
    }

    #[tokio::test]
    async fn test_resource_usage_tracking() {
        let limits = ResourceLimits::default();
        let monitor = ResourceMonitor::new(std::process::id(), limits);
        
        let usage = monitor.get_usage().await.unwrap();
        assert!(usage.memory_bytes > 0);
        assert!(usage.cpu_percent >= 0.0);
        assert!(usage.disk_bytes_read >= 0);
        assert!(usage.disk_bytes_written >= 0);
    }
}

mod resource_limits {
    use super::*;

    #[tokio::test]
    async fn test_memory_limit_check() {
        let mut limits = ResourceLimits::default();
        limits.memory_bytes = Some(1); // Set unreasonably low limit
        let monitor = ResourceMonitor::new(std::process::id(), limits);
        
        assert!(!monitor.check_limits().await.unwrap());
    }

    #[tokio::test]
    async fn test_cpu_limit_check() {
        let mut limits = ResourceLimits::default();
        limits.cpu_percent = Some(0.1); // Set very low CPU limit
        let monitor = ResourceMonitor::new(std::process::id(), limits);
        
        // Force some CPU usage
        for _ in 0..1000000 { let _ = 2.0_f64.sqrt(); }
        
        let result = monitor.check_limits().await.unwrap();
        assert!(!result);
    }
}

mod history_tracking {
    use super::*;

    #[tokio::test]
    async fn test_history_recording() {
        let limits = ResourceLimits::default();
        let monitor = ResourceMonitor::new(std::process::id(), limits);
        
        // Get usage multiple times
        for _ in 0..3 {
            monitor.get_usage().await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        let history = monitor.get_history().await.unwrap();
        assert!(history.len() >= 3);
    }

    #[tokio::test]
    async fn test_average_usage_calculation() {
        let limits = ResourceLimits::default();
        let monitor = ResourceMonitor::new(std::process::id(), limits);
        
        // Record some usage data
        for _ in 0..5 {
            monitor.get_usage().await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        let avg = monitor.get_average_usage(Duration::from_secs(1)).await.unwrap();
        assert!(avg.memory_bytes > 0);
        assert!(avg.cpu_percent >= 0.0);
    }
}

mod continuous_monitoring {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_task() {
        let limits = ResourceLimits::default();
        let monitor = ResourceMonitor::new(std::process::id(), limits);
        
        // Start monitoring
        monitor.start_monitoring(Duration::from_millis(100)).await.unwrap();
        
        // Wait for some data to be collected
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        let history = monitor.get_history().await.unwrap();
        assert!(history.len() >= 4); // Should have at least 4 data points
    }
}

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

#[tokio::test]
async fn test_limit_violations() {
    // Start a memory-intensive process
    let output = Command::new("python")
        .arg("-c")
        .arg(r#"
import numpy as np
arr = np.zeros((1000, 1000))  # Allocate ~8MB
while True:
    pass  # CPU intensive
"#)
        .spawn()
        .unwrap();
    
    let pid = output.id();

    // Set very restrictive limits
    let limits = ResourceLimits {
        memory_bytes: Some(1024 * 1024), // 1MB (should be exceeded)
        cpu_percent: Some(10.0),         // 10% CPU (should be exceeded)
        ..Default::default()
    };

    let monitor = ResourceMonitor::new(pid, limits);
    
    // Wait for resource usage to build up
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Check limits - should be violated
    assert!(!monitor.check_limits().await.unwrap());
    
    // Verify specific violations
    let usage = monitor.get_usage().await.unwrap();
    assert!(usage.memory_bytes > limits.memory_bytes.unwrap());
    assert!(usage.cpu_percent > limits.cpu_percent.unwrap());
}

#[tokio::test]
async fn test_monitoring_intervals() {
    let output = Command::new("sleep")
        .arg("5")
        .spawn()
        .unwrap();
    
    let pid = output.id();
    let monitor = ResourceMonitor::new(pid, ResourceLimits::default());

    // Start monitoring with short interval
    monitor.start_monitoring(Duration::from_millis(100)).await.unwrap();
    
    // Wait for multiple intervals
    tokio::time::sleep(Duration::from_millis(350)).await;
    
    // Should have approximately 3-4 history entries
    let history = monitor.get_history().await.unwrap();
    assert!(history.len() >= 3);
}

#[tokio::test]
async fn test_process_termination() {
    let output = Command::new("sleep")
        .arg("1")
        .spawn()
        .unwrap();
    
    let pid = output.id();
    let monitor = ResourceMonitor::new(pid, ResourceLimits::default());

    // Initial monitoring should work
    assert!(monitor.get_usage().await.is_ok());

    // Wait for process to finish
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Monitoring should fail after process ends
    assert!(monitor.get_usage().await.is_err());
}

#[tokio::test]
async fn test_resource_usage_aggregation() {
    let output = Command::new("python")
        .arg("-c")
        .arg(r#"
import time
for _ in range(5):
    time.sleep(0.1)
"#)
        .spawn()
        .unwrap();
    
    let pid = output.id();
    let monitor = ResourceMonitor::new(pid, ResourceLimits::default());

    // Start monitoring
    monitor.start_monitoring(Duration::from_millis(100)).await.unwrap();
    
    // Wait for process to finish
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Get history and check aggregations
    let history = monitor.get_history().await.unwrap();
    
    // Calculate our own averages
    let avg_memory: u64 = history.iter().map(|u| u.memory_bytes).sum::<u64>() / history.len() as u64;
    let avg_cpu: f64 = history.iter().map(|u| u.cpu_percent).sum::<f64>() / history.len() as f64;

    // Compare with monitor's calculations
    let monitor_avg = monitor.get_average_usage(Duration::from_secs(1)).await.unwrap();
    assert!((monitor_avg.memory_bytes as i64 - avg_memory as i64).abs() < 1024 * 1024); // Within 1MB
    assert!((monitor_avg.cpu_percent - avg_cpu).abs() < 1.0); // Within 1%
} 