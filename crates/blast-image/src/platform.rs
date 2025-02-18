//! Platform-specific requirements and information
//! 
//! This module provides types and functionality for managing platform-specific
//! requirements and capabilities for Python environments.

use serde::{Deserialize, Serialize};

/// Platform-specific requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformRequirements {
    /// Operating system requirements
    pub os: Vec<String>,
    /// CPU architecture
    pub arch: Vec<String>,
    /// Minimum CPU cores
    pub min_cores: u32,
    /// Minimum memory in bytes
    pub min_memory: u64,
    /// Minimum disk space in bytes
    pub min_disk_space: u64,
    /// Required system features
    pub required_features: Vec<String>,
    /// GPU requirements
    pub gpu_requirements: Option<GpuRequirements>,
}

/// GPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Required GPU memory
    pub min_memory: u64,
    /// Required CUDA version
    pub cuda_version: Option<String>,
    /// Required ROCm version
    pub rocm_version: Option<String>,
    /// Required features
    pub required_features: Vec<String>,
}

/// Platform-specific information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Operating system
    pub os: String,
    /// Architecture
    pub arch: String,
    /// Required system dependencies
    pub system_deps: Vec<String>,
    /// Minimum required disk space in bytes
    pub min_disk_space: u64,
    /// Minimum required memory in bytes
    pub min_memory: u64,
}

impl Default for PlatformRequirements {
    fn default() -> Self {
        Self {
            os: vec!["linux".to_string(), "darwin".to_string()],
            arch: vec!["x86_64".to_string(), "aarch64".to_string()],
            min_cores: 1,
            min_memory: 1024 * 1024 * 1024, // 1GB
            min_disk_space: 5 * 1024 * 1024 * 1024, // 5GB
            required_features: Vec::new(),
            gpu_requirements: None,
        }
    }
}

impl PlatformInfo {
    /// Get current platform information
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            system_deps: Vec::new(),
            min_disk_space: 1024 * 1024 * 1024, // 1GB
            min_memory: 512 * 1024 * 1024,      // 512MB
        }
    }

    /// Check if current platform meets requirements
    pub fn meets_requirements(&self, requirements: &PlatformRequirements) -> bool {
        // Check OS compatibility
        if !requirements.os.iter().any(|os| os == &self.os) {
            return false;
        }

        // Check architecture compatibility
        if !requirements.arch.iter().any(|arch| arch == &self.arch) {
            return false;
        }

        // Check memory requirements
        if self.min_memory < requirements.min_memory {
            return false;
        }

        // Check disk space requirements
        if self.min_disk_space < requirements.min_disk_space {
            return false;
        }

        true
    }

    /// Get available GPU devices
    #[cfg(feature = "gpu")]
    pub fn get_gpu_devices() -> Vec<GpuDevice> {
        // Implementation depends on GPU detection libraries
        vec![]
    }
}

#[cfg(feature = "gpu")]
#[derive(Debug, Clone)]
pub struct GpuDevice {
    /// Device name
    pub name: String,
    /// Available memory
    pub memory: u64,
    /// CUDA compute capability
    pub cuda_capability: Option<String>,
    /// ROCm version
    pub rocm_version: Option<String>,
} 