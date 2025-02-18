//! Dependency resolver for the Blast Python environment manager.
//!
//! This crate provides the dependency resolution functionality for Blast,
//! implementing the PubGrub algorithm for Python packages.

use std::sync::Arc;
use blast_core::error::BlastResult;
use blast_core::package::Package;

mod cache;
mod pypi;
mod pubgrub;
mod resolution;
pub mod resolver;

pub use cache::Cache;
pub use pypi::PyPIClient;
pub use resolver::DependencyResolver;
pub use resolution::{ResolutionStrategy, ResolutionResult, ResolutionGraph};

/// Configuration for the resolver
#[derive(Debug, Clone)]
pub struct Config {
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: u64,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Whether to verify SSL certificates
    pub verify_ssl: bool,
    /// Whether to allow pre-releases
    pub allow_prereleases: bool,
    /// Additional package sources
    pub additional_sources: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 10,
            request_timeout: 30,
            verify_ssl: true,
            allow_prereleases: false,
            additional_sources: Vec::new(),
        }
    }
}

/// Create a new resolver with default configuration
pub async fn create_resolver() -> BlastResult<Arc<DependencyResolver>> {
    create_resolver_with_config(Config::default()).await
}

/// Create a new resolver with the given configuration
pub async fn create_resolver_with_config(config: Config) -> BlastResult<Arc<DependencyResolver>> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| blast_core::error::BlastError::cache("Failed to get cache directory".to_string()))?
        .join("blast");

    let pypi_client = PyPIClient::new(
        config.max_concurrent_requests,
        config.request_timeout,
        config.verify_ssl,
    )?;

    let cache = Cache::new(cache_dir);
    Ok(Arc::new(DependencyResolver::new(pypi_client, cache)))
}

/// Resolve dependencies for a package
pub async fn resolve(package: Package) -> BlastResult<Vec<Package>> {
    let resolver = create_resolver().await?;
    resolver.resolve(&package).await
}
