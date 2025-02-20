use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};
use crate::error::BlastResult;
use crate::error::BlastError;
use crate::package::Package;
use crate::version::{Version, VersionConstraint};
use crate::python::PythonVersion;
use crate::metadata::PackageMetadata;

/// Package operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageOperation {
    /// Install package
    Install {
        name: String,
        version: Option<Version>,
        dependencies: Vec<String>,
    },
    /// Uninstall package
    Uninstall {
        name: String,
    },
    /// Update package
    Update {
        name: String,
        from_version: Version,
        to_version: Version,
    },
}

/// Package operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Operation success status
    pub success: bool,
    /// Operation error message
    pub error: Option<String>,
    /// Affected packages
    pub affected_packages: Vec<Package>,
    /// Required actions
    pub required_actions: Vec<PackageOperation>,
}

/// Package dependency graph
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Direct dependencies
    direct: HashMap<String, Version>,
    /// Transitive dependencies
    transitive: HashMap<String, HashSet<String>>,
    /// Version constraints
    constraints: HashMap<String, VersionConstraint>,
}

impl DependencyGraph {
    /// Add package to graph
    pub fn add_package(&mut self, package: &Package, is_direct: bool) {
        let name = package.name().to_string();
        let version = package.version().clone();
        
        if is_direct {
            self.direct.insert(name.clone(), version);
        }
        
        // Add dependencies
        let deps = package.metadata().dependencies.keys();
        let mut dep_set = HashSet::new();
        for dep in deps {
            dep_set.insert(dep.to_string());
        }
        self.transitive.insert(name.clone(), dep_set);
        
        // Add version constraints
        if let Some(constraint) = package.metadata().dependencies.get(&name) {
            self.constraints.insert(name, constraint.clone());
        }
    }

    /// Remove package from graph
    pub fn remove_package(&mut self, name: &str) {
        self.direct.remove(name);
        self.transitive.remove(name);
        self.constraints.remove(name);
    }

    /// Get direct dependencies
    pub fn direct_dependencies(&self) -> &HashMap<String, Version> {
        &self.direct
    }

    /// Get all dependencies for package
    pub fn dependencies_for(&self, name: &str) -> Option<&HashSet<String>> {
        self.transitive.get(name)
    }

    /// Get version constraint for package
    pub fn constraint_for(&self, name: &str) -> Option<&VersionConstraint> {
        self.constraints.get(name)
    }
}

/// Package layer state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageLayerState {
    /// Installed packages
    pub installed: HashMap<String, Package>,
    /// Package operations queue
    pub pending_operations: Vec<PackageOperation>,
    /// Last operation timestamp
    pub last_operation: Option<std::time::SystemTime>,
    /// Layer status
    pub status: PackageLayerStatus,
}

/// Package layer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageLayerStatus {
    /// Layer is idle
    Idle,
    /// Layer is processing operations
    Processing,
    /// Layer has pending operations
    Pending,
    /// Layer has conflicts
    Conflicted,
}

impl Default for PackageLayerState {
    fn default() -> Self {
        Self {
            installed: HashMap::new(),
            pending_operations: Vec::new(),
            last_operation: None,
            status: PackageLayerStatus::Idle,
        }
    }
}

/// Package layer implementation
pub struct PackageLayer {
    /// Layer state
    state: Arc<RwLock<PackageLayerState>>,
    /// Dependency graph
    graph: Arc<RwLock<DependencyGraph>>,
    /// Operation sender
    operation_tx: mpsc::Sender<PackageOperation>,
    /// Python version
    python_version: PythonVersion,
    /// Environment path
    env_path: PathBuf,
}

impl PackageLayer {
    /// Create new package layer
    pub async fn new(
        python_version: PythonVersion,
        env_path: PathBuf,
    ) -> BlastResult<Self> {
        let (tx, mut rx) = mpsc::channel(100);
        let state = Arc::new(RwLock::new(PackageLayerState::default()));
        let graph = Arc::new(RwLock::new(DependencyGraph::default()));
        
        let state_clone = Arc::clone(&state);
        let graph_clone = Arc::clone(&graph);
        
        // Spawn operation processor
        tokio::spawn(async move {
            while let Some(op) = rx.recv().await {
                let mut state = state_clone.write().await;
                let mut graph = graph_clone.write().await;
                
                match Self::process_operation(op, &mut state, &mut graph).await {
                    Ok(_) => {
                        state.status = PackageLayerStatus::Idle;
                    }
                    Err(e) => {
                        eprintln!("Operation processing error: {}", e);
                        state.status = PackageLayerStatus::Conflicted;
                    }
                }
            }
        });
        
        Ok(Self {
            state,
            graph,
            operation_tx: tx,
            python_version,
            env_path,
        })
    }

    /// Process package operation
    async fn process_operation(
        op: PackageOperation,
        state: &mut PackageLayerState,
        graph: &mut DependencyGraph,
    ) -> BlastResult<()> {
        match op {
            PackageOperation::Install { name, version, dependencies } => {
                // Create package with version constraint
                let version = version.unwrap_or_else(|| Version::parse("0.1.0").unwrap());
                let version_str = version.to_string();
                
                // Create metadata with dependencies
                let mut deps = HashMap::new();
                for dep in dependencies {
                    deps.insert(dep, VersionConstraint::any());
                }
                
                let metadata = PackageMetadata {
                    name: name.clone(),
                    version: version_str.clone(),
                    description: None,
                    author: None,
                    homepage: None,
                    license: None,
                    keywords: Vec::new(),
                    classifiers: Vec::new(),
                    documentation: None,
                    repository: None,
                    dependencies: deps,
                    extras: HashMap::new(),
                    python_version: VersionConstraint::any(),
                    platform_tags: Vec::new(),
                    yanked: false,
                    yanked_reason: None,
                };
                
                let package = Package::new(
                    name.clone(),
                    version_str,
                    metadata,
                    VersionConstraint::any(),
                )?;
                
                // Add to graph
                graph.add_package(&package, true);
                
                // Add to installed packages
                state.installed.insert(name, package);
                state.last_operation = Some(std::time::SystemTime::now());
            }
            PackageOperation::Uninstall { name } => {
                // Remove from graph
                graph.remove_package(&name);
                
                // Remove from installed packages
                state.installed.remove(&name);
                state.last_operation = Some(std::time::SystemTime::now());
            }
            PackageOperation::Update { name, from_version: _, to_version } => {
                if let Some(old_package) = state.installed.get(&name) {
                    // Create new package with updated version
                    let version_str = to_version.to_string();
                    let mut metadata = old_package.metadata().clone();
                    metadata.version = version_str.clone();
                    
                    let new_package = Package::new(
                        name.clone(),
                        version_str,
                        metadata,
                        VersionConstraint::any(),
                    )?;
                    
                    // Replace old package
                    state.installed.insert(name, new_package);
                    state.last_operation = Some(std::time::SystemTime::now());
                }
            }
        }
        
        Ok(())
    }

    /// Queue package operation
    pub async fn queue_operation(&self, op: PackageOperation) -> BlastResult<()> {
        let mut state = self.state.write().await;
        state.pending_operations.push(op.clone());
        state.status = PackageLayerStatus::Pending;
        
        // Send operation to processor
        if let Err(e) = self.operation_tx.send(op).await {
            return Err(BlastError::environment(format!("Failed to queue operation: {}", e)));
        }
        
        Ok(())
    }

    /// Get layer state
    pub async fn get_state(&self) -> BlastResult<PackageLayerState> {
        Ok(self.state.read().await.clone())
    }

    /// Get dependency graph
    pub async fn get_graph(&self) -> BlastResult<DependencyGraph> {
        Ok(self.graph.read().await.clone())
    }

    /// Check for conflicts
    pub async fn check_conflicts(&self) -> BlastResult<Vec<String>> {
        let state = self.state.read().await;
        let graph = self.graph.read().await;
        let mut conflicts = Vec::new();
        
        // Check version constraints
        for (name, package) in &state.installed {
            if let Some(constraint) = graph.constraint_for(name) {
                if !constraint.matches(package.version()) {
                    conflicts.push(format!(
                        "Package {} version {} violates constraint {}",
                        name,
                        package.version(),
                        constraint
                    ));
                }
            }
        }
        
        Ok(conflicts)
    }

    /// Get installed packages
    pub async fn get_installed(&self) -> BlastResult<HashMap<String, Package>> {
        Ok(self.state.read().await.installed.clone())
    }

    /// Get pending operations
    pub async fn get_pending(&self) -> BlastResult<Vec<PackageOperation>> {
        Ok(self.state.read().await.pending_operations.clone())
    }

    /// Get layer status
    pub async fn get_status(&self) -> BlastResult<PackageLayerStatus> {
        Ok(self.state.read().await.status)
    }

    /// Intercept pip operation
    pub async fn intercept_pip(&self, args: Vec<String>) -> BlastResult<()> {
        // Parse pip arguments to determine operation type
        if args.is_empty() {
            return Ok(());
        }

        match args[0].as_str() {
            "install" => {
                // Extract package name and version from install args
                let mut name = None;
                let mut version = None;

                for arg in args.iter().skip(1) {
                    if arg.starts_with("-") {
                        continue;
                    }
                    if let Some((pkg, ver)) = arg.split_once("==") {
                        name = Some(pkg.to_string());
                        version = Some(ver.to_string());
                        break;
                    } else {
                        name = Some(arg.to_string());
                        break;
                    }
                }

                if let Some(name) = name {
                    // Queue install operation
                    self.queue_operation(PackageOperation::Install {
                        name,
                        version: version.map(|v| Version::parse(&v).unwrap()),
                        dependencies: Vec::new(),
                    }).await?;
                }
            }
            "uninstall" | "remove" => {
                // Extract package name from uninstall args
                if let Some(name) = args.get(1) {
                    if !name.starts_with("-") {
                        // Queue uninstall operation
                        self.queue_operation(PackageOperation::Uninstall {
                            name: name.to_string(),
                        }).await?;
                    }
                }
            }
            _ => {
                // Other pip operations are passed through
                return Ok(());
            }
        }

        Ok(())
    }

    /// Get Python version
    pub fn python_version(&self) -> &PythonVersion {
        &self.python_version
    }

    /// Get environment path
    pub fn env_path(&self) -> &PathBuf {
        &self.env_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_package_layer() {
        let temp_dir = TempDir::new().unwrap();
        let python_version = PythonVersion::new(3, 9, Some(0));
        
        let layer = PackageLayer::new(
            python_version,
            temp_dir.path().to_path_buf(),
        ).await.unwrap();
        
        // Test installing package
        layer.queue_operation(PackageOperation::Install {
            name: "test-package".to_string(),
            version: Some(Version::parse("1.0.0").unwrap()),
            dependencies: vec!["dep1".to_string(), "dep2".to_string()],
        }).await.unwrap();
        
        // Wait for processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Check state
        let state = layer.get_state().await.unwrap();
        assert_eq!(state.status, PackageLayerStatus::Idle);
        assert!(state.installed.contains_key("test-package"));
        
        // Check graph
        let graph = layer.get_graph().await.unwrap();
        assert!(graph.direct_dependencies().contains_key("test-package"));
        assert_eq!(
            graph.dependencies_for("test-package").unwrap().len(),
            2
        );
    }
} 