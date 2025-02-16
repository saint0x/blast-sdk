use std::collections::HashSet;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::package::{Version, VersionRequirement};
use crate::python::PythonVersion;

/// Version change impact level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionImpact {
    /// No impact (patch version)
    None,
    /// Minor impact (new features)
    Minor,
    /// Major impact (breaking changes)
    Major,
    /// Breaking change
    Breaking,
}

impl VersionImpact {
    /// Determine impact level from version change
    pub fn from_version_change(from: &Version, to: &Version) -> Self {
        if from.as_semver().major != to.as_semver().major {
            Self::Breaking
        } else if from.as_semver().minor != to.as_semver().minor {
            Self::Major
        } else if from.as_semver().patch != to.as_semver().patch {
            Self::Minor
        } else {
            Self::None
        }
    }

    /// Check if the impact is a breaking change
    pub fn is_breaking(&self) -> bool {
        matches!(self, Self::Breaking)
    }
}

/// Version change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Previous version
    pub from_version: Option<Version>,
    /// New version
    pub to_version: Version,
    /// Change impact level
    pub impact: VersionImpact,
    /// Change reason
    pub reason: String,
    /// Python version at time of change
    pub python_version: PythonVersion,
    /// Whether this was a direct dependency
    pub is_direct: bool,
    /// Dependencies affected by this change
    pub affected_dependencies: HashMap<String, String>,
    /// Whether the change was approved
    pub approved: bool,
    /// Who approved the change
    pub approved_by: Option<String>,
    /// Policy at time of change
    pub policy_snapshot: Option<String>,
}

/// Analysis of version change impact
#[derive(Debug, Clone)]
pub struct VersionChangeAnalysis {
    /// Impact level
    pub impact: VersionImpact,
    /// Affected dependent packages
    pub affected_dependents: HashSet<String>,
    /// Breaking changes
    pub breaking_changes: Vec<String>,
    /// Compatibility issues
    pub compatibility_issues: Vec<String>,
}

impl VersionChangeAnalysis {
    /// Check if change is safe to apply
    pub fn is_safe(&self) -> bool {
        self.impact != VersionImpact::Breaking && 
        self.affected_dependents.is_empty() &&
        self.breaking_changes.is_empty() &&
        self.compatibility_issues.is_empty()
    }

    /// Generate analysis report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("Version Change Impact: {:?}\n\n", self.impact));

        if !self.affected_dependents.is_empty() {
            report.push_str("Affected Dependent Packages:\n");
            for dep in &self.affected_dependents {
                report.push_str(&format!("  - {}\n", dep));
            }
            report.push_str("\n");
        }

        if !self.breaking_changes.is_empty() {
            report.push_str("Breaking Changes:\n");
            for change in &self.breaking_changes {
                report.push_str(&format!("  - {}\n", change));
            }
            report.push_str("\n");
        }

        if !self.compatibility_issues.is_empty() {
            report.push_str("Compatibility Issues:\n");
            for issue in &self.compatibility_issues {
                report.push_str(&format!("  - {}\n", issue));
            }
            report.push_str("\n");
        }

        report.push_str(&format!(
            "Safe to Apply: {}\n",
            if self.is_safe() { "Yes" } else { "No" }
        ));

        report
    }
}

/// Version history for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    /// Package name
    pub package_name: String,
    /// Version events
    pub events: Vec<VersionEvent>,
    /// Current version
    pub current_version: Option<Version>,
    /// Version requirements
    pub requirements: HashMap<String, String>,
    /// Dependent packages
    pub dependents: Vec<String>,
}

impl VersionHistory {
    /// Create new version history
    pub fn new(package_name: String) -> Self {
        Self {
            package_name,
            events: Vec::new(),
            current_version: None,
            requirements: HashMap::new(),
            dependents: Vec::new(),
        }
    }

    /// Add a version event
    pub fn add_event(&mut self, event: VersionEvent) {
        self.current_version = Some(event.to_version.clone());
        self.events.push(event);
    }

    /// Check if a version exists in the history
    pub fn has_version(&self, version: &Version) -> bool {
        self.events.iter().any(|e| &e.to_version == version)
    }

    /// Get version events
    pub fn get_events(&self) -> &[VersionEvent] {
        &self.events
    }

    /// Get version requirements
    pub fn get_requirements(&self) -> &HashMap<String, String> {
        &self.requirements
    }

    /// Get dependent packages
    pub fn get_dependents(&self) -> &Vec<String> {
        &self.dependents
    }

    /// Check if version satisfies all requirements
    pub fn check_version(&self, version: &Version) -> bool {
        self.requirements.iter().all(|(_, req_str)| {
            if let Ok(req) = VersionRequirement::parse(req_str) {
                req.matches(version)
            } else {
                false
            }
        })
    }

    /// Find the latest compatible version
    pub fn find_latest_compatible(&self) -> Option<&Version> {
        self.events
            .iter()
            .rev()
            .find(|event| {
                self.check_version(&event.to_version)
            })
            .map(|event| &event.to_version)
    }

    /// Analyze impact of version change
    pub fn analyze_change_impact(&self, from: &Version, to: &Version) -> VersionChangeAnalysis {
        let mut analysis = VersionChangeAnalysis {
            impact: VersionImpact::from_version_change(from, to),
            affected_dependents: HashSet::new(),
            breaking_changes: Vec::new(),
            compatibility_issues: Vec::new(),
        };

        // Check for breaking changes
        if analysis.impact.is_breaking() {
            analysis.breaking_changes.push(format!(
                "Breaking version change from {} to {} may introduce breaking changes",
                from, to
            ));
        }

        // Check dependent packages
        for dependent in &self.dependents {
            if let Some(req) = self.requirements.get(dependent) {
                if !VersionRequirement::parse(req).unwrap().matches(to) {
                    analysis.affected_dependents.insert(dependent.clone());
                    analysis.compatibility_issues.push(format!(
                        "Package {} requires version {}, which is incompatible with {}",
                        dependent, req, to
                    ));
                }
            }
        }

        analysis
    }

    /// Generate version history report
    pub fn generate_report(&self) -> String {
        let mut report = format!("Version History Report for {}\n", self.package_name);
        report.push_str("=====================================\n\n");

        for event in &self.events {
            report.push_str(&format!(
                "Version Change: {} -> {}\n",
                event.from_version.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "None".to_string()),
                event.to_version
            ));
            report.push_str(&format!("Timestamp: {}\n", event.timestamp));
            report.push_str(&format!("Impact: {:?}\n", event.impact));
            report.push_str(&format!("Reason: {}\n", event.reason));
            report.push_str(&format!("Python Version: {}\n", event.python_version));
            report.push_str(&format!("Direct Dependency: {}\n", event.is_direct));
            
            if !event.affected_dependencies.is_empty() {
                report.push_str("Affected Dependencies:\n");
                for (dep, _) in &event.affected_dependencies {
                    report.push_str(&format!("  - {}\n", dep));
                }
            }

            if event.approved {
                report.push_str(&format!(
                    "Approved by: {}\n",
                    event.approved_by.as_deref().unwrap_or("Unknown")
                ));
            }

            report.push_str("\n");
        }

        if !self.requirements.is_empty() {
            report.push_str("\nVersion Requirements:\n");
            for (pkg, req) in &self.requirements {
                report.push_str(&format!("  - {} requires {}\n", pkg, req));
            }
        }

        if !self.dependents.is_empty() {
            report.push_str("\nDependent Packages:\n");
            for dep in &self.dependents {
                report.push_str(&format!("  - {}\n", dep));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
        
        history.add_requirement(VersionRequirement::parse(">=1.0.0, <2.0.0").unwrap());
        
        assert!(history.check_version(&Version::parse("1.0.0").unwrap()));
        assert!(history.check_version(&Version::parse("1.1.0").unwrap()));
        assert!(!history.check_version(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_change_analysis() {
        let mut history = VersionHistory::new("test-package".to_string());
        history.add_dependent("dependent-package".to_string());
        history.add_requirement(VersionRequirement::parse("<2.0.0").unwrap());

        let v100 = Version::parse("1.0.0").unwrap();
        let v200 = Version::parse("2.0.0").unwrap();

        let analysis = history.analyze_change_impact(&v100, &v200);
        assert_eq!(analysis.impact, VersionImpact::Major);
        assert!(!analysis.affected_dependents.is_empty());
        assert!(!analysis.breaking_changes.is_empty());
        assert!(!analysis.compatibility_issues.is_empty());
        assert!(!analysis.is_safe());
    }
}