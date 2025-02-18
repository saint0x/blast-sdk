use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};

use crate::error::BlastResult;
use crate::version::{Version, VersionConstraint};
use crate::metadata::{PackageMetadata, BuildMetadata, DistributionMetadata};

/// Unique identifier for a package
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackageId {
    name: String,
    version: Version,
}

impl PackageId {
    /// Create a new package ID
    pub fn new(name: impl Into<String>, version: Version) -> Self {
        Self {
            name: name.into(),
            version,
        }
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the package version
    pub fn version(&self) -> &Version {
        &self.version
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=={}", self.name, self.version)
    }
}

/// A Python package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package identifier
    id: PackageId,
    /// Package metadata
    metadata: PackageMetadata,
    /// Build metadata
    build_metadata: Option<BuildMetadata>,
    /// Distribution metadata
    dist_metadata: Option<DistributionMetadata>,
}

impl Package {
    /// Create a new package
    pub fn new(
        name: String,
        version_str: String,
        dependencies: impl Into<PackageMetadata>,
        python_version: VersionConstraint,
    ) -> BlastResult<Self> {
        let version = Version::parse(&version_str)?;
        let id = PackageId::new(name.clone(), version.clone());
        let mut metadata = dependencies.into();
        metadata.python_version = python_version;

        Ok(Self {
            id,
            metadata,
            build_metadata: None,
            dist_metadata: None,
        })
    }

    /// Get the package ID
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        self.id.name()
    }

    /// Get the package version
    pub fn version(&self) -> &Version {
        self.id.version()
    }

    /// Get the package metadata
    pub fn metadata(&self) -> &PackageMetadata {
        &self.metadata
    }

    /// Get mutable package metadata
    pub fn metadata_mut(&mut self) -> &mut PackageMetadata {
        &mut self.metadata
    }

    /// Get the build metadata
    pub fn build_metadata(&self) -> Option<&BuildMetadata> {
        self.build_metadata.as_ref()
    }

    /// Set the build metadata
    pub fn set_build_metadata(&mut self, metadata: BuildMetadata) {
        self.build_metadata = Some(metadata);
    }

    /// Get the distribution metadata
    pub fn dist_metadata(&self) -> Option<&DistributionMetadata> {
        self.dist_metadata.as_ref()
    }

    /// Set the distribution metadata
    pub fn set_dist_metadata(&mut self, metadata: DistributionMetadata) {
        self.dist_metadata = Some(metadata);
    }

    /// Check if package is compatible with the given Python version
    pub fn is_python_compatible(&self, version: &str) -> BlastResult<bool> {
        self.metadata.is_python_compatible(version)
    }

    /// Check if package is compatible with the given platform
    pub fn is_platform_compatible(&self, platform: &str) -> bool {
        self.metadata.is_platform_compatible(platform)
    }

    /// Get all dependencies including specified extras
    pub fn all_dependencies(&self, extras: &[String]) -> HashMap<String, VersionConstraint> {
        self.metadata.all_dependencies(extras)
    }

    /// Check if package has a specific extra
    pub fn has_extra(&self, extra: &str) -> bool {
        self.metadata.has_extra(extra)
    }

    /// Get dependencies for a specific extra
    pub fn extra_dependencies(&self, extra: &str) -> Option<HashMap<String, VersionConstraint>> {
        self.metadata.extra_dependencies(extra).map(|deps| deps.clone())
    }
} 