use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, warn};
use std::sync::Arc;

use blast_core::{
    package::Package,
    error::BlastError,
    state::{EnvironmentState, StateVerification, StateIssue},
    version_history::{VersionEvent, VersionImpact, VersionChangeAnalysis},
    sync::IssueSeverity,
};

use crate::{
    state::{StateManager, Checkpoint},
    error::DaemonError,
    DaemonResult,
};

#[derive(Debug, Clone)]
pub enum TransactionOperation {
    Install(Package),
    Uninstall(Package),
    Update { from: Package, to: Package },
    AddEnvironment {
        name: String,
        state: EnvironmentState,
    },
    RemoveEnvironment {
        name: String,
    },
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
    pub description: String,
    pub state_manager: Arc<RwLock<StateManager>>,
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
    pub fn new(description: String, state_manager: Arc<RwLock<StateManager>>) -> Self {
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
            description,
            state_manager,
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
            state_manager: RwLock::new(StateManager::new(PathBuf::from("environments/default"))),
        }
    }
}

impl TransactionManager {
    pub fn new(initial_state: EnvironmentState) -> Self {
        let state_manager = StateManager::new(PathBuf::from("environments/default"));
        let state_manager = RwLock::new(state_manager);
        
        Self {
            active_transactions: RwLock::new(HashMap::new()),
            state_manager,
        }
    }

    pub async fn list_active_transactions(&self) -> DaemonResult<HashMap<Uuid, TransactionContext>> {
        Ok(self.active_transactions.read().await.clone())
    }

    pub async fn begin_transaction(&self, description: String) -> DaemonResult<TransactionContext> {
        let transaction_id = Uuid::new_v4();
        let state_manager = Arc::new(RwLock::new(StateManager::new(PathBuf::from("environments/default"))));
        
        // Create initial checkpoint
        {
            let state_manager_guard = state_manager.write().await;
            state_manager_guard.create_checkpoint(
                Uuid::new_v4(),
                format!("Transaction start: {}", description),
                Some(transaction_id),
            ).await?;
        }

        Ok(TransactionContext {
            id: transaction_id,
            description,
            state_manager,
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
        })
    }

    pub async fn execute_transaction<F, T>(&self, description: String, f: F) -> DaemonResult<T>
    where
        F: FnOnce(TransactionContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = DaemonResult<T>> + Send>>,
    {
        let ctx = self.begin_transaction(description).await?;
        let transaction_id = ctx.id;

        match f(ctx).await {
            Ok(result) => {
                // Create success checkpoint
                let state_manager = self.state_manager.write().await;
                state_manager.create_checkpoint(
                    Uuid::new_v4(),
                    "Transaction completed successfully".to_string(),
                    Some(transaction_id),
                ).await?;
                Ok(result)
            }
            Err(e) => {
                // Restore to initial checkpoint
                let state_manager = self.state_manager.write().await;
                let checkpoints = state_manager.list_checkpoints().await?;
                if let Some(initial) = checkpoints.iter().find(|c| c.transaction_id == Some(transaction_id)) {
                    state_manager.restore_checkpoint(&initial.id.to_string()).await?;
                }
                Err(e)
            }
        }
    }

    pub async fn commit_transaction(&self, id: Uuid) -> DaemonResult<()> {
        let mut transactions = self.active_transactions.write().await;
        let txn = transactions.get_mut(&id)
            .ok_or_else(|| DaemonError::transaction("Transaction not found"))?;

        // Verify state before commit
        let verification = self.verify_transaction_state(txn).await?;
        txn.set_verification(verification.clone());

        if !verification.is_verified {
            let error_msg = verification.issues.iter()
                .map(|i| i.description.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(DaemonError::transaction(format!(
                "Transaction verification failed: {}", error_msg
            )));
        }

        // Update metrics before commit
        let metrics = self.collect_transaction_metrics(txn).await?;
        txn.update_metrics(metrics);

        // Perform commit
        let state_manager = self.state_manager.write().await;
        
        // Apply operations in order
        for op in &txn.operations {
            match op {
                TransactionOperation::Install(pkg) => {
                    let current_state = state_manager.get_current_state().await.map_err(|e| DaemonError::from(e))?;
                    let event = VersionEvent {
                        timestamp: Utc::now(),
                        from_version: None,
                        to_version: pkg.version().clone(),
                        impact: VersionImpact::None,
                        reason: format!("Installation via transaction {}", txn.id),
                        python_version: current_state.python_version.clone(),
                        is_direct: true,
                        affected_dependencies: Default::default(),
                        approved: true,
                        approved_by: None,
                        policy_snapshot: None,
                    };
                    state_manager.add_package_with_event(&pkg, event).await.map_err(|e| DaemonError::from(e))?;
                }
                TransactionOperation::Uninstall(pkg) => {
                    state_manager.remove_package(&pkg).await.map_err(|e| DaemonError::from(e))?;
                }
                TransactionOperation::Update { from, to } => {
                    let current_state = state_manager.get_current_state().await.map_err(|e| DaemonError::from(e))?;
                    let event = VersionEvent {
                        timestamp: Utc::now(),
                        from_version: Some(from.version().clone()),
                        to_version: to.version().clone(),
                        impact: VersionImpact::from_version_change(from.version(), to.version()),
                        reason: format!("Update via transaction {}", txn.id),
                        python_version: current_state.python_version.clone(),
                        is_direct: true,
                        affected_dependencies: Default::default(),
                        approved: true,
                        approved_by: None,
                        policy_snapshot: None,
                    };
                    state_manager.update_package_with_event(&from, &to, event).await.map_err(|e| DaemonError::from(e))?;
                }
                TransactionOperation::AddEnvironment { name, state } => {
                    state_manager.add_environment(name.clone(), state.clone()).await.map_err(|e| DaemonError::from(e))?;
                }
                TransactionOperation::RemoveEnvironment { name } => {
                    state_manager.remove_environment(name).await.map_err(|e| DaemonError::from(e))?;
                }
            }
        }

        // Create commit checkpoint
        let checkpoint_id = Uuid::new_v4();
        state_manager.create_checkpoint(
            checkpoint_id,
            format!("Transaction {} commit state", id),
            Some(id),
        ).await.map_err(|e| DaemonError::from(e))?;

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
                }
                TransactionOperation::Update { from, to: _ } => {
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
                }
                TransactionOperation::AddEnvironment { name, state: _ } => {
                    // Verify environment doesn't exist
                    if current_state.packages.contains_key(name) {
                        verification.add_issue(StateIssue {
                            description: format!("Environment {} already exists", name),
                            severity: IssueSeverity::Critical,
                            context: None,
                            recommendation: Some(format!("Choose a different name or remove existing environment {}", name)),
                        });
                    }
                }
                TransactionOperation::RemoveEnvironment { name } => {
                    // Verify environment exists
                    if !current_state.packages.contains_key(name) {
                        verification.add_issue(StateIssue {
                            description: format!("Environment {} not found", name),
                            severity: IssueSeverity::Critical,
                            context: None,
                            recommendation: Some(format!("Verify environment name {}", name)),
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
            .map_err(|e| DaemonError::transaction(format!("Failed to get memory usage: {}", e)))?
            .stdout;
        
        let memory = String::from_utf8_lossy(&memory_usage)
            .trim()
            .parse::<u64>()
            .unwrap_or(0) * 1024; // Convert KB to bytes

        let cpu_usage = std::process::Command::new("ps")
            .args(&["-o", "%cpu=", "-p", &std::process::id().to_string()])
            .output()
            .map_err(|e| DaemonError::transaction(format!("Failed to get CPU usage: {}", e)))?
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
            DaemonError::transaction("Transaction not found")
        })?;

        if let Some(checkpoint_id) = ctx.checkpoint_id {
            let state_manager = self.state_manager.write().await;
            state_manager.restore_checkpoint(&checkpoint_id.to_string()).await.map_err(|e| DaemonError::from(e))?;
            ctx.status = TransactionStatus::RolledBack;
            info!("Transaction {} rolled back successfully", id);
            Ok(())
        } else {
            Err(DaemonError::transaction("No checkpoint found for rollback"))
        }
    }

    pub async fn get_current_state(&self) -> DaemonResult<EnvironmentState> {
        let state_manager = self.state_manager.read().await;
        state_manager.get_current_state().await.map_err(DaemonError::from)
    }

    pub async fn list_checkpoints(&self) -> DaemonResult<Vec<Checkpoint>> {
        let state_manager = self.state_manager.read().await;
        state_manager.list_checkpoints().await.map_err(DaemonError::from)
    }

    pub async fn get_checkpoint(&self, id: &str) -> DaemonResult<Option<Checkpoint>> {
        let state_manager = self.state_manager.read().await;
        state_manager.get_checkpoint(id).await.map_err(DaemonError::from)
    }

    pub async fn get_transaction(&self, id: Uuid) -> DaemonResult<Option<TransactionContext>> {
        Ok(self.active_transactions.read().await.get(&id).cloned())
    }
} 