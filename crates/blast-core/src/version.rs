use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use semver::{Version as SemVer, VersionReq};

use crate::error::{BlastError, BlastResult};

/// Package version following PEP 440
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Version(SemVer);

impl Version {
    /// Create a new version
    pub fn new(version: SemVer) -> Self {
        Self(version)
    }

    /// Parse a version string
    pub fn parse(version: &str) -> BlastResult<Self> {
        Ok(Self(SemVer::parse(version).map_err(|e| {
            BlastError::version(format!("Invalid version '{}': {}", version, e))
        })?))
    }

    /// Get the underlying semver version
    pub fn as_semver(&self) -> &SemVer {
        &self.0
    }
}

impl FromStr for Version {
    type Err = BlastError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// Version constraint following PEP 440
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct VersionConstraint(VersionReq);

impl Default for VersionConstraint {
    fn default() -> Self {
        Self::any()
    }
}

impl VersionConstraint {
    /// Create a new version constraint
    pub fn new(req: VersionReq) -> Self {
        Self(req)
    }

    /// Create a constraint that matches any version
    pub fn any() -> Self {
        Self(VersionReq::STAR)
    }

    /// Parse a version constraint string
    pub fn parse(constraint: &str) -> BlastResult<Self> {
        Ok(Self(VersionReq::parse(constraint).map_err(|e| {
            BlastError::version(format!("Invalid version constraint '{}': {}", constraint, e))
        })?))
    }

    /// Check if a version matches this constraint
    pub fn matches(&self, version: &Version) -> bool {
        self.0.matches(version.as_semver())
    }
}

impl FromStr for VersionConstraint {
    type Err = BlastError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
} 