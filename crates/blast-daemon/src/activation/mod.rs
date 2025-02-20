use std::path::PathBuf;
use std::time::SystemTime;
use blast_core::python::PythonVersion;

/// Activation state for environments
#[derive(Debug, Clone)]
pub struct ActivationState {
    /// Currently active environment name
    pub(crate) active_env_name: Option<String>,
    /// Path to active environment
    pub(crate) active_env_path: Option<PathBuf>,
    /// Python version of active environment
    pub(crate) active_python_version: Option<PythonVersion>,
    /// Activation timestamp
    pub(crate) activated_at: Option<SystemTime>,
}

impl ActivationState {
    /// Create a new activation state
    pub fn new() -> Self {
        Self {
            active_env_name: None,
            active_env_path: None,
            active_python_version: None,
            activated_at: None,
        }
    }

    /// Get the active environment name
    pub fn active_env_name(&self) -> Option<&String> {
        self.active_env_name.as_ref()
    }

    /// Get the active environment path
    pub fn active_env_path(&self) -> Option<&PathBuf> {
        self.active_env_path.as_ref()
    }

    /// Get the active Python version
    pub fn active_python_version(&self) -> Option<&PythonVersion> {
        self.active_python_version.as_ref()
    }

    /// Get the activation timestamp
    pub fn activated_at(&self) -> Option<SystemTime> {
        self.activated_at
    }

    /// Set the active environment
    pub fn set_active_environment(
        &mut self,
        name: String,
        path: PathBuf,
        python_version: PythonVersion,
    ) {
        self.active_env_name = Some(name);
        self.active_env_path = Some(path);
        self.active_python_version = Some(python_version);
        self.activated_at = Some(SystemTime::now());
    }

    /// Clear the active environment
    pub fn clear_active_environment(&mut self) {
        self.active_env_name = None;
        self.active_env_path = None;
        self.active_python_version = None;
        self.activated_at = None;
    }

    /// Check if there is an active environment
    pub fn has_active_environment(&self) -> bool {
        self.active_env_name.is_some()
    }
} 