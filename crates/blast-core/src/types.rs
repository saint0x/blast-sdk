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
    Error,
    Warn,
    Info,
    Debug,
    Trace,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_update_strategy_serialization() {
        let strategy = UpdateStrategy::Automatic {
            interval: Duration::from_secs(3600),
            direct_only: true,
        };

        let serialized = serde_json::to_string(&strategy).unwrap();
        let deserialized: UpdateStrategy = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            UpdateStrategy::Automatic {
                interval,
                direct_only,
            } => {
                assert_eq!(interval.as_secs(), 3600);
                assert!(direct_only);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_cache_settings_defaults() {
        let settings = CacheSettings::default();
        assert!(settings.use_hardlinks);
        assert!(settings.use_cow);
        assert_eq!(settings.max_size, 10 * 1024 * 1024 * 1024);
        assert_eq!(settings.ttl.as_secs(), 30 * 24 * 60 * 60);
    }

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(
            tracing::Level::INFO,
            LogLevel::default().into()
        );
        assert_eq!(
            tracing::Level::DEBUG,
            LogLevel::Debug.into()
        );
    }
} 