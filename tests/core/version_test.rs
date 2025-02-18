use blast_core::version::{Version, VersionConstraint};

#[test]
fn test_version_parsing() {
    assert!(Version::parse("1.0.0").is_ok());
    assert!(Version::parse("invalid").is_err());
}

#[test]
fn test_version_constraint() {
    let version = Version::parse("1.0.0").unwrap();
    let constraint = VersionConstraint::parse(">=1.0.0").unwrap();
    assert!(constraint.matches(&version));
} 