use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::error::BlastResult;
use super::{PackageConfig, PackageOperation, Version};

/// Pip operation types
#[derive(Debug, Clone)]
pub enum PipOperation {
    /// Install packages
    Install {
        packages: Vec<String>,
        upgrade: bool,
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
        outdated: bool,
    },
    /// Show package info
    Show {
        packages: Vec<String>,
    },
    /// Download packages
    Download {
        packages: Vec<String>,
        dest: Option<String>,
    },
    /// Config operations
    Config {
        action: String,
        options: HashMap<String, String>,
    },
}

/// Pip interceptor implementation
pub struct PipInterceptor {
    /// Configuration
    config: PackageConfig,
    /// Operation sender
    operation_tx: mpsc::Sender<PackageOperation>,
}

impl PipInterceptor {
    /// Create new pip interceptor
    pub fn new(config: PackageConfig) -> Self {
        let (tx, _) = mpsc::channel(100);
        Self {
            config,
            operation_tx: tx,
        }
    }

    /// Handle pip command
    pub async fn handle_pip_command(&self, args: Vec<String>) -> BlastResult<()> {
        // Parse pip command
        let operation = self.parse_pip_args(&args)?;
        
        // Convert to package operations
        let package_ops = self.convert_pip_operation(operation)?;
        
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
            PipOperation::Install { packages, upgrade: _, .. } => {
                let mut ops = Vec::new();
                
                for pkg in packages {
                    if let Some((name, version)) = pkg.split_once("==") {
                        // Specific version
                        ops.push(PackageOperation::Install {
                            name: name.to_string(),
                            version: Some(Version {
                                version: version.to_string(),
                                released: chrono::Utc::now(),
                                python_requires: None,
                                dependencies: Vec::new(),
                            }),
                            dependencies: Vec::new(),
                        });
                    } else {
                        // Latest version
                        ops.push(PackageOperation::Install {
                            name: pkg,
                            version: None,
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