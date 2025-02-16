//! Package management and dependencies
//! 
//! This module provides functionality for managing Python packages,
//! dependencies, and package indexes.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use url::Url;

/// Package dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDependency {
    /// Package name
    pub name: String,
    /// Version specification
    pub version_spec: String,
    /// Optional extras
    pub extras: HashSet<String>,
    /// Whether this is a direct dependency
    pub is_direct: bool,
    /// Package hash (if available)
    pub hash: Option<String>,
    /// Package URL
    pub url: Option<String>,
    /// Build tags
    pub build_tags: Vec<String>,
    /// Platform tags
    pub platform_tags: Vec<String>,
}

/// Package index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageIndex {
    /// Index URL
    pub url: Url,
    /// Index name
    pub name: String,
    /// Whether this is a trusted index
    pub trusted: bool,
    /// Authentication credentials (if required)
    pub credentials: Option<IndexCredentials>,
    /// Index priority (lower is higher priority)
    pub priority: i32,
}

/// Package index credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCredentials {
    /// Username
    pub username: Option<String>,
    /// Password
    pub password: Option<String>,
    /// API token
    pub token: Option<String>,
}

/// Package manager configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageConfig {
    /// Package indexes
    pub indexes: Vec<PackageIndex>,
    /// Direct dependencies
    pub direct_dependencies: HashMap<String, PackageDependency>,
    /// Transitive dependencies
    pub transitive_dependencies: HashMap<String, PackageDependency>,
    /// Build isolation
    pub build_isolation: bool,
    /// Use user site packages
    pub use_user_site: bool,
    /// Cache directory
    pub cache_dir: Option<String>,
    /// Build directory
    pub build_dir: Option<String>,
}

impl PackageConfig {
    /// Create new package configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a package index
    pub fn add_index(&mut self, index: PackageIndex) {
        // Sort indexes by priority after adding
        self.indexes.push(index);
        self.indexes.sort_by_key(|idx| idx.priority);
    }

    /// Add a direct dependency
    pub fn add_direct_dependency(&mut self, dependency: PackageDependency) {
        self.direct_dependencies.insert(dependency.name.clone(), dependency);
    }

    /// Add a transitive dependency
    pub fn add_transitive_dependency(&mut self, dependency: PackageDependency) {
        self.transitive_dependencies.insert(dependency.name.clone(), dependency);
    }

    /// Get all dependencies (direct and transitive)
    pub fn all_dependencies(&self) -> HashMap<String, &PackageDependency> {
        let mut all = HashMap::new();
        
        // Add direct dependencies
        for (name, dep) in &self.direct_dependencies {
            all.insert(name.clone(), dep);
        }
        
        // Add transitive dependencies
        for (name, dep) in &self.transitive_dependencies {
            if !all.contains_key(name) {
                all.insert(name.clone(), dep);
            }
        }
        
        all
    }

    /// Get dependency tree
    pub fn dependency_tree(&self) -> DependencyTree {
        let mut tree = DependencyTree::new();
        
        // Add direct dependencies as roots
        for dep in self.direct_dependencies.values() {
            tree.add_dependency(dep.clone(), None);
        }
        
        // Add transitive dependencies
        for dep in self.transitive_dependencies.values() {
            // Find parent dependency
            if let Some(parent) = self.find_parent_dependency(dep) {
                tree.add_dependency(dep.clone(), Some(parent.clone()));
            }
        }
        
        tree
    }

    /// Find parent dependency for a transitive dependency
    fn find_parent_dependency(&self, dep: &PackageDependency) -> Option<PackageDependency> {
        // This is a simplified implementation
        // In reality, we would need to parse requirements and build a proper dependency graph
        self.direct_dependencies.values()
            .find(|d| d.name < dep.name)
            .cloned()
    }
}

/// Dependency tree node
#[derive(Debug, Clone)]
struct DependencyNode {
    dependency: PackageDependency,
    children: Vec<DependencyNode>,
}

/// Dependency tree
#[derive(Debug, Clone)]
pub struct DependencyTree {
    roots: Vec<DependencyNode>,
}

impl DependencyTree {
    /// Create new dependency tree
    fn new() -> Self {
        Self {
            roots: Vec::new(),
        }
    }

    /// Add a dependency to the tree
    fn add_dependency(&mut self, dependency: PackageDependency, parent: Option<PackageDependency>) {
        let node = DependencyNode {
            dependency,
            children: Vec::new(),
        };

        if let Some(parent) = parent {
            // Find parent node and add as child
            self.add_to_parent(&parent, node);
        } else {
            // Add as root
            self.roots.push(node);
        }
    }

    /// Add node to parent
    fn add_to_parent(&mut self, parent: &PackageDependency, node: DependencyNode) {
        // Recursive helper
        fn add_to_node(current: &mut DependencyNode, parent: &PackageDependency, node: DependencyNode) {
            if current.dependency.name == parent.name {
                current.children.push(node);
            } else {
                for child in &mut current.children {
                    add_to_node(child, parent, node.clone());
                }
            }
        }

        // Try to add to each root
        for root in &mut self.roots {
            add_to_node(root, parent, node.clone());
        }
    }

    /// Print tree
    pub fn print(&self) -> String {
        let mut output = String::new();
        
        for root in &self.roots {
            self.print_node(root, 0, &mut output);
        }
        
        output
    }

    /// Print node
    fn print_node(&self, node: &DependencyNode, depth: usize, output: &mut String) {
        let indent = "  ".repeat(depth);
        output.push_str(&format!("{}{} {}\n", 
            indent,
            node.dependency.name,
            node.dependency.version_spec
        ));
        
        for child in &node.children {
            self.print_node(child, depth + 1, output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_config() {
        let mut config = PackageConfig::new();
        
        // Add index
        let index = PackageIndex {
            url: Url::parse("https://pypi.org/simple").unwrap(),
            name: "PyPI".to_string(),
            trusted: true,
            credentials: None,
            priority: 0,
        };
        config.add_index(index);
        
        assert_eq!(config.indexes.len(), 1);
    }

    #[test]
    fn test_dependencies() {
        let mut config = PackageConfig::new();
        
        // Add direct dependency
        let requests = PackageDependency {
            name: "requests".to_string(),
            version_spec: ">=2.25.0".to_string(),
            extras: HashSet::new(),
            is_direct: true,
            hash: None,
            url: None,
            build_tags: Vec::new(),
            platform_tags: Vec::new(),
        };
        config.add_direct_dependency(requests);
        
        assert_eq!(config.direct_dependencies.len(), 1);
        assert!(config.direct_dependencies.contains_key("requests"));
    }

    #[test]
    fn test_dependency_tree() {
        let mut config = PackageConfig::new();
        
        // Add dependencies
        let requests = PackageDependency {
            name: "requests".to_string(),
            version_spec: ">=2.25.0".to_string(),
            extras: HashSet::new(),
            is_direct: true,
            hash: None,
            url: None,
            build_tags: Vec::new(),
            platform_tags: Vec::new(),
        };
        config.add_direct_dependency(requests);
        
        let urllib3 = PackageDependency {
            name: "urllib3".to_string(),
            version_spec: ">=1.21.1".to_string(),
            extras: HashSet::new(),
            is_direct: false,
            hash: None,
            url: None,
            build_tags: Vec::new(),
            platform_tags: Vec::new(),
        };
        config.add_transitive_dependency(urllib3);
        
        let tree = config.dependency_tree();
        let tree_str = tree.print();
        
        assert!(tree_str.contains("requests"));
        assert!(tree_str.contains("urllib3"));
    }
} 