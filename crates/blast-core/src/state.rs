use std::path::PathBuf;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, Duration};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    error::{BlastError, BlastResult},
    package::Package,
    version::{Version, VersionConstraint},
    python::{PythonEnvironment, PythonVersion},
    metadata::PackageMetadata,
    environment::Environment,
    sync::IssueSeverity,
    VersionHistory,
};

/// Environment state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentState {
    /// State ID
    pub id: String,
    /// Environment name
    pub name: String,
    /// Environment path
    pub path: PathBuf,
    /// Python version
    pub python_version: PythonVersion,
    /// Installed packages with their versions
    pub packages: HashMap<String, Version>,
    /// Package version histories
    pub version_histories: HashMap<String, VersionHistory>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// State creation timestamp
    pub created_at: SystemTime,
    /// State metadata
    pub metadata: StateMetadata,
    /// Verification status
    pub verification: Option<StateVerification>,
    active: bool,
    /// Creation time
    pub created_at_system: SystemTime,
    /// Last modified time
    pub modified_at: SystemTime,
    /// Package state
    pub package_state: PackageState,
    /// Container state
    pub container_state: ContainerState,
    /// Resource state
    pub resource_state: ResourceState,
    /// Security state
    pub security_state: SecurityState,
    /// Sync state
    pub sync_state: SyncState,
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

/// Package state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageState {
    /// Installed packages
    pub installed: HashMap<String, PackageInfo>,
    /// Package requirements
    pub requirements: Vec<String>,
    /// Package constraints
    pub constraints: Vec<String>,
    /// Package sources
    pub sources: Vec<String>,
}

/// Package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Installation time
    pub installed_at: SystemTime,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Whether it's a direct dependency
    pub is_direct: bool,
}

/// Container state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerState {
    /// Container ID
    pub id: Option<String>,
    /// Container status
    pub status: ContainerStatus,
    /// Container PID
    pub pid: Option<u32>,
    /// Container network
    pub network: Option<NetworkState>,
    /// Container mounts
    pub mounts: Vec<MountState>,
}

/// Container status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerStatus {
    /// Container is created
    Created,
    /// Container is running
    Running,
    /// Container is paused
    Paused,
    /// Container is stopped
    Stopped,
    /// Container is deleted
    Deleted,
}

impl Default for ContainerStatus {
    fn default() -> Self {
        Self::Created
    }
}

/// Network state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkState {
    /// Network ID
    pub id: String,
    /// Network name
    pub name: String,
    /// Network type
    pub network_type: String,
    /// IP address
    pub ip_address: Option<String>,
    /// Gateway
    pub gateway: Option<String>,
    /// DNS servers
    pub dns: Vec<String>,
}

/// Mount state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountState {
    /// Mount source
    pub source: PathBuf,
    /// Mount target
    pub target: PathBuf,
    /// Mount type
    pub mount_type: String,
    /// Mount options
    pub options: Vec<String>,
}

/// Resource state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceState {
    /// CPU usage
    pub cpu_usage: f64,
    /// Memory usage
    pub memory_usage: u64,
    /// Disk usage
    pub disk_usage: u64,
    /// Network usage
    pub network_usage: NetworkUsage,
}

/// Network usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkUsage {
    /// Bytes received
    pub rx_bytes: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Packets transmitted
    pub tx_packets: u64,
}

/// Security state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityState {
    /// Current capabilities
    pub capabilities: Vec<String>,
    /// Seccomp status
    pub seccomp_enabled: bool,
    /// AppArmor profile
    pub apparmor_profile: Option<String>,
    /// SELinux context
    pub selinux_context: Option<String>,
}

/// Sync state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Sync enabled
    pub enabled: bool,
    /// Last sync time
    pub last_sync: Option<SystemTime>,
    /// Pending changes
    pub pending_changes: usize,
    /// Stability score
    pub stability_score: u8,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            enabled: false,
            last_sync: None,
            pending_changes: 0,
            stability_score: 100,
        }
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

impl EnvironmentState {
    /// Create a new environment state
    pub fn new(
        id: String,
        name: String,
        path: PathBuf,
        python_version: PythonVersion,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name,
            path,
            python_version,
            packages: HashMap::new(),
            version_histories: HashMap::new(),
            env_vars: HashMap::new(),
            created_at: now,
            metadata: StateMetadata {
                description: None,
                tags: HashSet::new(),
                custom: HashMap::new(),
            },
            verification: None,
            active: false,
            created_at_system: now,
            modified_at: now,
            package_state: PackageState::default(),
            container_state: ContainerState::default(),
            resource_state: ResourceState::default(),
            security_state: SecurityState::default(),
            sync_state: SyncState::default(),
        }
    }

    /// Create a new environment state from a Python environment
    pub async fn from_environment(env: &PythonEnvironment) -> BlastResult<Self> {
        let _packages = env.get_packages().await?
            .into_iter()
            .map(|p| (p.name().to_string(), p.version().clone()))
            .collect::<HashMap<String, Version>>();

        let name = match env.name() {
            "" => "unnamed".to_string(),
            name => name.to_string(),
        };

        Ok(Self::new(
            name.clone(),
            name,
            env.path().to_path_buf(),
            PythonVersion::parse(env.python_version())?,
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

    /// Update modified time
    pub fn touch(&mut self) {
        self.modified_at = SystemTime::now();
    }

    /// Add environment variable
    pub fn add_env_var(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
        self.touch();
    }

    /// Remove environment variable
    pub fn remove_env_var(&mut self, key: &str) -> Option<String> {
        let value = self.env_vars.remove(key);
        if value.is_some() {
            self.touch();
        }
        value
    }

    /// Get environment variable
    pub fn get_env_var(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_environment_state() {
        let state = EnvironmentState::new(
            "test-id".to_string(),
            "test-env".to_string(),
            PathBuf::from("/tmp/test-env"),
            PythonVersion::new(3, 9, Some(0)),
        );

        assert_eq!(state.id, "test-id");
        assert_eq!(state.name, "test-env");
        assert_eq!(state.python_version, PythonVersion::new(3, 9, Some(0)));
    }
} 