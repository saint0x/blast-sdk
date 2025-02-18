use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::{HashMap, VecDeque};
use async_trait::async_trait;
use blast_core::error::BlastResult;
use crate::storage::CacheStorage;

/// LRU cache entry
#[derive(Clone)]
pub struct LruEntry<V> {
    /// Stored value
    pub value: V,
    /// When the entry was created
    pub created: Instant,
    /// Time-to-live duration
    pub ttl: Duration,
    /// Number of times this entry was accessed
    pub hits: u64,
}

/// LRU cache implementation
pub struct LruCache<K, V> {
    /// Maximum number of entries
    capacity: usize,
    /// Stored entries
    entries: HashMap<K, LruEntry<V>>,
    /// LRU ordering (most recently used at front)
    lru_list: VecDeque<K>,
}

impl<K, V> LruCache<K, V>
where
    K: Clone + Eq + std::hash::Hash,
{
    /// Create new LRU cache with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            lru_list: VecDeque::new(),
        }
    }

    /// Get entry by key
    pub fn get(&mut self, key: &K) -> Option<LruEntry<V>>
    where
        V: Clone,
    {
        // First check if the entry exists and clone it
        let entry = match self.entries.get(key) {
            Some(e) => e.clone(),
            None => return None,
        };

        // Check if expired
        if entry.created.elapsed() > entry.ttl {
            self.remove(key);
            return None;
        }

        // Update LRU order
        if let Some(pos) = self.lru_list.iter().position(|k| k == key) {
            self.lru_list.remove(pos);
            self.lru_list.push_front(key.clone());
        }

        // Update hit count
        if let Some(entry) = self.entries.get_mut(key) {
            entry.hits += 1;
        }

        Some(entry)
    }

    /// Put entry in cache
    pub fn put(&mut self, key: K, value: V, ttl: Duration) {
        let entry = LruEntry {
            value,
            created: Instant::now(),
            ttl,
            hits: 0,
        };

        // Remove oldest if at capacity
        if self.entries.len() >= self.capacity {
            if let Some(lru_key) = self.lru_list.pop_back() {
                self.entries.remove(&lru_key);
            }
        }

        // Add new entry
        self.entries.insert(key.clone(), entry);
        self.lru_list.push_front(key);
    }

    /// Remove entry by key
    pub fn remove(&mut self, key: &K) -> Option<LruEntry<V>> {
        if let Some(pos) = self.lru_list.iter().position(|k| k == key) {
            self.lru_list.remove(pos);
        }
        self.entries.remove(key)
    }

    /// Remove and return least recently used entry
    pub fn pop_lru(&mut self) -> Option<(K, LruEntry<V>)> {
        if let Some(key) = self.lru_list.pop_back() {
            if let Some(entry) = self.entries.remove(&key) {
                return Some((key, entry));
            }
        }
        None
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru_list.clear();
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get iterator over entries
    pub fn iter(&self) -> impl Iterator<Item = (&K, &LruEntry<V>)> {
        self.entries.iter()
    }
}

/// LRU cache storage implementation
pub struct LRUCache<S: CacheStorage + ?Sized> {
    inner: Arc<S>,
    capacity: usize,
    items: lru::LruCache<blake3::Hash, ()>,
}

impl<S: CacheStorage + ?Sized> LRUCache<S> {
    /// Create new LRU cache with given capacity
    pub fn new(inner: Arc<S>, capacity: usize) -> Self {
        Self {
            inner,
            capacity,
            items: lru::LruCache::new(std::num::NonZeroUsize::new(capacity).unwrap()),
        }
    }
}

#[async_trait]
impl<S: CacheStorage + Send + Sync + ?Sized> CacheStorage for LRUCache<S> {
    async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        // Store in inner storage
        self.inner.store(hash, data).await?;

        // Update LRU cache
        let mut items = self.items.clone();
        if items.len() >= self.capacity {
            if let Some((old_hash, _)) = items.pop_lru() {
                let _ = self.inner.remove(&old_hash).await;
            }
        }
        items.put(*hash, ());

        Ok(())
    }

    async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>> {
        // Update LRU status
        let mut items = self.items.clone();
        if items.get(hash).is_some() {
            let data = self.inner.load(hash).await?;
            items.promote(hash);
            Ok(data)
        } else {
            Err(blast_core::error::BlastError::cache("Data not found"))
        }
    }

    async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()> {
        let mut items = self.items.clone();
        items.pop(hash);
        self.inner.remove(hash).await
    }

    async fn clear(&self) -> BlastResult<()> {
        let mut items = self.items.clone();
        items.clear();
        self.inner.clear().await
    }

    fn hash_path(&self, hash: &blake3::Hash) -> std::path::PathBuf {
        self.inner.hash_path(hash)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
} 