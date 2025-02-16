use std::collections::{HashMap, HashSet};
use petgraph::{Graph, Directed};
use petgraph::graph::NodeIndex;
use petgraph::algo::is_cyclic_directed;
use petgraph::visit::Dfs;

use blast_core::{
    package::{Package, Version},
};

use crate::DaemonResult;

/// Validation result for a package dependency graph
#[derive(Debug)]
pub struct ValidationResult {
    /// Whether the graph is valid
    pub is_valid: bool,
    /// List of validation issues found
    pub issues: Vec<ValidationIssue>,
    /// Graph metrics
    pub metrics: ValidationMetrics,
}

/// Types of validation issues that can be found
#[derive(Debug)]
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

/// Metrics collected during validation
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

/// Dependency graph validator
#[derive(Debug)]
pub struct DependencyValidator {
    /// Graph representation of dependencies
    graph: Graph<Package, (), Directed>,
    /// Map of package names to node indices
    package_map: HashMap<String, NodeIndex>,
    /// Validation metrics
    metrics: ValidationMetrics,
}

impl DependencyValidator {
    /// Create a new dependency validator
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

    /// Add a package to the validation graph
    pub fn add_package(&mut self, package: Package) -> NodeIndex {
        let name = package.name().to_string();
        if let Some(&idx) = self.package_map.get(&name) {
            idx
        } else {
            let idx = self.graph.add_node(package);
            self.package_map.insert(name, idx);
            idx
        }
    }

    /// Add a dependency relationship between packages
    pub fn add_dependency(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, ());
        self.metrics.dependencies_checked += 1;
    }

    /// Validate the dependency graph
    pub fn validate(&mut self) -> DaemonResult<ValidationResult> {
        let mut issues = Vec::new();

        // Check for circular dependencies
        if is_cyclic_directed(&self.graph) {
            let cycles = self.find_cycles();
            for cycle in cycles {
                issues.push(ValidationIssue::CircularDependency {
                    packages: cycle.iter()
                        .map(|&idx| self.graph[idx].name().to_string())
                        .collect(),
                });
            }
        }

        // Check version conflicts
        let version_conflicts = self.check_version_conflicts();
        issues.extend(version_conflicts);

        // Check Python version compatibility
        let python_conflicts = self.check_python_version_conflicts();
        issues.extend(python_conflicts);

        // Update metrics
        self.metrics.packages_checked = self.graph.node_count();
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
                self.find_cycles_recursive(node, &mut visited, &mut stack, &mut cycles);
            }
        }

        cycles
    }

    /// Recursive helper for finding cycles
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
                let cycle_start = stack.iter().position(|&n| n == neighbor).unwrap();
                cycles.push(stack[cycle_start..].to_vec());
            }
        }

        stack.pop();
    }

    /// Check for version conflicts between packages
    fn check_version_conflicts(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let mut version_map: HashMap<String, HashSet<Version>> = HashMap::new();

        for node in self.graph.node_indices() {
            let package = &self.graph[node];
            version_map.entry(package.name().to_string())
                .or_default()
                .insert(package.version().clone());
        }

        for (package, versions) in version_map {
            if versions.len() > 1 {
                issues.push(ValidationIssue::VersionConflict {
                    package,
                    required_versions: versions.into_iter().collect(),
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

            for dep in self.graph.neighbors(node) {
                let dep_package = &self.graph[dep];
                // Check if versions are compatible using string representation
                
                if package.python_version().to_string() != dep_package.python_version().to_string() {
                    constraints.push(dep_package.python_version().to_string());
                }
            }

            if !constraints.is_empty() {
                issues.push(ValidationIssue::PythonVersionConflict {
                    package: package.name().to_string(),
                    version_constraints: constraints,
                });
            }
        }

        issues
    }

    /// Calculate the maximum dependency depth
    fn calculate_max_depth(&self) -> usize {
        let mut max_depth = 0;
        let mut dfs = Dfs::new(&self.graph, self.graph.node_indices().next().unwrap_or_else(|| panic!("Empty graph")));
        
        while let Some(_) = dfs.next(&self.graph) {
            max_depth += 1;
        }

        max_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blast_core::package::{PackageId, VersionConstraint};
    use std::str::FromStr;

    #[test]
    fn test_circular_dependency_detection() {
        let mut validator = DependencyValidator::new();

        // Create packages
        let package_a = Package::new(
            PackageId::new(
                "package-a",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        let package_b = Package::new(
            PackageId::new(
                "package-b",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        // Add packages and create circular dependency
        let a_idx = validator.add_package(package_a);
        let b_idx = validator.add_package(package_b);
        validator.add_dependency(a_idx, b_idx);
        validator.add_dependency(b_idx, a_idx);

        // Validate
        let result = validator.validate().unwrap();
        assert!(!result.is_valid);
        assert!(matches!(
            result.issues[0],
            ValidationIssue::CircularDependency { .. }
        ));
    }

    #[test]
    fn test_version_conflict_detection() {
        let mut validator = DependencyValidator::new();

        // Create packages with conflicting versions
        let package_a_v1 = Package::new(
            PackageId::new(
                "package-a",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        let package_a_v2 = Package::new(
            PackageId::new(
                "package-a",
                Version::parse("2.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        // Add packages
        validator.add_package(package_a_v1);
        validator.add_package(package_a_v2);

        // Validate
        let result = validator.validate().unwrap();
        assert!(!result.is_valid);
        assert!(matches!(
            result.issues[0],
            ValidationIssue::VersionConflict { .. }
        ));
    }

    #[test]
    fn test_python_version_conflict_detection() {
        let mut validator = DependencyValidator::new();

        // Create packages with conflicting Python versions
        let package_a = Package::new(
            PackageId::new(
                "package-a",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        let package_b = Package::new(
            PackageId::new(
                "package-b",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.9").unwrap(),
        );

        // Add packages and dependency
        let a_idx = validator.add_package(package_a);
        let b_idx = validator.add_package(package_b);
        validator.add_dependency(a_idx, b_idx);

        // Validate
        let result = validator.validate().unwrap();
        assert!(!result.is_valid);
        assert!(matches!(
            result.issues[0],
            ValidationIssue::PythonVersionConflict { .. }
        ));
    }
} 