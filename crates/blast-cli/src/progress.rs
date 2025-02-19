//! Progress tracking utilities for CLI operations

use std::fmt::Display;
use crate::output::Logger;

pub struct Progress {
    logger: Logger,
}

impl Progress {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            logger: Logger::new(),
        }
    }

    /// Start a new progress task
    pub fn start(&self, message: impl Display) {
        self.logger.progress(message);
    }

    /// Mark the progress task as completed successfully
    pub fn success(&self) {
        self.logger.done();
    }

    /// Mark the progress task as failed
    pub fn fail(&self) {
        self.logger.failed();
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
} 