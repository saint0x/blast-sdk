use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use blast_core::{
    package::Package,
    state::EnvironmentState,
};

/// Transaction operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionOperation {
    /// Install package
    Install(Package),
    /// Uninstall package
    Uninstall(Package),
    /// Update package
    Update {
        from: Package,
        to: Package,
    },
    /// Add environment
    AddEnvironment {
        name: String,
        state: EnvironmentState,
    },
    /// Remove environment
    RemoveEnvironment {
        name: String,
    },
}

/// Transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending
    Pending,
    /// Transaction is in progress
    InProgress,
    /// Transaction completed successfully
    Completed,
    /// Transaction failed
    Failed(String),
    /// Transaction was rolled back
    RolledBack,
}

/// Transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetadata {
    /// Transaction ID
    pub id: uuid::Uuid,
    /// Transaction description
    pub description: String,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Completion timestamp
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Transaction status
    pub status: TransactionStatus,
    /// Error message if failed
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl TransactionMetadata {
    /// Create new transaction metadata
    pub fn new(id: uuid::Uuid, description: String) -> Self {
        Self {
            id,
            description,
            created_at: chrono::Utc::now(),
            completed_at: None,
            status: TransactionStatus::Pending,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Mark transaction as completed
    pub fn complete(&mut self) {
        self.status = TransactionStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark transaction as failed
    pub fn fail(&mut self, error: String) {
        self.status = TransactionStatus::Failed(error.clone());
        self.error = Some(error);
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark transaction as rolled back
    pub fn rollback(&mut self) {
        self.status = TransactionStatus::RolledBack;
        self.completed_at = Some(chrono::Utc::now());
    }
} 