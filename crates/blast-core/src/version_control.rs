use std::collections::{HashMap, HashSet};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::BlastResult;
use crate::package::Package;
use crate::version::{Version, VersionConstraint};
use crate::python::PythonVersion;
use crate::version_history::{VersionEvent, VersionHistory, VersionImpact, VersionChangeAnalysis};
use crate::metadata::PackageMetadata;

/// Version policy for package upgrades
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionPolicy {
    /// Whether to allow major version upgrades
    pub allow_major: bool,
    /// Whether to allow minor version upgrades
    pub allow_minor: bool,
    /// Whether to allow patch version upgrades
    pub allow_patch: bool,
    /// Whether to allow pre-releases
    pub allow_prereleases: bool,
    /// Package-specific version constraints
    pub package_constraints: HashMap<String, VersionConstraint>,
    /// Python version constraints
    pub package_python_constraints: HashMap<String, VersionConstraint>,
}

impl Default for VersionPolicy {
    fn default() -> Self {
        Self {
            allow_major: false,
            allow_minor: true,
            allow_patch: true,
            allow_prereleases: false,
            package_constraints: HashMap::new(),
            package_python_constraints: HashMap::new(),
        }
    }
}

/// Version upgrade strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpgradeStrategy {
    /// Never upgrade
    Never,
    /// Only security updates
    SecurityOnly,
    /// Patch versions only
    PatchOnly,
    /// Minor versions and patches
    MinorAndPatch,
    /// All versions including major
    All,
    /// Custom policy
    Custom(VersionPolicy),
}

impl Default for UpgradeStrategy {
    fn default() -> Self {
        Self::MinorAndPatch
    }
}

/// Version manager for tracking and enforcing version policies
#[derive(Debug)]
pub struct VersionManager {
    /// Version histories by package
    histories: HashMap<String, VersionHistory>,
    /// Global version policy
    policy: VersionPolicy,
    /// Package-specific upgrade strategies
    upgrade_strategies: HashMap<String, UpgradeStrategy>,
}

impl VersionManager {
    /// Create new version manager
    pub fn new(policy: VersionPolicy) -> Self {
        Self {
            histories: HashMap::new(),
            policy,
            upgrade_strategies: HashMap::new(),
        }
    }

    /// Get the current version policy
    pub fn policy(&self) -> &VersionPolicy {
        &self.policy
    }

    /// Update the version policy
    pub fn set_policy(&mut self, policy: VersionPolicy) {
        self.policy = policy;
    }

    /// Add package installation
    pub fn add_installation(
        &mut self,
        package: &Package,
        is_direct: bool,
        python_version: &PythonVersion,
        reason: String,
    ) {
        let event = VersionEvent {
            timestamp: Utc::now(),
            from_version: None,
            to_version: package.version().clone(),
            impact: VersionImpact::None,
            reason,
            python_version: python_version.clone(),
            is_direct,
            affected_dependencies: HashMap::new(),
            approved: true,
            approved_by: None,
            policy_snapshot: None,
        };

        self.histories
            .entry(package.name().to_string())
            .or_insert_with(|| VersionHistory::new(package.name().to_string()))
            .add_event(event);
    }

    /// Add package installation with audit
    pub fn add_installation_with_audit(
        &mut self,
        package: &Package,
        is_direct: bool,
        python_version: &PythonVersion,
        reason: String,
        approved_by: Option<String>,
    ) {
        info!(
            "Installing package {} v{} (Python {})",
            package.name(),
            package.version(),
            python_version
        );

        let event = VersionEvent {
            timestamp: Utc::now(),
            from_version: None,
            to_version: package.version().clone(),
            impact: VersionImpact::None,
            reason,
            python_version: python_version.clone(),
            is_direct,
            affected_dependencies: HashMap::new(),
            approved: true,
            approved_by,
            policy_snapshot: Some(format!("{:?}", self.policy)),
        };

        self.histories
            .entry(package.name().to_string())
            .or_insert_with(|| VersionHistory::new(package.name().to_string()))
            .add_event(event);
    }

    /// Check if upgrade is allowed
    pub fn check_upgrade_allowed(
        &self,
        package: &Package,
        target_version: &Version,
    ) -> BlastResult<bool> {
        let strategy = self.upgrade_strategies
            .get(package.name())
            .cloned()
            .unwrap_or_else(|| UpgradeStrategy::Custom(self.policy.clone()));

        match strategy {
            UpgradeStrategy::Never => Ok(false),
            UpgradeStrategy::SecurityOnly => {
                // TODO: Implement security vulnerability checking
                Ok(false)
            }
            UpgradeStrategy::PatchOnly => {
                let impact = VersionImpact::from_version_change(package.version(), target_version);
                Ok(impact == VersionImpact::None)
            }
            UpgradeStrategy::MinorAndPatch => {
                let impact = VersionImpact::from_version_change(package.version(), target_version);
                Ok(impact != VersionImpact::Major)
            }
            UpgradeStrategy::All => Ok(true),
            UpgradeStrategy::Custom(policy) => {
                self.check_policy_allows_upgrade(&policy, package, target_version)
            }
        }
    }

    /// Set upgrade strategy for a package
    pub fn set_upgrade_strategy(&mut self, package_name: String, strategy: UpgradeStrategy) {
        self.upgrade_strategies.insert(package_name, strategy);
    }

    /// Get version history for a package
    pub fn get_history(&self, package_name: &str) -> Option<&VersionHistory> {
        self.histories.get(package_name)
    }

    /// Analyze version change impact
    pub fn analyze_change_impact(
        &self,
        package: &Package,
        target_version: &Version,
    ) -> BlastResult<VersionChangeAnalysis> {
        if let Some(history) = self.histories.get(package.name()) {
            Ok(history.analyze_change_impact(package.version(), target_version))
        } else {
            Ok(VersionChangeAnalysis {
                impact: VersionImpact::from_version_change(package.version(), target_version),
                affected_dependents: HashSet::new(),
                breaking_changes: Vec::new(),
                compatibility_issues: Vec::new(),
            })
        }
    }

    /// Export version history report
    pub fn export_history_report(&self, package_name: &str) -> BlastResult<Option<String>> {
        Ok(self.histories.get(package_name).map(|h| h.generate_report()))
    }

    /// Validate all package versions against current policy
    pub fn validate_all_versions(&self) -> BlastResult<Vec<(String, String)>> {
        let mut violations = Vec::new();

        for (package_name, history) in &self.histories {
            if let Some(current_version) = &history.current_version {
                let version_str = current_version.to_string();
                let package = Package::new(
                    package_name.clone(),
                    version_str.clone(),
                    create_package_metadata(
                        package_name.clone(),
                        version_str,
                        HashMap::new(),
                        VersionConstraint::any(),
                    ),
                    VersionConstraint::any(),
                ).unwrap();

                if let Ok(allowed) = self.check_upgrade_allowed(&package, current_version) {
                    if !allowed {
                        warn!("Package {} version {} violates current policy", package_name, current_version);
                        violations.push((
                            package_name.clone(),
                            format!("Version {} violates current policy", current_version)
                        ));
                    }
                }
            }
        }

        Ok(violations)
    }

    // Helper methods
    fn check_policy_allows_upgrade(
        &self,
        policy: &VersionPolicy,
        package: &Package,
        target_version: &Version,
    ) -> BlastResult<bool> {
        // Check package-specific constraints
        if let Some(constraint) = policy.package_constraints.get(package.name()) {
            if !constraint.matches(target_version) {
                return Ok(false);
            }
        }

        // Check version increment rules
        let impact = VersionImpact::from_version_change(package.version(), target_version);
        match impact {
            VersionImpact::Major if !policy.allow_major => return Ok(false),
            VersionImpact::Minor if !policy.allow_minor => return Ok(false),
            VersionImpact::None if !policy.allow_patch => return Ok(false),
            _ => {}
        }

        // Check pre-release
        if !policy.allow_prereleases && target_version.as_semver().pre.len() > 0 {
            return Ok(false);
        }

        Ok(true)
    }

    pub fn analyze_version_change(&self, from: &Version, to: &Version) -> VersionChangeAnalysis {
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

        // Get version history for the package
        if let Some(history) = self.histories.values().next() {
            // Check dependent packages
            for dependent in history.get_dependents() {
                if let Some(req) = history.get_requirements().get(dependent) {
                    if !VersionConstraint::parse(req).unwrap().matches(to) {
                        analysis.affected_dependents.insert(dependent.clone());
                        analysis.compatibility_issues.push(format!(
                            "Package {} requires version {}, which is incompatible with {}",
                            dependent, req, to
                        ));
                    }
                }
            }
        }

        analysis
    }
}

// Helper function to create package metadata from dependencies
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