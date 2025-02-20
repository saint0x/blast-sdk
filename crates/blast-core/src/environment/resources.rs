use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::BlastResult;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU usage limit (percentage)
    pub cpu_limit: f64,
    /// Memory usage limit (bytes)
    pub memory_limit: u64,
    /// Disk space limit (bytes)
    pub disk_limit: u64,
    /// I/O operations per second limit
    pub iops_limit: u32,
    /// I/O bandwidth limit (bytes/sec)
    pub io_bandwidth_limit: u64,
    /// Process count limit
    pub process_limit: u32,
    /// Thread count limit
    pub thread_limit: u32,
    /// File descriptor limit
    pub fd_limit: u32,
}

/// Resource usage tracking
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Disk usage in bytes
    pub disk_usage: u64,
    /// I/O statistics
    pub io_stats: IOStats,
    /// Process count
    pub process_count: u32,
    /// Thread count
    pub thread_count: u32,
    /// Open file descriptors
    pub fd_count: u32,
    /// Last update timestamp
    pub last_update: SystemTime,
}

/// I/O statistics
#[derive(Debug, Clone, Default)]
pub struct IOStats {
    /// Read operations count
    pub read_ops: u64,
    /// Write operations count
    pub write_ops: u64,
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Current read bandwidth (bytes/sec)
    pub read_bandwidth: f64,
    /// Current write bandwidth (bytes/sec)
    pub write_bandwidth: f64,
    /// I/O wait time
    pub io_wait: Duration,
}

/// Resource manager for monitoring and controlling resource usage
pub struct ResourceManager {
    /// Resource limits
    limits: ResourceLimits,
    /// Current usage
    usage: Arc<RwLock<ResourceUsage>>,
    /// Update interval
    update_interval: Duration,
}

impl ResourceManager {
    /// Create new resource manager
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits: limits.clone(),
            usage: Arc::new(RwLock::new(ResourceUsage {
                cpu_usage: 0.0,
                memory_usage: 0,
                disk_usage: 0,
                io_stats: IOStats::default(),
                process_count: 0,
                thread_count: 0,
                fd_count: 0,
                last_update: SystemTime::now(),
            })),
            update_interval: Duration::from_secs(1),
        }
    }

    /// Start resource monitoring
    pub async fn start_monitoring(&self) -> BlastResult<()> {
        let usage = Arc::clone(&self.usage);
        let limits = self.limits.clone();
        let interval = self.update_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                if let Err(e) = Self::update_usage(&usage, &limits).await {
                    tracing::error!("Failed to update resource usage: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Update resource usage
    async fn update_usage(usage: &Arc<RwLock<ResourceUsage>>, limits: &ResourceLimits) -> BlastResult<()> {
        let mut current = usage.write().await;
        let now = SystemTime::now();
        
        // Update CPU usage
        if let Ok(cpu) = Self::get_cpu_usage().await {
            current.cpu_usage = cpu;
            // Enforce CPU limit
            if cpu > limits.cpu_limit {
                Self::throttle_cpu().await?;
            }
        }
        
        // Update memory usage
        if let Ok(mem) = Self::get_memory_usage().await {
            current.memory_usage = mem;
            // Enforce memory limit
            if mem > limits.memory_limit {
                Self::limit_memory(limits.memory_limit).await?;
            }
        }
        
        // Update I/O stats
        if let Ok(io) = Self::get_io_stats().await {
            let elapsed = now.duration_since(current.last_update)
                .unwrap_or(Duration::from_secs(1));
            
            // Calculate I/O bandwidth
            if elapsed.as_secs_f64() > 0.0 {
                io.read_bandwidth = (io.bytes_read - current.io_stats.bytes_read) as f64 
                    / elapsed.as_secs_f64();
                io.write_bandwidth = (io.bytes_written - current.io_stats.bytes_written) as f64 
                    / elapsed.as_secs_f64();
            }
            
            // Enforce I/O limits
            if io.read_bandwidth > limits.io_bandwidth_limit as f64 
                || io.write_bandwidth > limits.io_bandwidth_limit as f64 {
                Self::throttle_io().await?;
            }
            
            current.io_stats = io;
        }
        
        // Update process stats
        if let Ok((procs, threads, fds)) = Self::get_process_stats().await {
            current.process_count = procs;
            current.thread_count = threads;
            current.fd_count = fds;
            
            // Enforce process limits
            if procs > limits.process_limit {
                Self::limit_processes(limits.process_limit).await?;
            }
        }
        
        current.last_update = now;
        Ok(())
    }

    /// Get current CPU usage
    async fn get_cpu_usage() -> BlastResult<f64> {
        #[cfg(target_os = "linux")]
        {
            use std::fs::File;
            use std::io::Read;
            
            let mut stat = String::new();
            File::open("/proc/stat")?.read_to_string(&mut stat)?;
            
            // Parse CPU stats
            // TODO: Implement proper CPU usage calculation
            Ok(0.0)
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Ok(0.0)
        }
    }

    /// Get current memory usage
    async fn get_memory_usage() -> BlastResult<u64> {
        #[cfg(target_os = "linux")]
        {
            use std::fs::File;
            use std::io::Read;
            
            let mut status = String::new();
            File::open("/proc/self/status")?.read_to_string(&mut status)?;
            
            // Parse memory usage
            // TODO: Implement proper memory usage calculation
            Ok(0)
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Ok(0)
        }
    }

    /// Get current I/O statistics
    async fn get_io_stats() -> BlastResult<IOStats> {
        #[cfg(target_os = "linux")]
        {
            use std::fs::File;
            use std::io::Read;
            
            let mut io = String::new();
            File::open("/proc/self/io")?.read_to_string(&mut io)?;
            
            // Parse I/O stats
            // TODO: Implement proper I/O stats calculation
            Ok(IOStats::default())
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Ok(IOStats::default())
        }
    }

    /// Get process statistics
    async fn get_process_stats() -> BlastResult<(u32, u32, u32)> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            
            // Count processes in /proc
            let procs = fs::read_dir("/proc")?
                .filter(|entry| {
                    if let Ok(entry) = entry {
                        entry.file_name()
                            .to_string_lossy()
                            .chars()
                            .all(|c| c.is_digit(10))
                    } else {
                        false
                    }
                })
                .count() as u32;
            
            // TODO: Implement proper thread and fd counting
            Ok((procs, 0, 0))
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Ok((0, 0, 0))
        }
    }

    /// Throttle CPU usage
    async fn throttle_cpu() -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            // TODO: Implement CPU throttling using cgroups
        }
        Ok(())
    }

    /// Limit memory usage
    async fn limit_memory(limit: u64) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            // TODO: Implement memory limiting using cgroups
        }
        Ok(())
    }

    /// Throttle I/O operations
    async fn throttle_io() -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            // TODO: Implement I/O throttling using cgroups
        }
        Ok(())
    }

    /// Limit number of processes
    async fn limit_processes(limit: u32) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            // TODO: Implement process limiting using cgroups
        }
        Ok(())
    }

    /// Get current resource usage
    pub async fn get_usage(&self) -> BlastResult<ResourceUsage> {
        Ok(self.usage.read().await.clone())
    }

    /// Check if resource limits are exceeded
    pub async fn check_limits(&self) -> BlastResult<Vec<String>> {
        let usage = self.usage.read().await;
        let mut violations = Vec::new();
        
        if usage.cpu_usage > self.limits.cpu_limit {
            violations.push(format!("CPU usage exceeds limit: {:.1}%", usage.cpu_usage));
        }
        
        if usage.memory_usage > self.limits.memory_limit {
            violations.push(format!("Memory usage exceeds limit: {} bytes", usage.memory_usage));
        }
        
        if usage.io_stats.read_bandwidth > self.limits.io_bandwidth_limit as f64 {
            violations.push(format!("Read bandwidth exceeds limit: {:.1} bytes/sec", 
                usage.io_stats.read_bandwidth));
        }
        
        if usage.io_stats.write_bandwidth > self.limits.io_bandwidth_limit as f64 {
            violations.push(format!("Write bandwidth exceeds limit: {:.1} bytes/sec",
                usage.io_stats.write_bandwidth));
        }
        
        if usage.process_count > self.limits.process_limit {
            violations.push(format!("Process count exceeds limit: {}", usage.process_count));
        }
        
        Ok(violations)
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limit: 100.0,
            memory_limit: 1024 * 1024 * 1024 * 2, // 2GB
            disk_limit: 1024 * 1024 * 1024 * 10,  // 10GB
            iops_limit: 1000,
            io_bandwidth_limit: 1024 * 1024 * 100, // 100MB/s
            process_limit: 50,
            thread_limit: 100,
            fd_limit: 1000,
        }
    }
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU limits
    pub cpu: CpuLimits,
    /// Memory limits
    pub memory: MemoryLimits,
    /// I/O limits
    pub io: IoLimits,
    /// Process limits
    pub process: ProcessLimits,
    /// Network limits
    pub network: NetworkLimits,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu: CpuLimits::default(),
            memory: MemoryLimits::default(),
            io: IoLimits::default(),
            process: ProcessLimits::default(),
            network: NetworkLimits::default(),
        }
    }
}

/// CPU resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuLimits {
    /// CPU shares (relative weight)
    pub shares: u64,
    /// CPU quota in microseconds
    pub quota_us: i64,
    /// CPU period in microseconds
    pub period_us: u64,
    /// CPU set (cores allowed)
    pub cpuset: Vec<u32>,
    /// CPU bandwidth weight
    pub weight: u16,
}

impl Default for CpuLimits {
    fn default() -> Self {
        Self {
            shares: 1024,
            quota_us: -1,
            period_us: 100000,
            cpuset: Vec::new(),
            weight: 100,
        }
    }
}

/// Memory resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    /// Memory limit in bytes
    pub limit_bytes: i64,
    /// Memory soft limit in bytes
    pub soft_limit_bytes: i64,
    /// Kernel memory limit in bytes
    pub kernel_limit_bytes: i64,
    /// Swap limit in bytes
    pub swap_limit_bytes: i64,
    /// Memory swappiness
    pub swappiness: u8,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            limit_bytes: -1,
            soft_limit_bytes: -1,
            kernel_limit_bytes: -1,
            swap_limit_bytes: -1,
            swappiness: 60,
        }
    }
}

/// I/O resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoLimits {
    /// I/O weight
    pub weight: u16,
    /// Device specific limits
    pub device_limits: HashMap<String, DeviceLimit>,
}

impl Default for IoLimits {
    fn default() -> Self {
        Self {
            weight: 100,
            device_limits: HashMap::new(),
        }
    }
}

/// Device specific I/O limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLimit {
    /// Read bytes per second
    pub read_bps: Option<u64>,
    /// Write bytes per second
    pub write_bps: Option<u64>,
    /// Read IOPS
    pub read_iops: Option<u64>,
    /// Write IOPS
    pub write_iops: Option<u64>,
}

/// Process resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessLimits {
    /// Maximum number of processes
    pub max_processes: i64,
    /// Maximum number of open file descriptors
    pub max_open_files: i64,
    /// Maximum number of threads
    pub max_threads: i64,
}

impl Default for ProcessLimits {
    fn default() -> Self {
        Self {
            max_processes: -1,
            max_open_files: -1,
            max_threads: -1,
        }
    }
}

/// Network resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLimits {
    /// Bandwidth limit in bytes per second
    pub bandwidth_bps: i64,
    /// Interface specific limits
    pub interface_limits: HashMap<String, InterfaceLimit>,
}

impl Default for NetworkLimits {
    fn default() -> Self {
        Self {
            bandwidth_bps: -1,
            interface_limits: HashMap::new(),
        }
    }
}

/// Interface specific network limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceLimit {
    /// Ingress bandwidth limit in bytes per second
    pub ingress_bps: Option<u64>,
    /// Egress bandwidth limit in bytes per second
    pub egress_bps: Option<u64>,
}

/// Resource manager implementation
pub struct ResourceManager {
    /// Resource limits
    #[allow(dead_code)]  // Used in async methods or part of public API
    limits: ResourceLimits,
    /// Resource usage statistics
    usage: ResourceUsage,
}

impl ResourceManager {
    /// Create new resource manager
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            usage: ResourceUsage::default(),
        }
    }

    /// Apply resource limits
    pub async fn apply_limits(&self) -> BlastResult<()> {
        self.apply_cpu_limits().await?;
        self.apply_memory_limits().await?;
        self.apply_io_limits().await?;
        self.apply_process_limits().await?;
        self.apply_network_limits().await?;
        Ok(())
    }

    /// Apply CPU limits
    async fn apply_cpu_limits(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            use std::io::Write;
            
            let cgroup_path = "/sys/fs/cgroup/cpu";
            
            // Set CPU shares
            let shares_path = format!("{}/cpu.shares", cgroup_path);
            fs::write(shares_path, self.limits.cpu.shares.to_string())?;
            
            // Set CPU quota
            let quota_path = format!("{}/cpu.cfs_quota_us", cgroup_path);
            fs::write(quota_path, self.limits.cpu.quota_us.to_string())?;
            
            // Set CPU period
            let period_path = format!("{}/cpu.cfs_period_us", cgroup_path);
            fs::write(period_path, self.limits.cpu.period_us.to_string())?;
            
            // Set CPU set
            if !self.limits.cpu.cpuset.is_empty() {
                let cpuset_path = format!("{}/cpuset.cpus", cgroup_path);
                let cpuset_str = self.limits.cpu.cpuset
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                fs::write(cpuset_path, cpuset_str)?;
            }
        }
        
        Ok(())
    }

    /// Apply memory limits
    async fn apply_memory_limits(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            
            let cgroup_path = "/sys/fs/cgroup/memory";
            
            // Set memory limit
            if self.limits.memory.limit_bytes >= 0 {
                let limit_path = format!("{}/memory.limit_in_bytes", cgroup_path);
                fs::write(limit_path, self.limits.memory.limit_bytes.to_string())?;
            }
            
            // Set memory soft limit
            if self.limits.memory.soft_limit_bytes >= 0 {
                let soft_limit_path = format!("{}/memory.soft_limit_in_bytes", cgroup_path);
                fs::write(soft_limit_path, self.limits.memory.soft_limit_bytes.to_string())?;
            }
            
            // Set kernel memory limit
            if self.limits.memory.kernel_limit_bytes >= 0 {
                let kernel_limit_path = format!("{}/memory.kmem.limit_in_bytes", cgroup_path);
                fs::write(kernel_limit_path, self.limits.memory.kernel_limit_bytes.to_string())?;
            }
            
            // Set swap limit
            if self.limits.memory.swap_limit_bytes >= 0 {
                let swap_limit_path = format!("{}/memory.memsw.limit_in_bytes", cgroup_path);
                fs::write(swap_limit_path, self.limits.memory.swap_limit_bytes.to_string())?;
            }
            
            // Set swappiness
            let swappiness_path = format!("{}/memory.swappiness", cgroup_path);
            fs::write(swappiness_path, self.limits.memory.swappiness.to_string())?;
        }
        
        Ok(())
    }

    /// Apply I/O limits
    async fn apply_io_limits(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            
            let cgroup_path = "/sys/fs/cgroup/blkio";
            
            // Set I/O weight
            let weight_path = format!("{}/blkio.weight", cgroup_path);
            fs::write(weight_path, self.limits.io.weight.to_string())?;
            
            // Set device specific limits
            for (device, limit) in &self.limits.io.device_limits {
                if let Some(read_bps) = limit.read_bps {
                    let read_bps_path = format!("{}/blkio.throttle.read_bps_device", cgroup_path);
                    fs::write(read_bps_path, format!("{} {}", device, read_bps))?;
                }
                
                if let Some(write_bps) = limit.write_bps {
                    let write_bps_path = format!("{}/blkio.throttle.write_bps_device", cgroup_path);
                    fs::write(write_bps_path, format!("{} {}", device, write_bps))?;
                }
                
                if let Some(read_iops) = limit.read_iops {
                    let read_iops_path = format!("{}/blkio.throttle.read_iops_device", cgroup_path);
                    fs::write(read_iops_path, format!("{} {}", device, read_iops))?;
                }
                
                if let Some(write_iops) = limit.write_iops {
                    let write_iops_path = format!("{}/blkio.throttle.write_iops_device", cgroup_path);
                    fs::write(write_iops_path, format!("{} {}", device, write_iops))?;
                }
            }
        }
        
        Ok(())
    }

    /// Apply process limits
    async fn apply_process_limits(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            
            let cgroup_path = "/sys/fs/cgroup/pids";
            
            // Set maximum number of processes
            if self.limits.process.max_processes >= 0 {
                let max_procs_path = format!("{}/pids.max", cgroup_path);
                fs::write(max_procs_path, self.limits.process.max_processes.to_string())?;
            }
            
            // Set maximum number of open files
            if self.limits.process.max_open_files >= 0 {
                use rlimit::Resource;
                rlimit::setrlimit(
                    Resource::NOFILE,
                    self.limits.process.max_open_files as u64,
                    self.limits.process.max_open_files as u64,
                )?;
            }
            
            // Set maximum number of threads
            if self.limits.process.max_threads >= 0 {
                let max_threads_path = format!("{}/pids.max_threads", cgroup_path);
                fs::write(max_threads_path, self.limits.process.max_threads.to_string())?;
            }
        }
        
        Ok(())
    }

    /// Apply network limits
    async fn apply_network_limits(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            
            // Set global bandwidth limit
            if self.limits.network.bandwidth_bps >= 0 {
                Command::new("tc")
                    .args(&[
                        "qdisc",
                        "add",
                        "dev",
                        "eth0",
                        "root",
                        "tbf",
                        "rate",
                        &self.limits.network.bandwidth_bps.to_string(),
                        "latency",
                        "50ms",
                        "burst",
                        "1540",
                    ])
                    .output()?;
            }
            
            // Set interface specific limits
            for (interface, limit) in &self.limits.network.interface_limits {
                if let Some(ingress_bps) = limit.ingress_bps {
                    Command::new("tc")
                        .args(&[
                            "qdisc",
                            "add",
                            "dev",
                            interface,
                            "ingress",
                        ])
                        .output()?;
                        
                    Command::new("tc")
                        .args(&[
                            "filter",
                            "add",
                            "dev",
                            interface,
                            "parent",
                            "ffff:",
                            "protocol",
                            "ip",
                            "u32",
                            "match",
                            "ip",
                            "src",
                            "0.0.0.0/0",
                            "police",
                            "rate",
                            &ingress_bps.to_string(),
                            "burst",
                            "10k",
                            "drop",
                            "flowid",
                            ":1",
                        ])
                        .output()?;
                }
                
                if let Some(egress_bps) = limit.egress_bps {
                    Command::new("tc")
                        .args(&[
                            "qdisc",
                            "add",
                            "dev",
                            interface,
                            "root",
                            "tbf",
                            "rate",
                            &egress_bps.to_string(),
                            "latency",
                            "50ms",
                            "burst",
                            "1540",
                        ])
                        .output()?;
                }
            }
        }
        
        Ok(())
    }

    /// Get current resource usage
    pub async fn get_usage(&self) -> BlastResult<ResourceUsage> {
        Ok(self.usage.clone())
    }

    /// Update resource usage statistics
    pub async fn update_usage(&mut self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            
            // Update CPU usage
            let cpu_usage_path = "/sys/fs/cgroup/cpu/cpuacct.usage";
            let cpu_usage = fs::read_to_string(cpu_usage_path)?
                .trim()
                .parse::<u64>()?;
            self.usage.cpu.usage_ns = cpu_usage;
            
            // Update memory usage
            let memory_usage_path = "/sys/fs/cgroup/memory/memory.usage_in_bytes";
            let memory_usage = fs::read_to_string(memory_usage_path)?
                .trim()
                .parse::<u64>()?;
            self.usage.memory.usage_bytes = memory_usage;
            
            // Update I/O usage
            let io_usage_path = "/sys/fs/cgroup/blkio/blkio.throttle.io_service_bytes";
            let io_usage = fs::read_to_string(io_usage_path)?;
            for line in io_usage.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 3 {
                    match parts[1] {
                        "Read" => {
                            self.usage.io.read_bytes = parts[2].parse::<u64>()?;
                        }
                        "Write" => {
                            self.usage.io.write_bytes = parts[2].parse::<u64>()?;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Resource usage statistics
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// CPU usage statistics
    pub cpu: CpuUsage,
    /// Memory usage statistics
    pub memory: MemoryUsage,
    /// I/O usage statistics
    pub io: IoUsage,
}

/// CPU usage statistics
#[derive(Debug, Clone, Default)]
pub struct CpuUsage {
    /// CPU usage in nanoseconds
    pub usage_ns: u64,
    /// CPU usage percentage
    pub usage_percent: f64,
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryUsage {
    /// Memory usage in bytes
    pub usage_bytes: u64,
    /// Memory usage percentage
    pub usage_percent: f64,
}

/// I/O usage statistics
#[derive(Debug, Clone, Default)]
pub struct IoUsage {
    /// Read bytes
    pub read_bytes: u64,
    /// Write bytes
    pub write_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_limits() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.cpu.shares, 1024);
        assert_eq!(limits.memory.swappiness, 60);
        assert_eq!(limits.io.weight, 100);
    }

    #[tokio::test]
    async fn test_resource_manager() {
        let limits = ResourceLimits::default();
        let manager = ResourceManager::new(limits);
        
        // Test applying limits
        manager.apply_limits().await.unwrap();
        
        // Test getting usage
        let usage = manager.get_usage().await.unwrap();
        assert_eq!(usage.cpu.usage_ns, 0);
        assert_eq!(usage.memory.usage_bytes, 0);
        assert_eq!(usage.io.read_bytes, 0);
        assert_eq!(usage.io.write_bytes, 0);
    }
} 