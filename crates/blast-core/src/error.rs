use std::path::PathBuf;
use std::io;
use glob::PatternError;
use thiserror::Error;

/// Custom result type for Blast operations
pub type BlastResult<T> = Result<T, BlastError>;

/// Custom error type for Blast operations
#[derive(Debug, Error)]
pub enum BlastError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("Python error: {0}")]
    Python(String),

    #[error("Package error: {0}")]
    Package(String),

    #[error("Environment error: {0}")]
    Environment(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Resolution error: {0}")]
    Resolution(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Snapshot error: {0}")]
    Snapshot(String),

    #[error("Import hook error: {0}")]
    ImportHook(String),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Version error: {0}")]
    Version(String),

    #[error("Lock error: {0}")]
    Lock(String),

    #[error("Pattern error: {0}")]
    Pattern(String),

    #[error("Daemon error: {0}")]
    Daemon(String),

    #[error("Command failed: {0} - {1}")]
    CommandFailed(String, String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Manifest error: {0}")]
    Manifest(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("State error: {0}")]
    State(String),
}

impl BlastError {
    /// Create a new Python error
    pub fn python<S: Into<String>>(msg: S) -> Self {
        BlastError::Python(msg.into())
    }

    /// Create a new package error
    pub fn package<S: Into<String>>(msg: S) -> Self {
        BlastError::Package(msg.into())
    }

    /// Create a new environment error
    pub fn environment<S: Into<String>>(msg: S) -> Self {
        BlastError::Environment(msg.into())
    }

    /// Create a new cache error
    pub fn cache<S: Into<String>>(msg: S) -> Self {
        BlastError::Cache(msg.into())
    }

    /// Create a new resolution error
    pub fn resolution<S: Into<String>>(msg: S) -> Self {
        BlastError::Resolution(msg.into())
    }

    /// Create a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        BlastError::Config(msg.into())
    }

    /// Create a new snapshot error
    pub fn snapshot<S: Into<String>>(msg: S) -> Self {
        BlastError::Snapshot(msg.into())
    }

    /// Create a new import hook error
    pub fn import_hook<S: Into<String>>(msg: S) -> Self {
        BlastError::ImportHook(msg.into())
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(msg: S) -> Self {
        BlastError::Network(msg.into())
    }

    /// Create a new serialization error
    pub fn serialization<S: Into<String>>(msg: S) -> Self {
        BlastError::Serialization(msg.into())
    }

    /// Create a new version error
    pub fn version<S: Into<String>>(msg: S) -> Self {
        BlastError::Version(msg.into())
    }

    /// Create a new lock error
    pub fn lock<S: Into<String>>(msg: S) -> Self {
        BlastError::Lock(msg.into())
    }

    /// Create a new other error
    pub fn other<S: Into<String>>(msg: S) -> Self {
        BlastError::Other(msg.into())
    }

    /// Create a new daemon error
    pub fn daemon<S: Into<String>>(msg: S) -> Self {
        BlastError::Daemon(msg.into())
    }

    /// Create a new sync error
    pub fn sync<S: Into<String>>(msg: S) -> Self {
        Self::Sync(msg.into())
    }

    /// Create a new security error
    pub fn security<S: Into<String>>(msg: S) -> Self {
        BlastError::Security(msg.into())
    }

    /// Create a new state error
    pub fn state<S: Into<String>>(msg: S) -> Self {
        Self::State(msg.into())
    }
}

impl From<serde_json::Error> for BlastError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for BlastError {
    fn from(err: toml::de::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<toml::ser::Error> for BlastError {
    fn from(err: toml::ser::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<io::Error> for BlastError {
    fn from(err: io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<PatternError> for BlastError {
    fn from(err: PatternError) -> Self {
        Self::Pattern(err.to_string())
    }
} 