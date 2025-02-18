//! Progress tracking utilities for CLI operations

use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};
use console::style;
use blast_core::package::Package;

/// Manages progress bars for concurrent operations
pub struct ProgressManager {
    resolution_spinner: Option<ProgressBar>,
    installation_progress: Option<ProgressBar>,
}

impl ProgressManager {
    /// Create a new progress manager
    pub fn new() -> Self {
        Self {
            resolution_spinner: None,
            installation_progress: None,
        }
    }

    /// Start the resolution process
    pub fn start_resolution(&mut self) {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg}")
                .unwrap(),
        );
        spinner.set_message("Resolving dependencies...");
        spinner.enable_steady_tick(Duration::from_millis(100));
        self.resolution_spinner = Some(spinner);
    }

    /// Finish the resolution process
    pub fn finish_resolution(&mut self) {
        if let Some(spinner) = self.resolution_spinner.take() {
            spinner.finish_with_message("Dependencies resolved");
        }
    }

    /// Start the installation process
    pub fn start_installation(&mut self, total: usize) {
        let progress = ProgressBar::new(total as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        progress.set_message("Installing packages...");
        self.installation_progress = Some(progress);
    }

    /// Set the progress for a specific package
    pub fn set_package(&mut self, package: &Package) {
        if let Some(progress) = &self.installation_progress {
            progress.set_message(format!("Installing {}", style(package.id()).cyan()));
        }
    }

    /// Increment the installation progress
    pub fn increment(&mut self) {
        if let Some(progress) = &self.installation_progress {
            progress.inc(1);
        }
    }

    /// Finish the installation process
    pub fn finish_installation(&mut self) {
        if let Some(progress) = self.installation_progress.take() {
            progress.finish_with_message("Installation complete");
        }
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
} 