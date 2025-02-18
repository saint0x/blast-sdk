use blast_cache::memory::MemoryStorage;

#[tokio::test]
async fn test_memory_storage_operations() {
    let storage = MemoryStorage::new();

    // Test data
    let data = b"test data";
    let hash = blake3::hash(data);

    // Store data
    storage.store(&hash, data).await.unwrap();

    // Load data
    let loaded = storage.load(&hash).await.unwrap();
    assert_eq!(loaded, data);

    // Remove data
    storage.remove(&hash).await.unwrap();
    assert!(storage.load(&hash).await.is_err());
}

#[tokio::test]
async fn test_memory_storage_clear() {
    let storage = MemoryStorage::new();

    // Add test data
    let data = b"test data";
    let hash = blake3::hash(data);
    storage.store(&hash, data).await.unwrap();

    // Clear storage
    storage.clear().await.unwrap();

    // Verify storage is empty
    assert!(storage.load(&hash).await.is_err());
}

#[tokio::test]
async fn test_memory_storage_size_limit() {
    let mut storage = MemoryStorage::new();
    storage.set_size_limit(100); // Set 100 byte limit

    // Add data within limit
    let small_data = b"small";
    let small_hash = blake3::hash(small_data);
    storage.store(&small_hash, small_data).await.unwrap();

    // Add data exceeding limit
    let large_data = vec![0u8; 200];
    let large_hash = blake3::hash(&large_data);
    assert!(storage.store(&large_hash, &large_data).await.is_err());
}
