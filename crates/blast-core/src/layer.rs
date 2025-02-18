use async_trait::async_trait;
use crate::error::BlastResult;

/// Layer trait for caching operations
#[async_trait]
pub trait Layer: Send + Sync {
    /// Get layer data by hash
    async fn get_layer(&self, hash: &str) -> BlastResult<Option<Vec<u8>>>;

    /// Store layer data with hash
    async fn put_layer(&self, hash: &str, data: Vec<u8>) -> BlastResult<()>;

    /// Remove layer by hash
    async fn remove_layer(&self, hash: &str) -> BlastResult<()>;

    /// Clean up expired or invalid layers
    async fn cleanup(&self) -> BlastResult<()>;
} 