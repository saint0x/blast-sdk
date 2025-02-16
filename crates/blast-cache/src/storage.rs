use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs;
use tracing::warn;

use blast_core::error::{BlastError, BlastResult};

/// Storage backend for cache data
#[async_trait]
pub trait CacheStorage: Send + Sync {
    /// Store data with given hash
    async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()>;

    /// Load data for given hash
    async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>>;

    /// Remove data for given hash
    async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()>;

    /// Clear all stored data
    async fn clear(&self) -> BlastResult<()>;

    /// Get the path for a given hash
    fn hash_path(&self, hash: &blake3::Hash) -> PathBuf;
}

/// File-based storage backend
pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    /// Create new file storage at given root path
    pub async fn new(root: impl AsRef<Path>) -> BlastResult<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)
            .await
            .map_err(|e| BlastError::cache(format!("Failed to create storage directory: {}", e)))?;
        Ok(Self { root })
    }

    fn hash_path(&self, hash: &blake3::Hash) -> PathBuf {
        let hex = hash.to_hex().to_string();
        self.root.join(&hex[0..2]).join(&hex[2..])
    }
}

#[async_trait]
impl CacheStorage for FileStorage {
    async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        let path = self.hash_path(hash);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| BlastError::cache(format!("Failed to create directory: {}", e)))?;
        }
        fs::write(&path, data)
            .await
            .map_err(|e| BlastError::cache(format!("Failed to write file: {}", e)))?;
        Ok(())
    }

    async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>> {
        let path = self.hash_path(hash);
        fs::read(&path)
            .await
            .map_err(|e| BlastError::cache(format!("Failed to read file: {}", e)))
    }

    async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()> {
        let path = self.hash_path(hash);
        if path.exists() {
            fs::remove_file(&path)
                .await
                .map_err(|e| BlastError::cache(format!("Failed to remove file: {}", e)))?;

            // Try to remove parent directory if empty
            if let Some(parent) = path.parent() {
                let mut entries = fs::read_dir(parent).await.map_err(|e| {
                    BlastError::cache(format!("Failed to read directory: {}", e))
                })?;

                if entries.next_entry().await?.is_none() {
                    if let Err(e) = fs::remove_dir(parent).await {
                        warn!("Failed to remove empty directory: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    async fn clear(&self) -> BlastResult<()> {
        fs::remove_dir_all(&self.root)
            .await
            .map_err(|e| BlastError::cache(format!("Failed to clear storage: {}", e)))?;
        fs::create_dir_all(&self.root)
            .await
            .map_err(|e| BlastError::cache(format!("Failed to recreate storage directory: {}", e)))?;
        Ok(())
    }

    fn hash_path(&self, hash: &blake3::Hash) -> PathBuf {
        let hex = hash.to_hex().to_string();
        self.root.join(&hex[0..2]).join(&hex[2..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_storage() {
        let dir = tempdir().unwrap();
        let storage = FileStorage::new(dir.path()).await.unwrap();

        let data = b"test data".to_vec();
        let hash = blake3::hash(&data);

        // Store data
        storage.store(&hash, &data).await.unwrap();
        let path = storage.hash_path(&hash);
        assert!(path.exists());

        // Load data
        let loaded = storage.load(&hash).await.unwrap();
        assert_eq!(loaded, data);

        // Remove data
        storage.remove(&hash).await.unwrap();
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_storage_clear() {
        let dir = tempdir().unwrap();
        let storage = FileStorage::new(dir.path()).await.unwrap();

        // Store multiple files
        for i in 0..5 {
            let data = format!("test data {}", i).into_bytes();
            let hash = blake3::hash(&data);
            storage.store(&hash, &data).await.unwrap();
        }

        // Clear storage
        storage.clear().await.unwrap();

        // Directory should be empty except for the root
        let entries = fs::read_dir(dir.path())
            .await
            .unwrap()
            .count_ready()
            .await;
        assert_eq!(entries, 0);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let dir = tempdir().unwrap();
        let storage = Arc::new(FileStorage::new(dir.path()).await.unwrap());

        let mut handles = Vec::new();
        for i in 0..10 {
            let storage = storage.clone();
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
} 