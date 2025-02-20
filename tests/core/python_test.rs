use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use blast_core::python::{PythonVersion, PythonEnvironment};
use blast_core::package::Package;
use blast_core::version::VersionConstraint;
use blast_core::metadata::PackageMetadata;
use blast_core::environment::package::Version;
use chrono::Utc;

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
fn test_python_version_parsing() {
    assert!(PythonVersion::parse("3.8").is_ok());
    assert!(PythonVersion::parse("3.8.0").is_ok());
    assert!(PythonVersion::parse("3").is_err());
    assert!(PythonVersion::parse("invalid").is_err());
}

#[test]
fn test_python_version_compatibility() {
    let v1 = PythonVersion::from_str("3.8").unwrap();
    let v2 = PythonVersion::from_str("3.9").unwrap();
    let v3 = PythonVersion::from_str("3.7").unwrap();
    let v4 = PythonVersion::from_str("2.7").unwrap();

    assert!(v1.is_compatible_with(&v2));
    assert!(!v2.is_compatible_with(&v1));
    assert!(v1.is_compatible_with(&v3));
    assert!(!v1.is_compatible_with(&v4));
}

#[tokio::test]
async fn test_environment_management() {
    let mut env = PythonEnvironment::new(
        "test-env".to_string(),
        PathBuf::from("/tmp/test-env"),
        PythonVersion::from_str("3.8").unwrap(),
    ).await.unwrap();

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

    env.install_package(package.name().to_string(), Some(package.version().to_string())).await.unwrap();
    let packages = env.get_packages().await.unwrap();
    assert_eq!(packages.len(), 1);

    env.uninstall_package(package.name().to_string()).await.unwrap();
    let packages = env.get_packages().await.unwrap();
    assert_eq!(packages.len(), 0);
}

#[tokio::test]
async fn test_package_installation() {
    let env = PythonEnvironment::new(
        PathBuf::from("test_env"),
        PythonVersion::parse("3.8").unwrap(),
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

    env.install_package(&package).await.unwrap();
    assert!(env.has_package(&package).await.unwrap());
    env.uninstall_package(&package).await.unwrap();
} 