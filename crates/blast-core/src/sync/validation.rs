use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Validation of sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncValidation {
    /// Whether the sync is valid
    pub is_valid: bool,
    /// List of validation issues
    pub issues: Vec<ValidationIssue>,
    /// Performance impact assessment
    pub performance_impact: PerformanceImpact,
}

/// Issue found during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Severity of the issue
    pub severity: IssueSeverity,
    /// Description of the issue
    pub description: String,
    /// Recommended action
    pub recommendation: String,
}

/// Severity of validation issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Critical issue that must be resolved
    Critical,
    /// Warning that should be reviewed
    Warning,
    /// Informational message
    Info,
}

/// Performance impact of sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImpact {
    /// Estimated time to complete sync
    #[serde(with = "duration_serde")]
    pub estimated_duration: Duration,
    /// Required disk space
    pub required_space: u64,
    /// Network bandwidth required
    pub network_bandwidth: u64,
    /// CPU usage estimate
    pub cpu_usage: f32,
    /// Memory usage estimate
    pub memory_usage: u64,
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
} 