#![allow(dead_code)]

mod config;
mod manager;
mod update;
mod import;

pub use config::*;
pub use manager::*;
pub use update::*;
pub use import::*;

use crate::error::BlastError;

/// Custom error type for hot reload operations
#[derive(Debug, thiserror::Error)]
pub enum HotReloadError {
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Python parsing error: {0}")]
    PythonParse(String),
}

impl From<HotReloadError> for BlastError {
    fn from(err: HotReloadError) -> Self {
        BlastError::environment(err.to_string())
    }
} 