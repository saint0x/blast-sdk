use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock, broadcast};
use std::sync::Arc;
use crate::error::BlastResult;
use super::{PackageConfig, PackageOperation, Version, DependencyResolver};
use super::{ProgressTracker, InstallationProgress, InstallationStep};
use crate::version::VersionConstraint;
use tokio::fs;
use std::time::Instant;

/// Pip operation types
#[derive(Debug, Clone)]
pub enum PipOperation {
    /// Install packages
    Install {
        packages: Vec<String>,
        #[allow(dead_code)]
        upgrade: bool,
        #[allow(dead_code)]
        editable: bool,
        requirements: Option<String>,
        constraints: Option<String>,
    },
    /// Uninstall packages
    Uninstall {
        packages: Vec<String>,
    },
    /// List packages
    List {
        #[allow(dead_code)]
        outdated: bool,
    },
    /// Show package info
    Show {
        #[allow(dead_code)]
        packages: Vec<String>,
    },
    /// Download packages
    Download {
        #[allow(dead_code)]
        packages: Vec<String>,
        #[allow(dead_code)]
        dest: Option<String>,
    },
    /// Config operations
    Config {
        #[allow(dead_code)]
        action: String,
        #[allow(dead_code)]
        options: HashMap<String, String>,
    },
}

#[derive(Debug, Clone)]
pub struct OperationState {
    pub operation_id: String,
    pub operation_type: String,
    pub start_time: Instant,
    pub status: OperationStatus,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

/// Pip interceptor implementation
pub struct PipInterceptor {
    /// Configuration
    config: PackageConfig,
    /// Operation sender
    operation_tx: mpsc::Sender<PackageOperation>,
    /// Operation receiver
    operation_rx: Arc<RwLock<mpsc::Receiver<PackageOperation>>>,
    /// Dependency resolver
    resolver: Arc<DependencyResolver>,
    /// Operation state broadcaster
    state_tx: broadcast::Sender<OperationState>,
    /// Active operations
    active_operations: Arc<RwLock<HashMap<String, OperationState>>>,
    /// Progress tracker
    progress_tracker: Arc<ProgressTracker>,
}

impl PipInterceptor {
    /// Create new pip interceptor
    pub fn new(config: PackageConfig) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let (state_tx, _) = broadcast::channel(100);
        let resolver = Arc::new(DependencyResolver::new(config.clone()));
        let progress_tracker = Arc::new(ProgressTracker::new());
        
        let interceptor = Self {
            config,
            operation_tx: tx,
            operation_rx: Arc::new(RwLock::new(rx)),
            resolver,
            state_tx,
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            progress_tracker,
        };

        // Spawn background task to process operations
        interceptor.spawn_operation_processor();
        
        interceptor
    }

    /// Subscribe to operation state updates
    pub fn subscribe_state_updates(&self) -> broadcast::Receiver<OperationState> {
        self.state_tx.subscribe()
    }

    /// Spawn background operation processor
    fn spawn_operation_processor(&self) {
        let rx = Arc::clone(&self.operation_rx);
        let resolver = Arc::clone(&self.resolver);
        let config = self.config.clone();
        let state_tx = self.state_tx.clone();
        let active_ops = Arc::clone(&self.active_operations);
        let progress_tracker = Arc::clone(&self.progress_tracker);

        tokio::spawn(async move {
            let mut rx = rx.write().await;
            while let Some(op) = rx.recv().await {
                let op_id = uuid::Uuid::new_v4().to_string();
                let op_type = format!("{:?}", op);
                let op_state = OperationState {
                    operation_id: op_id.clone(),
                    operation_type: op_type.clone(),
                    start_time: Instant::now(),
                    status: OperationStatus::Pending,
                    details: HashMap::new(),
                };

                // Update state
                active_ops.write().await.insert(op_id.clone(), op_state.clone());
                let _ = state_tx.send(op_state);

                // Process operation
                let result = Self::process_operation(op, &resolver, &progress_tracker, &config).await;
                
                // Update final state
                let final_state = match result {
                    Ok(()) => OperationState {
                        operation_id: op_id.clone(),
                        operation_type: op_type.clone(),
                        start_time: Instant::now(),
                        status: OperationStatus::Completed,
                        details: HashMap::new(),
                    },
                    Err(e) => OperationState {
                        operation_id: op_id.clone(),
                        operation_type: op_type,
                        start_time: Instant::now(),
                        status: OperationStatus::Failed(e.to_string()),
                        details: HashMap::new(),
                    },
                };

                active_ops.write().await.insert(op_id.clone(), final_state.clone());
                let _ = state_tx.send(final_state);
            }
        });
    }

    /// Process a package operation
    async fn process_operation(
        op: PackageOperation,
        resolver: &DependencyResolver,
        progress_tracker: &ProgressTracker,
        _config: &PackageConfig,
    ) -> BlastResult<()> {
        match op {
            PackageOperation::Install { name, version, dependencies } => {
                // Start tracking progress
                let op_id = progress_tracker.start_operation(name.clone()).await;
                
                // Update progress for dependency resolution
                progress_tracker.update_operation(
                    &op_id,
                    InstallationStep::ResolvingDependencies,
                    0.2,
                    "Resolving dependencies",
                ).await;

                // Resolve dependencies recursively with real-time updates
                let deps = match resolver.resolve_dependencies(&name, version.as_ref(), &dependencies).await {
                    Ok(deps) => {
                        progress_tracker.update_operation(
                            &op_id,
                            InstallationStep::ValidatingGraph,
                            0.4,
                            "Validating dependency graph",
                        ).await;
                        deps
                    }
                    Err(e) => {
                        progress_tracker.fail_operation(
                            &op_id,
                            format!("Failed to resolve dependencies: {}", e),
                        ).await;
                        return Err(e);
                    }
                };
                
                // Install all dependencies in correct order with progress tracking
                let total_deps = deps.installation_order().len();
                for (idx, dep_node) in deps.installation_order().iter().enumerate() {
                    let progress = 0.4 + (0.6 * (idx as f32 / total_deps as f32));
                    
                    progress_tracker.update_operation(
                        &op_id,
                        InstallationStep::Installing,
                        progress,
                        format!("Installing dependency: {}", dep_node.name),
                    ).await;
                    
                    tracing::info!("Installing dependency: {}", dep_node.name);
                    // TODO: Implement actual package installation
                }
                
                progress_tracker.complete_operation(&op_id).await;
            }
            PackageOperation::Uninstall { name } => {
                let op_id = progress_tracker.start_operation(name.clone()).await;
                
                // Check for reverse dependencies before uninstalling
                progress_tracker.update_operation(
                    &op_id,
                    InstallationStep::ValidatingGraph,
                    0.3,
                    "Checking dependencies",
                ).await;

                let state = match resolver.get_state().await {
                    Ok(state) => state,
                    Err(e) => {
                        progress_tracker.fail_operation(
                            &op_id,
                            format!("Failed to get package state: {}", e),
                        ).await;
                        return Err(e);
                    }
                };

                if !state.can_remove(&name) {
                    let error = format!("Cannot remove {}: other packages depend on it", name);
                    progress_tracker.fail_operation(&op_id, &error).await;
                    return Err(crate::error::BlastError::package(error));
                }

                progress_tracker.update_operation(
                    &op_id,
                    InstallationStep::Installing,
                    0.6,
                    "Uninstalling package",
                ).await;

                // TODO: Implement actual package uninstallation
                
                progress_tracker.complete_operation(&op_id).await;
            }
            _ => {}
        }
        Ok(())
    }

    /// Get operation status
    pub async fn get_operation_status(&self, operation_id: &str) -> Option<OperationState> {
        self.active_operations.read().await.get(operation_id).cloned()
    }

    /// Get all active operations
    pub async fn get_active_operations(&self) -> Vec<OperationState> {
        self.active_operations.read().await.values().cloned().collect()
    }

    /// Parse requirement file
    async fn parse_requirements(&self, path: &str) -> BlastResult<Vec<(String, VersionConstraint)>> {
        let content = fs::read_to_string(path).await?;
        let mut requirements = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse package specs like:
            // package==1.0.0
            // package>=1.0.0,<2.0.0
            // package~=1.0.0
            if let Some((name, version_spec)) = self.parse_package_spec(line)? {
                requirements.push((name, version_spec));
            }
        }
        
        Ok(requirements)
    }

    /// Parse package specification
    fn parse_package_spec(&self, spec: &str) -> BlastResult<Option<(String, VersionConstraint)>> {
        // Split on first occurrence of any version operator
        let operators = ["==", ">=", "<=", "!=", "~=", ">", "<"];
        
        for op in &operators {
            if let Some((name, version)) = spec.split_once(op) {
                let name = name.trim().to_string();
                let constraint = format!("{}{}", op, version.trim());
                return Ok(Some((name, VersionConstraint::parse(&constraint)?)));
            }
        }
        
        // No version specified, return just the package name
        if !spec.contains(char::is_whitespace) {
            return Ok(Some((spec.to_string(), VersionConstraint::any())));
        }
        
        Ok(None)
    }

    /// Handle pip command
    pub async fn handle_pip_command(&self, args: Vec<String>) -> BlastResult<()> {
        // Parse pip command
        let operation = self.parse_pip_args(&args)?;
        
        // Handle requirements files first
        let mut package_ops = Vec::new();
        
        match &operation {
            PipOperation::Install { requirements: Some(req_file), constraints, .. } => {
                // Parse requirements file
                let requirements = self.parse_requirements(req_file).await?;
                
                // Parse constraints if present
                let constraints = if let Some(constraint_file) = constraints {
                    self.parse_requirements(constraint_file).await?
                } else {
                    Vec::new()
                };
                
                // Create install operations for each requirement
                for (name, version_constraint) in requirements {
                    // Check if there's a matching constraint
                    let final_constraint = constraints.iter()
                        .find(|(n, _)| n == &name)
                        .map(|(_, c)| c)
                        .unwrap_or(&version_constraint);
                    
                    package_ops.push(PackageOperation::Install {
                        name,
                        version: Some(Version {
                            version: final_constraint.to_string(),
                            released: chrono::Utc::now(),
                            python_requires: None,
                            dependencies: Vec::new(), // Dependencies will be resolved later
                        }),
                        dependencies: Vec::new(),
                    });
                }
            }
            _ => {
                // Convert to package operations normally
                package_ops = self.convert_pip_operation(operation)?;
            }
        }
        
        // Queue package operations
        for op in package_ops {
            self.operation_tx.send(op).await.map_err(|e| {
                crate::error::BlastError::package(format!(
                    "Failed to queue package operation: {}", e
                ))
            })?;
        }
        
        Ok(())
    }

    /// Parse pip command arguments
    fn parse_pip_args(&self, args: &[String]) -> BlastResult<PipOperation> {
        if args.is_empty() {
            return Err(crate::error::BlastError::package(
                "No pip arguments provided"
            ));
        }

        match args[0].as_str() {
            "install" => {
                let mut packages = Vec::new();
                let mut upgrade = false;
                let mut editable = false;
                let mut requirements = None;
                let mut constraints = None;
                
                let mut i = 1;
                while i < args.len() {
                    match args[i].as_str() {
                        "-U" | "--upgrade" => upgrade = true,
                        "-e" | "--editable" => editable = true,
                        "-r" | "--requirement" => {
                            i += 1;
                            if i < args.len() {
                                requirements = Some(args[i].clone());
                            }
                        }
                        "-c" | "--constraint" => {
                            i += 1;
                            if i < args.len() {
                                constraints = Some(args[i].clone());
                            }
                        }
                        arg if arg.starts_with('-') => {
                            // Skip other options
                            if arg.contains('=') {
                                // Option with value
                                continue;
                            }
                            i += 1;
                            continue;
                        }
                        pkg => packages.push(pkg.to_string()),
                    }
                    i += 1;
                }
                
                Ok(PipOperation::Install {
                    packages,
                    upgrade,
                    editable,
                    requirements,
                    constraints,
                })
            }
            "uninstall" => {
                let mut packages = Vec::new();
                
                for arg in &args[1..] {
                    if !arg.starts_with('-') {
                        packages.push(arg.to_string());
                    }
                }
                
                Ok(PipOperation::Uninstall { packages })
            }
            "list" => {
                let outdated = args.contains(&"--outdated".to_string());
                Ok(PipOperation::List { outdated })
            }
            "show" => {
                let mut packages = Vec::new();
                
                for arg in &args[1..] {
                    if !arg.starts_with('-') {
                        packages.push(arg.to_string());
                    }
                }
                
                Ok(PipOperation::Show { packages })
            }
            "download" => {
                let mut packages = Vec::new();
                let mut dest = None;
                
                let mut i = 1;
                while i < args.len() {
                    match args[i].as_str() {
                        "-d" | "--dest" => {
                            i += 1;
                            if i < args.len() {
                                dest = Some(args[i].clone());
                            }
                        }
                        arg if arg.starts_with('-') => {
                            // Skip other options
                            if arg.contains('=') {
                                // Option with value
                                continue;
                            }
                            i += 1;
                            continue;
                        }
                        pkg => packages.push(pkg.to_string()),
                    }
                    i += 1;
                }
                
                Ok(PipOperation::Download { packages, dest })
            }
            "config" => {
                let mut action = String::new();
                let mut options = HashMap::new();
                
                if args.len() > 1 {
                    action = args[1].clone();
                }
                
                for arg in &args[2..] {
                    if let Some((key, value)) = arg.split_once('=') {
                        options.insert(key.to_string(), value.to_string());
                    }
                }
                
                Ok(PipOperation::Config { action, options })
            }
            cmd => Err(crate::error::BlastError::package(format!(
                "Unsupported pip command: {}", cmd
            ))),
        }
    }

    /// Convert pip operation to package operations
    fn convert_pip_operation(&self, operation: PipOperation) -> BlastResult<Vec<PackageOperation>> {
        match operation {
            PipOperation::Install { packages, .. } => {
                let mut ops = Vec::new();
                
                for pkg in packages {
                    // Parse package spec for version constraints
                    if let Some((name, version_constraint)) = self.parse_package_spec(&pkg)? {
                        ops.push(PackageOperation::Install {
                            name,
                            version: Some(Version {
                                version: version_constraint.to_string(),
                                released: chrono::Utc::now(),
                                python_requires: None,
                                dependencies: Vec::new(), // Dependencies will be resolved later
                            }),
                            dependencies: Vec::new(),
                        });
                    }
                }
                
                Ok(ops)
            }
            PipOperation::Uninstall { packages } => {
                Ok(packages
                    .into_iter()
                    .map(|name| PackageOperation::Uninstall { name })
                    .collect())
            }
            PipOperation::List { .. } | PipOperation::Show { .. } | 
            PipOperation::Download { .. } | PipOperation::Config { .. } => {
                // These operations don't modify packages
                Ok(Vec::new())
            }
        }
    }

    /// Subscribe to progress updates
    pub fn subscribe_progress(&self) -> broadcast::Receiver<InstallationProgress> {
        self.progress_tracker.subscribe()
    }

    /// Get progress for operation
    pub async fn get_operation_progress(&self, operation_id: &str) -> Option<InstallationProgress> {
        self.progress_tracker.get_progress(operation_id).await
    }
} 