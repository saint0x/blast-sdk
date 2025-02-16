use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use blast_core::error::{BlastError, BlastResult};

use crate::compression::CompressionLevel;
use crate::storage::CacheStorage;

/// Type of image layer
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Cache layer types
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
        timestamp: DateTime<Utc>,
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

/// Layered cache implementation
pub struct LayeredCache {
    /// Cache storage backend
    storage: Arc<dyn CacheStorage>,
    /// Cache index
    index: Arc<RwLock<LayeredCacheIndex>>,
    /// Size limits
    limits: CacheSizeLimits,
    /// Root path
    root: PathBuf,
}

/// Cache index for layered storage
#[derive(Debug, Serialize, Deserialize)]
pub struct LayeredCacheIndex {
    /// Layer entries by hash
    entries: HashMap<String, LayerCacheEntry>,
    /// LRU tracking (most recently used at the front)
    lru_list: Vec<String>,
    /// Total size
    total_size: u64,
    /// Size by layer type
    layer_sizes: HashMap<String, u64>,
}

impl LayeredCache {
    /// Create a new layered cache
    pub async fn new(root: PathBuf, storage: Arc<dyn CacheStorage>) -> BlastResult<Self> {
        let index = LayeredCacheIndex::load_or_create(&root).await?;
        Ok(Self {
            storage,
            index: Arc::new(RwLock::new(index)),
            limits: CacheSizeLimits::default(),
            root,
        })
    }

    /// Store a layer in the cache
    pub async fn store_layer(&self, layer: CacheLayer, data: &[u8]) -> BlastResult<()> {
        let hash = blake3::hash(data);
        let hex_hash = hash.to_hex().to_string();

        // Compress data with appropriate level
        let compression = match &layer {
            CacheLayer::Package { .. } => CompressionLevel::Best,
            CacheLayer::Build { .. } => CompressionLevel::Default,
            CacheLayer::Environment { .. } => CompressionLevel::Fast,
            CacheLayer::Resolution { .. } => CompressionLevel::Best,
            CacheLayer::ImageLayer { .. } => CompressionLevel::Default,
        };

        let compressed = crate::compression::compress(data, compression)?;

        // Store compressed data
        self.storage.store(&hash, &compressed).await?;

        // Update index
        let mut index = self.index.write().await;
        let entry = LayerCacheEntry {
            layer,
            hash: hex_hash.clone(),
            size: data.len() as u64,
            compressed_size: compressed.len() as u64,
            compression,
            path: self.storage.hash_path(&hash),
            accessed: SystemTime::now(),
            created: SystemTime::now(),
            access_count: 0,
            parent: None,
        };

        // Update size tracking
        index.total_size += entry.compressed_size;
        let layer_type = entry.layer_type_str();
        *index.layer_sizes.entry(layer_type).or_insert(0) += entry.compressed_size;

        // Add to LRU list
        index.lru_list.insert(0, hex_hash.clone());
        index.entries.insert(hex_hash, entry);

        // Check size limits and evict if needed
        self.evict_if_needed(&mut index).await?;

        // Save index
        index.save(&self.root).await?;

        Ok(())
    }

    /// Get a layer from the cache
    pub async fn get_layer(&self, hash: &str) -> BlastResult<Option<(CacheLayer, Vec<u8>)>> {
        let mut index = self.index.write().await;
        
        // Clone the layer data we need
        let entry_data = if let Some(entry) = index.entries.get(hash) {
            Some((entry.layer.clone(), entry.hash.clone()))
        } else {
            None
        };

        if let Some((layer, entry_hash)) = entry_data {
            // Update access time and count
            if let Some(entry) = index.entries.get_mut(hash) {
                entry.accessed = SystemTime::now();
                entry.access_count += 1;
            }

            // Move to front of LRU list
            index.move_to_front(hash);

            // Load and decompress data
            let hash_bytes = hex::decode(hash)
                .map_err(|e| BlastError::cache(format!("Invalid hash: {}", e)))?;
            let blake_hash = blake3::Hash::from_bytes(hash_bytes.try_into().unwrap());
            
            let compressed = self.storage.load(&blake_hash).await?;
            let data = crate::compression::decompress(&compressed)?;

            // Verify hash
            let computed_hash = blake3::hash(&data);
            if computed_hash.to_hex().to_string() != entry_hash {
                warn!("Cache corruption detected for hash: {}", entry_hash);
                // Use the string hash for removal
                self.remove_layer(hash).await?;
                return Ok(None);
            }

            Ok(Some((layer, data)))
        } else {
            Ok(None)
        }
    }

    /// Remove a layer from the cache
    pub async fn remove_layer(&self, hash: &str) -> BlastResult<()> {
        let mut index = self.index.write().await;
        
        if let Some(entry) = index.entries.remove(hash) {
            // Update size tracking
            index.total_size -= entry.compressed_size;
            let layer_type = entry.layer_type_str();
            if let Some(size) = index.layer_sizes.get_mut(&layer_type) {
                *size -= entry.compressed_size;
            }

            // Remove from LRU list
            index.remove_lru();

            // Remove from storage
            let hash_bytes = hex::decode(hash)
                .map_err(|e| BlastError::cache(format!("Invalid hash: {}", e)))?;
            let hash = blake3::Hash::from_bytes(hash_bytes.try_into().unwrap());
            self.storage.remove(&hash).await?;

            // Save index
            index.save(&self.root).await?;
        }

        Ok(())
    }

    /// Clear all layers from the cache
    pub async fn clear(&self) -> BlastResult<()> {
        let mut index = self.index.write().await;
        index.entries.clear();
        index.lru_list.clear();
        index.total_size = 0;
        index.layer_sizes.clear();
        index.save(&self.root).await?;
        self.storage.clear().await?;
        Ok(())
    }

    /// Get cache statistics
    pub async fn stats(&self) -> BlastResult<LayeredCacheStats> {
        let index = self.index.read().await;
        Ok(LayeredCacheStats {
            total_entries: index.entries.len(),
            total_size: index.total_size,
            layer_sizes: index.layer_sizes.clone(),
            compression_ratio: index.compression_ratio(),
        })
    }

    /// Evict entries if size limits are exceeded
    async fn evict_if_needed(&self, index: &mut LayeredCacheIndex) -> BlastResult<()> {
        // Check total size
        if index.total_size > self.limits.max_total_size {
            debug!("Cache size {} exceeds limit {}, evicting entries", 
                index.total_size, self.limits.max_total_size);
            
            while index.total_size > self.limits.target_size {
                if let Some(hash) = index.remove_lru() {
                    if let Some(entry) = index.entries.remove(&hash) {
                        index.total_size -= entry.compressed_size;
                        let layer_type = entry.layer_type_str();
                        if let Some(size) = index.layer_sizes.get_mut(&layer_type) {
                            *size -= entry.compressed_size;
                        }

                        let hash_bytes = hex::decode(&hash)
                            .map_err(|e| BlastError::cache(format!("Invalid hash: {}", e)))?;
                        let hash = blake3::Hash::from_bytes(hash_bytes.try_into().unwrap());
                        self.storage.remove(&hash).await?;
                    }
                } else {
                    break;
                }
            }
        }

        // Check per-layer limits
        let mut layers_to_evict = Vec::new();
        
        // First, collect all layers that need eviction
        for (layer_type, &size) in &index.layer_sizes {
            if let Some(&limit) = self.limits.max_layer_sizes.get(layer_type) {
                if size > limit {
                    layers_to_evict.push((layer_type.clone(), (limit as f64 * 0.8) as u64));
                }
            }
        }

        // Then process evictions
        for (layer_type, target_size) in layers_to_evict {
            debug!("Layer {} size exceeds limit, evicting entries", layer_type);
            
            let mut current_size = *index.layer_sizes.get(&layer_type).unwrap_or(&0);
            while current_size > target_size {
                if let Some(hash) = index.remove_lru() {
                    if let Some(entry) = index.entries.get(&hash) {
                        if entry.layer_type_str() == layer_type {
                            if let Some(entry) = index.entries.remove(&hash) {
                                current_size -= entry.compressed_size;
                                index.total_size -= entry.compressed_size;
                                if let Some(size) = index.layer_sizes.get_mut(&layer_type) {
                                    *size -= entry.compressed_size;
                                }

                                let hash_bytes = hex::decode(&hash)
                                    .map_err(|e| BlastError::cache(format!("Invalid hash: {}", e)))?;
                                let hash = blake3::Hash::from_bytes(hash_bytes.try_into().unwrap());
                                self.storage.remove(&hash).await?;
                            }
                        } else {
                            // Re-add to front if not of target type
                            index.lru_list.insert(0, hash);
                        }
                    }
                } else {
                    break;
                }
            }
        }

        Ok(())
    }
}

impl LayerCacheEntry {
    fn layer_type_str(&self) -> String {
        match &self.layer {
            CacheLayer::Package { .. } => "package",
            CacheLayer::Build { .. } => "build",
            CacheLayer::Environment { .. } => "environment",
            CacheLayer::Resolution { .. } => "resolution",
            CacheLayer::ImageLayer { .. } => "image",
        }.to_string()
    }
}

impl LayeredCacheIndex {
    /// Load or create cache index
    async fn load_or_create(root: &PathBuf) -> BlastResult<Self> {
        let index_path = root.join("index.json");
        if index_path.exists() {
            let contents = tokio::fs::read_to_string(&index_path)
                .await
                .map_err(|e| BlastError::cache(format!("Failed to read index: {}", e)))?;
            serde_json::from_str(&contents)
                .map_err(|e| BlastError::cache(format!("Failed to parse index: {}", e)))
        } else {
            Ok(Self {
                entries: HashMap::new(),
                lru_list: Vec::new(),
                total_size: 0,
                layer_sizes: HashMap::new(),
            })
        }
    }

    /// Save cache index
    async fn save(&self, root: &PathBuf) -> BlastResult<()> {
        let index_path = root.join("index.json");
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| BlastError::cache(format!("Failed to serialize index: {}", e)))?;
        tokio::fs::write(&index_path, contents)
            .await
            .map_err(|e| BlastError::cache(format!("Failed to write index: {}", e)))?;
        Ok(())
    }

    /// Calculate compression ratio
    fn compression_ratio(&self) -> f64 {
        if self.total_size == 0 {
            0.0
        } else {
            let total_uncompressed: u64 = self.entries.values()
                .map(|e| e.size)
                .sum();
            self.total_size as f64 / total_uncompressed as f64
        }
    }

    /// Move entry to front of LRU list
    fn move_to_front(&mut self, hash: &str) {
        if let Some(pos) = self.lru_list.iter().position(|h| h == hash) {
            self.lru_list.remove(pos);
            self.lru_list.insert(0, hash.to_string());
        }
    }

    /// Remove least recently used entry
    fn remove_lru(&mut self) -> Option<String> {
        self.lru_list.pop()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct LayeredCacheStats {
    pub total_entries: usize,
    pub total_size: u64,
    pub layer_sizes: HashMap<String, u64>,
    pub compression_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::storage::FileStorage;

    #[tokio::test]
    async fn test_layered_cache() {
        let dir = tempdir().unwrap();
        let storage = Arc::new(FileStorage::new(dir.path()).await.unwrap());
        let cache = LayeredCache::new(dir.path().to_path_buf(), storage).await.unwrap();

        // Test package layer
        let package_layer = CacheLayer::Package {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            hash: "test-hash".to_string(),
        };
        let package_data = b"test package data".to_vec();
        cache.store_layer(package_layer.clone(), &package_data).await.unwrap();

        // Test retrieval
        let (retrieved_layer, retrieved_data) = cache.get_layer(&blake3::hash(&package_data)
            .to_hex().to_string()).await.unwrap().unwrap();
        assert!(matches!(retrieved_layer, CacheLayer::Package { .. }));
        assert_eq!(retrieved_data, package_data);

        // Test environment layer
        let env_layer = CacheLayer::Environment {
            name: "test-env".to_string(),
            python_version: "3.8".to_string(),
            timestamp: Utc::now(),
        };
        let env_data = b"test environment data".to_vec();
        cache.store_layer(env_layer, &env_data).await.unwrap();

        // Test stats
        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.total_entries, 2);
        assert!(stats.compression_ratio > 0.0);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let dir = tempdir().unwrap();
        let storage = Arc::new(FileStorage::new(dir.path()).await.unwrap());
        let mut cache = LayeredCache::new(dir.path().to_path_buf(), storage).await.unwrap();

        // Override limits for testing
        cache.limits = CacheSizeLimits {
            max_total_size: 100,
            max_layer_sizes: {
                let mut m = HashMap::new();
                m.insert("package".to_string(), 50);
                m
            },
            target_size: 80,
        };

        // Add entries until eviction occurs
        for i in 0..10 {
            let layer = CacheLayer::Package {
                name: format!("package-{}", i),
                version: "1.0.0".to_string(),
                hash: format!("hash-{}", i),
            };
            let data = vec![0u8; 20]; // 20 bytes each
            cache.store_layer(layer, &data).await.unwrap();
        }

        let stats = cache.stats().await.unwrap();
        assert!(stats.total_size <= cache.limits.max_total_size);
        assert!(stats.layer_sizes.get("package").unwrap() <= &cache.limits.max_layer_sizes["package"]);
    }
} 