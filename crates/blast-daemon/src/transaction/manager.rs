use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use blast_core::state::EnvironmentState;

use crate::error::{DaemonError, DaemonResult};
use crate::transaction::TransactionContext;
use crate::transaction::types::{TransactionStatus, TransactionOperation};

/// Transaction manager
#[derive(Debug)]
pub struct TransactionManager {
    /// Active transactions
    transactions: Arc<RwLock<HashMap<uuid::Uuid, TransactionContext>>>,
    /// Current environment state
    current_state: Arc<RwLock<EnvironmentState>>,
}

impl TransactionManager {
    /// Create new transaction manager
    pub fn new(initial_state: EnvironmentState) -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            current_state: Arc::new(RwLock::new(initial_state)),
        }
    }

    /// Begin a new transaction
    pub async fn begin_transaction(&self, description: String) -> DaemonResult<TransactionContext> {
        let ctx = TransactionContext::new(description);
        
        // Store transaction
        self.transactions.write().await.insert(ctx.id, ctx.clone());
        
        Ok(ctx)
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, id: uuid::Uuid) -> DaemonResult<()> {
        let mut transactions = self.transactions.write().await;
        let ctx = transactions.get_mut(&id).ok_or_else(|| {
            DaemonError::service(format!("Transaction {} not found", id))
        })?;

        if ctx.metadata.status != TransactionStatus::Pending {
            return Err(DaemonError::service(format!(
                "Cannot commit transaction {} with status {:?}",
                id, ctx.metadata.status
            )));
        }

        // Apply operations
        let mut current_state = self.current_state.write().await;
        for operation in &ctx.operations {
            match operation {
                TransactionOperation::Install(ref package) => {
                    // Add package to state
                    current_state.add_package(package);
                }
                TransactionOperation::Uninstall(ref package) => {
                    // Remove package from state
                    current_state.remove_package(package);
                }
                TransactionOperation::Update { from: _, ref to } => {
                    // Update package in state
                    current_state.add_package(to);
                }
                TransactionOperation::AddEnvironment { name, state } => {
                    // Update environment state
                    *current_state = state.clone();
                    current_state.name = name.clone();
                }
                TransactionOperation::RemoveEnvironment { name: _ } => {
                    // Reset environment state
                    *current_state = EnvironmentState::new(
                        "default".to_string(),
                        "default".to_string(),
                        current_state.path.clone(),
                        current_state.python_version.clone(),
                    );
                }
            }
        }

        // Mark transaction as completed
        ctx.complete();
        
        Ok(())
    }

    /// Rollback a transaction
    pub async fn rollback_transaction(&self, id: uuid::Uuid) -> DaemonResult<()> {
        let mut transactions = self.transactions.write().await;
        let ctx = transactions.get_mut(&id).ok_or_else(|| {
            DaemonError::service(format!("Transaction {} not found", id))
        })?;

        if ctx.metadata.status != TransactionStatus::Pending 
            && ctx.metadata.status != TransactionStatus::InProgress {
            return Err(DaemonError::service(format!(
                "Cannot rollback transaction {} with status {:?}",
                id, ctx.metadata.status
            )));
        }

        // Reverse operations
        let mut current_state = self.current_state.write().await;
        for operation in ctx.operations.iter().rev() {
            match operation {
                TransactionOperation::Install(ref package) => {
                    // Remove installed package
                    current_state.remove_package(package);
                }
                TransactionOperation::Uninstall(ref package) => {
                    // Restore removed package
                    current_state.add_package(package);
                }
                TransactionOperation::Update { ref from, to: _ } => {
                    // Restore previous version
                    current_state.add_package(from);
                }
                TransactionOperation::AddEnvironment { name: _, state: _ } => {
                    // Reset environment state
                    *current_state = EnvironmentState::new(
                        "default".to_string(),
                        "default".to_string(),
                        current_state.path.clone(),
                        current_state.python_version.clone(),
                    );
                }
                TransactionOperation::RemoveEnvironment { name } => {
                    // Restore environment
                    current_state.name = name.clone();
                }
            }
        }

        // Mark transaction as rolled back
        ctx.rollback();
        
        Ok(())
    }

    /// Get transaction by ID
    pub async fn get_transaction(&self, id: &uuid::Uuid) -> DaemonResult<TransactionContext> {
        let transactions = self.transactions.read().await;
        transactions.get(id).cloned().ok_or_else(|| {
            DaemonError::service(format!("Transaction {} not found", id))
        })
    }

    /// List active transactions
    pub async fn list_active_transactions(&self) -> DaemonResult<HashMap<uuid::Uuid, TransactionContext>> {
        let transactions = self.transactions.read().await;
        Ok(transactions.clone())
    }

    /// Get current state
    pub async fn get_current_state(&self) -> DaemonResult<EnvironmentState> {
        Ok(self.current_state.read().await.clone())
    }

    /// Update current state
    pub async fn update_current_state(&self, state: EnvironmentState) -> DaemonResult<()> {
        *self.current_state.write().await = state;
        Ok(())
    }
} 