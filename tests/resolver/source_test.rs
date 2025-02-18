use std::collections::HashMap;
use async_trait::async_trait;
use blast_core::{
    error::BlastResult,
    package::{Package, PackageId, Version, VersionConstraint},
};
use blast_resolver::source::PackageSource;

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

    let mut chain = blast_resolver::source::PackageSourceChain::new();
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

#[tokio::test]
async fn test_source_priority() {
    let mut source1 = MockSource {
        name: "high-priority".to_string(),
        priority: 1,
        packages: HashMap::new(),
    };

    let mut source2 = MockSource {
        name: "low-priority".to_string(),
        priority: 2,
        packages: HashMap::new(),
    };

    // Create same package in both sources with different versions
    let package1 = Package::new(
        PackageId::new("test", Version::parse("1.0.0").unwrap()),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );

    let package2 = Package::new(
        PackageId::new("test", Version::parse("2.0.0").unwrap()),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );

    source1.packages.insert(package1.id().clone(), package1.clone());
    source2.packages.insert(package2.id().clone(), package2);

    let mut chain = blast_resolver::source::PackageSourceChain::new();
    chain.add_source(Box::new(source1));
    chain.add_source(Box::new(source2));

    // Should get package from higher priority source
    let result = chain.get_package(package1.id()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().version().to_string(), "1.0.0");
}

#[tokio::test]
async fn test_source_fallback() {
    let source1 = MockSource {
        name: "empty-source".to_string(),
        priority: 1,
        packages: HashMap::new(),
    };

    let mut source2 = MockSource {
        name: "fallback-source".to_string(),
        priority: 2,
        packages: HashMap::new(),
    };

    let package = Package::new(
        PackageId::new("test", Version::parse("1.0.0").unwrap()),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );

    source2.packages.insert(package.id().clone(), package.clone());

    let mut chain = blast_resolver::source::PackageSourceChain::new();
    chain.add_source(Box::new(source1));
    chain.add_source(Box::new(source2));

    // Should fall back to second source when first doesn't have package
    let result = chain.get_package(package.id()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().id(), package.id());
}

#[tokio::test]
async fn test_version_deduplication() {
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

    // Add same version to both sources
    let package = Package::new(
        PackageId::new("test", Version::parse("1.0.0").unwrap()),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );

    source1.packages.insert(package.id().clone(), package.clone());
    source2.packages.insert(package.id().clone(), package.clone());

    let mut chain = blast_resolver::source::PackageSourceChain::new();
    chain.add_source(Box::new(source1));
    chain.add_source(Box::new(source2));

    // Should deduplicate versions from multiple sources
    let versions = chain.get_versions("test").await.unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].to_string(), "1.0.0");
}
