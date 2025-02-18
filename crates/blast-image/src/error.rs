use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Error type for blast-image operations
#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {source}")]
    Io {
        source: io::Error,
        path: Option<PathBuf>,
    },

    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Validation error: {message}")]
    Validation {
        message: String,
        code: Option<String>,
    },

    #[error("Platform error: {message}")]
    Platform {
        message: String,
        platform: Option<String>,
    },

    #[error("Layer error: {message}")]
    Layer {
        message: String,
        layer: Option<String>,
    },

    #[error("Package error: {message}")]
    Package {
        message: String,
        package: Option<String>,
    },

    #[error("Compression error: {message}")]
    Compression {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Configuration error: {message}")]
    Config {
        message: String,
        key: Option<String>,
    },

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io {
            source: error,
            path: None,
        }
    }
}

impl Error {
    /// Create a new IO error with path
    pub fn io(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Io {
            source,
            path: Some(path.into()),
        }
    }

    /// Create a new serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new serialization error with source
    pub fn serialization_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Serialization {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// Create a new validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            code: None,
        }
    }

    /// Create a new validation error with code
    pub fn validation_with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            code: Some(code.into()),
        }
    }

    /// Create a new platform error
    pub fn platform(message: impl Into<String>) -> Self {
        Self::Platform {
            message: message.into(),
            platform: None,
        }
    }

    /// Create a new platform error with platform info
    pub fn platform_with_info(message: impl Into<String>, platform: impl Into<String>) -> Self {
        Self::Platform {
            message: message.into(),
            platform: Some(platform.into()),
        }
    }

    /// Create a new layer error
    pub fn layer(message: impl Into<String>) -> Self {
        Self::Layer {
            message: message.into(),
            layer: None,
        }
    }

    /// Create a new layer error with layer name
    pub fn layer_with_name(message: impl Into<String>, layer: impl Into<String>) -> Self {
        Self::Layer {
            message: message.into(),
            layer: Some(layer.into()),
        }
    }

    /// Create a new package error
    pub fn package(message: impl Into<String>) -> Self {
        Self::Package {
            message: message.into(),
            package: None,
        }
    }

    /// Create a new package error with package name
    pub fn package_with_name(message: impl Into<String>, package: impl Into<String>) -> Self {
        Self::Package {
            message: message.into(),
            package: Some(package.into()),
        }
    }

    /// Create a new compression error
    pub fn compression(message: impl Into<String>) -> Self {
        Self::Compression {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new compression error with source
    pub fn compression_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Compression {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// Create a new configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            key: None,
        }
    }

    /// Create a new configuration error with key
    pub fn config_with_key(message: impl Into<String>, key: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            key: Some(key.into()),
        }
    }
}

/// Result type for blast-image operations
pub type Result<T> = std::result::Result<T, Error>; 