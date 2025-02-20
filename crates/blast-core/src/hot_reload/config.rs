use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Hot reload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    /// Paths to watch for changes
    pub watch_paths: Vec<PathBuf>,
    /// Whether to watch for Python file changes
    pub watch_python: bool,
    /// Whether to watch for environment variable changes
    pub watch_env: bool,
    /// Whether to automatically install new dependencies
    pub auto_install: bool,
    /// Maximum number of updates to keep in history
    pub max_history: usize,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec![PathBuf::from(".")],
            watch_python: true,
            watch_env: true,
            auto_install: false,
            max_history: 100,
        }
    }
}

impl HotReloadConfig {
    /// Create new configuration with custom watch paths
    pub fn new(watch_paths: Vec<PathBuf>) -> Self {
        Self {
            watch_paths,
            ..Default::default()
        }
    }

    /// Enable or disable Python file watching
    pub fn with_python_watch(mut self, enabled: bool) -> Self {
        self.watch_python = enabled;
        self
    }

    /// Enable or disable environment variable watching
    pub fn with_env_watch(mut self, enabled: bool) -> Self {
        self.watch_env = enabled;
        self
    }

    /// Enable or disable automatic dependency installation
    pub fn with_auto_install(mut self, enabled: bool) -> Self {
        self.auto_install = enabled;
        self
    }

    /// Set maximum history size
    pub fn with_max_history(mut self, size: usize) -> Self {
        self.max_history = size;
        self
    }
} 