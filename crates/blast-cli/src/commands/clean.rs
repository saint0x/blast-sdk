use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::Daemon;
use tracing::{debug, info};

/// Execute the clean command
pub async fn execute(_config: &BlastConfig) -> BlastResult<()> {
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
    let daemon_config = blast_daemon::DaemonConfig {
        max_pending_updates: 100,
    };

    // Connect to existing daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get active environment
    if let Some(env) = daemon.get_active_environment().await? {
        info!("Cleaning environment: {}", env_name);
        
        // Save current state for recovery if needed
        daemon.save_environment_state(&env).await?;
        
        // Remove all packages
        daemon.clean_environment(&env).await?;
        
        // Reinitialize environment
        daemon.reinitialize_environment(&env).await?;
        
        // Restore essential packages
        daemon.restore_essential_packages(&env).await?;

        info!("Environment cleaned and reinitialized");
    } else {
        info!("No active environment found");
    }

    Ok(())
} 