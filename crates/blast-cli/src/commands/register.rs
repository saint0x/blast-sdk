use std::path::PathBuf;
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
    state::EnvironmentState,
    python::PythonVersion,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info};
use std::collections::HashMap;

pub async fn execute(
    name: String,
    path: PathBuf,
    config: &BlastConfig,
) -> BlastResult<()> {
    debug!("Registering environment: {} at {}", name, path.display());

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: path.clone(),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get Python version from environment
    let python_version = if let Ok(version) = std::env::var("BLAST_PYTHON_VERSION") {
        PythonVersion::parse(&version)?
    } else {
        // Default to Python 3.8 if not specified
        PythonVersion::parse("3.8")?
    };

    // Create environment state
    let env_state = EnvironmentState::new(
        name.clone(),
        python_version,
        HashMap::new(), // Empty packages initially
        HashMap::new(), // Empty env vars initially
    );

    // Update state manager
    let state_manager = daemon.state_manager();
    state_manager.write().await.update_current_state(env_state.clone())?;

    // Register as active environment
    daemon.register_active_environment(name.clone()).await?;

    info!("Successfully registered environment:");
    info!("  Name: {}", name);
    info!("  Path: {}", path.display());
    info!("  Python: {}", python_version);

    Ok(())
} 