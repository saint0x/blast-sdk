use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::Daemon;
use tracing::{debug, info};
use humantime;

/// Execute the list command
pub async fn execute(_config: &BlastConfig) -> BlastResult<()> {
    debug!("Listing environments");

    // Create daemon configuration
    let daemon_config = blast_daemon::DaemonConfig {
        max_pending_updates: 100,
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get all environments with their last access time
    let mut environments = daemon.list_environments().await?;
    
    // Sort by last access time (most recent first)
    environments.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

    // Get current environment name if any
    let current_env = std::env::var("BLAST_ENV_NAME").ok();

    info!("Blast environments:");
    if environments.is_empty() {
        info!("  No environments found");
    } else {
        for env in environments {
            let status = if Some(&env.name) == current_env.as_ref() {
                "*active*"
            } else {
                ""
            };
            
            let last_accessed = humantime::format_duration(
                std::time::SystemTime::now()
                    .duration_since(env.last_accessed)
                    .unwrap_or_default()
            );

            info!(
                "  {} {} (Python {}) [Last used: {} ago]",
                env.name,
                status,
                env.python_version,
                last_accessed
            );

            // Show saved images for this environment
            if let Ok(images) = daemon.list_environment_images(&env.name).await {
                for image in images {
                    info!(
                        "    └── Image: {} ({})",
                        image.name,
                        image.created.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
        }
    }

    Ok(())
} 