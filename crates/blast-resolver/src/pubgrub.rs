use std::cmp::Ordering;
use std::fmt;

use blast_core::version::Version;
use pubgrub::version::Version as PubgrubVersionTrait;

/// Version wrapper for PubGrub compatibility
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct PubgrubVersion(pub(crate) Version);

impl From<Version> for PubgrubVersion {
    fn from(v: Version) -> Self {
        Self(v)
    }
}

impl fmt::Display for PubgrubVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialOrd for PubgrubVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PubgrubVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PubgrubVersionTrait for PubgrubVersion {
    fn lowest() -> Self {
        Self(Version::parse("0.0.0").unwrap())
    }

    fn bump(&self) -> Self {
        // Simple increment of patch version
        // In practice, we should use proper version bumping logic
        let version_str = format!("{}.0", self.0);
        Self(Version::parse(&version_str).unwrap())
    }
} 