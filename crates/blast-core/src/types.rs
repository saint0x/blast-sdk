use std::path::PathBuf;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Strategy for updating packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateStrategy {
    /// Never update packages automatically
    Never,
    /// Update only when explicitly requested
    Manual,
    /// Update packages in the background
    Automatic {
        /// Interval between update checks
        #[serde(with = "duration_serde")]
        interval: Duration,
        /// Whether to update only direct dependencies
        direct_only: bool,
    },
}

impl Default for UpdateStrategy {
    fn default() -> Self {
        Self::Manual
    }
}

/// Settings for package caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    /// Directory for storing cached packages
    pub cache_dir: PathBuf,
    /// Maximum cache size in bytes
    pub max_size: u64,
    /// Time to keep unused packages in cache
    #[serde(with = "duration_serde")]
    pub ttl: Duration,
    /// Whether to use hardlinks when possible
    pub use_hardlinks: bool,
    /// Whether to use copy-on-write when possible
    pub use_cow: bool,
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            cache_dir: default_cache_dir(),
            max_size: 10 * 1024 * 1024 * 1024, // 10 GB
            ttl: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            use_hardlinks: true,
            use_cow: true,
        }
    }
}

/// Get the default cache directory
fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("~/.cache"))
        .join("blast")
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Logging level for Blast operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
} 