use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use blast_core::error::BlastResult;
use crate::storage::CacheStorage;
use crate::SerializableHash;

/// Cache entry in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Content hash
    pub hash: SerializableHash,
    /// Original size in bytes
    pub size: u64,
    /// Compressed size in bytes
    pub compressed_size: u64,
    /// Path to cached file
    pub path: PathBuf,
    /// Last access time
    pub accessed: SystemTime,
    /// Creation time
    pub created: SystemTime,
}

/// Index of cached items
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheIndex {
    /// Cached entries by key
    entries: HashMap<String, CacheEntry>,
    /// Path to index file
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl CacheIndex {
    /// Load index from file or create new if not exists
    pub async fn load_or_create(cache_dir: impl AsRef<Path>) -> BlastResult<Self> {
        let path = cache_dir.as_ref().join("index.json");
        
        if path.exists() {
            let data = fs::read(&path).await?;
            let mut index: Self = serde_json::from_slice(&data)?;
            index.path = Some(path);
            Ok(index)
        } else {
            let mut index = Self::default();
            index.path = Some(path);
            index.save().await?;
            Ok(index)
        }
    }

    /// Save index to file
    pub async fn save(&self) -> BlastResult<()> {
        if let Some(path) = &self.path {
            let data = serde_json::to_vec_pretty(self)?;
            fs::write(path, data).await?;
        }
        Ok(())
    }

    /// Insert entry into index
    pub fn insert(&mut self, key: String, entry: CacheEntry) {
        self.entries.insert(key, entry);
    }

    /// Get entry by key
    pub fn get(&self, key: &str) -> Option<&CacheEntry> {
        self.entries.get(key)
    }

    /// Get mutable entry by key
    pub fn get_mut(&mut self, key: &str) -> Option<&mut CacheEntry> {
        self.entries.get_mut(key)
    }

    /// Remove entry by key
    pub fn remove(&mut self, key: &str) -> Option<CacheEntry> {
        self.entries.remove(key)
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Get total size of cached items
    pub fn total_size(&self) -> u64 {
        self.entries.values().map(|e| e.size).sum()
    }

    /// Get total compressed size
    pub fn total_compressed_size(&self) -> u64 {
        self.entries.values().map(|e| e.compressed_size).sum()
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        let total = self.total_size();
        let compressed = self.total_compressed_size();
        if total > 0 {
            compressed as f64 / total as f64
        } else {
            1.0
        }
    }
}

/// Storage wrapper that adds key-based indexing
pub struct IndexedStorage<S: CacheStorage + ?Sized> {
    inner: Arc<S>,
    index: RwLock<HashMap<String, blake3::Hash>>,
}

impl<S: CacheStorage + ?Sized> IndexedStorage<S> {
    /// Create new indexed storage
    pub fn new(inner: Arc<S>) -> Self {
        Self {
            inner,
            index: RwLock::new(HashMap::new()),
        }
    }

    /// Store data with key
    pub async fn store_with_key(&self, key: &str, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        self.inner.store(hash, data).await?;
        self.index.write().await.insert(key.to_string(), *hash);
        Ok(())
    }

    /// Load data by key
    pub async fn load_by_key(&self, key: &str) -> BlastResult<Vec<u8>> {
        let index = self.index.read().await;
        let hash = index.get(key).ok_or_else(|| {
            blast_core::error::BlastError::cache("Key not found")
        })?;
        self.inner.load(hash).await
    }

    /// Remove data by key
    pub async fn remove_by_key(&self, key: &str) -> BlastResult<()> {
        if let Some(hash) = self.index.write().await.remove(key) {
            self.inner.remove(&hash).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl<S: CacheStorage + ?Sized> CacheStorage for IndexedStorage<S> {
    async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        self.inner.store(hash, data).await
    }

    async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>> {
        self.inner.load(hash).await
    }

    async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()> {
        self.inner.remove(hash).await
    }

    async fn clear(&self) -> BlastResult<()> {
        self.inner.clear().await?;
        self.index.write().await.clear();
        Ok(())
    }

    fn hash_path(&self, hash: &blake3::Hash) -> PathBuf {
        self.inner.hash_path(hash)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
} 