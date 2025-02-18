use std::collections::HashMap;
use std::path::PathBuf;
use blast_core::{
    package::{Package, PackageId, Version},
    security::{SecurityPolicy, VulnerabilitySeverity, SignatureRequirement},
};
use blast_daemon::security::verify::{PackageVerifier, VerificationResult};

#[tokio::test]
async fn test_package_verification() {
    let verifier = PackageVerifier::new();
    let package = Package::new(
        PackageId::new(
            "test-package",
            Version::parse("1.0.0").unwrap(),
        ),
        HashMap::new(),
        Default::default(),
    );

    // Test signature verification
    let result = verifier.verify_package(&package).await.unwrap();
    assert!(result.verified);
    assert!(result.signature.is_some());
    
    if let Some(sig) = result.signature {
        assert_eq!(sig.signature_type, "GPG");
        assert!(!sig.key_id.is_empty());
        assert!(sig.valid);
    }

    // Test vulnerability scanning
    let vulns = verifier.scan_vulnerabilities(&package).await.unwrap();
    assert!(!vulns.is_empty());
    
    let vuln = &vulns[0];
    assert!(vuln.id.starts_with("MOCK-"));
    assert_eq!(vuln.severity, VulnerabilitySeverity::Low);
    assert!(!vuln.affected_versions.is_empty());
    assert!(!vuln.fixed_versions.is_empty());
    assert!(!vuln.references.is_empty());

    // Test policy verification
    let policy = SecurityPolicy::default();
    let policy_result = verifier.verify_policy(&package, &policy).await.unwrap();
    assert!(policy_result.allowed);
    assert!(policy_result.violations.is_empty());
    assert!(policy_result.required_actions.is_empty());
}

#[tokio::test]
async fn test_unsigned_package_verification() {
    let verifier = PackageVerifier::new();
    let package = Package::new(
        PackageId::new(
            "unsigned-package",
            Version::parse("1.0.0").unwrap(),
        ),
        HashMap::new(),
        Default::default(),
    );

    let result = verifier.verify_package(&package).await.unwrap();
    assert!(result.verified); // Still verified as signature is optional
    assert!(!result.warnings.is_empty()); // Should have warning about missing signature
    assert!(result.warnings.iter().any(|w| w.contains("not signed")));
}

#[tokio::test]
async fn test_policy_violations() {
    let verifier = PackageVerifier::new();
    let package = Package::new(
        PackageId::new(
            "test-package",
            Version::parse("1.0.0").unwrap(),
        ),
        HashMap::new(),
        Default::default(),
    );

    // Create a strict policy
    let mut policy = SecurityPolicy::default();
    policy.verify_signatures = true;
    policy.vulnerability_scan = true;
    
    // Add package-specific policies
    let mut pkg_policies = HashMap::new();
    pkg_policies.insert(
        "test-package".to_string(),
        blast_core::security::PackagePolicy {
            allowed_versions: vec!["2.0".to_string()], // Different from package version
            required_signatures: vec!["specific-key-id".to_string()],
            ..Default::default()
        },
    );
    policy.package_policies = pkg_policies;

    let result = verifier.verify_policy(&package, &policy).await.unwrap();
    assert!(!result.allowed);
    
    // Should have version constraint violation
    assert!(result.violations.iter().any(|v| v.rule == "allowed_versions"));
    
    // Should have signature requirement violation
    assert!(result.violations.iter().any(|v| v.rule == "required_signatures"));
}

#[tokio::test]
async fn test_vulnerability_scanning() {
    let verifier = PackageVerifier::new();
    let package = Package::new(
        PackageId::new(
            "vulnerable-package",
            Version::parse("0.9.0").unwrap(), // Older than fixed version
        ),
        HashMap::new(),
        Default::default(),
    );

    let vulns = verifier.scan_vulnerabilities(&package).await.unwrap();
    assert!(!vulns.is_empty());
    
    // Test with policy that blocks vulnerable packages
    let mut policy = SecurityPolicy::default();
    policy.vulnerability_scan = true;

    let result = verifier.verify_policy(&package, &policy).await.unwrap();
    assert!(!result.allowed);
    assert!(result.violations.iter().any(|v| v.rule == "vulnerability_check"));
}

mod signature_verification {
    use super::*;

    #[tokio::test]
    async fn test_valid_signature() {
        let verifier = PackageVerifier::new();
        let package_path = PathBuf::from("test_data/valid_package.tar.gz");
        
        let result = verifier.verify_signature(&package_path).await.unwrap();
        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn test_invalid_signature() {
        let verifier = PackageVerifier::new();
        let package_path = PathBuf::from("test_data/invalid_signature.tar.gz");
        
        let result = verifier.verify_signature(&package_path).await.unwrap();
        assert!(!result.is_valid());
        assert!(result.error_message().contains("invalid signature"));
    }

    #[tokio::test]
    async fn test_missing_signature() {
        let verifier = PackageVerifier::new();
        let package_path = PathBuf::from("test_data/unsigned_package.tar.gz");
        
        let result = verifier.verify_signature(&package_path).await.unwrap();
        assert!(!result.is_valid());
        assert!(result.error_message().contains("missing signature"));
    }
}

mod vulnerability_scanning {
    use super::*;

    #[tokio::test]
    async fn test_vulnerability_detection() {
        let verifier = PackageVerifier::new();
        let package_path = PathBuf::from("test_data/vulnerable_package.tar.gz");
        
        let result = verifier.scan_vulnerabilities(&package_path).await.unwrap();
        assert!(!result.is_valid());
        assert!(!result.vulnerabilities().is_empty());
    }

    #[tokio::test]
    async fn test_clean_package() {
        let verifier = PackageVerifier::new();
        let package_path = PathBuf::from("test_data/clean_package.tar.gz");
        
        let result = verifier.scan_vulnerabilities(&package_path).await.unwrap();
        assert!(result.is_valid());
        assert!(result.vulnerabilities().is_empty());
    }

    #[tokio::test]
    async fn test_severity_levels() {
        let verifier = PackageVerifier::new();
        let package_path = PathBuf::from("test_data/mixed_vulnerabilities.tar.gz");
        
        let result = verifier.scan_vulnerabilities(&package_path).await.unwrap();
        let vulns = result.vulnerabilities();
        
        assert!(vulns.iter().any(|v| v.severity == "HIGH"));
        assert!(vulns.iter().any(|v| v.severity == "MEDIUM"));
        assert!(vulns.iter().any(|v| v.severity == "LOW"));
    }
}

mod policy_verification {
    use super::*;

    #[tokio::test]
    async fn test_policy_compliance() {
        let verifier = PackageVerifier::new();
        let package_path = PathBuf::from("test_data/compliant_package.tar.gz");
        
        let policy = SecurityPolicy {
            signature_requirement: SignatureRequirement::Required,
            max_vulnerability_severity: Some("MEDIUM".into()),
            allowed_sources: vec!["trusted-repo.com".into()],
        };
        
        let result = verifier.verify_policy(&package_path, &policy).await.unwrap();
        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn test_policy_violation() {
        let verifier = PackageVerifier::new();
        let package_path = PathBuf::from("test_data/non_compliant_package.tar.gz");
        
        let policy = SecurityPolicy {
            signature_requirement: SignatureRequirement::Required,
            max_vulnerability_severity: Some("LOW".into()),
            allowed_sources: vec!["trusted-repo.com".into()],
        };
        
        let result = verifier.verify_policy(&package_path, &policy).await.unwrap();
        assert!(!result.is_valid());
        assert!(result.violations().len() > 0);
    }

    #[tokio::test]
    async fn test_multiple_policy_rules() {
        let verifier = PackageVerifier::new();
        let package_path = PathBuf::from("test_data/mixed_compliance.tar.gz");
        
        let policy = SecurityPolicy {
            signature_requirement: SignatureRequirement::Required,
            max_vulnerability_severity: Some("MEDIUM".into()),
            allowed_sources: vec!["trusted-repo.com".into(), "verified-source.org".into()],
        };
        
        let result = verifier.verify_policy(&package_path, &policy).await.unwrap();
        let violations = result.violations();
        
        // Check specific policy violations
        assert!(violations.iter().any(|v| v.rule == "signature"));
        assert!(violations.iter().any(|v| v.rule == "vulnerability"));
        assert!(violations.iter().any(|v| v.rule == "source"));
    }
} 