use async_trait::async_trait;

use blast_core::error::BlastResult;
use blast_core::package::{Package, PackageId, Version};

/// Interface for package sources (e.g., PyPI, local directory, custom index)
#[async_trait]
pub trait PackageSource: Send + Sync + 'static {
    /// Get package metadata
    async fn get_package(&self, id: &PackageId) -> BlastResult<Package>;

    /// Get available versions for a package
    async fn get_versions(&self, package_name: &str) -> BlastResult<Vec<Version>>;

    /// Download package
    async fn download_package(&self, id: &PackageId) -> BlastResult<Vec<u8>>;

    /// Check if a package exists
    async fn package_exists(&self, id: &PackageId) -> BlastResult<bool>;

    /// Get source name
    fn name(&self) -> &str;

    /// Get source priority (lower is higher priority)
    fn priority(&self) -> u32;
}

/// A chain of package sources that are tried in order of priority
pub struct PackageSourceChain {
    sources: Vec<Box<dyn PackageSource>>,
}

impl PackageSourceChain {
    /// Create a new package source chain
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Add a package source to the chain
    pub fn add_source(&mut self, source: Box<dyn PackageSource>) {
        // Insert source in order of priority
        let pos = self.sources
            .binary_search_by_key(&source.priority(), |s| s.priority())
            .unwrap_or_else(|e| e);
        self.sources.insert(pos, source);
    }

    /// Get all sources in the chain
    pub fn sources(&self) -> &[Box<dyn PackageSource>] {
        &self.sources
    }
}

#[async_trait]
impl PackageSource for PackageSourceChain {
    async fn get_package(&self, id: &PackageId) -> BlastResult<Package> {
        let mut last_error = None;

        for source in &self.sources {
            match source.get_package(id).await {
                Ok(package) => return Ok(package),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            blast_core::error::BlastError::package(format!(
                "No package sources available for {}",
                id
            ))
        }))
    }

    async fn get_versions(&self, package_name: &str) -> BlastResult<Vec<Version>> {
        let mut versions = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for source in &self.sources {
            if let Ok(mut source_versions) = source.get_versions(package_name).await {
                // Only add versions we haven't seen before
                source_versions.retain(|v| seen.insert(v.clone()));
                versions.extend(source_versions);
            }
        }

        if versions.is_empty() {
            return Err(blast_core::error::BlastError::package(format!(
                "No versions found for package {}",
                package_name
            )));
        }

        // Sort versions in descending order
        versions.sort();
        versions.reverse();
        Ok(versions)
    }

    async fn download_package(&self, id: &PackageId) -> BlastResult<Vec<u8>> {
        let mut last_error = None;

        for source in &self.sources {
            match source.download_package(id).await {
                Ok(data) => return Ok(data),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            blast_core::error::BlastError::package(format!(
                "No package sources available for {}",
                id
            ))
        }))
    }

    async fn package_exists(&self, id: &PackageId) -> BlastResult<bool> {
        for source in &self.sources {
            if source.package_exists(id).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn name(&self) -> &str {
        "package-source-chain"
    }

    fn priority(&self) -> u32 {
        0
    }
} 