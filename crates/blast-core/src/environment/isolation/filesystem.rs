use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::error::BlastResult;
use nix::mount::MntFlags;

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
#[derive(Debug, Clone, Default)]
pub struct FileAccessInfo {
    /// Read count
    pub read_count: u64,
    /// Write count
    pub write_count: u64,
    /// Last access time
    pub last_access: std::time::SystemTime,
    /// File size
    pub size: u64,
    /// Is directory
    pub is_directory: bool,
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

    /// Track file access
    pub async fn track_access(&self, path: &Path, is_write: bool, size: u64) -> BlastResult<()> {
        let mut tracking = self.access_tracking.write().await;
        let info = tracking.entry(path.to_path_buf()).or_default();
        
        if is_write {
            info.write_count += 1;
        } else {
            info.read_count += 1;
        }
        
        info.last_access = std::time::SystemTime::now();
        info.size = size;
        info.is_directory = path.is_dir();
        
        Ok(())
    }

    /// Mount filesystem
    pub async fn mount(&self, mount_point: &Path, config: MountConfig) -> BlastResult<()> {
        let mut mounts = self.mounts.write().await;
        
        // Check if mount point is allowed
        if !self.is_path_allowed(mount_point) {
            return Err(crate::error::BlastError::security(format!(
                "Mount point not allowed: {}", mount_point.display()
            )));
        }
        
        // Create mount info
        let info = MountInfo {
            mount_point: mount_point.to_path_buf(),
            config: config.clone(),
            mounted_at: std::time::SystemTime::now(),
            current_size: 0,
        };
        
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
            
            mount(
                source,
                target,
                fstype,
                flags,
                Some(&config.options.join(",")),
            )?;
        }
        
        mounts.insert(mount_point.to_path_buf(), info);
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