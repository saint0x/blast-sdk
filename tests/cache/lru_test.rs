use std::sync::Arc;
use tokio::sync::RwLock;
use blast_cache::lru::LRUCache;
use blast_cache::memory::MemoryStorage;
use blast_image::blake3::hash;

#[tokio::test]
async fn test_lru_cache_basic() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut cache = LRUCache::new(inner, 2); // Capacity of 2 items

    // Test data
    let data1 = b"data1";
    let data2 = b"data2";
    let data3 = b"data3";
    let hash1 = hash(data1);
    let hash2 = hash(data2);
    let hash3 = hash(data3);

    // Store data
    cache.store(&hash1, data1).await.unwrap();
    cache.store(&hash2, data2).await.unwrap();

    // Verify both items are present
    assert!(cache.load(&hash1).await.is_ok());
    assert!(cache.load(&hash2).await.is_ok());

    // Add third item, should evict first item (LRU)
    cache.store(&hash3, data3).await.unwrap();
    assert!(cache.load(&hash1).await.is_err()); // Should be evicted
    assert!(cache.load(&hash2).await.is_ok());
    assert!(cache.load(&hash3).await.is_ok());
}

#[tokio::test]
async fn test_lru_cache_update_access() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut cache = LRUCache::new(inner, 2);

    // Add two items
    let data1 = b"data1";
    let data2 = b"data2";
    let data3 = b"data3";
    let hash1 = hash(data1);
    let hash2 = hash(data2);
    let hash3 = hash(data3);

    cache.store(&hash1, data1).await.unwrap();
    cache.store(&hash2, data2).await.unwrap();

    // Access first item to make it most recently used
    cache.load(&hash1).await.unwrap();

    // Add third item, should evict second item (now LRU)
    cache.store(&hash3, data3).await.unwrap();
    assert!(cache.load(&hash1).await.is_ok()); // Should still be present
    assert!(cache.load(&hash2).await.is_err()); // Should be evicted
    assert!(cache.load(&hash3).await.is_ok());
}

#[tokio::test]
async fn test_lru_cache_clear() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut cache = LRUCache::new(inner.clone(), 2);

    // Add items
    let data = b"test data";
    let hash = hash(data);
    cache.store(&hash, data).await.unwrap();

    // Clear cache
    cache.clear().await.unwrap();

    // Verify both cache and inner storage are cleared
    assert!(cache.load(&hash).await.is_err());
    assert!(inner.read().await.load(&hash).await.is_err());
}

#[tokio::test]
async fn test_lru_cache_remove() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut cache = LRUCache::new(inner.clone(), 2);

    // Add item
    let data = b"test data";
    let hash = hash(data);
    cache.store(&hash, data).await.unwrap();

    // Remove item
    cache.remove(&hash).await.unwrap();

    // Verify item is removed from both cache and inner storage
    assert!(cache.load(&hash).await.is_err());
    assert!(inner.read().await.load(&hash).await.is_err());
}
