use std::collections::{HashMap, HashSet};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use petgraph::visit::Dfs;
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::sync::RwLock;
use super::Dependency;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use chrono::{DateTime, Utc};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum GraphError {
    NotifyError(notify::Error),
    IoError(std::io::Error),
    Other(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::NotifyError(e) => write!(f, "Notify error: {}", e),
            GraphError::IoError(e) => write!(f, "IO error: {}", e),
            GraphError::Other(s) => write!(f, "Graph error: {}", s),
        }
    }
}

impl Error for GraphError {}

impl From<notify::Error> for GraphError {
    fn from(err: notify::Error) -> Self {
        GraphError::NotifyError(err)
    }
}

impl From<std::io::Error> for GraphError {
    fn from(err: std::io::Error) -> Self {
        GraphError::IoError(err)
    }
}

#[derive(Debug, Clone)]
pub enum GraphChange {
    NodeAdded {
        name: String,
        version: String,
    },
    NodeRemoved {
        name: String,
    },
    EdgeAdded {
        from: String,
        to: String,
    },
    EdgeRemoved {
        from: String,
        to: String,
    },
    NodeUpdated {
        name: String,
        old_version: String,
        new_version: String,
    },
}

/// Dependency node information
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Current version (if updating)
    pub current_version: Option<String>,
    /// Package dependencies
    pub dependencies: Vec<Dependency>,
    /// Direct dependency
    pub direct: bool,
    /// Package hash
    pub hash: Option<String>,
    /// Package size
    pub size: u64,
    /// Package source
    pub source: String,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Dependency graph implementation
#[derive(Debug)]
pub struct DependencyGraph {
    /// Graph structure
    graph: DiGraph<DependencyNode, ()>,
    /// Node indices by package name
    nodes: HashMap<String, NodeIndex>,
    /// Change broadcaster
    change_tx: broadcast::Sender<GraphChange>,
    /// File watcher
    watcher: Option<RecommendedWatcher>,
    /// Graph state
    state: Arc<RwLock<GraphState>>,
}

#[derive(Debug)]
struct GraphState {
    /// Pending changes
    pending_changes: Vec<GraphChange>,
    /// Last validation timestamp
    #[allow(dead_code)]
    last_validation: DateTime<Utc>,
    /// Validation status
    #[allow(dead_code)]
    is_valid: bool,
}

impl Default for GraphState {
    fn default() -> Self {
        Self {
            pending_changes: Vec::new(),
            last_validation: Utc::now(),
            is_valid: false,
        }
    }
}

impl DependencyGraph {
    /// Create new dependency graph
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            graph: DiGraph::new(),
            nodes: HashMap::new(),
            change_tx: tx,
            watcher: None,
            state: Arc::new(RwLock::new(GraphState::default())),
        }
    }

    /// Subscribe to graph changes
    pub fn subscribe_changes(&self) -> broadcast::Receiver<GraphChange> {
        self.change_tx.subscribe()
    }

    /// Start watching directory for changes
    pub async fn watch_directory(&mut self, path: &Path) -> Result<(), GraphError> {
        let tx = self.change_tx.clone();
        let state = Arc::clone(&self.state);
        
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let change = match event.kind {
                    notify::EventKind::Create(_) => Some(GraphChange::NodeAdded {
                        name: event.paths[0].to_string_lossy().into_owned(),
                        version: "0.0.0".to_string(), // Placeholder
                    }),
                    notify::EventKind::Remove(_) => Some(GraphChange::NodeRemoved {
                        name: event.paths[0].to_string_lossy().into_owned(),
                    }),
                    notify::EventKind::Modify(_) => None, // Handle in detail
                    _ => None,
                };

                if let Some(change) = change {
                    let _ = tx.send(change.clone());
                    let mut state = futures::executor::block_on(state.write());
                    state.pending_changes.push(change);
                }
            }
        })?;

        watcher.watch(path, RecursiveMode::Recursive)?;
        self.watcher = Some(watcher);
        Ok(())
    }

    /// Add package to graph with change notification
    pub fn add_package(&mut self, name: &str, version: String) -> NodeIndex {
        let idx = if let Some(&idx) = self.nodes.get(name) {
            // Update existing node
            let node = &mut self.graph[idx];
            if node.version != version {
                let change = GraphChange::NodeUpdated {
                    name: name.to_string(),
                    old_version: node.version.clone(),
                    new_version: version.clone(),
                };
                let _ = self.change_tx.send(change);
                node.version = version;
                node.last_updated = Utc::now();
            }
            idx
        } else {
            // Add new node
            let node = DependencyNode {
                name: name.to_string(),
                version: version.clone(),
                current_version: None,
                dependencies: Vec::new(),
                direct: false,
                hash: None,
                size: 0,
                source: String::new(),
                last_updated: Utc::now(),
            };
            
            let idx = self.graph.add_node(node);
            self.nodes.insert(name.to_string(), idx);
            
            let change = GraphChange::NodeAdded {
                name: name.to_string(),
                version,
            };
            let _ = self.change_tx.send(change);
            
            idx
        };
        
        idx
    }

    /// Add dependency between packages with change notification
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        if let (Some(&from_idx), Some(&to_idx)) = (self.nodes.get(from), self.nodes.get(to)) {
            self.graph.add_edge(from_idx, to_idx, ());
            
            let change = GraphChange::EdgeAdded {
                from: from.to_string(),
                to: to.to_string(),
            };
            let _ = self.change_tx.send(change);
        }
    }

    /// Remove package from graph with change notification
    pub fn remove_package(&mut self, name: &str) {
        if let Some(&idx) = self.nodes.get(name) {
            self.graph.remove_node(idx);
            self.nodes.remove(name);
            
            let change = GraphChange::NodeRemoved {
                name: name.to_string(),
            };
            let _ = self.change_tx.send(change);
        }
    }

    /// Get pending changes
    pub async fn get_pending_changes(&self) -> Vec<GraphChange> {
        self.state.read().await.pending_changes.clone()
    }

    /// Clear pending changes
    pub async fn clear_pending_changes(&self) {
        self.state.write().await.pending_changes.clear();
    }

    /// Get package node
    pub fn get_node(&self, name: &str) -> Option<&DependencyNode> {
        self.nodes.get(name).map(|&idx| &self.graph[idx])
    }

    /// Get package node mut
    pub fn get_node_mut(&mut self, name: &str) -> Option<&mut DependencyNode> {
        self.nodes.get(&name.to_string()).map(|&idx| &mut self.graph[idx])
    }

    /// Get all nodes
    pub fn nodes(&self) -> Vec<&DependencyNode> {
        self.graph.node_indices().map(|idx| &self.graph[idx]).collect()
    }

    /// Get direct dependencies
    pub fn direct_dependencies(&self) -> Vec<&DependencyNode> {
        self.graph
            .node_indices()
            .filter(|&idx| self.graph[idx].direct)
            .map(|idx| &self.graph[idx])
            .collect()
    }

    /// Get all dependencies for package
    pub fn get_dependencies(&self, name: &str) -> Vec<&DependencyNode> {
        let mut deps = Vec::new();
        
        if let Some(&start) = self.nodes.get(name) {
            let mut dfs = Dfs::new(&self.graph, start);
            while let Some(idx) = dfs.next(&self.graph) {
                if idx != start {
                    deps.push(&self.graph[idx]);
                }
            }
        }
        
        deps
    }

    /// Get reverse dependencies
    pub fn get_reverse_dependencies(&self, name: &str) -> Vec<&DependencyNode> {
        let mut rdeps = Vec::new();
        
        if let Some(&target) = self.nodes.get(name) {
            for idx in self.graph.node_indices() {
                if petgraph::algo::has_path_connecting(&self.graph, idx, target, None) {
                    rdeps.push(&self.graph[idx]);
                }
            }
        }
        
        rdeps
    }

    /// Check for cycles
    pub fn has_cycles(&self) -> bool {
        toposort(&self.graph, None).is_err()
    }

    /// Get installation order
    pub fn installation_order(&self) -> Vec<&DependencyNode> {
        match toposort(&self.graph, None) {
            Ok(order) => order.iter().map(|&idx| &self.graph[idx]).collect(),
            Err(_) => Vec::new(), // Graph has cycles
        }
    }

    /// Prune unused dependencies
    pub fn prune_unused(&mut self) {
        let mut used = HashSet::new();
        
        // Find all nodes reachable from direct dependencies
        for idx in self.graph.node_indices() {
            if self.graph[idx].direct {
                let mut dfs = Dfs::new(&self.graph, idx);
                while let Some(node) = dfs.next(&self.graph) {
                    used.insert(node);
                }
            }
        }
        
        // Remove unused nodes
        let mut to_remove = Vec::new();
        for idx in self.graph.node_indices() {
            if !used.contains(&idx) {
                to_remove.push(idx);
                self.nodes.remove(&self.graph[idx].name);
            }
        }
        
        for idx in to_remove {
            self.graph.remove_node(idx);
        }
    }

    /// Merge with another graph
    pub fn merge(&mut self, other: &DependencyGraph) {
        // Add nodes from other graph
        for node in other.nodes() {
            self.add_package(&node.name, node.version.clone());
            let target = self.get_node_mut(&node.name).unwrap();
            target.dependencies = node.dependencies.clone();
            target.direct = node.direct;
            target.hash = node.hash.clone();
            target.size = node.size;
            target.source = node.source.clone();
        }
        
        // Add edges from other graph
        for edge in other.graph.edge_indices() {
            let (from, to) = other.graph.edge_endpoints(edge).unwrap();
            let from_name = &other.graph[from].name;
            let to_name = &other.graph[to].name;
            self.add_dependency(from_name, to_name);
        }
    }

    /// Clone subgraph starting from node
    pub fn clone_subgraph(&self, start: &str) -> Option<Self> {
        let mut new_graph = Self::new();
        
        if let Some(&start_idx) = self.nodes.get(start) {
            let mut dfs = Dfs::new(&self.graph, start_idx);
            while let Some(idx) = dfs.next(&self.graph) {
                let node = &self.graph[idx];
                new_graph.add_package(&node.name, node.version.clone());
                let target = new_graph.get_node_mut(&node.name).unwrap();
                target.dependencies = node.dependencies.clone();
                target.direct = node.direct;
                target.hash = node.hash.clone();
                target.size = node.size;
                target.source = node.source.clone();
            }
            
            // Add edges
            for edge in self.graph.edge_indices() {
                let (from, to) = self.graph.edge_endpoints(edge).unwrap();
                let from_name = &self.graph[from].name;
                let to_name = &self.graph[to].name;
                if new_graph.nodes.contains_key(from_name) && new_graph.nodes.contains_key(to_name) {
                    new_graph.add_dependency(from_name, to_name);
                }
            }
            
            Some(new_graph)
        } else {
            None
        }
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
} 