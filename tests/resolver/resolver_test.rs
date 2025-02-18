use std::collections::HashMap;
use blast_core::package::{Package, PackageId};
use blast_core::version::{Version, VersionConstraint};
use blast_core::metadata::PackageMetadata;
use blast_resolver::resolver::DependencyResolver;
use blast_resolver::pypi::PyPIClient;
use blast_resolver::cache::Cache;

async fn create_resolver() -> blast_core::error::BlastResult<DependencyResolver> {
    let pypi = PyPIClient::new(10, 30, false)?;
    let cache = Cache::new(std::env::temp_dir().join("blast-test"));
    Ok(DependencyResolver::new(pypi, cache))
}

#[tokio::test]
async fn test_dependency_resolution() {
    let pypi = PyPIClient::new(10, 30, false).unwrap();
    let cache = Cache::new(std::env::temp_dir().join("blast-test"));
    let resolver = DependencyResolver::new(pypi, cache);

    let metadata = PackageMetadata::new(
        "requests".to_string(),
        "2.28.2".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );

    let package = Package::new(
        "requests".to_string(),
        "2.28.2".to_string(),
        metadata,
        VersionConstraint::parse(">=3.7").unwrap(),
    ).unwrap();

    let deps = resolver.resolve(&package).await.unwrap();
    assert!(!deps.is_empty());
}

#[tokio::test]
async fn test_resolver_creation() {
    let resolver = create_resolver().await.unwrap();
    
    // Test basic import resolution
    let numpy = resolver.resolve_import("numpy").await.unwrap();
    assert!(numpy.is_some());
    
    // Test version parsing
    let _version = Version::parse("1.0.0").unwrap();
    assert!(resolver.is_available("numpy").await);
}

#[tokio::test]
async fn test_resolver_cache() {
    let resolver = create_resolver().await.unwrap();
    
    // First resolution
    let metadata = PackageMetadata::new(
        "pandas".to_string(),
        "1.5.3".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.8").unwrap(),
    );

    let package = Package::new(
        "pandas".to_string(),
        "1.5.3".to_string(),
        metadata,
        VersionConstraint::parse(">=3.8").unwrap(),
    ).unwrap();

    let deps1 = resolver.resolve(&package).await.unwrap();
    let deps2 = resolver.resolve(&package).await.unwrap();
    
    // Second resolution should be cached
    assert_eq!(deps1.len(), deps2.len());
}

#[tokio::test]
async fn test_resolver_extras() {
    let resolver = create_resolver().await.unwrap();
    
    let metadata = PackageMetadata::new(
        "requests".to_string(),
        "2.28.2".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );

    let package = Package::new(
        "requests".to_string(),
        "2.28.2".to_string(),
        metadata,
        VersionConstraint::parse(">=3.7").unwrap(),
    ).unwrap();

    // Resolve with extras
    let extras = vec!["security".to_string()];
    let deps = resolver.resolve_with_extras(&package, &extras).await.unwrap();
    assert!(!deps.is_empty());
}

#[tokio::test]
async fn test_package_versions() {
    let resolver = create_resolver().await.unwrap();
    
    // Get available versions
    let versions = resolver.get_package_versions("numpy").await.unwrap();
    assert!(!versions.is_empty());
    
    // Verify version ordering
    for window in versions.windows(2) {
        assert!(window[0] < window[1]);
    }
} 