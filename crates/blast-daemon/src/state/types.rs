use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use blast_core::python::PythonVersion;

/// Daemon state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Active environment name
    pub active_env_name: Option<String>,
    /// Active environment path
    pub active_env_path: Option<PathBuf>,
    /// Active Python version
    pub active_python_version: Option<PythonVersion>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Package cache
    pub package_cache: HashMap<String, PackageState>,
    /// Last update timestamp
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_env_name: None,
            active_env_path: None,
            active_python_version: None,
            env_vars: HashMap::new(),
            package_cache: HashMap::new(),
            last_update: None,
        }
    }
}

/// Package state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageState {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Installation timestamp
    pub installed_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Package status
    pub status: PackageStatus,
}

/// Package status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageStatus {
    /// Package is installed
    Installed,
    /// Package is being installed
    Installing,
    /// Package is being updated
    Updating,
    /// Package is being removed
    Removing,
    /// Package has errors
    Error(String),
} 