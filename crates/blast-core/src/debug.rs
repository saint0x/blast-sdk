use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::dot::{Dot, Config};

/// Debug information collector
#[derive(Debug)]
pub struct DebugCollector {
    /// System information
    system_info: SystemInfo,
    /// Operation history
    history: Vec<OperationRecord>,
    /// Environment state
    environment_state: Option<EnvironmentState>,
    /// Dependency graph
    dependency_graph: Option<DependencyGraph>,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system
    pub os: String,
    /// CPU architecture
    pub arch: String,
    /// Python version
    pub python_version: String,
    /// Available memory
    pub memory: u64,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Blast version
    pub blast_version: String,
}

/// Operation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    /// Operation ID
    pub id: uuid::Uuid,
    /// Operation type
    pub operation_type: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Duration
    pub duration: std::time::Duration,
    /// Status
    pub status: OperationStatus,
    /// Error if any
    pub error: Option<String>,
    /// Stack trace if error occurred
    pub stack_trace: Option<String>,
}

/// Operation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
    /// Operation is in progress
    InProgress,
}

/// Environment state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentState {
    /// Installed packages
    pub packages: Vec<PackageInfo>,
    /// Python version
    pub python_version: String,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Virtual environment path
    pub venv_path: std::path::PathBuf,
    /// Last modified
    pub last_modified: DateTime<Utc>,
}

/// Package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Installation time
    pub installed_at: DateTime<Utc>,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Whether it's a direct dependency
    pub is_direct: bool,
}

/// Dependency graph representation
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Graph structure
    graph: DiGraph<String, ()>,
    /// Node mapping
    node_map: HashMap<String, NodeIndex>,
}

impl DebugCollector {
    /// Create a new debug collector
    pub fn new() -> Self {
        Self {
            system_info: SystemInfo::collect(),
            history: Vec::new(),
            environment_state: None,
            dependency_graph: None,
        }
    }

    /// Record an operation
    pub fn record_operation(&mut self, operation: OperationRecord) {
        self.history.push(operation);
    }

    /// Capture current environment state
    pub fn capture_environment(&mut self, state: EnvironmentState) {
        self.environment_state = Some(state);
        self.update_dependency_graph();
    }

    /// Get stack trace for current thread
    pub fn get_stack_trace() -> String {
        // Simple location tracking
        let location = std::panic::Location::caller();
        format!("Error occurred at {}:{}", location.file(), location.line())
    }

    /// Generate debug report
    pub fn generate_report(&self) -> DebugReport {
        DebugReport {
            timestamp: Utc::now(),
            system_info: self.system_info.clone(),
            operation_history: self.history.clone(),
            environment_state: self.environment_state.clone(),
            dependency_graph: self.dependency_graph.as_ref().map(|g| g.to_dot()),
        }
    }

    /// Update dependency graph from environment state
    fn update_dependency_graph(&mut self) {
        if let Some(state) = &self.environment_state {
            let mut graph = DependencyGraph::new();
            
            // Add all packages as nodes
            for package in &state.packages {
                graph.add_node(&package.name);
                
                // Add dependencies as edges
                for dep in &package.dependencies {
                    graph.add_dependency(&package.name, dep);
                }
            }

            self.dependency_graph = Some(graph);
        }
    }
}

impl SystemInfo {
    /// Collect system information
    fn collect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            python_version: String::new(), // TODO: Implement Python version detection
            memory: 0, // TODO: Implement memory detection
            env_vars: std::env::vars().collect(),
            blast_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl DependencyGraph {
    /// Create a new dependency graph
    fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a node to the graph
    fn add_node(&mut self, package: &str) -> NodeIndex {
        if let Some(&node) = self.node_map.get(package) {
            node
        } else {
            let node = self.graph.add_node(package.to_string());
            self.node_map.insert(package.to_string(), node);
            node
        }
    }

    /// Add a dependency edge
    fn add_dependency(&mut self, from: &str, to: &str) {
        let from_node = self.add_node(from);
        let to_node = self.add_node(to);
        self.graph.add_edge(from_node, to_node, ());
    }

    /// Convert graph to DOT format
    fn to_dot(&self) -> String {
        format!("{:?}", Dot::with_config(&self.graph, &[Config::EdgeNoLabel]))
    }
}

/// Debug report containing all collected information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugReport {
    /// Timestamp of the report
    pub timestamp: DateTime<Utc>,
    /// System information
    pub system_info: SystemInfo,
    /// Operation history
    pub operation_history: Vec<OperationRecord>,
    /// Environment state
    pub environment_state: Option<EnvironmentState>,
    /// Dependency graph in DOT format
    pub dependency_graph: Option<String>,
} 