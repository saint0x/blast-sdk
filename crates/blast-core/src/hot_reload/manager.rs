use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use notify::{Watcher, RecursiveMode, Event};
use crate::error::{BlastResult, BlastError};
use crate::version::VersionConstraint;
use crate::metadata::PackageMetadata;
use super::{
    HotReloadConfig,
    HotReloadUpdate,
    HotReloadUpdateType,
    HotReloadUpdateStatus,
    ImportStatement,
};

/// Hot reload manager
pub struct HotReloadManager {
    /// Configuration
    config: HotReloadConfig,
    /// File watcher
    watcher: Option<Box<dyn Watcher>>,
    /// Update history
    updates: Arc<RwLock<Vec<HotReloadUpdate>>>,
    /// Watched paths
    watched_paths: Vec<PathBuf>,
}

impl HotReloadManager {
    /// Create new hot reload manager
    pub fn new(config: HotReloadConfig) -> Self {
        Self {
            config,
            watcher: None,
            updates: Arc::new(RwLock::new(Vec::new())),
            watched_paths: Vec::new(),
        }
    }

    /// Start watching for changes
    pub async fn start(&mut self) -> BlastResult<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    let _ = tx.blocking_send(event);
                }
                Err(e) => eprintln!("Watch error: {:?}", e),
            }
        }).map_err(|e| BlastError::environment(format!("Failed to create watcher: {}", e)))?;

        // Watch configured paths
        for path in &self.config.watch_paths {
            watcher.watch(Path::new(path), RecursiveMode::Recursive)
                .map_err(|e| BlastError::environment(format!("Failed to watch path: {}", e)))?;
            self.watched_paths.push(PathBuf::from(path));
        }

        self.watcher = Some(Box::new(watcher));

        // Handle file system events
        let updates = self.updates.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event.kind {
                    notify::EventKind::Modify(_) => {
                        if let Some(path) = event.paths.first() {
                            if let Some(ext) = path.extension() {
                                if ext == "py" {
                                    // Handle Python file changes
                                    if let Ok(content) = tokio::fs::read_to_string(path).await {
                                        let imports = Self::extract_imports(&content);
                                        for import in imports {
                                            let pkg_name = import.get_package_name();
                                            let mut deps = HashMap::new();
                                            deps.insert(pkg_name.clone(), VersionConstraint::any());
                                            
                                            let metadata = PackageMetadata::new(
                                                pkg_name.clone(),
                                                "*".to_string(),
                                                deps,
                                                VersionConstraint::any(),
                                            );
                                            
                                            if let Ok(package) = crate::package::Package::new(
                                                pkg_name,
                                                "*".to_string(),
                                                metadata,
                                                VersionConstraint::any(),
                                            ) {
                                                let update = HotReloadUpdate {
                                                    timestamp: tokio::time::Instant::now(),
                                                    update_type: HotReloadUpdateType::Package(package),
                                                    status: HotReloadUpdateStatus::Pending,
                                                };
                                                updates.write().await.push(update);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Stop watching for changes
    pub fn stop(&mut self) {
        self.watcher = None;
    }

    /// Get pending updates
    pub async fn get_pending_updates(&self) -> Vec<HotReloadUpdate> {
        self.updates.read().await
            .iter()
            .filter(|u| u.status == HotReloadUpdateStatus::Pending)
            .cloned()
            .collect()
    }

    /// Update status for a specific update
    pub async fn update_status(&self, timestamp: tokio::time::Instant, status: HotReloadUpdateStatus) {
        let mut updates = self.updates.write().await;
        if let Some(update) = updates.iter_mut().find(|u| u.timestamp == timestamp) {
            update.status = status;
        }
    }

    /// Extract imports from Python code
    fn extract_imports(content: &str) -> Vec<ImportStatement> {
        let mut imports = Vec::new();
        let mut in_multiline = false;
        let mut multiline_buffer = String::new();

        for line in content.lines() {
            let line = line.trim();

            if in_multiline {
                multiline_buffer.push_str(line);
                if line.contains(')') {
                    in_multiline = false;
                    imports.extend(ImportStatement::parse_from_line(&multiline_buffer));
                    multiline_buffer.clear();
                }
            } else if line.contains('(') && !line.contains(')') {
                in_multiline = true;
                multiline_buffer = line.to_string();
            } else {
                imports.extend(ImportStatement::parse_from_line(line));
            }
        }

        imports
    }
} 