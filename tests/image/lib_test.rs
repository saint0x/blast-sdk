use blast_image::{VERSION, MIN_COMPATIBLE_VERSION, is_compatible_version};

#[test]
fn test_version_compatibility() {
    // Test exact version match
    assert!(is_compatible_version(MIN_COMPATIBLE_VERSION));
    
    // Test compatible patch versions
    assert!(is_compatible_version("0.1.0"));
    assert!(is_compatible_version("0.1.1"));
    assert!(is_compatible_version("0.1.99"));
    
    // Test incompatible versions
    assert!(!is_compatible_version("0.2.0"));
    assert!(!is_compatible_version("1.0.0"));
    assert!(!is_compatible_version("0.0.9"));
    
    // Test current version compatibility
    assert!(is_compatible_version(VERSION));
}

#[test]
fn test_version_constants() {
    // Verify version format
    assert!(VERSION.starts_with('0'));
    assert_eq!(VERSION.matches('.').count(), 2);
    
    // Verify minimum compatible version format
    assert_eq!(MIN_COMPATIBLE_VERSION, "0.1.0");
}

#[test]
fn test_invalid_version_strings() {
    // Test invalid version strings
    assert!(!is_compatible_version("invalid"));
    assert!(!is_compatible_version("1.0"));
    assert!(!is_compatible_version("0.1"));
    assert!(!is_compatible_version("0.1.0.0"));
    assert!(!is_compatible_version(""));
}
