use std::collections::HashMap;
use std::path::PathBuf;
use blast_core::{
    python::{PythonEnvironment, PythonVersion},
    sync::{
        SyncManager, SyncStatus, ConflictType, ConflictResolution,
        MergeStrategy, OperationStatus, ValidationResult,
    },
    package::{Package, PackageMetadata},
    version::{Version, VersionConstraint},
    error::BlastResult,
};

#[tokio::test]
async fn test_sync_manager_creation() {
    let manager = SyncManager::new();
    assert!(manager.operations.is_empty());
}

#[tokio::test]
async fn test_sync_plan_creation() {
    let mut manager = SyncManager::new();
    
    let source = PythonEnvironment::new(
        PathBuf::from("/tmp/source"),
        PythonVersion::parse("3.9.0").unwrap(),
    );
    
    let target = PythonEnvironment::new(
        PathBuf::from("/tmp/target"),
        PythonVersion::parse("3.9.0").unwrap(),
    );
    
    let result = manager.plan_sync(&source, &target).await;
    assert!(result.is_ok());
    
    let operation = result.unwrap();
    assert_eq!(operation.status, SyncStatus::Planning);
    assert!(operation.completed_at.is_none());
}

#[tokio::test]
async fn test_conflict_resolution() -> BlastResult<()> {
    let manager = SyncManager::new();
    
    // Create test packages for conflict resolution
    let metadata1 = PackageMetadata::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::any(),
    );
    
    let package1 = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        metadata1,
        VersionConstraint::any(),
    )?;
    
    let metadata2 = PackageMetadata::new(
        "test-package".to_string(),
        "2.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::any(),
    );
    
    let package2 = Package::new(
        "test-package".to_string(),
        "2.0.0".to_string(),
        metadata2,
        VersionConstraint::any(),
    )?;
    
    let resolved = manager.resolve_conflict(&package1, &package2)?;
    assert_eq!(resolved.version().to_string(), "2.0.0");
    
    Ok(())
}

#[tokio::test]
async fn test_merge_environments() -> BlastResult<()> {
    let mut manager = SyncManager::new();
    
    // Create source environment with a package
    let mut source = PythonEnvironment::new(
        PathBuf::from("/tmp/source"),
        PythonVersion::parse("3.9.0").unwrap(),
    );
    
    let metadata1 = PackageMetadata::new(
        "test-package".to_string(),
        "2.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::any(),
    );
    
    let package1 = Package::new(
        "test-package".to_string(),
        "2.0.0".to_string(),
        metadata1,
        VersionConstraint::any(),
    )?;
    source.add_package(package1);
    
    // Create target environment with different version
    let mut target = PythonEnvironment::new(
        PathBuf::from("/tmp/target"),
        PythonVersion::parse("3.9.0").unwrap(),
    );
    
    let metadata2 = PackageMetadata::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::any(),
    );
    
    let package2 = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        metadata2,
        VersionConstraint::any(),
    )?;
    target.add_package(package2);
    
    // Test merge
    manager.merge_environments(&source, &mut target).await?;
    
    // Verify changes
    let updated_package = target.get_package("test-package").unwrap();
    assert_eq!(updated_package.version().to_string(), "2.0.0");
    
    Ok(())
}

#[tokio::test]
async fn test_dependency_conflict_handling() -> BlastResult<()> {
    let mut manager = SyncManager::new();
    
    // Create source environment
    let mut source = PythonEnvironment::new(
        PathBuf::from("/tmp/source"),
        PythonVersion::parse("3.9.0").unwrap(),
    );
    
    // Add packages with dependency relationships
    let mut deps = HashMap::new();
    deps.insert(
        "dep-package".to_string(),
        VersionConstraint::parse(">=2.0.0").unwrap(),
    );
    
    let dep_metadata = PackageMetadata::new(
        "dep-package".to_string(),
        "2.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::any(),
    );
    
    let dep_package = Package::new(
        "dep-package".to_string(),
        "2.0.0".to_string(),
        dep_metadata,
        VersionConstraint::any(),
    )?;
    
    let main_metadata = PackageMetadata::new(
        "main-package".to_string(),
        "2.0.0".to_string(),
        deps,
        VersionConstraint::any(),
    );
    
    let main_package = Package::new(
        "main-package".to_string(),
        "2.0.0".to_string(),
        main_metadata,
        VersionConstraint::any(),
    )?;
    
    source.add_package(dep_package);
    source.add_package(main_package);
    
    // Create target environment with conflicting dependency version
    let mut target = PythonEnvironment::new(
        PathBuf::from("/tmp/target"),
        PythonVersion::parse("3.9.0").unwrap(),
    );
    
    let old_dep_metadata = PackageMetadata::new(
        "dep-package".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::any(),
    );
    
    let old_dep = Package::new(
        "dep-package".to_string(),
        "1.0.0".to_string(),
        old_dep_metadata,
        VersionConstraint::any(),
    )?;
    target.add_package(old_dep);
    
    // Test merge
    manager.merge_environments(&source, &mut target).await?;
    
    // Verify both packages were updated
    let dep = target.get_package("dep-package").unwrap();
    let main = target.get_package("main-package").unwrap();
    
    assert_eq!(dep.version().to_string(), "2.0.0");
    assert_eq!(main.version().to_string(), "2.0.0");
    
    Ok(())
}

#[tokio::test]
async fn test_sync_validation() -> BlastResult<()> {
    let mut manager = SyncManager::new();
    
    let source = PythonEnvironment::new(
        PathBuf::from("/tmp/source"),
        PythonVersion::parse("3.9.0").unwrap(),
    );
    
    let target = PythonEnvironment::new(
        PathBuf::from("/tmp/target"),
        PythonVersion::parse("3.8.0").unwrap(), // Different Python version
    );
    
    let operation = manager.plan_sync(&source, &target).await?;
    
    // Validate changes
    let validation = manager.validate_changes(&operation.changes, &source, &target).await?;
    
    // Should detect Python version mismatch
    assert!(!validation.is_valid);
    assert!(!validation.issues.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_merge_strategies() -> BlastResult<()> {
    let mut manager = SyncManager::new();
    
    let mut source = PythonEnvironment::new(
        PathBuf::from("/tmp/source"),
        PythonVersion::parse("3.9.0").unwrap(),
    );
    
    let mut target = PythonEnvironment::new(
        PathBuf::from("/tmp/target"),
        PythonVersion::parse("3.9.0").unwrap(),
    );
    
    // Add conflicting packages
    let pkg1 = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        PackageMetadata::new(
            "test-package".to_string(),
            "1.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    )?;
    
    let pkg2 = Package::new(
        "test-package".to_string(),
        "2.0.0".to_string(),
        PackageMetadata::new(
            "test-package".to_string(),
            "2.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    )?;
    
    source.add_package(pkg1.clone());
    target.add_package(pkg2.clone());
    
    // Test different merge strategies
    let strategies = vec![
        MergeStrategy::KeepSource,
        MergeStrategy::KeepTarget,
        MergeStrategy::PreferSource,
    ];
    
    for strategy in strategies {
        let mut target_clone = target.clone();
        manager.merge_environments_with_strategy(&source, &mut target_clone, strategy).await?;
        
        let result_pkg = target_clone.get_package("test-package").unwrap();
        match strategy {
            MergeStrategy::KeepSource => assert_eq!(result_pkg.version(), pkg1.version()),
            MergeStrategy::KeepTarget => assert_eq!(result_pkg.version(), pkg2.version()),
            MergeStrategy::PreferSource => assert_eq!(result_pkg.version(), pkg1.version()),
            _ => {}
        }
    }
    
    Ok(())
} 