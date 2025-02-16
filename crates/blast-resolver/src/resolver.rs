use std::collections::HashMap;
use std::sync::Arc;
use std::cmp::Ordering;
use std::borrow::Borrow;
use std::error::Error as StdError;

use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::solver::{Dependencies, DependencyProvider};
use pubgrub::version::Version as PubgrubVersionTrait;
use tokio::sync::RwLock;
use tracing::debug;

use blast_core::error::{BlastError, BlastResult};
use blast_core::package::{Package, PackageId, Version};

use crate::cache::Cache;
use crate::pypi::PyPIClient;

/// Dependency resolver for Python packages
pub struct DependencyResolver {
    pub(crate) pypi: PyPIClient,
    cache: Arc<RwLock<Cache>>,
    resolution_cache: Arc<RwLock<HashMap<PackageId, Vec<Package>>>>,
}

impl DependencyResolver {
    /// Create a new resolver
    pub fn new(pypi: PyPIClient, cache: Cache) -> Self {
        Self {
            pypi,
            cache: Arc::new(RwLock::new(cache)),
            resolution_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get available versions for a package
    pub async fn get_package_versions(&self, name: &str) -> BlastResult<Vec<Version>> {
        self.pypi.get_package_versions(name).await
    }

    /// Resolve dependencies for a package
    pub async fn resolve(&self, package: &Package) -> BlastResult<Vec<Package>> {
        // Check resolution cache first
        let cache_key = package.id().clone();
        if let Some(deps) = self.resolution_cache.read().await.get(&cache_key) {
            debug!("Using cached resolution for {}", package.name());
            return Ok(deps.clone());
        }

        let provider = PubGrubProvider::new(self.pypi.clone());
        let root = package.name().to_string();
        let root_version = PubgrubVersion::from(package.version().clone());

        // Convert PubGrub errors to our error type which implements Send + Sync
        let solution = pubgrub::solver::resolve(&provider, root.clone(), root_version)
            .map_err(|e| match e {
                PubGrubError::NoSolution(tree) => BlastError::resolution(format!(
                    "No solution found for package {}: {:?}",
                    package.name(),
                    tree
                )),
                _ => BlastError::resolution(format!(
                    "Resolution error for package {}: {}",
                    package.name(),
                    e
                )),
            })?;

        let mut packages = Vec::new();
        for (name, version) in solution {
            if name != root {
                // Check package cache first
                let pkg_id = PackageId::new(name.clone(), version.0.clone());
                let pkg = if let Some(cached_pkg) = self.cache.write().await.get_package(&pkg_id) {
                    debug!("Using cached package {}", pkg_id);
                    cached_pkg.clone()
                } else {
                    let pkg = self.pypi.get_package_metadata(&name).await?;
                    self.cache.write().await.store_package(pkg.clone()).await?;
                    pkg
                };
                packages.push(pkg);
            }
        }

        // Cache the resolution
        self.resolution_cache.write().await.insert(cache_key, packages.clone());
        Ok(packages)
    }

    /// Clear resolution cache
    pub async fn clear_cache(&self) {
        self.resolution_cache.write().await.clear();
    }

    /// Resolve dependencies for a package with specified extras
    pub async fn resolve_with_extras(&self, package: &Package, extras: &[String]) -> BlastResult<Vec<Package>> {
        // Get all dependencies including extras
        let all_deps = package.all_dependencies(extras);
        
        // Create a new package with all dependencies
        let package_with_extras = Package::new(
            package.id().clone(),
            all_deps,
            package.python_version().clone(),
        );

        self.resolve(&package_with_extras).await
    }
}

/// PubGrub dependency provider
struct PubGrubProvider {
    pypi: PyPIClient,
}

/// Wrapper type for Version to implement PubgrubVersion
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PubgrubVersion(pub(crate) Version);

impl std::fmt::Display for PubgrubVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PubgrubVersionTrait for PubgrubVersion {
    fn lowest() -> Self {
        PubgrubVersion(Version::parse("0.0.0").unwrap())
    }

    fn bump(&self) -> Self {
        // Since we can't access Version's internals directly,
        // we'll parse the version string and increment
        let version_str = self.0.to_string();
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() >= 3 {
            let major: u64 = parts[0].parse().unwrap_or(0);
            let minor: u64 = parts[1].parse().unwrap_or(0);
            let patch: u64 = parts[2].parse().unwrap_or(0);
            let new_version = format!("{}.{}.{}", major, minor, patch + 1);
            PubgrubVersion(Version::parse(&new_version).unwrap())
        } else {
            // Fallback to a simple increment
            PubgrubVersion(Version::parse("0.0.1").unwrap())
        }
    }
}

impl From<Version> for PubgrubVersion {
    fn from(v: Version) -> Self {
        PubgrubVersion(v)
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

impl PubGrubProvider {
    pub fn new(pypi: PyPIClient) -> Self {
        Self { pypi }
    }
}

impl DependencyProvider<String, PubgrubVersion> for PubGrubProvider {
    fn get_dependencies(
        &self,
        package: &String,
        version: &PubgrubVersion,
    ) -> Result<Dependencies<String, PubgrubVersion>, Box<dyn StdError>> {
        let rt = tokio::runtime::Handle::current();
        match rt.block_on(async {
            self.pypi.get_dependencies(package, &version.0).await
        }) {
            Ok(deps) => Ok(deps),
            Err(e) => Err(Box::new(BlastError::resolution(format!(
                "Failed to get dependencies for {}: {}",
                package, e
            ))) as Box<dyn StdError>),
        }
    }

    fn choose_package_version<T, U>(
        &self,
        available_versions: impl Iterator<Item = (T, U)>,
    ) -> Result<(T, Option<PubgrubVersion>), Box<dyn StdError>>
    where
        T: Borrow<String>,
        U: Borrow<Range<PubgrubVersion>>,
    {
        // Find the highest compatible version
        let result = available_versions
            .filter_map(|(package, range)| {
                // Get all versions from PyPI
                let rt = tokio::runtime::Handle::current();
                let versions = match rt.block_on(async {
                    self.pypi.get_package_versions(package.borrow()).await
                }) {
                    Ok(v) => v,
                    Err(_) => return None,
                };

                // Find highest version that satisfies the range
                let max_version = versions.into_iter()
                    .map(PubgrubVersion)
                    .filter(|v| range.borrow().contains(v))
                    .max();

                Some((package, max_version))
            })
            .next()
            .ok_or_else(|| Box::new(BlastError::resolution(
                "No available versions satisfy the constraints".to_string()
            )) as Box<dyn StdError>)?;
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blast_core::package::VersionConstraint;

    #[tokio::test]
    async fn test_dependency_resolution() {
        let pypi = PyPIClient::new(10, 30, false).unwrap();
        let cache = Cache::new(std::env::temp_dir().join("blast-test"));
        let resolver = DependencyResolver::new(pypi, cache);

        let package = Package::new(
            PackageId::new("requests", Version::parse("2.28.2").unwrap()),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        let deps = resolver.resolve(&package).await.unwrap();
        assert!(!deps.is_empty());
    }

    #[tokio::test]
    async fn test_resolver_creation() {
        let resolver = create_resolver().await.unwrap();
        
        // Test basic import resolution
        let numpy = resolver.resolve_import("numpy").await.unwrap();
        assert!(numpy.is_some());
        
        // Test version parsing
        let version = Version::parse("1.0.0").unwrap();
        assert!(resolver.is_available("numpy").await);
    }
} 