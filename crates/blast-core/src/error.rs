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
    Io(#[from] io::Error),

    #[error("Python error: {0}")]
    Python(String),

    #[error("Package error: {0}")]
    Package(String),

    #[error("Environment error: {0}")]
    Environment(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Dependency resolution error: {0}")]
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
    Pattern(#[from] PatternError),

    #[error("Daemon error: {0}")]
    Daemon(String),

    #[error("Other error: {0}")]
    Other(String),

    /// Sync-related errors
    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Security error: {0}")]
    Security(String),
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
}

impl From<toml::de::Error> for BlastError {
    fn from(err: toml::de::Error) -> Self {
        BlastError::Config(format!("TOML deserialization error: {}", err))
    }
}

impl From<toml::ser::Error> for BlastError {
    fn from(err: toml::ser::Error) -> Self {
        BlastError::Config(format!("TOML serialization error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = BlastError::python("Python error");
        assert!(matches!(err, BlastError::Python(_)));

        let err = BlastError::package("Package error");
        assert!(matches!(err, BlastError::Package(_)));

        let err = BlastError::environment("Environment error");
        assert!(matches!(err, BlastError::Environment(_)));
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: BlastError = io_err.into();
        assert!(matches!(err, BlastError::Io(_)));

        let pattern_err = glob::Pattern::new("[").unwrap_err();
        let err: BlastError = pattern_err.into();
        assert!(matches!(err, BlastError::Pattern(_)));
    }

    #[test]
    fn test_error_display() {
        let err = BlastError::python("test error");
        assert_eq!(err.to_string(), "Python error: test error");

        let err = BlastError::package("test error");
        assert_eq!(err.to_string(), "Package error: test error");
    }
} 