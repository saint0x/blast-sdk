use std::path::{Path, PathBuf};
use tokio::fs;
use async_trait::async_trait;
use blast_core::error::BlastResult;
use std::any::Any;

/// Storage backend for cache data
#[async_trait]
pub trait CacheStorage: Send + Sync + Any {
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

    /// Get as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// File-based storage backend
#[derive(Debug)]
pub struct FileStorage {
    path: PathBuf,
}

impl FileStorage {
    /// Create new file storage at path
    pub async fn new(path: impl AsRef<Path>) -> BlastResult<Self> {
        let path = path.as_ref().to_path_buf();
        fs::create_dir_all(&path).await?;
        Ok(Self { path })
    }

    /// Get file path for hash
    fn hash_path(&self, hash: &blake3::Hash) -> PathBuf {
        let hex = hash.to_hex().to_string();
        self.path.join(&hex[0..2]).join(&hex[2..])
    }
}

#[async_trait]
impl CacheStorage for FileStorage {
    async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        let path = self.hash_path(hash);
        fs::write(path, data).await?;
        Ok(())
    }

    async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>> {
        let path = self.hash_path(hash);
        fs::read(path).await.map_err(Into::into)
    }

    async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()> {
        let path = self.hash_path(hash);
        fs::remove_file(path).await?;
        Ok(())
    }

    async fn clear(&self) -> BlastResult<()> {
        let mut entries = fs::read_dir(&self.path).await?;
        while let Some(entry) = entries.next_entry().await? {
            fs::remove_file(entry.path()).await?;
        }
        Ok(())
    }

    fn hash_path(&self, hash: &blake3::Hash) -> PathBuf {
        let hex = hash.to_hex().to_string();
        self.path.join(&hex[0..2]).join(&hex[2..])
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
} 