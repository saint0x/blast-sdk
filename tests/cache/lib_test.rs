use std::sync::Arc;
use blast_cache::{CacheStorage, CacheBuilder};
use blast_image::blake3;
use tempfile::tempdir;

#[tokio::test]
async fn test_cache_creation() {
    let dir = tempdir().unwrap();
    let cache = CacheBuilder::new()
        .memory_size(1024 * 1024) // 1MB memory cache
        .compression(true)
        .path(dir.path())
        .build()
        .await
        .unwrap();

    assert!(cache.is_compression_enabled());
}

#[tokio::test]
async fn test_cache_operations() {
    let dir = tempdir().unwrap();
    let cache = CacheBuilder::new()
        .path(dir.path())
        .build()
        .await
        .unwrap();

    // Test data
    let data = b"test data";
    let hash = blake3::hash(data);

    // Store data
    cache.store(&hash, data).await.unwrap();

    // Load data
    let loaded = cache.load(&hash).await.unwrap();
    assert_eq!(loaded, data);

    // Remove data
    cache.remove(&hash).await.unwrap();
    assert!(cache.load(&hash).await.is_err());
}

#[tokio::test]
async fn test_cache_with_key() {
    let dir = tempdir().unwrap();
    let cache = CacheBuilder::new()
        .path(dir.path())
        .indexed(true)
        .build()
        .await
        .unwrap();

    // Test data
    let data = b"test data";
    let hash = blake3::hash(data);
    let key = "test_key";

    // Store with key
    cache.store_with_key(key, &hash, data).await.unwrap();

    // Load by key
    let loaded = cache.load_by_key(key).await.unwrap();
    assert_eq!(loaded, data);

    // Remove by key
    cache.remove_by_key(key).await.unwrap();
    assert!(cache.load_by_key(key).await.is_err());
}

#[tokio::test]
async fn test_cache_clear() {
    let dir = tempdir().unwrap();
    let cache = CacheBuilder::new()
        .path(dir.path())
        .build()
        .await
        .unwrap();

    // Add test data
    let data = b"test data";
    let hash = blake3::hash(data);
    cache.store(&hash, data).await.unwrap();

    // Clear cache
    cache.clear().await.unwrap();

    // Verify cache is empty
    assert!(cache.load(&hash).await.is_err());
}

#[tokio::test]
async fn test_cache_persistence() {
    let dir = tempdir().unwrap();
    let path = dir.path().to_owned();

    // Store data in first cache instance
    let cache1 = CacheBuilder::new()
        .path(&path)
        .build()
        .await
        .unwrap();

    let data = b"test data";
    let hash = blake3::hash(data);
    cache1.store(&hash, data).await.unwrap();

    // Drop first cache instance
    drop(cache1);

    // Create new cache instance with same path
    let cache2 = CacheBuilder::new()
        .path(&path)
        .build()
        .await
        .unwrap();

    // Verify data persisted
    let loaded = cache2.load(&hash).await.unwrap();
    assert_eq!(loaded, data);
}
