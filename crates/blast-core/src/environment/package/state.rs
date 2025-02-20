use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::BlastResult;
use super::{Version, DependencyGraph};

/// Package metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package version
    pub version: Version,
    /// Installation time
    pub installed_at: chrono::DateTime<chrono::Utc>,
    /// Last update time
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Direct dependency
    pub direct: bool,
    /// Package hash
    pub hash: Option<String>,
    /// Package size
    pub size: u64,
    /// Installation source
    pub source: String,
}

/// Package state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageState {
    /// Installed packages
    pub installed: HashMap<String, PackageMetadata>,
    /// Last update check
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
    /// State version
    pub version: u32,
}

impl PackageState {
    /// Create new package state
    pub fn new() -> Self {
        Self {
            installed: HashMap::new(),
            last_check: None,
            version: 1,
        }
    }

    /// Update state from dependency graph
    pub async fn update_from_graph(&mut self, graph: &DependencyGraph) -> BlastResult<()> {
        let now = chrono::Utc::now();
        
        // Update installed packages
        for node in graph.nodes() {
            let metadata = PackageMetadata {
                version: Version {
                    version: node.version.clone(),
                    released: now,
                    python_requires: None,
                    dependencies: node.dependencies.clone(),
                },
                installed_at: now,
                updated_at: now,
                direct: node.direct,
                hash: node.hash.clone(),
                size: node.size,
                source: node.source.clone(),
            };
            
            self.installed.insert(node.name.clone(), metadata);
        }
        
        // Update last check time
        self.last_check = Some(now);
        
        Ok(())
    }

    /// Remove package from state
    pub async fn remove_package(&mut self, name: &str) -> BlastResult<()> {
        self.installed.remove(name);
        Ok(())
    }

    /// Get installed version
    pub fn get_installed_version(&self, name: &str) -> Option<&Version> {
        self.installed.get(name).map(|m| &m.version)
    }

    /// Check if package is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed.contains_key(name)
    }

    /// Get direct dependencies
    pub fn get_direct_dependencies(&self) -> Vec<String> {
        self.installed
            .iter()
            .filter(|(_, m)| m.direct)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get all dependencies for package
    pub fn get_dependencies(&self, name: &str) -> Vec<String> {
        if let Some(metadata) = self.installed.get(name) {
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
        self.installed
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
            self.installed
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

    /// Save state to file
    pub async fn save(&self, path: &std::path::Path) -> BlastResult<()> {
        // Serialize to string first
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::BlastError::package(format!(
                "Failed to serialize package state: {}", e
            )))?;
        
        // Write to file
        tokio::fs::write(path, json).await
            .map_err(|e| crate::error::BlastError::package(format!(
                "Failed to save package state: {}", e
            )))?;
        
        Ok(())
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
} 