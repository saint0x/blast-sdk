use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::BlastResult;
use crate::environment::PackageOperation;
use super::{Version, DependencyGraph, Dependency};
use std::path::Path;
use tokio::fs;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid;

/// Package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package version information
    pub version: Version,
    /// Installation time
    pub installed_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
    /// Whether it's a direct dependency
    pub direct: bool,
    /// Package hash
    pub hash: Option<String>,
    /// Package size
    pub size: u64,
    /// Installation source
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransaction {
    id: String,
    timestamp: DateTime<Utc>,
    changes: Vec<StateChange>,
    status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Committed,
    RolledBack,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChange {
    PackageAdded {
        name: String,
        version: String,
        dependencies: Vec<String>,
    },
    PackageRemoved {
        name: String,
    },
    PackageUpdated {
        name: String,
        old_version: String,
        new_version: String,
        dependencies: Vec<String>,
    },
}

/// Package state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageState {
    /// Installed packages
    packages: HashMap<String, PackageInfo>,
    /// Transaction history
    transactions: Vec<StateTransaction>,
    /// Current transaction
    current_transaction: Option<StateTransaction>,
    /// State version
    version: u32,
    /// Last modified timestamp
    last_modified: DateTime<Utc>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct StateManager {
    /// Current state
    state: Arc<RwLock<PackageState>>,
    /// State file path
    state_path: Box<Path>,
    /// Backup directory
    backup_dir: Box<Path>,
    /// Maximum transaction history
    max_history: usize,
}

#[allow(dead_code)]
impl StateManager {
    /// Create new state manager
    pub fn new<P: AsRef<Path>>(state_path: P, backup_dir: P) -> Self {
        Self {
            state: Arc::new(RwLock::new(PackageState::new())),
            state_path: state_path.as_ref().into(),
            backup_dir: backup_dir.as_ref().into(),
            max_history: 100, // Configurable
        }
    }

    /// Begin new transaction
    pub async fn begin_transaction(&self) -> BlastResult<String> {
        let mut state = self.state.write().await;
        
        // Check for existing transaction
        if state.current_transaction.is_some() {
            return Err(crate::error::BlastError::state("Transaction already in progress"));
        }
        
        // Create new transaction
        let transaction = StateTransaction {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            changes: Vec::new(),
            status: TransactionStatus::Pending,
        };
        
        state.current_transaction = Some(transaction.clone());
        Ok(transaction.id)
    }

    /// Commit current transaction
    pub async fn commit_transaction(&self) -> BlastResult<()> {
        let mut state = self.state.write().await;
        
        // Get current transaction
        let transaction = state.current_transaction.take()
            .ok_or_else(|| crate::error::BlastError::state("No transaction in progress"))?;
        
        // Apply changes
        for change in &transaction.changes {
            match change {
                StateChange::PackageAdded { name, version, dependencies } => {
                    let version_obj = Version {
                        version: version.clone(),
                        released: Utc::now(),
                        python_requires: None,
                        dependencies: dependencies.iter().map(|d| Dependency {
                            name: d.clone(),
                            version_constraint: String::new(),
                            optional: false,
                            markers: None,
                        }).collect(),
                    };
                    
                    state.packages.insert(name.clone(), PackageInfo {
                        version: version_obj,
                        installed_at: Utc::now(),
                        updated_at: Utc::now(),
                        direct: false,
                        hash: None,
                        size: 0,
                        source: String::new(),
                    });
                }
                StateChange::PackageRemoved { name } => {
                    state.packages.remove(name);
                }
                StateChange::PackageUpdated { name, new_version, dependencies, .. } => {
                    if let Some(pkg) = state.packages.get_mut(name) {
                        pkg.version = Version {
                            version: new_version.clone(),
                            released: Utc::now(),
                            python_requires: None,
                            dependencies: dependencies.iter().map(|d| Dependency {
                                name: d.clone(),
                                version_constraint: String::new(),
                                optional: false,
                                markers: None,
                            }).collect(),
                        };
                        pkg.updated_at = Utc::now();
                    }
                }
            }
        }
        
        // Update transaction status and add to history
        let mut committed_transaction = transaction;
        committed_transaction.status = TransactionStatus::Committed;
        
        // Handle transaction history
        {
            let transactions = &mut state.transactions;
            let len = transactions.len();
            if len > self.max_history {
                transactions.drain(..len - self.max_history);
            }
            transactions.push(committed_transaction);
        }
        
        // Update state metadata
        state.version += 1;
        state.last_modified = Utc::now();
        
        // Save state
        self.save_state(&state).await?;
        
        Ok(())
    }

    /// Rollback current transaction
    pub async fn rollback_transaction(&self) -> BlastResult<()> {
        let mut state = self.state.write().await;
        
        // Get current transaction
        let mut transaction = state.current_transaction.take()
            .ok_or_else(|| crate::error::BlastError::state("No transaction in progress"))?;
        
        // Update transaction status and add to history
        transaction.status = TransactionStatus::RolledBack;
        state.transactions.push(transaction);
        
        Ok(())
    }

    /// Add state change to current transaction
    pub async fn add_state_change(&self, change: StateChange) -> BlastResult<()> {
        let mut state = self.state.write().await;
        
        if let Some(transaction) = state.current_transaction.as_mut() {
            transaction.changes.push(change);
            Ok(())
        } else {
            Err(crate::error::BlastError::state("No transaction in progress"))
        }
    }

    /// Save state to file
    async fn save_state(&self, state: &PackageState) -> BlastResult<()> {
        // Create backup
        let backup_path = self.backup_dir.join(format!(
            "state_backup_{}.json",
            state.version
        ));
        
        let state_json = serde_json::to_string_pretty(state)?;
        
        // Write to backup first
        fs::write(&backup_path, &state_json).await?;
        
        // Then update main state file
        fs::write(&self.state_path, state_json).await?;
        
        Ok(())
    }

    /// Load state from file
    pub async fn load_state(&self) -> BlastResult<()> {
        let state_json = fs::read_to_string(&self.state_path).await?;
        let loaded_state: PackageState = serde_json::from_str(&state_json)?;
        
        let mut state = self.state.write().await;
        *state = loaded_state;
        
        Ok(())
    }

    /// Get transaction history
    pub async fn get_transaction_history(&self) -> BlastResult<Vec<StateTransaction>> {
        let state = self.state.read().await;
        Ok(state.transactions.clone())
    }

    /// Get current state
    pub async fn get_current_state(&self) -> BlastResult<PackageState> {
        let state = self.state.read().await;
        Ok(state.clone())
    }
}

impl Default for PackageState {
    fn default() -> Self {
        Self {
            packages: HashMap::new(),
            transactions: Vec::new(),
            current_transaction: None,
            version: 0,
            last_modified: Utc::now(),
        }
    }
}

impl PackageState {
    /// Create a new package state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update state from dependency graph
    pub async fn update_from_graph(&mut self, graph: &DependencyGraph) -> BlastResult<()> {
        for node in graph.nodes() {
            let version_obj = Version {
                    version: node.version.clone(),
                released: Utc::now(),
                    python_requires: None,
                    dependencies: node.dependencies.clone(),
            };
            
            self.packages.insert(node.name.clone(), PackageInfo {
                version: version_obj,
                installed_at: Utc::now(),
                updated_at: Utc::now(),
                direct: node.direct,
                hash: node.hash.clone(),
                size: node.size,
                source: node.source.clone(),
            });
        }
        
        self.version += 1;
        self.last_modified = Utc::now();
        
        Ok(())
    }

    /// Remove package from state
    pub async fn remove_package(&mut self, name: &str) -> BlastResult<()> {
        self.packages.remove(name);
        self.version += 1;
        self.last_modified = Utc::now();
        
        Ok(())
    }

    /// Get installed version
    pub fn get_installed_version(&self, name: &str) -> Option<&Version> {
        self.packages.get(name).map(|m| &m.version)
    }

    /// Check if package is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }

    /// Get direct dependencies
    pub fn get_direct_dependencies(&self) -> Vec<&PackageInfo> {
        self.packages.values()
            .filter(|p| p.direct)
            .collect()
    }

    /// Get all dependencies for package
    pub fn get_dependencies(&self, name: &str) -> Vec<String> {
        if let Some(metadata) = self.packages.get(name) {
            metadata
                .version
                .dependencies
                .iter()
                .map(|d| d.name.clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get reverse dependencies
    pub fn get_reverse_dependencies(&self, name: &str) -> Vec<String> {
        self.packages
            .iter()
            .filter(|(_, m)| {
                m.version.dependencies.iter().any(|d| d.name == name)
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Check if package can be safely removed
    pub fn can_remove(&self, name: &str) -> bool {
        // Get reverse dependencies
        let rdeps = self.get_reverse_dependencies(name);
        
        // Package can be removed if it has no reverse dependencies
        // or if all reverse dependencies are optional
        rdeps.is_empty() || rdeps.iter().all(|dep| {
            self.packages
                .get(dep)
                .map(|m| {
                    m.version
                        .dependencies
                        .iter()
                        .find(|d| d.name == name)
                        .map(|d| d.optional)
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        })
    }

    /// Get last modification time
    pub fn last_modified(&self) -> DateTime<Utc> {
        self.last_modified
    }

    /// Save state to file
    pub async fn save(&self, path: &Path) -> BlastResult<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::BlastError::package(format!(
                "Failed to serialize package state: {}", e
            )))?;
        
        tokio::fs::write(path, json).await
            .map_err(|e| crate::error::BlastError::package(format!(
                "Failed to save package state: {}", e
            )))
    }

    /// Load state from file
    pub async fn load(path: &std::path::Path) -> BlastResult<Self> {
        // Read file contents
        let json = tokio::fs::read_to_string(path).await
            .map_err(|e| crate::error::BlastError::package(format!(
                "Failed to read package state: {}", e
            )))?;
        
        // Deserialize from string
        let state = serde_json::from_str(&json)
            .map_err(|e| crate::error::BlastError::package(format!(
                "Failed to deserialize package state: {}", e
            )))?;
        
        Ok(state)
    }

    /// Update package metadata
    pub async fn update_package_metadata(&mut self, metadata: &PackageInfo) -> BlastResult<()> {
        self.packages.insert(metadata.version.version.clone(), metadata.clone());
        self.last_modified = Utc::now();
        Ok(())
    }

    /// Get package info
    pub fn get_package(&self, name: &str) -> Option<&PackageInfo> {
        self.packages.get(name)
    }

    /// Get all packages
    pub fn get_all_packages(&self) -> Vec<&PackageInfo> {
        self.packages.values().collect()
    }

    /// Update state from package operation
    pub async fn update_from_operation(&mut self, operation: &PackageOperation) -> BlastResult<()> {
        match operation {
            PackageOperation::Install { name, version, dependencies: _ } => {
                if let Some(version_info) = version {
                    let pkg_info = PackageInfo {
                        version: version_info.clone(),
                        installed_at: Utc::now(),
                        updated_at: Utc::now(),
                        direct: true,
                        hash: None,
                        size: 0,
                        source: String::new(),
                    };
                    self.packages.insert(name.clone(), pkg_info);
                }
            }
            PackageOperation::Uninstall { name } => {
                self.packages.remove(name);
            }
            PackageOperation::Update { name, from_version: _, to_version } => {
                if let Some(pkg) = self.packages.get_mut(name) {
                    pkg.version = to_version.clone();
                    pkg.updated_at = Utc::now();
                }
            }
        }
        
        self.version += 1;
        self.last_modified = Utc::now();
        
        Ok(())
    }
} 