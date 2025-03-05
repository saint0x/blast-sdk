use std::collections::HashMap;
use blast_core::version::VersionConstraint;
use blast_core::package::Package;
use blast_daemon::validation::{DependencyValidator, ValidationIssue, ValidationResult};
use petgraph::graph::NodeIndex;

#[test]
fn test_circular_dependency_detection() {
    let mut validator = DependencyValidator::new();

    // Create packages
    let package_a = Package::new(
        "package-a".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    ).unwrap();

    let package_b = Package::new(
        "package-b".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    ).unwrap();

    // Add packages and create circular dependency
    let a_idx = validator.add_package(package_a);
    let b_idx = validator.add_package(package_b);
    validator.add_dependency(a_idx, b_idx);
    validator.add_dependency(b_idx, a_idx);

    // Validate
    let result = validator.validate().unwrap();
    assert!(!result.is_valid);
    assert!(matches!(
        result.issues[0],
        ValidationIssue::CircularDependency { .. }
    ));
}

#[test]
fn test_version_conflict_detection() {
    let mut validator = DependencyValidator::new();

    // Create packages with conflicting versions
    let package_a_v1 = Package::new(
        "package-a".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    ).unwrap();

    let package_a_v2 = Package::new(
        "package-a".to_string(),
        "2.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    ).unwrap();

    // Add packages
    validator.add_package(package_a_v1);
    validator.add_package(package_a_v2);

    // Validate
    let result = validator.validate().unwrap();
    assert!(!result.is_valid);
    assert!(matches!(
        result.issues[0],
        ValidationIssue::VersionConflict { .. }
    ));
}

#[test]
fn test_python_version_conflict_detection() {
    let mut validator = DependencyValidator::new();

    // Create packages with conflicting Python versions
    let package_a = Package::new(
        "package-a".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    ).unwrap();

    let package_b = Package::new(
        "package-b".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.9").unwrap(),
    ).unwrap();

    // Add packages and dependency
    let a_idx = validator.add_package(package_a);
    let b_idx = validator.add_package(package_b);
    validator.add_dependency(a_idx, b_idx);

    // Validate
    let result = validator.validate().unwrap();
    assert!(!result.is_valid);
    assert!(matches!(
        result.issues[0],
        ValidationIssue::PythonVersionConflict { .. }
    ));
}

#[test]
fn test_missing_dependency_detection() {
    let mut validator = DependencyValidator::new();

    // Create packages
    let package_a = Package::new(
        "package-a".to_string(),
        "1.0.0".to_string(),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    ).unwrap();

    // Add package but reference non-existent dependency
    let a_idx = validator.add_package(package_a);
    let fake_idx = NodeIndex::new(999); // Non-existent node
    validator.add_dependency(a_idx, fake_idx);

    // Validate
    let result = validator.validate().unwrap();
    assert!(!result.is_valid);
    assert!(matches!(
        result.issues[0],
        ValidationIssue::MissingDependency { .. }
    ));
}

#[test]
fn test_validation_metrics() {
    let mut validator = DependencyValidator::new();

    // Create a chain of dependencies
    let packages: Vec<_> = (0..3).map(|i| {
        Package::new(
            format!("package-{}", i),
            "1.0.0".to_string(),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        ).unwrap()
    }).collect();

    // Add packages and chain dependencies
    let indices: Vec<_> = packages.into_iter()
        .map(|p| validator.add_package(p))
        .collect();

    for i in 0..indices.len()-1 {
        validator.add_dependency(indices[i], indices[i+1]);
    }

    // Validate and check metrics
    let result = validator.validate().unwrap();
    assert!(result.is_valid);
    assert_eq!(result.metrics.packages_checked, 3);
    assert_eq!(result.metrics.dependencies_checked, 2);
    assert_eq!(result.metrics.max_depth, 2);
}
