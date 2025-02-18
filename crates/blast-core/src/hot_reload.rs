use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};
use tokio::sync::Mutex;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use tracing::{debug, warn};
use regex::Regex;
use crate::{
    error::{BlastError, BlastResult},
    version::Version,
    python::PythonEnvironment,
    sync::SyncManager,
    package::Package,
};

/// Custom error type for hot reload operations
#[derive(Debug, thiserror::Error)]
pub enum HotReloadError {
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Python parsing error: {0}")]
    PythonParse(String),
}

impl From<HotReloadError> for BlastError {
    fn from(err: HotReloadError) -> Self {
        BlastError::environment(err.to_string())
    }
}

/// Represents a Python import statement
#[derive(Debug, Clone, PartialEq, Eq)]
struct ImportStatement {
    /// The module being imported
    module: String,
    /// Specific names being imported (for 'from' imports)
    names: Vec<String>,
    /// Whether this is a 'from' import
    is_from: bool,
    /// The full module path for 'from' imports
    from_path: Option<String>,
}

impl ImportStatement {
    fn new(module: String) -> Self {
        Self {
            module,
            names: Vec::new(),
            is_from: false,
            from_path: None,
        }
    }

    fn with_names(module: String, names: Vec<String>, from_path: String) -> Self {
        Self {
            module,
            names,
            is_from: true,
            from_path: Some(from_path),
        }
    }

    /// Get the root package name that needs to be installed
    fn get_package_name(&self) -> String {
        if self.is_from {
            // For 'from' imports, use the first part of the path
            self.from_path.as_ref()
                .and_then(|p| p.split('.').next())
                .unwrap_or(&self.module)
                .to_string()
        } else {
            // For regular imports, use the first part of the module name
            self.module.split('.').next()
                .unwrap_or(&self.module)
                .to_string()
        }
    }
}

/// Hot reload manager for Python environments
pub struct HotReloadManager {
    /// Active environments
    environments: Arc<Mutex<HashMap<String, EnvironmentContext>>>,
    /// File system watcher
    watcher: Arc<Mutex<notify::RecommendedWatcher>>,
    /// Sync manager for version synchronization
    sync_manager: Arc<Mutex<SyncManager>>,
}

/// Environment context for hot reloading
#[derive(Clone)]
struct EnvironmentContext {
    /// Environment reference
    environment: PythonEnvironment,
    /// Package versions
    package_versions: HashMap<String, Version>,
    /// Last sync operation
    last_sync: Option<String>,
}

impl HotReloadManager {
    /// Parse Python imports from a line of code
    fn parse_imports_from_line(line: &str) -> Vec<ImportStatement> {
        let mut imports = Vec::new();
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with('#') || line.is_empty() {
            return imports;
        }

        // Handle multiline imports with parentheses
        if line.contains('(') && !line.contains(')') {
            // This is a multiline import - it should be handled by the caller
            return imports;
        }

        // Match 'from ... import ...' statements
        if line.starts_with("from ") {
            let from_re = Regex::new(r"^from\s+([.\w]+)\s+import\s+(.+)$").unwrap();
            if let Some(caps) = from_re.captures(line) {
                let from_path = caps.get(1).unwrap().as_str().to_string();
                let imports_str = caps.get(2).unwrap().as_str();

                // Handle multiple imports
                let names: Vec<String> = imports_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !names.is_empty() {
                    imports.push(ImportStatement::with_names(
                        names[0].clone(),
                        names,
                        from_path,
                    ));
                }
            }
        }
        // Match 'import ...' statements
        else if line.starts_with("import ") {
            let import_re = Regex::new(r"^import\s+(.+)$").unwrap();
            if let Some(caps) = import_re.captures(line) {
                let modules = caps.get(1).unwrap().as_str();
                
                // Handle multiple imports and aliases
                for module in modules.split(',') {
                    let module = module.trim();
                    if module.is_empty() {
                        continue;
                    }

                    // Handle 'as' aliases
                    let module_name = module.split_whitespace()
                        .next()
                        .unwrap_or(module)
                        .to_string();

                    imports.push(ImportStatement::new(module_name));
                }
            }
        }

        imports
    }

    /// Analyze Python file for imports
    pub async fn analyze_file(&self, path: &Path) -> BlastResult<HashSet<String>> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(HotReloadError::from)?;

        let mut imports = HashSet::new();
        let mut in_multiline = false;
        let mut multiline_buffer = String::new();

        // Process the file line by line
        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            // Handle multiline imports
            if in_multiline {
                multiline_buffer.push_str(line);
                if line.contains(')') {
                    // End of multiline import
                    in_multiline = false;
                    for import in Self::parse_imports_from_line(line) {
                        imports.insert(import.get_package_name());
                    }
                    multiline_buffer.clear();
                }
                continue;
            }

            if line.contains('(') && !line.contains(')') {
                // Start of multiline import
                in_multiline = true;
                multiline_buffer = line.to_string();
                continue;
            }

            // Process single-line imports
            for import in Self::parse_imports_from_line(line) {
                imports.insert(import.get_package_name());
            }
        }

        Ok(imports)
    }

    /// Create new hot reload manager
    pub async fn new() -> BlastResult<Self> {
        let environments = Arc::new(Mutex::new(HashMap::new()));
        let sync_manager = Arc::new(Mutex::new(SyncManager::new()));

        // Create file system watcher with environment reference
        let env_ref = Arc::clone(&environments);
        let sync_ref = Arc::clone(&sync_manager);
        let watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                debug!("File system event detected: {:?}", event);
                if let EventKind::Create(_) | EventKind::Modify(_) = event.kind {
                    if let Some(path) = event.paths.first() {
                        if path.extension().map_or(false, |ext| ext == "py") {
                            let env_clone = env_ref.clone();
                            let sync_clone = sync_ref.clone();
                            let path_clone = path.to_path_buf();
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_python_file_change(&env_clone, &sync_clone, &path_clone).await {
                                    warn!("Failed to handle Python file change: {}", e);
                                }
                            });
                        }
                    }
                }
            }
        }).map_err(HotReloadError::from)?;

        Ok(Self {
            environments,
            watcher: Arc::new(Mutex::new(watcher)),
            sync_manager,
        })
    }

    /// Handle Python file changes
    async fn handle_python_file_change(
        environments: &Arc<Mutex<HashMap<String, EnvironmentContext>>>,
        sync_manager: &Arc<Mutex<SyncManager>>,
        path: &Path,
    ) -> BlastResult<()> {
        let mut envs = environments.lock().await;
        
        // Find environment containing this file
        for context in envs.values_mut() {
            if path.starts_with(context.environment.path()) {
                // Check if we need to sync versions
                if context.last_sync.is_none() {
                    let sync_manager = Arc::clone(sync_manager);
                    let operation = sync_manager.lock().await.plan_sync(
                        &context.environment,
                        &context.environment,
                    ).await?;
                    
                    context.last_sync = Some(operation.id.clone());
                    
                    // Apply sync in background
                    let operation_id = operation.id.clone();
                    let mut env_clone = context.environment.clone();
                    tokio::spawn(async move {
                        if let Err(e) = sync_manager.lock().await.apply_sync(&operation_id, &mut env_clone).await {
                            warn!("Failed to apply sync operation: {}", e);
                        }
                    });
                }
                break;
            }
        }

        Ok(())
    }

    /// Register environment for hot reloading
    pub async fn register_environment(&self, env: PythonEnvironment) -> BlastResult<()> {
        let mut environments = self.environments.lock().await;
        let env_name = env.name().unwrap_or("unnamed").to_string();
        
        // Create context
        let context = EnvironmentContext {
            environment: env.clone(),
            package_versions: HashMap::new(),
            last_sync: None,
        };

        // Add to active environments
        environments.insert(env_name.clone(), context);

        // Start watching environment path
        self.watcher.lock().await.watch(
            env.path(),
            RecursiveMode::Recursive,
        ).map_err(HotReloadError::from)?;

        Ok(())
    }

    /// Handle package installation detection
    pub async fn handle_package_installed(
        &self,
        env_name: &str,
        package: Package,
    ) -> BlastResult<()> {
        let mut environments = self.environments.lock().await;
        let context = environments.get_mut(env_name)
            .ok_or_else(|| BlastError::environment("Environment not found"))?;

        // Update package version
        context.package_versions.insert(package.name().to_string(), package.version().clone());

        // Trigger version sync
        let sync_manager = Arc::clone(&self.sync_manager);
        let operation = sync_manager.lock().await.plan_sync(
            &context.environment,
            &context.environment,
        ).await?;

        context.last_sync = Some(operation.id.clone());

        // Apply sync in background
        let operation_id = operation.id.clone();
        let mut env_clone = context.environment.clone();
        tokio::spawn(async move {
            if let Err(e) = sync_manager.lock().await.apply_sync(&operation_id, &mut env_clone).await {
                warn!("Failed to apply sync operation: {}", e);
            }
        });

        Ok(())
    }

    /// Start monitoring for changes
    pub async fn start_monitoring(&self) -> BlastResult<()> {
        // Implementation is handled by the file system watcher
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_import() {
        let imports = HotReloadManager::parse_imports_from_line("import numpy");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].module, "numpy");
        assert!(!imports[0].is_from);
    }

    #[test]
    fn test_parse_multiple_imports() {
        let imports = HotReloadManager::parse_imports_from_line("import numpy, pandas, tensorflow");
        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].module, "numpy");
        assert_eq!(imports[1].module, "pandas");
        assert_eq!(imports[2].module, "tensorflow");
    }

    #[test]
    fn test_parse_from_import() {
        let imports = HotReloadManager::parse_imports_from_line("from numpy import array, zeros");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].module, "array");
        assert!(imports[0].is_from);
        assert_eq!(imports[0].names, vec!["array", "zeros"]);
        assert_eq!(imports[0].from_path, Some("numpy".to_string()));
    }

    #[test]
    fn test_parse_import_with_alias() {
        let imports = HotReloadManager::parse_imports_from_line("import numpy as np");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].module, "numpy");
    }

    #[test]
    fn test_parse_nested_import() {
        let imports = HotReloadManager::parse_imports_from_line("from tensorflow.keras import layers");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].get_package_name(), "tensorflow");
    }
} 