use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use semver::{Version as SemVer, VersionReq};

use crate::error::{BlastError, BlastResult};

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

/// Version constraint for package dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Create a version constraint that matches any version
    pub fn any() -> Self {
        Self::parse("*").unwrap()
    }

    /// Parse a version constraint string
    pub fn parse(constraint: &str) -> BlastResult<Self> {
        VersionReq::parse(constraint)
            .map(Self)
            .map_err(|e| BlastError::version(e.to_string()))
    }

    /// Check if a version satisfies this constraint
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

/// Enhanced version constraint system
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum VersionRequirement {
    /// Exact version match
    Exact(Version),
    /// Version range
    Range {
        min: Option<Version>,
        max: Option<Version>,
        include_min: bool,
        include_max: bool,
    },
    /// Multiple requirements (AND)
    And(Vec<VersionRequirement>),
    /// Alternative requirements (OR)
    Or(Vec<VersionRequirement>),
    /// Negated requirement (NOT)
    Not(Box<VersionRequirement>),
    /// Any version
    Any,
}

impl VersionRequirement {
    /// Create a new exact version requirement
    pub fn exact(version: Version) -> Self {
        Self::Exact(version)
    }

    /// Create a requirement that matches any version
    pub fn any() -> Self {
        Self::Any
    }

    /// Create a new range requirement
    pub fn range(
        min: Option<Version>,
        max: Option<Version>,
        include_min: bool,
        include_max: bool,
    ) -> Self {
        Self::Range {
            min,
            max,
            include_min,
            include_max,
        }
    }

    /// Create a requirement for versions greater than
    pub fn greater_than(version: Version) -> Self {
        Self::Range {
            min: Some(version),
            max: None,
            include_min: false,
            include_max: false,
        }
    }

    /// Create a requirement for versions greater than or equal to
    pub fn greater_than_eq(version: Version) -> Self {
        Self::Range {
            min: Some(version),
            max: None,
            include_min: true,
            include_max: false,
        }
    }

    /// Create a requirement for versions less than
    pub fn less_than(version: Version) -> Self {
        Self::Range {
            min: None,
            max: Some(version),
            include_min: false,
            include_max: false,
        }
    }

    /// Create a requirement for versions less than or equal to
    pub fn less_than_eq(version: Version) -> Self {
        Self::Range {
            min: None,
            max: Some(version),
            include_min: false,
            include_max: true,
        }
    }

    /// Check if a version satisfies this requirement
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Exact(req) => req == version,
            Self::Range { min, max, include_min, include_max } => {
                let min_satisfied = min.as_ref().map_or(true, |min| {
                    if *include_min {
                        version >= min
                    } else {
                        version > min
                    }
                });

                let max_satisfied = max.as_ref().map_or(true, |max| {
                    if *include_max {
                        version <= max
                    } else {
                        version < max
                    }
                });

                min_satisfied && max_satisfied
            }
            Self::And(reqs) => reqs.iter().all(|req| req.matches(version)),
            Self::Or(reqs) => reqs.iter().any(|req| req.matches(version)),
            Self::Not(req) => !req.matches(version),
            Self::Any => true,
        }
    }

    /// Parse a version requirement string
    pub fn parse(input: &str) -> BlastResult<Self> {
        if input == "*" {
            return Ok(Self::Any);
        }

        let mut chars = input.chars().peekable();
        let mut requirements = Vec::new();
        let mut current = String::new();

        while let Some(c) = chars.next() {
            match c {
                ',' => {
                    if !current.is_empty() {
                        requirements.push(Self::parse_single(&current)?);
                        current.clear();
                    }
                }
                '|' => {
                    if chars.peek() == Some(&'|') {
                        chars.next(); // consume second '|'
                        if !current.is_empty() {
                            requirements.push(Self::parse_single(&current)?);
                            current.clear();
                        }
                        return Ok(Self::Or(requirements));
                    } else {
                        current.push(c);
                    }
                }
                _ => current.push(c),
            }
        }

        if !current.is_empty() {
            requirements.push(Self::parse_single(&current)?);
        }

        if requirements.len() == 1 {
            Ok(requirements.pop().unwrap())
        } else {
            Ok(Self::And(requirements))
        }
    }

    /// Parse a single version requirement
    fn parse_single(input: &str) -> BlastResult<Self> {
        let input = input.trim();
        
        if input.starts_with('!') {
            return Ok(Self::Not(Box::new(Self::parse_single(&input[1..])?))); 
        }

        if input.starts_with('=') {
            let version = Version::parse(&input[1..].trim())?;
            return Ok(Self::Exact(version));
        }

        if input.starts_with(">=") {
            let version = Version::parse(&input[2..].trim())?;
            return Ok(Self::greater_than_eq(version));
        }

        if input.starts_with('>') {
            let version = Version::parse(&input[1..].trim())?;
            return Ok(Self::greater_than(version));
        }

        if input.starts_with("<=") {
            let version = Version::parse(&input[2..].trim())?;
            return Ok(Self::less_than_eq(version));
        }

        if input.starts_with('<') {
            let version = Version::parse(&input[1..].trim())?;
            return Ok(Self::less_than(version));
        }

        // Try parsing as exact version
        let version = Version::parse(input)?;
        Ok(Self::Exact(version))
    }
}

impl FromStr for VersionRequirement {
    type Err = BlastError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for VersionRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(v) => write!(f, "={}", v),
            Self::Range { min, max, include_min, include_max } => {
                if let Some(min) = min {
                    if *include_min {
                        write!(f, ">={}", min)?;
                    } else {
                        write!(f, ">{}", min)?;
                    }
                }
                if min.is_some() && max.is_some() {
                    write!(f, ", ")?;
                }
                if let Some(max) = max {
                    if *include_max {
                        write!(f, "<={}", max)?;
                    } else {
                        write!(f, "<{}", max)?;
                    }
                }
                Ok(())
            }
            Self::And(reqs) => {
                let mut first = true;
                for req in reqs {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", req)?;
                    first = false;
                }
                Ok(())
            }
            Self::Or(reqs) => {
                let mut first = true;
                for req in reqs {
                    if !first {
                        write!(f, " || ")?;
                    }
                    write!(f, "{}", req)?;
                    first = false;
                }
                Ok(())
            }
            Self::Not(req) => write!(f, "!{}", req),
            Self::Any => write!(f, "*"),
        }
    }
}

/// A Python package with its metadata and dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    id: PackageId,
    description: Option<String>,
    author: Option<String>,
    homepage: Option<String>,
    dependencies: HashMap<String, VersionConstraint>,
    python_version: VersionConstraint,
    extras: HashMap<String, HashMap<String, VersionConstraint>>,
}

impl Package {
    /// Create a new package
    pub fn new(
        id: PackageId,
        dependencies: HashMap<String, VersionConstraint>,
        python_version: VersionConstraint,
    ) -> Self {
        Self {
            id,
            description: None,
            author: None,
            homepage: None,
            dependencies,
            python_version,
            extras: HashMap::new(),
        }
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

    /// Get the package description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Set the package description
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = Some(description.into());
    }

    /// Get the package author
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    /// Set the package author
    pub fn set_author(&mut self, author: impl Into<String>) {
        self.author = Some(author.into());
    }

    /// Get the package homepage
    pub fn homepage(&self) -> Option<&str> {
        self.homepage.as_deref()
    }

    /// Set the package homepage
    pub fn set_homepage(&mut self, homepage: impl Into<String>) {
        self.homepage = Some(homepage.into());
    }

    /// Get the package dependencies
    pub fn dependencies(&self) -> &HashMap<String, VersionConstraint> {
        &self.dependencies
    }

    /// Get the required Python version
    pub fn python_version(&self) -> &VersionConstraint {
        &self.python_version
    }

    /// Get the available extras
    pub fn extras(&self) -> &HashMap<String, HashMap<String, VersionConstraint>> {
        &self.extras
    }

    /// Get dependencies for a specific extra
    pub fn extra_dependencies(&self, extra: &str) -> Option<&HashMap<String, VersionConstraint>> {
        self.extras.get(extra)
    }

    /// Set dependencies for a specific extra
    pub fn set_extra_dependencies(&mut self, extra: String, dependencies: HashMap<String, VersionConstraint>) {
        self.extras.insert(extra, dependencies);
    }

    /// Get all dependencies including those from the specified extras
    pub fn all_dependencies(&self, extras: &[String]) -> HashMap<String, VersionConstraint> {
        let mut all_deps = self.dependencies.clone();
        
        for extra in extras {
            if let Some(extra_deps) = self.extras.get(extra) {
                for (pkg, constraint) in extra_deps {
                    all_deps.insert(pkg.clone(), constraint.clone());
                }
            }
        }
        
        all_deps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_id() {
        let version = Version::parse("1.0.0").unwrap();
        let id = PackageId::new("test-package", version);
        assert_eq!(id.name(), "test-package");
        assert_eq!(id.version().to_string(), "1.0.0");
        assert_eq!(id.to_string(), "test-package==1.0.0");
    }

    #[test]
    fn test_version_parsing() {
        assert!(Version::parse("1.0.0").is_ok());
        assert!(Version::parse("1.0.0-alpha").is_ok());
        assert!(Version::parse("invalid").is_err());
    }

    #[test]
    fn test_version_constraint() {
        let version = Version::parse("1.0.0").unwrap();
        let constraint = VersionConstraint::parse(">=1.0.0").unwrap();
        assert!(constraint.matches(&version));

        let version = Version::parse("0.9.0").unwrap();
        assert!(!constraint.matches(&version));
    }

    #[test]
    fn test_package() {
        let version = Version::parse("1.0.0").unwrap();
        let id = PackageId::new("test-package", version);
        let mut deps = HashMap::new();
        deps.insert(
            "dep-package".to_string(),
            VersionConstraint::parse(">=2.0.0").unwrap(),
        );
        let python_version = VersionConstraint::parse(">=3.7").unwrap();

        let mut package = Package::new(id, deps, python_version);
        package.set_description("Test package");
        package.set_author("Test Author");
        package.set_homepage("https://example.com");

        assert_eq!(package.name(), "test-package");
        assert_eq!(package.version().to_string(), "1.0.0");
        assert_eq!(package.description(), Some("Test package"));
        assert_eq!(package.author(), Some("Test Author"));
        assert_eq!(package.homepage(), Some("https://example.com"));
        assert_eq!(package.dependencies().len(), 1);
        assert!(package
            .dependencies()
            .get("dep-package")
            .unwrap()
            .matches(&Version::parse("2.0.0").unwrap()));
    }
} 