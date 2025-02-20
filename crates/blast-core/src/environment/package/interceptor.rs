use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use crate::error::BlastResult;
use super::{PackageConfig, PackageOperation, Version, DependencyResolver};
use crate::version::VersionConstraint;
use tokio::fs;

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
}

impl PipInterceptor {
    /// Create new pip interceptor
    pub fn new(config: PackageConfig) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let resolver = Arc::new(DependencyResolver::new(config.clone()));
        
        let interceptor = Self {
            config,
            operation_tx: tx,
            operation_rx: Arc::new(RwLock::new(rx)),
            resolver,
        };

        // Spawn background task to process operations
        interceptor.spawn_operation_processor();
        
        interceptor
    }

    /// Spawn background operation processor
    fn spawn_operation_processor(&self) {
        let rx = Arc::clone(&self.operation_rx);
        let resolver = Arc::clone(&self.resolver);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut rx = rx.write().await;
            while let Some(op) = rx.recv().await {
                if let Err(e) = Self::process_operation(op, &resolver, &config).await {
                    tracing::error!("Failed to process package operation: {}", e);
                }
            }
        });
    }

    /// Process a package operation
    async fn process_operation(
        op: PackageOperation,
        resolver: &DependencyResolver,
        _config: &PackageConfig,
    ) -> BlastResult<()> {
        match op {
            PackageOperation::Install { name, version, .. } => {
                // Resolve dependencies recursively
                let deps = resolver.resolve_dependencies(&name, version.as_ref(), &[]).await?;
                
                // Install all dependencies in correct order
                for dep_node in deps.installation_order() {
                    tracing::info!("Installing dependency: {}", dep_node.name);
                    // TODO: Actual package installation
                }
            }
            PackageOperation::Uninstall { name } => {
                // Check for reverse dependencies before uninstalling
                let state = resolver.get_state().await?;
                if !state.can_remove(&name) {
                    return Err(crate::error::BlastError::package(
                        format!("Cannot remove {}: other packages depend on it", name)
                    ));
                }
                // TODO: Actual package uninstallation
            }
            _ => {}
        }
        Ok(())
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
} 