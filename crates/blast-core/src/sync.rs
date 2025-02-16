use std::collections::HashMap;
use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{BlastError, BlastResult};
use crate::package::{Package, Version, VersionConstraint};
use crate::python::{PythonEnvironment, PythonVersion};
use crate::state::{EnvironmentState, StateDiff};

/// Sync operation between environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOperation {
    /// Operation ID
    pub id: String,
    /// Source environment
    pub source: String,
    /// Target environment
    pub target: String,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Operation status
    pub status: SyncStatus,
    /// Package changes
    pub changes: Vec<SyncChange>,
    /// Conflicts that need resolution
    pub conflicts: Vec<SyncConflict>,
    /// Validation results
    pub validation: SyncValidation,
}

/// Status of sync operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Sync is being planned
    Planning,
    /// Sync is in progress
    InProgress,
    /// Sync completed successfully
    Completed,
    /// Sync failed
    Failed(String),
    /// Sync was cancelled
    Cancelled,
}

/// Change to be applied during sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncChange {
    /// Install a new package
    InstallPackage(Package),
    /// Remove a package
    RemovePackage(Package),
    /// Update package version
    UpdatePackage {
        package: Package,
        from_version: crate::package::Version,
        to_version: crate::package::Version,
    },
    /// Update environment variables
    UpdateEnvVars(HashMap<String, String>),
    /// Update Python version
    UpdatePythonVersion {
        from_version: PythonVersion,
        to_version: PythonVersion,
    },
}

/// Conflict during sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Description of the conflict
    pub description: String,
    /// Possible resolutions
    pub resolutions: Vec<ConflictResolution>,
    /// Selected resolution
    pub selected_resolution: Option<ConflictResolution>,
}

/// Type of sync conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    /// Version mismatch between environments
    VersionMismatch,
    /// Package exists in both environments with different versions
    PackageVersionConflict,
    /// Package dependencies are incompatible
    DependencyConflict,
    /// Environment variables conflict
    EnvVarConflict,
    /// Python version incompatibility
    PythonVersionConflict,
}

/// Resolution for a conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Use source version
    UseSource,
    /// Use target version
    UseTarget,
    /// Use specific version
    UseVersion(Version),
    /// Skip this change
    Skip,
    /// Merge changes
    Merge,
}

/// Validation of sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncValidation {
    /// Whether the sync is valid
    pub is_valid: bool,
    /// List of validation issues
    pub issues: Vec<ValidationIssue>,
    /// Performance impact assessment
    pub performance_impact: PerformanceImpact,
}

/// Issue found during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Severity of the issue
    pub severity: IssueSeverity,
    /// Description of the issue
    pub description: String,
    /// Recommended action
    pub recommendation: String,
}

/// Severity of validation issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Critical issue that must be resolved
    Critical,
    /// Warning that should be reviewed
    Warning,
    /// Informational message
    Info,
}

/// Performance impact of sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImpact {
    /// Estimated time to complete sync
    pub estimated_duration: std::time::Duration,
    /// Required disk space
    pub required_space: u64,
    /// Network bandwidth required
    pub network_bandwidth: u64,
    /// CPU usage estimate
    pub cpu_usage: f32,
    /// Memory usage estimate
    pub memory_usage: u64,
}

/// Manager for synchronization between environments
pub struct SyncManager {
    /// Active sync operations
    operations: HashMap<String, SyncOperation>,
    /// Version stability scores
    stability_scores: HashMap<String, StabilityScore>,
    /// Environment dependencies
    dependencies: HashMap<String, EnvironmentDeps>,
}

#[allow(dead_code)]  // Used for future stability tracking
struct StabilityScore {
    /// Package name
    package: String,
    /// Version scores (higher is more stable)
    version_scores: HashMap<Version, f64>,
    /// Usage count
    usage_count: u64,
    /// Last updated
    last_updated: DateTime<Utc>,
}

/// Environment dependencies
#[derive(Debug, Clone)]
#[allow(dead_code)]  // Fields are used in derived traits and future implementations
struct EnvironmentDeps {
    /// Environment name
    environment: String,
    /// Direct dependencies
    direct_deps: HashMap<String, Version>,
    /// Transitive dependencies
    transitive_deps: HashMap<String, Version>,
    /// Last sync timestamp
    last_sync: Option<DateTime<Utc>>,
}

#[allow(dead_code)]  // Methods are used in future implementations
impl EnvironmentDeps {
    fn new(environment: String) -> Self {
        Self {
            environment,
            direct_deps: HashMap::new(),
            transitive_deps: HashMap::new(),
            last_sync: None,
        }
    }

    fn get_state(&self) -> EnvironmentState {
        let mut packages = HashMap::new();
        let env_vars = HashMap::new();
        
        // Convert direct dependencies to package versions
        for (name, version) in &self.direct_deps {
            packages.insert(name.clone(), version.clone());
        }
        
        // Include transitive dependencies
        for (name, version) in &self.transitive_deps {
            if !packages.contains_key(name) {
                packages.insert(name.clone(), version.clone());
            }
        }
        
        // Create a default Python version if none exists
        let python_version = PythonVersion::parse("3.8").unwrap_or_else(|_| {
            PythonVersion::new(3, 8, None)
        });

        EnvironmentState::new(
            self.environment.clone(),  // Use environment name
            python_version,
            packages,
            env_vars,
        )
    }

    fn apply_diff(&mut self, diff: &StateDiff) {
        // Handle direct dependency changes
        for (name, version) in &diff.added_packages {
            self.direct_deps.insert(name.clone(), version.clone());
        }
        for (name, _) in &diff.removed_packages {
            self.direct_deps.remove(name);
        }
        for (name, (_, new_version)) in &diff.updated_packages {
            self.direct_deps.insert(name.clone(), new_version.clone());
        }

        // Update transitive dependencies based on resolution
        self.update_transitive_deps();
        
        // Update last sync timestamp
        self.last_sync = Some(chrono::Utc::now());
    }

    fn update_transitive_deps(&mut self) {
        let mut new_transitive = HashMap::new();
        
        // For each direct dependency, resolve its dependencies
        for (name, version) in &self.direct_deps {
            if let Ok(deps) = self.resolve_dependencies(name, version) {
                for (dep_name, dep_version) in deps {
                    if !self.direct_deps.contains_key(&dep_name) {
                        new_transitive.insert(dep_name, dep_version);
                    }
                }
            }
        }
        
        self.transitive_deps = new_transitive;
    }

    fn resolve_dependencies(&self, _package: &str, _version: &Version) -> BlastResult<HashMap<String, Version>> {
        // This would typically call out to a package resolver
        // For now, return empty map to avoid compilation errors
        Ok(HashMap::new())
    }
}

/// Package upgrade operation
#[derive(Debug)]
pub struct UpgradeOperation {
    /// Package being upgraded
    pub package: String,
    /// Current version
    pub from_version: crate::package::Version,
    /// Target version
    pub to_version: crate::package::Version,
    /// Operation ID
    pub id: Uuid,
    /// Operation status
    pub status: OperationStatus,
    /// Start timestamp
    pub started_at: u64,
    /// Completion timestamp
    pub completed_at: Option<u64>,
}

impl UpgradeOperation {
    pub fn new(package: String, from_version: crate::package::Version, to_version: crate::package::Version) -> Self {
        Self {
            package,
            from_version,
            to_version,
            id: Uuid::new_v4(),
            status: OperationStatus::Pending,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            completed_at: None,
        }
    }

    pub fn complete(&mut self, status: OperationStatus) {
        self.status = status;
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    pub fn validate_versions(&self, package: &Package) -> bool {
        let current_constraint = VersionConstraint::parse(&format!("={}", self.from_version))
            .unwrap_or_else(|_| VersionConstraint::any());
        let target_constraint = VersionConstraint::parse(&format!("={}", self.to_version))
            .unwrap_or_else(|_| VersionConstraint::any());
        
        current_constraint.matches(package.version()) || target_constraint.matches(package.version())
    }
}

/// Operation status
#[derive(Debug, Clone, PartialEq)]
pub enum OperationStatus {
    /// Operation is pending
    Pending,
    /// Operation is in progress
    InProgress,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed(String),
}

/// Validate package changes
pub fn validate_changes(_source: &Package, _target: &Package) -> bool {
    // Implement validation logic here
    true
}

impl SyncManager {
    /// Create new sync manager
    pub fn new() -> Self {
        Self {
            operations: HashMap::new(),
            stability_scores: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Plan sync operation between environments
    pub async fn plan_sync(
        &mut self,
        source: &PythonEnvironment,
        target: &PythonEnvironment,
    ) -> BlastResult<SyncOperation> {
        let op_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        // Get current dependencies
        let source_deps = self.get_environment_deps(source).await?;
        let target_deps = self.get_environment_deps(target).await?;
        
        // Calculate required changes
        let (changes, conflicts) = self.analyze_diff(&source_deps, &target_deps)?;
        
        // Validate changes
        let validation = self.validate_changes(&changes, source, target)?;
        
        let operation = SyncOperation {
            id: op_id.clone(),
            source: source.name().unwrap_or("unnamed").to_string(),
            target: target.name().unwrap_or("unnamed").to_string(),
            started_at: now,
            completed_at: None,
            status: SyncStatus::Planning,
            changes,
            conflicts,
            validation,
        };
        
        self.operations.insert(op_id, operation.clone());
        Ok(operation)
    }

    /// Apply sync operation
    pub async fn apply_sync(
        &mut self,
        operation_id: &str,
        target: &mut PythonEnvironment,
    ) -> BlastResult<()> {
        let mut operation = self.operations.get(operation_id)
            .ok_or_else(|| BlastError::sync("Operation not found"))?
            .clone();

        for change in operation.changes {
            match change {
                SyncChange::UpdatePackage { package, from_version: _, to_version } => {
                    // Create new package with updated version
                    let new_id = crate::package::PackageId::new(
                        package.name().to_string(),
                        to_version,
                    );
                    let new_package = Package::new(
                        new_id,
                        package.dependencies().clone(),
                        package.python_version().clone(),
                    );
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
                        target.update_env_var(&key, &value);
                    }
                }
                SyncChange::UpdatePythonVersion { to_version, .. } => {
                    target.update_python_version(&to_version.to_string())?;
                }
            }
        }

        // Update environment deps
        if let Some(deps) = self.dependencies.get_mut(&target.name().unwrap_or("unnamed").to_string()) {
            deps.last_sync = Some(Utc::now());
        }

        operation.status = SyncStatus::Completed;
        operation.completed_at = Some(Utc::now());

        Ok(())
    }

    /// Get environment dependencies
    async fn get_environment_deps(&mut self, env: &PythonEnvironment) -> BlastResult<EnvironmentDeps> {
        let env_name = env.name().unwrap_or("unnamed").to_string();
        
        if let Some(deps) = self.dependencies.get(&env_name) {
            return Ok(deps.clone());
        }

        let mut direct_deps = HashMap::new();
        let mut transitive_deps = HashMap::new();

        // Get installed packages
        for package in env.get_packages()? {
            direct_deps.insert(package.name().to_string(), package.version().clone());
            
            // Add dependencies
            for (dep_name, _) in package.dependencies() {
                if let Some(dep) = env.get_package(dep_name) {
                    transitive_deps.insert(dep_name.to_string(), dep.version().clone());
                }
            }
        }

        let deps = EnvironmentDeps {
            environment: env_name.clone(),
            direct_deps,
            transitive_deps,
            last_sync: None,
        };

        self.dependencies.insert(env_name, deps.clone());
        Ok(deps)
    }

    #[allow(dead_code)]  // Reserved for future stability tracking
    fn get_version_stability(&self, package: &str, version: &Version) -> f64 {
        self.stability_scores
            .get(package)
            .and_then(|score| score.version_scores.get(version))
            .copied()
            .unwrap_or(0.5) // Default score for unknown versions
    }

    #[allow(dead_code)]  // Reserved for future stability tracking
    fn update_stability_score(&mut self, package: &str, version: &Version, factor: f64) {
        let score = self.stability_scores.entry(package.to_string()).or_insert_with(|| StabilityScore {
            package: package.to_string(),
            version_scores: HashMap::new(),
            usage_count: 0,
            last_updated: Utc::now(),
        });

        let version_score = score.version_scores.entry(version.clone()).or_insert(0.5);
        *version_score = (*version_score * score.usage_count as f64 + factor) / (score.usage_count as f64 + 1.0);
        
        score.usage_count += 1;
        score.last_updated = Utc::now();
    }

    /// Analyze differences between environments
    fn analyze_diff(
        &self,
        source_deps: &EnvironmentDeps,
        target_deps: &EnvironmentDeps,
    ) -> BlastResult<(Vec<SyncChange>, Vec<SyncConflict>)> {
        let mut changes = Vec::new();
        let mut conflicts = Vec::new();

        // Compare direct dependencies
        for (name, version) in &source_deps.direct_deps {
            match target_deps.direct_deps.get(name) {
                Some(target_version) => {
                    if version != target_version {
                        // Version conflict
                conflicts.push(SyncConflict {
                            conflict_type: ConflictType::PackageVersionConflict,
                    description: format!(
                                "Package {} has different versions: {} vs {}",
                                name, version, target_version
                    ),
                    resolutions: vec![
                                ConflictResolution::UseSource,
                        ConflictResolution::UseTarget,
                                ConflictResolution::Skip,
                    ],
                    selected_resolution: None,
                });
                    }
                }
                None => {
                    // Package needs to be installed
                    let package_id = crate::package::PackageId::new(
                        name.clone(),
                        version.clone()
                    );
                    let package = Package::new(
                        package_id,
                        HashMap::new(),
                        crate::package::VersionConstraint::any()
                    );
                    changes.push(SyncChange::InstallPackage(package));
                }
            }
        }

        Ok((changes, conflicts))
    }
    
    fn validate_changes(
        &self,
        changes: &[SyncChange],
        _source: &PythonEnvironment,  // Unused but kept for API consistency
        target: &PythonEnvironment,
    ) -> BlastResult<SyncValidation> {
        let mut validation = SyncValidation {
            is_valid: true,
            issues: Vec::new(),
            performance_impact: PerformanceImpact {
                estimated_duration: std::time::Duration::from_secs(1),
                required_space: 0,
                network_bandwidth: 0,
                cpu_usage: 0.0,
                memory_usage: 0,
            },
        };

        for change in changes {
            match change {
                SyncChange::UpdatePackage { package, from_version: _, to_version } => {
                    // Convert Python version to package version for comparison
                    let target_version = crate::package::Version::parse(
                        &target.python_version().to_string()
                    ).map_err(|e| BlastError::version(e.to_string()))?;

                    if !package.python_version().matches(&target_version) {
                        validation.is_valid = false;
                        validation.issues.push(ValidationIssue {
                            severity: IssueSeverity::Critical,
                            description: format!(
                                "Package {} version {} is not compatible with Python {}",
                                package.name(),
                                to_version,
                                target.python_version()
                            ),
                            recommendation: "Use a compatible package version".to_string(),
                        });
                    }
                },
                SyncChange::InstallPackage(package) => {
                    // Convert Python version to package version for comparison
                    let target_version = crate::package::Version::parse(
                        &target.python_version().to_string()
                    ).map_err(|e| BlastError::version(e.to_string()))?;

                    if !package.python_version().matches(&target_version) {
                        validation.is_valid = false;
                        validation.issues.push(ValidationIssue {
                            severity: IssueSeverity::Critical,
                            description: format!(
                                "Package {} requires Python {}, but environment has {}",
                                package.name(),
                                package.python_version(),
                                target.python_version()
                            ),
                            recommendation: "Use a compatible package version".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(validation)
    }
}

/// Strategy for merging environment changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Keep all changes from source
    KeepSource,
    /// Keep all changes from target
    KeepTarget,
    /// Prefer source changes but allow manual resolution
    PreferSource,
    /// Interactive merge with manual conflict resolution
    Interactive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::python::PythonVersion;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let manager = SyncManager::new();
        assert!(manager.operations.is_empty());
    }

    #[tokio::test]
    async fn test_sync_plan_creation() {
        let mut manager = SyncManager::new();
        
        let source = PythonEnvironment::new(
            PathBuf::from("/tmp/source"),
            PythonVersion::parse("3.9.0").unwrap(),
        );
        
        let target = PythonEnvironment::new(
            PathBuf::from("/tmp/target"),
            PythonVersion::parse("3.9.0").unwrap(),
        );
        
        let result = manager.plan_sync(&source, &target).await;
        assert!(result.is_ok());
        
        let operation = result.unwrap();
        assert_eq!(operation.status, SyncStatus::Planning);
        assert!(operation.completed_at.is_none());
    }

    #[tokio::test]
    async fn test_conflict_resolution() {
        let mut manager = SyncManager::new();
        
        // Create a sync operation with a conflict
        let operation = SyncOperation {
            id: Uuid::new_v4().to_string(),
            source: "source".to_string(),
            target: "target".to_string(),
            started_at: Utc::now(),
            completed_at: None,
            status: SyncStatus::Planning,
            changes: Vec::new(),
            conflicts: vec![SyncConflict {
                conflict_type: ConflictType::PackageVersionConflict,
                description: "Package version conflict".to_string(),
                resolutions: vec![ConflictResolution::UseSource],
                selected_resolution: None,
            }],
            validation: SyncValidation {
                is_valid: true,
                issues: Vec::new(),
                performance_impact: PerformanceImpact {
                    estimated_duration: std::time::Duration::from_secs(60),
                    required_space: 1024 * 1024 * 100,
                    network_bandwidth: 1024 * 1024 * 10,
                    cpu_usage: 0.5,
                    memory_usage: 1024 * 1024 * 50,
                },
            },
        };
        
        manager.operations.insert(operation.id.clone(), operation);
        
        // Resolve the conflict
        let result = manager.resolve_conflict(
            &operation.id,
            0,
            ConflictResolution::UseSource,
        );
        
        assert!(result.is_ok());
        
        let updated_operation = manager.operations.get(&operation.id).unwrap();
        assert!(updated_operation.conflicts[0].selected_resolution.is_some());
    }

    #[tokio::test]
    async fn test_merge_environments() {
        let mut manager = SyncManager::new();
        
        // Create source environment with a package
        let source = PythonEnvironment::new(
            PathBuf::from("/tmp/source"),
            PythonVersion::parse("3.9.0").unwrap(),
        );
        source.add_package(Package::new(
            crate::package::PackageId::new(
                "test-package",
                Version::parse("2.0.0").unwrap(),
            ),
            HashMap::new(),
            crate::package::VersionConstraint::any(),
        ));
        
        // Create target environment with different version
        let target = PythonEnvironment::new(
            PathBuf::from("/tmp/target"),
            PythonVersion::parse("3.9.0").unwrap(),
        );
        target.add_package(Package::new(
            crate::package::PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            crate::package::VersionConstraint::any(),
        ));
        
        // Test KeepSource strategy
        let result = manager.merge_environments(&source, &target, MergeStrategy::KeepSource).await;
        assert!(result.is_ok());
        
        let operation = result.unwrap();
        assert_eq!(operation.status, SyncStatus::Planning);
        
        // Verify changes
        assert!(!operation.changes.is_empty());
        match &operation.changes[0] {
            SyncChange::UpdatePackage { package, from_version, to_version } => {
                assert_eq!(package.name(), "test-package");
                assert_eq!(from_version.to_string(), "1.0.0");
                assert_eq!(to_version.to_string(), "2.0.0");
            }
            _ => panic!("Expected UpdatePackage change"),
        }
        
        // Test Interactive strategy
        let result = manager.merge_environments(&source, &target, MergeStrategy::Interactive).await;
        assert!(result.is_ok());
        
        let operation = result.unwrap();
        assert!(!operation.conflicts.is_empty());
        assert_eq!(operation.conflicts[0].conflict_type, ConflictType::PackageVersionConflict);
    }

    #[tokio::test]
    async fn test_conflict_handling() {
        let mut manager = SyncManager::new();
        
        // Create source environment with conflicting packages
        let source = PythonEnvironment::new(
            PathBuf::from("/tmp/source"),
            PythonVersion::parse("3.9.0").unwrap(),
        );
        
        // Add a package with dependencies
        let main_package = Package::new(
            crate::package::PackageId::new(
                "main-package".to_string(),
                Version::parse("2.0.0").unwrap(),
            ),
            {
                let mut deps = HashMap::new();
                deps.insert(
                    "dep-package".to_string(),
                    VersionConstraint::parse(">=1.0.0").unwrap(),
                );
                deps
            },
            crate::package::VersionConstraint::any(),
        );
        source.add_package(main_package.clone());
        
        // Create target environment with different versions
        let target = PythonEnvironment::new(
            PathBuf::from("/tmp/target"),
            PythonVersion::parse("3.9.0").unwrap(),
        );
        
        // Add same package with older version
        let old_package = Package::new(
            crate::package::PackageId::new(
                "main-package".to_string(),
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            crate::package::VersionConstraint::any(),
        );
        target.add_package(old_package);
        
        // Test interactive merge with conflict resolution
        let result = manager.merge_environments(&source, &target, MergeStrategy::Interactive).await;
        assert!(result.is_ok());
        
        let operation = result.unwrap();
        assert!(!operation.conflicts.is_empty());
        
        // Resolve the version conflict
        manager.resolve_conflict(
            &operation.id,
            0,
            ConflictResolution::UseVersion(Version::parse("1.5.0").unwrap()),
        )?;
        
        // Try to apply the sync
        let mut target_env = target.clone();
        let result = manager.apply_sync(&operation.id, &mut target_env).await;
        assert!(result.is_ok());
        
        // Verify the changes were applied correctly
        let updated_package = target_env.get_package("main-package").unwrap();
        assert_eq!(updated_package.version().to_string(), "1.5.0");
    }

    #[tokio::test]
    async fn test_dependency_conflict_handling() {
        let mut manager = SyncManager::new();
        
        // Create source environment
        let source = PythonEnvironment::new(
            PathBuf::from("/tmp/source"),
            PythonVersion::parse("3.9.0").unwrap(),
        );
        
        // Add packages with dependency relationships
        let dep_package = Package::new(
            crate::package::PackageId::new(
                "dep-package".to_string(),
                Version::parse("2.0.0").unwrap(),
            ),
            HashMap::new(),
            crate::package::VersionConstraint::any(),
        );
        
        let main_package = Package::new(
            crate::package::PackageId::new(
                "main-package".to_string(),
                Version::parse("2.0.0").unwrap(),
            ),
            {
                let mut deps = HashMap::new();
                deps.insert(
                    "dep-package".to_string(),
                    VersionConstraint::parse(">=2.0.0").unwrap(),
                );
                deps
            },
            crate::package::VersionConstraint::any(),
        );
        
        source.add_package(dep_package);
        source.add_package(main_package);
        
        // Create target environment with conflicting dependency version
        let target = PythonEnvironment::new(
            PathBuf::from("/tmp/target"),
            PythonVersion::parse("3.9.0").unwrap(),
        );
        
        let old_dep = Package::new(
            crate::package::PackageId::new(
                "dep-package".to_string(),
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            crate::package::VersionConstraint::any(),
        );
        target.add_package(old_dep);
        
        // Test merge with dependency conflict
        let result = manager.merge_environments(&source, &target, MergeStrategy::Interactive).await;
        assert!(result.is_ok());
        
        let operation = result.unwrap();
        
        // Verify dependency conflicts are detected
        assert!(operation.conflicts.iter().any(|c| 
            matches!(c.conflict_type, ConflictType::DependencyConflict)
        ));
        
        // Resolve all conflicts with UseSource
        for i in 0..operation.conflicts.len() {
            manager.resolve_conflict(&operation.id, i, ConflictResolution::UseSource)?;
        }
        
        // Apply the sync
        let mut target_env = target.clone();
        let result = manager.apply_sync(&operation.id, &mut target_env).await;
        assert!(result.is_ok());
        
        // Verify both packages were updated in the correct order
        let dep = target_env.get_package("dep-package").unwrap();
        let main = target_env.get_package("main-package").unwrap();
        
        assert_eq!(dep.version().to_string(), "2.0.0");
        assert_eq!(main.version().to_string(), "2.0.0");
    }
} 