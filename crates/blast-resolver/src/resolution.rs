use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;

use blast_core::error::BlastResult;
use blast_core::package::Package;

use crate::cache::Cache;
use crate::pypi::PyPIClient;

/// Result of a dependency resolution
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// Resolved packages
    pub packages: Vec<Package>,
    /// Resolution graph (if available)
    pub graph: Option<ResolutionGraph>,
    /// Resolution metrics
    pub metrics: ResolutionMetrics,
}

/// Dependency resolution graph
#[derive(Debug, Clone)]
pub struct ResolutionGraph {
    /// Direct dependencies
    pub direct_deps: Vec<Package>,
    /// Transitive dependencies
    pub transitive_deps: Vec<Package>,
    /// Dependency relationships
    pub relationships: Vec<DependencyRelationship>,
}

/// Relationship between packages
#[derive(Debug, Clone)]
pub struct DependencyRelationship {
    /// Source package
    pub from: Package,
    /// Target package
    pub to: Package,
    /// Type of relationship
    pub kind: DependencyKind,
}

/// Type of dependency relationship
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyKind {
    /// Direct dependency
    Direct,
    /// Transitive dependency
    Transitive,
    /// Optional dependency
    Optional,
    /// Development dependency
    Development,
}

/// Metrics from dependency resolution
#[derive(Debug, Clone, Default)]
pub struct ResolutionMetrics {
    /// Number of packages resolved
    pub package_count: usize,
    /// Time taken for resolution (ms)
    pub resolution_time_ms: u64,
    /// Number of network requests
    pub network_requests: usize,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of version conflicts resolved
    pub conflicts_resolved: usize,
}

/// Strategy for resolving dependencies
#[async_trait]
pub trait ResolutionStrategy: Send + Sync {
    /// Resolve dependencies for a package
    async fn resolve(
        &self,
        package: &Package,
        pypi: &PyPIClient,
        cache: &Arc<RwLock<Cache>>,
    ) -> BlastResult<ResolutionResult>;

    /// Check if a version conflict exists
    fn has_conflict(&self, package: &Package, resolved: &[Package]) -> bool;

    /// Get resolution metrics
    fn get_metrics(&self) -> ResolutionMetrics;
} 