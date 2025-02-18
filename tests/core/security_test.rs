use chrono::Utc;
use blast_core::security::{
    SecurityPolicy, IsolationLevel, ResourceLimits, AuditRecord,
    AuditEventType, AuditResult, ResourceUsage, VerificationResult,
    SignatureInfo, Vulnerability, VulnerabilitySeverity, PolicyResult,
    PolicyViolation,
};
use blast_core::python::PythonVersion;

#[test]
fn test_security_policy_default() {
    let policy = SecurityPolicy::default();
    assert_eq!(policy.isolation_level, IsolationLevel::Process);
    assert_eq!(policy.python_version, PythonVersion::parse("3.8.0").unwrap());
    
    // Test resource limits
    assert!(policy.resource_limits.max_memory > 0);
    assert!(policy.resource_limits.max_disk > 0);
    assert!(policy.resource_limits.max_processes > 0);
}

#[test]
fn test_vulnerability_severity_ordering() {
    assert!(VulnerabilitySeverity::Critical > VulnerabilitySeverity::High);
    assert!(VulnerabilitySeverity::High > VulnerabilitySeverity::Medium);
    assert!(VulnerabilitySeverity::Medium > VulnerabilitySeverity::Low);
    
    // Test equality
    assert_eq!(VulnerabilitySeverity::High, VulnerabilitySeverity::High);
    
    // Test reverse comparisons
    assert!(VulnerabilitySeverity::Low < VulnerabilitySeverity::Medium);
}

#[test]
fn test_resource_limits() {
    let limits = ResourceLimits {
        max_memory: 1024 * 1024 * 1024, // 1GB
        max_disk: 10 * 1024 * 1024 * 1024, // 10GB
        max_processes: 10,
    };
    
    let usage = ResourceUsage {
        memory_usage: 512 * 1024 * 1024, // 512MB
        cpu_usage: 50.0,
        disk_usage: 5 * 1024 * 1024 * 1024, // 5GB
        bandwidth_usage: 1024 * 1024, // 1MB/s
    };
    
    // Test resource limit checks
    assert!(usage.memory_usage < limits.max_memory);
    assert!(usage.disk_usage < limits.max_disk);
}

#[test]
fn test_audit_record() {
    let record = AuditRecord {
        id: "test-123".to_string(),
        timestamp: Utc::now(),
        event_type: AuditEventType::PackageInstall,
        details: "Installing numpy".to_string(),
        actor: "test-user".to_string(),
        resource: "numpy-1.21.0".to_string(),
        result: AuditResult::Success,
    };
    
    assert_eq!(record.actor, "test-user");
    assert_eq!(record.resource, "numpy-1.21.0");
    
    // Test failure case
    let failure_record = AuditRecord {
        result: AuditResult::Failure("Permission denied".to_string()),
        ..record
    };
    
    match failure_record.result {
        AuditResult::Failure(ref msg) => assert_eq!(msg, "Permission denied"),
        _ => panic!("Expected Failure"),
    }
}

#[test]
fn test_verification_result() {
    let signature = SignatureInfo {
        signature_type: "GPG".to_string(),
        key_id: "ABC123".to_string(),
        timestamp: Utc::now(),
        valid: true,
    };
    
    let result = VerificationResult {
        verified: true,
        details: "Package verified successfully".to_string(),
        signature: Some(signature),
        warnings: vec!["Old signature format".to_string()],
    };
    
    assert!(result.verified);
    assert!(result.signature.is_some());
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn test_vulnerability() {
    let vuln = Vulnerability {
        id: "CVE-2023-1234".to_string(),
        severity: VulnerabilitySeverity::High,
        description: "Buffer overflow vulnerability".to_string(),
        affected_versions: vec!["1.0.0".to_string(), "1.1.0".to_string()],
        fixed_versions: vec!["1.2.0".to_string()],
        references: vec!["https://example.com/cve-2023-1234".to_string()],
    };
    
    assert_eq!(vuln.severity, VulnerabilitySeverity::High);
    assert_eq!(vuln.affected_versions.len(), 2);
    assert_eq!(vuln.fixed_versions.len(), 1);
}

#[test]
fn test_policy_result() {
    let violation = PolicyViolation {
        rule: "no-network-access".to_string(),
        details: "Package attempts to access network".to_string(),
        severity: VulnerabilitySeverity::Medium,
        recommendations: vec!["Use offline mode".to_string()],
    };
    
    let result = PolicyResult {
        allowed: false,
        violations: vec![violation],
        required_actions: vec!["Remove network access".to_string()],
    };
    
    assert!(!result.allowed);
    assert_eq!(result.violations.len(), 1);
    assert_eq!(result.required_actions.len(), 1);
    assert_eq!(result.violations[0].severity, VulnerabilitySeverity::Medium);
}

#[test]
fn test_isolation_levels() {
    assert_eq!(IsolationLevel::default(), IsolationLevel::Process);
    
    let levels = vec![
        IsolationLevel::None,
        IsolationLevel::Process,
        IsolationLevel::Container,
    ];
    
    for level in levels {
        let policy = SecurityPolicy {
            isolation_level: level,
            ..Default::default()
        };
        assert_eq!(policy.isolation_level, level);
    }
} 