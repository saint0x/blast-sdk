use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::error::BlastResult;
use nix::mount::MntFlags;
use std::time::{SystemTime, Duration};

/// Filesystem policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPolicy {
    /// Root directory
    pub root_dir: PathBuf,
    /// Read-only paths
    pub readonly_paths: Vec<PathBuf>,
    /// Hidden paths
    pub hidden_paths: Vec<PathBuf>,
    /// Allowed paths
    pub allowed_paths: Vec<PathBuf>,
    /// Denied paths
    pub denied_paths: Vec<PathBuf>,
    /// Mount points
    pub mount_points: HashMap<PathBuf, MountConfig>,
    /// Temporary directory
    pub tmp_dir: PathBuf,
    /// Maximum file size
    pub max_file_size: u64,
    /// Maximum total size
    pub max_total_size: u64,
}

/// Mount point configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountConfig {
    /// Source path
    pub source: PathBuf,
    /// Mount options
    pub options: Vec<String>,
    /// Mount type
    pub mount_type: MountType,
    /// Read-only mount
    pub readonly: bool,
}

/// Mount type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MountType {
    /// Bind mount
    Bind,
    /// Tmpfs mount
    Tmpfs,
    /// Overlay mount
    Overlay,
}

/// Filesystem state tracking
#[derive(Debug)]
pub struct FilesystemState {
    /// Active mounts
    mounts: Arc<RwLock<HashMap<PathBuf, MountInfo>>>,
    /// File access tracking
    access_tracking: Arc<RwLock<HashMap<PathBuf, FileAccessInfo>>>,
    /// Filesystem policy
    policy: FilesystemPolicy,
}

/// Mount information
#[derive(Debug, Clone)]
pub struct MountInfo {
    /// Mount point
    pub mount_point: PathBuf,
    /// Mount configuration
    pub config: MountConfig,
    /// Mount time
    pub mounted_at: std::time::SystemTime,
    /// Current size
    pub current_size: u64,
}

/// File access information
#[derive(Debug, Clone)]
pub struct FileAccessInfo {
    /// Read count
    pub read_count: u64,
    /// Write count
    pub write_count: u64,
    /// Last access time
    pub last_access: std::time::SystemTime,
    /// Creation time
    pub created_at: std::time::SystemTime,
    /// Last modification time
    pub last_modified: std::time::SystemTime,
    /// File size
    pub size: u64,
    /// Is directory
    pub is_directory: bool,
    /// Access patterns detected
    pub access_patterns: Vec<AccessPattern>,
    /// Security violations
    pub security_violations: Vec<SecurityViolation>,
    /// Owner
    pub owner: String,
    /// Permissions
    pub permissions: u32,
}

/// Access pattern types
#[derive(Debug, Clone)]
pub enum AccessPattern {
    /// Rapid repeated access
    RapidAccess { count: u32, interval: Duration },
    /// Large data transfer
    LargeTransfer { bytes: u64, duration: Duration },
    /// Sequential access
    Sequential { operations: u32 },
    /// Random access
    Random { operations: u32 },
}

/// Security violation types
#[derive(Debug, Clone)]
pub enum SecurityViolation {
    /// Unauthorized access attempt
    UnauthorizedAccess { timestamp: SystemTime, reason: String },
    /// Size limit exceeded
    SizeLimitExceeded { size: u64, limit: u64 },
    /// Invalid operation
    InvalidOperation { op: String, reason: String },
    /// Suspicious pattern
    SuspiciousPattern { pattern: String, details: String },
}

impl Default for FileAccessInfo {
    fn default() -> Self {
        let now = std::time::SystemTime::now();
        Self {
            read_count: 0,
            write_count: 0,
            last_access: now,
            created_at: now,
            last_modified: now,
            size: 0,
            is_directory: false,
            access_patterns: Vec::new(),
            security_violations: Vec::new(),
            owner: String::new(),
            permissions: 0o644,
        }
    }
}

/// Mount operation record for rollback
#[derive(Debug, Clone)]
pub struct MountOperation {
    mount_point: PathBuf,
    config: MountConfig,
    timestamp: SystemTime,
    successful: bool,
}

impl FilesystemState {
    /// Create new filesystem state
    pub fn new(policy: FilesystemPolicy) -> Self {
        Self {
            mounts: Arc::new(RwLock::new(HashMap::new())),
            access_tracking: Arc::new(RwLock::new(HashMap::new())),
            policy,
        }
    }

    /// Check if path is allowed
    pub fn is_path_allowed(&self, path: &Path) -> bool {
        // Check denied paths first
        if self.policy.denied_paths.iter().any(|p| path.starts_with(p)) {
            return false;
        }

        // Check if path is explicitly allowed
        if self.policy.allowed_paths.iter().any(|p| path.starts_with(p)) {
            return true;
        }

        // Check if path is under root directory
        path.starts_with(&self.policy.root_dir)
    }

    /// Check if path is read-only
    pub fn is_path_readonly(&self, path: &Path) -> bool {
        self.policy.readonly_paths.iter().any(|p| path.starts_with(p))
    }

    /// Check if path should be hidden
    pub fn should_hide_path(&self, path: &Path) -> bool {
        self.policy.hidden_paths.iter().any(|p| path.starts_with(p))
    }

    /// Track file access with enhanced monitoring
    pub async fn track_access(&self, path: &Path, is_write: bool, size: u64) -> BlastResult<()> {
        let mut tracking = self.access_tracking.write().await;
        let info = tracking.entry(path.to_path_buf()).or_default();
        let now = std::time::SystemTime::now();
        
        if is_write {
            info.write_count += 1;
            info.last_modified = now;

            // Check size limits
            if size > self.policy.max_file_size {
                info.security_violations.push(SecurityViolation::SizeLimitExceeded {
                    size,
                    limit: self.policy.max_file_size,
                });
                return Err(crate::error::BlastError::security("File size limit exceeded"));
            }
        } else {
            info.read_count += 1;
        }
        
        // Update basic info
        info.last_access = now;
        info.size = size;
        info.is_directory = path.is_dir();
        
        // Detect access patterns
        if let Some(last_access) = info.last_access.checked_sub(Duration::from_secs(1)) {
            if last_access.elapsed().unwrap().as_secs() < 1 {
                info.access_patterns.push(AccessPattern::RapidAccess {
                    count: 1,
                    interval: Duration::from_secs(1),
                });
            }
        }
        
        if size > 1024 * 1024 { // 1MB
            info.access_patterns.push(AccessPattern::LargeTransfer {
                bytes: size,
                duration: Duration::from_secs(1),
            });
        }

        // Log suspicious patterns
        if info.access_patterns.len() > 10 {
            info.security_violations.push(SecurityViolation::SuspiciousPattern {
                pattern: "High frequency access".to_string(),
                details: format!("Access count: {}", info.access_patterns.len()),
            });
            tracing::warn!("Suspicious access pattern detected for path: {}", path.display());
        }
        
        Ok(())
    }

    /// Validate mount configuration
    async fn validate_mount_config(&self, mount_point: &Path, config: &MountConfig) -> BlastResult<()> {
        // Check for path traversal
        if mount_point.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err(crate::error::BlastError::security("Path traversal detected in mount point"));
        }

        // Validate source path
        if let MountType::Bind = config.mount_type {
            if !config.source.exists() {
                return Err(crate::error::BlastError::security("Mount source does not exist"));
            }
            if config.source.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                return Err(crate::error::BlastError::security("Path traversal detected in source"));
            }
        }

        // Validate mount options
        let forbidden_opts = ["suid", "dev", "exec"];
        for opt in &config.options {
            if forbidden_opts.iter().any(|f| opt.contains(f)) {
                return Err(crate::error::BlastError::security(format!(
                    "Forbidden mount option detected: {}", opt
                )));
            }
        }

        // Check for recursive mounts
        let mounts = self.mounts.read().await;
        for existing in mounts.keys() {
            if mount_point.starts_with(existing) || existing.starts_with(mount_point) {
                return Err(crate::error::BlastError::security("Recursive mount detected"));
            }
        }

        Ok(())
    }

    /// Perform mount operation with rollback support
    async fn perform_mount_operation(&self, mount_point: &Path, config: &MountConfig) -> BlastResult<()> {
        let _operation = MountOperation {
            mount_point: mount_point.to_path_buf(),
            config: config.clone(),
            timestamp: SystemTime::now(),
            successful: false,
        };

        // Create parent directories if they don't exist
        if let Some(parent) = mount_point.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Perform mount
        #[cfg(target_os = "linux")]
        {
            use nix::mount::mount;
            
            let source = config.source.to_str();
            let target = mount_point.to_str();
            let fstype = match config.mount_type {
                MountType::Bind => Some("none"),
                MountType::Tmpfs => Some("tmpfs"),
                MountType::Overlay => Some("overlay"),
            };
            
            let mut flags = nix::mount::MsFlags::empty();
            if config.readonly {
                flags |= nix::mount::MsFlags::MS_RDONLY;
            }
            flags |= nix::mount::MsFlags::MS_NODEV | nix::mount::MsFlags::MS_NOSUID | nix::mount::MsFlags::MS_NOEXEC;
            
            if let Err(e) = mount(
                source,
                target,
                fstype,
                flags,
                Some(&config.options.join(",")),
            ) {
                // Cleanup on failure
                if let Err(cleanup_err) = self.cleanup_failed_mount(&_operation).await {
                    tracing::error!("Failed to cleanup after mount failure: {}", cleanup_err);
                }
                return Err(e.into());
            }
        }

        // Update mount state
        let _info = MountInfo {
            mount_point: mount_point.to_path_buf(),
            config: config.clone(),
            mounted_at: SystemTime::now(),
            current_size: 0,
        };
        
        let mut mounts = self.mounts.write().await;
        mounts.insert(mount_point.to_path_buf(), _info);
        Ok(())
    }

    /// Cleanup after failed mount
    async fn cleanup_failed_mount(&self, _operation: &MountOperation) -> BlastResult<()> {
        // Remove any created directories
        if let Some(parent) = _operation.mount_point.parent() {
            if parent.exists() {
                let mut entries = tokio::fs::read_dir(parent).await?;
                if entries.next_entry().await?.is_none() {
                    tokio::fs::remove_dir_all(parent).await?;
                }
            }
        }

        // Attempt to unmount if partially mounted
        #[cfg(target_os = "linux")]
        {
            use nix::mount::umount;
            if _operation.mount_point.exists() {
                if let Err(e) = umount(&_operation.mount_point) {
                    tracing::warn!("Failed to unmount during cleanup: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Recover mount state
    pub async fn recover_mount_state(&self) -> BlastResult<()> {
        let mounts = self.mounts.read().await;
        let mut failed_mounts = Vec::new();

        for (mount_point, _info) in mounts.iter() {
            // Check if mount is still valid
            if !mount_point.exists() {
                failed_mounts.push(mount_point.clone());
                continue;
            }

            // Attempt to remount if necessary
            #[cfg(target_os = "linux")]
            {
                use nix::mount::{mount, umount};
                
                // Unmount first to ensure clean state
                if let Err(e) = umount(mount_point) {
                    tracing::warn!("Failed to unmount during recovery: {}", e);
                    failed_mounts.push(mount_point.clone());
                    continue;
                }

                // Attempt remount
                if let Err(e) = self.perform_mount_operation(mount_point, &info.config).await {
                    tracing::error!("Failed to recover mount {}: {}", mount_point.display(), e);
                    failed_mounts.push(mount_point.clone());
                }
            }
        }

        // Remove failed mounts from state
        if !failed_mounts.is_empty() {
            let mut mounts = self.mounts.write().await;
            for mount_point in failed_mounts {
                mounts.remove(&mount_point);
            }
        }

        Ok(())
    }

    /// Mount filesystem with recovery
    pub async fn mount(&self, mount_point: &Path, config: MountConfig) -> BlastResult<()> {
        // Validate mount configuration first
        self.validate_mount_config(mount_point, &config).await?;

        let mut mounts = self.mounts.write().await;
        
        // Check if mount point is allowed
        if !self.is_path_allowed(mount_point) {
            return Err(crate::error::BlastError::security(format!(
                "Mount point not allowed: {}", mount_point.display()
            )));
        }

        // Attempt mount operation
        if let Err(e) = self.perform_mount_operation(mount_point, &config).await {
            tracing::error!("Mount operation failed: {}", e);
            return Err(e);
        }
        
        // Update mount state
        let _info = MountInfo {
            mount_point: mount_point.to_path_buf(),
            config: config.clone(),
            mounted_at: SystemTime::now(),
            current_size: 0,
        };
        
        mounts.insert(mount_point.to_path_buf(), _info);
        Ok(())
    }

    /// Unmount filesystem
    pub async fn unmount(&self, mount_point: &Path) -> BlastResult<()> {
        let mut mounts = self.mounts.write().await;
        
        if let Some(_) = mounts.remove(mount_point) {
            #[cfg(target_os = "linux")]
            {
                use nix::mount::umount;
                umount(mount_point)?;
            }
        }
        
        Ok(())
    }

    /// Get mount information
    pub async fn get_mount_info(&self, mount_point: &Path) -> BlastResult<Option<MountInfo>> {
        Ok(self.mounts.read().await.get(mount_point).cloned())
    }

    /// Get file access information
    pub async fn get_access_info(&self, path: &Path) -> BlastResult<Option<FileAccessInfo>> {
        Ok(self.access_tracking.read().await.get(path).cloned())
    }

    /// Calculate total filesystem usage
    pub async fn get_total_size(&self) -> BlastResult<u64> {
        let tracking = self.access_tracking.read().await;
        Ok(tracking.values().map(|info| info.size).sum())
    }

    /// Clean up old files
    pub async fn cleanup_old_files(&self, max_age: std::time::Duration) -> BlastResult<()> {
        let mut tracking = self.access_tracking.write().await;
        let now = std::time::SystemTime::now();
        
        tracking.retain(|path, info| {
            if let Ok(age) = now.duration_since(info.last_access) {
                if age > max_age {
                    // Remove file if it exists
                    let _ = std::fs::remove_file(path);
                    return false;
                }
            }
            true
        });
        
        Ok(())
    }

    /// Get mount points
    pub async fn get_mount_points(&self) -> BlastResult<Vec<PathBuf>> {
        Ok(self.mounts.read().await.keys().cloned().collect())
    }

    /// Get security violations for path
    pub async fn get_security_violations(&self, path: &Path) -> BlastResult<Vec<SecurityViolation>> {
        let tracking = self.access_tracking.read().await;
        Ok(tracking.get(path)
            .map(|info| info.security_violations.clone())
            .unwrap_or_default())
    }

    /// Get access patterns for path
    pub async fn get_access_patterns(&self, path: &Path) -> BlastResult<Vec<AccessPattern>> {
        let tracking = self.access_tracking.read().await;
        Ok(tracking.get(path)
            .map(|info| info.access_patterns.clone())
            .unwrap_or_default())
    }
}

impl Default for FilesystemPolicy {
    fn default() -> Self {
        Self {
            root_dir: PathBuf::from("/var/lib/blast/environments"),
            readonly_paths: vec![
                PathBuf::from("/usr"),
                PathBuf::from("/lib"),
                PathBuf::from("/bin"),
            ],
            hidden_paths: vec![
                PathBuf::from("/proc"),
                PathBuf::from("/sys"),
                PathBuf::from("/dev"),
            ],
            allowed_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
            ],
            denied_paths: vec![
                PathBuf::from("/etc/shadow"),
                PathBuf::from("/etc/passwd"),
                PathBuf::from("/root"),
            ],
            mount_points: HashMap::new(),
            tmp_dir: PathBuf::from("/tmp"),
            max_file_size: 1024 * 1024 * 100, // 100MB
            max_total_size: 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// Mount flags wrapper
#[derive(Debug, Clone)]
pub struct MountFlags(pub MntFlags);

impl Serialize for MountFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize the bits as a u32
        serializer.serialize_u32(self.0.bits().try_into().unwrap())
    }
}

impl<'de> Deserialize<'de> for MountFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize as u32 and convert back to MntFlags
        let bits = u32::deserialize(deserializer)?;
        Ok(MountFlags(MntFlags::from_bits_truncate(bits.try_into().unwrap())))
    }
}

impl Default for MountFlags {
    fn default() -> Self {
        Self(MntFlags::empty())
    }
} 