//! Image layer handling

use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::fs;
use walkdir::WalkDir;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use blake3::Hasher;

use blast_core::error::{BlastError, BlastResult};
use blast_core::python::PythonEnvironment;

/// Layer types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerType {
    /// Base Python installation
    Base,
    /// Package installations
    Packages,
    /// Custom files
    Custom,
    /// Configuration
    Config,
}

/// Compression types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// Zstandard compression
    Zstd,
    /// GZIP compression
    Gzip,
}

/// Compression level for layer storage
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::Default
    }
}

impl CompressionLevel {
    fn to_zstd_level(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Fast => 1,
            Self::Default => 3,
            Self::Best => 19,
        }
    }
}

/// Layer metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerMetadata {
    /// Layer creation time
    pub created_at: DateTime<Utc>,
    /// Layer compression level
    pub compression: CompressionLevel,
    /// Layer size before compression
    pub original_size: u64,
    /// Layer size after compression
    pub compressed_size: u64,
    /// Layer hash (blake3)
    pub hash: String,
}

/// Layer in a Blast image
#[derive(Debug, Clone)]
pub struct Layer {
    /// Layer name
    pub name: String,
    /// Layer path
    pub path: PathBuf,
    /// Layer metadata
    pub metadata: LayerMetadata,
    /// Compression level
    pub compression: CompressionLevel,
}

impl Layer {
    /// Create a new layer from an environment
    pub fn from_environment(env: &PythonEnvironment) -> BlastResult<Self> {
        Self::from_environment_with_compression(env, CompressionLevel::default())
    }

    /// Create a new layer with specific compression
    pub fn from_environment_with_compression(
        env: &PythonEnvironment,
        compression: CompressionLevel,
    ) -> BlastResult<Self> {
        let path = env.path().to_path_buf();
        let original_size = calculate_size(&path)?;
        let hash = calculate_hash(&path)?;

        Ok(Self {
            name: env.name().unwrap_or("unnamed").to_string(),
            path,
            metadata: LayerMetadata {
                created_at: Utc::now(),
                compression,
                original_size,
                compressed_size: 0, // Will be set after compression
                hash,
            },
            compression,
        })
    }

    /// Save layer to a file
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> BlastResult<()> {
        let mut file = fs::File::create(path).map_err(BlastError::from)?;
        let mut buf = Vec::new();
        
        // Create tar archive
        {
            let mut tar = tar::Builder::new(&mut buf);
            tar.append_dir_all(".", &self.path)
                .map_err(|e| BlastError::serialization(e.to_string()))?;
            tar.finish().map_err(|e| BlastError::serialization(e.to_string()))?;
        }

        // Compress the archive
        let compressed = zstd::encode_all(
            &buf[..],
            self.compression.to_zstd_level(),
        ).map_err(|e| BlastError::serialization(e.to_string()))?;

        // Update metadata
        self.metadata.compressed_size = compressed.len() as u64;
        
        // Write compressed data
        file.write_all(&compressed).map_err(BlastError::from)?;

        Ok(())
    }

    /// Load layer from a file
    pub fn load<P1: AsRef<Path>, P2: AsRef<Path>>(path: P1, target: P2) -> BlastResult<Self> {
        let file = fs::File::open(path.as_ref()).map_err(BlastError::from)?;
        let decompressed = zstd::decode_all(file)
            .map_err(|e| BlastError::serialization(e.to_string()))?;
        
        // Extract the archive
        let mut archive = tar::Archive::new(&decompressed[..]);
        archive.unpack(target.as_ref())
            .map_err(|e| BlastError::serialization(e.to_string()))?;

        let original_size = calculate_size(target.as_ref())?;
        let hash = calculate_hash(target.as_ref())?;

        Ok(Self {
            name: path.as_ref().file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unnamed")
                .to_string(),
            path: target.as_ref().to_path_buf(),
            metadata: LayerMetadata {
                created_at: Utc::now(),
                compression: CompressionLevel::default(),
                original_size,
                compressed_size: decompressed.len() as u64,
                hash,
            },
            compression: CompressionLevel::default(),
        })
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.metadata.original_size == 0 {
            0.0
        } else {
            self.metadata.compressed_size as f64 / self.metadata.original_size as f64
        }
    }

    /// Verify layer integrity
    pub fn verify(&self) -> BlastResult<bool> {
        // Calculate current hash
        let current_hash = calculate_hash(&self.path)?;
        
        // Compare with stored hash
        Ok(current_hash == self.metadata.hash)
    }
}

/// Calculate directory size
fn calculate_size(path: &Path) -> BlastResult<u64> {
    let mut total_size = 0;

    for entry in WalkDir::new(path) {
        let entry = entry.map_err(|e| BlastError::serialization(e.to_string()))?;
        if entry.file_type().is_file() {
            total_size += entry.metadata()
                .map_err(|e| BlastError::serialization(e.to_string()))?
                .len();
        }
    }

    Ok(total_size)
}

/// Calculate directory hash
fn calculate_hash(path: &Path) -> BlastResult<String> {
    let mut hasher = Hasher::new();

    for entry in WalkDir::new(path).sort_by_file_name() {
        let entry = entry.map_err(|e| BlastError::serialization(e.to_string()))?;
        if entry.file_type().is_file() {
            let mut file = fs::File::open(entry.path()).map_err(BlastError::from)?;
            io::copy(&mut file, &mut hasher).map_err(BlastError::from)?;
        }
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::str::FromStr;

    #[test]
    fn test_layer_creation() {
        let dir = tempdir().unwrap();
        let version = blast_core::python::PythonVersion::from_str("3.8").unwrap();
        let mut env = PythonEnvironment::new(dir.path().to_path_buf(), version);
        env.set_name("test-env".to_string());

        // Create test files
        fs::write(dir.path().join("test.txt"), b"test data").unwrap();

        let layer = Layer::from_environment(&env).unwrap();
        assert_eq!(layer.name, "test-env");
        assert_eq!(layer.path, env.path());
        assert_eq!(layer.metadata.original_size, 9); // "test data" is 9 bytes
        assert!(!layer.metadata.hash.is_empty());
    }

    #[test]
    fn test_layer_compression_levels() {
        let dir = tempdir().unwrap();
        let version = blast_core::python::PythonVersion::from_str("3.8").unwrap();
        let mut env = PythonEnvironment::new(dir.path().to_path_buf(), version);
        
        // Create test files with repetitive content for better compression
        let content = "test data ".repeat(1000);
        fs::write(dir.path().join("test.txt"), content).unwrap();

        // Test different compression levels
        let compression_levels = vec![
            CompressionLevel::None,
            CompressionLevel::Fast,
            CompressionLevel::Default,
            CompressionLevel::Best,
        ];

        let mut sizes = Vec::new();
        for level in compression_levels {
            let mut layer = Layer::from_environment_with_compression(&env, level).unwrap();
            let temp_file = tempdir().unwrap().path().join("layer.tar.zst");
            layer.save(&temp_file).unwrap();
            sizes.push(layer.metadata.compressed_size);
        }

        // Verify that higher compression levels generally produce smaller files
        for i in 1..sizes.len() {
            assert!(sizes[i] <= sizes[i-1], 
                "Higher compression level {} produced larger file than level {}", i, i-1);
        }
    }

    #[test]
    fn test_layer_save_load() {
        let src_dir = tempdir().unwrap();
        let version = blast_core::python::PythonVersion::from_str("3.8").unwrap();
        let mut env = PythonEnvironment::new(src_dir.path().to_path_buf(), version);
        env.set_name("test-env".to_string());
        
        // Create test files
        fs::write(src_dir.path().join("test.txt"), b"test data").unwrap();
        fs::create_dir(src_dir.path().join("subdir")).unwrap();
        fs::write(src_dir.path().join("subdir/test2.txt"), b"more data").unwrap();

        let mut layer = Layer::from_environment(&env).unwrap();
        
        let layer_file = tempdir().unwrap().path().join("layer.tar.zst");
        layer.save(&layer_file).unwrap();

        let target_dir = tempdir().unwrap();
        let loaded = Layer::load(&layer_file, target_dir.path()).unwrap();

        assert_eq!(loaded.metadata.original_size, layer.metadata.original_size);
        assert_eq!(loaded.metadata.hash, layer.metadata.hash);
        assert!(loaded.compression_ratio() > 0.0);
    }
} 