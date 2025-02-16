//! Blast Python environment manager.
//! 
//! This crate provides a high-performance, automated Python environment
//! management system with real-time dependency monitoring and optimization.

pub use blast_core as core;
pub use blast_cache as cache;
pub use blast_resolver as resolver;

/// Initialize logging for the entire system
pub fn init() {
    tracing_subscriber::fmt::init();
}

/// Version of the Blast system
pub const VERSION: &str = env!("CARGO_PKG_VERSION"); 