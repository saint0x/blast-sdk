use std::io::{self, Write};
use serde::{Deserialize, Serialize};
use zstd::stream::{Encoder, Decoder};
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

use crate::error::{Error, Result};

/// Compression types supported by Blast
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// Zstandard compression
    Zstd,
    /// GZIP compression
    Gzip,
}

/// Compression level for layer storage
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum CompressionLevel {
    /// No compression
    None,
    /// Fast compression
    Fast,
    /// Default compression
    Default,
    /// Maximum compression
    Best,
}

impl CompressionLevel {
    /// Convert to zstd compression level
    pub fn to_zstd_level(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Fast => 1,
            Self::Default => 3,
            Self::Best => 19,
        }
    }

    /// Convert to gzip compression level
    pub fn to_gzip_level(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Fast => 1,
            Self::Default => 6,
            Self::Best => 9,
        }
    }
}

/// Compression strategy trait
pub trait CompressionStrategy: Send + Sync {
    /// Compress data
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Decompress data
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Get compression type
    fn compression_type(&self) -> CompressionType;
    
    /// Get compression level
    fn compression_level(&self) -> CompressionLevel;
}

/// Zstandard compression strategy
pub struct ZstdStrategy {
    level: CompressionLevel,
}

impl ZstdStrategy {
    /// Create a new Zstd strategy with the given compression level
    pub fn new(level: CompressionLevel) -> Self {
        Self { level }
    }
}

impl CompressionStrategy for ZstdStrategy {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = Encoder::new(Vec::new(), self.level.to_zstd_level())
            .map_err(|e| Error::Io { source: e, path: None })?;
        encoder.write_all(data)
            .map_err(|e| Error::Io { source: e, path: None })?;
        Ok(encoder.finish()
            .map_err(|e| Error::Io { source: e, path: None })?)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = Decoder::new(data)
            .map_err(|e| Error::Io { source: e, path: None })?;
        let mut decompressed = Vec::new();
        io::copy(&mut decoder, &mut decompressed)
            .map_err(|e| Error::Io { source: e, path: None })?;
        Ok(decompressed)
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Zstd
    }

    fn compression_level(&self) -> CompressionLevel {
        self.level
    }
}

/// GZIP compression strategy
pub struct GzipStrategy {
    level: CompressionLevel,
}

impl GzipStrategy {
    /// Create a new GZIP strategy with the given compression level
    pub fn new(level: CompressionLevel) -> Self {
        Self { level }
    }
}

impl CompressionStrategy for GzipStrategy {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level.to_gzip_level()));
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        io::copy(&mut decoder, &mut decompressed)?;
        Ok(decompressed)
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Gzip
    }

    fn compression_level(&self) -> CompressionLevel {
        self.level
    }
}

/// Create a compression strategy for the given type and level
pub fn create_strategy(
    compression_type: CompressionType,
    level: CompressionLevel,
) -> Box<dyn CompressionStrategy> {
    match compression_type {
        CompressionType::None => Box::new(NoopStrategy),
        CompressionType::Zstd => Box::new(ZstdStrategy::new(level)),
        CompressionType::Gzip => Box::new(GzipStrategy::new(level)),
    }
}

/// No-op compression strategy
#[derive(Default)]
pub struct NoopStrategy;

impl CompressionStrategy for NoopStrategy {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::None
    }

    fn compression_level(&self) -> CompressionLevel {
        CompressionLevel::None
    }
}

/// Calculate compression ratio
pub fn compression_ratio(original_size: u64, compressed_size: u64) -> f64 {
    if original_size == 0 {
        return 1.0;
    }
    compressed_size as f64 / original_size as f64
} 