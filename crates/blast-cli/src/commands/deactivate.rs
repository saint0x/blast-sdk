use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info};

pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Deactivating environment");

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments/default"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Deactivate current environment
    daemon.deactivate_environment().await?;

    // Get current state to show what was deactivated
    let state_manager = daemon.state_manager();
    let current_state = state_manager.read().await.get_current_state();

    info!("Deactivated environment: {}", current_state.name());
    info!("Run 'blast list' to see available environments");

    Ok(())
} 