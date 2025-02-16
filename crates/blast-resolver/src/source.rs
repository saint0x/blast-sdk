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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use blast_core::package::VersionConstraint;

    struct MockSource {
        name: String,
        priority: u32,
        packages: HashMap<PackageId, Package>,
    }

    #[async_trait]
    impl PackageSource for MockSource {
        async fn get_package(&self, id: &PackageId) -> BlastResult<Package> {
            self.packages
                .get(id)
                .cloned()
                .ok_or_else(|| blast_core::error::BlastError::package("Package not found"))
        }

        async fn get_versions(&self, package_name: &str) -> BlastResult<Vec<Version>> {
            let versions: Vec<Version> = self
                .packages
                .keys()
                .filter(|id| id.name() == package_name)
                .map(|id| id.version().clone())
                .collect();
            Ok(versions)
        }

        async fn download_package(&self, id: &PackageId) -> BlastResult<Vec<u8>> {
            Ok(Vec::new()) // Mock implementation
        }

        async fn package_exists(&self, id: &PackageId) -> BlastResult<bool> {
            Ok(self.packages.contains_key(id))
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> u32 {
            self.priority
        }
    }

    #[tokio::test]
    async fn test_package_source_chain() {
        let mut source1 = MockSource {
            name: "source1".to_string(),
            priority: 1,
            packages: HashMap::new(),
        };

        let mut source2 = MockSource {
            name: "source2".to_string(),
            priority: 2,
            packages: HashMap::new(),
        };

        let package1 = Package::new(
            PackageId::new("test1", Version::parse("1.0.0").unwrap()),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        let package2 = Package::new(
            PackageId::new("test2", Version::parse("2.0.0").unwrap()),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        source1.packages.insert(package1.id().clone(), package1.clone());
        source2.packages.insert(package2.id().clone(), package2.clone());

        let mut chain = PackageSourceChain::new();
        chain.add_source(Box::new(source1));
        chain.add_source(Box::new(source2));

        // Test package retrieval
        let result = chain.get_package(package1.id()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id(), package1.id());

        let result = chain.get_package(package2.id()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id(), package2.id());

        // Test version listing
        let versions = chain.get_versions("test1").await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0], *package1.version());
    }
} 