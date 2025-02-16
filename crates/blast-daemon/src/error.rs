use blast_core::error::BlastError;

/// Daemon error types
#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    /// Service error
    #[error("Service error: {0}")]
    Service(String),

    /// IPC error
    #[error("IPC error: {0}")]
    Ipc(String),

    /// Monitor error
    #[error("Monitor error: {0}")]
    Monitor(String),

    /// Core error
    #[error("Core error: {0}")]
    Core(String),

    /// Resolver error
    #[error("Resolver error: {0}")]
    Resolver(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Version error
    #[error("Version error: {0}")]
    Version(String),

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Implement conversion from BlastError
impl From<BlastError> for DaemonError {
    fn from(error: BlastError) -> Self {
        match error {
            BlastError::Io(e) => DaemonError::Io(e),
            BlastError::Python(msg) => DaemonError::Core(format!("Python error: {}", msg)),
            BlastError::Package(msg) => DaemonError::Core(format!("Package error: {}", msg)),
            BlastError::Environment(msg) => DaemonError::Core(format!("Environment error: {}", msg)),
            BlastError::Resolution(msg) => DaemonError::Resolver(msg),
            BlastError::Version(msg) => DaemonError::Version(msg),
            _ => DaemonError::Core(error.to_string()),
        }
    }
}

// Implement conversion to BlastError
impl From<DaemonError> for BlastError {
    fn from(error: DaemonError) -> Self {
        match error {
            DaemonError::Service(msg) => BlastError::Daemon(format!("Service error: {}", msg)),
            DaemonError::Ipc(msg) => BlastError::Daemon(format!("IPC error: {}", msg)),
            DaemonError::Monitor(msg) => BlastError::Daemon(format!("Monitor error: {}", msg)),
            DaemonError::Core(msg) => BlastError::Daemon(format!("Core error: {}", msg)),
            DaemonError::Resolver(msg) => BlastError::Resolution(msg),
            DaemonError::Transaction(msg) => BlastError::Daemon(format!("Transaction error: {}", msg)),
            DaemonError::Validation(msg) => BlastError::Daemon(format!("Validation error: {}", msg)),
            DaemonError::Version(msg) => BlastError::Version(msg),
            DaemonError::ResourceLimit(msg) => BlastError::Daemon(format!("Resource limit error: {}", msg)),
            DaemonError::Io(e) => BlastError::Io(e),
        }
    }
}

// Explicitly implement Send + Sync
unsafe impl Send for DaemonError {}
unsafe impl Sync for DaemonError {}

// Helper methods for error creation
impl DaemonError {
    pub fn service(msg: impl Into<String>) -> Self {
        Self::Service(msg.into())
    }

    pub fn ipc(msg: impl Into<String>) -> Self {
        Self::Ipc(msg.into())
    }

    pub fn monitor(msg: impl Into<String>) -> Self {
        Self::Monitor(msg.into())
    }

    pub fn core(msg: impl Into<String>) -> Self {
        Self::Core(msg.into())
    }

    pub fn resolver(msg: impl Into<String>) -> Self {
        Self::Resolver(msg.into())
    }

    pub fn transaction(msg: impl Into<String>) -> Self {
        Self::Transaction(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn version(msg: impl Into<String>) -> Self {
        Self::Version(msg.into())
    }

    pub fn resource_limit(msg: impl Into<String>) -> Self {
        Self::ResourceLimit(msg.into())
    }
}

/// Result type for daemon operations
pub type DaemonResult<T> = Result<T, DaemonError>; 