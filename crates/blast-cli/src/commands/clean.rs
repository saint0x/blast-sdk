use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info};

/// Execute the clean command
pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Cleaning environment");

    // Check if we're in a blast environment
    let env_name = match std::env::var("BLAST_ENV_NAME") {
        Ok(name) => name,
        Err(_) => {
            info!("Not in a blast environment");
            return Ok(());
        }
    };

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments/default"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to existing daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get current environment state
    let state_manager = daemon.state_manager();
    let state_guard = state_manager.read().await;
    let current_state = state_guard.get_current_state().await?;

    if current_state.is_active() {
        info!("Cleaning environment: {}", env_name);
        
        // Create a Python environment instance
        let env = blast_core::python::PythonEnvironment::new(
            config.project_root.join("environments").join(&env_name),
            current_state.python_version.clone(),
        );
        
        // Clean the environment
        daemon.clean_environment(&env).await?;
        
        // Reinitialize environment
        daemon.reinitialize_environment(&env).await?;

        info!("Environment cleaned and reinitialized");
    } else {
        info!("No active environment found");
    }

    Ok(())
} 