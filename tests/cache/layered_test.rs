use std::sync::Arc;
use tokio::sync::RwLock;
use blast_cache::layered::LayeredCache;
use blast_cache::memory::MemoryStorage;
use blast_cache::storage::FileStorage;
use tempfile::tempdir;
use blast_cache::layered::{LayerType, CacheLayer};
use blast_image::blake3;

#[tokio::test]
async fn test_layered_cache_operations() {
    let dir = tempdir().unwrap();
    let memory = Arc::new(RwLock::new(MemoryStorage::new()));
    let disk = Arc::new(FileStorage::new(dir.path()).await.unwrap());
    
    let cache = LayeredCache::new(memory.clone(), disk);

    // Test data
    let data = b"test data";
    let hash = blake3::hash(data);

    // Store in cache
    cache.store(&hash, data).await.unwrap();

    // Load from memory
    let loaded = cache.load(&hash).await.unwrap();
    assert_eq!(loaded, data);

    // Clear memory and load from disk
    memory.write().await.clear().await.unwrap();
    let loaded = cache.load(&hash).await.unwrap();
    assert_eq!(loaded, data);

    // Remove from cache
    cache.remove(&hash).await.unwrap();
    assert!(cache.load(&hash).await.is_err());
}

#[tokio::test]
async fn test_layered_cache_clear() {
    let dir = tempdir().unwrap();
    let memory = Arc::new(RwLock::new(MemoryStorage::new()));
    let disk = Arc::new(FileStorage::new(dir.path()).await.unwrap());
    
    let cache = LayeredCache::new(memory.clone(), disk);

    // Add test data
    let data = b"test data";
    let hash = blake3::hash(data);
    cache.store(&hash, data).await.unwrap();

    // Clear cache
    cache.clear().await.unwrap();

    // Verify both layers are cleared
    assert!(memory.read().await.load(&hash).await.is_err());
    assert!(cache.load(&hash).await.is_err());
}

#[tokio::test]
async fn test_layered_cache() {
    let temp_dir = tempdir().unwrap();
    let cache = LayeredCache::new(
        temp_dir.path(),
        1024 * 1024,       // 1MB memory limit
        1024 * 1024 * 10,  // 10MB disk limit
    )
    .await.unwrap();

    // Test put and get
    let hash = "test-hash";
    let data = vec![1, 2, 3, 4];
    cache.put_layer(hash, data.clone()).await.unwrap();

    let retrieved = cache.get_layer(hash).await.unwrap().unwrap();
    assert_eq!(retrieved, data);

    // Test removal
    cache.remove_layer(hash).await.unwrap();
    assert!(cache.get_layer(hash).await.unwrap().is_none());

    // Test cleanup
    cache.cleanup().await.unwrap();
}

#[tokio::test]
async fn test_layer_types() {
    let temp_dir = tempdir().unwrap();
    let cache = LayeredCache::new(
        temp_dir.path(),
        1024 * 1024,
        1024 * 1024 * 10,
    )
    .await.unwrap();

    // Test different layer types
    let package_layer = CacheLayer::Package {
        name: "test-pkg".to_string(),
        version: "1.0.0".to_string(),
        hash: "test-hash".to_string(),
    };

    let image_layer = CacheLayer::ImageLayer {
        hash: "test-hash".to_string(),
        layer_type: LayerType::Base,
        parent: None,
    };

    // Store and retrieve package layer
    let data = vec![1, 2, 3];
    cache.put_layer("pkg-hash", data.clone()).await.unwrap();
    let retrieved = cache.get_layer("pkg-hash").await.unwrap().unwrap();
    assert_eq!(retrieved, data);
}
