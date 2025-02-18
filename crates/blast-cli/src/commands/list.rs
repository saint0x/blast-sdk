use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info};

/// Execute the list command
pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Listing environments");

    // Create daemon configuration with resolved paths
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get environment list
    let environments = daemon.list_environments().await?;
    debug!("Retrieved environment list");

    // Get current state to check which environment is active
    let state_manager = daemon.state_manager();
    let state_guard = state_manager.read().await;
    let current_state = state_guard.get_current_state().await?;

    info!("Blast environments:");
    if environments.is_empty() {
        info!("  No environments found");
        return Ok(());
    }

    for env in environments {
        let status = if current_state.is_active() && env.name == current_state.name() {
            "*active*"
        } else {
            ""
        };

        info!(
            "  {} {} (Python {}) [{}]",
            env.name,
            status,
            env.python_version,
            env.path.display()
        );
    }

    Ok(())
} 