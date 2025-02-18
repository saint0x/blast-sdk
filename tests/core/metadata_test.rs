use std::collections::HashMap;
use blast_core::version::VersionConstraint;
use blast_core::metadata::PackageMetadata;

#[test]
fn test_package_metadata() {
    let mut deps = HashMap::new();
    deps.insert("requests".to_string(), VersionConstraint::default());
    
    let metadata = PackageMetadata::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        deps,
        VersionConstraint::default(),
    );

    assert_eq!(metadata.name, "test-package");
    assert_eq!(metadata.version, "1.0.0");
    assert!(metadata.description.is_none());
}

#[test]
fn test_python_compatibility() {
    let metadata = PackageMetadata::new(
        "test".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );

    assert!(metadata.is_python_compatible("3.8.0").unwrap());
    assert!(!metadata.is_python_compatible("3.6.0").unwrap());
}

#[test]
fn test_platform_compatibility() {
    let mut metadata = PackageMetadata::new(
        "test".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::default(),
    );

    // No platform tags means compatible with all
    assert!(metadata.is_platform_compatible("linux_x86_64"));

    metadata.platform_tags = vec!["linux_x86_64".to_string()];
    assert!(metadata.is_platform_compatible("linux_x86_64"));
    assert!(!metadata.is_platform_compatible("win_amd64"));
} 