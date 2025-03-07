use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::fs;

use blast_core::error::{BlastError, BlastResult};
use blast_core::package::{Package, PackageId};

const CACHE_DIR_NAME: &str = "blast-resolver";
const CACHE_FILE_NAME: &str = "package-cache.json";
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    package: Package,
    last_used: SystemTime,
}

/// Cache for resolved dependencies
#[derive(Debug)]
pub struct Cache {
    cache_dir: PathBuf,
    packages: HashMap<PackageId, CacheEntry>,
    last_cleanup: SystemTime,
}

impl Cache {
    /// Create a new cache
    pub fn new(cache_dir: PathBuf) -> Self {
        let cache_dir = cache_dir.join(CACHE_DIR_NAME);
        Self {
            cache_dir,
            packages: HashMap::new(),
            last_cleanup: SystemTime::now(),
        }
    }

    /// Store a package in the cache
    pub async fn store_package(&mut self, package: Package) -> BlastResult<()> {
        let id = package.id().clone();
        let entry = CacheEntry {
            package,
            last_used: SystemTime::now(),
        };

        self.packages.insert(id, entry);
        self.save().await
    }

    /// Get a package from the cache
    pub fn get_package(&mut self, id: &PackageId) -> Option<&Package> {
        if let Some(entry) = self.packages.get_mut(id) {
            entry.last_used = SystemTime::now();
            Some(&entry.package)
        } else {
            None
        }
    }

    /// Save the cache to disk
    async fn save(&self) -> BlastResult<()> {
        let cache_file = self.cache_dir.join(CACHE_FILE_NAME);
        let json = serde_json::to_string(&self.packages)
            .map_err(|e| BlastError::cache(format!("Failed to serialize cache: {}", e)))?;
        fs::write(cache_file, json).await
            .map_err(|e| BlastError::cache(format!("Failed to write cache file: {}", e)))
    }

    /// Load the cache from disk
    pub async fn load(&mut self) -> BlastResult<()> {
        let cache_file = self.cache_dir.join(CACHE_FILE_NAME);
        if !cache_file.exists() {
            return Ok(());
        }

        let json = fs::read_to_string(cache_file).await
            .map_err(|e| BlastError::cache(format!("Failed to read cache file: {}", e)))?;
        self.packages = serde_json::from_str(&json)
            .map_err(|e| BlastError::cache(format!("Failed to deserialize cache: {}", e)))?;
        Ok(())
    }

    /// Clean up old entries
    pub async fn cleanup(&mut self) -> BlastResult<()> {
        let now = SystemTime::now();
        if now
            .duration_since(self.last_cleanup)
            .unwrap()
            .as_secs()
            < 3600
        {
            return Ok(());
        }

        self.packages.retain(|_, entry| {
            entry
                .last_used
                .elapsed()
                .map(|d| d < CACHE_TTL)
                .unwrap_or(false)
        });

        self.last_cleanup = now;
        self.save().await
    }

    pub async fn add_package(&mut self, package: Package) -> BlastResult<()> {
        self.packages.insert(package.id().clone(), CacheEntry {
            package,
            last_used: SystemTime::now(),
        });
        Ok(())
    }
} 