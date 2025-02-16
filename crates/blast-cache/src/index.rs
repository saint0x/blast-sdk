use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::fs;

use blast_core::error::{BlastError, BlastResult};

use crate::SerializableHash;

/// Cache index for tracking stored entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheIndex {
    /// Path to the index file
    path: PathBuf,
    /// Cached entries
    entries: HashMap<String, CacheEntry>,
    /// Last modified time
    last_modified: SystemTime,
}

/// Cache entry metadata
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

/// Cache index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Content hash
    pub hash: SerializableHash,
    /// Original size in bytes
    pub size: u64,
    /// Compressed size in bytes
    pub compressed_size: u64,
    /// Last access time
    pub accessed: SystemTime,
    /// Creation time
    pub created: SystemTime,
}

impl CacheIndex {
    /// Create a new cache index
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            entries: HashMap::new(),
            last_modified: SystemTime::now(),
        }
    }

    /// Load an existing index or create a new one
    pub async fn load_or_create(cache_dir: impl AsRef<Path>) -> BlastResult<Self> {
        let path = cache_dir.as_ref().join("index.json");
        
        if path.exists() {
            let data = fs::read(&path)
                .await
                .map_err(|e| BlastError::cache(format!("Failed to read index: {}", e)))?;
            
            serde_json::from_slice(&data)
                .map_err(|e| BlastError::cache(format!("Failed to parse index: {}", e)))
        } else {
            Ok(Self::new(path))
        }
    }

    /// Save the index to disk
    pub async fn save(&self) -> BlastResult<()> {
        let data = serde_json::to_vec_pretty(self)
            .map_err(|e| BlastError::cache(format!("Failed to serialize index: {}", e)))?;
        
        // Write atomically using a temporary file
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, &data)
            .await
            .map_err(|e| BlastError::cache(format!("Failed to write index: {}", e)))?;
        
        fs::rename(&temp_path, &self.path)
            .await
            .map_err(|e| BlastError::cache(format!("Failed to rename index: {}", e)))?;
        
        Ok(())
    }

    /// Insert an entry into the index
    pub fn insert(&mut self, key: String, entry: CacheEntry) {
        self.entries.insert(key, entry);
        self.last_modified = SystemTime::now();
    }

    /// Get an entry from the index
    pub fn get(&self, key: &str) -> Option<&CacheEntry> {
        self.entries.get(key)
    }

    /// Get a mutable reference to an entry
    pub fn get_mut(&mut self, key: &str) -> Option<&mut CacheEntry> {
        self.entries.get_mut(key)
    }

    /// Remove an entry from the index
    pub fn remove(&mut self, key: &str) -> Option<CacheEntry> {
        let entry = self.entries.remove(key);
        if entry.is_some() {
            self.last_modified = SystemTime::now();
        }
        entry
    }

    /// Clear all entries from the index
    pub fn clear(&mut self) {
        self.entries.clear();
        self.last_modified = SystemTime::now();
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the total size of all entries
    pub fn total_size(&self) -> u64 {
        self.entries.values().map(|e| e.size).sum()
    }

    /// Get the total compressed size of all entries
    pub fn total_compressed_size(&self) -> u64 {
        self.entries.values().map(|e| e.compressed_size).sum()
    }

    /// Get the compression ratio
    pub fn compression_ratio(&self) -> f64 {
        let total_size = self.total_size();
        let total_compressed = self.total_compressed_size();
        
        if total_size == 0 {
            1.0
        } else {
            total_compressed as f64 / total_size as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_index_persistence() {
        let dir = tempdir().unwrap();
        let mut index = CacheIndex::load_or_create(dir.path()).await.unwrap();

        // Add some entries
        for i in 0..5 {
            let key = format!("key-{}", i);
            let entry = CacheEntry {
                hash: SerializableHash(blake3::hash(key.as_bytes())),
                size: 100,
                compressed_size: 50,
                path: PathBuf::from(format!("file-{}", i)),
                accessed: SystemTime::now(),
                created: SystemTime::now(),
            };
            index.insert(key, entry);
        }

        // Save and reload
        index.save().await.unwrap();
        let loaded = CacheIndex::load_or_create(dir.path()).await.unwrap();

        assert_eq!(index.len(), loaded.len());
        assert_eq!(index.total_size(), loaded.total_size());
        assert_eq!(index.compression_ratio(), loaded.compression_ratio());
    }

    #[tokio::test]
    async fn test_index_operations() {
        let dir = tempdir().unwrap();
        let mut index = CacheIndex::load_or_create(dir.path()).await.unwrap();

        // Insert
        let key = "test-key";
        let entry = CacheEntry {
            hash: blake3::hash(b"test"),
            size: 100,
            compressed_size: 50,
            path: PathBuf::from("test-file"),
            accessed: SystemTime::now(),
            created: SystemTime::now(),
        };
        index.insert(key.to_string(), entry.clone());

        // Get
        let retrieved = index.get(key).unwrap();
        assert_eq!(retrieved.hash, entry.hash);

        // Remove
        let removed = index.remove(key).unwrap();
        assert_eq!(removed.hash, entry.hash);
        assert!(index.get(key).is_none());

        // Clear
        index.clear();
        assert!(index.is_empty());
    }

    #[tokio::test]
    async fn test_compression_stats() {
        let dir = tempdir().unwrap();
        let mut index = CacheIndex::load_or_create(dir.path()).await.unwrap();

        // Add entries with different compression ratios
        for i in 0..3 {
            let key = format!("key-{}", i);
            let entry = CacheEntry {
                hash: blake3::hash(key.as_bytes()),
                size: 100,
                compressed_size: 50 + i * 10, // Different compression ratios
                path: PathBuf::from(format!("file-{}", i)),
                accessed: SystemTime::now(),
                created: SystemTime::now(),
            };
            index.insert(key, entry);
        }

        assert_eq!(index.total_size(), 300);
        assert_eq!(index.total_compressed_size(), 180);
        assert!((index.compression_ratio() - 0.6).abs() < f64::EPSILON);
    }
} 