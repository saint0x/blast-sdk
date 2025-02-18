use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::version::VersionConstraint;
use crate::error::BlastResult;

/// Package metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    pub description: Option<String>,
    /// Package author
    pub author: Option<String>,
    /// Package homepage
    pub homepage: Option<String>,
    /// Package license
    pub license: Option<String>,
    /// Package keywords
    pub keywords: Vec<String>,
    /// Package classifiers
    pub classifiers: Vec<String>,
    /// Package documentation URL
    pub documentation: Option<String>,
    /// Package repository URL
    pub repository: Option<String>,
    /// Package dependencies
    pub dependencies: HashMap<String, VersionConstraint>,
    /// Package extras
    pub extras: HashMap<String, HashMap<String, VersionConstraint>>,
    /// Required Python version
    pub python_version: VersionConstraint,
    /// Package platform tags
    pub platform_tags: Vec<String>,
    /// Package is yanked
    pub yanked: bool,
    /// Yanked reason if applicable
    pub yanked_reason: Option<String>,
}

impl PackageMetadata {
    /// Create new package metadata
    pub fn new(
        name: String,
        version: String,
        dependencies: HashMap<String, VersionConstraint>,
        python_version: VersionConstraint,
    ) -> Self {
        Self {
            name,
            version,
            description: None,
            author: None,
            homepage: None,
            license: None,
            keywords: Vec::new(),
            classifiers: Vec::new(),
            documentation: None,
            repository: None,
            dependencies,
            extras: HashMap::new(),
            python_version,
            platform_tags: Vec::new(),
            yanked: false,
            yanked_reason: None,
        }
    }

    /// Check if package is compatible with the given Python version
    pub fn is_python_compatible(&self, version: &str) -> BlastResult<bool> {
        let version = crate::version::Version::parse(version)?;
        Ok(self.python_version.matches(&version))
    }

    /// Check if package is compatible with the given platform
    pub fn is_platform_compatible(&self, platform: &str) -> bool {
        if self.platform_tags.is_empty() {
            return true; // No platform restrictions
        }
        self.platform_tags.iter().any(|tag| tag == platform || tag == "any")
    }

    /// Get all dependencies including specified extras
    pub fn all_dependencies(&self, extras: &[String]) -> HashMap<String, VersionConstraint> {
        let mut deps = self.dependencies.clone();
        for extra in extras {
            if let Some(extra_deps) = self.extras.get(extra) {
                deps.extend(extra_deps.clone());
            }
        }
        deps
    }

    /// Check if package has a specific extra
    pub fn has_extra(&self, extra: &str) -> bool {
        self.extras.contains_key(extra)
    }

    /// Get dependencies for a specific extra
    pub fn extra_dependencies(&self, extra: &str) -> Option<&HashMap<String, VersionConstraint>> {
        self.extras.get(extra)
    }
}

/// Build metadata for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMetadata {
    /// Build system requirements
    pub build_requires: Vec<String>,
    /// Backend requirements
    pub backend_requires: Vec<String>,
    /// Build backend
    pub build_backend: String,
    /// Backend options
    pub backend_options: HashMap<String, String>,
}

impl Default for BuildMetadata {
    fn default() -> Self {
        Self {
            build_requires: vec!["setuptools>=40.8.0".to_string(), "wheel".to_string()],
            backend_requires: Vec::new(),
            build_backend: "setuptools.build_meta".to_string(),
            backend_options: HashMap::new(),
        }
    }
}

/// Package distribution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionMetadata {
    /// Distribution type (wheel, sdist)
    pub dist_type: DistributionType,
    /// Python tags
    pub python_tags: Vec<String>,
    /// ABI tags
    pub abi_tags: Vec<String>,
    /// Platform tags
    pub platform_tags: Vec<String>,
    /// Build number
    pub build_number: Option<u32>,
    /// Build metadata
    pub build_metadata: Option<BuildMetadata>,
}

/// Type of distribution
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DistributionType {
    /// Wheel distribution
    Wheel,
    /// Source distribution
    Sdist,
    /// Egg distribution (legacy)
    Egg,
}

impl DistributionType {
    // ... existing code ...
} 