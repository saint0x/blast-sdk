//! CLI command implementations

mod start;
mod kill;
mod save;
mod load;
mod clean;
mod list;
mod check;

use std::path::PathBuf;
use blast_core::{
    config::BlastConfig,
    error::{BlastResult, BlastError},
};
use blast_daemon::{Daemon, DaemonConfig};

// Export command functions with clear names
pub use start::execute as execute_start;
pub use kill::execute as execute_kill;
pub use save::execute as execute_save;
pub use load::execute as execute_load;
pub use clean::execute as execute_clean;
pub use list::execute as execute_list;
pub use check::execute as execute_check;

/// Get a configured daemon instance with proper paths
pub(crate) async fn get_daemon(config: &BlastConfig, env_name: Option<&str>) -> BlastResult<Daemon> {
    let env_path = if let Some(name) = env_name {
        config.project_root.join("environments").join(name)
    } else {
        config.project_root.join("environments/default")
    };

    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path,
        cache_path: config.project_root.join("cache"),
    };

    Daemon::new(daemon_config).await.map_err(BlastError::from)
} 