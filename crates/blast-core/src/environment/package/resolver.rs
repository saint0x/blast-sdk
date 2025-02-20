use std::collections::{HashMap, HashSet};
use semver::{Version as SemVersion, VersionReq};
use crate::error::BlastResult;
use super::{PackageConfig, Version, Dependency, DependencyGraph, PackageState, PackageInfo};
use std::path::Path;
use crate::version::VersionConstraint;
use tokio::sync::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VersionConflict {
    pub package: String,
    pub required_versions: Vec<Version>,
    pub requiring_packages: HashMap<String, VersionReq>,
    pub resolution_strategy: ConflictResolutionStrategy,
}

#[derive(Debug, Clone)]
pub enum ConflictResolutionStrategy {
    UseLatest,
    UseMinimum,
    UseMaximumSatisfying,
    ForceVersion(Version),
    Skip,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ResolutionContext {
    visited: HashSet<String>,
    depth: usize,
    max_depth: usize,
    conflicts: Vec<VersionConflict>,
    resolution_strategies: HashMap<String, ConflictResolutionStrategy>,
}

/// Dependency resolver implementation
pub struct DependencyResolver {
    /// Configuration
    config: PackageConfig,
    /// Package cache
    cache: HashMap<String, Vec<Version>>,
    /// Current package state
    state: PackageState,
    /// Resolution context
    context: Arc<RwLock<ResolutionContext>>,
}

impl DependencyResolver {
    /// Create new dependency resolver
    pub fn new(config: PackageConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
            state: PackageState::new(),
            context: Arc::new(RwLock::new(ResolutionContext {
                visited: HashSet::new(),
                depth: 0,
                max_depth: 100, // Configurable
                conflicts: Vec::new(),
                resolution_strategies: HashMap::new(),
            })),
        }
    }

    /// Set conflict resolution strategy
    pub async fn set_resolution_strategy(&self, package: &str, strategy: ConflictResolutionStrategy) {
        let mut context = self.context.write().await;
        context.resolution_strategies.insert(package.to_string(), strategy);
    }

    /// Resolve version conflicts
    #[allow(dead_code)]
    async fn resolve_conflicts(&self, conflicts: &[VersionConflict]) -> BlastResult<HashMap<String, Version>> {
        let mut resolutions = HashMap::new();
        
        for conflict in conflicts {
            let strategy = self.get_resolution_strategy(conflict).await;
            let resolved_version = match strategy {
                ConflictResolutionStrategy::UseLatest => {
                    // Get latest version that satisfies all constraints
                    self.find_latest_satisfying_version(&conflict.package, &conflict.requiring_packages)?
                }
                ConflictResolutionStrategy::UseMinimum => {
                    // Get minimum version that satisfies all constraints
                    self.find_minimum_satisfying_version(&conflict.package, &conflict.requiring_packages)?
                }
                ConflictResolutionStrategy::UseMaximumSatisfying => {
                    // Get maximum version that satisfies most constraints
                    self.find_maximum_satisfying_version(&conflict.package, &conflict.requiring_packages)?
                }
                ConflictResolutionStrategy::ForceVersion(version) => {
                    // Use forced version
                    version
                }
                ConflictResolutionStrategy::Skip => {
                    // Skip resolution
                    continue;
                }
            };
            
            resolutions.insert(conflict.package.clone(), resolved_version);
        }
        
        Ok(resolutions)
    }

    /// Get resolution strategy for conflict
    #[allow(dead_code)]
    async fn get_resolution_strategy(&self, conflict: &VersionConflict) -> ConflictResolutionStrategy {
        let context = self.context.read().await;
        context.resolution_strategies
            .get(&conflict.package)
            .cloned()
            .unwrap_or(ConflictResolutionStrategy::UseMaximumSatisfying)
    }

    /// Find latest version satisfying all constraints
    #[allow(dead_code)]
    fn find_latest_satisfying_version(
        &self,
        package: &str,
        constraints: &HashMap<String, VersionReq>,
    ) -> BlastResult<Version> {
        let versions = self.get_available_versions(package)?;
        
        versions.into_iter()
            .rev() // Latest first
            .find(|version| {
                constraints.values().all(|req| {
                    req.matches(&SemVersion::parse(&version.version).unwrap())
                })
            })
            .ok_or_else(|| {
                crate::error::BlastError::resolution(
                    format!("No version of {} satisfies all constraints", package)
                )
            })
    }

    /// Find minimum version satisfying all constraints
    #[allow(dead_code)]
    fn find_minimum_satisfying_version(
        &self,
        package: &str,
        constraints: &HashMap<String, VersionReq>,
    ) -> BlastResult<Version> {
        let versions = self.get_available_versions(package)?;
        
        versions.into_iter()
            .find(|version| {
                constraints.values().all(|req| {
                    req.matches(&SemVersion::parse(&version.version).unwrap())
                })
            })
            .ok_or_else(|| {
                crate::error::BlastError::resolution(
                    format!("No version of {} satisfies all constraints", package)
                )
            })
    }

    /// Find version satisfying most constraints
    #[allow(dead_code)]
    fn find_maximum_satisfying_version(
        &self,
        package: &str,
        constraints: &HashMap<String, VersionReq>,
    ) -> BlastResult<Version> {
        let versions = self.get_available_versions(package)?;
        
        let mut max_satisfaction = 0;
        let mut best_version = None;
        
        for version in versions {
            let satisfaction_count = constraints.values()
                .filter(|req| {
                    req.matches(&SemVersion::parse(&version.version).unwrap())
                })
                .count();
            
            if satisfaction_count > max_satisfaction {
                max_satisfaction = satisfaction_count;
                best_version = Some(version);
            }
        }
        
        best_version.ok_or_else(|| {
            crate::error::BlastError::resolution(
                format!("No suitable version found for {}", package)
            )
        })
    }

    /// Get available versions for package
    #[allow(dead_code)]
    fn get_available_versions(&self, package: &str) -> BlastResult<Vec<Version>> {
        self.cache.get(package)
            .cloned()
            .ok_or_else(|| {
                crate::error::BlastError::resolution(
                    format!("No versions found for package {}", package)
                )
            })
    }

    /// Get current package state
    pub async fn get_state(&self) -> BlastResult<PackageState> {
        Ok(self.state.clone())
    }

    /// Resolve dependencies for package
    pub async fn resolve_dependencies(
        &self,
        name: &str,
        version: Option<&Version>,
        dependencies: &[Dependency],
    ) -> BlastResult<DependencyGraph> {
        let mut graph = DependencyGraph::new();
        let mut visited = HashSet::new();
        let mut queue = Vec::new();

        // Add root package
        let root_version = version.cloned().unwrap_or_else(|| {
            self.get_latest_version(name)
                .expect("Failed to get latest version")
        });

        // Add root package to graph
        graph.add_package(name, root_version.version.clone());
        let root_node = graph.get_node_mut(name).unwrap();
        root_node.dependencies = dependencies.to_vec();
        root_node.direct = true;

        // Add dependencies to queue
        for dep in dependencies {
            if !dep.optional {
                queue.push((name.to_string(), dep.clone()));
            }
        }

        // Process dependency queue
        while let Some((parent, dep)) = queue.pop() {
            if visited.contains(&dep.name) {
                continue;
            }

            // Get compatible version
            let version_req = VersionReq::parse(&dep.version_constraint)
                .map_err(|e| crate::error::BlastError::resolution(e.to_string()))?;

            let dep_version = self.find_compatible_version(&dep.name, &version_req)?;

            // Add to graph
            graph.add_package(&dep.name, dep_version.version.clone());
            graph.add_dependency(&parent, &dep.name);

            // Add its dependencies to queue
            for sub_dep in &dep_version.dependencies {
                if !sub_dep.optional {
                    queue.push((dep.name.clone(), sub_dep.clone()));
                }
            }

            visited.insert(dep.name.clone());
        }

        Ok(graph)
    }

    /// Resolve version update
    pub async fn resolve_version_update(
        &self,
        name: &str,
        _from_version: &Version,
        to_version: &Version,
    ) -> BlastResult<DependencyGraph> {
        // Get dependencies for new version
        let new_deps = &to_version.dependencies;
        
        // Resolve dependencies
        self.resolve_dependencies(name, Some(to_version), new_deps).await
    }

    /// Check state conflicts
    pub async fn check_state_conflicts(&self, state: &PackageState) -> BlastResult<Vec<String>> {
        let mut conflicts = Vec::new();
        let mut checked = HashSet::new();
        
        // Check each package
        for pkg in state.get_all_packages() {
            let name = &pkg.version.version;
            if checked.contains(name as &str) {
                continue;
            }
            
            // Get package dependencies
            let deps = &pkg.version.dependencies;
            
            // Check each dependency
            for dep in deps {
                if let Some(installed) = state.get_package(&dep.name) {
                    // Parse version constraint
                    let version_req = VersionReq::parse(&dep.version_constraint)
                        .map_err(|e| crate::error::BlastError::resolution(e.to_string()))?;
                    
                    // Check if installed version satisfies constraint
                    let version = SemVersion::parse(&installed.version.version)
                        .map_err(|e| crate::error::BlastError::resolution(e.to_string()))?;
                    
                    if !version_req.matches(&version) {
                        conflicts.push(format!(
                            "Package {} requires {} {} but {} is installed",
                            name, dep.name, dep.version_constraint, installed.version.version
                        ));
                    }
                }
                
                checked.insert(dep.name.clone());
            }
            
            checked.insert(name.clone());
        }
        
        Ok(conflicts)
    }

    /// Get latest compatible version
    fn get_latest_version(&self, name: &str) -> BlastResult<Version> {
        if let Some(versions) = self.cache.get(name) {
            // Find latest version compatible with Python version
            for version in versions.iter().rev() {
                if let Some(ref requires) = version.python_requires {
                    if self.is_python_compatible(requires) {
                        return Ok(version.clone());
                    }
                } else {
                    return Ok(version.clone());
                }
            }
        }
        
        Err(crate::error::BlastError::resolution(format!(
            "No compatible version found for package {}", name
        )))
    }

    /// Find compatible version
    fn find_compatible_version(&self, name: &str, req: &VersionReq) -> BlastResult<Version> {
        if let Some(versions) = self.cache.get(name) {
            // Find latest compatible version
            for version in versions.iter().rev() {
                let semver = SemVersion::parse(&version.version)
                    .map_err(|e| crate::error::BlastError::resolution(e.to_string()))?;

                if req.matches(&semver) {
                    if let Some(ref requires) = version.python_requires {
                        if self.is_python_compatible(requires) {
                            return Ok(version.clone());
                        }
                    } else {
                        return Ok(version.clone());
                    }
                }
            }
        }

        Err(crate::error::BlastError::resolution(format!(
            "No compatible version found for package {} matching {}", name, req
        )))
    }

    /// Check Python version compatibility
    fn is_python_compatible(&self, requires: &str) -> bool {
        // Parse Python version requirement
        if let Ok(req) = VersionReq::parse(requires) {
            // Parse current Python version
            if let Ok(version) = SemVersion::parse(&self.config.python_version) {
                return req.matches(&version);
            }
        }
        
        false
    }

    /// Get compatible versions for a package
    pub async fn get_compatible_versions(
        &self,
        name: &str,
        required: &VersionConstraint,
        installed: &Version,
    ) -> BlastResult<Vec<Version>> {
        let mut compatible = Vec::new();
        
        if let Some(versions) = self.cache.get(name) {
            for version in versions {
                let semver = SemVersion::parse(&version.version)
                    .map_err(|e| crate::error::BlastError::resolution(e.to_string()))?;
                if required.matches(&crate::version::Version::parse(&semver.to_string())?) && version.version != installed.version {
                    compatible.push(version.clone());
                }
            }
        }
        
        Ok(compatible)
    }

    /// Check if version is compatible with package
    pub fn is_version_compatible(&self, package: &str, version: &Version) -> BlastResult<bool> {
        if let Some(versions) = self.cache.get(package) {
            for cached in versions {
                if cached.version == version.version {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Read package metadata from dist-info
    pub async fn read_package_metadata(&self, path: &Path) -> BlastResult<PackageInfo> {
        // Read metadata from METADATA file in dist-info directory
        let metadata_path = path.join("METADATA");
        let content = tokio::fs::read_to_string(metadata_path).await
            .map_err(|e| crate::error::BlastError::package(format!(
                "Failed to read package metadata: {}", e
            )))?;
        
        // Parse metadata (simplified for now)
        let mut _pkg_name = String::new();
        let mut version = String::new();
        let mut requires_python = None;
        let mut dependencies = Vec::new();
        
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("Name: ") {
                _pkg_name = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("Version: ") {
                version = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("Requires-Python: ") {
                requires_python = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("Requires-Dist: ") {
                // Parse dependency specification
                if let Some((dep_name, constraint)) = value.split_once(' ') {
                    dependencies.push(Dependency {
                        name: dep_name.trim().to_string(),
                        version_constraint: constraint.trim().to_string(),
                        optional: false,
                        markers: None,
                    });
                }
            }
        }
        
        let now = chrono::Utc::now();
        Ok(PackageInfo {
            version: Version {
                version: version.clone(),
                released: now,
                python_requires: requires_python,
                dependencies,
            },
            installed_at: now,
            updated_at: now,
            direct: false,
            hash: None,
            size: 0,
            source: String::new(),
        })
    }
} 