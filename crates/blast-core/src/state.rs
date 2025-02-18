use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;
use std::time::Duration;

use crate::{
    error::{BlastError, BlastResult},
    package::Package,
    version::{Version, VersionConstraint},
    version_history::VersionHistory,
    python::{PythonEnvironment, PythonVersion},
    sync::IssueSeverity,
    metadata::PackageMetadata,
};

/// Environment state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentState {
    /// State ID
    pub id: String,
    /// Environment name
    pub name: String,
    /// Python version
    /// 
    /// 
    pub python_version: PythonVersion,
    /// Installed packages with their versions
    pub packages: HashMap<String, Version>,
    /// Package version histories
    pub version_histories: HashMap<String, VersionHistory>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// State creation timestamp
    pub created_at: DateTime<Utc>,
    /// State metadata
    pub metadata: StateMetadata,
    /// Verification status
    pub verification: Option<StateVerification>,
    active: bool,
}

/// Metadata for environment state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    /// State description
    pub description: Option<String>,
    /// State tags
    pub tags: HashSet<String>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

/// State verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateVerification {
    /// Whether the state is verified
    pub is_verified: bool,
    /// Verification timestamp
    pub verified_at: Option<DateTime<Utc>>,
    /// Verification issues found
    pub issues: Vec<StateIssue>,
    /// Verification metrics
    pub metrics: Option<VerificationMetrics>,
}

/// State verification issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateIssue {
    /// Description of the issue
    pub description: String,
    /// Severity of the issue
    pub severity: IssueSeverity,
    /// Context of the issue
    pub context: Option<String>,
    /// Recommendation for the issue
    pub recommendation: Option<String>,
}

/// Verification metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMetrics {
    /// Duration of the verification
    pub duration: Duration,
    /// Number of packages checked
    pub packages_checked: usize,
    /// Number of dependencies checked
    pub dependencies_checked: usize,
}

/// Difference between two environment states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    /// Added packages
    pub added_packages: HashMap<String, Version>,
    /// Removed packages
    pub removed_packages: HashMap<String, Version>,
    /// Updated packages
    pub updated_packages: HashMap<String, (Version, Version)>,
    /// Added environment variables
    pub added_env_vars: HashMap<String, String>,
    /// Removed environment variables
    pub removed_env_vars: HashSet<String>,
    /// Changed environment variables
    pub changed_env_vars: HashMap<String, (String, String)>,
    /// Python version change
    pub python_version_change: Option<(PythonVersion, PythonVersion)>,
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

impl EnvironmentState {
    /// Create a new environment state
    pub fn new(
        name: String,
        python_version: PythonVersion,
        packages: HashMap<String, Version>,
        env_vars: HashMap<String, String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            python_version,
            packages,
            version_histories: HashMap::new(),
            env_vars,
            created_at: Utc::now(),
            metadata: StateMetadata {
                description: None,
                tags: HashSet::new(),
                custom: HashMap::new(),
            },
            verification: None,
            active: false,
        }
    }

    /// Create a new environment state from a Python environment
    pub fn from_environment(env: &PythonEnvironment) -> BlastResult<Self> {
        let packages = env.get_packages()?
            .into_iter()
            .map(|p| (p.name().to_string(), p.version().clone()))
            .collect();

        Ok(Self::new(
            env.name().unwrap_or("unnamed").to_string(),
            env.python_version().clone(),
            packages,
            HashMap::new(), // Environment variables will be added when supported
        ))
    }

    /// Add package to state
    pub fn add_package(&mut self, package: &Package) {
        self.packages.insert(
            package.name().to_string(),
            package.version().clone(),
        );
    }

    /// Remove package from state
    pub fn remove_package(&mut self, package: &Package) {
        self.packages.remove(package.name());
    }

    /// Add version history
    pub fn add_version_history(&mut self, name: String, history: VersionHistory) {
        self.version_histories.insert(name, history);
    }

    /// Verify state
    pub fn verify(&mut self) -> BlastResult<StateVerification> {
        let start_time = std::time::Instant::now();
        let mut issues = Vec::new();
        let mut metrics = VerificationMetrics {
            duration: Duration::from_secs(0),
            packages_checked: 0,
            dependencies_checked: 0,
        };

        // Verify packages
        for (name, version) in &self.packages {
            metrics.packages_checked += 1;

            // Check version history
            if let Some(history) = self.version_histories.get(name) {
                if let Some(current) = &history.current_version {
                    if current != version {
                        issues.push(StateIssue {
                            description: format!("Version mismatch for package {}", name),
                            severity: IssueSeverity::Critical,
                            context: Some(format!("Expected {}, got {}", current, version)),
                            recommendation: None,
                        });
                    }
                }
            }
        }

        // Verify environment variables
        metrics.dependencies_checked = self.env_vars.len();
        
        // Calculate metrics
        metrics.duration = start_time.elapsed();

        let verification = StateVerification {
            is_verified: issues.is_empty(),
            verified_at: Some(Utc::now()),
            issues,
            metrics: Some(metrics),
        };

        self.verification = Some(verification.clone());
        Ok(verification)
    }

    /// Compare with another state
    pub fn diff(&self, other: &EnvironmentState) -> StateDiff {
        let mut diff = StateDiff {
            added_packages: HashMap::new(),
            removed_packages: HashMap::new(),
            updated_packages: HashMap::new(),
            added_env_vars: HashMap::new(),
            removed_env_vars: HashSet::new(),
            changed_env_vars: HashMap::new(),
            python_version_change: None,
        };

        // Check Python version change
        if self.python_version != other.python_version {
            diff.python_version_change = Some((
                self.python_version.clone(),
                other.python_version.clone(),
            ));
        }

        // Check package changes
        for (name, version) in &self.packages {
            match other.packages.get(name) {
                Some(other_version) if other_version != version => {
                    diff.updated_packages.insert(
                        name.clone(),
                        (version.clone(), other_version.clone()),
                    );
                }
                None => {
                    diff.removed_packages.insert(name.clone(), version.clone());
                }
                _ => {}
            }
        }

        for (name, version) in &other.packages {
            if !self.packages.contains_key(name) {
                diff.added_packages.insert(name.clone(), version.clone());
            }
        }

        // Check environment variable changes
        for (name, value) in &self.env_vars {
            match other.env_vars.get(name) {
                Some(other_value) if other_value != value => {
                    diff.changed_env_vars.insert(
                        name.clone(),
                        (value.clone(), other_value.clone()),
                    );
                }
                None => {
                    diff.removed_env_vars.insert(name.clone());
                }
                _ => {}
            }
        }

        for (name, value) in &other.env_vars {
            if !self.env_vars.contains_key(name) {
                diff.added_env_vars.insert(name.clone(), value.clone());
            }
        }

        diff
    }

    /// Create a checkpoint of the current state
    pub fn create_checkpoint(&self) -> BlastResult<StateCheckpoint> {
        Ok(StateCheckpoint {
            state: self.clone(),
            created_at: Utc::now(),
            metadata: CheckpointMetadata {
                description: None,
                tags: HashSet::new(),
                custom: HashMap::new(),
            },
        })
    }

    /// Restore from a checkpoint
    pub fn restore_from_checkpoint(&mut self, checkpoint: StateCheckpoint) -> BlastResult<()> {
        // Verify checkpoint compatibility
        if checkpoint.state.python_version != self.python_version {
            return Err(BlastError::environment(format!(
                "Python version mismatch: checkpoint uses {}, current environment uses {}",
                checkpoint.state.python_version,
                self.python_version,
            )));
        }

        // Apply checkpoint state
        self.packages = checkpoint.state.packages;
        self.version_histories = checkpoint.state.version_histories;
        self.env_vars = checkpoint.state.env_vars;
        self.metadata = checkpoint.state.metadata;

        info!("Restored environment state from checkpoint created at {}", checkpoint.created_at);
        Ok(())
    }

    /// Verify a checkpoint's state
    pub fn verify_checkpoint_state(&self, state: &EnvironmentState) -> BlastResult<StateVerification> {
        let mut verification = StateVerification::default();

        // Verify Python version
        if !state.python_version.is_compatible_with(&PythonVersion::new(3, 6, None)) {
            verification.add_issue(StateIssue {
                description: "Unsupported Python version".to_string(),
                severity: IssueSeverity::Critical,
                context: None,
                recommendation: None,
            });
        }

        // Create a map of packages for easier access
        let packages: HashMap<String, Package> = state.packages.iter()
            .map(|(name, version)| -> BlastResult<(String, Package)> {
                let package = Package::new(
                    name.clone(),
                    version.to_string(),
                    create_package_metadata(
                        name.clone(),
                        version.to_string(),
                        HashMap::new(),
                        VersionConstraint::any(),
                    ),
                    VersionConstraint::any(),
                )?;
                Ok((name.clone(), package))
            })
            .collect::<BlastResult<HashMap<_, _>>>()?;

        // Verify package dependencies
        for (name, package) in &packages {
            // Check if all dependencies are satisfied
            let deps = package.all_dependencies(&[]);
            for (dep_name, constraint) in deps {
                if let Some(dep_version) = state.packages.get(&dep_name) {
                    if !constraint.matches(dep_version) {
                        verification.add_issue(StateIssue {
                            description: format!(
                                "Package {} dependency {} {} not satisfied (found {})",
                                name,
                                dep_name,
                                constraint,
                                dep_version
                            ),
                            severity: IssueSeverity::Critical,
                            context: None,
                            recommendation: None,
                        });
                    }
                } else {
                    verification.add_issue(StateIssue {
                        description: format!(
                            "Package {} dependency {} not found",
                            name,
                            dep_name
                        ),
                        severity: IssueSeverity::Critical,
                        context: None,
                        recommendation: None,
                    });
                }
            }

            // Check Python version compatibility
            let python_version_str = state.python_version.to_string();
            let python_version = Version::parse(&python_version_str).unwrap_or_else(|_| Version::parse("3.6.0").unwrap());
            
            if !package.metadata().python_version.matches(&python_version) {
                verification.add_issue(StateIssue {
                    severity: IssueSeverity::Warning,
                    description: format!(
                        "Package {} requires Python version {} but environment has {}",
                        package.name(),
                        package.metadata().python_version,
                        state.python_version
                    ),
                    context: None,
                    recommendation: Some("Consider upgrading Python version or using a different package version".to_string()),
                });
            }
        }

        // Verify version histories
        for (name, history) in &self.version_histories {
            if let Some(version) = state.packages.get(name) {
                if !history.has_version(version) {
                    verification.add_issue(StateIssue {
                        description: format!(
                            "Package {} version {} not found in version history",
                            name,
                            version
                        ),
                        severity: IssueSeverity::Warning,
                        context: None,
                        recommendation: None,
                    });
                }
            }
        }

        Ok(verification)
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

/// Checkpoint for environment state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCheckpoint {
    /// Captured state
    pub state: EnvironmentState,
    /// Checkpoint creation timestamp
    pub created_at: DateTime<Utc>,
    /// Checkpoint metadata
    pub metadata: CheckpointMetadata,
}

/// Metadata for state checkpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Checkpoint description
    pub description: Option<String>,
    /// Checkpoint tags
    pub tags: HashSet<String>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

impl Default for StateVerification {
    fn default() -> Self {
        Self {
            is_verified: true,
            verified_at: None,
            issues: Vec::new(),
            metrics: None,
        }
    }
}

impl StateVerification {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_issue(&mut self, issue: StateIssue) {
        if issue.severity == IssueSeverity::Critical {
            self.is_verified = false;
        }
        self.issues.push(issue);
    }
}

impl StateIssue {
    pub fn new(description: String, severity: IssueSeverity) -> Self {
        Self {
            description,
            severity,
            context: None,
            recommendation: None,
        }
    }

    pub fn with_context(description: String, severity: IssueSeverity, context: String) -> Self {
        Self {
            description,
            severity,
            context: Some(context),
            recommendation: None,
        }
    }
}

impl EnvironmentState {
    pub fn name(&self) -> &str {
        &self.name
    }
} 