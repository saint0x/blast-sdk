use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use blast_core::package::{Package, PackageId};
use blast_core::version::{Version, VersionConstraint};
use blast_core::metadata::PackageMetadata;
use blast_resolver::cache::Cache;
use tempfile::tempdir;

#[tokio::test]
async fn test_cache_operations() {
    let temp_dir = tempdir().unwrap();
    let mut cache = Cache::new(temp_dir.path().to_path_buf());

    let id = PackageId::new("test-package".to_string(), Version::parse("1.0.0").unwrap());
    let metadata = PackageMetadata::new(
        id.name().to_string(),
        id.version().to_string(),
        HashMap::new(),
        VersionConstraint::any(),
    );

    let package = Package::new(
        id.name().to_string(),
        id.version().to_string(),
        metadata,
        VersionConstraint::any()
    ).unwrap();

    // Test storing
    cache.add_package(package.clone()).await.unwrap();
    assert!(cache.packages.contains_key(&id));

    // Test retrieval
    let retrieved = cache.get_package(&id).unwrap();
    assert_eq!(retrieved.name(), package.name());
    assert_eq!(retrieved.version(), package.version());

    // Test clearing
    cache.cleanup().await.unwrap();
    assert!(cache.packages.is_empty());
}

#[tokio::test]
async fn test_cache_expiry() {
    let mut cache = Cache::new(dirs::cache_dir().unwrap().join("blast"));

    let id = PackageId::new("test-package".to_string(), Version::parse("1.0.0").unwrap());
    let metadata = PackageMetadata::new(
        id.name().to_string(),
        id.version().to_string(),
        HashMap::new(),
        VersionConstraint::any(),
    );

    let package = Package::new(
        id.name().to_string(),
        id.version().to_string(),
        metadata,
        VersionConstraint::any()
    ).unwrap();

    cache.add_package(package).await.unwrap();

    // Simulate time passing
    let entry = cache.packages.get_mut(&id).unwrap();
    entry.last_used = SystemTime::now()
        .checked_sub(Duration::from_secs(25 * 60 * 60))
        .unwrap();

    cache.cleanup().await.unwrap();
    assert!(!cache.packages.contains_key(&id));
} 