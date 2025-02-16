use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::Daemon;
use tracing::{debug, info, warn};

/// Execute the kill command
pub async fn execute(
    force: bool,
    _config: &BlastConfig,
) -> BlastResult<()> {
    debug!("Killing blast environment");
    debug!("Force: {}", force);

    // Check if we're in a blast environment
    let env_name = match std::env::var("BLAST_ENV_NAME") {
        Ok(name) => name,
        Err(_) => {
            warn!("Not in a blast environment");
            return Ok(());
        }
    };

    let env_path = match std::env::var("BLAST_ENV_PATH") {
        Ok(path) => path,
        Err(_) => {
            warn!("Environment path not found");
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
        if force {
            // Force kill
            daemon.destroy_environment(&env).await?;
        } else {
            // Graceful shutdown
            info!("Gracefully shutting down environment");
            
            // Save state if needed
            daemon.save_environment_state(&env).await?;
            
            // Stop monitoring
            daemon.stop_monitoring(&env).await?;
            
            // Destroy environment
            daemon.destroy_environment(&env).await?;
        }

        // Clean up environment variables
        std::env::remove_var("BLAST_ENV_NAME");
        std::env::remove_var("BLAST_ENV_PATH");
        std::env::remove_var("BLAST_SOCKET_PATH");

        // Restore shell prompt
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("bash") {
                std::env::remove_var("PS1");
            } else if shell.contains("zsh") {
                std::env::remove_var("PROMPT");
            }
        }

        info!("Killed blast environment:");
        info!("  Name: {}", env_name);
        info!("  Path: {}", env_path);
    } else {
        warn!("Environment not found: {}", env_name);
    }

    Ok(())
} 