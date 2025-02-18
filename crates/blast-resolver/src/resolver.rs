use std::collections::HashMap;
use std::sync::Arc;
use std::cmp::Ordering;
use std::borrow::Borrow;
use std::error::Error as StdError;
use rustc_hash::FxHashMap;

use pubgrub::range::Range;
use pubgrub::solver::{Dependencies, DependencyProvider};
use pubgrub::version::Version as PubgrubVersionTrait;
use tokio::sync::RwLock;
use tracing::debug;
use async_trait::async_trait;

use blast_core::error::{BlastError, BlastResult};
use blast_core::package::{Package, PackageId};
use blast_core::version::Version;
use blast_core::security::{PackageVerification, PolicyResult, SecurityPolicy, VerificationResult, Vulnerability};

use crate::cache::Cache;
use crate::pypi::PyPIClient;
use crate::resolution::{ResolutionStrategy, ResolutionResult};

/// Dependency resolver for Python packages
pub struct DependencyResolver {
    pub(crate) pypi: PyPIClient,
    cache: Arc<RwLock<Cache>>,
    resolution_cache: Arc<RwLock<HashMap<PackageId, Vec<Package>>>>,
    resolution_strategy: PubGrubProvider,
}

impl DependencyResolver {
    /// Create a new resolver
    pub fn new(pypi: PyPIClient, cache: Cache) -> Self {
        Self {
            pypi: pypi.clone(),
            cache: Arc::new(RwLock::new(cache)),
            resolution_cache: Arc::new(RwLock::new(HashMap::new())),
            resolution_strategy: PubGrubProvider::new(pypi),
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

        let ResolutionResult { packages, .. } = self.resolution_strategy
            .resolve(package, &self.pypi, &self.cache)
            .await?;

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
        let all_deps = package.all_dependencies(extras);
        let mut resolved = Vec::new();

        for (name, constraint) in all_deps {
            let versions = self.get_package_versions(&name).await?;
            // Find highest version that satisfies the constraint
            if let Some(_version) = versions.into_iter().rev().find(|v| constraint.matches(v)) {
                let pkg = self.pypi.get_package_metadata(&name).await?;
                resolved.extend(self.resolve(&pkg).await?);
            }
        }

        Ok(resolved)
    }

    pub async fn resolve_import(&self, import_name: &str) -> BlastResult<Option<Package>> {
        self.pypi.resolve_import(import_name).await
    }

    pub async fn is_available(&self, import_name: &str) -> bool {
        self.pypi.is_available(import_name).await
    }
}

impl PackageVerification for DependencyResolver {
    fn verify_package(&self, _package: &Package) -> BlastResult<VerificationResult> {
        // TODO: Implement package verification
        Ok(VerificationResult {
            verified: true,
            details: String::new(),
            warnings: Vec::new(),
            signature: None,
        })
    }

    fn scan_vulnerabilities(&self, _package: &Package) -> BlastResult<Vec<Vulnerability>> {
        // TODO: Implement vulnerability scanning
        Ok(Vec::new())
    }

    fn verify_policy(&self, _package: &Package, _policy: &SecurityPolicy) -> BlastResult<PolicyResult> {
        // TODO: Implement policy verification
        Ok(PolicyResult {
            allowed: true,
            required_actions: Vec::new(),
            violations: Vec::new(),
        })
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

#[async_trait]
impl ResolutionStrategy for PubGrubProvider {
    async fn resolve(
        &self,
        package: &Package,
        pypi: &PyPIClient,
        cache: &Arc<RwLock<Cache>>,
    ) -> BlastResult<ResolutionResult> {
        let start_time = std::time::Instant::now();
        let mut metrics = crate::resolution::ResolutionMetrics::default();

        let root = package.name().to_string();
        let root_version = PubgrubVersion(package.version().clone());

        let solution = pubgrub::solver::resolve(self, root.clone(), root_version)
            .map_err(|e| BlastError::resolution(format!(
                "No solution found for package {}: {:?}",
                package.name(),
                e
            )))?;

        let mut packages = Vec::new();
        for (name, version) in solution.into_iter() {
            if name != root {
                let pkg_id = PackageId::new(name.clone(), version.0.clone());
                let pkg = if let Some(cached_pkg) = cache.write().await.get_package(&pkg_id) {
                    debug!("Using cached package {}", pkg_id);
                    metrics.cache_hits += 1;
                    cached_pkg.clone()
                } else {
                    metrics.network_requests += 1;
                    let pkg = pypi.get_package_metadata(&name).await?;
                    cache.write().await.store_package(pkg.clone()).await?;
                    pkg
                };
                packages.push(pkg);
            }
        }

        metrics.package_count = packages.len();
        metrics.resolution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(ResolutionResult {
            packages,
            graph: None,
            metrics,
        })
    }

    fn has_conflict(&self, package: &Package, resolved: &[Package]) -> bool {
        for dep in resolved {
            if dep.name() == package.name() && dep.version() != package.version() {
                return true;
            }
        }
        false
    }

    fn get_metrics(&self) -> crate::resolution::ResolutionMetrics {
        crate::resolution::ResolutionMetrics::default()
    }
}

impl DependencyProvider<String, PubgrubVersion> for PubGrubProvider {
    fn get_dependencies(
        &self,
        package: &String,
        version: &PubgrubVersion,
    ) -> Result<Dependencies<String, PubgrubVersion>, Box<dyn StdError>> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Box::new(BlastError::resolution(e.to_string())) as Box<dyn StdError>)?;
        
        let deps_map = rt.block_on(self.pypi.get_package_dependencies(package, &version.0))
            .map_err(|e| Box::new(BlastError::resolution(e.to_string())) as Box<dyn StdError>)?;

        let mut ranges = FxHashMap::default();
        for (name, _constraint) in deps_map {
            let range = Range::any(); // TODO: Convert VersionConstraint to PubGrub Range
            ranges.insert(name, range);
        }
        Ok(Dependencies::Known(ranges))
    }

    fn choose_package_version<T, U>(
        &self,
        mut available_versions: impl Iterator<Item = (T, U)>,
    ) -> Result<(T, Option<PubgrubVersion>), Box<dyn StdError>>
    where
        T: Borrow<String>,
        U: Borrow<Range<PubgrubVersion>>,
    {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| Box::new(BlastError::resolution(e.to_string())) as Box<dyn StdError>)?;

        if let Some((package, range)) = available_versions.next() {
            let versions = rt.block_on(self.pypi.get_package_versions(package.borrow()))
                .map_err(|e| Box::new(BlastError::resolution(e.to_string())) as Box<dyn StdError>)?;

            let mut best_version = None;
            for version in versions {
                let pubgrub_version = PubgrubVersion(version);
                if range.borrow().contains(&pubgrub_version) {
                    match best_version {
                        None => best_version = Some(pubgrub_version),
                        Some(ref current) if pubgrub_version > *current => {
                            best_version = Some(pubgrub_version)
                        }
                        _ => {}
                    }
                }
            }
            Ok((package, best_version))
        } else {
            Err(Box::new(BlastError::resolution("No versions available".to_string())))
        }
    }
} 