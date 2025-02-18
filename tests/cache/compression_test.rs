use std::sync::Arc;
use tokio::sync::RwLock;
use blast_cache::compression::CompressedStorage;
use blast_cache::memory::MemoryStorage;

#[tokio::test]
async fn test_compression_storage() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let storage = CompressedStorage::new(inner.clone());

    // Test data with good compression potential
    let data = b"test data test data test data test data".repeat(10);
    let hash = blake3::hash(&data);

    // Store data
    storage.store(&hash, &data).await.unwrap();

    // Load and verify data
    let loaded = storage.load(&hash).await.unwrap();
    assert_eq!(loaded, data);

    // Verify compressed size is smaller
    let compressed_size = inner.read().await.load(&hash).await.unwrap().len();
    assert!(compressed_size < data.len());
}

#[tokio::test]
async fn test_compression_storage_clear() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let storage = CompressedStorage::new(inner.clone());

    // Add test data
    let data = b"test data".repeat(10);
    let hash = blake3::hash(&data);
    storage.store(&hash, &data).await.unwrap();

    // Clear storage
    storage.clear().await.unwrap();

    // Verify storage is empty
    assert!(storage.load(&hash).await.is_err());
    assert!(inner.read().await.load(&hash).await.is_err());
}

#[tokio::test]
async fn test_compression_storage_remove() {
    let inner = Arc::new(RwLock::new(MemoryStorage::new()));
    let storage = CompressedStorage::new(inner.clone());

    // Add test data
    let data = b"test data".repeat(10);
    let hash = blake3::hash(&data);
    storage.store(&hash, &data).await.unwrap();

    // Remove data
    storage.remove(&hash).await.unwrap();

    // Verify data is removed
    assert!(storage.load(&hash).await.is_err());
    assert!(inner.read().await.load(&hash).await.is_err());
}
