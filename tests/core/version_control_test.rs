use std::collections::HashMap;
use std::str::FromStr;
use blast_core::{
    version_control::{
        VersionManager,
        VersionPolicy,
        UpgradeStrategy,
        VersionChangeAnalysis,
        VersionChangeType,
    },
    package::{Package, PackageMetadata},
    version::{Version, VersionConstraint, PythonVersion},
};

fn create_package_metadata(
    name: String,
    version: String,
    dependencies: HashMap<String, VersionConstraint>,
    python_version: VersionConstraint,
) -> PackageMetadata {
    PackageMetadata::new(
        name,
        version,
        dependencies,
        python_version,
    )
}

#[test]
fn test_version_manager() {
    let policy = VersionPolicy::default();
    let mut manager = VersionManager::new(policy);
    let python_version = PythonVersion::from_str("3.8").unwrap();

    let package = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        create_package_metadata(
            "test-package".to_string(),
            "1.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    ).unwrap();

    manager.add_installation(&package, true, &python_version, "Initial install".to_string());
    
    let history = manager.get_history("test-package").unwrap();
    assert_eq!(history.events.len(), 1);
    assert_eq!(history.current_version.as_ref().unwrap().to_string(), "1.0.0");
}

#[test]
fn test_upgrade_strategies() {
    let policy = VersionPolicy::default();
    let mut manager = VersionManager::new(policy);

    let package = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        create_package_metadata(
            "test-package".to_string(),
            "1.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    ).unwrap();

    // Test PatchOnly strategy
    manager.set_upgrade_strategy(
        "test-package".to_string(),
        UpgradeStrategy::PatchOnly,
    );

    assert!(manager.check_upgrade_allowed(&package, &Version::parse("1.0.1").unwrap()).unwrap());
    assert!(!manager.check_upgrade_allowed(&package, &Version::parse("1.1.0").unwrap()).unwrap());
    assert!(!manager.check_upgrade_allowed(&package, &Version::parse("2.0.0").unwrap()).unwrap());

    // Test MinorAndPatch strategy
    manager.set_upgrade_strategy(
        "test-package".to_string(),
        UpgradeStrategy::MinorAndPatch,
    );

    assert!(manager.check_upgrade_allowed(&package, &Version::parse("1.0.1").unwrap()).unwrap());
    assert!(manager.check_upgrade_allowed(&package, &Version::parse("1.1.0").unwrap()).unwrap());
    assert!(!manager.check_upgrade_allowed(&package, &Version::parse("2.0.0").unwrap()).unwrap());
}

#[test]
fn test_version_policy() {
    let mut policy = VersionPolicy::default();
    policy.package_constraints.insert(
        "test-package".to_string(),
        VersionConstraint::parse("<2.0.0").unwrap(),
    );

    let manager = VersionManager::new(policy);
    let package = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        create_package_metadata(
            "test-package".to_string(),
            "1.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    ).unwrap();

    assert!(manager.check_upgrade_allowed(&package, &Version::parse("1.1.0").unwrap()).unwrap());
    assert!(!manager.check_upgrade_allowed(&package, &Version::parse("2.0.0").unwrap()).unwrap());
}

#[test]
fn test_change_impact_analysis() {
    let policy = VersionPolicy::default();
    let mut manager = VersionManager::new(policy);
    let python_version = PythonVersion::from_str("3.8").unwrap();

    let package = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        create_package_metadata(
            "test-package".to_string(),
            "1.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    ).unwrap();

    manager.add_installation(&package, true, &python_version, "Initial install".to_string());

    let analysis = manager.analyze_change_impact(&package, &Version::parse("2.0.0").unwrap()).unwrap();
    assert_eq!(analysis.change_type, VersionChangeType::Major);
    assert!(analysis.breaking_changes);
}

#[test]
fn test_security_only_strategy() {
    let policy = VersionPolicy::default();
    let mut manager = VersionManager::new(policy);

    let package = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        create_package_metadata(
            "test-package".to_string(),
            "1.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    ).unwrap();

    manager.set_upgrade_strategy(
        "test-package".to_string(),
        UpgradeStrategy::SecurityOnly,
    );

    // Regular updates should be blocked
    assert!(!manager.check_upgrade_allowed(&package, &Version::parse("1.0.1").unwrap()).unwrap());
    assert!(!manager.check_upgrade_allowed(&package, &Version::parse("1.1.0").unwrap()).unwrap());
    assert!(!manager.check_upgrade_allowed(&package, &Version::parse("2.0.0").unwrap()).unwrap());

    // TODO: Add test for security updates once we have vulnerability data
}

#[test]
fn test_custom_policy() {
    let mut custom_policy = VersionPolicy::default();
    custom_policy.allow_major = false;
    custom_policy.allow_minor = true;
    custom_policy.allow_patch = true;
    custom_policy.allow_prereleases = false;

    let mut manager = VersionManager::new(VersionPolicy::default());
    manager.set_upgrade_strategy(
        "test-package".to_string(),
        UpgradeStrategy::Custom(custom_policy),
    );

    let package = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        create_package_metadata(
            "test-package".to_string(),
            "1.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    ).unwrap();

    // Test custom policy rules
    assert!(manager.check_upgrade_allowed(&package, &Version::parse("1.0.1").unwrap()).unwrap());
    assert!(manager.check_upgrade_allowed(&package, &Version::parse("1.1.0").unwrap()).unwrap());
    assert!(!manager.check_upgrade_allowed(&package, &Version::parse("2.0.0").unwrap()).unwrap());
    assert!(!manager.check_upgrade_allowed(&package, &Version::parse("1.1.0-alpha.1").unwrap()).unwrap());
}

#[test]
fn test_version_history_export() {
    let policy = VersionPolicy::default();
    let mut manager = VersionManager::new(policy);
    let python_version = PythonVersion::from_str("3.8").unwrap();

    let package = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        create_package_metadata(
            "test-package".to_string(),
            "1.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    ).unwrap();

    // Add multiple versions
    manager.add_installation(&package, true, &python_version, "Initial install".to_string());
    manager.add_installation_with_audit(
        &package,
        true,
        &python_version,
        "Security update".to_string(),
        Some("admin".to_string()),
    );

    let report = manager.export_history_report("test-package").unwrap().unwrap();
    assert!(report.contains("Initial install"));
    assert!(report.contains("Security update"));
    assert!(report.contains("admin"));
} 