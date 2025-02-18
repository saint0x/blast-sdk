use std::io::{Read, Write};
use std::sync::Arc;
use async_trait::async_trait;
use blast_core::error::{BlastError, BlastResult};
use crate::storage::CacheStorage;
use serde::{Serialize, Deserialize};

/// Compression level for cache entries
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionLevel {
    /// No compression
    None,
    /// Fast compression
    Fast,
    /// Default compression
    Default,
    /// Maximum compression
    Maximum,
}

impl CompressionLevel {
    fn to_level(&self) -> i32 {
        match self {
            CompressionLevel::None => 0,
            CompressionLevel::Fast => 1,
            CompressionLevel::Default => 3,
            CompressionLevel::Maximum => 19,
        }
    }
}

/// Compressed storage that compresses data before storing
pub struct CompressedStorage<S: CacheStorage + ?Sized> {
    inner: Arc<S>,
}

impl<S: CacheStorage + ?Sized> CompressedStorage<S> {
    /// Create new compressed storage
    pub fn new(inner: Arc<S>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<S: CacheStorage + Send + Sync + ?Sized> CacheStorage for CompressedStorage<S> {
    async fn store(&self, hash: &blake3::Hash, data: &[u8]) -> BlastResult<()> {
        let compressed = compress(data, CompressionLevel::Default)?;
        self.inner.store(hash, &compressed).await
    }

    async fn load(&self, hash: &blake3::Hash) -> BlastResult<Vec<u8>> {
        let compressed = self.inner.load(hash).await?;
        decompress(&compressed)
    }

    async fn remove(&self, hash: &blake3::Hash) -> BlastResult<()> {
        self.inner.remove(hash).await
    }

    async fn clear(&self) -> BlastResult<()> {
        self.inner.clear().await
    }

    fn hash_path(&self, hash: &blake3::Hash) -> std::path::PathBuf {
        self.inner.hash_path(hash)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Compress data using zstd
pub fn compress(data: &[u8], level: CompressionLevel) -> BlastResult<Vec<u8>> {
    let mut encoder = zstd::Encoder::new(Vec::new(), level.to_level())
        .map_err(|e| BlastError::cache(format!("Failed to create zstd encoder: {}", e)))?;
    
    encoder.write_all(data)
        .map_err(|e| BlastError::cache(format!("Failed to compress data: {}", e)))?;
    
    encoder.finish()
        .map_err(|e| BlastError::cache(format!("Failed to finish compression: {}", e)))
}

/// Decompress zstd compressed data
pub fn decompress(data: &[u8]) -> BlastResult<Vec<u8>> {
    let mut decoder = zstd::Decoder::new(data)
        .map_err(|e| BlastError::cache(format!("Failed to create zstd decoder: {}", e)))?;
    
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)
        .map_err(|e| BlastError::cache(format!("Failed to decompress data: {}", e)))?;
    
    Ok(decompressed)
} 