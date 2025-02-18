use std::time::{Duration, SystemTime};
use std::num::NonZeroUsize;
use tokio::sync::RwLock;
use lru::LruCache;
use blast_core::error::BlastResult;
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use blast_core::error::BlastError;
use crate::storage::CacheStorage;

/// Memory cache statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryCacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// Number of items in cache
    pub items: usize,
    /// Number of evicted items
    pub evictions: u64,
}

/// Memory cache entry with TTL
#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    created: SystemTime,
    ttl: Option<Duration>,
}

/// Memory-based cache implementation
pub struct MemoryCache<K, V> {
    cache: RwLock<LruCache<K, CacheEntry<V>>>,
    stats: RwLock<MemoryCacheStats>,
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> MemoryCache<K, V> {
    /// Create new memory cache with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: RwLock::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())),
            stats: RwLock::new(MemoryCacheStats::default()),
        }
    }

    /// Get value by key
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        
        if let Some(entry) = cache.get(key) {
            if let Some(ttl) = entry.ttl {
                if SystemTime::now().duration_since(entry.created).unwrap() > ttl {
                    cache.pop(key);
                    stats.misses += 1;
                    stats.evictions += 1;
                    return None;
                }
            }
            stats.hits += 1;
            Some(entry.value.clone())
        } else {
            stats.misses += 1;
            None
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> MemoryCacheStats {
        self.stats.read().await.clone()
    }

    /// Put value with key
    pub async fn put(&self, key: K, value: V, ttl: Option<Duration>) -> BlastResult<()> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        
        // Track evictions
        let old_len = cache.len();
        cache.put(key, CacheEntry {
            value,
            created: SystemTime::now(),
            ttl,
        });
        if cache.len() <= old_len {
            stats.evictions += 1;
        }
        
        stats.items = cache.len();
        stats.memory_usage = std::mem::size_of::<CacheEntry<V>>() * cache.len();
        Ok(())
    }

    /// Remove value by key
    pub async fn remove(&self, key: &K) -> BlastResult<()> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        if cache.pop(key).is_some() {
            stats.evictions += 1;
        }
        Ok(())
    }

    /// Clear all entries
    pub async fn clear(&self) -> BlastResult<()> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        stats.evictions += cache.len() as u64;
        cache.clear();
        Ok(())
    }

    /// Remove expired entries
    pub async fn cleanup(&self) -> BlastResult<()> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        let now = SystemTime::now();
        let expired: Vec<_> = cache.iter()
            .filter(|(_, entry)| {
                entry.ttl.map_or(false, |ttl| {
                    now.duration_since(entry.created).unwrap() > ttl
                })
            })
            .map(|(k, _)| k.clone())
            .collect();
        
        stats.evictions += expired.len() as u64;
        for key in expired {
            cache.pop(&key);
        }
        Ok(())
    }
}

/// In-memory storage backend
#[derive(Debug)]
pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<blake3::Hash, Vec<u8>>>>,
    size_limit: Option<usize>,
    current_size: usize,
}

impl MemoryStorage {
    /// Create new memory storage
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            size_limit: None,
            current_size: 0,
        }
    }

    /// Set size limit in bytes
    pub fn set_size_limit(&mut self, limit: usize) {
        self.size_limit = Some(limit);
    }

    /// Get current size in bytes
    pub fn size(&self) -> usize {
        self.current_size
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CacheStorage for MemoryStorage {
    async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        let mut storage = self.data.write().await;
        
        // Check size limit
        if let Some(limit) = self.size_limit {
            if data.len() > limit {
                return Err(BlastError::cache("Data exceeds size limit"));
            }
        }

        storage.insert(*hash, data.to_vec());
        Ok(())
    }

    async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>> {
        let storage = self.data.read().await;
        storage
            .get(hash)
            .cloned()
            .ok_or_else(|| BlastError::cache("Data not found"))
    }

    async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()> {
        let mut storage = self.data.write().await;
        storage.remove(hash);
        Ok(())
    }

    async fn clear(&self) -> BlastResult<()> {
        let mut storage = self.data.write().await;
        storage.clear();
        Ok(())
    }

    fn hash_path(&self, _hash: &blake3::Hash) -> std::path::PathBuf {
        // Memory storage doesn't use paths
        std::path::PathBuf::new()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl CacheStorage for RwLock<MemoryStorage> {
    async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        self.write().await.store(hash, data).await
    }

    async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>> {
        self.read().await.load(hash).await
    }

    async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()> {
        self.write().await.remove(hash).await
    }

    async fn clear(&self) -> BlastResult<()> {
        self.write().await.clear().await
    }

    fn hash_path(&self, _hash: &blake3::Hash) -> std::path::PathBuf {
        self.blocking_read().hash_path(_hash)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
} 