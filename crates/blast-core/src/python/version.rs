use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::error::{BlastError, BlastResult};

/// Python version
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PythonVersion {
    major: u32,
    minor: u32,
    patch: Option<u32>,
}

impl Default for PythonVersion {
    fn default() -> Self {
        Self {
            major: 3,
            minor: 8,
            patch: Some(0),
        }
    }
}

impl PythonVersion {
    /// Create new Python version
    pub fn new(major: u32, minor: u32, patch: Option<u32>) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse Python version from string
    pub fn parse(version: &str) -> BlastResult<Self> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return Err(BlastError::Python(format!(
                "Invalid Python version format: {}",
                version
            )));
        }

        let major = parts[0].parse().map_err(|_| {
            BlastError::Python(format!("Invalid major version: {}", parts[0]))
        })?;
        let minor = parts[1].parse().map_err(|_| {
            BlastError::Python(format!("Invalid minor version: {}", parts[1]))
        })?;
        let patch = if parts.len() == 3 {
            Some(parts[2].parse().map_err(|_| {
                BlastError::Python(format!("Invalid patch version: {}", parts[2]))
            })?)
        } else {
            None
        };

        Ok(Self::new(major, minor, patch))
    }

    /// Get major version
    pub fn major(&self) -> u32 {
        self.major
    }

    /// Get minor version
    pub fn minor(&self) -> u32 {
        self.minor
    }

    /// Get patch version
    pub fn patch(&self) -> Option<u32> {
        self.patch
    }

    /// Check if this version is compatible with another version
    pub fn is_compatible_with(&self, other: &PythonVersion) -> bool {
        // Major version must match exactly
        if self.major != other.major {
            return false;
        }

        // Minor version must be greater than or equal
        if self.minor < other.minor {
            return false;
        }

        // If minor versions match and both have patch versions, compare them
        if self.minor == other.minor {
            match (self.patch, other.patch) {
                (Some(self_patch), Some(other_patch)) => {
                    if self_patch < other_patch {
                        return false;
                    }
                }
                // If one version has no patch number, they're compatible
                _ => {}
            }
        }

        true
    }
}

impl std::fmt::Display for PythonVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.patch {
            Some(patch) => write!(f, "{}.{}.{}", self.major, self.minor, patch),
            None => write!(f, "{}.{}", self.major, self.minor),
        }
    }
}

impl FromStr for PythonVersion {
    type Err = BlastError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let version = PythonVersion::parse("3.9.0").unwrap();
        assert_eq!(version.major(), 3);
        assert_eq!(version.minor(), 9);
        assert_eq!(version.patch(), Some(0));

        let version = PythonVersion::parse("3.9").unwrap();
        assert_eq!(version.major(), 3);
        assert_eq!(version.minor(), 9);
        assert_eq!(version.patch(), None);
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = PythonVersion::parse("3.9.0").unwrap();
        let v2 = PythonVersion::parse("3.9.1").unwrap();
        let v3 = PythonVersion::parse("3.8.0").unwrap();
        let v4 = PythonVersion::parse("4.0.0").unwrap();

        assert!(v2.is_compatible_with(&v1));
        assert!(!v1.is_compatible_with(&v2));
        assert!(v1.is_compatible_with(&v3));
        assert!(!v3.is_compatible_with(&v1));
        assert!(!v1.is_compatible_with(&v4));
    }
} 