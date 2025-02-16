use std::io::{Read, Write};

use blast_core::error::{BlastError, BlastResult};

/// Compression level for cache entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompressionLevel {
    /// No compression
    None,
    /// Fast compression with moderate ratio
    Fast,
    /// Default compression level
    Default,
    /// Maximum compression
    Best,
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::Default
    }
}

impl From<CompressionLevel> for i32 {
    fn from(level: CompressionLevel) -> Self {
        match level {
            CompressionLevel::None => 0,
            CompressionLevel::Fast => 1,
            CompressionLevel::Default => 3,
            CompressionLevel::Best => 19,
        }
    }
}

/// Compress data using zstd
pub fn compress(data: &[u8], level: CompressionLevel) -> BlastResult<Vec<u8>> {
    let mut encoder = zstd::Encoder::new(Vec::new(), level.into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_roundtrip() {
        let data = b"test data for compression".to_vec();
        
        for level in &[
            CompressionLevel::None,
            CompressionLevel::Fast,
            CompressionLevel::Default,
            CompressionLevel::Best,
        ] {
            let compressed = compress(&data, *level).unwrap();
            let decompressed = decompress(&compressed).unwrap();
            assert_eq!(decompressed, data);
        }
    }

    #[test]
    fn test_compression_ratio() {
        let data = vec![0u8; 1000]; // Highly compressible data
        
        let compressed = compress(&data, CompressionLevel::Default).unwrap();
        assert!(compressed.len() < data.len());
        
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_invalid_compressed_data() {
        let result = decompress(b"invalid data");
        assert!(result.is_err());
    }
} 