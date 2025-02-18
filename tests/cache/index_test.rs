use std::sync::Arc;
use tokio::sync::RwLock;
use blast_cache::index::IndexedStorage;
use blast_cache::memory::MemoryStorage;

#[tokio::test]
async fn test_indexed_storage_basic() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let storage = IndexedStorage::new(inner.clone());

    // Test data
    let data = b"test data";
    let hash = blake3::hash(data);
    let key = "test_key";

    // Store with key
    storage.store_with_key(key, &hash, data).await.unwrap();

    // Load by key
    let loaded = storage.load_by_key(key).await.unwrap();
    assert_eq!(loaded, data);

    // Load by hash
    let loaded = storage.load(&hash).await.unwrap();
    assert_eq!(loaded, data);
}

#[tokio::test]
async fn test_indexed_storage_remove() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let storage = IndexedStorage::new(inner.clone());

    // Store data
    let data = b"test data";
    let hash = blake3::hash(data);
    let key = "test_key";
    storage.store_with_key(key, &hash, data).await.unwrap();

    // Remove by key
    storage.remove_by_key(key).await.unwrap();

    // Verify data is removed
    assert!(storage.load_by_key(key).await.is_err());
    assert!(storage.load(&hash).await.is_err());
}

#[tokio::test]
async fn test_indexed_storage_clear() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let storage = IndexedStorage::new(inner.clone());

    // Store multiple items
    let data1 = b"data1";
    let data2 = b"data2";
    let hash1 = blake3::hash(data1);
    let hash2 = blake3::hash(data2);
    
    storage.store_with_key("key1", &hash1, data1).await.unwrap();
    storage.store_with_key("key2", &hash2, data2).await.unwrap();

    // Clear storage
    storage.clear().await.unwrap();

    // Verify all data is removed
    assert!(storage.load_by_key("key1").await.is_err());
    assert!(storage.load_by_key("key2").await.is_err());
    assert!(storage.load(&hash1).await.is_err());
    assert!(storage.load(&hash2).await.is_err());
}

#[tokio::test]
async fn test_indexed_storage_key_collision() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let storage = IndexedStorage::new(inner.clone());

    // Store data with key
    let data1 = b"data1";
    let hash1 = blake3::hash(data1);
    let key = "test_key";
    storage.store_with_key(key, &hash1, data1).await.unwrap();

    // Store different data with same key
    let data2 = b"data2";
    let hash2 = blake3::hash(data2);
    storage.store_with_key(key, &hash2, data2).await.unwrap();

    // Verify only latest data is accessible by key
    let loaded = storage.load_by_key(key).await.unwrap();
    assert_eq!(loaded, data2);

    // Original data should not be accessible by hash
    assert!(storage.load(&hash1).await.is_err());
}
