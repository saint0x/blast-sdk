use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig, state::StateManagement};
use tracing::{debug, info};

/// Execute the check command
pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Checking environment status");

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments/default"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get current environment state
    let state_manager = daemon.state_manager();
    let state_guard = state_manager.read().await;
    let state_manager: &dyn StateManagement = &*state_guard;
    let current_state = state_manager.get_current_state().await?;

    info!("Environment Status:");
    if current_state.active_env_name.is_none() {
        info!("  No active environment");
        return Ok(());
    }

    // Show environment details
    let env_name = current_state.active_env_name.clone().unwrap();
    let python_version = current_state.active_python_version
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    info!("  Name: {}", env_name);
    info!("  Python: {}", python_version);
    info!("  Status: Active");
    
    // TODO: Implement package listing
    info!("  Packages: Not implemented yet");
    info!("  Path: {}", config.project_root.join("environments").join(&env_name).display());

    Ok(())
} 