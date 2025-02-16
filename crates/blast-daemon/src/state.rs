use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use blast_core::{
    error::{BlastError, BlastResult},
    state::{EnvironmentState, StateVerification, StateIssue},
    package::{Package, PackageId, VersionConstraint, Version},
    version_history::{VersionHistory, VersionEvent},
    sync::IssueSeverity,
    python::PythonVersion,
};

/// Checkpoint for environment state
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Checkpoint ID
    pub id: Uuid,
    /// Checkpoint description
    pub description: String,
    /// Transaction ID if associated with a transaction
    pub transaction_id: Option<Uuid>,
    /// Environment state at checkpoint
    pub state: EnvironmentState,
    /// Checkpoint creation time
    pub created_at: DateTime<Utc>,
}

/// Manager for environment state and checkpoints
#[derive(Debug)]
pub struct StateManager {
    current_state: EnvironmentState,
    checkpoints: HashMap<Uuid, Checkpoint>,
    version_histories: HashMap<String, VersionHistory>,
    max_checkpoints: usize,
}

impl StateManager {
    /// Create new state manager
    pub fn new(initial_state: EnvironmentState) -> Self {
        Self {
            current_state: initial_state,
            checkpoints: HashMap::new(),
            version_histories: HashMap::new(),
            max_checkpoints: 10,
        }
    }

    /// Get current state
    pub fn get_current_state(&self) -> &EnvironmentState {
        &self.current_state
    }

    /// Create a new checkpoint with verification
    pub fn create_checkpoint(
        &mut self,
        id: Uuid,
        description: String,
        transaction_id: Option<Uuid>,
    ) -> BlastResult<()> {
        // Verify current state before creating checkpoint
        let verification = self.verify_state()?;
        if !verification.is_verified {
            return Err(BlastError::environment(format!(
                "Cannot create checkpoint: state verification failed: {}",
                verification.issues.iter()
                    .map(|i| i.description.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        // Manage checkpoint limit
        if self.checkpoints.len() >= self.max_checkpoints {
            // Remove oldest checkpoint
            if let Some(oldest) = self.checkpoints.iter()
                .min_by_key(|(_, cp)| cp.created_at)
                .map(|(id, _)| *id)
            {
                self.checkpoints.remove(&oldest);
            }
        }

        let checkpoint = Checkpoint {
            id,
            description,
            transaction_id,
            state: self.current_state.clone(),
            created_at: Utc::now(),
        };

        self.checkpoints.insert(id, checkpoint);
        Ok(())
    }

    /// Get a checkpoint by ID
    pub fn get_checkpoint(&self, id: Uuid) -> BlastResult<Option<Checkpoint>> {
        Ok(self.checkpoints.get(&id).cloned())
    }

    /// List all checkpoints
    pub fn list_checkpoints(&self) -> BlastResult<Vec<Checkpoint>> {
        Ok(self.checkpoints.values().cloned().collect())
    }

    /// Restore from a checkpoint with verification
    pub fn restore_checkpoint(&mut self, id: Uuid) -> BlastResult<()> {
        // Get checkpoint and clone early to avoid borrow issues
        let checkpoint_state = match self.checkpoints.get(&id) {
            Some(checkpoint) => checkpoint.state.clone(),
            None => return Err(BlastError::environment("Checkpoint not found")),
        };

        // Verify checkpoint state
        let verification = self.verify_checkpoint_state(&checkpoint_state)?;
        if !verification.is_verified {
            return Err(BlastError::environment(format!(
                "Cannot restore checkpoint: verification failed: {}",
                verification.issues.iter()
                    .map(|i| i.description.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        // Create backup checkpoint of current state
        let backup_id = Uuid::new_v4();
        let backup = Checkpoint {
            id: backup_id,
            description: "Automatic backup before checkpoint restore".to_string(),
            transaction_id: None,
            state: self.current_state.clone(),
            created_at: Utc::now(),
        };

        // Store backup and restore state
        self.checkpoints.insert(backup_id, backup);
        self.current_state = checkpoint_state;
        
        Ok(())
    }

    /// Add a package with version event
    pub fn add_package_with_event(&mut self, package: &Package, event: VersionEvent) -> BlastResult<()> {
        // Update version history
        self.version_histories
            .entry(package.name().to_string())
            .or_insert_with(|| VersionHistory::new(package.name().to_string()))
            .add_event(event);

        // Update current state
        self.current_state.add_package(package);
        Ok(())
    }

    /// Update a package with version event
    pub fn update_package_with_event(
        &mut self,
        from: &Package,
        to: &Package,
        event: VersionEvent,
    ) -> BlastResult<()> {
        // Update version history
        self.version_histories
            .entry(to.name().to_string())
            .or_insert_with(|| VersionHistory::new(to.name().to_string()))
            .add_event(event);

        // Update current state
        self.current_state.remove_package(from);
        self.current_state.add_package(to);
        Ok(())
    }

    /// Remove a package
    pub fn remove_package(&mut self, package: &Package) -> BlastResult<()> {
        // Update current state
        self.current_state.remove_package(package);
        Ok(())
    }

    /// Get version history for a package
    pub fn get_version_history(&self, package_name: &str) -> Option<&VersionHistory> {
        self.version_histories.get(package_name)
    }

    /// Verify current state
    pub fn verify_state(&self) -> BlastResult<StateVerification> {
        self.verify_checkpoint_state(&self.current_state)
    }

    /// Verify a checkpoint's state
    pub fn verify_checkpoint_state(&self, state: &EnvironmentState) -> BlastResult<StateVerification> {
        let mut verification = StateVerification::default();

        // Verify Python version
        let min_python = PythonVersion::new(3, 6, None);
        if !state.python_version.is_compatible_with(&min_python) {
            verification.add_issue(StateIssue {
                description: format!(
                    "Python version {} is not supported (minimum required: {})",
                    state.python_version,
                    min_python
                ),
                severity: IssueSeverity::Critical,
                context: None,
                recommendation: Some(format!("Upgrade to Python {} or later", min_python)),
            });
        }

        // Convert versions to packages for verification
        let packages: HashMap<String, _> = state.packages.iter()
            .map(|(name, version)| {
                let id = PackageId::new(name.clone(), version.clone());
                let package = Package::new(
                    id,
                    HashMap::new(),
                    VersionConstraint::parse(&format!(">={}", state.python_version)).unwrap_or_else(|_| VersionConstraint::any()),
                );
                (name.clone(), package)
            })
            .collect();

        // Verify package dependencies and compatibility
        for (name, package) in &packages {
            // Check Python version compatibility
            let python_version_str = state.python_version.to_string();
            let python_version = Version::parse(&python_version_str).unwrap_or_else(|_| Version::parse("3.6.0").unwrap());
            
            if !package.python_version().matches(&python_version) {
                verification.add_issue(StateIssue {
                    description: format!(
                        "Package {} requires Python {} but found {}",
                        name,
                        package.python_version(),
                        state.python_version
                    ),
                    severity: IssueSeverity::Warning,
                    context: None,
                    recommendation: Some(format!(
                        "Use Python {} or update package to a compatible version",
                        package.python_version()
                    )),
                });
            }

            // Check for duplicate packages
            if packages.values()
                .filter(|p| p.id().name() == package.id().name())
                .count() > 1 
            {
                verification.add_issue(StateIssue {
                    description: format!("Duplicate package found: {}", package.id().name()),
                    severity: IssueSeverity::Critical,
                    context: None,
                    recommendation: Some(format!("Remove duplicate installations of {}", package.id().name())),
                });
            }

            // Check dependencies
            for (dep_name, constraint) in package.dependencies() {
                if let Some(dep_pkg) = packages.get(dep_name) {
                    if !constraint.matches(dep_pkg.id().version()) {
                        verification.add_issue(StateIssue {
                            description: format!(
                                "Package {} dependency {} {} not satisfied (found {})",
                                name,
                                dep_name,
                                constraint,
                                dep_pkg.id().version()
                            ),
                            severity: IssueSeverity::Critical,
                            context: None,
                            recommendation: Some(format!("Update {} to version {}", dep_name, constraint)),
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
                        recommendation: Some(format!("Install {} package", dep_name)),
                    });
                }
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
                        recommendation: Some(format!(
                            "Verify installation source for {} {}",
                            name,
                            version
                        )),
                    });
                }
            }
        }

        Ok(verification)
    }

    /// Get checkpoint metrics
    pub fn get_checkpoint_metrics(&self) -> CheckpointMetrics {
        CheckpointMetrics {
            total_checkpoints: self.checkpoints.len(),
            oldest_checkpoint: self.checkpoints.values()
                .min_by_key(|cp| cp.created_at)
                .map(|cp| cp.created_at),
            newest_checkpoint: self.checkpoints.values()
                .max_by_key(|cp| cp.created_at)
                .map(|cp| cp.created_at),
            total_size: self.checkpoints.values()
                .map(|cp| cp.state.packages.len())
                .sum(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckpointMetrics {
    pub total_checkpoints: usize,
    pub oldest_checkpoint: Option<DateTime<Utc>>,
    pub newest_checkpoint: Option<DateTime<Utc>>,
    pub total_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use blast_core::python::PythonVersion;
    use std::str::FromStr;

    #[test]
    fn test_state_manager() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let mut manager = StateManager::new(initial_state);
        
        // Create checkpoint
        let checkpoint_id = Uuid::new_v4();
        manager.create_checkpoint(checkpoint_id, "Test checkpoint".to_string(), None).unwrap();
        
        // Add package
        let package = Package::new(
            blast_core::package::PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            blast_core::package::VersionRequirement::any(),
        );

        let event = VersionEvent {
            timestamp: Utc::now(),
            from_version: None,
            to_version: package.version().clone(),
            impact: blast_core::version_history::VersionImpact::None,
            reason: "Test installation".to_string(),
            python_version: PythonVersion::from_str("3.8").unwrap(),
            is_direct: true,
            affected_dependencies: Default::default(),
            approved: true,
            approved_by: None,
            policy_snapshot: None,
        };

        manager.add_package_with_event(&package, event).unwrap();
        
        // Verify state
        assert!(manager.get_current_state().packages.contains_key("test-package"));
        
        // Verify version history
        let history = manager.get_version_history("test-package").unwrap();
        assert_eq!(history.events.len(), 1);
    }

    #[test]
    fn test_checkpoint_restore() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let mut manager = StateManager::new(initial_state);
        
        // Create checkpoint
        let checkpoint_id = Uuid::new_v4();
        manager.create_checkpoint(checkpoint_id, "Test checkpoint".to_string(), None).unwrap();
        
        // Add package
        let package = Package::new(
            blast_core::package::PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            blast_core::package::VersionRequirement::any(),
        );

        let event = VersionEvent {
            timestamp: Utc::now(),
            from_version: None,
            to_version: package.version().clone(),
            impact: blast_core::version_history::VersionImpact::None,
            reason: "Test installation".to_string(),
            python_version: PythonVersion::from_str("3.8").unwrap(),
            is_direct: true,
            affected_dependencies: Default::default(),
            approved: true,
            approved_by: None,
            policy_snapshot: None,
        };

        manager.add_package_with_event(&package, event).unwrap();
        
        // Restore checkpoint
        manager.restore_checkpoint(checkpoint_id).unwrap();
        
        // Verify state was restored
        assert!(!manager.get_current_state().packages.contains_key("test-package"));
    }
} 