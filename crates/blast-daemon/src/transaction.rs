use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, warn};

use blast_core::{
    package::{Package, PackageId, VersionConstraint},
    error::BlastError,
    state::{EnvironmentState, StateVerification, StateIssue},
    version_history::{VersionEvent, VersionImpact, VersionChangeAnalysis},
    sync::IssueSeverity,
};

use crate::{
    state::{StateManager, Checkpoint},
    DaemonResult,
};

#[derive(Debug, Clone)]
pub enum TransactionOperation {
    Install(Package),
    Uninstall(Package),
    Update { from: Package, to: Package },
}

#[derive(Debug, Clone)]
pub struct TransactionContext {
    pub id: Uuid,
    pub operations: Vec<TransactionOperation>,
    pub state_before: HashMap<String, Package>,
    pub created_at: DateTime<Utc>,
    pub status: TransactionStatus,
    pub checkpoint_id: Option<Uuid>,
    pub metrics: TransactionMetrics,
    pub verification: Option<StateVerification>,
}

#[derive(Debug, Clone)]
pub struct TransactionMetrics {
    pub duration: Option<std::time::Duration>,
    pub memory_usage: u64,
    pub cpu_usage: f32,
    pub network_operations: u32,
    pub cache_hits: u32,
    pub dependencies_checked: u32,
}

#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Pending,
    InProgress,
    Committed,
    RolledBack,
    Failed(String),
}

impl TransactionContext {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            operations: Vec::new(),
            state_before: HashMap::new(),
            created_at: Utc::now(),
            status: TransactionStatus::Pending,
            checkpoint_id: None,
            metrics: TransactionMetrics {
                duration: None,
                memory_usage: 0,
                cpu_usage: 0.0,
                network_operations: 0,
                cache_hits: 0,
                dependencies_checked: 0,
            },
            verification: None,
        }
    }

    pub fn add_operation(&mut self, operation: TransactionOperation) -> DaemonResult<()> {
        match self.status {
            TransactionStatus::Pending => {
                // Analyze version impact for updates
                if let TransactionOperation::Update { ref from, ref to } = operation {
                    let analysis = VersionChangeAnalysis {
                        impact: VersionImpact::from_version_change(from.version(), to.version()),
                        affected_dependents: Default::default(),
                        breaking_changes: Vec::new(),
                        compatibility_issues: Vec::new(),
                    };

                    if analysis.impact.is_breaking() {
                        warn!(
                            "Adding potentially unsafe update operation: {} {} -> {}",
                            from.id().name(),
                            from.id().version(),
                            to.id().version()
                        );
                    }
                }

                self.operations.push(operation);
                Ok(())
            }
            _ => Err(BlastError::environment(
                "Transaction is no longer pending"
            ).into()),
        }
    }

    pub fn set_state_before(&mut self, packages: HashMap<String, Package>) {
        self.state_before = packages;
    }

    pub fn update_metrics(&mut self, metrics: TransactionMetrics) {
        self.metrics = metrics;
    }

    pub fn set_verification(&mut self, verification: StateVerification) {
        self.verification = Some(verification);
    }
}

#[derive(Debug)]
pub struct TransactionManager {
    active_transactions: RwLock<HashMap<Uuid, TransactionContext>>,
    state_manager: RwLock<StateManager>,
}

impl Clone for TransactionManager {
    fn clone(&self) -> Self {
        // Create new RwLocks with empty contents
        Self {
            active_transactions: RwLock::new(HashMap::new()),
            state_manager: RwLock::new(StateManager::new(self.state_manager.blocking_read().get_current_state().clone())),
        }
    }
}

impl TransactionManager {
    pub fn new(initial_state: EnvironmentState) -> Self {
        Self {
            active_transactions: RwLock::new(HashMap::new()),
            state_manager: RwLock::new(StateManager::new(initial_state)),
        }
    }

    pub async fn begin_transaction(&self) -> DaemonResult<TransactionContext> {
        let mut txn = TransactionContext::new();
        
        // Create initial checkpoint
        let checkpoint_id = Uuid::new_v4();
        {
            let mut state_manager = self.state_manager.write().await;
            state_manager.create_checkpoint(
                checkpoint_id,
                format!("Transaction {} initial state", txn.id),
                Some(txn.id),
            )?;
        }
        txn.checkpoint_id = Some(checkpoint_id);

        // Store initial state
        let current_state = self.get_current_state().await?;
        let packages_map = current_state.packages.iter()
            .map(|(name, version)| {
                let id = PackageId::new(name.clone(), version.clone());
                let package = Package::new(
                    id,
                    HashMap::new(),  // Empty dependencies for now
                    VersionConstraint::any(),  // Default constraint
                );
                (name.clone(), package)
            })
            .collect();
        txn.state_before = packages_map;

        // Store transaction
        self.active_transactions.write().await
            .insert(txn.id, txn.clone());

        Ok(txn)
    }

    pub async fn commit_transaction(&self, id: Uuid) -> DaemonResult<()> {
        let mut transactions = self.active_transactions.write().await;
        let txn = transactions.get_mut(&id)
            .ok_or_else(|| BlastError::environment("Transaction not found"))?;

        // Verify state before commit
        let verification = self.verify_transaction_state(txn).await?;
        txn.set_verification(verification.clone());

        if !verification.is_verified {
            let error_msg = verification.issues.iter()
                .map(|i| i.description.clone())
                .collect::<Vec<_>>()
                .join(", ");
            txn.status = TransactionStatus::Failed(error_msg.clone());
            return Err(BlastError::environment(format!(
                "Transaction verification failed: {}", error_msg
            )).into());
        }

        // Update metrics before commit
        let metrics = self.collect_transaction_metrics(txn).await?;
        txn.update_metrics(metrics);

        // Perform commit
        let mut state_manager = self.state_manager.write().await;
        
        // Apply operations in order
        for op in &txn.operations {
            match op {
                TransactionOperation::Install(pkg) => {
                    let event = VersionEvent {
                        timestamp: Utc::now(),
                        from_version: None,
                        to_version: pkg.version().clone(),
                        impact: VersionImpact::None,
                        reason: format!("Installation via transaction {}", txn.id),
                        python_version: state_manager.get_current_state().python_version.clone(),
                        is_direct: true,
                        affected_dependencies: Default::default(),
                        approved: true,
                        approved_by: None,
                        policy_snapshot: None,
                    };
                    state_manager.add_package_with_event(pkg, event)?;
                }
                TransactionOperation::Uninstall(pkg) => {
                    state_manager.remove_package(pkg)?;
                }
                TransactionOperation::Update { from, to } => {
                    let event = VersionEvent {
                        timestamp: Utc::now(),
                        from_version: Some(from.version().clone()),
                        to_version: to.version().clone(),
                        impact: VersionImpact::from_version_change(from.version(), to.version()),
                        reason: format!("Update via transaction {}", txn.id),
                        python_version: state_manager.get_current_state().python_version.clone(),
                        is_direct: true,
                        affected_dependencies: Default::default(),
                        approved: true,
                        approved_by: None,
                        policy_snapshot: None,
                    };
                    state_manager.update_package_with_event(from, to, event)?;
                }
            }
        }

        // Create commit checkpoint
        let checkpoint_id = Uuid::new_v4();
        state_manager.create_checkpoint(
            checkpoint_id,
            format!("Transaction {} commit state", id),
            Some(id),
        )?;

        txn.status = TransactionStatus::Committed;
        txn.checkpoint_id = Some(checkpoint_id);

        Ok(())
    }

    async fn verify_transaction_state(&self, txn: &TransactionContext) -> DaemonResult<StateVerification> {
        let mut verification = StateVerification::default();
        
        // Get current state
        let current_state = self.get_current_state().await?;
        
        // Verify each operation
        for op in &txn.operations {
            match op {
                TransactionOperation::Install(pkg) => {
                    // Check if package is already installed
                    if current_state.packages.contains_key(pkg.id().name()) {
                        verification.add_issue(StateIssue {
                            description: format!("Package {} is already installed", pkg.id().name()),
                            severity: IssueSeverity::Critical,
                            context: None,
                            recommendation: Some(format!("Remove existing {} package first", pkg.id().name())),
                        });
                    }
                    
                    // Verify dependencies
                    for (dep_name, constraint) in pkg.dependencies() {
                        if let Some(dep_version) = current_state.packages.get(dep_name) {
                            if !constraint.matches(dep_version) {
                                verification.add_issue(StateIssue {
                                    description: format!(
                                        "Package {} dependency {} {} not satisfied (found {})",
                                        pkg.id().name(),
                                        dep_name,
                                        constraint,
                                        dep_version
                                    ),
                                    severity: IssueSeverity::Critical,
                                    context: None,
                                    recommendation: Some(format!("Update {} to a compatible version", dep_name)),
                                });
                            }
                        } else {
                            verification.add_issue(StateIssue {
                                description: format!("Required dependency {} not found", dep_name),
                                severity: IssueSeverity::Critical,
                                context: None,
                                recommendation: Some(format!("Install {} package", dep_name)),
                            });
                        }
                    }
                }
                TransactionOperation::Update { from, to } => {
                    // Verify current version matches
                    if !current_state.packages.contains_key(from.id().name()) {
                        verification.add_issue(StateIssue {
                            description: format!(
                                "Package {} version {} not found",
                                from.id().name(),
                                from.id().version()
                            ),
                            severity: IssueSeverity::Critical,
                            context: None,
                            recommendation: Some("Install the correct version before updating".to_string()),
                        });
                    }
                    
                    // Verify update compatibility
                    let impact = VersionImpact::from_version_change(from.id().version(), to.id().version());
                    if impact.is_breaking() {
                        verification.add_issue(StateIssue {
                            description: format!(
                                "Breaking changes detected in update from {} to {}",
                                from.id().version(),
                                to.id().version()
                            ),
                            severity: IssueSeverity::Warning,
                            context: None,
                            recommendation: Some("Review breaking changes and update dependent packages if needed".to_string()),
                        });
                    }
                }
                TransactionOperation::Uninstall(pkg) => {
                    // Verify package exists
                    if !current_state.packages.contains_key(pkg.id().name()) {
                        verification.add_issue(StateIssue {
                            description: format!("Package {} not found", pkg.id().name()),
                            severity: IssueSeverity::Critical,
                            context: None,
                            recommendation: Some("Verify package name and version".to_string()),
                        });
                    }
                    
                    // Check for dependent packages
                    let dependents: Vec<_> = current_state.packages.iter()
                        .filter(|(name, _)| {
                            // Check if any package has this as a dependency
                            pkg.id().name() == name.as_str()
                        })
                        .map(|(name, _)| name.clone())
                        .collect();

                    if !dependents.is_empty() {
                        verification.add_issue(StateIssue {
                            description: format!(
                                "Package {} is required by: {}",
                                pkg.id().name(),
                                dependents.join(", ")
                            ),
                            severity: IssueSeverity::Warning,
                            context: None,
                            recommendation: Some("Update or remove dependent packages first".to_string()),
                        });
                    }
                }
            }
        }

        Ok(verification)
    }

    async fn collect_transaction_metrics(&self, txn: &TransactionContext) -> DaemonResult<TransactionMetrics> {
        // Get process stats
        let memory_usage = std::process::Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .map_err(|e| BlastError::environment(format!("Failed to get memory usage: {}", e)))?
            .stdout;
        
        let memory = String::from_utf8_lossy(&memory_usage)
            .trim()
            .parse::<u64>()
            .unwrap_or(0) * 1024; // Convert KB to bytes

        let cpu_usage = std::process::Command::new("ps")
            .args(&["-o", "%cpu=", "-p", &std::process::id().to_string()])
            .output()
            .map_err(|e| BlastError::environment(format!("Failed to get CPU usage: {}", e)))?
            .stdout;
        
        let cpu = String::from_utf8_lossy(&cpu_usage)
            .trim()
            .parse::<f32>()
            .unwrap_or(0.0);

        Ok(TransactionMetrics {
            duration: Some(Utc::now().signed_duration_since(txn.created_at).to_std().unwrap_or_default()),
            memory_usage: memory,
            cpu_usage: cpu,
            network_operations: txn.operations.len() as u32,
            cache_hits: 0,
            dependencies_checked: txn.operations.iter()
                .filter(|op| matches!(op, TransactionOperation::Install(_) | TransactionOperation::Update { .. }))
                .count() as u32,
        })
    }

    pub async fn rollback_transaction(&self, id: Uuid) -> DaemonResult<()> {
        let mut transactions = self.active_transactions.write().await;
        let mut ctx = transactions.remove(&id).ok_or_else(|| {
            BlastError::environment("Transaction not found")
        })?;

        if let Some(checkpoint_id) = ctx.checkpoint_id {
            let mut state_manager = self.state_manager.write().await;
            state_manager.restore_checkpoint(checkpoint_id)?;
            ctx.status = TransactionStatus::RolledBack;
            info!("Transaction {} rolled back successfully", id);
            Ok(())
        } else {
            Err(BlastError::environment("No checkpoint found for rollback").into())
        }
    }

    pub async fn get_current_state(&self) -> DaemonResult<EnvironmentState> {
        Ok(self.state_manager.read().await.get_current_state().clone())
    }

    pub async fn list_checkpoints(&self) -> DaemonResult<Vec<Checkpoint>> {
        Ok(self.state_manager.read().await.list_checkpoints()?)
    }

    pub async fn get_checkpoint(&self, id: Uuid) -> DaemonResult<Option<Checkpoint>> {
        Ok(self.state_manager.read().await.get_checkpoint(id)?)
    }

    pub async fn get_transaction(&self, id: Uuid) -> DaemonResult<Option<TransactionContext>> {
        Ok(self.active_transactions.read().await.get(&id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blast_core::python::PythonVersion;

    #[tokio::test]
    async fn test_transaction_lifecycle() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        
        // Begin transaction
        let mut ctx = manager.begin_transaction().await.unwrap();
        
        // Add operations
        let package = Package::new(
            blast_core::package::PackageId::new(
                "test-package",
                blast_core::package::Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            blast_core::package::VersionConstraint::any(),
        );
        
        ctx.add_operation(TransactionOperation::Install(package.clone())).unwrap();
        
        // Commit transaction
        manager.commit_transaction(ctx.id).await.unwrap();
        
        // Verify state
        let state = manager.get_current_state().await.unwrap();
        assert!(state.packages.contains_key("test-package"));
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        
        // Begin transaction
        let mut ctx = manager.begin_transaction().await.unwrap();
        
        // Add operations
        let package = Package::new(
            blast_core::package::PackageId::new(
                "test-package",
                blast_core::package::Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            blast_core::package::VersionConstraint::any(),
        );
        
        ctx.add_operation(TransactionOperation::Install(package.clone())).unwrap();
        
        // Rollback transaction
        manager.rollback_transaction(ctx.id).await.unwrap();
        
        // Verify state
        let state = manager.get_current_state().await.unwrap();
        assert!(!state.packages.contains_key("test-package"));
    }

    #[tokio::test]
    async fn test_transaction_update() {
        let mut initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let v1 = blast_core::package::Version::parse("1.0.0").unwrap();
        initial_state.packages.insert("test-package".to_string(), v1.clone());

        let manager = TransactionManager::new(initial_state);
        
        // Begin transaction
        let mut ctx = manager.begin_transaction().await.unwrap();
        
        // Add update operation
        let from_pkg = Package::new(
            blast_core::package::PackageId::new(
                "test-package",
                v1,
            ),
            HashMap::new(),
            blast_core::package::VersionConstraint::any(),
        );

        let to_pkg = Package::new(
            blast_core::package::PackageId::new(
                "test-package",
                blast_core::package::Version::parse("2.0.0").unwrap(),
            ),
            HashMap::new(),
            blast_core::package::VersionConstraint::any(),
        );
        
        ctx.add_operation(TransactionOperation::Update {
            from: from_pkg,
            to: to_pkg,
        }).unwrap();
        
        // Commit transaction
        manager.commit_transaction(ctx.id).await.unwrap();
        
        // Verify state
        let state = manager.get_current_state().await.unwrap();
        assert_eq!(
            state.packages.get("test-package").unwrap().to_string(),
            "2.0.0"
        );
    }
} 