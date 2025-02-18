//! Caching system for the Blast Python environment manager.
//! 
//! This crate provides a high-performance, persistent caching system with
//! compression and atomic operations.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use hex;

use blast_core::error::{BlastError, BlastResult};

pub mod compression;
pub mod storage;
pub mod memory;
pub mod layered;
pub mod lru;
pub mod index;
pub mod disk;

use std::path::Path;
use memory::MemoryStorage;
use layered::LayeredCache;
use compression::CompressedStorage;
use lru::LRUCache;
use index::IndexedStorage;

// Re-export types
pub use layered::{CacheLayer, LayerType};
pub use compression::CompressionLevel;

pub use index::CacheIndex;
pub use storage::{CacheStorage, FileStorage};
pub use memory::{MemoryCache, MemoryCacheStats};

/// Wrapper for blake3::Hash that implements serialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// Cache builder for configuring cache options
pub struct CacheBuilder {
    path: Option<std::path::PathBuf>,
    memory_size: Option<usize>,
    compression: bool,
    indexed: bool,
}

impl CacheBuilder {
    /// Create new cache builder
    pub fn new() -> Self {
        Self {
            path: None,
            memory_size: None,
            compression: false,
            indexed: false,
        }
    }

    /// Set cache path
    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set memory cache size
    pub fn memory_size(mut self, size: usize) -> Self {
        self.memory_size = Some(size);
        self
    }

    /// Enable compression
    pub fn compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }

    /// Enable key indexing
    pub fn indexed(mut self, enabled: bool) -> Self {
        self.indexed = enabled;
        self
    }

    /// Build cache with current configuration
    pub async fn build(self) -> BlastResult<Cache> {
        let path = self.path.ok_or_else(|| {
            blast_core::error::BlastError::cache("Cache path not specified")
        })?;

        // Create base storage
        let disk = Arc::new(FileStorage::new(&path).await?);
        let memory = Arc::new(RwLock::new(MemoryStorage::new()));
        let layered = Arc::new(LayeredCache::new(memory, disk));

        // Add optional features
        let mut storage: Arc<dyn CacheStorage + Send + Sync> = layered;

        if self.compression {
            storage = Arc::new(CompressedStorage::new(storage));
        }

        if let Some(size) = self.memory_size {
            storage = Arc::new(LRUCache::new(storage, size));
        }

        if self.indexed {
            storage = Arc::new(IndexedStorage::new(storage));
        }

        Ok(Cache { storage })
    }
}

impl Default for CacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache interface
pub struct Cache {
    storage: Arc<dyn CacheStorage + Send + Sync>,
}

impl Cache {
    /// Store data in cache
    pub async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        self.storage.store(hash, data).await
    }

    /// Load data from cache
    pub async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>> {
        self.storage.load(hash).await
    }

    /// Remove data from cache
    pub async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()> {
        self.storage.remove(hash).await
    }

    /// Clear all cached data
    pub async fn clear(&self) -> BlastResult<()> {
        self.storage.clear().await
    }

    /// Store data with key
    pub async fn store_with_key(&self, key: &str, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        if let Some(indexed) = self.storage.as_any().downcast_ref::<IndexedStorage<dyn CacheStorage + Send + Sync>>() {
            indexed.store_with_key(key, hash, data).await
        } else {
            Err(blast_core::error::BlastError::cache("Indexing not enabled"))
        }
    }

    /// Load data by key
    pub async fn load_by_key(&self, key: &str) -> BlastResult<Vec<u8>> {
        if let Some(indexed) = self.storage.as_any().downcast_ref::<IndexedStorage<dyn CacheStorage + Send + Sync>>() {
            indexed.load_by_key(key).await
        } else {
            Err(blast_core::error::BlastError::cache("Indexing not enabled"))
        }
    }

    /// Remove data by key
    pub async fn remove_by_key(&self, key: &str) -> BlastResult<()> {
        if let Some(indexed) = self.storage.as_any().downcast_ref::<IndexedStorage<dyn CacheStorage + Send + Sync>>() {
            indexed.remove_by_key(key).await
        } else {
            Err(blast_core::error::BlastError::cache("Indexing not enabled"))
        }
    }

    /// Check if compression is enabled
    pub fn is_compression_enabled(&self) -> bool {
        self.storage.as_any().is::<CompressedStorage<dyn CacheStorage + Send + Sync>>()
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