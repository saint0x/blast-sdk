//! Image layer handling

use std::path::{Path, PathBuf};
use std::io::{self, Write, Read};
use std::fs;
use std::fmt;
use walkdir::WalkDir;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use blake3::Hasher;

use blast_core::python::PythonEnvironment;
use blast_core::environment::Environment;

use crate::compression::{CompressionLevel, CompressionType, CompressionStrategy, create_strategy};
use crate::error::{Error, Result};

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

/// Layer metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerMetadata {
    /// Layer creation time
    pub created_at: DateTime<Utc>,
    /// Layer type
    pub layer_type: LayerType,
    /// Layer compression type
    pub compression_type: CompressionType,
    /// Layer compression level
    pub compression_level: CompressionLevel,
    /// Layer size before compression
    pub original_size: u64,
    /// Layer size after compression
    pub compressed_size: u64,
    /// Layer hash (blake3)
    pub hash: String,
    /// Layer dependencies
    pub dependencies: Vec<String>,
}

impl LayerMetadata {
    /// Create new layer metadata
    pub fn new(
        layer_type: LayerType,
        compression_type: CompressionType,
        compression_level: CompressionLevel,
    ) -> Self {
        Self {
            created_at: Utc::now(),
            layer_type,
            compression_type,
            compression_level,
            original_size: 0,
            compressed_size: 0,
            hash: String::new(),
            dependencies: Vec::new(),
        }
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        crate::compression::compression_ratio(self.original_size, self.compressed_size)
    }
}

/// Layer in a Blast image
pub struct Layer {
    /// Layer name
    pub name: String,
    /// Layer path
    pub path: PathBuf,
    /// Layer metadata
    pub metadata: LayerMetadata,
    /// Compression strategy
    compression_strategy: Box<dyn CompressionStrategy>,
}

impl fmt::Debug for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layer")
            .field("name", &self.name)
            .field("path", &self.path)
            .field("metadata", &self.metadata)
            .field("compression_strategy", &"<dyn CompressionStrategy>")
            .finish()
    }
}

impl Layer {
    /// Create a new layer
    pub fn new(
        name: String,
        path: PathBuf,
        layer_type: LayerType,
        compression_type: CompressionType,
        compression_level: CompressionLevel,
    ) -> Self {
        let strategy = create_strategy(compression_type.clone(), compression_level);
        Self {
            name,
            path,
            metadata: LayerMetadata::new(layer_type, compression_type, compression_level),
            compression_strategy: strategy,
        }
    }

    /// Create a new layer from an environment
    pub fn from_environment(env: &PythonEnvironment) -> Result<Self> {
        Self::from_environment_with_options(
            env,
            LayerType::Base,
            CompressionType::Zstd,
            CompressionLevel::default(),
        )
    }

    /// Create a new layer from an environment with options
    pub fn from_environment_with_options(
        env: &PythonEnvironment,
        layer_type: LayerType,
        compression_type: CompressionType,
        compression_level: CompressionLevel,
    ) -> Result<Self> {
        let name = format!("{}_{}", 
            Environment::name(env), 
            Environment::python_version(env)
        );
        let path = Environment::path(env).to_path_buf();
        Ok(Self::new(name, path, layer_type, compression_type, compression_level))
    }

    /// Save layer to a file
    pub fn save<P: AsRef<Path>>(&mut self, target: P) -> Result<()> {
        let target_path = target.as_ref().to_path_buf();
        
        // Calculate original size and hash
        self.metadata.original_size = calculate_size(&self.path)?;
        self.metadata.hash = calculate_hash(&self.path)?;

        // Create tar archive
        let mut tar = tar::Builder::new(Vec::new());
        for entry in WalkDir::new(&self.path) {
            let entry = entry.map_err(|e| Error::layer_with_name(e.to_string(), self.name.clone()))?;
            let path = entry.path();
            if path.is_file() {
                let name = path.strip_prefix(&self.path)
                    .map_err(|e| Error::layer_with_name(format!("Failed to strip prefix: {}", e), self.name.clone()))?;
                tar.append_path_with_name(path, name)
                    .map_err(|e| Error::io(e, path.to_path_buf()))?;
            }
        }
        let tar_data = tar.into_inner()
            .map_err(|e| Error::io(e, target_path.clone()))?;

        // Compress data
        let compressed_data = self.compression_strategy.compress(&tar_data)
            .map_err(|e| Error::compression_with_source(format!("Failed to compress layer {}", self.name), e))?;
        self.metadata.compressed_size = compressed_data.len() as u64;

        // Write compressed data and metadata
        let mut file = fs::File::create(&target_path)
            .map_err(|e| Error::io(e, target_path.clone()))?;
        serde_json::to_writer(&mut file, &self.metadata)
            .map_err(|e| Error::serialization_with_source(format!("Failed to write metadata for layer {}", self.name), e))?;
        file.write_all(&compressed_data)
            .map_err(|e| Error::io(e, target_path.clone()))?;

        Ok(())
    }

    /// Load layer from a file
    pub fn load<P1: AsRef<Path>, P2: AsRef<Path>>(source: P1, target: P2) -> Result<Self> {
        let source_path = source.as_ref().to_path_buf();
        let target_path = target.as_ref().to_path_buf();
        
        let mut file = fs::File::open(&source_path)
            .map_err(|e| Error::io(e, source_path.clone()))?;
        let metadata: LayerMetadata = serde_json::from_reader(&file)
            .map_err(|e| Error::serialization_with_source(format!("Failed to read metadata from {}", source_path.display()), e))?;

        let strategy = create_strategy(metadata.compression_type.clone(), metadata.compression_level);
        let layer = Self {
            name: source.as_ref().file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path: target_path.clone(),
            metadata,
            compression_strategy: strategy,
        };

        // Read and decompress data
        let mut compressed_data = Vec::new();
        file.read_to_end(&mut compressed_data)
            .map_err(|e| Error::io(e, source_path.clone()))?;
        let tar_data = layer.compression_strategy.decompress(&compressed_data)
            .map_err(|e| Error::compression_with_source(format!("Failed to decompress layer {}", layer.name), e))?;

        // Extract tar archive
        let mut archive = tar::Archive::new(&tar_data[..]);
        archive.unpack(&target_path)
            .map_err(|e| Error::io(e, target_path.clone()))?;

        Ok(layer)
    }

    /// Verify layer integrity
    pub fn verify(&self) -> Result<bool> {
        let current_size = calculate_size(&self.path)?;
        if current_size != self.metadata.original_size {
            return Ok(false);
        }

        let current_hash = calculate_hash(&self.path)?;
        Ok(current_hash == self.metadata.hash)
    }

    /// Get layer size
    pub fn size(&self) -> u64 {
        self.metadata.original_size
    }

    /// Get compressed size
    pub fn compressed_size(&self) -> u64 {
        self.metadata.compressed_size
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        self.metadata.compression_ratio()
    }
}

fn calculate_size(path: &Path) -> Result<u64> {
    let mut size = 0;
    for entry in WalkDir::new(path) {
        let entry = entry.map_err(|e| Error::io(io::Error::new(io::ErrorKind::Other, e), path.to_path_buf()))?;
        if entry.path().is_file() {
            size += entry.metadata()
                .map_err(|e| Error::io(io::Error::new(io::ErrorKind::Other, e.to_string()), entry.path().to_path_buf()))?.len();
        }
    }
    Ok(size)
}

fn calculate_hash(path: &Path) -> Result<String> {
    let mut hasher = Hasher::new();
    for entry in WalkDir::new(path) {
        let entry = entry.map_err(|e| Error::io(io::Error::new(io::ErrorKind::Other, e), path.to_path_buf()))?;
        if entry.path().is_file() {
            let file_path = entry.path().to_path_buf();
            let mut file = fs::File::open(&file_path)
                .map_err(|e| Error::io(e, file_path.clone()))?;
            io::copy(&mut file, &mut hasher)
                .map_err(|e| Error::io(e, file_path))?;
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self::Default
    }
} 