use std::collections::{HashMap, HashSet};
use petgraph::{Graph, Directed};
use petgraph::graph::NodeIndex;
use petgraph::algo::kosaraju_scc;

use blast_core::{
    error::BlastResult,
    package::Package,
    version::Version,
};

use crate::error::DaemonResult;

/// Result of dependency validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the graph is valid
    pub is_valid: bool,
    /// List of validation issues found
    pub issues: Vec<ValidationIssue>,
    /// Graph metrics
    pub metrics: ValidationMetrics,
}

/// Validation issue types
#[derive(Debug, Clone)]
pub enum ValidationIssue {
    /// Circular dependency detected
    CircularDependency {
        packages: Vec<String>,
    },
    /// Version conflict detected
    VersionConflict {
        package: String,
        required_versions: Vec<Version>,
    },
    /// Missing dependency
    MissingDependency {
        package: String,
        required_by: String,
    },
    /// Python version conflict
    PythonVersionConflict {
        package: String,
        version_constraints: Vec<String>,
    },
}

/// Validation metrics
#[derive(Debug, Clone)]
pub struct ValidationMetrics {
    /// Number of packages checked
    pub packages_checked: usize,
    /// Number of dependencies checked
    pub dependencies_checked: usize,
    /// Maximum dependency depth
    pub max_depth: usize,
    /// Number of version constraints checked
    pub version_constraints_checked: usize,
}

/// Dependency validator
pub struct DependencyValidator {
    /// Graph representation of dependencies
    graph: Graph<Package, (), Directed>,
    /// Map of package names to node indices
    package_map: HashMap<String, NodeIndex>,
    /// Validation metrics
    metrics: ValidationMetrics,
}

impl DependencyValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            package_map: HashMap::new(),
            metrics: ValidationMetrics {
                packages_checked: 0,
                dependencies_checked: 0,
                max_depth: 0,
                version_constraints_checked: 0,
            },
        }
    }

    /// Add a package to the graph
    pub fn add_package(&mut self, package: Package) -> NodeIndex {
        let idx = self.graph.add_node(package.clone());
        self.package_map.insert(package.name().to_string(), idx);
        idx
    }

    /// Add a dependency between packages
    pub fn add_dependency(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, ());
    }

    /// Validate the dependency graph
    pub fn validate(&mut self) -> DaemonResult<ValidationResult> {
        let mut issues = Vec::new();
        
        // Check for circular dependencies
        let cycles = self.find_cycles();
        for cycle in cycles {
            let packages = cycle.iter()
                .map(|&idx| self.graph[idx].name().to_string())
                .collect();
            issues.push(ValidationIssue::CircularDependency { packages });
        }
        
        // Check for version conflicts
        issues.extend(self.check_version_conflicts());
        
        // Check for Python version conflicts
        issues.extend(self.check_python_version_conflicts());
        
        // Update metrics
        self.metrics.packages_checked = self.graph.node_count();
        self.metrics.dependencies_checked = self.graph.edge_count();
        self.metrics.max_depth = self.calculate_max_depth();
        
        Ok(ValidationResult {
            is_valid: issues.is_empty(),
            issues,
            metrics: self.metrics.clone(),
        })
    }

    /// Find cycles in the dependency graph
    fn find_cycles(&self) -> Vec<Vec<NodeIndex>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        
        for node in self.graph.node_indices() {
            if !visited.contains(&node) {
                self.find_cycles_recursive(
                    node,
                    &mut visited,
                    &mut stack,
                    &mut cycles,
                );
            }
        }
        
        cycles
    }

    /// Recursive helper for cycle detection
    fn find_cycles_recursive(
        &self,
        node: NodeIndex,
        visited: &mut HashSet<NodeIndex>,
        stack: &mut Vec<NodeIndex>,
        cycles: &mut Vec<Vec<NodeIndex>>,
    ) {
        visited.insert(node);
        stack.push(node);
        
        for neighbor in self.graph.neighbors(node) {
            if !visited.contains(&neighbor) {
                self.find_cycles_recursive(neighbor, visited, stack, cycles);
            } else if stack.contains(&neighbor) {
                // Found a cycle
                let cycle_start = stack.iter().position(|&n| n == neighbor).unwrap();
                cycles.push(stack[cycle_start..].to_vec());
            }
        }
        
        stack.pop();
    }

    /// Check for version conflicts
    fn check_version_conflicts(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let mut version_map: HashMap<String, Vec<Version>> = HashMap::new();
        
        for node in self.graph.node_indices() {
            let package = &self.graph[node];
            version_map.entry(package.name().to_string())
                .or_default()
                .push(package.version().clone());
        }
        
        for (package, versions) in version_map {
            if versions.len() > 1 {
                issues.push(ValidationIssue::VersionConflict {
                    package,
                    required_versions: versions,
                });
            }
        }
        
        issues
    }

    /// Check for Python version conflicts
    fn check_python_version_conflicts(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        for node in self.graph.node_indices() {
            let package = &self.graph[node];
            let mut constraints = Vec::new();
            
            for neighbor in self.graph.neighbors(node) {
                let dep = &self.graph[neighbor];
                constraints.push(dep.python_version().to_string());
                
                if !package.python_version().is_compatible_with(dep.python_version()) {
                    issues.push(ValidationIssue::PythonVersionConflict {
                        package: package.name().to_string(),
                        version_constraints: constraints.clone(),
                    });
                }
            }
        }
        
        issues
    }

    /// Calculate maximum dependency depth
    fn calculate_max_depth(&self) -> usize {
        let mut max_depth = 0;
        let mut visited = HashSet::new();
        
        for node in self.graph.node_indices() {
            if !visited.contains(&node) {
                let depth = self.calculate_depth(node, &mut visited);
                max_depth = max_depth.max(depth);
            }
        }
        
        max_depth
    }

    /// Calculate depth for a node
    fn calculate_depth(&self, node: NodeIndex, visited: &mut HashSet<NodeIndex>) -> usize {
        if visited.contains(&node) {
            return 0;
        }
        
        visited.insert(node);
        let mut max_child_depth = 0;
        
        for neighbor in self.graph.neighbors(node) {
            let depth = self.calculate_depth(neighbor, visited);
            max_child_depth = max_child_depth.max(depth);
        }
        
        max_child_depth + 1
    }
}

impl Default for DependencyValidator {
    fn default() -> Self {
        Self::new()
    }
} 