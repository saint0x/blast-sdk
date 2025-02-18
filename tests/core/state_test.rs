use std::collections::HashMap;
use std::str::FromStr;
use blast_core::version::{Version, VersionConstraint};
use blast_core::package::Package;
use blast_core::metadata::PackageMetadata;
use blast_core::state::EnvironmentState;
use blast_core::python::PythonVersion;

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
fn test_state_creation() {
    let python_version = PythonVersion::from_str("3.8").unwrap();
    let packages = HashMap::new();
    let env_vars = HashMap::new();

    let state = EnvironmentState::new(
        "test-env".to_string(),
        python_version.clone(),
        packages,
        env_vars,
    );

    assert_eq!(state.name, "test-env");
    assert_eq!(state.python_version, python_version);
    assert!(state.packages.is_empty());
    assert!(state.env_vars.is_empty());
}

#[test]
fn test_state_diff() {
    let python_version = PythonVersion::from_str("3.8").unwrap();
    let mut packages1 = HashMap::new();
    packages1.insert(
        "package-a".to_string(),
        Version::parse("1.0.0").unwrap(),
    );

    let mut packages2 = HashMap::new();
    packages2.insert(
        "package-a".to_string(),
        Version::parse("2.0.0").unwrap(),
    );
    packages2.insert(
        "package-b".to_string(),
        Version::parse("1.0.0").unwrap(),
    );

    let state1 = EnvironmentState::new(
        "test-env".to_string(),
        python_version.clone(),
        packages1,
        HashMap::new(),
    );

    let state2 = EnvironmentState::new(
        "test-env".to_string(),
        python_version,
        packages2,
        HashMap::new(),
    );

    let diff = state1.diff(&state2);
    assert_eq!(diff.added_packages.len(), 1);
    assert_eq!(diff.updated_packages.len(), 1);
    assert!(diff.removed_packages.is_empty());
}

#[test]
fn test_state_verification() {
    let python_version = PythonVersion::from_str("3.8").unwrap();
    let mut state = EnvironmentState::new(
        "test-env".to_string(),
        python_version,
        HashMap::new(),
        HashMap::new(),
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

    state.add_package(&package);
    
    let verification = state.verify().unwrap();
    assert!(verification.is_verified);
    assert_eq!(verification.metrics.as_ref().map(|m| m.packages_checked), Some(1));
}

#[test]
fn test_checkpoint_operations() {
    let python_version = PythonVersion::from_str("3.8").unwrap();
    let mut state = EnvironmentState::new(
        "test-env".to_string(),
        python_version,
        HashMap::new(),
        HashMap::new(),
    );

    let checkpoint = state.create_checkpoint().unwrap();
    assert_eq!(checkpoint.state.name, state.name);
    assert_eq!(checkpoint.state.python_version, state.python_version);

    // Modify state
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

    state.add_package(&package);
    assert_eq!(state.packages.len(), 1);

    // Restore from checkpoint
    state.restore_from_checkpoint(checkpoint).unwrap();
    assert!(state.packages.is_empty());
} 