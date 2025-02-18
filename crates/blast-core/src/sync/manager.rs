use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

use crate::error::{BlastError, BlastResult};
use crate::python::PythonEnvironment;
use crate::package::Package;
use crate::version::Version;

use super::operations::{SyncOperation, SyncStatus};
use super::types::SyncChange;
use super::conflicts::{SyncConflict, ConflictType, ConflictResolution};
use super::validation::{SyncValidation, ValidationIssue, IssueSeverity, PerformanceImpact};

/// Manager for synchronization between environments
pub struct SyncManager {
    /// Active sync operations
    operations: HashMap<String, SyncOperation>,
    /// Version stability scores
    stability_scores: HashMap<String, StabilityScore>,
}

#[derive(Debug, Clone)]
struct StabilityScore {
    /// Version scores (higher is more stable)
    version_scores: HashMap<Version, f64>,
}

impl SyncManager {
    /// Create new sync manager
    pub fn new() -> Self {
        Self {
            operations: HashMap::new(),
            stability_scores: HashMap::new(),
        }
    }

    /// Plan sync operation between environments
    pub async fn plan_sync(
        &mut self,
        source: &PythonEnvironment,
        target: &PythonEnvironment,
    ) -> BlastResult<SyncOperation> {
        // Analyze differences and conflicts
        let (changes, conflicts) = self.analyze_diff(source, target).await?;

        // Validate changes and calculate performance impact
        let validation = self.validate_changes(&changes, source, target).await?;
        
        // Create sync operation
        let operation = SyncOperation {
            id: Uuid::new_v4().to_string(),
            source: source.name().unwrap_or("source").to_string(),
            target: target.name().unwrap_or("target").to_string(),
            started_at: Utc::now(),
            completed_at: None,
            status: SyncStatus::Planning,
            changes,
            conflicts,
            validation,
        };

        // Store operation
        self.operations.insert(operation.id.clone(), operation.clone());

        Ok(operation)
    }

    /// Apply sync operation
    pub async fn apply_sync(
        &mut self,
        operation_id: &str,
        target: &mut PythonEnvironment,
    ) -> BlastResult<()> {
        let operation = self.operations.get_mut(operation_id)
            .ok_or_else(|| BlastError::sync("Operation not found"))?;

        operation.status = SyncStatus::InProgress;

        for change in &operation.changes {
            match change {
                SyncChange::UpdatePackage { package, from_version: _, to_version } => {
                    // Create new package with updated version
                    let new_package = Package::new(
                        package.name().to_string(),
                        to_version.to_string(),
                        package.metadata().clone(),
                        package.metadata().python_version.clone(),
                    )?;
                    target.remove_package(&package);
                    target.add_package(new_package);
                },
                SyncChange::InstallPackage(package) => {
                    target.add_package(package.clone());
                }
                SyncChange::RemovePackage(package) => {
                    target.remove_package(&package);
                }
                SyncChange::UpdateEnvVars(vars) => {
                    for (key, value) in vars {
                        target.update_env_var(key, value);
                    }
                }
                SyncChange::UpdatePythonVersion { to_version, .. } => {
                    target.update_python_version(&to_version.to_string())?;
                }
            }
        }

        operation.status = SyncStatus::Completed;
        operation.completed_at = Some(Utc::now());

        Ok(())
    }

    /// Analyze differences between environments
    async fn analyze_diff(
        &self,
        source: &PythonEnvironment,
        target: &PythonEnvironment,
    ) -> BlastResult<(Vec<SyncChange>, Vec<SyncConflict>)> {
        let mut changes = Vec::new();
        let mut conflicts = Vec::new();

        // Compare Python versions
        let source_python = source.python_version();
        let target_python = target.python_version();
        if source_python != target_python {
            changes.push(SyncChange::UpdatePythonVersion {
                from_version: target_python.clone(),
                to_version: source_python.clone(),
            });
        }

        // Compare packages
        let source_packages = source.get_packages()?;
        let target_packages = target.get_packages()?;

        for source_pkg in &source_packages {
            if let Some(target_pkg) = target_packages.iter().find(|p| p.name() == source_pkg.name()) {
                if source_pkg.version() != target_pkg.version() {
                    // Version mismatch - check compatibility
                    if !self.is_compatible(source_pkg, target_pkg) {
                        conflicts.push(SyncConflict {
                            conflict_type: ConflictType::PackageVersionConflict,
                            description: format!(
                                "Package {} version conflict: {} vs {}",
                                source_pkg.name(),
                                source_pkg.version(),
                                target_pkg.version()
                            ),
                            resolutions: vec![
                                ConflictResolution::UseSource,
                                ConflictResolution::UseTarget,
                                ConflictResolution::Skip,
                            ],
                            selected_resolution: None,
                        });
                    } else {
                        changes.push(SyncChange::UpdatePackage {
                            package: (*source_pkg).clone(),
                            from_version: target_pkg.version().clone(),
                            to_version: source_pkg.version().clone(),
                        });
                    }
                }
            } else {
                // Package not in target - add it
                changes.push(SyncChange::InstallPackage((*source_pkg).clone()));
            }
        }

        // Check for packages to remove
        for target_pkg in &target_packages {
            if !source_packages.iter().any(|p| p.name() == target_pkg.name()) {
                changes.push(SyncChange::RemovePackage((*target_pkg).clone()));
            }
        }

        Ok((changes, conflicts))
    }

    /// Check if packages are compatible
    fn is_compatible(&self, pkg1: &Package, pkg2: &Package) -> bool {
        // Check version constraints
        if !pkg1.metadata().python_version.matches(&pkg2.version()) {
            return false;
        }

        // Check dependencies
        let deps1 = pkg1.all_dependencies(&[]);
        let deps2 = pkg2.all_dependencies(&[]);

        for (name, constraint) in &deps1 {
            if let Some(other_constraint) = deps2.get(name) {
                if let Ok(version) = Version::parse(&other_constraint.to_string()) {
                    if !constraint.matches(&version) {
                        return false;
                    }
                } else {
                    // If we can't parse the version, consider it incompatible
                    return false;
                }
            }
        }

        true
    }

    /// Validate changes
    async fn validate_changes(
        &self,
        changes: &[SyncChange],
        _source: &PythonEnvironment,
        target: &PythonEnvironment,
    ) -> BlastResult<SyncValidation> {
        let mut issues = Vec::new();
        let mut is_valid = true;

        // Validate each change
        for change in changes {
            match change {
                SyncChange::InstallPackage(package) => {
                    // Check Python compatibility
                    if !package.is_python_compatible(&target.python_version().to_string())? {
                        is_valid = false;
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Critical,
                            description: format!(
                                "Package {} is not compatible with Python {}",
                                package.name(),
                                target.python_version()
                            ),
                            recommendation: "Choose a different package version or update Python".to_string(),
                        });
                    }
                }
                SyncChange::UpdatePackage { package, from_version, to_version } => {
                    // Check version stability
                    let stability_score = self.get_version_stability(package.name(), to_version);
                    if stability_score < 0.3 {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Warning,
                            description: format!(
                                "Package {} version {} has low stability score ({})",
                                package.name(),
                                to_version,
                                stability_score
                            ),
                            recommendation: "Consider using a more stable version".to_string(),
                        });
                    }

                    // Check for major version changes
                    if from_version.as_semver().major != to_version.as_semver().major {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Warning,
                            description: format!(
                                "Major version change for package {} ({} -> {})",
                                package.name(),
                                from_version,
                                to_version
                            ),
                            recommendation: "Review breaking changes in changelog".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        // Estimate performance impact
        let performance_impact = self.estimate_performance_impact(changes);

        Ok(SyncValidation {
            is_valid,
            issues,
            performance_impact,
        })
    }

    fn get_version_stability(&self, package: &str, version: &Version) -> f64 {
        self.stability_scores
            .get(package)
            .and_then(|score| score.version_scores.get(version))
            .copied()
            .unwrap_or(0.5)
    }

    fn estimate_performance_impact(&self, changes: &[SyncChange]) -> PerformanceImpact {
        let num_changes = changes.len();
        
        PerformanceImpact {
            estimated_duration: std::time::Duration::from_secs((num_changes * 10) as u64),
            required_space: (num_changes * 1024 * 1024) as u64,
            network_bandwidth: (num_changes * 512 * 1024) as u64,
            cpu_usage: 0.1 * num_changes as f32,
            memory_usage: (num_changes * 256 * 1024 * 1024) as u64,
        }
    }
} 