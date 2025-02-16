use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{Mutex, mpsc};
use notify::{Watcher, RecursiveMode, Event};
use python_parser::{ast, Parse};
use tracing::{debug, info, warn};
use crate::{
    error::{BlastError, BlastResult},
    package::{Package, Version},
    python::PythonEnvironment,
    sync::SyncManager,
};

/// Hot reload manager for Python environments
pub struct HotReloadManager {
    /// Active environments
    environments: Arc<Mutex<HashMap<String, EnvironmentContext>>>,
    /// File system watcher
    watcher: Arc<Mutex<notify::RecommendedWatcher>>,
    /// Import cache
    import_cache: Arc<Mutex<HashMap<String, ImportInfo>>>,
    /// Import notification channel
    import_tx: mpsc::Sender<ImportNotification>,
    /// Sync manager for version synchronization
    sync_manager: Arc<Mutex<SyncManager>>,
}

/// Environment context for hot reloading
struct EnvironmentContext {
    /// Environment reference
    environment: PythonEnvironment,
    /// Watched paths
    watched_paths: HashSet<PathBuf>,
    /// Active imports
    active_imports: HashSet<String>,
    /// Package versions
    package_versions: HashMap<String, Version>,
    /// Last sync operation
    last_sync: Option<String>,
}

/// Import information
#[derive(Debug, Clone)]
struct ImportInfo {
    /// Package name
    package: String,
    /// First seen timestamp
    first_seen: chrono::DateTime<chrono::Utc>,
    /// Last used timestamp
    last_used: chrono::DateTime<chrono::Utc>,
    /// Usage count
    usage_count: u64,
    /// Source files
    sources: HashSet<PathBuf>,
}

/// Import notification from Python
#[derive(Debug)]
struct ImportNotification {
    /// Environment name
    environment: String,
    /// Import name
    import_name: String,
    /// Source file
    source: Option<PathBuf>,
    /// Timestamp
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl HotReloadManager {
    /// Create new hot reload manager
    pub async fn new() -> BlastResult<Self> {
        let (import_tx, mut import_rx) = mpsc::channel(1000);
        let environments = Arc::new(Mutex::new(HashMap::new()));
        let import_cache = Arc::new(Mutex::new(HashMap::new()));
        let sync_manager = Arc::new(Mutex::new(SyncManager::new()));

        // Create file system watcher
        let watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                debug!("File system event detected: {:?}", event);
                // Handle file system events
            }
        })?;

        // Spawn import notification handler
        let env_clone = environments.clone();
        let cache_clone = import_cache.clone();
        let sync_clone = sync_manager.clone();
        
        tokio::spawn(async move {
            while let Some(notification) = import_rx.recv().await {
                Self::handle_import_notification(
                    &env_clone,
                    &cache_clone,
                    &sync_clone,
                    notification
                ).await.ok();
            }
        });

        Ok(Self {
            environments,
            watcher: Arc::new(Mutex::new(watcher)),
            import_cache,
            import_tx,
            sync_manager,
        })
    }

    /// Handle import notification
    async fn handle_import_notification(
        environments: &Arc<Mutex<HashMap<String, EnvironmentContext>>>,
        import_cache: &Arc<Mutex<HashMap<String, ImportInfo>>>,
        sync_manager: &Arc<Mutex<SyncManager>>,
        notification: ImportNotification,
    ) -> BlastResult<()> {
        let mut cache = import_cache.lock().await;
        let mut envs = environments.lock().await;

        // Update import cache
        let info = cache.entry(notification.import_name.clone()).or_insert_with(|| ImportInfo {
            package: notification.import_name.clone(),
            first_seen: notification.timestamp,
            last_used: notification.timestamp,
            usage_count: 0,
            sources: HashSet::new(),
        });

        info.last_used = notification.timestamp;
        info.usage_count += 1;
        if let Some(source) = notification.source {
            info.sources.insert(source);
        }

        // Update environment context
        if let Some(context) = envs.get_mut(&notification.environment) {
            context.active_imports.insert(notification.import_name);
            
            // Check if we need to sync versions
            if context.last_sync.is_none() {
                let mut sync_manager = sync_manager.lock().await;
                let operation = sync_manager.plan_sync(
                    &context.environment,
                    &context.environment,
                ).await?;
                
                context.last_sync = Some(operation.id.clone());
                
                // Apply sync in background
                let sync_manager_clone = sync_manager.clone();
                let operation_id = operation.id.clone();
                let mut env_clone = context.environment.clone();
                tokio::spawn(async move {
                    if let Err(e) = sync_manager_clone.apply_sync(&operation_id, &mut env_clone).await {
                        warn!("Failed to apply sync operation: {}", e);
                    }
                });
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
            watched_paths: HashSet::new(),
            active_imports: HashSet::new(),
            package_versions: HashMap::new(),
            last_sync: None,
        };

        // Add to active environments
        environments.insert(env_name.clone(), context);

        // Start watching environment path
        self.watcher.lock().await.watch(
            env.path(),
            RecursiveMode::Recursive,
        )?;

        Ok(())
    }

    /// Analyze Python file for imports
    pub async fn analyze_file(&self, path: &Path) -> BlastResult<HashSet<String>> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut imports = HashSet::new();

        // Parse Python file
        if let Ok(ast) = ast::Suite::parse(&content) {
            for stmt in ast.statements {
                match stmt {
                    ast::Statement::Import(import) => {
                        for name in import.names {
                            imports.insert(name.name);
                        }
                    }
                    ast::Statement::ImportFrom(from_import) => {
                        if let Some(module) = from_import.module {
                            imports.insert(module);
                        }
                        for name in from_import.names {
                            imports.insert(name.name);
                        }
                    }
                    _ => continue,
                }
            }
        }

        Ok(imports)
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
        let mut sync_manager = self.sync_manager.lock().await;
        let operation = sync_manager.plan_sync(
            &context.environment,
            &context.environment,
        ).await?;

        context.last_sync = Some(operation.id.clone());

        // Apply sync in background
        let sync_manager_clone = sync_manager.clone();
        let operation_id = operation.id.clone();
        let mut env_clone = context.environment.clone();
        
        tokio::spawn(async move {
            if let Err(e) = sync_manager_clone.apply_sync(&operation_id, &mut env_clone).await {
                warn!("Failed to apply sync operation: {}", e);
            }
        });

        Ok(())
    }

    /// Generate optimized import hook script
    pub fn generate_import_hook(&self) -> String {
        r#"
import sys
import os
import threading
import json
import socket
from importlib.abc import MetaPathFinder
from typing import Optional, Sequence

class BlastImportHook(MetaPathFinder):
    def __init__(self):
        self.imported = set()
        self.lock = threading.Lock()
        self.socket_path = os.environ.get('BLAST_SOCKET_PATH')
        self._setup_ipc()

    def _setup_ipc(self):
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        if self.socket_path:
            try:
                self.socket.connect(self.socket_path)
            except:
                print("Warning: Could not connect to Blast daemon")

    def find_spec(self, fullname: str, path: Optional[Sequence[str]] = None, target: Optional[object] = None):
        with self.lock:
            if fullname not in self.imported:
                self.imported.add(fullname)
                self._notify_blast(fullname, path[0] if path else None)
        return None

    def _notify_blast(self, import_name: str, source: Optional[str] = None):
        if not hasattr(self, 'socket'):
            return

        try:
            data = {
                'type': 'import',
                'name': import_name,
                'source': source,
                'timestamp': import_time.isoformat() if (import_time := datetime.datetime.now(datetime.timezone.utc)) else None,
                'environment': os.environ.get('BLAST_ENV_NAME', 'unnamed')
            }
            self.socket.sendall(json.dumps(data).encode())
        except:
            # Fail silently - we don't want to break imports if notification fails
            pass

# Install the import hook
sys.meta_path.insert(0, BlastImportHook())
        "#.to_string()
    }

    /// Start hot reload monitoring
    pub async fn start_monitoring(&self) -> BlastResult<()> {
        let environments = self.environments.clone();
        let import_cache = self.import_cache.clone();
        let sync_manager = self.sync_manager.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                let envs = environments.lock().await;
                for (name, context) in envs.iter() {
                    // Check for new Python files
                    if let Ok(entries) = tokio::fs::read_dir(context.environment.path()).await {
                        for entry in entries {
                            if let Ok(entry) = entry {
                                if let Some(ext) = entry.path().extension() {
                                    if ext == "py" {
                                        // Analyze file for imports
                                        if let Ok(imports) = Self::analyze_file(&entry.path()).await {
                                            for import in imports {
                                                // Send import notification
                                                let notification = ImportNotification {
                                                    environment: name.clone(),
                                                    import_name: import,
                                                    source: Some(entry.path()),
                                                    timestamp: chrono::Utc::now(),
                                                };
                                                Self::handle_import_notification(
                                                    &environments,
                                                    &import_cache,
                                                    &sync_manager,
                                                    notification
                                                ).await.ok();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_hot_reload() {
        let manager = HotReloadManager::new().await.unwrap();
        let temp_dir = tempdir().unwrap();
        
        // Create test environment
        let env = PythonEnvironment::new(
            temp_dir.path().to_path_buf(),
            crate::python::PythonVersion::parse("3.8").unwrap(),
        );

        // Register environment
        manager.register_environment(env).await.unwrap();

        // Create test Python file
        let test_file = temp_dir.path().join("test.py");
        tokio::fs::write(&test_file, r#"
import requests
from pandas import DataFrame
        "#).await.unwrap();

        // Analyze imports
        let imports = manager.analyze_file(&test_file).await.unwrap();
        assert!(imports.contains("requests"));
        assert!(imports.contains("pandas"));

        // Generate import hook
        let hook_script = manager.generate_import_hook();
        assert!(hook_script.contains("BlastImportHook"));
    }
} 