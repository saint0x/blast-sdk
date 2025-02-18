use std::path::Path;
use blast_core::error::BlastResult;
use crate::storage::{CacheStorage, FileStorage};

/// Statistics for disk cache
#[derive(Debug, Clone, Default)]
pub struct DiskCacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Total size in bytes
    pub total_size: u64,
    /// Number of items in cache
    pub items: usize,
}

/// Disk-based cache implementation
pub struct DiskCache {
    storage: FileStorage,
    stats: DiskCacheStats,
}

impl DiskCache {
    /// Create new disk cache at given path
    pub async fn new(path: impl AsRef<Path>, _max_size: u64) -> BlastResult<Self> {
        Ok(Self {
            storage: FileStorage::new(path).await?,
            stats: DiskCacheStats::default(),
        })
    }

    /// Get layer data by hash
    pub async fn get_layer(&mut self, hash: &str) -> BlastResult<Option<Vec<u8>>> {
        let hash = blake3::hash(hash.as_bytes());
        match CacheStorage::load(&self.storage, &hash).await {
            Ok(data) => {
                self.stats.hits += 1;
                Ok(Some(data))
            }
            Err(_) => {
                self.stats.misses += 1;
                Ok(None)
            }
        }
    }

    /// Store layer data with hash
    pub async fn put_layer(&mut self, hash: &str, data: Vec<u8>) -> BlastResult<()> {
        let hash = blake3::hash(hash.as_bytes());
        CacheStorage::store(&self.storage, &hash, &data).await?;
        self.stats.total_size += data.len() as u64;
        self.stats.items += 1;
        Ok(())
    }

    /// Remove layer by hash
    pub async fn remove_layer(&self, hash: &str) -> BlastResult<()> {
        let hash = blake3::hash(hash.as_bytes());
        CacheStorage::remove(&self.storage, &hash).await
    }

    /// Clean up expired or invalid layers
    pub async fn cleanup(&self) -> BlastResult<()> {
        CacheStorage::clear(&self.storage).await
    }

    /// Get cache statistics
    pub fn stats(&self) -> DiskCacheStats {
        self.stats.clone()
    }
} 