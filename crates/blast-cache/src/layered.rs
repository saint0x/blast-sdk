use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use std::sync::Arc;

use chrono;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use blast_core::error::BlastResult;

use crate::compression::CompressionLevel;
use crate::memory::MemoryCacheStats;
use crate::disk::DiskCacheStats;
use crate::storage::CacheStorage;

/// Cache layer type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    /// Base layer containing Python interpreter
    Base,
    /// Layer containing installed packages
    Packages,
    /// Layer containing environment configuration
    Config,
    /// Layer containing user files
    Files,
}

/// Cache layer metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheLayer {
    /// Package layer (wheels, source distributions)
    Package {
        name: String,
        version: String,
        hash: String,
    },
    /// Built package layer
    Build {
        package: String,
        version: String,
        platform: String,
        python_version: String,
    },
    /// Environment snapshot layer
    Environment {
        name: String,
        python_version: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Dependency resolution layer
    Resolution {
        requirements: Vec<String>,
        python_version: String,
        platform: String,
    },
    /// Image layer
    ImageLayer {
        hash: String,
        layer_type: LayerType,
        parent: Option<String>,
    },
}

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerCacheEntry {
    /// Layer type and metadata
    pub layer: CacheLayer,
    /// Content hash
    pub hash: String,
    /// Original size in bytes
    pub size: u64,
    /// Compressed size in bytes
    pub compressed_size: u64,
    /// Compression level used
    pub compression: CompressionLevel,
    /// Path to cached file
    pub path: PathBuf,
    /// Last access time
    pub accessed: SystemTime,
    /// Creation time
    pub created: SystemTime,
    /// Access count
    pub access_count: u64,
    /// Parent layer hash if any
    pub parent: Option<String>,
}

/// Cache size limits
#[derive(Debug, Clone)]
pub struct CacheSizeLimits {
    /// Maximum total size
    pub max_total_size: u64,
    /// Maximum size per layer type
    pub max_layer_sizes: HashMap<String, u64>,
    /// Target size after cleanup
    pub target_size: u64,
}

impl Default for CacheSizeLimits {
    fn default() -> Self {
        let mut max_layer_sizes = HashMap::new();
        // 1GB for packages
        max_layer_sizes.insert("package".to_string(), 1024 * 1024 * 1024);
        // 2GB for builds
        max_layer_sizes.insert("build".to_string(), 2 * 1024 * 1024 * 1024);
        // 5GB for environments
        max_layer_sizes.insert("environment".to_string(), 5 * 1024 * 1024 * 1024);
        // 100MB for resolutions
        max_layer_sizes.insert("resolution".to_string(), 100 * 1024 * 1024);
        // 10GB for image layers
        max_layer_sizes.insert("image".to_string(), 10 * 1024 * 1024 * 1024);

        Self {
            // 20GB total
            max_total_size: 20 * 1024 * 1024 * 1024,
            max_layer_sizes,
            // 80% of max size
            target_size: 16 * 1024 * 1024 * 1024,
        }
    }
}

/// Combined statistics for layered cache
#[derive(Debug, Clone)]
pub struct LayeredCacheStats {
    /// Memory cache statistics
    pub memory_stats: MemoryCacheStats,
    /// Disk cache statistics
    pub disk_stats: DiskCacheStats,
    /// Total cache size in bytes
    pub total_size: u64,
    /// Total number of cached items
    pub total_items: usize,
    /// Cache hit ratio (0.0-1.0)
    pub hit_ratio: f64,
}

/// Layered cache implementation
pub struct LayeredCache<M, D>
where
    M: CacheStorage + Send + Sync + ?Sized,
    D: CacheStorage + Send + Sync + ?Sized,
{
    memory: Arc<M>,
    disk: Arc<D>,
}

impl<M, D> LayeredCache<M, D>
where
    M: CacheStorage + Send + Sync + ?Sized,
    D: CacheStorage + Send + Sync + ?Sized,
{
    /// Create new layered cache
    pub fn new(memory: Arc<M>, disk: Arc<D>) -> Self {
        Self { memory, disk }
    }
}

#[async_trait]
impl<M, D> CacheStorage for LayeredCache<M, D>
where
    M: CacheStorage + Send + Sync + ?Sized,
    D: CacheStorage + Send + Sync + ?Sized,
{
    async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        // Store in both layers
        self.memory.store(hash, data).await?;
        self.disk.store(hash, data).await?;
        Ok(())
    }

    async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>> {
        // Try memory first
        match self.memory.load(hash).await {
            Ok(data) => Ok(data),
            Err(_) => {
                // Try disk and cache in memory if found
                let data = self.disk.load(hash).await?;
                let _ = self.memory.store(hash, &data).await;
                Ok(data)
            }
        }
    }

    async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()> {
        // Remove from both layers
        let _ = self.memory.remove(hash).await;
        self.disk.remove(hash).await?;
        Ok(())
    }

    async fn clear(&self) -> BlastResult<()> {
        // Clear both layers
        self.memory.clear().await?;
        self.disk.clear().await?;
        Ok(())
    }

    fn hash_path(&self, hash: &blake3::Hash) -> std::path::PathBuf {
        self.disk.hash_path(hash)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl LayerCacheEntry {
    // Removing unused method layer_type_str
} 