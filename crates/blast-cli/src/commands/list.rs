use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig, state::StateManagement};
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

    // Get current state to check which environment is active
    let state_manager = daemon.state_manager();
    let state_guard = state_manager.read().await;
    let state_manager: &dyn StateManagement = &*state_guard;
    let current_state = state_manager.get_current_state().await?;

    info!("Blast environments:");
    
    // For now, just show the active environment if there is one
    if let Some(env_name) = current_state.active_env_name {
        let status = "*active*";
        let python_version = current_state.active_python_version
            .as_ref()
            .map_or("unknown".to_string(), |v| v.to_string());
        let path = current_state.active_env_path
            .as_ref()
            .map_or_else(|| "unknown".to_string(), |p| p.display().to_string());

        info!(
            "  {} {} (Python {}) [{}]",
            env_name,
            status,
            python_version,
            path
        );
    } else {
        info!("  No environments found");
    }

    Ok(())
} 