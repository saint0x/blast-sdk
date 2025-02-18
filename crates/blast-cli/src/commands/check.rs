use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig};
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
    let current_state = state_guard.get_current_state().await?;

    // Get activation state
    let _activation_state = daemon.get_activation_state().await?;

    info!("Environment Status:");
    if !current_state.is_active() {
        info!("  No active environment");
        return Ok(());
    }

    // Show environment details
    info!("  Name: {}", current_state.name());
    info!("  Python: {}", current_state.python_version);
    info!("  Status: Active");
    info!("  Packages: {}", current_state.packages.len());

    // List environments to get package count
    let environments = daemon.list_environments().await?;
    if let Some(env) = environments.iter().find(|e| e.name == current_state.name()) {
        info!("  Path: {}", env.path.display());
    }

    // Show verification status
    if let Some(verification) = current_state.verification.as_ref() {
        info!("\nVerification Status:");
        info!("  Verified: {}", verification.is_verified);
        if !verification.issues.is_empty() {
            info!("  Issues:");
            for issue in &verification.issues {
                info!("    - {} ({:?})", issue.description, issue.severity);
                if let Some(context) = &issue.context {
                    info!("      Context: {}", context);
                }
                if let Some(recommendation) = &issue.recommendation {
                    info!("      Recommendation: {}", recommendation);
                }
            }
        }
    }

    Ok(())
} 