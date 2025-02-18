use std::sync::Arc;
use tempfile::tempdir;
use tokio::fs;
use blast_cache::storage::FileStorage;

#[tokio::test]
async fn test_storage_clear() {
    let dir = tempdir().unwrap();
    let storage = FileStorage::new(dir.path()).await.unwrap();

    // Add some test data
    let data = b"test data";
    let hash = blake3::hash(data);
    storage.store(&hash, data).await.unwrap();

    // Clear storage
    storage.clear().await.unwrap();

    // Directory should be empty except for the root
    let mut entries = fs::read_dir(dir.path()).await.unwrap();
    let mut count = 0;
    while let Some(_) = entries.next_entry().await.unwrap() {
        count += 1;
    }
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_concurrent_access() {
    let dir = tempdir().unwrap();
    let storage = Arc::new(FileStorage::new(dir.path()).await.unwrap());

    let mut handles = Vec::new();
    for i in 0..10 {
        let storage = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let data = format!("test data {}", i).into_bytes();
            let hash = blake3::hash(&data);
            storage.store(&hash, &data).await.unwrap();
            let loaded = storage.load(&hash).await.unwrap();
            assert_eq!(loaded, data);
            storage.remove(&hash).await.unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
