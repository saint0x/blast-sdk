use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use chrono::Utc;
use blast_core::{
    version::{Version, VersionConstraint},
    version_history::{VersionHistory, VersionEvent, VersionImpact},
    python::PythonVersion,
};

#[test]
fn test_version_history() {
    let mut history = VersionHistory::new("test-package".to_string());
    let python_version = PythonVersion::from_str("3.8").unwrap();
    
    let event = VersionEvent {
        timestamp: Utc::now(),
        from_version: None,
        to_version: Version::parse("1.0.0").unwrap(),
        impact: VersionImpact::None,
        reason: "Initial installation".to_string(),
        python_version,
        is_direct: true,
        affected_dependencies: HashMap::new(),
        approved: true,
        approved_by: Some("test-user".to_string()),
        policy_snapshot: None,
    };

    history.add_event(event);
    assert_eq!(history.events.len(), 1);
    assert!(history.current_version.is_some());
}

#[test]
fn test_version_impact() {
    let v100 = Version::parse("1.0.0").unwrap();
    let v110 = Version::parse("1.1.0").unwrap();
    let v200 = Version::parse("2.0.0").unwrap();

    assert_eq!(VersionImpact::from_version_change(&v100, &v110), VersionImpact::Minor);
    assert_eq!(VersionImpact::from_version_change(&v100, &v200), VersionImpact::Major);
    assert_eq!(VersionImpact::from_version_change(&v100, &Version::parse("1.0.1").unwrap()), VersionImpact::None);
}

#[test]
fn test_version_requirements() {
    let mut history = VersionHistory::new("test-package".to_string());
    
    history.add_requirement(VersionConstraint::parse(">=1.0.0, <2.0.0").unwrap());
    
    assert!(history.check_version(&Version::parse("1.0.0").unwrap()));
    assert!(history.check_version(&Version::parse("1.1.0").unwrap()));
    assert!(!history.check_version(&Version::parse("2.0.0").unwrap()));
}

#[test]
fn test_change_analysis() {
    let mut history = VersionHistory::new("test-package".to_string());
    history.add_dependent(
        "dependent-package".to_string(),
        Version::parse("1.0.0").unwrap()
    );
    history.add_requirement(VersionConstraint::parse("<2.0.0").unwrap());

    let v100 = Version::parse("1.0.0").unwrap();
    let v200 = Version::parse("2.0.0").unwrap();

    let analysis = history.analyze_change_impact(&v100, &v200);
    assert_eq!(analysis.impact, VersionImpact::Major);
    assert!(!analysis.affected_dependents.is_empty());
    assert!(!analysis.breaking_changes.is_empty());
    assert!(!analysis.compatibility_issues.is_empty());
    assert!(!analysis.is_safe());
}

#[test]
fn test_version_history_report() {
    let mut history = VersionHistory::new("test-package".to_string());
    let python_version = PythonVersion::from_str("3.8").unwrap();
    
    // Add multiple events
    let events = vec![
        ("1.0.0", None, "Initial release"),
        ("1.1.0", Some("1.0.0"), "Added new features"),
        ("2.0.0", Some("1.1.0"), "Breaking changes"),
    ];

    for (version, from, reason) in events {
        let event = VersionEvent {
            timestamp: Utc::now(),
            from_version: from.map(|v| Version::parse(v).unwrap()),
            to_version: Version::parse(version).unwrap(),
            impact: VersionImpact::None,
            reason: reason.to_string(),
            python_version: python_version.clone(),
            is_direct: true,
            affected_dependencies: HashMap::new(),
            approved: true,
            approved_by: Some("test-user".to_string()),
            policy_snapshot: None,
        };
        history.add_event(event);
    }

    let report = history.generate_report();
    assert!(report.contains("Initial release"));
    assert!(report.contains("Added new features"));
    assert!(report.contains("Breaking changes"));
}

#[test]
fn test_version_compatibility() {
    let mut history = VersionHistory::new("test-package".to_string());
    
    // Add requirements from multiple dependents
    history.add_requirement(VersionConstraint::parse(">=1.0.0, <2.0.0").unwrap());
    history.add_requirement(VersionConstraint::parse(">=1.1.0, <3.0.0").unwrap());
    
    // Test version compatibility
    assert!(history.check_version(&Version::parse("1.1.0").unwrap()));
    assert!(!history.check_version(&Version::parse("0.9.0").unwrap()));
    assert!(!history.check_version(&Version::parse("3.0.0").unwrap()));
    
    // Test finding latest compatible version
    let latest = history.find_latest_compatible();
    assert!(latest.is_some());
}

#[test]
fn test_dependent_tracking() {
    let mut history = VersionHistory::new("test-package".to_string());
    
    // Add multiple dependents
    history.add_dependent(
        "dependent-1".to_string(),
        Version::parse("1.0.0").unwrap()
    );
    history.add_dependent(
        "dependent-2".to_string(),
        Version::parse("1.1.0").unwrap()
    );
    
    let dependents = history.get_dependents();
    assert_eq!(dependents.len(), 2);
    assert!(dependents.contains(&"dependent-1".to_string()));
    assert!(dependents.contains(&"dependent-2".to_string()));
} 