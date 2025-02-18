//! Environment hooks management
//! 
//! This module provides functionality for managing environment hooks that are
//! executed during environment activation and deactivation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Environment hooks configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentHooks {
    /// Commands to run before environment activation
    pub pre_activate: Vec<String>,
    /// Commands to run after environment activation
    pub post_activate: Vec<String>,
    /// Commands to run before environment deactivation
    pub pre_deactivate: Vec<String>,
    /// Commands to run after environment deactivation
    pub post_deactivate: Vec<String>,
    /// Environment variables to set during activation
    pub env_vars: HashMap<String, String>,
    /// Path modifications (prepend/append)
    pub path_modifications: PathModifications,
}

/// Path modifications for environment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathModifications {
    /// Paths to prepend to PATH
    pub prepend_path: Vec<String>,
    /// Paths to append to PATH
    pub append_path: Vec<String>,
    /// Paths to prepend to PYTHONPATH
    pub prepend_python_path: Vec<String>,
    /// Paths to append to PYTHONPATH
    pub append_python_path: Vec<String>,
}

impl EnvironmentHooks {
    /// Create new environment hooks
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pre-activation hook
    pub fn add_pre_activate(&mut self, command: String) {
        self.pre_activate.push(command);
    }

    /// Add a post-activation hook
    pub fn add_post_activate(&mut self, command: String) {
        self.post_activate.push(command);
    }

    /// Add a pre-deactivation hook
    pub fn add_pre_deactivate(&mut self, command: String) {
        self.pre_deactivate.push(command);
    }

    /// Add a post-deactivation hook
    pub fn add_post_deactivate(&mut self, command: String) {
        self.post_deactivate.push(command);
    }

    /// Set an environment variable
    pub fn set_env_var(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
    }

    /// Add a PATH prepend directory
    pub fn prepend_path(&mut self, path: String) {
        self.path_modifications.prepend_path.push(path);
    }

    /// Add a PATH append directory
    pub fn append_path(&mut self, path: String) {
        self.path_modifications.append_path.push(path);
    }

    /// Add a PYTHONPATH prepend directory
    pub fn prepend_python_path(&mut self, path: String) {
        self.path_modifications.prepend_python_path.push(path);
    }

    /// Add a PYTHONPATH append directory
    pub fn append_python_path(&mut self, path: String) {
        self.path_modifications.append_python_path.push(path);
    }

    /// Generate activation script
    pub fn generate_activation_script(&self) -> String {
        let mut script = String::new();

        // Add pre-activation hooks
        for cmd in &self.pre_activate {
            script.push_str(&format!("{}\n", cmd));
        }

        // Set environment variables
        for (key, value) in &self.env_vars {
            script.push_str(&format!("export {}={}\n", key, value));
        }

        // Modify PATH
        if !self.path_modifications.prepend_path.is_empty() {
            let paths = self.path_modifications.prepend_path.join(":");
            script.push_str(&format!("export PATH={}:$PATH\n", paths));
        }
        if !self.path_modifications.append_path.is_empty() {
            let paths = self.path_modifications.append_path.join(":");
            script.push_str(&format!("export PATH=$PATH:{}\n", paths));
        }

        // Modify PYTHONPATH
        if !self.path_modifications.prepend_python_path.is_empty() {
            let paths = self.path_modifications.prepend_python_path.join(":");
            script.push_str(&format!("export PYTHONPATH={}:$PYTHONPATH\n", paths));
        }
        if !self.path_modifications.append_python_path.is_empty() {
            let paths = self.path_modifications.append_python_path.join(":");
            script.push_str(&format!("export PYTHONPATH=$PYTHONPATH:{}\n", paths));
        }

        // Add post-activation hooks
        for cmd in &self.post_activate {
            script.push_str(&format!("{}\n", cmd));
        }

        script
    }

    /// Generate deactivation script
    pub fn generate_deactivation_script(&self) -> String {
        let mut script = String::new();

        // Add pre-deactivation hooks
        for cmd in &self.pre_deactivate {
            script.push_str(&format!("{}\n", cmd));
        }

        // Unset environment variables
        for key in self.env_vars.keys() {
            script.push_str(&format!("unset {}\n", key));
        }

        // Add post-deactivation hooks
        for cmd in &self.post_deactivate {
            script.push_str(&format!("{}\n", cmd));
        }

        script
    }
} 