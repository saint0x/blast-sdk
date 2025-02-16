//! Caching system for the Blast Python environment manager.
//! 
//! This crate provides a high-performance, persistent caching system with
//! compression and atomic operations.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::error;
use hex;

use blast_core::error::{BlastError, BlastResult};
use blast_core::types::CacheSettings;

mod compression;
mod index;
mod storage;
mod layered;

pub use compression::CompressionLevel;
pub use index::CacheIndex;
pub use storage::{CacheStorage, FileStorage};
pub use layered::{
    LayeredCache,
    CacheLayer,
    LayerCacheEntry,
    CacheSizeLimits,
    LayeredCacheStats,
};

/// Wrapper for blake3::Hash that implements serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableHash(String);

impl From<blake3::Hash> for SerializableHash {
    fn from(hash: blake3::Hash) -> Self {
        Self(hash.to_hex().to_string())
    }
}

impl TryFrom<SerializableHash> for blake3::Hash {
    type Error = BlastError;

    fn try_from(hash: SerializableHash) -> Result<Self, Self::Error> {
        let bytes = hex::decode(&hash.0)
            .map_err(|e| BlastError::cache(format!("Invalid hash: {}", e)))?;
        Ok(blake3::Hash::from_bytes(bytes.try_into().unwrap()))
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// Content hash
    hash: SerializableHash,
    /// Original size in bytes
    size: u64,
    /// Compressed size in bytes
    compressed_size: u64,
    /// Path to cached file
    path: PathBuf,
    /// Last access time
    accessed: SystemTime,
    /// Creation time
    created: SystemTime,
}

/// High-performance cache for Python packages and environments
pub struct Cache {
    settings: CacheSettings,
    storage: Arc<dyn CacheStorage>,
    index: Arc<RwLock<CacheIndex>>,
}

impl Cache {
    /// Create a new cache with the given settings
    pub async fn new(settings: CacheSettings) -> BlastResult<Self> {
        let storage = Arc::new(FileStorage::new(&settings.cache_dir).await?);
        let index = Arc::new(RwLock::new(CacheIndex::load_or_create(&settings.cache_dir).await?));

        Ok(Self {
            settings,
            storage,
            index,
        })
    }

    /// Store data in the cache with the given key
    pub async fn store(&self, key: &str, data: &[u8]) -> BlastResult<()> {
        // Calculate hash of data
        let hash = blake3::hash(data);
        
        // Compress data
        let compressed = compression::compress(
            data,
            CompressionLevel::default(),
        )?;

        // Store compressed data
        self.storage.store(&hash, &compressed).await?;

        // Update index
        let mut index = self.index.write().await;
        index.insert(
            key.to_string(),
            index::CacheEntry {
                hash: SerializableHash::from(hash),
                size: data.len() as u64,
                compressed_size: compressed.len() as u64,
                path: self.storage.hash_path(&hash),
                accessed: SystemTime::now(),
                created: SystemTime::now(),
            },
        );
        index.save().await?;

        Ok(())
    }

    /// Retrieve data from the cache
    pub async fn get(&self, key: &str) -> BlastResult<Option<Vec<u8>>> {
        let entry = {
            let mut index = self.index.write().await;
            if let Some(entry) = index.get_mut(key) {
                entry.accessed = SystemTime::now();
                entry.clone()
            } else {
                return Ok(None);
            }
        };

        // Check if entry has expired
        if let Ok(age) = entry.created.elapsed() {
            if age > self.settings.ttl {
                self.remove(key).await?;
                return Ok(None);
            }
        }

        // Load and decompress data
        let entry_hash: blake3::Hash = entry.hash.try_into()?;
        let compressed = self.storage.load(&entry_hash).await?;
        let data = compression::decompress(&compressed)?;

        // Verify hash
        let hash = blake3::hash(&data);
        if hash != entry_hash {
            error!("Cache corruption detected for key: {}", key);
            self.remove(key).await?;
            return Ok(None);
        }

        Ok(Some(data))
    }

    /// Remove an entry from the cache
    pub async fn remove(&self, key: &str) -> BlastResult<()> {
        let entry = {
            let mut index = self.index.write().await;
            index.remove(key)
        };

        if let Some(entry) = entry {
            let hash: blake3::Hash = entry.hash.try_into()?;
            self.storage.remove(&hash).await?;
        }

        Ok(())
    }

    /// Clear all entries from the cache
    pub async fn clear(&self) -> BlastResult<()> {
        let mut index = self.index.write().await;
        index.clear();
        index.save().await?;
        self.storage.clear().await?;
        Ok(())
    }

    /// Get cache statistics
    pub async fn stats(&self) -> BlastResult<CacheStats> {
        let index = self.index.read().await;
        Ok(CacheStats {
            total_entries: index.len(),
            total_size: index.total_size(),
            total_compressed_size: index.total_compressed_size(),
            compression_ratio: index.compression_ratio(),
        })
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size: u64,
    pub total_compressed_size: u64,
    pub compression_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_basic_cache_operations() {
        let dir = tempdir().unwrap();
        let settings = CacheSettings {
            cache_dir: dir.path().to_path_buf(),
            ttl: Duration::from_secs(3600),
            ..Default::default()
        };

        let cache = Cache::new(settings).await.unwrap();

        // Store data
        let key = "test-key";
        let data = b"test data".to_vec();
        cache.store(key, &data).await.unwrap();

        // Retrieve data
        let retrieved = cache.get(key).await.unwrap().unwrap();
        assert_eq!(retrieved, data);

        // Remove data
        cache.remove(key).await.unwrap();
        assert!(cache.get(key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let dir = tempdir().unwrap();
        let settings = CacheSettings {
            cache_dir: dir.path().to_path_buf(),
            ttl: Duration::from_secs(0), // Immediate expiration
            ..Default::default()
        };

        let cache = Cache::new(settings).await.unwrap();

        let key = "test-key";
        let data = b"test data".to_vec();
        cache.store(key, &data).await.unwrap();

        // Data should be expired
        assert!(cache.get(key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cache_corruption() {
        let dir = tempdir().unwrap();
        let settings = CacheSettings {
            cache_dir: dir.path().to_path_buf(),
            ttl: Duration::from_secs(3600),
            ..Default::default()
        };

        let cache = Cache::new(settings).await.unwrap();

        let key = "test-key";
        let data = b"test data".to_vec();
        cache.store(key, &data).await.unwrap();

        // Corrupt the data
        let entry = cache.index.read().await.get(key).unwrap().clone();
        fs::write(&entry.path, b"corrupted data").await.unwrap();

        // Corrupted data should be detected and removed
        assert!(cache.get(key).await.unwrap().is_none());
    }
} 