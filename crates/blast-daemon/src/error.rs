use blast_core::error::BlastError;

/// Daemon error types
#[derive(Debug, thiserror::Error, Clone)]
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

    /// State error
    #[error("State error: {0}")]
    State(String),

    /// Snapshot error
    #[error("Snapshot error: {0}")]
    Snapshot(String),

    /// Access error
    #[error("Access error: {0}")]
    Access(String),

    /// Environment error
    #[error("Environment error: {0}")]
    Environment(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(String),
}

// Implement conversion from BlastError
impl From<BlastError> for DaemonError {
    fn from(error: BlastError) -> Self {
        match error {
            BlastError::Io(msg) => DaemonError::Io(msg),
            BlastError::Python(msg) => DaemonError::Core(format!("Python error: {}", msg)),
            BlastError::Package(msg) => DaemonError::Core(format!("Package error: {}", msg)),
            BlastError::Environment(msg) => DaemonError::Environment(msg),
            BlastError::Resolution(msg) => DaemonError::Resolver(msg),
            BlastError::Version(msg) => DaemonError::Version(msg),
            BlastError::Cache(msg) => DaemonError::Core(format!("Cache error: {}", msg)),
            BlastError::Daemon(msg) => DaemonError::Core(msg),
            _ => DaemonError::Core(error.to_string()),
        }
    }
}

// Implement conversion from std::io::Error
impl From<std::io::Error> for DaemonError {
    fn from(error: std::io::Error) -> Self {
        DaemonError::Io(error.to_string())
    }
}

// Implement conversion to BlastError
impl From<DaemonError> for BlastError {
    fn from(error: DaemonError) -> Self {
        match error {
            DaemonError::Service(msg) => BlastError::daemon(msg),
            DaemonError::Ipc(msg) => BlastError::daemon(format!("IPC error: {}", msg)),
            DaemonError::Monitor(msg) => BlastError::daemon(format!("Monitor error: {}", msg)),
            DaemonError::Core(msg) => BlastError::daemon(msg),
            DaemonError::Resolver(msg) => BlastError::resolution(msg),
            DaemonError::Transaction(msg) => BlastError::daemon(format!("Transaction error: {}", msg)),
            DaemonError::Validation(msg) => BlastError::daemon(format!("Validation error: {}", msg)),
            DaemonError::Version(msg) => BlastError::version(msg),
            DaemonError::ResourceLimit(msg) => BlastError::daemon(format!("Resource limit error: {}", msg)),
            DaemonError::State(msg) => BlastError::daemon(format!("State error: {}", msg)),
            DaemonError::Snapshot(msg) => BlastError::daemon(format!("Snapshot error: {}", msg)),
            DaemonError::Access(msg) => BlastError::daemon(msg),
            DaemonError::Environment(msg) => BlastError::environment(msg),
            DaemonError::Io(msg) => BlastError::Io(msg),
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

    pub fn state(msg: impl Into<String>) -> Self {
        Self::State(msg.into())
    }

    pub fn snapshot(msg: impl Into<String>) -> Self {
        Self::Snapshot(msg.into())
    }

    pub fn access(msg: impl Into<String>) -> Self {
        Self::Access(msg.into())
    }

    pub fn environment(msg: impl Into<String>) -> Self {
        Self::Environment(msg.into())
    }

    pub fn io(msg: impl Into<String>) -> Self {
        Self::Io(msg.into())
    }
}

/// Result type for daemon operations
pub type DaemonResult<T> = Result<T, DaemonError>; 