use thiserror::Error;
use blast_core::error::BlastError;

/// Result type for daemon operations
pub type DaemonResult<T> = Result<T, DaemonError>;

/// Error type for daemon operations
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("State error: {0}")]
    State(String),
    #[error("Environment error: {0}")]
    Environment(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Service error: {0}")]
    Service(String),
    #[error("Monitor error: {0}")]
    Monitor(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// Helper methods for error creation
impl DaemonError {
    pub fn service(msg: impl Into<String>) -> Self {
        Self::Service(msg.into())
    }

    pub fn monitor(msg: impl Into<String>) -> Self {
        Self::Monitor(msg.into())
    }

    pub fn state(msg: impl Into<String>) -> Self {
        Self::State(msg.into())
    }

    pub fn environment(msg: impl Into<String>) -> Self {
        Self::Environment(msg.into())
    }
}

impl From<BlastError> for DaemonError {
    fn from(error: BlastError) -> Self {
        match error {
            BlastError::Io(err) => DaemonError::Io(std::io::Error::new(std::io::ErrorKind::Other, err)),
            BlastError::Python(err) => DaemonError::State(err),
            BlastError::Package(err) => DaemonError::State(err),
            BlastError::Environment(err) => DaemonError::Environment(err),
            BlastError::Resolution(err) => DaemonError::Service(format!("Resolution error: {}", err)),
            BlastError::Version(err) => DaemonError::Service(format!("Version error: {}", err)),
            BlastError::Cache(err) => DaemonError::Service(format!("Cache error: {}", err)),
            BlastError::Daemon(err) => DaemonError::Service(err),
            _ => DaemonError::Service(error.to_string()),
        }
    }
}

impl From<DaemonError> for BlastError {
    fn from(error: DaemonError) -> Self {
        match error {
            DaemonError::Service(msg) => BlastError::daemon(msg),
            DaemonError::Monitor(msg) => BlastError::daemon(format!("Monitor error: {}", msg)),
            DaemonError::State(err) => BlastError::state(err),
            DaemonError::Environment(err) => BlastError::environment(err),
            DaemonError::Io(err) => BlastError::Io(err.to_string()),
            DaemonError::Json(err) => BlastError::daemon(format!("JSON error: {}", err)),
        }
    }
}

// Explicitly implement Send + Sync
unsafe impl Send for DaemonError {}
unsafe impl Sync for DaemonError {} 