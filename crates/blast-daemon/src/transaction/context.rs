use std::collections::HashMap;
use crate::error::DaemonResult;
use super::types::{TransactionOperation, TransactionMetadata, TransactionStatus};

/// Transaction context
#[derive(Debug, Clone)]
pub struct TransactionContext {
    /// Transaction ID
    pub id: uuid::Uuid,
    /// Transaction operations
    pub operations: Vec<TransactionOperation>,
    /// Transaction metadata
    pub metadata: TransactionMetadata,
    /// Transaction state
    pub state: HashMap<String, serde_json::Value>,
}

impl TransactionContext {
    /// Create new transaction context
    pub fn new(description: String) -> Self {
        let id = uuid::Uuid::new_v4();
        Self {
            id,
            operations: Vec::new(),
            metadata: TransactionMetadata::new(id, description),
            state: HashMap::new(),
        }
    }

    /// Add operation to transaction
    pub fn add_operation(&mut self, operation: TransactionOperation) -> DaemonResult<()> {
        if self.metadata.status != TransactionStatus::Pending {
            return Err(crate::error::DaemonError::service(
                "Cannot add operation to non-pending transaction".to_string(),
            ));
        }
        self.operations.push(operation);
        Ok(())
    }

    /// Set transaction state
    pub fn set_state(&mut self, key: &str, value: serde_json::Value) {
        self.state.insert(key.to_string(), value);
    }

    /// Get transaction state
    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Mark transaction as completed
    pub fn complete(&mut self) {
        self.metadata.complete();
    }

    /// Mark transaction as failed
    pub fn fail(&mut self, error: String) {
        self.metadata.fail(error);
    }

    /// Mark transaction as rolled back
    pub fn rollback(&mut self) {
        self.metadata.rollback();
    }
} 